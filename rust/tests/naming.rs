// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: &[&[u8]] = &[
    include_bytes!("fixtures/naming-release.bin"),
    include_bytes!("fixtures/naming-debug.bin"),
];
const CLOSURE: &str = "NamingCases.Factory/<>c__DisplayClass0_0";

#[test]
fn generated_owner_name_does_not_block_an_admitted_object_base_constructor() {
    for &bytes in IMAGES {
        let ctor = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == CLOSURE && m.simple_name == ".ctor")
            .expect("owned closure constructor");
        for style in [OutputStyle::Readable, OutputStyle::Details] {
            let result = decompile_with_style(bytes, ctor.token, style).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        }
    }
}
