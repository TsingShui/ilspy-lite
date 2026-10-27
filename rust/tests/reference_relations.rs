// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [(&[u8], &[u8]); 2] = [
    (
        include_bytes!("fixtures/reference-relation-release.bin"),
        include_bytes!("fixtures/relation-library-release.bin"),
    ),
    (
        include_bytes!("fixtures/reference-relation-debug.bin"),
        include_bytes!("fixtures/relation-library-debug.bin"),
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
fn ilspy_relationships_use_the_selected_definition_not_literal_reference_versions() {
    let (bytes, library) = IMAGES[0];
    let md = cildec::PeImage::parse(library).unwrap().metadata().unwrap();
    let assembly = md.tables().row_bytes(cildec::TableId::Assembly, 1).unwrap();
    let at = assembly.as_ptr() as usize - library.as_ptr() as usize;
    let mut selected = library.to_vec();
    selected[at + 4..at + 6].copy_from_slice(&4u16.to_le_bytes());
    let path = std::env::temp_dir().join(format!(
        "ilspy-selected-relation-{}.dll",
        std::process::id()
    ));
    std::fs::write(&path, selected).unwrap();
    let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
    context
        .discover_references(
            std::path::Path::new("owned.dll"),
            &ilspy_lite::ReferenceSearchOptions {
                automatic: false,
                files: vec![path.clone()],
                ..ilspy_lite::ReferenceSearchOptions::default()
            },
        )
        .unwrap();
    let result = context
        .decompile(
            token(bytes, "RelationCases.Ops", "ForeignGeneric"),
            OutputStyle::Readable,
        )
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(result.status, Status::Decompiled, "{}", result.text);
}

#[test]
fn local_constructed_reference_relationships_preserve_arguments_and_scope() {
    for (bytes, _) in IMAGES {
        for name in [
            "NullForeignFormal",
            "DeclaredOverload",
            "Up",
            "Swap",
            "ClosedUp",
            "Interface",
            "Root",
            "SwappedInterface",
            "Covariant",
            "CovariantClosed",
            "Contravariant",
            "CovariantGeneric",
        ] {
            let result = decompile_with_style(
                bytes,
                token(bytes, "RelationCases.Ops", name),
                OutputStyle::Readable,
            )
            .unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(
                !result.text.contains("IL fallback"),
                "{name}: {}",
                result.text
            );
        }
    }
}

#[test]
fn open_self_cast_references_keep_their_declaring_member_identity() {
    for (bytes, _) in IMAGES {
        let result = decompile_with_style(
            bytes,
            token(bytes, "RelationCases.Cell`1", "ReadCast"),
            OutputStyle::Readable,
        )
        .unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    }
}

#[test]
fn relationships_do_not_erase_positions_or_apply_variance_to_values() {
    for (bytes, _) in IMAGES {
        for (name, pattern, replacement) in
            [("Swap", [0x1e, 1], 0u8), ("Covariant", [1, 0x0e], 8u8)]
        {
            let token = token(bytes, "RelationCases.Ops", name);
            let pe = cildec::PeImage::parse(bytes).unwrap();
            let md = pe.metadata().unwrap();
            let row = md.tables().method_def(token & 0xffffff).unwrap();
            let signature = md.blobs().get(row.signature).unwrap();
            let at = signature.windows(2).position(|w| w == pattern).unwrap();
            let base = signature.as_ptr() as usize - bytes.as_ptr() as usize;
            let mut malformed = bytes.to_vec();
            malformed[base + at + 1] = replacement;
            let actual = decompile_with_style(&malformed, token, OutputStyle::Readable).unwrap();
            assert_eq!(actual.status, Status::Partial, "{name}: {}", actual.text);
        }

        let method = token(bytes, "RelationCases.Ops", "CovariantGeneric");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let parameters = md.tables().generic_params(cildec::Token(method)).unwrap();
        assert_eq!(parameters.len(), 1);
        let rid = parameters[0];
        let parameter = md.tables().generic_param(rid).unwrap();
        assert_ne!(parameter.flags & 4, 0);
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::GenericParam, rid)
            .unwrap();
        let at = raw.as_ptr() as usize - bytes.as_ptr() as usize + 2;
        let mut unconstrained = bytes.to_vec();
        unconstrained[at..at + 2].copy_from_slice(&(parameter.flags & !4).to_le_bytes());
        let actual = decompile_with_style(&unconstrained, method, OutputStyle::Readable).unwrap();
        assert_eq!(actual.status, Status::Partial, "{}", actual.text);
    }
}

