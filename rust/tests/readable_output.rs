// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{
    OutputStyle, Status, decompile_type, decompile_type_with_style, decompile_with_style,
    disassemble, list_methods,
};
use std::process::Command;

const IMAGE: &[u8] = include_bytes!("fixtures/class-release.bin");

#[test]
fn readable_class_view_uses_declarations_without_dumping_diagnostic_records() {
    let readable =
        decompile_type_with_style(IMAGE, "ClassCases.Mixed", OutputStyle::Readable).unwrap();
    let detailed = decompile_type(IMAGE, "ClassCases.Mixed").unwrap();
    assert_eq!(readable.summary, detailed.summary);
    assert!(
        readable.text.contains("namespace ClassCases;"),
        "{}",
        readable.text
    );
    assert!(readable.text.contains("public static class Mixed"));
    assert!(
        readable.text.contains("public static int Good(int value)"),
        "{}",
        readable.text
    );
    assert!(
        readable
            .text
            .contains("public static string Unsupported(string value)")
    );
    assert!(readable.text.contains("// IL fallback:"));
    assert!(!readable.text.contains("see --details"));
    assert!(readable.text.contains("// IL_0001: callvirt"));
    assert!(readable.text.contains("// IL_0006: ret"));
    let good = readable
        .text
        .split("Good(int value)")
        .nth(1)
        .unwrap()
        .split("public static string Unsupported")
        .next()
        .unwrap();
    assert!(!good.contains("// IL_"), "{good}");
    for noise in [
        ".type-flags",
        ".method-status",
        "signature-bytes=",
        "parsed=",
        "raw-row-bytes=",
        "public static int @Good",
        "// token ",
    ] {
        assert!(
            !readable.text.contains(noise),
            "unexpected {noise}: {}",
            readable.text
        );
    }
    assert!(detailed.text.contains(".method-status") && detailed.text.contains(".code-bytes"));
}

#[test]
fn readable_fallback_keeps_complete_il_inside_its_method_and_class_body() {
    for (bytes, ty, name) in [
        (IMAGE, "ClassCases.Mixed", "Unsupported"),
        (
            include_bytes!("fixtures/class-debug.bin").as_slice(),
            "ClassCases.Mixed",
            "Unsupported",
        ),
        (
            include_bytes!("fixtures/release.bin").as_slice(),
            "LiteSamples.ControlFlowSample",
            "Divide",
        ),
    ] {
        let token = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == ty && m.simple_name == name)
            .unwrap()
            .token;
        let method = decompile_with_style(bytes, token, OutputStyle::Readable).unwrap();
        assert_eq!(method.status, Status::Partial);
        let class = decompile_type_with_style(bytes, ty, OutputStyle::Readable).unwrap();
        assert_eq!(class.summary.exit_code(), 2);
        let header = method.text.lines().next().unwrap();
        let body = method
            .text
            .split_once("\n{\n")
            .unwrap()
            .1
            .strip_suffix("}\n")
            .unwrap();
        let class_body = class
            .text
            .split(header)
            .nth(1)
            .unwrap()
            .split_once("{\n")
            .unwrap()
            .1
            .split("\n    }")
            .next()
            .unwrap();
        let il = disassemble(bytes, token).unwrap();
        // Disassembly's first two lines identify the method/flags; every body
        // line (including locals and EH ranges) belongs inside the fallback.
        for evidence in il.lines().skip(2) {
            for actual in [body, class_body] {
                let expected = format!("// {evidence}");
                assert!(
                    actual.lines().any(|l| l.trim() == expected.trim()),
                    "missing {evidence}: {actual}"
                );
            }
        }
        assert!(!body.contains("see --details"));
        if name == "Divide" {
            assert!(body.contains("// .try IL_"), "{body}");
        }
    }
}

