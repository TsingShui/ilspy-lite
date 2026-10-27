// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [(&[u8], &[u8]); 2] = [
    (
        include_bytes!("fixtures/value-call-release.bin"),
        include_bytes!("fixtures/value-call-library-release.bin"),
    ),
    (
        include_bytes!("fixtures/value-call-debug.bin"),
        include_bytes!("fixtures/value-call-library-debug.bin"),
    ),
];

fn token(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn constructor_calls_keep_existing_cells_and_reject_nonlocation_or_virtual_receivers() {
    for (bytes, _) in IMAGES {
        let method = token(bytes, "ValueCallCases.Ops", "ReinitializePlain");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let instructions = body.instructions().map(Result::unwrap).collect::<Vec<_>>();
        let created = instructions
            .iter()
            .find(|i| i.opcode.name() == "newobj")
            .unwrap();
        let stored = instructions
            .iter()
            .find(|i| i.opcode.name() == "stobj")
            .unwrap();
        let base = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut owned = bytes.to_vec();
        owned[base + created.offset as usize] = 0x28;
        owned[base + stored.offset as usize..base + stored.offset as usize + 5].fill(0);
        let result = decompile_with_style(&owned, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("__cil_value_ctor_"), "{}", result.text);
        assert!(
            !result.text.contains("__cil_value_newobj_"),
            "{}",
            result.text
        );
        let mut virtual_call = owned.clone();
        virtual_call[base + created.offset as usize] = 0x6f;
        let result = decompile_with_style(&virtual_call, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("constructor/intrinsic callvirt"),
            "{}",
            result.text
        );
        let receiver = instructions
            .iter()
            .find(|i| i.opcode.name() == "ldarg.0")
            .unwrap();
        owned[base + receiver.offset as usize] = 0x03;
        let result = decompile_with_style(&owned, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("exact managed location"),
            "{}",
            result.text
        );
    }
}

#[test]
fn ordinary_value_constructor_metadata_requires_instance_void_and_constructor_flags() {
    for (bytes, _) in IMAGES {
        let method = token(bytes, "ValueCallCases.Ops", "NewPlain");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let ctor = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "newobj")
            .unwrap()
            .operand
            .token()
            .unwrap();
        let row = md.tables().method_def(ctor.rid()).unwrap();
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::MethodDef, ctor.rid())
            .unwrap();
        let at = raw.as_ptr() as usize - bytes.as_ptr() as usize + 6;
        for flags in [row.flags | 0x10, row.flags | 0x40, row.flags & !0x1800] {
            let mut invalid = bytes.to_vec();
            invalid[at..at + 2].copy_from_slice(&flags.to_le_bytes());
            let result = decompile_with_style(&invalid, method, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
        }
        let signature = md.blobs().get(row.signature).unwrap();
        assert_eq!(signature[2], 1);
        let at = signature.as_ptr() as usize - bytes.as_ptr() as usize + 2;
        let mut invalid = bytes.to_vec();
        invalid[at] = 8;
        let result = decompile_with_style(&invalid, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("ordinary instance void"),
            "{}",
            result.text
        );
    }
}

#[test]
fn ordinary_value_constructor_delegation_initializes_the_existing_value_receiver() {
    for (bytes, _) in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let constructor = list_methods(bytes)
            .unwrap()
            .into_iter()
            .filter(|m| m.declaring_type == "ValueCallCases.Plain" && m.simple_name == ".ctor")
            .find(|m| {
                let row = md.tables().method_def(m.token & 0xffffff).unwrap();
                cildec::MethodSig::parse(md.blobs().get(row.signature).unwrap())
                    .unwrap()
                    .params
                    .len()
                    == 1
            })
            .unwrap();
        let result = decompile_with_style(bytes, constructor.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("__cil_value_ctor_"), "{}", result.text);
    }
}

#[test]
fn ordinary_local_value_construction_uses_the_exact_method_definition_and_completed_copy() {
    for (bytes, _) in IMAGES {
        for name in [
            "NewPlain",
            "NewPlainInto",
            "NewPlainOrder",
            "NewPlainRef",
            "NewPlainAliased",
        ] {
            let method = token(bytes, "ValueCallCases.Ops", name);
            let pe = cildec::PeImage::parse(bytes).unwrap();
            let md = pe.metadata().unwrap();
            let body = cildec::MethodBody::from_image(
                &pe,
                &md.tables().method_def(method & 0xffffff).unwrap(),
            )
            .unwrap()
            .unwrap();
            let operand = body
                .instructions()
                .map(Result::unwrap)
                .find(|i| i.opcode.name() == "newobj")
                .unwrap()
                .operand
                .token()
                .unwrap();
            assert_eq!(operand.table(), Some(cildec::TableId::MethodDef));
            let result = decompile_with_style(bytes, method, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(
                result.text.contains("__cil_value_newobj_"),
                "{}",
                result.text
            );
        }
    }
}

#[test]
fn boxing_and_unbox_any_keep_struct_and_scoped_generic_identities() {
    for (bytes, _) in IMAGES {
        for name in [
            "Box",
            "BoxGeneric",
            "Unbox",
            "UnboxCell",
            "CopyBox",
            "SameBox",
            "DifferentBoxes",
        ] {
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result =
                    decompile_with_style(bytes, token(bytes, "ValueCallCases.Ops", name), style)
                        .unwrap();
                assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            }
        }
    }
}

