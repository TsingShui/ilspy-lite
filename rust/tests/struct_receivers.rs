// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/struct-receiver-release.bin"),
    include_bytes!("fixtures/struct-receiver-debug.bin"),
];

fn method(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type.starts_with("StructReceiverCases.") && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn struct_value_arguments_are_copies_while_receivers_are_locations() {
    for bytes in IMAGES {
        let read = decompile(bytes, method(bytes, "Read")).unwrap();
        assert!(read.text.contains("ref "), "{}", read.text);
        let asm = decompile(bytes, method(bytes, "ArgumentWrite")).unwrap();
        assert!(asm.text.contains("a0"), "{}", asm.text);
    }
}

#[test]
fn scan_preserves_struct_receiver_admission() {
    for bytes in IMAGES {
        let mut records = Vec::new();
        ilspy_lite::scan_methods(bytes, |token, status, _| {
            records.push((token, status));
            Ok(())
        })
        .unwrap();
        let methods = list_methods(bytes).unwrap();
        assert_eq!(records.len(), methods.len());
        for (definition, (token, status)) in methods.iter().zip(records) {
            assert_eq!(definition.token, token);
            assert_eq!(decompile(bytes, token).unwrap().status, status);
        }
    }
}

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring). There
// are no counts; a promotion must drop its row and a regression must add one.
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "StructReceiverCases.Ops",
        "UnsupportedConstruct",
        "value-type receiver needs IL fallback",
    ),
    (
        "StructReceiverCases.Ops",
        "UnsupportedRefReturn",
        "byref return needs escape/lifetime analysis",
    ),
    (
        "StructReceiverCases.Ops",
        "UnsupportedArray",
        "operation needs a core TypeRef",
    ),
];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}
