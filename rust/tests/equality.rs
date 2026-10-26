// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/equality-release.bin"),
    include_bytes!("fixtures/equality-debug.bin"),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "EqualityCases.Ops" && m.simple_name == name)
        .unwrap()
        .token
}

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[(
    "EqualityCases.Ops",
    "UnsupportedExternal",
    "callvirt on an opaque external receiver needs dependency facts",
)];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}

#[test]
fn reference_equality_is_identity_comparison_without_dereference() {
    for bytes in IMAGES {
        for name in ["IsNull", "NotNull", "SameTyped", "BothNull"] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert!(
                result.text.contains("ReferenceEquals"),
                "{name}\n{}",
                result.text
            );
            assert!(!result.text.contains("=="), "{name}\n{}", result.text);
        }
    }
}
