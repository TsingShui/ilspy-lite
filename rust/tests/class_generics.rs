// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::decompile_type;

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

fn parameter(bytes: &[u8], name: &str) -> (u32, usize) {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let owner = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == name)
        .unwrap()
        .0;
    let rid = md
        .tables()
        .generic_params(cildec::Token(0x02000000 | owner))
        .unwrap()[0];
    let row = md
        .tables()
        .row_bytes(cildec::TableId::GenericParam, rid)
        .unwrap();
    assert_eq!(row.len(), 8);
    (rid, row.as_ptr() as usize - bytes.as_ptr() as usize)
}

#[test]
fn unreadable_generic_name_keeps_constraint_identity_and_method_results() {
    let bytes = IMAGES[0];
    let (rid, offset) = parameter(bytes, "Constrained`1");
    let mut patched = bytes.to_vec();
    patched[offset + 6..offset + 8].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Constrained`1").unwrap();
    assert!(result.summary.metadata_failed > 0);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(
        result
            .text
            .contains(&format!(".generic 0x{:08X}", 0x2a000000 | rid))
    );
    assert!(result.text.contains("name-index=0x0000FFFF"));
    assert!(result.text.contains("FAILED metadata"));
    assert!(result.text.contains("ClassCases.IMarker"));
    assert!(result.text.contains("::Echo"));
}

#[test]
fn invalid_constraint_retains_original_token_and_peer_members() {
    let bytes = IMAGES[0];
    let (rid, _) = parameter(bytes, "Constrained`1");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let constraint = md
        .tables()
        .generic_param_constraints(cildec::Rid::new(rid))
        .unwrap()[0];
    let row = md
        .tables()
        .row_bytes(cildec::TableId::GenericParamConstraint, constraint)
        .unwrap();
    assert_eq!(row.len(), 4);
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 2..offset + 4].copy_from_slice(&0xfffdu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Constrained`1").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(result.text.contains(&format!(
        ".constraint 0x{:08X} owner=0x{:08X} type=0x01003FFF",
        0x2c000000 | constraint,
        0x2a000000 | rid
    )));
    assert!(result.text.contains("FAILED metadata"));
    assert!(result.text.contains("ClassCases.Base"));
    assert!(result.text.contains("!1 @Value"));
}

#[test]
fn duplicate_generic_numbers_are_reported_without_renumbering() {
    let bytes = IMAGES[0];
    let (rid, _) = parameter(bytes, "IVariance`2");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::GenericParam, rid + 1)
        .unwrap();
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset..offset + 2].copy_from_slice(&0u16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.IVariance`2").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    assert_eq!(
        result
            .text
            .lines()
            .filter(|l| l.starts_with(".generic "))
            .count(),
        2
    );
    assert!(
        result
            .text
            .contains("invalid or duplicate generic parameter number")
    );
}

#[test]
fn generic_context_preserves_owners_constraints_and_nested_members() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Constrained`1").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        for marker in [
            ".generic 0x2A",
            "owner=0x02",
            "owner=0x06",
            "reference-type",
            "default-constructor",
            "nonnullable-value-type",
            ".constraint 0x2C",
            "ClassCases.IMarker",
            "ClassCases.Base",
            "System.ValueType",
            "ClassCases.Constrained`1/Nested`1",
            "!1 @Value",
            "!!0",
        ] {
            assert!(
                result.text.contains(marker),
                "missing {marker}: {}",
                result.text
            );
        }
        // Members of a generic type use the open self instantiation, which is now
        // admitted; nothing in this type may fall back.
        assert_eq!(result.summary.partial, 0, "{}", result.text);
        assert_eq!(result.summary.failed, 0, "{}", result.text);
        assert!(result.text.contains("Echo") && result.text.contains(".ctor"));
        let nested = decompile_type(bytes, "ClassCases.Constrained`1/Nested`1").unwrap();
        assert!(nested.text.contains("number=0") && nested.text.contains("number=1"));
        assert!(!nested.text.contains("::Echo"));
        let variance = decompile_type(bytes, "ClassCases.IVariance`2").unwrap();
        assert_eq!(variance.summary.metadata_failed, 0);
        assert!(variance.text.contains("contravariant") && variance.text.contains("covariant"));
        assert_eq!(variance.summary.exit_code(), 0);
    }
}
