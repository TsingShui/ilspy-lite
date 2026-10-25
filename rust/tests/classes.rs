// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

#[test]
fn class_entry_selects_types_with_no_methods() {
    for bytes in IMAGES {
        for name in [
            "ClassCases.Empty",
            "ClassCases.EmptyGeneric`1",
            "ClassCases.Outer/EmptyNested",
        ] {
            let result = decompile_type(bytes, name).unwrap();
            assert!(result.text.contains(name));
            assert!(result.text.contains("// type token 0x02"));
            assert_eq!(result.summary.decompiled, 0);
            assert_eq!(result.summary.declarations, 0);
            assert_eq!(result.summary.partial, 0);
            assert_eq!(result.summary.failed, 0);
            assert_eq!(result.summary.exit_code(), 0);
        }
    }
}

#[test]
fn selected_class_contains_owned_nested_types_not_siblings() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Outer").unwrap();
        assert_eq!(result.summary.decompiled, 3);
        assert_eq!(result.summary.exit_code(), 0);
        for name in [
            "ClassCases.Outer",
            "ClassCases.Outer/EmptyNested",
            "ClassCases.Outer/Inner",
            "ClassCases.Outer/Inner/Leaf",
        ] {
            assert!(result.text.contains(name), "{}", result.text);
        }
        assert!(!result.text.contains("OuterSuffix"));
        let result = decompile_type(bytes, "ClassCases.Outer/Inner").unwrap();
        assert_eq!(result.summary.decompiled, 2);
        assert!(!result.text.contains("EmptyNested"));
    }
}

#[test]
fn class_methods_keep_supported_and_unsupported_results_together() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Mixed").unwrap();
        assert_eq!(result.summary.decompiled, 1);
        assert_eq!(result.summary.partial, 1);
        assert_eq!(result.summary.failed, 0);
        assert_eq!(result.summary.exit_code(), 2);
        assert!(result.text.contains("@Good("));
        assert!(result.text.contains("IL FALLBACK"));
        for method in list_methods(bytes)
            .unwrap()
            .into_iter()
            .filter(|m| m.declaring_type == "ClassCases.Mixed")
        {
            assert!(result.text.contains(&format!("0x{:08X}", method.token)));
        }
        let result = decompile_type(bytes, "ClassCases.Generic`1").unwrap();
        assert!(result.summary.partial > 0);
        assert!(result.text.contains("Echo"));
        assert!(result.text.contains(".ctor"));
        assert_eq!(result.summary.exit_code(), 2);
    }
}

fn method_row_offset(bytes: &[u8], owner: &str, name: &str) -> (usize, u32) {
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap();
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::MethodDef, method.token & 0xffffff)
        .unwrap();
    (
        row.as_ptr() as usize - bytes.as_ptr() as usize,
        method.token,
    )
}

#[test]
fn one_unreadable_method_does_not_hide_its_peers() {
    let bytes = IMAGES[0];
    let (offset, token) = method_row_offset(bytes, "ClassCases.Mixed", "Unsupported");
    let mut patched = bytes.to_vec();
    patched[offset..offset + 4].copy_from_slice(&0u32.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Mixed").unwrap();
    assert_eq!(result.summary.decompiled, 1);
    assert_eq!(result.summary.failed, 1);
    assert_eq!(result.summary.partial, 0);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(result.text.contains("@Good("));
    assert!(result.text.contains(&format!("FAILED token 0x{token:08X}")));
    assert!(result.text.contains("ordinary IL method has no body RVA"));
}

#[test]
fn unrelated_method_signature_is_not_needed_for_type_selection() {
    let bytes = IMAGES[0];
    let (offset, _) = method_row_offset(bytes, "ClassCases.Generic`1", "Echo");
    let mut patched = bytes.to_vec();
    let row = &bytes[offset..offset + 14];
    assert_eq!(row.len(), 14);
    patched[offset + 10..offset + 12].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Outer").unwrap();
    assert_eq!(result.summary.decompiled, 3);
    assert_eq!(result.summary.exit_code(), 0);
}

#[test]
fn invalid_type_member_range_is_not_silently_clamped() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Mixed")
        .unwrap()
        .0;
    let row = md
        .tables()
        .row_bytes(cildec::TableId::TypeDef, rid)
        .unwrap();
    assert_eq!(row.len(), 14); // Fixture uses two-byte heap/table indices.
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    for value in [0u16, u16::MAX] {
        let mut patched = bytes.to_vec();
        patched[offset + 12..offset + 14].copy_from_slice(&value.to_le_bytes());
        assert!(decompile_type(&patched, "ClassCases.Mixed").is_err());
    }
}

