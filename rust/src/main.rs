// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{
    DependencyContext, OutputStyle, ReferenceSearchOptions, ReferenceVersionPolicy, Result, Summary,
};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const USAGE: &str = "Usage:\n  ilspy-lite list <assembly>\n  ilspy-lite identities <assembly>...\n  ilspy-lite scan <assembly> [--threads <n>] [reference options]\n  ilspy-lite il <assembly> --method <0x06000001>\n  ilspy-lite decompile <assembly> --method <0x06000001> [--details]\n  ilspy-lite decompile <assembly> --type <Namespace.Type> [--details]\n\nDefault: readable C#-like view. --details: full metadata/IL evidence.\nReference options (decompile and scan): --reference <assembly>, --reference-dir <directory> (repeatable), --no-auto-references, --reference-version-policy <ilspy|exact|same-major> (default: ilspy).\nidentities prints one JSON line per image: its own identity and the reference identities it declares, read from that image alone.\nScan prints declared reference identities on stderr; --details also prints dependency selection and provenance. Reference facts can affect method status.\nDecompile exit: 0 complete/declaration, 2 partial IL fallback, 1 failure.\nScan: NDJSON status/reason per MethodDef; exit 0 means enumeration completed, not every body recovered. Scan exit 1 means request/input/output failure.\nOutput is C#-like; no promise of recompilable source.";