#[test]
fn foreign_exception_ancestry_needs_a_selected_class_hierarchy() {
    for (bytes, library) in IMAGES {
        let methods = ["ForeignCatch", "ForeignThrow"];
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        for name in methods {
            let method = token(bytes, "ExceptionRegionCases.Ops", name);
            assert_eq!(
                context
                    .decompile(method, OutputStyle::Readable)
                    .unwrap()
                    .status,
                Status::Partial
            );
        }
        context
            .add_reference("owned relation library", library.to_vec())
            .unwrap();
        for name in methods {
            let method = token(bytes, "ExceptionRegionCases.Ops", name);
            let result = context.decompile(method, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }

        let pe = cildec::PeImage::parse(library).unwrap();
        let md = pe.metadata().unwrap();
        let base = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, row)| {
                md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "ForeignException"
            })
            .unwrap()
            .0;
        let object = md
            .tables()
            .iter::<cildec::tables::TypeRefRow>()
            .find(|(_, row)| {
                let row = row.as_ref().unwrap();
                md.strings().str(row.type_namespace).unwrap() == "System"
                    && md.strings().str(row.type_name).unwrap() == "Object"
            })
            .unwrap()
            .0;
        let row = md
            .tables()
            .row_bytes(cildec::TableId::TypeDef, base)
            .unwrap();
        assert_eq!(row.len(), 14);
        let at = row.as_ptr() as usize - library.as_ptr() as usize + 8;
        for replacement in [(object << 2) | 1, base << 2] {
            let mut modified = library.to_vec();
            modified[at..at + 2]
                .copy_from_slice(&u16::try_from(replacement).unwrap().to_le_bytes());
            let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
            context
                .add_reference("unrelated or cyclic owned library", modified)
                .unwrap();
            for name in methods {
                let result = context
                    .decompile(
                        token(bytes, "ExceptionRegionCases.Ops", name),
                        OutputStyle::Readable,
                    )
                    .unwrap();
                assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            }
        }
    }
}

#[test]
fn nested_base_constructor_calls_compare_the_declared_instantiation() {
    for (bytes, library) in IMAGES {
        // A nested class that inherits its enclosing type's parameters names its
        // base through the same substitution the call site uses.
        let method = token(bytes, "RelationCases.LocalOuter`2/NestedLocal", ".ctor");
        let result = decompile_with_style(bytes, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("__cil_ctor_call_"), "{}", result.text);

        // The same shape whose base is defined in another image still needs a
        // dependency fact, so no answer is guessed from the local declaration.
        let foreign = token(bytes, "RelationCases.ForeignOuter`2/NestedForeign", ".ctor");
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("owned relation library", library.to_vec())
            .unwrap();
        let refused = context.decompile(foreign, OutputStyle::Readable).unwrap();
        assert_eq!(refused.status, Status::Partial, "{}", refused.text);
        assert!(
            refused
                .text
                .contains("constructed member needs a local definition"),
            "{}",
            refused.text
        );
    }
}

