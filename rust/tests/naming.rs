// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{
    OutputStyle, Status, decompile_type_with_style, decompile_with_style, list_methods,
};

const IMAGES: &[&[u8]] = &[
    include_bytes!("fixtures/naming-release.bin"),
    include_bytes!("fixtures/naming-debug.bin"),
];
const CLOSURE: &str = "NamingCases.Factory/<>c__DisplayClass0_0";

fn rename_type(bytes: &[u8], old: &str, new: &str) -> Vec<u8> {
    assert_eq!(old.len(), new.len());
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (_, row) = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .map(|(rid, row)| (rid, row.unwrap()))
        .find(|(_, row)| md.strings().str(row.type_name).unwrap() == old)
        .expect("owned type to rename");
    let value = md.strings().str(row.type_name).unwrap();
    let offset = value.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut changed = bytes.to_vec();
    changed[offset..offset + old.len()].copy_from_slice(new.as_bytes());
    changed
}

#[test]
fn generated_alias_does_not_collide_with_a_user_type_in_the_same_owner() {
    for &bytes in IMAGES {
        let changed = rename_type(bytes, "Plain", "<X>__");
        let result =
            decompile_type_with_style(&changed, "NamingCases.CollisionHost", OutputStyle::Readable)
                .unwrap();
        // "<X>__" has the literal encoded alias below. A user spelling that
        // alias must receive a distinct name rather than becoming this type.
        assert_eq!(
            result.text.matches("class __name_3C583E5F5F").count(),
            1,
            "{}",
            result.text
        );
    }
}

#[test]
fn generated_and_reserved_type_references_agree_with_their_declarations() {
    for &bytes in IMAGES {
        let changed = rename_type(bytes, "Plain", "<X>__");
        let result =
            decompile_type_with_style(&changed, "NamingCases.CollisionHost", OutputStyle::Readable)
                .unwrap();
        for declaration in [
            "public NamingCases.CollisionHost.__name_3C583E5F5F Left;",
            "public NamingCases.CollisionHost.__name_5F5F6E616D655F33433538334535463546 Right;",
        ] {
            assert!(
                result.text.contains(declaration),
                "missing {declaration}: {}",
                result.text
            );
        }
        assert!(
            !result.text.contains("CollisionHost.<X>__"),
            "{}",
            result.text
        );
    }
}

fn rename_field(bytes: &[u8], old: &str, new: &str) -> (Vec<u8>, u32) {
    assert_eq!(old.len(), new.len());
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let (rid, row) = md
        .tables()
        .iter::<cildec::tables::FieldRow>()
        .map(|(rid, row)| (rid, row.unwrap()))
        .find(|(_, row)| md.strings().str(row.name).unwrap() == old)
        .expect("owned field");
    let value = md.strings().str(row.name).unwrap();
    let at = value.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut changed = bytes.to_vec();
    changed[at..at + old.len()].copy_from_slice(new.as_bytes());
    (changed, 0x04000000 | rid)
}

fn rename_method(bytes: &[u8], old: &str, new: &str) -> (Vec<u8>, u32) {
    assert_eq!(old.len(), new.len());
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let (rid, row) = md
        .tables()
        .iter::<cildec::tables::MethodDefRow>()
        .map(|(rid, row)| (rid, row.unwrap()))
        .find(|(_, row)| md.strings().str(row.name).unwrap() == old)
        .expect("owned method");
    let value = md.strings().str(row.name).unwrap();
    let at = value.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut changed = bytes.to_vec();
    changed[at..at + old.len()].copy_from_slice(new.as_bytes());
    (changed, 0x06000000 | rid)
}

#[test]
fn renamed_static_method_calls_keep_the_target_alias_and_original_token() {
    for &bytes in IMAGES {
        let (changed, target) = rename_method(bytes, "Callee", "<M>___");
        let method = list_methods(&changed)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "NamingCases.StaticTargets" && m.simple_name == "Update")
            .unwrap();
        let actual = decompile_with_style(&changed, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(actual.status, Status::Decompiled, "{}", actual.text);
        assert!(
            actual.text.contains("StaticTargets.__name_3C4D3E5F5F5F("),
            "{}",
            actual.text
        );
        let details = decompile_with_style(&changed, method.token, OutputStyle::Details).unwrap();
        assert!(
            details.text.contains(&format!("call token 0x{target:08X}")),
            "{}",
            details.text
        );
        let ty =
            decompile_type_with_style(&changed, "NamingCases.StaticTargets", OutputStyle::Readable)
                .unwrap();
        assert!(
            ty.text.contains("int __name_3C4D3E5F5F5F(int value)"),
            "{}",
            ty.text
        );
    }
}

