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
fn scan_reports_every_method_with_the_same_status_as_token_decompilation() {
    let out = run(&["scan", fixture()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let methods = ilspy_lite::list_methods(include_bytes!("fixtures/release.bin")).unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let records: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), 59);
    for (method, record) in methods.iter().zip(&records) {
        assert_eq!(record["token"], format!("0x{:08X}", method.token));
        let result =
            ilspy_lite::decompile(include_bytes!("fixtures/release.bin"), method.token).unwrap();
        assert_eq!(record["status"], format!("{:?}", result.status));
        if result.status == ilspy_lite::Status::Partial {
            let reason = result
                .text
                .lines()
                .find_map(|line| line.strip_prefix("// IL FALLBACK: "))
                .unwrap();
            assert_eq!(record["reason"], reason);
        } else {
            assert!(record["reason"].is_null(), "{record}");
        }
    }
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

#[test]
fn scan_records_the_selected_reference_set_without_changing_any_status() {
    let plain = run(&["scan", fixture()]);
    let with_references = run(&[
        "scan",
        fixture(),
        "--no-auto-references",
        "--reference",
        &format!(
            "{}/tests/fixtures/cast-release.bin",
            env!("CARGO_MANIFEST_DIR")
        ),
    ]);
    assert!(
        with_references.status.success(),
        "{}",
        String::from_utf8_lossy(&with_references.stderr)
    );
    // Selecting references must not move a single method between statuses. A
    // blocker may gain the concrete refusal behind it, which is diagnostic text,
    // so the comparison is over statuses rather than over whole records.
    let statuses = |stdout: Vec<u8>| -> Vec<(String, String)> {
        String::from_utf8(stdout)
            .unwrap()
            .lines()
            .map(|line| {
                let record: serde_json::Value = serde_json::from_str(line).unwrap();
                (
                    record["token"].as_str().unwrap().to_string(),
                    record["status"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    };
    assert_eq!(statuses(plain.stdout), statuses(with_references.stdout));
    let stderr = String::from_utf8(with_references.stderr).unwrap();
    let summary = stderr
        .lines()
        .find(|line| line.starts_with("references="))
        .unwrap_or_else(|| panic!("no reference summary in {stderr}"));
    let declared: usize = summary
        .strip_prefix("references=")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap();
    assert!(declared > 0, "{summary}");
    let listed = stderr
        .lines()
        .filter(|line| line.starts_with("reference=0x"))
        .count();
    assert_eq!(listed, declared, "{stderr}");
}

#[test]
fn scan_surfaces_a_reference_directory_that_was_not_available() {
    let missing = format!(
        "{}/tests/fixtures/absent-reference-directory",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = run(&[
        "scan",
        fixture(),
        "--no-auto-references",
        "--reference-dir",
        &missing,
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("reference directory unavailable:"),
        "{stderr}"
    );
}

#[test]
fn scan_rejects_an_unknown_option_instead_of_ignoring_it() {
    let out = run(&["scan", fixture(), "--not-an-option"]);
    assert!(!out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().is_empty());
}

#[test]
fn identities_report_the_image_and_the_references_it_declares() {
    let out = run(&["identities", fixture()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    let record: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(
        record["identity"].as_str().unwrap().split(',').next(),
        Some("Fixtures")
    );
    let references = record["references"].as_array().unwrap();
    assert!(
        references
            .iter()
            .any(|r| r.as_str().unwrap().starts_with("System.Runtime, ")),
        "{record}"
    );
    assert!(record.get("error").is_none(), "{record}");
}

#[test]
fn identities_reject_a_file_that_is_not_an_assembly() {
    let out = run(&[
        "identities",
        &format!("{}/README.md", env!("CARGO_MANIFEST_DIR")),
    ]);
    // A per-file rejection is reported on its own line and does not claim the run failed.
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    let record: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert!(!record["error"].as_str().unwrap().is_empty(), "{record}");
}

#[test]
fn scan_and_decompile_agree_when_the_same_reference_set_is_given() {
    let fixture = format!(
        "{}/tests/fixtures/external-enum-release.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let library = format!(
        "{}/tests/fixtures/external-enum-library-release.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let options = ["--no-auto-references", "--reference", &library];
    let mut scan = vec!["scan", &fixture];
    scan.extend(options);
    let scanned = run(&scan);
    assert!(
        scanned.status.success(),
        "{}",
        String::from_utf8_lossy(&scanned.stderr)
    );
    let records: Vec<serde_json::Value> = String::from_utf8(scanned.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(!records.is_empty());
    let mut recovered = 0;
    for record in &records {
        let token = record["token"].as_str().unwrap();
        let mut single = vec!["decompile", &fixture, "--method", token];
        single.extend(options);
        let out = run(&single);
        let summary = String::from_utf8(out.stderr).unwrap();
        let status = if summary.contains("decompiled=1") {
            recovered += 1;
            "Decompiled"
        } else if summary.contains("partial=1") {
            "Partial"
        } else if summary.contains("declarations=1") {
            "Declaration"
        } else {
            panic!("unexpected decompile summary for {token}: {summary}")
        };
        assert_eq!(
            record["status"].as_str().unwrap(),
            status,
            "{token}: {summary}"
        );
    }
    // The declared reference is what admits the external storage; a fixture that
    // stops exercising it would otherwise pass this parity check trivially.
    assert!(recovered >= 7, "only {recovered} methods recovered");
}