#[test]
fn failed_readable_bodies_keep_raw_bytes_or_the_original_parse_error_inline() {
    let token = list_methods(IMAGE)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ClassCases.Methods" && m.simple_name == "Hidden")
        .unwrap()
        .token;
    let pe = cildec::PeImage::parse(IMAGE).unwrap();
    let md = pe.metadata().unwrap();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::MethodDef, token & 0xffffff)
        .unwrap();
    let offset = row.as_ptr() as usize - IMAGE.as_ptr() as usize;
    let body =
        cildec::MethodBody::from_image(&pe, &md.tables().method_def(token & 0xffffff).unwrap())
            .unwrap()
            .unwrap();
    assert_eq!(body.code, &[0x02, 0x2a]);
    let code = body.code.as_ptr() as usize - IMAGE.as_ptr() as usize;
    for failure in ["signature", "instruction", "rva"] {
        let mut patched = IMAGE.to_vec();
        match failure {
            "signature" => {
                patched[offset + 10..offset + 12].copy_from_slice(&0xffffu16.to_le_bytes())
            }
            "instruction" => patched[code] = 0xff,
            "rva" => patched[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes()),
            _ => unreachable!(),
        }
        let readable =
            decompile_type_with_style(&patched, "ClassCases.Methods", OutputStyle::Readable)
                .unwrap();
        let details = decompile_type(&patched, "ClassCases.Methods").unwrap();
        assert_eq!(readable.summary, details.summary);
        assert_eq!(readable.summary.failed, 1);
        assert_eq!(readable.summary.exit_code(), 1);
        assert!(readable.text.contains("// Failed"), "{}", readable.text);
        assert!(
            readable.text.contains("Pick(long value)"),
            "{}",
            readable.text
        );
        for entry in details.text.lines().filter(|l| {
            l.starts_with("// Raw IL bytes")
                || l.starts_with(&format!("// FAILED token 0x{token:08X}:"))
        }) {
            let expected = entry.replacen("FAILED", "Failed", 1);
            assert!(
                readable.text.lines().any(|l| l.trim() == expected),
                "{}",
                readable.text
            );
        }
        if failure == "signature" {
            assert!(
                readable
                    .text
                    .contains("// Raw IL bytes (undecoded): [02, 2A]")
            );
            assert!(readable.text.contains("// IL_0000: ldarg.0 0"));
            assert!(readable.text.contains("// IL_0001: ret"));
        }
        if failure == "instruction" {
            assert!(
                readable
                    .text
                    .contains("// Raw IL bytes (undecoded): [FF, 2A]")
            );
            assert!(readable.text.contains("// Body evidence decoding failed:"));
        }
    }
}

#[test]
fn simple_expression_view_removes_constant_argument_copies_and_unused_labels() {
    let result =
        decompile_type_with_style(IMAGE, "ClassCases.Mixed", OutputStyle::Readable).unwrap();
    assert!(
        result.text.contains("return unchecked(value + 1);"),
        "{}",
        result.text
    );
    let good = result
        .text
        .split("Good(int value)")
        .nth(1)
        .unwrap()
        .split("public static string Unsupported")
        .next()
        .unwrap();
    assert!(!good.contains("IL_0000:"), "{good}");
    assert!(!good.contains("int v0 = value;"), "{good}");
}

#[test]
fn compacted_copy_chains_and_nested_indentation_stay_readable() {
    let bytes = include_bytes!("fixtures/reference-release.bin");
    let result =
        decompile_type_with_style(bytes, "ReferenceCases.Cell", OutputStyle::Readable).unwrap();
    let credit = result
        .text
        .split("public int Credit(int delta)")
        .nth(1)
        .unwrap()
        .split("public int Nested()")
        .next()
        .unwrap();
    assert!(
        !credit.lines().any(|line| line.trim().starts_with("int v")
            && line.trim().split_once(" = ").is_some_and(|(_, value)| value
                .strip_prefix('v')
                .is_some_and(|s| s.trim_end_matches(';').bytes().all(|b| b.is_ascii_digit())))),
        "{credit}"
    );
    let choose = result
        .text
        .split("Choose(")
        .nth(1)
        .unwrap()
        .split("public static ")
        .next()
        .unwrap();
    assert!(
        choose.lines().any(|line| line.starts_with("            ")),
        "{choose}"
    );
}

#[test]
fn boolean_comparison_returns_do_not_roundtrip_through_integer_temporaries() {
    let bytes = include_bytes!("fixtures/integer-release.bin");
    let result =
        decompile_type_with_style(bytes, "IntegerCases.Cases", OutputStyle::Readable).unwrap();
    let equal = result
        .text
        .split("public static bool Equal(")
        .nth(1)
        .unwrap()
        .split("public static ")
        .next()
        .unwrap();
    assert!(
        equal.contains("return (left == right);") || equal.contains("return (a == b);"),
        "{equal}"
    );
    assert!(
        !equal.contains("? 1 : 0") && !equal.contains("!= 0"),
        "{equal}"
    );
}

