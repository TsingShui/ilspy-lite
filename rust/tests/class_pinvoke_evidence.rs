// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{TableId, Token};
use ilspy_lite::decompile_type;

#[test]
fn invalid_import_name_or_module_name_keeps_the_other_identity_and_raw_records() {
    for bytes in [
        include_bytes!("fixtures/class-release.bin").as_slice(),
        include_bytes!("fixtures/class-debug.bin").as_slice(),
    ] {
        let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
        let (rid, import) = md
            .tables()
            .iter::<cildec::tables::ImplMapRow>()
            .find(|(_, row)| {
                md.strings().str(row.as_ref().unwrap().import_name).unwrap() == "native_count"
            })
            .unwrap();
        let import = import.unwrap();
        let raw = md.tables().row_bytes(TableId::ImplMap, rid).unwrap();
        assert_eq!(raw.len(), 8);
        let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        let module = md
            .tables()
            .row_bytes(TableId::ModuleRef, import.import_scope.get())
            .unwrap();
        let module_offset = module.as_ptr() as usize - bytes.as_ptr() as usize;
        for corrupt in [0, 1, 2] {
            let mut patched = bytes.to_vec();
            if corrupt == 0 {
                patched[offset + 4..offset + 6].copy_from_slice(&0xffffu16.to_le_bytes());
            } else if corrupt == 1 {
                patched[module_offset..module_offset + 2].copy_from_slice(&0xffffu16.to_le_bytes());
            } else {
                patched[offset + 6..offset + 8].copy_from_slice(&0xffffu16.to_le_bytes());
            }
            let result = decompile_type(&patched, "ClassCases.Methods").unwrap();
            assert_eq!(result.summary.metadata_failed, 1, "{}", result.text);
            assert_eq!(result.summary.exit_code(), 1);
            let line = result
                .text
                .lines()
                .find(|l| {
                    l.starts_with(&format!(
                        ".pinvoke 0x{:08X} ",
                        Token::new(TableId::ImplMap, rid).0
                    ))
                })
                .unwrap();
            assert!(
                line.contains("flags=0x0100")
                    && line.contains(&format!("method=0x{:08X}", import.member_forwarded.0)),
                "{line}"
            );
            assert!(
                line.contains(&format!(
                    "raw-row-bytes={:02X?}",
                    &patched[offset..offset + raw.len()]
                )),
                "{line}"
            );
            if corrupt == 0 {
                assert!(
                    line.contains("name-index=0x0000FFFF")
                        && line.contains("module=fixture-native"),
                    "{line}"
                );
                assert!(
                    line.contains(&format!("module-raw-row-bytes={module:02X?}")),
                    "{line}"
                );
            } else {
                assert!(line.contains("name=native_count"), "{line}");
                assert!(
                    line.contains(if corrupt == 1 {
                        "module-name-index=0x0000FFFF"
                    } else {
                        "scope-rid=0x0000FFFF"
                    }),
                    "{line}"
                );
            }
            assert!(result.text.contains("ClassCases.Methods::Pick(int64)"));
        }
    }
}
