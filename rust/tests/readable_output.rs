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
