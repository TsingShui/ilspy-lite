// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use std::process::{Command, Output};
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .args(args)
        .output()
        .unwrap()
}
fn fixture() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/release.bin")
}

#[test]
fn type_selection_accepts_empty_and_generic_types_without_methods() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/class-release.bin"
    );
    for name in [
        "ClassCases.Empty",
        "ClassCases.EmptyGeneric`1",
        "ClassCases.Outer/EmptyNested",
    ] {
        let out = run(&["decompile", fixture, "--type", name, "--details"]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains(name));
        assert_eq!(
            String::from_utf8_lossy(&out.stderr).trim(),
            "decompiled=0 declarations=0 partial=0 failed=0"
        );
    }
}

#[test]
fn class_cli_reports_one_method_failure_without_hiding_other_bodies() {
    let source = include_bytes!("fixtures/class-release.bin");
    let token = ilspy_lite::list_methods(source)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ClassCases.Mixed" && m.simple_name == "Unsupported")
        .unwrap()
        .token;
    let pe = cildec::PeImage::parse(source).unwrap();
    let md = pe.metadata().unwrap();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::MethodDef, token & 0xffffff)
        .unwrap();
    let offset = row.as_ptr() as usize - source.as_ptr() as usize;
    let mut patched = source.to_vec();
    patched[offset..offset + 4].copy_from_slice(&0u32.to_le_bytes());
    let path = std::env::temp_dir().join(format!("ilspy-class-cli-{}.bin", std::process::id()));
    std::fs::write(&path, patched).unwrap();
    let out = run(&[
        "decompile",
        path.to_str().unwrap(),
        "--type",
        "ClassCases.Mixed",
        "--details",
    ]);
    std::fs::remove_file(path).unwrap();
    assert_eq!(out.status.code(), Some(1));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("@Good("));
    assert!(text.contains(&format!("FAILED token 0x{token:08X}")));
    assert_eq!(
        String::from_utf8(out.stderr).unwrap().trim(),
        "decompiled=1 declarations=0 partial=0 failed=1"
    );
}

#[test]
fn invalid_class_metadata_is_not_counted_as_a_failed_method() {
    let source = include_bytes!("fixtures/class-release.bin");
    let pe = cildec::PeImage::parse(source).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Empty")
        .unwrap()
        .0;
    let row = md
        .tables()
        .row_bytes(cildec::TableId::TypeDef, rid)
        .unwrap();
    assert_eq!(row.len(), 14);
    let offset = row.as_ptr() as usize - source.as_ptr() as usize;
    let mut patched = source.to_vec();
    patched[offset + 8..offset + 10].copy_from_slice(&0xfffdu16.to_le_bytes());
    let path =
        std::env::temp_dir().join(format!("ilspy-class-metadata-{}.bin", std::process::id()));
    std::fs::write(&path, patched).unwrap();
    let out = run(&[
        "decompile",
        path.to_str().unwrap(),
        "--type",
        "ClassCases.Empty",
        "--details",
    ]);
    std::fs::remove_file(path).unwrap();
    assert_eq!(out.status.code(), Some(1));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("ClassCases.Empty"));
    assert!(text.contains(".type-flags"));
    assert!(text.contains("FAILED metadata"));
    assert_eq!(
        String::from_utf8(out.stderr).unwrap().trim(),
        "decompiled=0 declarations=0 partial=0 failed=0\nmetadata_failed=1"
    );
}

#[test]
fn inventory_and_token_cli_work_without_dotnet() {
    let out = run(&["list", fixture()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(text.lines().count(), 59);
    assert!(text.contains("MoveNext"));
    let out = run(&["decompile", fixture(), "--method", "0x06000019"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("checked("));
}

#[test]
fn partial_type_preserves_inventory_and_returns_nonzero() {
    let out = run(&[
        "decompile",
        fixture(),
        "--type",
        "LiteSamples.ControlFlowSample",
        "--details",
    ]);
    assert_eq!(out.status.code(), Some(2));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("IL FALLBACK"));
    assert!(text.contains("PreserveOldValue"));
    assert!(text.contains(".method-flags"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("partial=1"));
}

#[test]
fn type_output_keeps_nested_state_machine_methods_visible() {
    let out = run(&["decompile", fixture(), "--type", "LiteSamples.AsyncSample"]);
    assert_eq!(out.status.code(), Some(2));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("MoveNext"));
    assert!(text.contains("SetStateMachine"));
}

#[test]
fn malformed_requests_are_not_silent_success() {
    for args in [
        vec![],
        vec!["decompile", fixture()],
        vec!["il", fixture(), "--method", "0x0"],
        vec!["list", "missing.dll"],
        vec!["decompile", fixture(), "--type", "Absent"],
    ] {
        let out = run(&args);
        assert_eq!(out.status.code(), Some(1));
        assert!(!out.stderr.is_empty());
    }
}
