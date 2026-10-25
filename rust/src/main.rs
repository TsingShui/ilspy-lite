// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Result, Summary};
use std::io::{Read, Write};

const USAGE: &str = "Usage:\n  ilspy-lite list <assembly>\n  ilspy-lite il <assembly> --method <0x06000001>\n  ilspy-lite decompile <assembly> --method <0x06000001>\n  ilspy-lite decompile <assembly> --type <Namespace.Type>\n\nExit: 0 complete/declaration, 2 partial IL fallback, 1 failure.\nOutput is C#-like; no promise of recompilable source.";

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
    let args: Vec<_> = std::env::args_os().skip(1).collect();
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
    let value = args[3].to_str().ok_or("selector must be UTF-8")?;
    let summary = if args[2] == "--type" {
        let result = ilspy_lite::decompile_type(&bytes, value)?;
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
        match ilspy_lite::decompile(&bytes, token) {
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
