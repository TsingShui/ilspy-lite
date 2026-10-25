// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::decompile_type;

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

fn field(bytes: &[u8], name: &str) -> (u32, usize, cildec::tables::FieldRow) {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (rid, row) = md
        .tables()
        .iter::<cildec::tables::FieldRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == name)
        .unwrap();
    let row = row.unwrap();
    let raw = md.tables().row_bytes(cildec::TableId::Field, rid).unwrap();
    (
        0x04000000 | rid,
        raw.as_ptr() as usize - bytes.as_ptr() as usize,
        row,
    )
}

#[test]
fn unreadable_field_signature_keeps_header_constant_and_peers() {
    let bytes = IMAGES[0];
    let (token, offset, _) = field(bytes, "Limit");
    let mut patched = bytes.to_vec();
    patched[offset + 4..offset + 6].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Fields").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    let declaration = result
        .text
        .lines()
        .find(|l| l.starts_with(&format!(".field 0x{token:08X} ")))
        .unwrap();
    assert!(declaration.contains("flags=0x8056"), "{declaration}");
    assert!(declaration.contains("@Limit"), "{declaration}");
    assert!(
        result
            .text
            .contains("type=0x0008 value=int32(-7) bytes=[F9, FF, FF, FF]"),
        "{}",
        result.text
    );
    assert!(result.text.contains("static uint32 @Counter"));
    assert!(result.text.contains("@Read("));
}

#[test]
fn unreadable_constant_keeps_field_header_and_other_constants() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (token, _, _) = field(bytes, "Limit");
    let rid = md
        .tables()
        .rows_for_parent(cildec::TableId::Constant, cildec::Token(token))
        .unwrap()[0];
    let row = md
        .tables()
        .row_bytes(cildec::TableId::Constant, rid)
        .unwrap();
    assert_eq!(row.len(), 6);
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 4..offset + 6].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Fields").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(
        result
            .text
            .lines()
            .any(|l| l.starts_with(&format!(".field 0x{token:08X} ")) && l.contains("@Limit")),
        "{}",
        result.text
    );
    assert!(result.text.contains(&format!(
        ".constant 0x{:08X} parent=0x{token:08X} type=0x0008",
        0x0b000000 | rid
    )));
    assert!(result.text.contains("value-blob-index=0x0000FFFF"));
    assert!(result.text.contains("value=uint64(18446744073709551615)"));
    assert!(result.text.contains("@Read("));
}

#[test]
fn malformed_constant_keeps_raw_bytes_and_other_members() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (token, _, _) = field(bytes, "Limit");
    let rid = md
        .tables()
        .rows_for_parent(cildec::TableId::Constant, cildec::Token(token))
        .unwrap()[0];
    let row = md
        .tables()
        .row_bytes(cildec::TableId::Constant, rid)
        .unwrap();
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset..offset + 2].copy_from_slice(&0x0002u16.to_le_bytes()); // I4 bytes with Boolean type.
    let result = decompile_type(&patched, "ClassCases.Fields").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(result.text.contains(&format!(
        ".constant 0x{:08X} parent=0x{token:08X} type=0x0002 bytes=[F9, FF, FF, FF]",
        0x0b000000 | rid
    )));
    assert!(result.text.contains("FAILED metadata"));
    assert!(result.text.contains("@Read("));
    assert!(result.text.contains("@Mask"));
}

#[test]
fn field_layout_and_marshalling_evidence_are_not_dropped() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.LayoutFields").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        assert!(result.text.contains("offset=0"));
        assert!(result.text.contains("offset=4"));
        assert!(result.text.contains(".field-marshal "), "{}", result.text);
        assert!(result.text.contains("bytes=[02]"), "{}", result.text);
    }
}

#[test]
fn rva_field_data_preserves_known_extent_bytes() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (rid, field) = md
        .tables()
        .iter::<cildec::tables::FieldRow>()
        .find(|(_, row)| row.as_ref().unwrap().flags & 0x100 != 0)
        .unwrap();
    let field = field.unwrap();
    let owner = md
        .tables()
        .type_of_field(cildec::Rid::new(rid))
        .unwrap()
        .unwrap();
    let name = cildec::Names::new(&md).type_def(owner).unwrap();
    let result = decompile_type(bytes, &name).unwrap();
    assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
    assert!(
        result
            .text
            .contains(&md.strings().str(field.name).unwrap().to_string())
    );
    assert!(result.text.contains(".field-rva "), "{}", result.text);
    assert!(
        result
            .text
            .contains("bytes=[01, 00, 00, 00, 02, 00, 00, 00"),
        "{}",
        result.text
    );
    assert!(
        result.text.contains("07, 00, 00, 00, 08, 00, 00, 00]"),
        "{}",
        result.text
    );
}

