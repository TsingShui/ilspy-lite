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
fn member_references_bind_using_full_signatures_and_owning_definitions() {
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
        let pe = cildec::PeImage::parse(main).unwrap();
        let md = pe.metadata().unwrap();
        for name in ["get_Enabled", "set_Enabled", "get_Pretender", "Echo"] {
            let (rid, row) = md
                .tables()
                .iter::<cildec::tables::MemberRefRow>()
                .find(|(_, r)| {
                    let r = r.as_ref().unwrap();
                    md.strings().str(r.name).unwrap() == name
                        && r.class.table() == Some(cildec::TableId::TypeRef)
                })
                .unwrap();
            let row = row.unwrap();
            let method = context.resolve_method(0x0a000000 | rid).unwrap();
            assert_eq!(method.name, name);
            assert_eq!(method.owner.name, "BindingLibrary.Base");
            assert_eq!(method.definition.source, "BindingLibrary.dll");
            assert_eq!(
                method.reference_signature,
                md.blobs().get(row.signature).unwrap()
            );
            assert!(!method.definition_signature.is_empty());
        }
    }
}

#[test]
fn property_accessors_require_metadata_relationships_not_get_or_set_names() {
    let main = include_bytes!("fixtures/binding-release.bin");
    let dependency = include_bytes!("fixtures/binding-library-release.bin");
    let mut context = DependencyContext::new(main.to_vec()).unwrap();
    context
        .add_reference("BindingLibrary.dll", dependency.to_vec())
        .unwrap();
    let md = cildec::PeImage::parse(main).unwrap().metadata().unwrap();
    for (name, property, role) in [
        ("get_Enabled", Some("Enabled"), "get"),
        ("set_Enabled", Some("Enabled"), "set"),
        ("get_Pretender", None, ""),
    ] {
        let rid = md
            .tables()
            .iter::<cildec::tables::MemberRefRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == name)
            .unwrap()
            .0;
        let accessor = context.resolve_property_accessor(0x0a000000 | rid).unwrap();
        match property {
            Some(property) => {
                let accessor = accessor.unwrap();
                assert_eq!(accessor.name, property);
                assert_eq!(accessor.role, role);
                assert_eq!(accessor.owner.name, "BindingLibrary.Base");
                assert!(!accessor.signature.is_empty());
            }
            None => assert!(accessor.is_none()),
        }
    }
}

#[test]
fn property_semantics_flags_and_signatures_must_agree() {
    let main = include_bytes!("fixtures/binding-release.bin");
    let dependency = include_bytes!("fixtures/binding-library-release.bin");
    let md = cildec::PeImage::parse(main).unwrap().metadata().unwrap();
    let rid = md
        .tables()
        .iter::<cildec::tables::MemberRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "get_Enabled")
        .unwrap()
        .0;
    let def = cildec::PeImage::parse(dependency)
        .unwrap()
        .metadata()
        .unwrap();
    let (method, _) = def
        .tables()
        .iter::<cildec::tables::MethodDefRow>()
        .find(|(_, row)| def.strings().str(row.as_ref().unwrap().name).unwrap() == "get_Enabled")
        .unwrap();
    let (relation, _) = def
        .tables()
        .iter::<cildec::tables::MethodSemanticsRow>()
        .find(|(_, row)| row.as_ref().unwrap().method.get() == method)
        .unwrap();
    let raw = def
        .tables()
        .row_bytes(cildec::TableId::MethodSemantics, relation)
        .unwrap();
    let offset = raw.as_ptr() as usize - dependency.as_ptr() as usize;
    let property = def.tables().property(1).unwrap();
    assert_eq!(def.strings().str(property.name).unwrap(), "Enabled");
    let bytes = def.blobs().get(property.type_signature).unwrap();
    assert_eq!(bytes, &[0x28, 0, 2]);
    let signature_offset = bytes.as_ptr() as usize - dependency.as_ptr() as usize;
    for invalid in ["semantics", "signature"] {
        let mut patched = dependency.to_vec();
        if invalid == "semantics" {
            patched[offset..offset + 2].copy_from_slice(&3u16.to_le_bytes());
        } else {
            patched[signature_offset + 2] = 8;
        }
        let mut context = DependencyContext::new(main.to_vec()).unwrap();
        context.add_reference("changed.dll", patched).unwrap();
        assert!(context.resolve_method(0x0a000000 | rid).is_ok());
        assert!(
            context.resolve_property_accessor(0x0a000000 | rid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn member_signature_mismatch_is_not_a_name_only_binding() {
    let main = include_bytes!("fixtures/binding-release.bin");
    let md = cildec::PeImage::parse(main).unwrap().metadata().unwrap();
    let (rid, row) = md
        .tables()
        .iter::<cildec::tables::MemberRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "get_Enabled")
        .unwrap();
    let signature = md.blobs().get(row.unwrap().signature).unwrap();
    assert_eq!(signature, &[0x20, 0, 2]);
    let offset = signature.as_ptr() as usize - main.as_ptr() as usize;
    let mut changed = main.to_vec();
    changed[offset + 2] = 8;
    let mut context = DependencyContext::new(changed).unwrap();
    context
        .add_reference(
            "BindingLibrary.dll",
            include_bytes!("fixtures/binding-library-release.bin").to_vec(),
        )
        .unwrap();
    assert!(
        context
            .resolve_method(0x0a000000 | rid)
            .unwrap_err()
            .to_string()
            .contains("signature")
    );
}

#[test]
fn constructed_generic_member_owners_keep_type_arguments_and_nested_positions() {
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
        let md = cildec::PeImage::parse(main).unwrap().metadata().unwrap();
        for (rid, row) in md.tables().iter::<cildec::tables::MemberRefRow>() {
            let row = row.unwrap();
            if row.class.table() != Some(cildec::TableId::TypeSpec)
                || md.strings().str(row.name).unwrap() == "Right"
            {
                continue;
            }
            let resolved = context.resolve_method(0x0a000000 | rid).unwrap();
            if resolved.name == "Read" {
                assert_eq!(resolved.owner.name, "BindingLibrary.Outer`1/Inner`1");
                assert_eq!(resolved.owner.generic_arguments, ["I4", "String"]);
            } else {
                assert_eq!(resolved.owner.name, "BindingLibrary.Generic`1");
                assert_eq!(resolved.owner.generic_arguments, ["I4"]);
                assert!(matches!(resolved.name.as_str(), "get_Item" | "Echo"));
            }
            assert!(resolved.signature_identity.contains("I4"));
            assert!(!resolved.signature_identity.contains("var:0"));
            if resolved.name == "get_Item" {
                assert_eq!(
                    context
                        .resolve_property_accessor(0x0a000000 | rid)
                        .unwrap()
                        .unwrap()
                        .name,
                    "Item"
                );
            }
        }
    }
}

#[test]
fn generic_field_reference_uses_the_instantiated_owner_signature() {
    let main = include_bytes!("fixtures/binding-release.bin");
    let dependency = include_bytes!("fixtures/binding-library-release.bin");
    let mut context = DependencyContext::new(main.to_vec()).unwrap();
    context
        .add_reference("BindingLibrary.dll", dependency.to_vec())
        .unwrap();
    let md = cildec::PeImage::parse(main).unwrap().metadata().unwrap();
    let rid = md
        .tables()
        .iter::<cildec::tables::MemberRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "Right")
        .unwrap()
        .0;
    let bound = context.resolve_field(0x0a000000 | rid).unwrap();
    assert_eq!(bound.name, "Right");
    assert_eq!(bound.owner.name, "BindingLibrary.Outer`1/Inner`1");
    assert_eq!(bound.owner.generic_arguments, ["I4", "String"]);
    assert!(bound.signature_identity.contains("String"));
    assert_eq!(bound.reference_signature, bound.definition_signature);
}

