// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/cast-release.bin"),
    include_bytes!("fixtures/cast-debug.bin"),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "CastCases.Ops" && m.simple_name == name)
        .unwrap()
        .token
}

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "CastCases.Ops",
        "UnsupportedString",
        "cast needs a local class definition; external/interface casts need more facts",
    ),
    (
        "CastCases.Ops",
        "UnsupportedInterface",
        "callvirt on an opaque external receiver needs dependency facts",
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
fn casts_keep_the_runtime_check_instead_of_being_erased() {
    for bytes in IMAGES {
        let downcast = decompile(bytes, token(bytes, "Downcast")).unwrap();
        assert!(
            downcast.text.contains("CastCases.Derived"),
            "{}",
            downcast.text
        );
        let as_form = decompile(bytes, token(bytes, "As")).unwrap();
        assert!(as_form.text.contains(" as "), "{}", as_form.text);
        assert!(!as_form.text.contains("IL FALLBACK"), "{}", as_form.text);
    }
}