#[test]
fn explicit_layout_with_members_is_not_a_proven_rva_data_extent() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (rid, row) = md
        .tables()
        .iter::<cildec::tables::FieldRow>()
        .find(|(_, row)| row.as_ref().unwrap().flags & 0x100 != 0)
        .unwrap();
    let row = row.unwrap();
    let owner = md
        .tables()
        .type_of_field(cildec::Rid::new(rid))
        .unwrap()
        .unwrap();
    let name = cildec::Names::new(&md).type_def(owner).unwrap();
    let layout = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| {
            md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "LayoutFields"
        })
        .unwrap()
        .0;
    assert!(layout << 2 < 0x80);
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(&signature[..2], &[0x06, 0x11]);
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 2] = (layout << 2) as u8;
    let result = decompile_type(&patched, &name).unwrap();
    assert_eq!(result.summary.metadata_failed, 0);
    let data = result
        .text
        .lines()
        .find(|l| l.starts_with(".field-rva "))
        .unwrap();
    assert!(data.contains("data extent unresolved"), "{data}");
    assert!(!data.contains("bytes="), "{data}");
}

#[test]
fn invalid_field_ownership_ranges_do_not_hide_fields() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Fields")
        .unwrap()
        .0;
    let raw = md
        .tables()
        .row_bytes(cildec::TableId::TypeDef, rid)
        .unwrap();
    assert_eq!(raw.len(), 14);
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    for start in [0u16, u16::MAX] {
        let mut patched = bytes.to_vec();
        patched[offset + 10..offset + 12].copy_from_slice(&start.to_le_bytes());
        assert!(decompile_type(&patched, "ClassCases.Fields").is_err());
    }
    let mut patched = bytes.to_vec();
    let next = md
        .tables()
        .row_bytes(cildec::TableId::TypeDef, rid + 1)
        .unwrap();
    let after = md
        .tables()
        .row_bytes(cildec::TableId::TypeDef, rid + 2)
        .unwrap();
    let n = next.as_ptr() as usize - bytes.as_ptr() as usize;
    let a = after.as_ptr() as usize - bytes.as_ptr() as usize;
    patched[n + 10..n + 12].copy_from_slice(&8u16.to_le_bytes());
    patched[a + 10..a + 12].copy_from_slice(&7u16.to_le_bytes());
    assert!(decompile_type(&patched, "ClassCases.Fields").is_err());
}

#[test]
fn field_constants_keep_type_value_and_exact_bytes() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Fields").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        for marker in [
            "type=0x0008 value=int32(-7) bytes=[F9, FF, FF, FF]",
            "type=0x000B value=uint64(18446744073709551615) bytes=[FF, FF, FF, FF, FF, FF, FF, FF]",
            "type=0x000E value=utf16[0041, 0000, D800] bytes=[41, 00, 00, 00, 00, D8]",
        ] {
            assert!(
                result.text.contains(marker),
                "missing {marker}: {}",
                result.text
            );
        }
        let label = result
            .text
            .lines()
            .find(|line| line.starts_with(".field ") && line.contains("@Label"))
            .unwrap();
        assert!(!label.contains("value=")); // Instance initialization belongs to its constructor.
    }
}

#[test]
fn class_fields_include_private_static_readonly_and_generated_storage() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Fields").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        for name in [
            "storage",
            "Counter",
            "Label",
            "Limit",
            "Mask",
            "Text",
            "Item",
            "<Auto>k__BackingField",
            "Nested",
        ] {
            assert!(
                result
                    .text
                    .lines()
                    .any(|line| line.starts_with(".field 0x04") && line.contains(name)),
                "missing field {name}: {}",
                result.text
            );
        }
        assert!(
            result.text.contains("private int32 @storage"),
            "{}",
            result.text
        );
        assert!(
            result.text.contains("static uint32 @Counter"),
            "{}",
            result.text
        );
        assert!(
            result.text.contains("initonly string @Label"),
            "{}",
            result.text
        );
        assert!(result.text.contains("object @Item"), "{}", result.text);
        assert!(result.text.contains("__il_field_"));
        assert!(result.text.contains("@Read("));
        let generic = decompile_type(bytes, "ClassCases.FieldHolder`1").unwrap();
        assert!(generic.text.contains("!0 @Stored"), "{}", generic.text);
        assert_eq!(generic.summary.metadata_failed, 0);
    }
}
