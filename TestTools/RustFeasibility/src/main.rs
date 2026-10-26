// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{MethodBody, MethodSig, Names, PeImage, Rid, tables::MethodDefRow};
use std::collections::BTreeMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or("assembly path required")?;
    let dump_dir = std::env::args().nth(2).filter(|s| s != "-");
    let filters: Vec<String> = std::env::args()
        .nth(3)
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let data = std::fs::read(&path)?;
    let start = std::time::Instant::now();
    let image = PeImage::parse(&data)?;
    let metadata = image.metadata()?;
    let names = Names::new(&metadata);
    let mut bodies = BTreeMap::new();
    let mut decode_errors = 0usize;
    let mut methods = 0usize;
    let mut signatures = 0usize;
    let mut instructions = 0usize;
    for (rid, row) in metadata.tables().iter::<MethodDefRow>() {
        methods += 1;
        let row = row?;
        MethodSig::parse(metadata.blobs().get(row.signature)?)?;
        signatures += 1;
        match MethodBody::from_image(&image, &row) {
            Ok(Some(body)) => {
                let mut count = 0usize;
                for ins in body.instructions() {
                    ins?;
                    count += 1;
                }
                body.validate_handlers()?;
                if let Some(token) = body.local_var_sig {
                    let r = metadata.tables().stand_alone_sig(token.rid())?;
                    cildec::LocalVarSig::parse(metadata.blobs().get(r.signature)?)?;
                }
                instructions += count;
                bodies.insert(
                    0x06000000 | rid,
                    (
                        body.code_size(),
                        body.exception_handlers.len(),
                        names.method_def_full(Rid::new(rid))?,
                    ),
                );
            }
            Ok(None) => {}
            Err(e) => {
                decode_errors += 1;
                eprintln!("decode-error {rid}: {e}");
            }
        }
    }
    println!(
        "cildec methods={methods} signatures={signatures} bodies={} instructions={instructions} decode_errors={decode_errors} diagnostics={} elapsed_ms={:.1}",
        bodies.len(),
        image.diagnostics().len() + metadata.diagnostics().len(),
        start.elapsed().as_secs_f64() * 1000.0
    );
    if !filters.is_empty() {
        bodies.retain(|_, (_, _, name)| filters.iter().any(|f| name.contains(f)));
    }
    println!("selected_bodies={}", bodies.len());
    if bodies.is_empty() {
        return Err("no method bodies selected".into());
    }
    if std::env::var_os("PROBE_DECODE_ONLY").is_some() {
        if decode_errors != 0 {
            return Err("method body decoding failed".into());
        }
        return Ok(());
    }
    let start = std::time::Instant::now();
    let lenient = std::env::var_os("PROBE_LENIENT").is_some();
    let assembly = if lenient {
        dotscope::CilObject::from_path_with_validation(
            &path,
            dotscope::ValidationConfig {
                lenient: true,
                ..Default::default()
            },
        )?
    } else {
        dotscope::CilObject::from_path(&path)?
    };
    let diagnostics = assembly.diagnostics().count();
    eprintln!("dotscope lenient={lenient} diagnostics={diagnostics}");
    for diagnostic in assembly.diagnostics().iter() {
        eprintln!("diagnostic {diagnostic:?}");
    }
    println!(
        "dotscope methods={} parse_ms={:.1}",
        assembly.methods().len(),
        start.elapsed().as_secs_f64() * 1000.0
    );
    let mut built = 0usize;
    let mut failed = 0usize;
    let mut invalid = 0usize;
    let mut mismatches = 0usize;
    let mut missing = 0usize;
    for (token, (size, eh, name)) in &bodies {
        let token = dotscope::metadata::token::Token::new(*token);
        let Some(entry) = assembly.methods().get(&token) else {
            missing += 1;
            continue;
        };
        let method = entry.value();
        match method.body.get() {
            Some(body) if body.size_code == *size && body.exception_handlers.len() == *eh => {}
            _ => {
                mismatches += 1;
                eprintln!("body-mismatch {name}");
            }
        }
        match method.ssa(&assembly) {
            Ok(ssa) => {
                built += 1;
                if let Err(e) = ssa.validate() {
                    invalid += 1;
                    eprintln!("invalid-ssa {name}: {e:?}");
                }
                if ssa.exception_handlers().len() != *eh {
                    mismatches += 1;
                    eprintln!("eh-mismatch {name}");
                }
                if let Some(dir) = &dump_dir {
                    std::fs::create_dir_all(dir)?;
                    std::fs::write(
                        format!("{dir}/{:08X}.ssa.txt", token.value()),
                        format!("// {name}\n{ssa}"),
                    )?;
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("ssa-error {name}: {e}");
            }
        }
    }
    println!(
        "dotscope built={built} failed={failed} invalid={invalid} mismatches={mismatches} missing={missing} parse_plus_ssa_ms={:.1}",
        start.elapsed().as_secs_f64() * 1000.0
    );
    if decode_errors + failed + invalid + mismatches + missing + diagnostics != 0 {
        return Err("probe failed one or more gates".into());
    }
    Ok(())
}
