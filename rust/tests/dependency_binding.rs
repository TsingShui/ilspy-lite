// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::DependencyContext;

const MAIN: &[u8] = include_bytes!("fixtures/external-release.bin");
const DEPENDENCY: &[u8] = include_bytes!("fixtures/external-library-release.bin");

fn type_reference(bytes: &[u8], namespace: &str, name: &str) -> u32 {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md
        .tables()
        .iter::<cildec::tables::TypeRefRow>()
        .find(|(_, row)| {
            let row = row.as_ref().unwrap();
            md.strings().str(row.type_namespace).unwrap() == namespace
                && md.strings().str(row.type_name).unwrap() == name
        })
        .unwrap()
        .0;
    0x01000000 | rid
}

#[test]
fn type_names_do_not_cross_assembly_scope_and_duplicate_definitions_are_rejected() {
    let pe = cildec::PeImage::parse(DEPENDENCY).unwrap();
    let md = pe.metadata().unwrap();
    let assembly = md.tables().row_bytes(cildec::TableId::Assembly, 1).unwrap();
    assert_eq!(assembly.len(), 22);
    let at = assembly.as_ptr() as usize - DEPENDENCY.as_ptr() as usize;
    let actor = md.tables().row_bytes(cildec::TableId::TypeDef, 2).unwrap();
    let mut wrong_scope = DEPENDENCY.to_vec();
    // Change only Assembly.Name. The ExternalLibrary.Base namespace/name stays.
    wrong_scope[at + 18..at + 20].copy_from_slice(&actor[4..6]);
    let mut context = DependencyContext::new(MAIN.to_vec()).unwrap();
    context
        .add_reference("ExternalLibrary.dll", wrong_scope)
        .unwrap();
    let reference = type_reference(MAIN, "ExternalLibrary", "Base");
    assert!(context.resolve_type(reference).is_err());
    context
        .add_reference("real-definition.dll", DEPENDENCY.to_vec())
        .unwrap();
    assert_eq!(
        context.resolve_type(reference).unwrap().definition.source,
        "real-definition.dll"
    );

    let other = md.tables().row_bytes(cildec::TableId::TypeDef, 3).unwrap();
    let base = md.tables().row_bytes(cildec::TableId::TypeDef, 4).unwrap();
    let at = other.as_ptr() as usize - DEPENDENCY.as_ptr() as usize;
    let mut duplicate = DEPENDENCY.to_vec();
    duplicate[at + 4..at + 6].copy_from_slice(&base[4..6]);
    let mut context = DependencyContext::new(MAIN.to_vec()).unwrap();
    context
        .add_reference("duplicate-definition.dll", duplicate)
        .unwrap();
    assert!(
        context
            .resolve_type(reference)
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
}

#[test]
fn assembly_scoped_type_reference_binds_to_the_supplied_definition() {
    let mut context = DependencyContext::new(MAIN.to_vec()).unwrap();
    let reference = type_reference(MAIN, "ExternalLibrary", "Base");
    assert!(context.resolve_type(reference).is_err());
    context
        .add_reference("owned/ExternalLibrary.dll", DEPENDENCY.to_vec())
        .unwrap();
    let bound = context.resolve_type(reference).unwrap();
    assert_eq!(bound.name, "ExternalLibrary.Base");
    assert_eq!(bound.definition.assembly.name, "ExternalLibrary");
    assert_eq!(bound.definition.source, "owned/ExternalLibrary.dll");
    assert_eq!(bound.definition.token, 0x02000004);
    assert_eq!(bound.flags & 0x20, 0); // Class, not interface.
}