fn rename_reference(bytes: &[u8], old: &str, replacement: &str) -> Vec<u8> {
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let rid = md
        .tables()
        .iter::<cildec::tables::AssemblyRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == old)
        .unwrap()
        .0;
    // Assembly names and namespaces can share a heap entry. Reuse an unrelated
    // method-name slot (or the facade's module-name slot) instead of renaming a namespace.
    let slot = md
        .tables()
        .iter::<cildec::tables::MethodDefRow>()
        .map(|(_, r)| r.unwrap().name)
        .find(|index| md.strings().str(*index).unwrap().starts_with("get_Virtual"))
        .unwrap_or(md.tables().module(1).unwrap().name);
    let name = md.strings().str(slot).unwrap().as_bytes();
    assert!(replacement.len() <= name.len());
    let at = name.as_ptr() as usize - bytes.as_ptr() as usize;
    let raw = md
        .tables()
        .row_bytes(cildec::TableId::AssemblyRef, rid)
        .unwrap();
    assert_eq!(raw.len(), 20);
    let row_at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut result = bytes.to_vec();
    result[at..at + name.len()].fill(0);
    result[at..at + replacement.len()].copy_from_slice(replacement.as_bytes());
    result[row_at + 14..row_at + 16].copy_from_slice(&(slot.0 as u16).to_le_bytes());
    result
}

#[test]
fn type_forwarders_reach_the_defining_assembly_and_cycles_remain_unresolved() {
    let main = include_bytes!("fixtures/binding-release.bin");
    let facade = include_bytes!("fixtures/binding-facade-release.bin");
    let input = rename_reference(main, "BindingLibrary", "BindingFacade");
    let token = type_reference(&input, "BindingLibrary", "Peer");
    let mut context = DependencyContext::new(input.clone()).unwrap();
    context
        .add_reference("BindingFacade.dll", facade.to_vec())
        .unwrap();
    assert!(context.resolve_type(token).is_err());
    context
        .add_reference(
            "BindingLibrary.dll",
            include_bytes!("fixtures/binding-library-release.bin").to_vec(),
        )
        .unwrap();
    let peer = context.resolve_type(token).unwrap();
    assert_eq!(peer.definition.assembly.name, "BindingLibrary");
    assert_eq!(peer.name, "BindingLibrary.Peer");
    assert_eq!(peer.forwarders.len(), 1);
    assert_eq!(peer.forwarders[0].assembly.name, "BindingFacade");
    let inner = context
        .resolve_type(type_reference(&input, "", "Inner`1"))
        .unwrap();
    assert_eq!(inner.name, "BindingLibrary.Outer`1/Inner`1");
    assert_eq!(inner.forwarders.len(), 1);
    let mut cyclic = DependencyContext::new(input).unwrap();
    cyclic
        .add_reference(
            "cycle.dll",
            rename_reference(facade, "BindingLibrary", "BindingFacade"),
        )
        .unwrap();
    assert!(
        cyclic
            .resolve_type(token)
            .unwrap_err()
            .to_string()
            .contains("cyclic")
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