#[test]
fn return_only_branch_targets_can_replace_gotos_without_reordering_operations() {
    let bytes = include_bytes!("fixtures/reference-release.bin");
    let result =
        decompile_type_with_style(bytes, "ReferenceCases.Cell", OutputStyle::Readable).unwrap();
    let read = result
        .text
        .split("public int Read()")
        .nth(1)
        .unwrap()
        .split("public int Credit(")
        .next()
        .unwrap();
    assert!(!read.contains("IL_"), "{read}");
    // Return-only switch successors can shed goto scaffolding; loops cannot.
    let integer = include_bytes!("fixtures/release.bin");
    let result = decompile_type_with_style(
        integer,
        "LiteSamples.ControlFlowSample",
        OutputStyle::Readable,
    )
    .unwrap();
    let select = result
        .text
        .split("Select(")
        .nth(1)
        .unwrap()
        .split("public static ")
        .next()
        .unwrap();
    assert!(!select.contains("goto IL_"), "{select}");
    assert_eq!(select.matches("return 7;").count(), 1, "{select}");
    assert_eq!(select.matches("return -1;").count(), 1, "{select}");
}

#[test]
fn field_old_value_is_not_delayed_until_after_a_store() {
    for bytes in [
        include_bytes!("fixtures/reference-release.bin").as_slice(),
        include_bytes!("fixtures/reference-debug.bin").as_slice(),
    ] {
        let result =
            decompile_type_with_style(bytes, "ReferenceCases.Cell", OutputStyle::Readable).unwrap();
        let credit = result
            .text
            .split("public int Credit(int delta)")
            .nth(1)
            .unwrap()
            .split("public int Nested()")
            .next()
            .unwrap();
        let read = credit.find("= this.Value;").unwrap();
        let store = credit.find("this.Value =").unwrap();
        let returned = credit.find("return ").unwrap();
        assert!(read < store && store < returned, "{credit}");
        assert!(!credit.contains("return this.Value;"), "{credit}");
        assert!(credit.contains("this.Value = unchecked("), "{credit}");
    }
}

#[test]
fn argument_renaming_does_not_rewrite_a_field_named_like_a_generated_argument() {
    let bytes = include_bytes!("fixtures/reference-release.bin");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let field = md.tables().field(1).unwrap();
    let raw = md.strings().str(field.name).unwrap().as_bytes();
    assert_eq!(raw, b"Value");
    let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset..offset + raw.len()].copy_from_slice(b"a0\0\0\0");
    let result =
        decompile_type_with_style(&patched, "ReferenceCases.Cell", OutputStyle::Readable).unwrap();
    assert!(result.text.contains("public int a0;"));
    assert!(result.text.contains("= this.a0;"), "{}", result.text);
    assert!(!result.text.contains("this.this"), "{}", result.text);
}

#[test]
fn type_namespace_segments_are_not_substituted_as_generated_arguments() {
    let bytes = include_bytes!("fixtures/reference-release.bin");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let row = md.tables().type_def(2).unwrap();
    let namespace = md.strings().str(row.type_namespace).unwrap().as_bytes();
    assert_eq!(namespace, b"ReferenceCases");
    let offset = namespace.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset..offset + namespace.len()].fill(0);
    patched[offset..offset + 2].copy_from_slice(b"a0");
    let result = decompile_type_with_style(&patched, "a0.Cell", OutputStyle::Readable).unwrap();
    assert!(!result.text.contains("this.Cell"), "{}", result.text);
    assert!(!result.text.contains("target.Cell"), "{}", result.text);
    assert!(result.text.contains("namespace a0;"));
}

#[test]
fn fallback_parameter_names_cannot_collide_with_existing_source_parameters() {
    let result =
        decompile_type_with_style(IMAGE, "ClassCases.PresentationCases", OutputStyle::Readable)
            .unwrap();
    assert!(
        !result.text.contains("Difference(int arg0, int arg0)"),
        "{}",
        result.text
    );
    assert!(
        !result.text.contains("return unchecked(arg0 - arg0);"),
        "{}",
        result.text
    );
    assert!(result.text.contains("this.v1 = 1;"), "{}", result.text);
    assert!(result.text.contains("this.a0 = a0;"), "{}", result.text);
}

