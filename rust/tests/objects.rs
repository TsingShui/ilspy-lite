// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, list_methods};
use std::process::Command;

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/object-release.bin"),
    include_bytes!("fixtures/object-debug.bin"),
];

#[test]
fn unsupported_object_operations_retain_their_original_il() {
    for bytes in IMAGES {
        // The frozen table is the list: every row must still be a marked fallback
        // that carries its original IL, and a promotion removes its row here.
        for (owner, name, _) in EXPECTED_PARTIAL {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == *owner && m.simple_name == *name)
                .unwrap_or_else(|| panic!("fixture has no {owner}.{name}"));
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(
                result.text.contains("IL FALLBACK"),
                "{name}: {}",
                result.text
            );
            assert!(
                result.text.contains(".code-bytes"),
                "{name}: {}",
                result.text
            );
        }
    }
}

#[test]
fn object_cli_scan_and_method_output_have_identical_admission() {
    for config in ["release", "debug"] {
        let fixture = format!(
            "{}/tests/fixtures/object-{config}.bin",
            env!("CARGO_MANIFEST_DIR")
        );
        let output = Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
            .args(["scan", &fixture])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let records: Vec<serde_json::Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let bytes = std::fs::read(fixture).unwrap();
        let methods = list_methods(&bytes).unwrap();
        assert_eq!(records.len(), methods.len());
        for (record, method) in records.iter().zip(&methods) {
            let result = decompile(&bytes, method.token).unwrap();
            assert_eq!(record["token"], format!("0x{:08X}", method.token));
            assert_eq!(record["status"], format!("{:?}", result.status));
        }
    }
}

#[test]
fn integer_values_cannot_return_as_object_without_boxing() {
    let bytes = IMAGES[0];
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "Identity")
        .unwrap();
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method.token & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.code, [0x02, 0x2a]);
    let offset = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset] = 0x17; // ldc.i4.1; ret object
    let result = decompile(&patched, method.token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("IL FALLBACK"));
}

#[test]
fn object_values_cannot_be_integer_operands_or_typed_field_receivers() {
    let bytes = IMAGES[0];
    let methods = list_methods(bytes).unwrap();
    let method = methods
        .iter()
        .find(|m| m.simple_name == "ExchangeField")
        .unwrap();
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method.token & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    let offset = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    assert!(body.code.len() >= 4);
    let mut patched = bytes.to_vec();
    patched[offset..offset + 4].copy_from_slice(&[0x03, 0x03, 0x58, 0x2a]);
    let result = decompile(&patched, method.token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(
        result
            .text
            .contains("integer operation on non-integer value"),
        "{}",
        result.text
    );

    let method = methods
        .iter()
        .find(|m| m.simple_name == "ReadField")
        .unwrap();
    let row = md.tables().method_def(method.token & 0xffffff).unwrap();
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature.len(), 5);
    assert_eq!(&signature[..4], &[0, 1, 0x1c, 0x12]);
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    assert_eq!(bytes[offset - 1], 5);
    let mut patched = bytes.to_vec();
    patched[offset - 1] = 4;
    patched[offset + 3] = 0x1c;
    let result = decompile(&patched, method.token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(
        result.text.contains("incompatible value/receiver types"),
        "{}",
        result.text
    );
}

#[test]
fn integer_cannot_be_stored_into_an_object_static_field_without_boxing() {
    let bytes = IMAGES[0];
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "StaticExchange")
        .unwrap();
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method.token & 0xffffff).unwrap();
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature, [0, 1, 0x1c, 0x1c]);
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 3] = 0x08;
    let result = decompile(&patched, method.token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(
        result.text.contains("incompatible value/receiver types"),
        "{}",
        result.text
    );
}

#[test]
fn object_to_string_assignment_requires_an_explicit_cast() {
    let bytes = IMAGES[0];
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "Identity")
        .unwrap();
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method.token & 0xffffff).unwrap();
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature, [0, 1, 0x1c, 0x1c]);
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 2] = 0x0e; // string Identity(object)
    let result = decompile(&patched, method.token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("IL FALLBACK"));
}

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring). There
// are no counts; a promotion must drop its row and a regression must add one.
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[(
    "ObjectCases.Ops",
    "UnsupportedAllocation",
    "external allocation needs dependency facts; only local classes and local instantiations are admitted",
)];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}
