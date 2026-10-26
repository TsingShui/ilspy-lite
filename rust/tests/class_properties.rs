// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

fn property(bytes: &[u8], name: &str) -> (u32, usize, cildec::tables::PropertyRow) {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (rid, row) = md
        .tables()
        .iter::<cildec::tables::PropertyRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == name)
        .unwrap();
    let row = row.unwrap();
    let raw = md
        .tables()
        .row_bytes(cildec::TableId::Property, rid)
        .unwrap();
    assert_eq!(raw.len(), 6);
    (
        0x17000000 | rid,
        raw.as_ptr() as usize - bytes.as_ptr() as usize,
        row,
    )
}

#[test]
fn unreadable_property_name_has_an_explicit_failure_and_keeps_accessors() {
    let bytes = IMAGES[0];
    let (token, offset, _) = property(bytes, "Only");
    let mut patched = bytes.to_vec();
    patched[offset + 2..offset + 4].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Properties").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(
        result
            .text
            .contains(&format!(".property 0x{token:08X} flags=0x0000"))
    );
    assert!(result.text.contains("name-index=0x0000FFFF"));
    assert!(
        result.text.contains("// FAILED metadata:"),
        "{}",
        result.text
    );
    assert!(result.text.lines().any(|l| l.starts_with(".accessor ")
        && l.contains(&format!("association=0x{token:08X}"))
        && l.contains("get_Only")));
    assert!(result.text.contains("@get_Only("));
    assert!(result.text.contains("instance int32 Item(int32)"));
}

#[test]
fn malformed_property_signature_does_not_remove_its_accessors() {
    let bytes = IMAGES[0];
    let (token, offset, _) = property(bytes, "Only");
    let mut patched = bytes.to_vec();
    patched[offset + 4..offset + 6].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Properties").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(
        result
            .text
            .contains(&format!(".property 0x{token:08X} flags=0x0000 Only"))
    );
    assert!(result.text.contains("signature-blob-index=0x0000FFFF"));
    assert!(result.text.lines().any(|l| l.starts_with(".accessor ")
        && l.contains(&format!("association=0x{token:08X}"))
        && l.contains("get_Only")));
    assert!(result.text.contains("@get_Only("));
    assert!(result.text.contains("instance int32 Item(int32)"));
}

#[test]
fn other_and_unknown_accessor_semantics_keep_original_evidence() {
    let bytes = IMAGES[0];
    let (token, _, _) = property(bytes, "Only");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md.tables().semantics_for(cildec::Token(token)).unwrap()[0];
    let row = md
        .tables()
        .row_bytes(cildec::TableId::MethodSemantics, rid)
        .unwrap();
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    for (bits, kind, failures) in [(4u16, "other", 0), (0x40, "unknown", 1)] {
        let mut patched = bytes.to_vec();
        patched[offset..offset + 2].copy_from_slice(&bits.to_le_bytes());
        let result = decompile_type(&patched, "ClassCases.Properties").unwrap();
        assert_eq!(result.summary.metadata_failed, failures);
        assert!(result.text.contains(&format!(
            ".accessor 0x{:08X} association=0x{token:08X} semantics=0x{bits:04X} {kind} method=",
            0x18000000 | rid
        )));
        assert!(result.text.contains("@get_Only("));
    }
}

#[test]
fn accessor_from_an_unowned_method_is_reported_without_hiding_peers() {
    let bytes = IMAGES[0];
    let (token, _, _) = property(bytes, "Only");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md.tables().semantics_for(cildec::Token(token)).unwrap()[0];
    let row = md
        .tables()
        .row_bytes(cildec::TableId::MethodSemantics, rid)
        .unwrap();
    assert_eq!(row.len(), 6);
    let other = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ClassCases.Outer" && m.simple_name == "Value")
        .unwrap();
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 2..offset + 4]
        .copy_from_slice(&((other.token & 0xffffff) as u16).to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Properties").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert!(
        result
            .text
            .contains(&format!("getter method=0x{:08X}", other.token))
    );
    assert!(
        result
            .text
            .contains("invalid or duplicate property accessor relationship")
    );
    assert!(result.text.contains("@get_Only("));
    assert!(result.text.contains("@get_Item("));
}

#[test]
fn property_map_ranges_are_not_silently_clamped() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let owner = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Properties")
        .unwrap()
        .0;
    let map = md
        .tables()
        .property_map_of(cildec::Rid::new(owner))
        .unwrap()
        .unwrap();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::PropertyMap, map.get())
        .unwrap();
    assert_eq!(row.len(), 4);
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    for value in [0u16, u16::MAX] {
        let mut patched = bytes.to_vec();
        patched[offset + 2..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(decompile_type(&patched, "ClassCases.Properties").is_err());
    }
}

#[test]
fn properties_keep_signatures_and_accessor_methods_without_syntax_recovery() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Properties").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        let properties: Vec<_> = result
            .text
            .lines()
            .filter(|l| l.starts_with(".property "))
            .collect();
        assert_eq!(properties.len(), 4, "{}", result.text);
        for signature in [
            "instance int32 Item(int32)",
            "instance int32 Only()",
            "instance int32 WriteOnly()",
            "static int32 Static()",
        ] {
            assert!(
                properties.iter().any(|l| l.contains(signature)),
                "missing {signature}: {}",
                result.text
            );
        }
        assert_eq!(
            result
                .text
                .lines()
                .filter(|l| l.starts_with(".accessor "))
                .count(),
            5
        );
        for name in [
            "get_Item",
            "set_Item",
            "get_Only",
            "set_WriteOnly",
            "get_Static",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ClassCases.Properties" && m.simple_name == name)
                .unwrap();
            assert!(
                result.text.lines().any(|l| l.starts_with(".accessor ")
                    && l.contains(&format!("method=0x{:08X}", method.token))
                    && l.contains(name)),
                "{}",
                result.text
            );
            assert!(
                result
                    .text
                    .contains(&format!(".method-status 0x{:08X} Decompiled", method.token))
            );
        }
        let generic = decompile_type(bytes, "ClassCases.GenericProperty`1").unwrap();
        assert_eq!(generic.summary.metadata_failed, 0);
        assert!(
            generic.text.contains("instance !0 Stored()"),
            "{}",
            generic.text
        );
        assert!(generic.text.contains("get_Stored") && generic.text.contains("set_Stored"));
        // Generic members are admitted through the open self instantiation now.
        assert_eq!(generic.summary.partial, 0, "{}", generic.text);
    }
}