#[test]
fn external_generic_instance_calls_need_the_selected_definition_flags() {
    for (bytes, library) in IMAGES {
        let method = token(bytes, "RelationCases.Ops", "ExternalGeneric");
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        let absent = context.decompile(method, OutputStyle::Readable).unwrap();
        assert_eq!(absent.status, Status::Partial, "{}", absent.text);
        assert!(
            absent
                .text
                .contains("external generic instance dispatch needs dependency facts"),
            "{}",
            absent.text
        );
        context
            .add_reference("owned relation library", library.to_vec())
            .unwrap();
        let present = context.decompile(method, OutputStyle::Readable).unwrap();
        assert_eq!(present.status, Status::Decompiled, "{}", present.text);
        assert!(
            present.text.contains("__cil_callvirt_")
                || present.text.contains("__cil_methodspec_call_"),
            "{}",
            present.text
        );

        // The same call site keeps refusing when the selected definition is virtual.
        let virtual_call = token(bytes, "RelationCases.ExternalGenericNegative", "Virtual");
        let refused = context
            .decompile(virtual_call, OutputStyle::Readable)
            .unwrap();
        assert_eq!(refused.status, Status::Partial, "{}", refused.text);
        assert!(
            refused
                .text
                .contains("generic virtual dispatch needs runtime lookup-order modeling"),
            "{}",
            refused.text
        );

        // A mutated owner hierarchy/flag cannot admit the same token.
        let pe = cildec::PeImage::parse(library).unwrap();
        let md = pe.metadata().unwrap();
        let api = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Api")
            .unwrap()
            .0;
        let echo = md
            .tables()
            .method_range(cildec::Rid::new(api))
            .unwrap()
            .find(|rid| {
                md.strings()
                    .str(md.tables().method_def(*rid).unwrap().name)
                    .unwrap()
                    == "Echo"
            })
            .unwrap();
        let row = md
            .tables()
            .row_bytes(cildec::TableId::MethodDef, echo)
            .unwrap();
        assert_eq!(row.len(), 14, "Owned small-heap MethodDef row changed.");
        let at = row.as_ptr() as usize - library.as_ptr() as usize + 6;
        let mut flags = u16::from_le_bytes([row[6], row[7]]);
        assert_eq!(flags & 0x0040, 0, "owned Echo must start nonvirtual");
        flags |= 0x0040;
        let mut marked_virtual = library.to_vec();
        marked_virtual[at..at + 2].copy_from_slice(&flags.to_le_bytes());
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("owned virtual relation library", marked_virtual)
            .unwrap();
        let refused = context.decompile(method, OutputStyle::Readable).unwrap();
        assert_eq!(refused.status, Status::Partial, "{}", refused.text);
        assert!(
            refused
                .text
                .contains("generic virtual dispatch needs runtime lookup-order modeling"),
            "{}",
            refused.text
        );
    }
}

