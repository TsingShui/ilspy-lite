// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, decompile_with_style, list_methods};

#[test]
fn many_missing_dependencies_use_one_default_summary_but_keep_all_detailed_identities() {
    let bytes = include_bytes!("fixtures/release.bin");
    let context = DependencyContext::new(bytes.to_vec()).unwrap();
    assert!(context.references().len() > 3);
    let token = list_methods(bytes).unwrap().first().unwrap().token;
    let readable = context.decompile(token, OutputStyle::Readable).unwrap();
    let original = decompile_with_style(bytes, token, OutputStyle::Readable).unwrap();
    assert_eq!(readable.status, original.status);
    assert!(readable.text.starts_with(&original.text));
    // The readable view is source-shaped; dependency provenance is evidence.
    assert!(
        !readable.text.contains("// Dependency"),
        "{}",
        readable.text
    );
    assert!(
        !readable.text.contains("// Dependencies"),
        "{}",
        readable.text
    );
    let details = context.decompile(token, OutputStyle::Details).unwrap();
    for reference in context.references() {
        assert!(details.text.contains(&format!(
            ".dependency 0x{:08X} status=missing",
            reference.token
        )));
        assert!(details.text.contains(&reference.identity.to_string()));
    }
}

#[test]
fn reference_input_is_explicitly_a_contract_not_an_executable_implementation() {
    let bytes = include_bytes!("fixtures/binding-contract-release.bin");
    let context = DependencyContext::new(bytes.to_vec()).unwrap();
    let token = list_methods(bytes).unwrap().first().unwrap().token;
    let readable = context.decompile(token, OutputStyle::Readable).unwrap();
    assert!(
        readable
            .text
            .starts_with("// Reference assembly API contract;"),
        "{}",
        readable.text
    );
    assert_eq!(
        readable.status,
        decompile_with_style(bytes, token, OutputStyle::Readable)
            .unwrap()
            .status
    );
    let details = context.decompile(token, OutputStyle::Details).unwrap();
    assert!(
        details
            .text
            .contains(".dependency-main kind=ReferenceContract"),
        "{}",
        details.text
    );
}

#[test]
fn dependency_warning_names_cannot_inject_uncommented_source_lines() {
    // The readable view carries no dependency text, so a crafted reference name
    // cannot reach source lines there. The invariant is kept for the details view,
    // which is the only place the identity is printed.
    let bytes = include_bytes!("fixtures/external-release.bin");
    let context = DependencyContext::new(bytes.to_vec()).unwrap();
    let token = list_methods(bytes).unwrap().first().unwrap().token;
    let readable = context.decompile(token, OutputStyle::Readable).unwrap();
    assert!(!readable.text.contains("Dependency"), "{}", readable.text);
    let details = context.decompile(token, OutputStyle::Details).unwrap();
    for line in details.text.lines() {
        if line.contains(".dependency ") {
            assert!(line.starts_with(".dependency 0x"), "{line}");
        }
    }
}
