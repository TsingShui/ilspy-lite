// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{TableId, Token};
use ilspy_lite::decompile_type;

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

#[test]
fn unreadable_type_reference_parts_keep_raw_identity_and_independent_scope_values() {
    for bytes in IMAGES {
        let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
        let (rid, row) = md
            .tables()
            .iter::<cildec::tables::TypeRefRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Object")
            .unwrap();
        let row = row.unwrap();
        let raw = md.tables().row_bytes(TableId::TypeRef, rid).unwrap();
        assert_eq!(raw.len(), 6);
        let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        for (column, surviving, missing) in [
            (2, "namespace=System", "name-index=0x0000FFFF"),
            (4, "name=Object", "namespace-index=0x0000FFFF"),
        ] {
            let mut patched = bytes.to_vec();
            patched[offset + column..offset + column + 2].copy_from_slice(&0xffffu16.to_le_bytes());
            let result = decompile_type(&patched, "ClassCases.Fields").unwrap();
            assert_eq!(result.summary.exit_code(), 1);
            assert!(result.summary.metadata_failed > 0);
            assert!(result.text.contains(&format!(
                ".extends 0x{:08X}",
                Token::new(TableId::TypeRef, rid).0
            )));
            assert!(
                result.text.contains(surviving) && result.text.contains(missing),
                "{}",
                result.text
            );
            assert!(
                result.text.contains(&format!(
                    "resolution-scope=0x{:08X}",
                    row.resolution_scope.0
                )),
                "{}",
                result.text
            );
            assert!(
                result.text.contains("name=System.Runtime")
                    && result.text.contains("version=10.0.0.0"),
                "{}",
                result.text
            );
            assert!(
                result
                    .text
                    .contains("key-or-token=[B0, 3F, 5F, 7F, 11, D5, 0A, 3A]")
            );
            assert!(result.text.contains(&format!(
                "raw-row-bytes={:02X?}",
                &patched[offset..offset + raw.len()]
            )));
            assert!(result.text.contains("@Counter") && result.text.contains(".method-status "));
        }
    }
}

#[test]
fn unreadable_assembly_reference_parts_do_not_erase_type_or_other_assembly_fields() {
    let bytes = IMAGES[0];
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let rid = 1;
    let raw = md.tables().row_bytes(TableId::AssemblyRef, rid).unwrap();
    assert_eq!(raw.len(), 20);
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    for (column, evidence) in [
        (14, "name-index=0x0000FFFF"),
        (16, "culture-index=0x0000FFFF"),
        (12, "key-or-token-blob-index=0x0000FFFF"),
    ] {
        let mut patched = bytes.to_vec();
        patched[offset + column..offset + column + 2].copy_from_slice(&0xffffu16.to_le_bytes());
        let result = decompile_type(&patched, "ClassCases.Fields").unwrap();
        assert_eq!(result.summary.exit_code(), 1);
        assert!(
            result.text.contains("name=Object") && result.text.contains("namespace=System"),
            "{}",
            result.text
        );
        assert!(
            result.text.contains("resolution-scope=0x23000001")
                && result.text.contains("version=10.0.0.0")
        );
        assert!(result.text.contains(evidence), "{}", result.text);
        assert!(result.text.contains(&format!(
            "raw-row-bytes={:02X?}",
            &patched[offset..offset + raw.len()]
        )));
        assert!(result.text.contains("@Limit"));
    }
}

#[test]
fn invalid_resolution_scope_keeps_encoded_scope_and_typeref_heap_values() {
    let bytes = IMAGES[0];
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let rid = md
        .tables()
        .iter::<cildec::tables::TypeRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Object")
        .unwrap()
        .0;
    let raw = md.tables().row_bytes(TableId::TypeRef, rid).unwrap();
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset..offset + 2].copy_from_slice(&0xfffeu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Fields").unwrap();
    assert_eq!(result.summary.exit_code(), 1);
    assert!(
        result.text.contains("resolution-scope-encoded=0x0000FFFE"),
        "{}",
        result.text
    );
    assert!(result.text.contains("name=Object") && result.text.contains("namespace=System"));
    assert!(result.text.contains("raw-row-bytes="));
}
