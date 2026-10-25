// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{TableId, Token};
use ilspy_lite::decompile_type;

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

#[test]
fn custom_attributes_preserve_parent_constructor_payload_and_uninterpreted_status() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let result = decompile_type(bytes, "ClassCases.Attached`1").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        let parent = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, row)| {
                md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Attached`1"
            })
            .unwrap()
            .0;
        let attribute = md
            .tables()
            .rows_for_parent(TableId::CustomAttribute, Token(0x02000000 | parent))
            .unwrap();
        assert!(!attribute.is_empty());
        for rid in attribute {
            let row = md.tables().custom_attribute(rid).unwrap();
            let line = result
                .text
                .lines()
                .find(|line| {
                    line.starts_with(&format!(".custom-attribute 0x{:08X} ", 0x0c000000 | rid))
                })
                .unwrap();
            assert!(
                line.contains(&format!(
                    "parent=0x{:08X} constructor=0x{:08X}",
                    row.parent.0, row.constructor.0
                )),
                "{line}"
            );
            assert!(line.contains(&format!(
                "value-bytes={:02X?}",
                md.blobs().get(row.value).unwrap()
            )));
            assert!(line.contains("raw-row-bytes="));
            assert!(line.contains("payload=uninterpreted"));
        }
        assert!(
            result.text.contains("ClassCases.EvidenceAttribute::.ctor")
                && result.text.contains("signature-bytes=")
        );
        for table in [0x02u8, 0x04, 0x06, 0x08, 0x17, 0x14, 0x2a] {
            assert!(
                result
                    .text
                    .lines()
                    .any(|l| l.starts_with(".custom-attribute ")
                        && l.contains(&format!("parent=0x{table:02X}"))),
                "missing attributed owner table {table:X}: {}",
                result.text
            );
        }
        assert!(result.text.contains("ClassCases.Attached`1/Nested"));
    }
}

#[test]
fn broken_attribute_constructor_or_blob_keeps_record_and_peer_members() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let parent = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Attached`1")
        .unwrap()
        .0;
    let rid = md
        .tables()
        .rows_for_parent(TableId::CustomAttribute, Token(0x02000000 | parent))
        .unwrap()[0];
    let raw = md
        .tables()
        .row_bytes(TableId::CustomAttribute, rid)
        .unwrap();
    assert_eq!(raw.len(), 6);
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    for (column, value) in [(2, 0xffffu16), (4, 0xffff)] {
        let mut patched = bytes.to_vec();
        patched[offset + column..offset + column + 2].copy_from_slice(&value.to_le_bytes());
        let result = decompile_type(&patched, "ClassCases.Attached`1").unwrap();
        assert_eq!(result.summary.metadata_failed, 1, "{}", result.text);
        assert_eq!(result.summary.exit_code(), 1);
        assert!(
            result
                .text
                .contains(&format!(".custom-attribute 0x{:08X}", 0x0c000000 | rid))
        );
        assert!(result.text.contains("raw-row-bytes="));
        assert!(
            result.text.contains("ClassCases.Attached`1::Read") && result.text.contains("@Value")
        );
        assert!(result.text.contains("payload=uninterpreted"));
    }
}
