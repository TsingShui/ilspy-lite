// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::decompile_type;

#[test]
fn overlapping_method_ranges_are_not_counted_as_selected_members() {
    let bytes = include_bytes!("fixtures/class-release.bin");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let mut patched = bytes.to_vec();
    for (rid, name, start) in [
        (3, "EmptyGeneric`1", 1u16),
        (4, "Outer", 3),
        (5, "OuterSuffix", 2),
    ] {
        let definition = md.tables().type_def(rid).unwrap();
        assert_eq!(md.strings().str(definition.type_name).unwrap(), name);
        let row = md
            .tables()
            .row_bytes(cildec::TableId::TypeDef, rid)
            .unwrap();
        assert_eq!(row.len(), 14); // This fixture has two-byte heap/table indices.
        let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
        patched[offset + 12..offset + 14].copy_from_slice(&start.to_le_bytes());
    }
    // Selected range 1..3 looks in-bounds, but another owner begins at 2.
    assert!(decompile_type(&patched, "ClassCases.EmptyGeneric`1").is_err());
}
