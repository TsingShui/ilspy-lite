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
fn nested_reference_scopes_and_local_definitions_keep_their_declaring_identity() {
    for (main, dependency) in [
        (
            include_bytes!("fixtures/binding-release.bin").as_slice(),
            include_bytes!("fixtures/binding-library-release.bin").as_slice(),
        ),
        (
            include_bytes!("fixtures/binding-debug.bin").as_slice(),
            include_bytes!("fixtures/binding-library-debug.bin").as_slice(),
        ),
    ] {
        let mut context = DependencyContext::new(main.to_vec()).unwrap();
        context
            .add_reference("BindingLibrary.dll", dependency.to_vec())
            .unwrap();
        let inner = type_reference(main, "", "Inner`1");
        let bound = context.resolve_type(inner).unwrap();
        assert_eq!(bound.name, "BindingLibrary.Outer`1/Inner`1");
        assert_eq!(bound.definition.token, 0x02000006);
        assert_eq!(bound.definition.assembly.name, "BindingLibrary");
        let local = context.resolve_type(0x02000002).unwrap();
        assert_eq!(local.name, "BindingCases.Derived");
        assert!(local.definition.assembly.name.starts_with("binding-"));

        let pe = cildec::PeImage::parse(main).unwrap();
        let md = pe.metadata().unwrap();
        let row = md
            .tables()
            .row_bytes(cildec::TableId::TypeRef, inner & 0xffffff)
            .unwrap();
        assert_eq!(row.len(), 6);
        let offset = row.as_ptr() as usize - main.as_ptr() as usize;
        let mut cyclic = main.to_vec();
        cyclic[offset..offset + 2]
            .copy_from_slice(&((((inner & 0xffffff) << 2) | 3) as u16).to_le_bytes());
        let mut context = DependencyContext::new(cyclic).unwrap();
        context
            .add_reference("BindingLibrary.dll", dependency.to_vec())
            .unwrap();
        assert!(
            context
                .resolve_type(inner)
                .unwrap_err()
                .to_string()
                .contains("cyclic")
        );
    }
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