#[test]
fn value_newobj_constructs_a_value_in_its_original_instantiation() {
    for (bytes, library) in IMAGES {
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("owned constructor library", library.to_vec())
            .unwrap();
        for name in ["New", "ForeignNew"] {
            let result = context
                .decompile(
                    token(bytes, "ValueCallCases.Ops", name),
                    OutputStyle::Readable,
                )
                .unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(
                result.text.contains("__cil_value_newobj_"),
                "{}",
                result.text
            );
            assert!(
                !result.text.contains("fresh object identity"),
                "{}",
                result.text
            );
        }
    }
}

#[test]
fn constructed_local_value_calls_use_the_original_managed_location() {
    for (bytes, _) in IMAGES {
        for name in ["Read", "CopyRead", "Generic", "Alias", "Write", "Static"] {
            let result = decompile_with_style(
                bytes,
                token(bytes, "ValueCallCases.Ops", name),
                OutputStyle::Readable,
            )
            .unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
        for name in ["Read", "Write", "Echo"] {
            let result = decompile_with_style(
                bytes,
                token(bytes, "ValueCallCases.Cell`1", name),
                OutputStyle::Readable,
            )
            .unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn value_calls_and_fields_do_not_discard_operand_kind_or_signature() {
    for (bytes, _) in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = token(bytes, "ValueCallCases.Ops", "Read");
        let row = md.tables().method_def(method & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let call = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "call")
            .unwrap();
        let at = body.code.as_ptr() as usize - bytes.as_ptr() as usize + call.offset as usize;
        let mut virtual_call = bytes.to_vec();
        virtual_call[at] = 0x6f;
        let result = decompile_with_style(&virtual_call, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("boxing/constrained"),
            "{}",
            result.text
        );

        let alias = token(bytes, "ValueCallCases.Ops", "Alias");
        let row = md.tables().method_def(alias & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let field = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "ldfld")
            .unwrap()
            .operand
            .token()
            .unwrap();
        assert_eq!(field.table(), Some(cildec::TableId::MemberRef));
        let field = md.tables().member_ref(field.rid()).unwrap();
        let signature = md.blobs().get(field.signature).unwrap();
        assert_eq!(signature, [0x06, 0x08]);
        let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut mismatch = bytes.to_vec();
        mismatch[at + 1] = 0x0a;
        let result = decompile_with_style(&mismatch, alias, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[test]
fn constructed_value_fields_require_the_requested_instantiated_signature() {
    for (bytes, _) in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = token(bytes, "ValueCallCases.Ops", "StringCount");
        let row = md.tables().method_def(method & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let operand = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "ldfld")
            .unwrap()
            .operand
            .token()
            .unwrap();
        assert_eq!(operand.table(), Some(cildec::TableId::MemberRef));
        let row = md
            .tables()
            .row_bytes(cildec::TableId::MemberRef, operand.rid())
            .unwrap();
        assert_eq!(row.len(), 6);
        let generic = (1..=md.tables().row_count(cildec::TableId::Field))
            .map(|rid| md.tables().field(rid).unwrap())
            .find(|f| md.blobs().get(f.signature).unwrap() == [0x06, 0x13, 0x00])
            .unwrap();
        let at = row.as_ptr() as usize - bytes.as_ptr() as usize + 4;
        let mut mismatch = bytes.to_vec();
        mismatch[at..at + 2].copy_from_slice(&(generic.signature.0 as u16).to_le_bytes());
        let result = decompile_with_style(&mismatch, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("field reference signature differs"),
            "{}",
            result.text
        );
    }
}

#[test]
fn constructed_foreign_value_calls_keep_the_declared_owner_and_argument_scope() {
    for (bytes, library) in IMAGES {
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("owned value-call library", library.to_vec())
            .unwrap();
        for name in ["Foreign", "ForeignGeneric", "ForeignWrite"] {
            let method = token(bytes, "ValueCallCases.Ops", name);
            let absent = decompile_with_style(bytes, method, OutputStyle::Readable).unwrap();
            assert_eq!(absent.status, Status::Partial, "{}", absent.text);
            let result = context.decompile(method, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}
