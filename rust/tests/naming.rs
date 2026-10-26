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
