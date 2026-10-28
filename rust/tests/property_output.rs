// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, decompile_type_with_style};

const INPUT: &[u8] = include_bytes!("fixtures/binding-release.bin");
const LIBRARY: &[u8] = include_bytes!("fixtures/binding-library-release.bin");

fn replace_bytes(bytes: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let mut result = bytes.to_vec();
    let positions = bytes
        .windows(from.len())
        .enumerate()
        .filter_map(|(i, value)| (value == from).then_some(i))
        .collect::<Vec<_>>();
    assert!(!positions.is_empty());
    for position in positions {
        result[position..position + to.len()].copy_from_slice(to);
    }
    result
}

fn method_token(bytes: &[u8], name: &str) -> u32 {
    ilspy_lite::list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn reserved_property_identifiers_are_escaped_after_relationship_binding() {
    for keyword in [
        "byte", "char", "foreach", "decimal", "sbyte", "short", "ushort", "volatile",
    ] {
        let from = if keyword.len() <= 7 {
            b"Enabled".as_slice()
        } else {
            b"VirtualFlag".as_slice()
        };
        let mut name = keyword.as_bytes().to_vec();
        name.resize(from.len(), 0);
        let input = replace_bytes(INPUT, from, &name);
        let library = replace_bytes(LIBRARY, from, &name);
        let mut context = DependencyContext::new(input.clone()).unwrap();
        context.add_reference("keyword library", library).unwrap();
        let actual = context
            .decompile(
                method_token(
                    &input,
                    if keyword.len() <= 7 {
                        "Read"
                    } else {
                        "ReadVirtual"
                    },
                ),
                OutputStyle::Readable,
            )
            .unwrap();
        assert_eq!(actual.status, ilspy_lite::Status::Decompiled);
        // Non-overridable base accessors are written bare, so only the escaped
        // identifier remains; an overridable one keeps the `base.` qualifier.
        let expected = if keyword.len() <= 7 {
            format!("return @{keyword};")
        } else {
            format!("return base.@{keyword};")
        };
        assert!(actual.text.contains(&expected), "{}", actual.text);
    }
}

#[test]
fn a_presentation_alias_cannot_rename_a_proven_runtime_property_lookup() {
    let input = replace_bytes(INPUT, b"Enabled", b"__cil_X");
    let library = replace_bytes(LIBRARY, b"Enabled", b"__cil_X");
    let mut context = DependencyContext::new(input.clone()).unwrap();
    context
        .add_reference("reserved property name", library)
        .unwrap();
    let actual = context
        .decompile(method_token(&input, "Read"), OutputStyle::Readable)
        .unwrap();
    assert_eq!(actual.status, ilspy_lite::Status::Decompiled);
    assert!(
        actual.text.contains("__cil_memberref_call_"),
        "{}",
        actual.text
    );
    assert!(!actual.text.contains("base.__name_"), "{}", actual.text);
}

#[test]
fn helper_like_user_member_names_and_string_literals_are_not_property_call_sites() {
    let bytes = include_bytes!("fixtures/property-release.bin");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let getter = md
        .tables()
        .iter::<cildec::tables::MemberRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "get_Enabled")
        .unwrap()
        .0;
    let target = format!("{:08X}", 0x0a000000 | getter);
    let input = replace_bytes(bytes, b"FFFFFFFF", target.as_bytes());
    let input = replace_bytes(
        &input,
        &"FFFFFFFF"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
        &target
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let mut context = DependencyContext::new(input.clone()).unwrap();
    context
        .add_reference("property library", LIBRARY.to_vec())
        .unwrap();
    let actual = context
        .decompile(method_token(&input, "Collision"), OutputStyle::Readable)
        .unwrap();
    assert_eq!(actual.status, ilspy_lite::Status::Decompiled);
    // ILSpy writes a non-overridable base member bare (`Enabled`), and only spells
    // `base.` when a non-virtual call must suppress an override. The user member
    // whose name merely looks like a helper keeps its exact call form.
    assert!(
        actual.text.contains("__cil_bool_load(Enabled)"),
        "{}",
        actual.text
    );
    assert!(
        actual
            .text
            .contains(&format!("x__cil_memberref_call_{target}(this)")),
        "{}",
        actual.text
    );
    assert!(!actual.text.contains("xbase.Enabled"));
    let literal = context
        .decompile(method_token(&input, "Literal"), OutputStyle::Readable)
        .unwrap();
    assert_eq!(literal.status, ilspy_lite::Status::Decompiled);
    assert!(
        literal
            .text
            .contains(&format!("\"__cil_memberref_call_{target}(this)\"")),
        "{}",
        literal.text
    );
}

#[test]
fn substituted_signature_types_cannot_prove_property_source_binding_for_an_exact_owner() {
    use ilspy_lite::{AssemblyResolution, ReferenceSearchOptions, ReferenceVersionPolicy};
    let input = include_bytes!("fixtures/property-signature-PropertyConsumer.bin");
    let middle = include_bytes!("fixtures/property-signature-PropertyMiddle.bin");
    let payload = include_bytes!("fixtures/property-signature-PropertyPayload.bin");
    let mut exact = DependencyContext::new(input.to_vec()).unwrap();
    exact.add_reference("middle", middle.to_vec()).unwrap();
    exact.add_reference("payload", payload.to_vec()).unwrap();
    let token = method_token(input, "Read");
    // The qualified test only needs the property to be written as an access; the
    // qualifier follows ILSpy, so a non-overridable base member is bare.
    let qualified = exact.decompile(token, OutputStyle::Readable).unwrap();
    assert!(qualified.text.contains("Value"), "{}", qualified.text);
    let md = cildec::PeImage::parse(payload).unwrap().metadata().unwrap();
    let row = md.tables().row_bytes(cildec::TableId::Assembly, 1).unwrap();
    assert_eq!(row.len(), 22);
    let start = row.as_ptr() as usize - payload.as_ptr() as usize;
    let mut newer = payload.to_vec();
    newer[start + 10..start + 12].copy_from_slice(&1u16.to_le_bytes());
    let directory =
        std::env::temp_dir().join(format!("ilspy-property-signature-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("middle.dll"), middle).unwrap();
    std::fs::write(directory.join("payload.dll"), newer).unwrap();
    let mut context = DependencyContext::new(input.to_vec()).unwrap();
    context
        .discover_references(
            &directory.join("consumer.dll"),
            &ReferenceSearchOptions {
                automatic: false,
                files: vec![directory.join("middle.dll"), directory.join("payload.dll")],
                version_policy: ReferenceVersionPolicy::SameMajor,
                ..ReferenceSearchOptions::default()
            },
        )
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    let request = context
        .references()
        .iter()
        .find(|r| r.identity.name == "PropertyPayload")
        .unwrap();
    assert!(matches!(
        context.resolve(request.token).unwrap(),
        AssemblyResolution::Substituted { .. }
    ));
    let md = cildec::PeImage::parse(input).unwrap().metadata().unwrap();
    let getter = md
        .tables()
        .iter::<cildec::tables::MemberRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "get_Value")
        .unwrap()
        .0;
    let accessor = context
        .resolve_property_accessor(0x0a000000 | getter)
        .unwrap()
        .unwrap();
    assert!(
        accessor.owner.exact_identity,
        "the owner must be exact so this tests signature scope uncertainty"
    );
    for name in ["Read", "Write"] {
        let actual = context
            .decompile(method_token(input, name), OutputStyle::Readable)
            .unwrap();
        assert_eq!(actual.status, ilspy_lite::Status::Decompiled);
        assert!(!actual.text.contains("base.Value"), "{}", actual.text);
        assert!(actual.text.contains("__cil_memberref_call_"));
        // Dependency provenance is not part of the readable view; the property
        // refusal above is the invariant this test guards.
    }
}

#[test]
fn an_independent_substituted_property_signature_path_cannot_hide_behind_exact_accessors() {
    use ilspy_lite::{ReferenceSearchOptions, ReferenceVersionPolicy};
    let consumer = include_bytes!("fixtures/property-signature-PropertyConsumer.bin");
    let middle = include_bytes!("fixtures/property-signature-PropertyMiddle.bin");
    let payload = include_bytes!("fixtures/property-signature-PropertyPayload.bin");
    let set_payload_request = |bytes: &[u8]| {
        let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
        let rid = md
            .tables()
            .iter::<cildec::tables::AssemblyRefRow>()
            .find(|(_, row)| {
                md.strings().str(row.as_ref().unwrap().name).unwrap() == "PropertyPayload"
            })
            .unwrap()
            .0;
        let row = md
            .tables()
            .row_bytes(cildec::TableId::AssemblyRef, rid)
            .unwrap();
        assert_eq!(row.len(), 20);
        let start = row.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut result = bytes.to_vec();
        result[start + 6..start + 8].copy_from_slice(&1u16.to_le_bytes());
        result
    };
    let input = set_payload_request(consumer);
    let mut definition = set_payload_request(middle);
    let md = cildec::PeImage::parse(middle).unwrap().metadata().unwrap();
    let table = md.tables();
    let peer = table
        .iter::<cildec::tables::TypeRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Peer")
        .unwrap()
        .0;
    let alias = table
        .iter::<cildec::tables::TypeRefRow>()
        .find(|(_, row)| {
            md.strings().str(row.as_ref().unwrap().type_name).unwrap()
                == "CompilationRelaxationsAttribute"
        })
        .unwrap()
        .0;
    let system = table
        .iter::<cildec::tables::AssemblyRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "System.Runtime")
        .unwrap()
        .0;
    let namespace =
        u16::try_from(table.raw_column(cildec::TableId::TypeRef, peer, 2).unwrap()).unwrap();
    let name = u16::try_from(table.raw_column(cildec::TableId::TypeRef, peer, 1).unwrap()).unwrap();
    let record = table
        .row_bytes(cildec::TableId::AssemblyRef, system)
        .unwrap();
    assert_eq!(record.len(), 20);
    let at = record.as_ptr() as usize - middle.as_ptr() as usize;
    definition[at..at + 20].fill(0);
    definition[at + 14..at + 16].copy_from_slice(&namespace.to_le_bytes());
    let record = table.row_bytes(cildec::TableId::TypeRef, alias).unwrap();
    assert_eq!(record.len(), 6);
    let at = record.as_ptr() as usize - middle.as_ptr() as usize;
    definition[at..at + 2].copy_from_slice(&u16::try_from(system << 2 | 2).unwrap().to_le_bytes());
    definition[at + 2..at + 4].copy_from_slice(&name.to_le_bytes());
    definition[at + 4..at + 6].copy_from_slice(&namespace.to_le_bytes());
    let property = table.property(1).unwrap();
    assert_eq!(md.strings().str(property.name).unwrap(), "Value");
    let signature = md.blobs().get(property.type_signature).unwrap();
    assert_eq!(signature.len(), 4);
    assert_eq!(&signature[..3], &[0x28, 0, 0x12]);
    let at = signature.as_ptr() as usize - middle.as_ptr() as usize;
    definition[at + 3] = u8::try_from(alias << 2 | 1).unwrap();
    let md = cildec::PeImage::parse(payload).unwrap().metadata().unwrap();
    let record = md.tables().row_bytes(cildec::TableId::Assembly, 1).unwrap();
    let at = record.as_ptr() as usize - payload.as_ptr() as usize;
    let mut version = payload.to_vec();
    version[at + 10..at + 12].copy_from_slice(&1u16.to_le_bytes());
    let directory = std::env::temp_dir().join(format!(
        "ilspy-property-signature-path-{}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("middle.dll"), definition).unwrap();
    std::fs::write(directory.join("payload.dll"), version).unwrap();
    let mut context = DependencyContext::new(input.clone()).unwrap();
    context
        .discover_references(
            &directory.join("consumer.dll"),
            &ReferenceSearchOptions {
                automatic: false,
                files: vec![directory.join("middle.dll"), directory.join("payload.dll")],
                version_policy: ReferenceVersionPolicy::SameMajor,
                ..ReferenceSearchOptions::default()
            },
        )
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    let md = cildec::PeImage::parse(&input).unwrap().metadata().unwrap();
    let getter = md
        .tables()
        .iter::<cildec::tables::MemberRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "get_Value")
        .unwrap()
        .0;
    let bound = context
        .resolve_property_accessor(0x0a000000 | getter)
        .unwrap()
        .unwrap();
    assert!(bound.owner.exact_identity);
    assert!(
        context
            .references()
            .iter()
            .filter(|r| r.identity.name == "PropertyPayload")
            .all(|r| matches!(
                context.resolve(r.token).unwrap(),
                ilspy_lite::AssemblyResolution::Resolved { .. }
            ))
    );
    let actual = context
        .decompile(method_token(&input, "Read"), OutputStyle::Readable)
        .unwrap();
    assert_eq!(actual.status, ilspy_lite::Status::Decompiled);
    assert!(!actual.text.contains("base.Value"), "{}", actual.text);
    assert!(actual.text.contains("__cil_memberref_call_"));
    assert!(
        actual.text.contains("metadata-only type substitution")
            && actual.text.contains("source=")
            && actual.text.contains("0.0.0.1"),
        "{}",
        actual.text
    );
}

fn patch_body(bytes: &[u8], token: u32, code: &[u8]) -> Vec<u8> {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let row = pe
        .metadata()
        .unwrap()
        .tables()
        .method_def(token & 0xffffff)
        .unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert!(code.len() <= body.code.len());
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut result = bytes.to_vec();
    match body.kind {
        cildec::HeaderKind::Tiny => result[start - 1] = ((code.len() as u8) << 2) | 2,
        cildec::HeaderKind::Fat => {
            let flags = u16::from_le_bytes(bytes[start - 12..start - 10].try_into().unwrap());
            assert_eq!(flags >> 12, 3);
            result[start - 8..start - 4].copy_from_slice(&(code.len() as u32).to_le_bytes());
        }
    }
    result[start..start + code.len()].copy_from_slice(code);
    result
}

#[test]
fn changed_this_and_other_receivers_keep_exact_calls_instead_of_property_syntax() {
    for (bytes, library) in [
        (
            include_bytes!("fixtures/property-release.bin").as_slice(),
            LIBRARY,
        ),
        (
            include_bytes!("fixtures/property-debug.bin").as_slice(),
            include_bytes!("fixtures/binding-library-debug.bin").as_slice(),
        ),
    ] {
        let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
        let getter = md
            .tables()
            .iter::<cildec::tables::MemberRefRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "get_Enabled")
            .unwrap()
            .0;
        let mut code = vec![0x03, 0x10, 0x00, 0x02, 0x28];
        code.extend((0x0a000000 | getter).to_le_bytes());
        code.push(0x2a);
        let changed = method_token(bytes, "ReassignReceiver");
        let mut context = DependencyContext::new(patch_body(bytes, changed, &code)).unwrap();
        context
            .add_reference("receiver library", library.to_vec())
            .unwrap();
        let result = context.decompile(changed, OutputStyle::Readable).unwrap();
        assert_eq!(
            result.status,
            ilspy_lite::Status::Decompiled,
            "{}",
            result.text
        );
        assert!(!result.text.contains("base.Enabled"));
        assert!(
            result.text.contains("this = other;") && result.text.contains("__cil_memberref_call_"),
            "{}",
            result.text
        );

        let other = method_token(bytes, "ReadConstantReceiver");
        let row = md.tables().method_def(other & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&cildec::PeImage::parse(bytes).unwrap(), &row)
            .unwrap()
            .unwrap();
        let mut code = body.code.to_vec();
        let call = body
            .instructions()
            .map(|i| i.unwrap())
            .find(|i| i.opcode.name() == "callvirt")
            .unwrap();
        code[call.offset as usize] = 0x28;
        let mut context = DependencyContext::new(patch_body(bytes, other, &code)).unwrap();
        context
            .add_reference("other library", library.to_vec())
            .unwrap();
        let result = context.decompile(other, OutputStyle::Readable).unwrap();
        assert_eq!(
            result.status,
            ilspy_lite::Status::Decompiled,
            "{}",
            result.text
        );
        assert!(result.text.contains("__cil_memberref_call_"));
        assert!(
            !result.text.contains("base.ConstantFlag")
                && !result.text.contains("other.ConstantFlag")
        );
    }
}

#[test]
fn a_real_property_relationship_does_not_override_a_conflicting_source_field_name() {
    let md = cildec::PeImage::parse(LIBRARY).unwrap().metadata().unwrap();
    let field = md
        .tables()
        .iter::<cildec::tables::FieldRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "Ready")
        .unwrap()
        .0;
    let property = md
        .tables()
        .iter::<cildec::tables::PropertyRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "Enabled")
        .unwrap()
        .0;
    let row = md
        .tables()
        .row_bytes(cildec::TableId::Field, field)
        .unwrap();
    assert_eq!(row.len(), 6);
    let start = row.as_ptr() as usize - LIBRARY.as_ptr() as usize;
    let name = u16::try_from(
        md.tables()
            .raw_column(cildec::TableId::Property, property, 1)
            .unwrap(),
    )
    .unwrap();
    let mut library = LIBRARY.to_vec();
    library[start + 2..start + 4].copy_from_slice(&name.to_le_bytes());
    let mut context = DependencyContext::new(INPUT.to_vec()).unwrap();
    context
        .add_reference("conflicting source name", library)
        .unwrap();
    let result = context
        .decompile(method_token(INPUT, "Read"), OutputStyle::Readable)
        .unwrap();
    assert_eq!(result.status, ilspy_lite::Status::Decompiled);
    assert!(!result.text.contains("base.Enabled"));
    assert!(
        result.text.contains("field hides the source property name"),
        "{}",
        result.text
    );
}