fn main() -> std::process::ExitCode {
    match run() {
        Ok(code) => std::process::ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut style = OutputStyle::Readable;
    let mut references = Vec::new();
    let mut reference_dirs = Vec::new();
    let mut version_policy = ReferenceVersionPolicy::IlSpy;
    let mut scan_threads = 1usize;
    let command = args.first().and_then(|arg| arg.to_str()).map(str::to_owned);
    let mut automatic = matches!(command.as_deref(), Some("decompile" | "scan"));
    let option_start = match command.as_deref() {
        Some("decompile") if args.len() >= 4 => Some(4),
        Some("scan") if args.len() >= 3 => Some(2),
        _ => None,
    };
    if let Some(mut index) = option_start {
        while index < args.len() {
            if args[index] == "--details" {
                style = OutputStyle::Details;
            } else if args[index] == "--no-auto-references" {
                automatic = false;
            } else if args[index] == "--reference-version-policy" {
                index += 1;
                version_policy = match args.get(index).and_then(|s| s.to_str()) {
                    Some("exact") => ReferenceVersionPolicy::Exact,
                    Some("same-major") => ReferenceVersionPolicy::SameMajor,
                    Some("ilspy") => ReferenceVersionPolicy::IlSpy,
                    _ => {
                        return Err(
                            "--reference-version-policy needs ilspy, exact or same-major".into(),
                        );
                    }
                };
            } else if args[index] == "--threads" {
                index += 1;
                scan_threads = args
                    .get(index)
                    .and_then(|value| value.to_str())
                    .and_then(|value| value.parse().ok())
                    .filter(|threads: &usize| *threads >= 1)
                    .ok_or("--threads needs a positive integer")?;
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
        let retained = if command.as_deref() == Some("scan") {
            2
        } else {
            4
        };
        args.truncate(retained);
    }
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        writeln!(out, "{USAGE}")?;
        out.flush()?;
        return Ok(0);
    }
    if args.len() >= 2 && args[0] == "identities" {
        for path in &args[1..] {
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take(512 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            let record = match ilspy_lite::image_identity(&bytes) {
                Ok(value) => serde_json::json!({
                    "path": path.to_string_lossy(),
                    "identity": value.identity.to_string(),
                    "references": value
                        .references
                        .iter()
                        .map(|reference| reference.to_string())
                        .collect::<Vec<_>>(),
                }),
                Err(error) => serde_json::json!({
                    "path": path.to_string_lossy(),
                    "error": error.to_string(),
                }),
            };
            serde_json::to_writer(&mut out, &record)?;
            writeln!(out)?;
        }
        out.flush()?;
        return Ok(0);
    }
    let valid = (args.len() == 2 && (args[0] == "list" || args[0] == "scan"))
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
    if args[0] == "scan" {
        let wants_context = automatic || !references.is_empty() || !reference_dirs.is_empty();
        let context = if wants_context {
            let context = reference_context(
                Path::new(&args[1]),
                &bytes,
                automatic,
                references.clone(),
                reference_dirs.clone(),
                version_policy,
            )?;
            let declared = context.references().len();
            let failed = context.reference_failures().len();
            // Selection is recorded before the scan so the audit trail exists even
            // when every method still falls back.
            eprintln!(
                "references={declared} resolved={} failed={failed}",
                declared - failed
            );
            for reference in context.references() {
                eprintln!("reference=0x{:08X} {}", reference.token, reference.identity);
            }
            for warning in dependency_warnings(&context) {
                eprintln!("dependency warning: {warning}");
            }
            if style == OutputStyle::Details {
                eprint!("{}", context.dependency_report()?);
            }
            Some(context)
        } else {
            None
        };
        let summary = if scan_threads > 1 {
            // Each worker rebuilds the reference context so fact caches stay
            // thread-local; the emitted records are identical to a serial scan.
            let assembly = args[1].clone();
            let refs = references.clone();
            let dirs = reference_dirs.clone();
            let factory = || -> Result<Option<DependencyContext>> {
                reference_context(
                    Path::new(&assembly),
                    &bytes,
                    automatic,
                    refs.clone(),
                    dirs.clone(),
                    version_policy,
                )
                .map(Some)
            };
            let factory_ref: Option<&(dyn Fn() -> Result<Option<DependencyContext>> + Sync)> =
                if wants_context { Some(&factory) } else { None };
            ilspy_lite::scan_methods_parallel(
                &bytes,
                scan_threads,
                factory_ref,
                |token, status, reason| write_scan_record(&mut out, token, status, reason),
            )?
        } else {
            ilspy_lite::scan_methods_with_context(
                &bytes,
                context.as_ref(),
                |token, status, reason| write_scan_record(&mut out, token, status, reason),
            )?
        };
        out.flush()?;
        eprintln!(
            "decompiled={} declarations={} partial={} failed={}",
            summary.decompiled, summary.declarations, summary.partial, summary.failed
        );
        // A completed scan can contain failed methods; request/I/O failure is exit 1.
        return Ok(0);
    }
    let mut reference_warnings = Vec::new();
    let context = if !automatic && references.is_empty() && reference_dirs.is_empty() {
        None
    } else {
        let context = reference_context(
            Path::new(&args[1]),
            &bytes,
            automatic,
            references,
            reference_dirs,
            version_policy,
        )?;
        reference_warnings.extend(dependency_warnings(&context));
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
        writeln!(out, "\n// {warning}")?;
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

/// Build the explicit reference set for one image. The caller names every input;
/// nothing is discovered from the filesystem beyond the requested directories.
fn write_scan_record(
    out: &mut impl Write,
    token: u32,
    status: ilspy_lite::Status,
    reason: Option<String>,
) -> Result<()> {
    serde_json::to_writer(
        &mut *out,
        &serde_json::json!({
            "token": format!("0x{token:08X}"),
            "status": format!("{status:?}"),
            "reason": reason,
        }),
    )?;
    writeln!(out)?;
    Ok(())
}

fn reference_context(
    assembly: &Path,
    bytes: &[u8],
    automatic: bool,
    references: Vec<PathBuf>,
    reference_dirs: Vec<PathBuf>,
    version_policy: ReferenceVersionPolicy,
) -> Result<DependencyContext> {
    let mut context = DependencyContext::new(bytes.to_vec())?;
    let environment = if automatic {
        ilspy_lite::discovery_paths::environment_from_host()
    } else {
        ilspy_lite::discovery_paths::DiscoveryEnvironment::default()
    };
    context.discover_references(
        assembly,
        &ReferenceSearchOptions {
            automatic,
            files: references,
            directories: reference_dirs,
            environment,
            version_policy,
            ..ReferenceSearchOptions::default()
        },
    )?;
    Ok(context)
}

/// Diagnostics that mean an input was refused rather than merely absent.
fn dependency_warnings(context: &DependencyContext) -> Vec<String> {
    context
        .search_diagnostics()
        .iter()
        .filter(|s| {
            s.starts_with("Dependency input rejected:")
                || s.starts_with("unsafe assembly filename:")
                || s.starts_with("reference directory unavailable:")
        })
        .cloned()
        .collect()
}