#[test]
fn uninitialized_local_slots_need_definite_assignment_on_every_path() {
    for (bytes, _) in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        for name in ["SkipInitBranch", "SkipInitLoop", "SkipInitAddress"] {
            let method = token(bytes, "RelationCases.Ops", name);
            let body = cildec::MethodBody::from_image(
                &pe,
                &md.tables().method_def(method & 0xffffff).unwrap(),
            )
            .unwrap()
            .unwrap();
            assert!(!body.flags.init_locals(), "{name}");
            assert!(body.local_var_sig.is_some(), "{name}");
            let result = decompile_with_style(bytes, method, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
        let bypass = token(bytes, "RelationCases.Ops", "SkipInitBypass");
        let result = decompile_with_style(bytes, bypass, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);

        for (name, local) in [
            ("SkipInitBranch", 0),
            ("SkipInitLoop", 0),
            ("SkipInitLoop", 1),
        ] {
            let method = token(bytes, "RelationCases.Ops", name);
            let body = cildec::MethodBody::from_image(
                &pe,
                &md.tables().method_def(method & 0xffffff).unwrap(),
            )
            .unwrap()
            .unwrap();
            let store = body.instructions().map(Result::unwrap).find(|instruction| {
                instruction.opcode.canonical().name() == "stloc"
                    && matches!(instruction.operand, cildec::Operand::Var(index) if index == local)
            }).expect("first store to owned local");
            let at = body.code.as_ptr() as usize - bytes.as_ptr() as usize + store.offset as usize;
            assert_eq!(bytes[at], 0x0a + u8::try_from(local).unwrap());
            let mut missing_store = bytes.to_vec();
            missing_store[at] = 0x26; // pop the assigned value without initializing the local.
            let result =
                decompile_with_style(&missing_store, method, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(
                result
                    .text
                    .contains("local may be used before definite assignment"),
                "{}",
                result.text
            );
        }
        for (name, reason) in [
            (
                "AddressBeforeAssignment",
                "local may be used before definite assignment",
            ),
            (
                "ProtectedRegion",
                "uninitialized local exception-region analysis is not implemented",
            ),
        ] {
            let method = token(bytes, "RelationCases.LocalInitNegative", name);
            let result = decompile_with_style(bytes, method, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(result.text.contains(reason), "{}", result.text);
        }
    }
}

#[test]
fn interface_edges_cannot_prove_exception_ancestry() {
    for (bytes, library) in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let typeref = |name: &str| {
            md.tables()
                .iter::<cildec::tables::TypeRefRow>()
                .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == name)
                .unwrap()
                .0
        };
        let method = token(bytes, "ExceptionRegionCases.Ops", "ForeignThrow");
        let signature = md
            .blobs()
            .get(md.tables().method_def(method & 0xffffff).unwrap().signature)
            .unwrap();
        assert_eq!(signature.len(), 5);
        assert_eq!(&signature[..4], &[0, 1, 1, 0x12]);
        assert_eq!(
            signature[4],
            u8::try_from((typeref("ForeignException") << 2) | 1).unwrap()
        );
        let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut consumer = bytes.to_vec();
        consumer[at + 4] = u8::try_from((typeref("Child") << 2) | 1).unwrap();

        let pe = cildec::PeImage::parse(library).unwrap();
        let md = pe.metadata().unwrap();
        let interface = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "IRoot")
            .unwrap()
            .0;
        let exception = md
            .tables()
            .iter::<cildec::tables::TypeRefRow>()
            .find(|(_, row)| {
                md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Exception"
            })
            .unwrap()
            .0;
        let row = md
            .tables()
            .row_bytes(cildec::TableId::TypeDef, interface)
            .unwrap();
        assert_eq!(row.len(), 14);
        let at = row.as_ptr() as usize - library.as_ptr() as usize + 8;
        let mut modified = library.to_vec();
        modified[at..at + 2]
            .copy_from_slice(&u16::try_from((exception << 2) | 1).unwrap().to_le_bytes());
        let mut context = DependencyContext::new(consumer).unwrap();
        context
            .add_reference("owned malformed interface library", modified)
            .unwrap();
        let result = context.decompile(method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[test]
fn cyclic_foreign_hierarchies_cannot_supply_assignment_proofs() {
    for (bytes, library) in IMAGES {
        let pe = cildec::PeImage::parse(library).unwrap();
        let md = pe.metadata().unwrap();
        let find = |name: &str| {
            md.tables()
                .iter::<cildec::tables::TypeDefRow>()
                .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == name)
                .unwrap()
                .0
        };
        let base = find("Base");
        let child = find("Child");
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::TypeDef, base)
            .unwrap();
        assert_eq!(raw.len(), 14, "Owned small-heap TypeDef row changed.");
        let at = raw.as_ptr() as usize - library.as_ptr() as usize + 8;
        let mut cyclic = library.to_vec();
        cyclic[at..at + 2].copy_from_slice(&u16::try_from(child << 2).unwrap().to_le_bytes());
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("owned cyclic relation library", cyclic)
            .unwrap();
        let actual = context
            .decompile(
                token(bytes, "RelationCases.Ops", "Foreign"),
                OutputStyle::Readable,
            )
            .unwrap();
        assert_eq!(actual.status, Status::Partial, "{}", actual.text);
    }
}

#[test]
fn reference_registry_rebuilds_when_selected_definitions_change() {
    for (bytes, library) in IMAGES {
        let method = token(bytes, "RelationCases.Ops", "ForeignGeneric");
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        let absent = context.decompile(method, OutputStyle::Readable).unwrap();
        assert_eq!(absent.status, Status::Partial, "{}", absent.text);
        context
            .add_reference("owned relation library", library.to_vec())
            .unwrap();
        let present = context.decompile(method, OutputStyle::Readable).unwrap();
        assert_eq!(present.status, Status::Decompiled, "{}", present.text);
        let second = context
            .decompile(
                token(bytes, "RelationCases.Ops", "Foreign"),
                OutputStyle::Readable,
            )
            .unwrap();
        assert_eq!(second.status, Status::Decompiled, "{}", second.text);
        let again = context.decompile(method, OutputStyle::Readable).unwrap();
        assert_eq!(
            present.text, again.text,
            "Query order changed the same public output."
        );
    }
}

#[test]
fn foreign_reference_relationships_need_exact_selected_definitions() {
    for (bytes, library) in IMAGES {
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("owned relation library", library.to_vec())
            .unwrap();
        for name in [
            "Foreign",
            "ForeignInterface",
            "ForeignGenericInterface",
            "ForeignGeneric",
            "ForeignOpenInterface",
            "LocalToForeign",
        ] {
            let token = token(bytes, "RelationCases.Ops", name);
            let absent = decompile_with_style(bytes, token, OutputStyle::Readable).unwrap();
            assert_eq!(absent.status, Status::Partial, "{name}: {}", absent.text);
            let present = context.decompile(token, OutputStyle::Readable).unwrap();
            assert_eq!(
                present.status,
                Status::Decompiled,
                "{name}: {}",
                present.text
            );
        }
    }
}
