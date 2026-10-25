// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::TableId;
use ilspy_lite::decompile_type;

#[test]
fn decodable_invalid_owners_keep_contextual_records_and_peer_method_statuses() {
    let bytes = include_bytes!("fixtures/class-release.bin");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let baseline = decompile_type(bytes, "ClassCases.ExplicitMethods").unwrap();
    let statuses = |text: &str| {
        text.lines()
            .filter(|l| l.starts_with(".method-status "))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    for (table, owner_offset, encoded, marker) in [
        (TableId::CustomAttribute, 0, 0xffe3u16, ".custom-attribute"),
        (TableId::DeclSecurity, 2, 0xfffc, ".decl-security"),
        (TableId::ClassLayout, 6, 0xffff, ".class-layout"),
        (TableId::MethodImpl, 0, 0xffff, ".method-impl"),
    ] {
        // Mutate the last row to preserve the table's declared sort order.
        let rid = md.tables().row_count(table);
        assert!(rid > 0);
        let raw = md.tables().row_bytes(table, rid).unwrap();
        let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut patched = bytes.to_vec();
        patched[offset + owner_offset..offset + owner_offset + 2]
            .copy_from_slice(&encoded.to_le_bytes());
        let result = decompile_type(&patched, "ClassCases.ExplicitMethods").unwrap();
        assert_eq!(
            result.summary.metadata_failed, 1,
            "{table:?}: {}",
            result.text
        );
        assert_eq!(result.summary.exit_code(), 1);
        let token = cildec::Token::new(table, rid).0;
        let record = result
            .text
            .lines()
            .find(|l| l.starts_with(&format!("{marker} 0x{token:08X} ")))
            .unwrap();
        assert!(record.contains("raw-row-bytes="), "{record}");
        assert!(
            result.text.contains("ownership=unowned-invalid"),
            "{}",
            result.text
        );
        assert_eq!(statuses(&result.text), statuses(&baseline.text));
    }
}

#[test]
fn zero_attachment_owner_is_not_a_valid_unrelated_scope() {
    let bytes = include_bytes!("fixtures/class-release.bin");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let rid = 1;
    let raw = md
        .tables()
        .row_bytes(TableId::CustomAttribute, rid)
        .unwrap();
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset..offset + 2].copy_from_slice(&0u16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.ExplicitMethods").unwrap();
    assert_eq!(result.summary.metadata_failed, 1, "{}", result.text);
    assert!(result.text.contains(".custom-attribute 0x0C000001 "));
    assert!(result.text.contains("ownership=unowned-invalid"));
}
