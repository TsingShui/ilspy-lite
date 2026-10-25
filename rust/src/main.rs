// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Result, Status};
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
    let tokens = if args[2] == "--method" {
        let token = if let Some(s) = value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
        {
            u32::from_str_radix(s, 16)?
        } else {
            value.parse()?
        };
        vec![token]
    } else {
        let tokens: Vec<_> = ilspy_lite::list_methods(&bytes)?
            .into_iter()
            .filter(|m| {
                m.declaring_type == value || m.declaring_type.starts_with(&format!("{value}/"))
            })
            .map(|m| m.token)
            .collect();
        if tokens.is_empty() {
            return Err("no methods match the declaring type or its nested types".into());
        }
        tokens
    };
    if args[0] == "il" {
        write!(out, "{}", ilspy_lite::disassemble(&bytes, tokens[0])?)?;
        out.flush()?;
        return Ok(0);
    }
    let (mut done, mut declarations, mut partial, mut failed) = (0, 0, 0, 0);
    for token in tokens {
        match ilspy_lite::decompile(&bytes, token) {
            Ok(result) => {
                match result.status {
                    Status::Decompiled => done += 1,
                    Status::Declaration => declarations += 1,
                    Status::Partial => partial += 1,
                    Status::Failed => failed += 1,
                }
                writeln!(out, "{}", result.text)?;
            }
            Err(e) => {
                failed += 1;
                writeln!(
                    out,
                    "// FAILED token 0x{token:08X}: {}",
                    e.to_string().escape_default()
                )?;
            }
        }
    }
    out.flush()?;
    eprintln!("decompiled={done} declarations={declarations} partial={partial} failed={failed}");
    Ok(if failed > 0 {
        1
    } else if partial > 0 {
        2
    } else {
        0
    })
}
