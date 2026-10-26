// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/new-object-release.bin"),
    include_bytes!("fixtures/new-object-debug.bin"),
];

fn token(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap()
        .token
}

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring).
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "NewObjectCases.Ops",
        "UnsupportedExternal",
        "external allocation needs dependency facts; only local classes and local instantiations are admitted",
    ),
    (
        "NewObjectCases.Ops",
        "UnsupportedStruct",
        "value-type receiver needs IL fallback",
    ),
];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}

#[test]
fn allocation_is_one_fresh_identity_then_exact_constructor_in_order() {
    for bytes in IMAGES {
        for (owner, name) in [
            ("NewObjectCases.Node", ".ctor"),
            ("NewObjectCases.Ops", "Empty"),
            ("NewObjectCases.Ops", "Tag"),
            ("NewObjectCases.Ops", "Shared"),
        ] {
            let result = decompile(bytes, token(bytes, owner, name)).unwrap();
            if name == ".ctor" {
                continue;
            }
            assert!(
                result.text.contains("__cil_newobj_"),
                "{owner}.{name}\n{}",
                result.text
            );
        }
        let tag = decompile(bytes, token(bytes, "NewObjectCases.Ops", "Tag")).unwrap();
        let value = tag.text.find("v1").unwrap();
        let argument = tag.text.find("__cil_newobj_").unwrap();
        assert!(value < argument, "{}", tag.text);
    }
}

#[test]
fn unproved_external_and_value_allocations_stay_gated() {
    for bytes in IMAGES {
        for name in ["UnsupportedExternal", "UnsupportedStruct"] {
            let result = decompile(bytes, token(bytes, "NewObjectCases.Ops", name)).unwrap();
            assert!(
                result.text.contains("IL FALLBACK"),
                "{name}\n{}",
                result.text
            );
        }
    }
}
