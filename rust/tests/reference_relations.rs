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
