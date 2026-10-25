// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, decompile_type, decompile_type_with_style};
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
    assert!(readable.text.contains("// IL fallback:") && readable.text.contains("--details"));
    for noise in [
        ".type-flags",
        ".method-status",
        "signature-bytes=",
        "parsed=",
        "raw-row-bytes=",
        "// IL_",
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
fn simple_expression_view_removes_constant_argument_copies_and_unused_labels() {
    let result =
        decompile_type_with_style(IMAGE, "ClassCases.Mixed", OutputStyle::Readable).unwrap();
    assert!(
        result.text.contains("return unchecked(value + 1);"),
        "{}",
        result.text
    );
    assert!(!result.text.contains("IL_0000:"), "{}", result.text);
    assert!(!result.text.contains("int v0 = value;"), "{}", result.text);
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
    }
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
    assert!(
        String::from_utf8(default.stdout)
            .unwrap()
            .contains("public static class Mixed")
    );
    assert_eq!(
        String::from_utf8(details.stdout).unwrap(),
        decompile_type(IMAGE, "ClassCases.Mixed").unwrap().text
    );
}
