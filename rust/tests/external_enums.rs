// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, Status, list_methods};

const IMAGES: [(&[u8], &[u8]); 2] = [
    (
        include_bytes!("fixtures/external-enum-release.bin"),
        include_bytes!("fixtures/external-enum-library-release.bin"),
    ),
    (
        include_bytes!("fixtures/external-enum-debug.bin"),
        include_bytes!("fixtures/external-enum-library-debug.bin"),
    ),
];

/// This fixture has no negative surface left: the external struct that the enum
/// milestone could not admit is admitted by the external value-type slice. The
/// negatives that remain are byref-like definitions and field access, covered by
/// external_values.rs and generic_values.rs.
const EXPECTED_PARTIAL: &[(&str, &str, &str)] = &[];

fn context(bytes: &[u8], library: &[u8]) -> DependencyContext {
    let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
    context
        .add_reference("ExternalEnumLibrary", library.to_vec())
        .unwrap();
    context
}

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ExternalEnumCases.Ops" && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn declared_reference_storage_admits_external_enums() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        for name in [
            "ReadLevel",
            "Next",
            "IsHigh",
            "ReadWide",
            "ReadPlain",
            "Widen",
            "Copy",
            "UnsupportedClass",
            "UnsupportedStruct",
        ] {
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result = context.decompile(token(bytes, name), style).unwrap();
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
fn the_same_fixture_without_the_declared_reference_stays_byte_only() {
    for (bytes, _) in IMAGES {
        for name in ["ReadLevel", "Next", "IsHigh", "ReadPlain", "Widen", "Copy"] {
            let result = ilspy_lite::decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(
                result.status,
                Status::Partial,
                "{name} recovered without any declared reference: {}",
                result.text
            );
            assert!(
                result
                    .text
                    .contains("enum storage requires an exact local definition"),
                "{name}: {}",
                result.text
            );
        }
    }
}

#[test]
fn external_enum_storage_is_named_and_sized_in_the_recovered_source() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        let readable = context
            .decompile(token(bytes, "ReadLevel"), OutputStyle::Readable)
            .unwrap();
        assert!(
            readable.text.contains("ExternalEnums.Level"),
            "{}",
            readable.text
        );
        let copy = context
            .decompile(token(bytes, "Copy"), OutputStyle::Readable)
            .unwrap();
        assert!(!copy.text.contains("IL fallback"), "{}", copy.text);
    }
}

#[test]
fn the_negative_surface_keeps_its_stated_blocker() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        let methods = list_methods(bytes).unwrap();
        let mut seen = Vec::new();
        for method in &methods {
            let result = context
                .decompile(method.token, OutputStyle::Details)
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
fn scan_with_the_same_context_reports_the_same_status_for_every_method() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        let mut scanned = Vec::new();
        let summary = ilspy_lite::scan_methods_with_context(
            bytes,
            Some(&context),
            |token, status, reason| {
                scanned.push((token, status, reason));
                Ok(())
            },
        )
        .unwrap();
        let methods = list_methods(bytes).unwrap();
        assert_eq!(scanned.len(), methods.len());
        for (method, (token, status, _)) in methods.iter().zip(&scanned) {
            assert_eq!(*token, method.token);
            let alone = context
                .decompile(method.token, OutputStyle::Details)
                .unwrap();
            assert_eq!(
                *status, alone.status,
                "{}.{}: scan and single-method decompilation disagree",
                method.declaring_type, method.simple_name
            );
        }
        assert_eq!(
            summary.partial, 0,
            "every method in this fixture is admitted"
        );
        assert_eq!(summary.failed, 0);
    }
}
