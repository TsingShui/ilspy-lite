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
    assert!(
        readable.text.contains(&format!(
            "// Dependencies unresolved: {};",
            context.references().len()
        )),
        "{}",
        readable.text
    );
    assert!(!readable.text.contains("// Dependency unresolved:"));
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
    let bytes = include_bytes!("fixtures/external-release.bin");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let (rid, row) = md
        .tables()
        .iter::<cildec::tables::TypeRefRow>()
        .find(|(_, row)| {
            md.strings()
                .str(row.as_ref().unwrap().type_namespace)
                .unwrap()
                == "System.Runtime.CompilerServices"
        })
        .unwrap();
    let namespace = md.strings().str(row.unwrap().type_namespace).unwrap();
    let heap_index = u16::try_from(
        md.tables()
            .raw_column(cildec::TableId::TypeRef, rid, 2)
            .unwrap(),
    )
    .unwrap();
    let at = namespace.as_ptr() as usize - bytes.as_ptr() as usize;
    let reference = md
        .tables()
        .iter::<cildec::tables::AssemblyRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "ExternalLibrary")
        .unwrap()
        .0;
    let row = md
        .tables()
        .row_bytes(cildec::TableId::AssemblyRef, reference)
        .unwrap();
    assert_eq!(row.len(), 20);
    let record = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut input = bytes.to_vec();
    input[at..at + namespace.len()].fill(0);
    let label = b"Missing\nSPOOFED";
    input[at..at + label.len()].copy_from_slice(label);
    input[record + 14..record + 16].copy_from_slice(&heap_index.to_le_bytes());
    let token = list_methods(&input)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "Unused")
        .unwrap()
        .token;
    let context = DependencyContext::new(input).unwrap();
    let readable = context.decompile(token, OutputStyle::Readable).unwrap();
    assert!(
        !readable
            .text
            .lines()
            .any(|line| line.starts_with("SPOOFED")),
        "{}",
        readable.text
    );
    assert!(
        readable.text.contains("Missing\\nSPOOFED"),
        "{}",
        readable.text
    );
}