#[test]
fn exact_base_property_relationships_render_reads_and_writes_without_losing_dispatch() {
    let mut context = DependencyContext::new(INPUT.to_vec()).unwrap();
    context
        .add_reference("owned BindingLibrary", LIBRARY.to_vec())
        .unwrap();
    let actual = context
        .decompile_type("BindingCases.Derived", OutputStyle::Readable)
        .unwrap();
    assert!(actual.text.contains("return Enabled;"), "{}", actual.text);
    assert!(actual.text.contains("Enabled = "), "{}", actual.text);
    assert!(
        actual.text.contains("return base.VirtualFlag;"),
        "a direct base call must not dispatch to the override: {}",
        actual.text
    );
    assert!(
        actual.text.contains("return ConstantFlag;"),
        "{}",
        actual.text
    );
    assert!(!actual.text.contains("base.Pretender"));
    assert!(
        actual.text.contains("__cil_memberref_call_"),
        "ordinary methods retain exact calls"
    );
    let no_context =
        decompile_type_with_style(INPUT, "BindingCases.Derived", OutputStyle::Readable).unwrap();
    assert!(!no_context.text.contains("base.Enabled"));
    assert_eq!(actual.summary, no_context.summary);
    let detailed = context
        .decompile_type("BindingCases.Derived", OutputStyle::Details)
        .unwrap();
    assert!(detailed.text.contains("__cil_memberref_call_"));
    assert!(!detailed.text.contains("base.Enabled"));
}
