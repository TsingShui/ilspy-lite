// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/generic-declaring-release.bin"),
    include_bytes!("fixtures/generic-declaring-debug.bin"),
];

/// A base field read from a derived generic class needs the constructed base to be
/// named in the access; the definition name alone is an open generic, so the shape
/// stays gated with its own reason instead of printing an illegal cast.
const EXPECTED_PARTIAL: &[(&str, &str, &str)] = &[(
    "GenericDeclaringCases.Derived`1",
    "Score",
    "base field access needs the constructed owner name",
)];

fn token(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap_or_else(|| panic!("fixture has no {owner}.{name}"))
        .token
}

#[test]
fn open_generic_declaring_types_keep_their_members() {
    for bytes in IMAGES {
        for (owner, name) in [
            ("GenericDeclaringCases.Holder`1", ".ctor"),
            ("GenericDeclaringCases.Holder`1", "Score"),
            ("GenericDeclaringCases.Holder`1", "Self"),
            ("GenericDeclaringCases.Derived`1", ".ctor"),
            ("GenericDeclaringCases.Ops", "Identity"),
            ("GenericDeclaringCases.Ops", "Field"),
            ("GenericDeclaringCases.Ops", "UnsupportedContent"),
            ("GenericDeclaringCases.Ops", "UnsupportedStruct"),
            ("GenericDeclaringCases.Ops", "CallScore"),
            ("GenericDeclaringCases.Ops", "MakeHolder"),
            ("GenericDeclaringCases.Ops", "SelfCall"),
            ("GenericDeclaringCases.Ops", "MakeDerived"),
            ("GenericDeclaringCases.Ops", "BoxInt"),
            ("GenericDeclaringCases.Ops", "BoxEnum"),
            ("GenericDeclaringCases.Ops", "UnboxInt"),
        ] {
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result =
                    ilspy_lite::decompile_with_style(bytes, token(bytes, owner, name), style)
                        .unwrap();
                assert_eq!(
                    result.status,
                    Status::Decompiled,
                    "{owner}.{name} ({style:?}): {}",
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
fn substituting_the_argument_is_not_the_same_as_leaving_the_parameter_open() {
    for bytes in IMAGES {
        let substituted = ilspy_lite::decompile_with_style(
            bytes,
            token(bytes, "GenericDeclaringCases.Ops", "UnsupportedContent"),
            OutputStyle::Readable,
        )
        .unwrap();
        assert!(
            substituted.text.contains("holder.Value"),
            "{}",
            substituted.text
        );
        assert!(
            !substituted.text.contains("__cil_generic"),
            "the field type must be the supplied argument: {}",
            substituted.text
        );
    }
}
