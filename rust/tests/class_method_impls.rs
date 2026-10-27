// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{TableId, Token};
use ilspy_lite::{OutputStyle, decompile_type, decompile_type_with_style};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

#[test]
fn explicit_method_mappings_preserve_both_exact_signatures_and_raw_records() {
    for bytes in IMAGES {
        let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
        let owner = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, row)| {
                md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "ExplicitMethods"
            })
            .unwrap()
            .0;
        let result = decompile_type(bytes, "ClassCases.ExplicitMethods").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        let records = md
            .tables()
            .rows_for_parent(TableId::MethodImpl, Token(0x02000000 | owner))
            .unwrap();
        assert_eq!(records.len(), 4);
        for rid in records {
            let row = md.tables().method_impl(rid).unwrap();
            let line = result
                .text
                .lines()
                .find(|l| l.starts_with(&format!(".method-impl 0x{:08X} ", 0x19000000 | rid)))
                .unwrap();
            assert!(
                line.contains(&format!(
                    "class=0x{:08X} body=0x{:08X} declaration=0x{:08X}",
                    0x02000000 | row.class.get(),
                    row.method_body.0,
                    row.method_declaration.0
                )),
                "{line}"
            );
            assert!(line.contains(&format!(
                "raw-row-bytes={:02X?}",
                md.tables().row_bytes(TableId::MethodImpl, rid).unwrap()
            )));
            for (kind, token) in [
                ("body", row.method_body),
                ("declaration", row.method_declaration),
            ] {
                let identity = result
                    .text
                    .lines()
                    .find(|l| l.starts_with(&format!(".method-impl-{kind} 0x{:08X} ", token.0)))
                    .unwrap();
                assert!(
                    identity.contains("owner=")
                        && identity.contains("signature-bytes=")
                        && identity.contains("parsed="),
                    "{identity}"
                );
            }
        }
        assert!(result.text.contains("ClassCases.IExplicit::Read"));
        assert!(result.text.contains("System.IDisposable::Dispose"));
        assert!(result.text.contains("System.IComparable`1"));
        assert!(
            result
                .text
                .contains("dispatch/access/ancestry validation is not performed")
        );
        assert!(result.text.contains("ClassCases.ExplicitMethods::Read"));
    }
}

#[test]
fn broken_method_mapping_keeps_record_and_all_peer_bodies() {
    let bytes = IMAGES[0];
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let rid = 1;
    let row = md.tables().method_impl(rid).unwrap();
    let name = cildec::Names::new(&md).type_def(row.class).unwrap();
    assert_eq!(name, "ClassCases.ExplicitMethods");
    let raw = md.tables().row_bytes(TableId::MethodImpl, rid).unwrap();
    assert_eq!(raw.len(), 6);
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    for column in [2, 4] {
        let mut patched = bytes.to_vec();
        patched[offset + column..offset + column + 2].copy_from_slice(&0xffffu16.to_le_bytes());
        let result = decompile_type(&patched, &name).unwrap();
        assert_eq!(result.summary.metadata_failed, 1, "{}", result.text);
        assert_eq!(result.summary.exit_code(), 1);
        assert!(result.text.contains(".method-impl 0x19000001 "));
        assert!(result.text.contains("raw-row-bytes="));
        assert!(result.text.contains("ClassCases.ExplicitMethods::Read"));
        assert_eq!(
            result
                .text
                .lines()
                .filter(|l| l.starts_with(".method-status "))
                .count(),
            6
        );
    }
}

#[test]
fn constant_strings_render_printable_ascii_and_escape_only_the_rest() {
    for bytes in IMAGES {
        let fields =
            decompile_type_with_style(bytes, "ClassCases.Fields", OutputStyle::Readable).unwrap();
        assert!(
            fields
                .text
                .contains(r#"public const string Text = "A\u0000\uD800";"#),
            "{}",
            fields.text
        );
        assert!(fields.text.is_ascii(), "{}", fields.text);
    }
}
