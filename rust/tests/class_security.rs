// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{TableId, Token};
use ilspy_lite::decompile_type;

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

#[test]
fn class_method_and_assembly_security_records_are_preserved_without_enforcement_claims() {
    for bytes in IMAGES {
        let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
        let result = decompile_type(bytes, "ClassCases.Secured").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        assert!(result.text.contains("ClassCases.Secured::Read"));
        let records = md
            .tables()
            .iter::<cildec::tables::DeclSecurityRow>()
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 3);
        for (rid, row) in records {
            let row = row.unwrap();
            let line = result
                .text
                .lines()
                .find(|l| l.starts_with(&format!(".decl-security 0x{:08X} ", 0x0e000000 | rid)))
                .unwrap();
            assert!(
                line.contains(&format!(
                    "parent=0x{:08X} action=0x{:04X}",
                    row.parent.0, row.action
                )),
                "{line}"
            );
            assert!(
                line.contains(&format!(
                    "permission-set-bytes={:02X?}",
                    md.blobs().get(row.permission_set).unwrap()
                )),
                "{line}"
            );
            assert!(line.contains(&format!(
                "raw-row-bytes={:02X?}",
                md.tables().row_bytes(TableId::DeclSecurity, rid).unwrap()
            )));
            assert!(line.contains("payload=uninterpreted"));
            assert!(line.contains("security enforcement is runtime-owned"));
        }
    }
}

#[test]
fn broken_security_payload_keeps_raw_record_and_peer_method() {
    let bytes = IMAGES[0];
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let parent = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Secured")
        .unwrap()
        .0;
    let rid = md
        .tables()
        .rows_for_parent(TableId::DeclSecurity, Token(0x02000000 | parent))
        .unwrap()[0];
    let raw = md.tables().row_bytes(TableId::DeclSecurity, rid).unwrap();
    assert_eq!(raw.len(), 6);
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 4..offset + 6].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Secured").unwrap();
    assert_eq!(result.summary.metadata_failed, 1, "{}", result.text);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(
        result
            .text
            .contains(&format!(".decl-security 0x{:08X} ", 0x0e000000 | rid))
    );
    assert!(result.text.contains("raw-row-bytes="));
    assert!(result.text.contains("ClassCases.Secured::Read"));
}