#[test]
fn cyclic_nested_ownership_is_not_treated_as_a_name() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::NestedClass, 1)
        .unwrap();
    assert_eq!(row.len(), 4); // Two TypeDef RIDs.
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 2..offset + 4].copy_from_slice(&row[..2]);
    assert!(decompile_type(&patched, "ClassCases.Outer").is_err());
}

#[test]
fn generated_type_can_be_selected_directly_without_hiding_methods() {
    let bytes = include_bytes!("fixtures/release.bin");
    let methods = list_methods(bytes).unwrap();
    let owner = &methods
        .iter()
        .find(|m| {
            m.simple_name == "MoveNext" && m.declaring_type.starts_with("LiteSamples.AsyncSample/")
        })
        .unwrap()
        .declaring_type;
    let result = decompile_type(bytes, owner).unwrap();
    let owned: Vec<_> = methods
        .iter()
        .filter(|m| &m.declaring_type == owner)
        .collect();
    assert_eq!(
        result.summary.decompiled
            + result.summary.declarations
            + result.summary.partial
            + result.summary.failed,
        owned.len()
    );
    for method in owned {
        assert!(result.text.contains(&format!("0x{:08X}", method.token)));
        assert!(result.text.contains(&method.simple_name));
    }
}

#[test]
fn legitimate_bodyless_member_keeps_declaration_status() {
    let bytes = IMAGES[0];
    let (offset, token) = method_row_offset(bytes, "ClassCases.Mixed", "Unsupported");
    let mut patched = bytes.to_vec();
    patched[offset..offset + 4].copy_from_slice(&0u32.to_le_bytes());
    patched[offset + 6..offset + 8].copy_from_slice(&0x2016u16.to_le_bytes()); // PInvokeImpl
    let result = decompile_type(&patched, "ClassCases.Mixed").unwrap();
    assert_eq!(result.summary.decompiled, 1);
    assert_eq!(result.summary.declarations, 1);
    assert_eq!(result.summary.failed, 0);
    assert_eq!(result.summary.exit_code(), 0);
    assert!(result.text.contains(&format!("0x{token:08X}")));
    assert!(result.text.contains("// no IL body"));
}

#[test]
fn class_result_retains_the_same_method_text_as_individual_selection() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Mixed").unwrap();
        for method in list_methods(bytes)
            .unwrap()
            .into_iter()
            .filter(|m| m.declaring_type == "ClassCases.Mixed")
        {
            let single = ilspy_lite::decompile(bytes, method.token).unwrap();
            assert!(result.text.contains(&single.text), "{}", result.text);
        }
    }
}

#[test]
fn metadata_name_prefix_does_not_create_nested_ownership() {
    let bytes = IMAGES[0];
    let from = b"OuterSuffix";
    let offset = bytes.windows(from.len()).position(|s| s == from).unwrap();
    let mut patched = bytes.to_vec();
    patched[offset..offset + from.len()].copy_from_slice(b"Outer/Fauxx");
    let result = decompile_type(&patched, "ClassCases.Outer").unwrap();
    assert_eq!(result.summary.decompiled, 3);
    assert!(!result.text.contains("Fauxx"));
    let result = decompile_type(&patched, "ClassCases.Outer/Fauxx").unwrap();
    assert_eq!(result.summary.decompiled, 1);
    assert!(!result.text.contains("EmptyNested"));
}

#[test]
fn class_entry_rejects_missing_type_and_invalid_image() {
    assert!(decompile_type(IMAGES[0], "ClassCases.Absent").is_err());
    assert!(decompile_type(IMAGES[0], "").is_err());
    assert!(decompile_type(&[], "ClassCases.Empty").is_err());
}