#[test]
fn static_field_alias_is_shared_by_its_declaration_load_and_store() {
    for &bytes in IMAGES {
        let (changed, token) = rename_field(bytes, "Data", "<F>_");
        let method = list_methods(&changed)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "NamingCases.StaticTargets" && m.simple_name == "Update")
            .unwrap();
        let actual = decompile_with_style(&changed, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(actual.status, Status::Decompiled, "{}", actual.text);
        let alias = format!("__il_field_{token:08X}");
        assert!(
            actual
                .text
                .matches(&format!("StaticTargets.{alias}"))
                .count()
                >= 2,
            "{}",
            actual.text
        );
        let ty =
            decompile_type_with_style(&changed, "NamingCases.StaticTargets", OutputStyle::Readable)
                .unwrap();
        assert!(
            ty.text.contains(&format!("public static int {alias};")),
            "{}",
            ty.text
        );
    }
}

fn module_local_reference_variant(bytes: &[u8]) -> Vec<u8> {
    let mut changed = rename_type(bytes, "Plain", "Q12`1");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let tables = md.tables();
    let definition = |name| {
        tables
            .iter::<cildec::tables::TypeDefRow>()
            .map(|(rid, row)| (rid, row.unwrap()))
            .find(|(_, row)| md.strings().str(row.type_name).unwrap() == name)
            .unwrap()
            .0
    };
    let reference = |name| {
        tables
            .iter::<cildec::tables::TypeRefRow>()
            .map(|(rid, row)| (rid, row.unwrap()))
            .find(|(_, row)| md.strings().str(row.type_name).unwrap() == name)
            .unwrap()
            .0
    };
    let outer = definition("CollisionHost");
    let inner = definition("Plain");
    let root_ref = reference("Func`2");
    let nested_ref = reference("CompilerGeneratedAttribute");
    let root_name = tables
        .raw_column(cildec::TableId::TypeDef, outer, 1)
        .unwrap();
    let root_namespace = tables
        .raw_column(cildec::TableId::TypeDef, outer, 2)
        .unwrap();
    let inner_name = tables
        .raw_column(cildec::TableId::TypeDef, inner, 1)
        .unwrap();
    for (rid, values) in [
        (root_ref, [4, root_name, root_namespace]),
        (nested_ref, [root_ref << 2 | 3, inner_name, 0]),
    ] {
        let row = tables.row_bytes(cildec::TableId::TypeRef, rid).unwrap();
        assert_eq!(row.len(), 6);
        let at = row.as_ptr() as usize - bytes.as_ptr() as usize;
        for (column, value) in values.into_iter().enumerate() {
            changed[at + column * 2..at + column * 2 + 2]
                .copy_from_slice(&u16::try_from(value).unwrap().to_le_bytes());
        }
    }
    let (_, field) = tables
        .iter::<cildec::tables::FieldRow>()
        .map(|(rid, row)| (rid, row.unwrap()))
        .find(|(_, row)| md.strings().str(row.name).unwrap() == "Left")
        .unwrap();
    let signature = md.blobs().get(field.signature).unwrap();
    assert_eq!(signature.len(), 3);
    assert_eq!(&signature[..2], &[6, 0x12]);
    let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    changed[at + 2] = u8::try_from(nested_ref << 2 | 1).unwrap();
    changed
}

#[test]
fn module_local_typeref_uses_its_exact_definition_name_policy() {
    for (index, &bytes) in IMAGES.iter().enumerate() {
        let changed = module_local_reference_variant(bytes);
        // The optional compiler gate consumes these independent mutated inputs,
        // not a renderer-produced name table or implementation-derived oracle.
        if let Some(directory) = std::env::var_os("ILSPY_LITE_NAMING_FIXTURE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(format!("local-ref-{index}.bin")), &changed).unwrap();
        }
        let actual =
            decompile_type_with_style(&changed, "NamingCases.CollisionHost", OutputStyle::Readable)
                .unwrap();
        assert!(
            actual.text.contains("class __name_5131326031"),
            "{}",
            actual.text
        );
        assert!(
            actual
                .text
                .contains("NamingCases.CollisionHost.__name_5131326031 Left;"),
            "{}",
            actual.text
        );
    }
}

