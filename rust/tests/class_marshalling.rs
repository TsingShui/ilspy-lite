// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{TableId, Token};
use ilspy_lite::decompile_type;

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

#[test]
fn return_and_ordinary_parameter_marshalling_retains_full_evidence_without_execution_claims() {
    for bytes in IMAGES {
        let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
        let result = decompile_type(bytes, "ClassCases.MarshalMethods").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        let params = md
            .tables()
            .iter::<cildec::tables::ParamRow>()
            .filter(|(_, row)| row.as_ref().unwrap().flags & 0x2000 != 0)
            .collect::<Vec<_>>();
        assert_eq!(params.len(), 3);
        assert!(
            params
                .iter()
                .any(|(_, row)| row.as_ref().unwrap().sequence == 0)
        );
        for (rid, _) in params {
            let parent = Token::new(TableId::Param, rid);
            let records = md
                .tables()
                .rows_for_parent(TableId::FieldMarshal, parent)
                .unwrap();
            assert_eq!(records.len(), 1);
            let marshal = records[0];
            let row = md.tables().field_marshal(marshal).unwrap();
            let line = result
                .text
                .lines()
                .find(|l| {
                    l.starts_with(&format!(
                        ".field-marshal 0x{:08X} ",
                        Token::new(TableId::FieldMarshal, marshal).0
                    ))
                })
                .unwrap();
            assert!(
                line.contains(&format!("parent=0x{:08X}", parent.0)),
                "{line}"
            );
            assert!(
                line.contains(&format!(
                    "native-type-blob-index=0x{:08X}",
                    row.native_type.0
                )),
                "{line}"
            );
            assert!(
                line.contains(&format!(
                    "bytes={:02X?}",
                    md.blobs().get(row.native_type).unwrap()
                )),
                "{line}"
            );
            assert!(line.contains(&format!(
                    "raw-row-bytes={:02X?}",
                    md.tables()
                        .row_bytes(TableId::FieldMarshal, marshal)
                        .unwrap()
                )));
            assert!(
                line.contains("payload=uninterpreted")
                    && line.contains("interop marshaling is runtime-owned"),
                "{line}"
            );
        }
        assert!(result.text.contains("ClassCases.MarshalMethods::Copy"));
        assert!(result.text.contains("ClassCases.MarshalMethods::Native"));
    }
}

#[test]
fn broken_parameter_marshal_blob_or_flag_keeps_record_and_peer_methods() {
    let bytes = IMAGES[0];
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let (param, _) = md
        .tables()
        .iter::<cildec::tables::ParamRow>()
        .find(|(_, row)| {
            row.as_ref().unwrap().flags & 0x2000 != 0 && row.as_ref().unwrap().sequence == 0
        })
        .unwrap();
    let rid = md
        .tables()
        .rows_for_parent(TableId::FieldMarshal, Token::new(TableId::Param, param))
        .unwrap()[0];
    let raw = md.tables().row_bytes(TableId::FieldMarshal, rid).unwrap();
    assert_eq!(raw.len(), 4);
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    let param_row = md.tables().row_bytes(TableId::Param, param).unwrap();
    let param_offset = param_row.as_ptr() as usize - bytes.as_ptr() as usize;
    for bad_blob in [true, false] {
        let mut patched = bytes.to_vec();
        if bad_blob {
            patched[offset + 2..offset + 4].copy_from_slice(&0xffffu16.to_le_bytes());
        } else {
            patched[param_offset..param_offset + 2].copy_from_slice(&0u16.to_le_bytes());
        }
        let result = decompile_type(&patched, "ClassCases.MarshalMethods").unwrap();
        assert_eq!(result.summary.metadata_failed, 1, "{}", result.text);
        assert_eq!(result.summary.exit_code(), 1);
        assert!(result.text.contains(&format!(
            ".field-marshal 0x{:08X} ",
            Token::new(TableId::FieldMarshal, rid).0
        )));
        assert!(result.text.contains("raw-row-bytes="));
        assert!(
            result.text.contains("ClassCases.MarshalMethods::Copy")
                && result.text.contains("ClassCases.MarshalMethods::Native")
        );
    }
}
