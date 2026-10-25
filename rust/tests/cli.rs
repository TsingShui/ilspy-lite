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
