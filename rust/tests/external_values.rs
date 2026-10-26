// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, Status, list_methods};

const IMAGES: [(&[u8], &[u8]); 2] = [
    (
        include_bytes!("fixtures/external-value-release.bin"),
        include_bytes!("fixtures/external-value-library-release.bin"),
    ),
    (
        include_bytes!("fixtures/external-value-debug.bin"),
        include_bytes!("fixtures/external-value-library-debug.bin"),
    ),
];

/// A byref-like definition may never be copied, and a field read needs the
/// instantiated field type; both stay gated with their own reasons.
const EXPECTED_PARTIAL: &[(&str, &str, &str)] = &[
    (
        "ExternalValueCases.Ops",
        "UnsupportedRefStruct",
        "byref-like struct copies need location/lifetime semantics",
    ),
    (
        "ExternalValueCases.Ops",
        "UnsupportedField",
        "external member reference needs dependency facts",
    ),
];

fn context(bytes: &[u8], library: &[u8]) -> DependencyContext {
    let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
    context
        .add_reference("ExternalValueLibrary", library.to_vec())
        .unwrap();
    context
}

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ExternalValueCases.Ops" && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn declared_reference_value_types_are_copied_whole() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        for name in ["Identity", "Pass", "Keep", "Wrap", "Level", "Ignore"] {
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
fn the_same_fixture_without_the_declared_reference_stays_gated() {
    for (bytes, _) in IMAGES {
        for name in ["Identity", "Pass", "Keep", "Wrap"] {
            let result = ilspy_lite::decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
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
fn the_negative_surface_keeps_its_stated_blocker() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        let methods = list_methods(bytes).unwrap();
        let expected: Vec<_> = EXPECTED_PARTIAL
            .iter()
            .map(|(owner, name, reason)| (owner.to_string(), name.to_string(), reason.to_string()))
            .collect();
        let mut seen = Vec::new();
        for method in &methods {
            if method.declaring_type != "ExternalValueCases.Ops" {
                continue;
            }
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
        assert_eq!(seen, expected);
    }
}

#[test]
fn scan_with_the_same_context_reports_the_same_status_for_every_method() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        let mut scanned = Vec::new();
        ilspy_lite::scan_methods_with_context(bytes, Some(&context), |token, status, reason| {
            scanned.push((token, status, reason));
            Ok(())
        })
        .unwrap();
        for (method, (token, status, _)) in list_methods(bytes).unwrap().iter().zip(&scanned) {
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
    }
}