#[test]
fn ambiguous_or_cyclic_local_references_are_not_aliased_into_a_definition() {
    for &bytes in IMAGES {
        let base = module_local_reference_variant(bytes);
        let md = cildec::PeImage::parse(&base).unwrap().metadata().unwrap();
        let tables = md.tables();
        let reference = tables
            .iter::<cildec::tables::TypeRefRow>()
            .map(|(rid, row)| (rid, row.unwrap()))
            .find(|(_, row)| md.strings().str(row.type_name).unwrap() == "Q12`1")
            .unwrap()
            .0;
        let name_index = tables
            .raw_column(cildec::TableId::TypeRef, reference, 1)
            .unwrap();
        let user = tables
            .iter::<cildec::tables::TypeDefRow>()
            .map(|(rid, row)| (rid, row.unwrap()))
            .find(|(_, row)| md.strings().str(row.type_name).unwrap() == "__name_3C583E5F5F")
            .unwrap()
            .0;
        let row = tables.row_bytes(cildec::TableId::TypeDef, user).unwrap();
        assert_eq!(row.len(), 14);
        let at = row.as_ptr() as usize - base.as_ptr() as usize;
        let mut ambiguous = base.clone();
        ambiguous[at + 4..at + 6]
            .copy_from_slice(&u16::try_from(name_index).unwrap().to_le_bytes());
        let row = tables
            .row_bytes(cildec::TableId::TypeRef, reference)
            .unwrap();
        let at = row.as_ptr() as usize - base.as_ptr() as usize;
        let mut cyclic = base.clone();
        cyclic[at..at + 2]
            .copy_from_slice(&u16::try_from(reference << 2 | 3).unwrap().to_le_bytes());
        for (changed, reason) in [(ambiguous, "ambiguous"), (cyclic, "cyclic")] {
            let context = ilspy_lite::DependencyContext::new(changed.clone()).unwrap();
            assert!(
                context
                    .resolve_type(0x01000000 | reference)
                    .unwrap_err()
                    .to_string()
                    .contains(reason)
            );
            let actual = decompile_type_with_style(
                &changed,
                "NamingCases.CollisionHost",
                OutputStyle::Readable,
            )
            .unwrap();
            assert!(
                actual.text.contains("// Unreadable field"),
                "{}",
                actual.text
            );
        }
    }
}

#[test]
fn a_backtick_in_a_nongeneric_metadata_name_is_not_generic_context() {
    for &bytes in IMAGES {
        let changed = rename_type(bytes, "Plain", "X`Y__");
        let actual =
            decompile_type_with_style(&changed, "NamingCases.CollisionHost", OutputStyle::Readable)
                .unwrap();
        assert_eq!(actual.summary.decompiled, 3, "{}", actual.text);
        assert!(
            actual.text.contains("class __name_5860595F5F"),
            "{}",
            actual.text
        );
        assert!(
            actual
                .text
                .contains("CollisionHost.__name_5860595F5F Left;"),
            "{}",
            actual.text
        );
        assert!(!actual.text.contains("class X\n"), "{}", actual.text);
    }
}

#[test]
fn namespace_keywords_are_escaped_in_both_declarations_and_type_references() {
    for &bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let namespace = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .map(|(_, row)| row.unwrap())
            .map(|row| md.strings().str(row.type_namespace).unwrap())
            .find(|ns| *ns == "NamingCases")
            .expect("owned namespace");
        let at = namespace.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut changed = bytes.to_vec();
        changed[at..at + namespace.len()].copy_from_slice(b"namespace.a");
        let result =
            decompile_type_with_style(&changed, "namespace.a.CollisionHost", OutputStyle::Readable)
                .unwrap();
        assert!(
            result.text.starts_with("namespace @namespace.a;"),
            "{}",
            result.text
        );
        assert!(
            result
                .text
                .contains("@namespace.a.CollisionHost.Plain Left;"),
            "{}",
            result.text
        );
    }
}

#[test]
fn generated_method_name_does_not_block_captured_i4_reads_and_writes() {
    for &bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == CLOSURE && m.simple_name == "<Accumulator>b__0")
            .expect("owned captured-I4 method");
        for style in [OutputStyle::Readable, OutputStyle::Details] {
            let actual = decompile_with_style(bytes, method.token, style).unwrap();
            assert_eq!(actual.status, Status::Decompiled, "{}", actual.text);
        }
    }
}

#[test]
fn static_initialization_is_not_admitted_as_an_ordinary_renamed_method() {
    for &bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "NamingCases.Special" && m.simple_name == ".cctor")
            .expect("owned static constructor");
        let actual = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(actual.status, Status::Partial, "{}", actual.text);
        assert!(actual.text.contains("// IL_") && actual.text.contains("ret"));
    }
}

#[test]
fn generated_owner_name_does_not_block_an_admitted_object_base_constructor() {
    for &bytes in IMAGES {
        let ctor = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == CLOSURE && m.simple_name == ".ctor")
            .expect("owned closure constructor");
        for style in [OutputStyle::Readable, OutputStyle::Details] {
            let result = decompile_with_style(bytes, ctor.token, style).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        }
    }
}
