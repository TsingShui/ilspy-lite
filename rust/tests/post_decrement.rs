// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{
    OutputStyle, Status, decompile, decompile_type_with_style, decompile_with_style, list_methods,
};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/property-release.bin"),
    include_bytes!("fixtures/property-debug.bin"),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn adjacent_original_this_i4_decrement_compares_the_old_value_under_unchecked() {
    for bytes in IMAGES {
        let method = token(bytes, "Decrement");
        let actual = decompile_with_style(bytes, method, OutputStyle::Readable).unwrap();
        assert_eq!(actual.status, Status::Decompiled);
        assert!(
            actual
                .text
                .contains("return unchecked(this.Counter-- == 0);"),
            "{}",
            actual.text
        );
        let class =
            decompile_type_with_style(bytes, "PropertyCases.Cell", OutputStyle::Readable).unwrap();
        assert!(
            class
                .text
                .contains("return unchecked(this.Counter-- == 0);")
        );
        let details = decompile(bytes, method).unwrap();
        assert!(!details.text.contains("this.Counter--"));
        assert!(details.text.contains("unchecked("));
    }
}

#[test]
fn local_and_field_postfix_keep_old_snapshots_before_later_reads_and_effects() {
    let debug = IMAGES[1];
    let local =
        decompile_with_style(debug, token(debug, "LocalDecrement"), OutputStyle::Readable).unwrap();
    assert_eq!(local.status, Status::Decompiled);
    assert!(local.text.contains("unchecked(l0--)"), "{}", local.text);
    for bytes in IMAGES {
        for name in ["KeepOldAcrossEffect", "TwoReads"] {
            let actual =
                decompile_with_style(bytes, token(bytes, name), OutputStyle::Readable).unwrap();
            assert_eq!(actual.status, Status::Decompiled);
            assert!(
                actual.text.contains("= unchecked(this.Counter--);"),
                "{name}: {}",
                actual.text
            );
            assert!(
                !actual.text.contains("return unchecked(this.Counter--"),
                "old snapshot must stay before subsequent operations: {}",
                actual.text
            );
        }
    }
}

#[test]
fn effectful_or_different_receivers_checks_and_extra_reads_keep_explicit_snapshots() {
    for bytes in IMAGES {
        for name in [
            "CheckedDecrement",
            "OtherDecrement",
            "EffectfulReceiver",
            "EffectBetweenReadAndWrite",
            "DifferentLocation",
            "UnsignedDecrement",
        ] {
            let actual =
                decompile_with_style(bytes, token(bytes, name), OutputStyle::Readable).unwrap();
            assert_eq!(actual.status, Status::Decompiled, "{name}: {}", actual.text);
            assert!(!actual.text.contains("--"), "{name}: {}", actual.text);
            if name == "CheckedDecrement" {
                assert!(actual.text.contains("checked("), "{}", actual.text);
                assert!(!actual.text.contains("unchecked("));
            }
        }
    }
}
