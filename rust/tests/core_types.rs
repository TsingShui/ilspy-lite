// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, Status, decompile, list_methods};

const CORE: &[u8] = include_bytes!("fixtures/core-types.bin");

fn token(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap_or_else(|| panic!("missing owned method {owner}::{name}"))
        .token
}

#[test]
fn a_qualified_local_core_object_is_a_legal_reference_hierarchy_root() {
    for (owner, name) in [
        ("System.Object", "RootConstant"),
        ("System.Object", ".ctor"),
        ("Example.Subject", "Read"),
        ("Example.Subject", ".ctor"),
    ] {
        let result = decompile(CORE, token(CORE, owner, name)).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    }
}

#[test]
fn local_core_value_type_and_enum_descendants_are_not_reference_receivers() {
    // A struct receiver is a managed location of its own value type, which is not
    // a reference receiver; an enum receiver still needs its own storage model.
    let number = decompile(CORE, token(CORE, "Example.Number", "ReadNumber")).unwrap();
    assert_eq!(number.status, Status::Decompiled, "{}", number.text);
    assert!(number.text.contains("ref "), "{}", number.text);
    let enumeration = decompile(CORE, token(CORE, "Example.Flags", "ReadEnum")).unwrap();
    assert_eq!(enumeration.status, Status::Partial, "{}", enumeration.text);
    assert!(
        enumeration
            .text
            .contains("value-type receiver needs IL fallback"),
        "{}",
        enumeration.text
    );
}

#[test]
fn null_bases_cycles_and_generic_owners_do_not_become_core_roots() {
    for (owner, name) in [
        ("Other.Object", "OtherConstant"),
        ("Example.Cyclic", "CycleConstant"),
        ("Example.GenericRoot", "GenericConstant"),
    ] {
        let result = decompile(CORE, token(CORE, owner, name)).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("IL FALLBACK"));
    }
}

#[test]
fn short_name_or_wrong_core_identity_cannot_admit_a_null_base() {
    for bytes in [
        include_bytes!("fixtures/core-fake-name.bin").as_slice(),
        include_bytes!("fixtures/core-wrong-key.bin").as_slice(),
        include_bytes!("fixtures/core-wrong-culture.bin").as_slice(),
    ] {
        let result = decompile(bytes, token(bytes, "Example.Subject", "Read")).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[test]
fn noncanonical_null_base_tags_cannot_alias_the_core_root() {
    let pe = cildec::PeImage::parse(CORE).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().row_bytes(cildec::TableId::TypeDef, 2).unwrap();
    assert_eq!(row.len(), 14);
    assert_eq!(&row[8..10], &[0, 0]);
    let offset = row.as_ptr() as usize - CORE.as_ptr() as usize;
    for encoded in [1u16, 2, 3] {
        let mut patched = CORE.to_vec();
        patched[offset + 8..offset + 10].copy_from_slice(&encoded.to_le_bytes());
        let result = decompile(&patched, token(CORE, "System.Object", "RootConstant")).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[test]
fn malformed_local_reference_marker_payload_is_not_contract_proof() {
    let bytes = include_bytes!("fixtures/core-reference.bin");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().custom_attribute(1).unwrap();
    let payload = md.blobs().get(row.value).unwrap();
    assert_eq!(payload, [1, 0, 0, 0]);
    let offset = payload.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 3] = 0xff;
    let context = DependencyContext::new(patched).unwrap();
    let result = context
        .decompile(
            token(bytes, "System.Object", "RootConstant"),
            OutputStyle::Details,
        )
        .unwrap();
    assert!(
        result.text.contains(".dependency-main kind=Unclassified "),
        "{}",
        result.text
    );
    assert!(
        result
            .text
            .contains("invalid reference assembly marker evidence"),
        "{}",
        result.text
    );
}

#[test]
fn a_core_defined_reference_marker_is_recognized_without_running_its_constructor() {
    for (bytes, expected) in [
        (
            include_bytes!("fixtures/core-reference.bin").as_slice(),
            "ReferenceContract",
        ),
        (
            include_bytes!("fixtures/core-fake-marker.bin").as_slice(),
            "ImplementationImage",
        ),
        (
            include_bytes!("fixtures/core-generic-marker.bin").as_slice(),
            "ImplementationImage",
        ),
        (CORE, "ImplementationImage"),
    ] {
        let context = DependencyContext::new(bytes.to_vec()).unwrap();
        let result = context
            .decompile(
                token(bytes, "System.Object", "RootConstant"),
                OutputStyle::Details,
            )
            .unwrap();
        assert!(
            result
                .text
                .contains(&format!(".dependency-main kind={expected} ")),
            "{}",
            result.text
        );
    }
}
