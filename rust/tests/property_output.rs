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
        assert!(
            actual.text.contains(&format!("return base.@{keyword};")),
            "{}",
            actual.text
        );
    }
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
    assert!(actual.text.contains("base.Enabled"));
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
fn exact_base_property_relationships_render_reads_and_writes_without_losing_dispatch() {
    let mut context = DependencyContext::new(INPUT.to_vec()).unwrap();
    context
        .add_reference("owned BindingLibrary", LIBRARY.to_vec())
        .unwrap();
    let actual = context
        .decompile_type("BindingCases.Derived", OutputStyle::Readable)
        .unwrap();
    assert!(
        actual.text.contains("return base.Enabled;"),
        "{}",
        actual.text
    );
    assert!(actual.text.contains("base.Enabled = "), "{}", actual.text);
    assert!(
        actual.text.contains("return base.VirtualFlag;"),
        "a direct base call must not dispatch to the override: {}",
        actual.text
    );
    assert!(
        actual.text.contains("return base.ConstantFlag;"),
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
