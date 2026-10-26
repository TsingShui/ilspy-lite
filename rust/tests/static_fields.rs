// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/static-field-release.bin"),
    include_bytes!("fixtures/static-field-debug.bin"),
];

#[test]
fn readonly_initializer_permission_needs_the_same_closed_declaring_storage() {
    for bytes in IMAGES {
        let methods = list_methods(bytes).unwrap();
        let method = methods
            .iter()
            .find(|m| {
                m.declaring_type == "StaticFieldCases.Initialized`1" && m.simple_name == ".cctor"
            })
            .unwrap();
        let anchor = methods
            .iter()
            .find(|m| m.simple_name == "ClosedInitializerAnchor")
            .unwrap();
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body = |token: u32| {
            cildec::MethodBody::from_image(&pe, &md.tables().method_def(token & 0xffffff).unwrap())
                .unwrap()
                .unwrap()
        };
        let readonly = body(method.token)
            .instructions()
            .map(Result::unwrap)
            .filter(|i| i.opcode.name() == "stsfld")
            .map(|i| i.operand.token().unwrap())
            .find(|t| {
                md.strings()
                    .str(md.tables().member_ref(t.rid()).unwrap().name)
                    .unwrap()
                    == "Readonly"
            })
            .unwrap();
        let closed = body(anchor.token)
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "ldsfld")
            .unwrap()
            .operand
            .token()
            .unwrap();
        let parent = md.tables().member_ref(closed.rid()).unwrap().class;
        assert_eq!(parent.table(), Some(cildec::TableId::TypeSpec));
        let row = md
            .tables()
            .row_bytes(cildec::TableId::MemberRef, readonly.rid())
            .unwrap();
        assert_eq!(row.len(), 6);
        let at = row.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut wrong = bytes.to_vec();
        wrong[at..at + 2].copy_from_slice(
            &u16::try_from((parent.rid() << 3) | 4)
                .unwrap()
                .to_le_bytes(),
        );
        let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("readonly static field write"),
            "{}",
            result.text
        );
    }
}

#[test]
fn value_type_static_containers_need_no_instance_layout_or_receiver_copy() {
    for (bytes, library) in IMAGES.into_iter().zip([
        include_bytes!("fixtures/static-field-library-release.bin").as_slice(),
        include_bytes!("fixtures/static-field-library-debug.bin").as_slice(),
    ]) {
        let mut context = ilspy_lite::DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("StaticFieldLibrary", library.to_vec())
            .unwrap();
        for name in [
            "ReadLocalValue",
            "WriteLocalValue",
            "ReadForeignValue",
            "WriteForeignValue",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = context
                .decompile(method.token, OutputStyle::Readable)
                .unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn static_field_facts_follow_exact_reference_identity_and_added_references() {
    for (bytes, library) in IMAGES.into_iter().zip([
        include_bytes!("fixtures/static-field-library-release.bin").as_slice(),
        include_bytes!("fixtures/static-field-library-debug.bin").as_slice(),
    ]) {
        let pe = cildec::PeImage::parse(library).unwrap();
        let md = pe.metadata().unwrap();
        let assembly = md.tables().assembly(1).unwrap();
        let name = md.strings().str(assembly.name).unwrap();
        let at = name.as_ptr() as usize - library.as_ptr() as usize;
        let mut wrong = library.to_vec();
        wrong[at] ^= 1;
        let mut context = ilspy_lite::DependencyContext::new(bytes.to_vec()).unwrap();
        context.add_reference("wrong-scope", wrong).unwrap();
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "ReadForeign")
            .unwrap();
        assert_eq!(
            context
                .decompile(method.token, OutputStyle::Readable)
                .unwrap()
                .status,
            Status::Partial
        );
        context
            .add_reference("correct-definition", library.to_vec())
            .unwrap();
        let result = context
            .decompile(method.token, OutputStyle::Readable)
            .unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    }
}

#[test]
fn foreign_static_storage_requires_checked_selected_definition_facts() {
    for (bytes, library) in IMAGES.into_iter().zip([
        include_bytes!("fixtures/static-field-library-release.bin").as_slice(),
        include_bytes!("fixtures/static-field-library-debug.bin").as_slice(),
    ]) {
        let mut context = ilspy_lite::DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("StaticFieldLibrary", library.to_vec())
            .unwrap();
        for name in [
            "ReadForeign",
            "WriteForeign",
            "ReadForeignGeneric",
            "WriteForeignGeneric",
            "AddressForeign",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let absent = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(absent.status, Status::Partial, "{name}: {}", absent.text);
            let result = context
                .decompile(method.token, OutputStyle::Readable)
                .unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn static_field_operands_need_exact_storage_signatures_and_static_definitions() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "ReadInt")
            .unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method.token & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let field = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "ldsfld")
            .unwrap()
            .operand
            .token()
            .unwrap();
        let anchor = (1..=md.tables().row_count(cildec::TableId::Field))
            .find(|&r| {
                md.strings()
                    .str(md.tables().field(r).unwrap().name)
                    .unwrap()
                    == "SignatureAnchor"
            })
            .unwrap();
        let signature = md
            .tables()
            .raw_column(cildec::TableId::Field, anchor, 2)
            .unwrap();
        let row = md
            .tables()
            .row_bytes(cildec::TableId::MemberRef, field.rid())
            .unwrap();
        assert_eq!(row.len(), 6);
        let at = row.as_ptr() as usize - bytes.as_ptr() as usize + 4;
        let mut wrong = bytes.to_vec();
        wrong[at..at + 2].copy_from_slice(&u16::try_from(signature).unwrap().to_le_bytes());
        let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("static field reference signature differs"),
            "{}",
            result.text
        );

        let declaration = (1..=md.tables().row_count(cildec::TableId::Field))
            .find(|&r| {
                md.strings()
                    .str(md.tables().field(r).unwrap().name)
                    .unwrap()
                    == "Value"
                    && md
                        .strings()
                        .str(
                            md.tables()
                                .type_def(
                                    md.tables()
                                        .type_of_field(cildec::Rid::new(r))
                                        .unwrap()
                                        .unwrap()
                                        .get(),
                                )
                                .unwrap()
                                .type_name,
                        )
                        .unwrap()
                        == "Slots`1"
            })
            .unwrap();
        let row = md
            .tables()
            .row_bytes(cildec::TableId::Field, declaration)
            .unwrap();
        let at = row.as_ptr() as usize - bytes.as_ptr() as usize;
        let flags = md.tables().field(declaration).unwrap().flags;
        for flags in [flags & !0x10, flags | 0x40] {
            let mut wrong = bytes.to_vec();
            wrong[at..at + 2].copy_from_slice(&flags.to_le_bytes());
            let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
        }
    }
}

#[test]
fn generic_static_loads_and_stores_keep_the_instantiated_field_type() {
    for bytes in IMAGES {
        for name in [
            "ReadMixed",
            "ReadInt",
            "WriteInt",
            "Read",
            "Write",
            "ReadSelf",
            "WriteSelf",
            "AddressInt",
            "InitRead",
            "InitWrite",
            "InitAddress",
            "InitReadonly",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}
