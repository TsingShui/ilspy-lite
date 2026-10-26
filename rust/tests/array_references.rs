// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/array-reference-release.bin"),
    include_bytes!("fixtures/array-reference-debug.bin"),
];

#[test]
fn scan_and_method_views_use_identical_vector_admission() {
    for config in ["release", "debug"] {
        let path = format!(
            "{}/tests/fixtures/array-reference-{config}.bin",
            env!("CARGO_MANIFEST_DIR")
        );
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
            .args(["scan", &path])
            .output()
            .unwrap();
        assert!(output.status.success());
        let records: Vec<serde_json::Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let bytes = std::fs::read(path).unwrap();
        let methods = list_methods(&bytes).unwrap();
        assert_eq!(records.len(), methods.len());
        for (record, method) in records.iter().zip(methods) {
            assert_eq!(record["token"], format!("0x{:08X}", method.token));
            assert_eq!(
                record["status"],
                format!("{:?}", decompile(&bytes, method.token).unwrap().status)
            );
        }
    }
}

#[test]
fn different_vector_element_types_are_not_merged() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "LocalIdentity")
            .unwrap();
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md.tables().method_def(method.token & 0xffffff).unwrap();
        let signature = md.blobs().get(row.signature).unwrap();
        assert_eq!(signature, &[0, 1, 0x1d, 5, 0x1d, 5]);
        let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut patched = bytes.to_vec();
        patched[offset + 3] = 4; // sbyte[] return, byte[] parameter
        let result = decompile(&patched, method.token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring). There
// are no counts; a promotion must drop its row and a regression must add one.
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "ArrayReferenceCases.Ops",
        "UnsupportedCovariance",
        "incompatible value/receiver types",
    ),
    (
        "ArrayReferenceCases.Ops",
        "UnsupportedRead",
        "unsupported opcode ldelem.ref",
    ),
    (
        "ArrayReferenceCases.Ops",
        "UnsupportedStore",
        "unsupported opcode stelem.ref",
    ),
    (
        "ArrayReferenceCases.Ops",
        "UnsupportedAllocation",
        "array element type needs IL fallback",
    ),
    (
        "ArrayReferenceCases.Ops",
        "UnsupportedValue",
        "unsupported opcode ldelem",
    ),
    (
        "ArrayReferenceCases.Ops",
        "UnsupportedLength",
        "expected int[] or uint[] evaluation-stack value",
    ),
];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}
