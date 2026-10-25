// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{AssemblyResolution, DependencyContext, OutputStyle, Result, Summary};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const USAGE: &str = "Usage:\n  ilspy-lite list <assembly>\n  ilspy-lite il <assembly> --method <0x06000001>\n  ilspy-lite decompile <assembly> --method <0x06000001> [--details]\n  ilspy-lite decompile <assembly> --type <Namespace.Type> [--details]\n\nDefault: readable C#-like view. --details: full metadata/IL evidence.\nDecompile options: --reference <assembly>, --reference-dir <directory> (repeatable), --no-auto-references.\nExit: 0 complete/declaration, 2 partial IL fallback, 1 failure.\nOutput is C#-like; no promise of recompilable source.";

fn main() -> std::process::ExitCode {
    match run() {
        Ok(code) => std::process::ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::from(1)
        }
    }
}

fn load_reference(
    context: &mut DependencyContext,
    path: &Path,
    seen: &mut BTreeSet<PathBuf>,
    warnings: &mut Vec<String>,
) {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !seen.insert(path.clone()) {
        return;
    }
    let source = path
        .to_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("{path:?}"));
    let loaded =
        (|| -> Result<()> { context.add_reference_reader(&source, std::fs::File::open(&path)?) })();
    if let Err(error) = loaded {
        warnings.push(format!(
            "{source:?}: {}",
            error.to_string().escape_default()
        ));
    }
}

fn run() -> Result<u8> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut style = OutputStyle::Readable;
    let mut references = Vec::new();
    let mut reference_dirs = Vec::new();
    let mut automatic = args.first().is_some_and(|arg| arg == "decompile");
    if args.first().is_some_and(|arg| arg == "decompile") && args.len() >= 4 {
        let mut index = 4;
        while index < args.len() {
            if args[index] == "--details" {
                style = OutputStyle::Details;
            } else if args[index] == "--no-auto-references" {
                automatic = false;
            } else if args[index] == "--reference" || args[index] == "--reference-dir" {
                let directory = args[index] == "--reference-dir";
                index += 1;
                let path = args
                    .get(index)
                    .filter(|p| !p.to_string_lossy().starts_with("--"))
                    .ok_or("reference option needs a path")?;
                if directory {
                    reference_dirs.push(PathBuf::from(path));
                } else {
                    references.push(PathBuf::from(path));
                }
            } else {
                return Err(USAGE.into());
            }
            index += 1;
        }
        args.truncate(4);
    }
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        writeln!(out, "{USAGE}")?;
        out.flush()?;
        return Ok(0);
    }
    let valid = (args.len() == 2 && args[0] == "list")
        || (args.len() == 4
            && (args[0] == "il" || args[0] == "decompile")
            && (args[2] == "--method" || (args[0] == "decompile" && args[2] == "--type")));
    if !valid {
        return Err(USAGE.into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&args[1])?
        .take(512 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if args[0] == "list" {
        for m in ilspy_lite::list_methods(&bytes)? {
            writeln!(out, "0x{:08X} {}", m.token, m.name.escape_default())?;
        }
        out.flush()?;
        return Ok(0);
    }
    let mut reference_warnings = Vec::new();
    let context = if !automatic && references.is_empty() && reference_dirs.is_empty() {
        None
    } else {
        let mut context = DependencyContext::new(std::mem::take(&mut bytes))?;
        let mut seen = BTreeSet::new();
        for reference in references {
            load_reference(&mut context, &reference, &mut seen, &mut reference_warnings);
        }
        if automatic {
            let directory = Path::new(&args[1])
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            reference_dirs.push(directory.to_path_buf());
        }
        for directory in &reference_dirs {
            if !directory.is_dir() {
                reference_warnings.push(format!("{directory:?}: reference directory unavailable"));
            }
        }
        for reference in context.references().to_vec() {
            let name = &reference.identity.name;
            if name.is_empty()
                || matches!(name.as_str(), "." | "..")
                || name.contains(['/', '\\', ':', '\0'])
            {
                reference_warnings.push(format!("unsafe assembly filename: {name:?}"));
                continue;
            }
            'search: for directory in &reference_dirs {
                for extension in ["dll", "exe"] {
                    if !matches!(
                        context.resolve(reference.token)?,
                        AssemblyResolution::Missing { .. }
                    ) {
                        break 'search;
                    }
                    let path = directory.join(format!("{name}.{extension}"));
                    if path.is_file() {
                        load_reference(&mut context, &path, &mut seen, &mut reference_warnings);
                    }
                }
            }
        }
        Some(context)
    };
    let value = args[3].to_str().ok_or("selector must be UTF-8")?;
    let summary = if args[2] == "--type" {
        let result = if let Some(context) = &context {
            context.decompile_type(value, style)?
        } else {
            ilspy_lite::decompile_type_with_style(&bytes, value, style)?
        };
        write!(out, "{}", result.text)?;
        result.summary
    } else {
        let token = if let Some(s) = value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
        {
            u32::from_str_radix(s, 16)?
        } else {
            value.parse()?
        };
        if args[0] == "il" {
            write!(out, "{}", ilspy_lite::disassemble(&bytes, token)?)?;
            out.flush()?;
            return Ok(0);
        }
        let mut summary = Summary::default();
        let result = if let Some(context) = &context {
            context.decompile(token, style)
        } else {
            ilspy_lite::decompile_with_style(&bytes, token, style)
        };
        match result {
            Ok(result) => {
                summary.record(&result.status);
                writeln!(out, "{}", result.text)?;
            }
            Err(error) => {
                summary.failed += 1;
                writeln!(
                    out,
                    "// FAILED token 0x{token:08X}: {}",
                    error.to_string().escape_default()
                )?;
            }
        }
        summary
    };
    for warning in reference_warnings {
        writeln!(out, "\n// Dependency input rejected: {warning}")?;
        eprintln!("dependency warning: {warning}");
    }
    out.flush()?;
    eprintln!(
        "decompiled={} declarations={} partial={} failed={}",
        summary.decompiled, summary.declarations, summary.partial, summary.failed
    );
    if summary.metadata_failed > 0 {
        eprintln!("metadata_failed={}", summary.metadata_failed);
    }
    Ok(summary.exit_code())
}
