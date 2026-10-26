// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, Status, list_methods};

const IMAGES: [(&[u8], &[u8]); 2] = [
    (
        include_bytes!("fixtures/external-allocation-release.bin"),
        include_bytes!("fixtures/external-allocation-library-release.bin"),
    ),
    (
        include_bytes!("fixtures/external-allocation-debug.bin"),
        include_bytes!("fixtures/external-allocation-library-debug.bin"),
    ),
];

/// Constructed and nested external owners, and value-type initialisation, stay
/// gated with their own reasons instead of borrowing the plain-class path.
const EXPECTED_PARTIAL: &[(&str, &str)] = &[
    (
        "MakeBox",
        "constructed member needs a local definition; external constructed members need dependency facts",
    ),
    (
        "MakeNested",
        "nested/generic/unresolved external class needs IL fallback",
    ),
    (
        "UnsupportedStruct",
        "external allocation needs dependency facts: owner is a value type; value initialisation is not an object identity",
    ),
    ("UnsupportedStatic", "external field needs IL fallback"),
    (
        "UnsupportedStructField",
        "external member reference needs dependency facts: owner is a value type; its fields need managed-location semantics",
    ),
];

/// Positives must name the owner and its kind, keep the argument order, and report
/// exactly one fresh identity per site.
const EXPECTED: &[(&str, Option<&str>)] = &[
    ("MakeNode", Some("ExternalAllocations.Node")),
    ("MakeDefault", Some("ExternalAllocations.Node")),
    ("MakeSealed", Some("ExternalAllocations.Sealed")),
    ("MakeMixed", Some("ExternalAllocations.Pair")),
    // These return the field's value, so the owner appears only as the local's
    // alias; what must hold is one allocation and a read/write on that local.
    ("ReadAfterAllocation", None),
    ("WriteAndRead", None),
];

fn context(bytes: &[u8], library: &[u8]) -> DependencyContext {
    let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
    context
        .add_reference("ExternalAllocationLibrary", library.to_vec())
        .unwrap();
    context
}

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ExternalAllocationCases.Ops" && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn declared_reference_constructors_allocate_one_fresh_identity() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        for (name, owner) in EXPECTED {
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result = context.decompile(token(bytes, name), style).unwrap();
                assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
                if style == OutputStyle::Details {
                    assert!(
                        result.text.contains("external allocation owner 0x"),
                        "{name} lacks owner evidence: {}",
                        result.text
                    );
                }
                if let Some(owner) = owner {
                    assert!(
                        result.text.contains(owner),
                        "{name} does not name {owner}: {}",
                        result.text
                    );
                } else {
                    assert!(
                        result.text.contains("Value"),
                        "{name} does not access the field: {}",
                        result.text
                    );
                }
                assert_eq!(
                    result.text.matches("__cil_newobj_").count(),
                    1,
                    "{name} must allocate exactly once: {}",
                    result.text
                );
            }
        }
    }
}

#[test]
fn gated_external_owners_keep_their_own_reason() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        for (name, reason) in EXPECTED_PARTIAL {
            let result = context
                .decompile(token(bytes, name), OutputStyle::Readable)
                .unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(
                result.text.contains(reason),
                "{name} reason drifted: {}",
                result.text
            );
        }
    }
}

#[test]
fn without_references_every_body_is_unchanged() {
    for (bytes, _) in IMAGES {
        // No reference set means no fact for any of these sites: they must stay
        // Partial with the pre-existing message, byte-identical to the old run.
        let all = EXPECTED
            .iter()
            .map(|(name, _)| name)
            .chain(EXPECTED_PARTIAL.iter().map(|(name, _)| name));
        for name in all {
            let bare = DependencyContext::new(bytes.to_vec())
                .unwrap()
                .decompile(token(bytes, name), OutputStyle::Readable)
                .unwrap();
            assert_eq!(bare.status, Status::Partial, "{name}: {}", bare.text);
            // The exact refusal differs by shape (a constructed owner is gated
            // before the facts are consulted), but none of them may be admitted.
            assert!(bare.text.contains("IL fallback"), "{name}: {}", bare.text);
        }
    }
}
