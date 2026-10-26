// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/generic-value-release.bin"),
    include_bytes!("fixtures/generic-value-debug.bin"),
];

/// Reading a field of a constructed value needs the instantiated field type,
/// which is a separate capability.
const EXPECTED_PARTIAL: &[(&str, &str, &str)] = &[
    (
        "GenericValueCases.Ops",
        "Sum",
        "constructed member parent is not a class instantiation",
    ),
    (
        "GenericValueCases.Ops",
        "Peek",
        "constructed member parent is not a class instantiation",
    ),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "GenericValueCases.Ops" && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn constructed_value_plumbing_is_recovered_in_both_views() {
    for bytes in IMAGES {
        for name in ["Identity", "Pass", "Boxed", "Wrapped", "Ignore"] {
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result =
                    ilspy_lite::decompile_with_style(bytes, token(bytes, name), style).unwrap();
                assert_eq!(
                    result.status,
                    Status::Decompiled,
                    "{name} ({style:?}): {}",
                    result.text
                );
            }
        }
    }
}

#[test]
fn the_negative_surface_keeps_its_stated_blocker() {
    for bytes in IMAGES {
        let mut seen = Vec::new();
        for method in list_methods(bytes).unwrap() {
            let result =
                ilspy_lite::decompile_with_style(bytes, method.token, OutputStyle::Details)
                    .unwrap();
            if result.status == Status::Partial {
                let reason = result
                    .text
                    .lines()
                    .find_map(|line| line.strip_prefix("// IL FALLBACK: "))
                    .unwrap_or("<no reason>");
                seen.push((
                    method.declaring_type.clone(),
                    method.simple_name.clone(),
                    reason.to_string(),
                ));
            }
        }
        let expected: Vec<_> = EXPECTED_PARTIAL
            .iter()
            .map(|(owner, name, reason)| (owner.to_string(), name.to_string(), reason.to_string()))
            .collect();
        assert_eq!(seen, expected);
    }
}

#[test]
fn an_open_value_definition_is_not_copied_as_a_closed_value() {
    // The definition itself carries a generic parameter field; only an
    // instantiation names every value it holds. Asking for the open definition as
    // a value type identity must not silently produce a closed copy.
    for bytes in IMAGES {
        let image =
            ilspy_lite::decompile_with_style(bytes, token(bytes, "Identity"), OutputStyle::Details)
                .unwrap();
        assert!(
            image.text.contains("Pair`1<int32>"),
            "identity must carry the instantiation: {}",
            image.text
        );
    }
}