#[test]
fn out_parameter_contract_is_retained_even_when_the_body_falls_back() {
    let result =
        decompile_type_with_style(IMAGE, "ClassCases.Methods", OutputStyle::Readable).unwrap();
    assert!(
        result.text.contains("ByRef(ref int input, out int output)"),
        "{}",
        result.text
    );
    assert!(!result.text.contains("ByRef(ref int input, ref int output)"));
}

#[test]
fn backing_field_declarations_and_accessors_use_the_same_token_alias() {
    let result =
        decompile_type_with_style(IMAGE, "ClassCases.Fields", OutputStyle::Readable).unwrap();
    let md = cildec::PeImage::parse(IMAGE).unwrap().metadata().unwrap();
    let (rid, _) = md
        .tables()
        .iter::<cildec::tables::FieldRow>()
        .find(|(_, row)| {
            md.strings().str(row.as_ref().unwrap().name).unwrap() == "<Auto>k__BackingField"
        })
        .unwrap();
    let alias = format!("__il_field_{:08X}", 0x04000000 | rid);
    assert!(
        result.text.contains(&format!("private int {alias};")),
        "{}",
        result.text
    );
    assert!(
        result.text.contains(&format!("this.{alias}")),
        "{}",
        result.text
    );
    assert!(!result.text.contains("__name_3C4175746F"));
}

#[test]
fn property_and_event_relationships_show_names_types_and_accessors_not_tokens() {
    let properties =
        decompile_type_with_style(IMAGE, "ClassCases.Properties", OutputStyle::Readable).unwrap();
    assert!(
        properties
            .text
            .contains("// property int Only { get: get_Only; }"),
        "{}",
        properties.text
    );
    assert!(
        properties.text.contains("// property int this[int index0]"),
        "{}",
        properties.text
    );
    assert!(!properties.text.contains("relationship 0x"));
    let events =
        decompile_type_with_style(IMAGE, "ClassCases.Events", OutputStyle::Readable).unwrap();
    assert!(
        events.text.contains(
            "// event System.EventHandler External { add: add_External; remove: remove_External; }"
        ),
        "{}",
        events.text
    );
    assert!(!events.text.contains("relationship 0x"));
    let generic =
        decompile_type_with_style(IMAGE, "ClassCases.GenericEvents`1", OutputStyle::Readable)
            .unwrap();
    assert!(
        generic.text.contains("// event System.Action<T> Updated"),
        "{}",
        generic.text
    );
    assert!(!generic.text.contains("System.Action<T0>"));
}

#[test]
fn readable_fields_and_nested_types_remain_visible() {
    let fields =
        decompile_type_with_style(IMAGE, "ClassCases.Fields", OutputStyle::Readable).unwrap();
    for declaration in [
        "public class Fields",
        "private int storage;",
        "public static uint Counter;",
        "public readonly string Label;",
        "public const int Limit = -7;",
        "public class Inner",
        "public int Nested;",
    ] {
        assert!(
            fields.text.contains(declaration),
            "missing {declaration}: {}",
            fields.text
        );
    }
    let empty =
        decompile_type_with_style(IMAGE, "ClassCases.EmptyGeneric`1", OutputStyle::Readable)
            .unwrap();
    assert!(
        empty.text.contains("public static class EmptyGeneric<T>"),
        "{}",
        empty.text
    );
    assert_eq!(empty.summary.exit_code(), 0);
}

#[test]
fn cli_defaults_to_readable_and_details_preserves_the_complete_analysis() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/class-release.bin"
    );
    let default = Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .args(["decompile", path, "--type", "ClassCases.Mixed"])
        .output()
        .unwrap();
    let details = Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .args(["decompile", path, "--type", "ClassCases.Mixed", "--details"])
        .output()
        .unwrap();
    assert_eq!(default.status.code(), Some(2));
    assert_eq!(details.status.code(), Some(2));
    assert_eq!(default.stderr, details.stderr);
    let text = String::from_utf8(default.stdout).unwrap();
    assert!(text.contains("public static class Mixed"));
    let fallback = text.split("Unsupported(string value)").nth(1).unwrap();
    assert!(fallback.contains("// IL_0001: callvirt"), "{fallback}");
    assert!(fallback.contains("// IL_0006: ret"), "{fallback}");
    assert!(!fallback.contains("see --details"));
    assert_eq!(
        String::from_utf8(details.stdout).unwrap(),
        decompile_type(IMAGE, "ClassCases.Mixed").unwrap().text
    );
}
