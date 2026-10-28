// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{AssemblyResolution, DependencyContext, DependencyLimits, OutputStyle};

const MAIN: &[u8] = include_bytes!("fixtures/external-release.bin");
const DEPENDENCY: &[u8] = include_bytes!("fixtures/external-library-release.bin");

// Author identity-only metadata in an owned image. The replacement blob lives
// in unused-for-this-test attribute payload space; no signature is authenticated
// and no method or attribute is executed.
fn identity_key(bytes: &[u8], reference: bool, key: &[u8], full: bool) -> Vec<u8> {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let payload = md
        .tables()
        .iter::<cildec::tables::CustomAttributeRow>()
        .map(|(_, row)| md.blobs().get(row.unwrap().value).unwrap())
        .find(|value| value.len() > key.len())
        .unwrap();
    assert!(key.len() < 128);
    let start = payload.as_ptr() as usize - bytes.as_ptr() as usize;
    let blob_index = payload.as_ptr() as usize - md.stream("#Blob").unwrap().data.as_ptr() as usize;
    let (table, rid, flags_offset, key_offset) = if reference {
        let rid = md
            .tables()
            .iter::<cildec::tables::AssemblyRefRow>()
            .find(|(_, row)| {
                md.strings().str(row.as_ref().unwrap().name).unwrap() == "ExternalLibrary"
            })
            .unwrap()
            .0;
        (cildec::TableId::AssemblyRef, rid, 8, 12)
    } else {
        (cildec::TableId::Assembly, 1, 12, 16)
    };
    let row = md.tables().row_bytes(table, rid).unwrap();
    assert_eq!(row.len(), if reference { 20 } else { 22 });
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let flags = u32::from_le_bytes(row[flags_offset..flags_offset + 4].try_into().unwrap());
    let mut patched = bytes.to_vec();
    patched[start] = key.len() as u8;
    patched[start + 1..start + 1 + key.len()].copy_from_slice(key);
    patched[offset + flags_offset..offset + flags_offset + 4]
        .copy_from_slice(&((flags & !1) | u32::from(full)).to_le_bytes());
    patched[offset + key_offset..offset + key_offset + 2]
        .copy_from_slice(&(blob_index as u16).to_le_bytes());
    patched
}

#[test]
fn full_public_keys_match_their_cli_tokens_but_not_different_tokens() {
    // ECMA public key and its standardized b77a5c561934e089 token.
    let key = [0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0];
    let token = [0xb7, 0x7a, 0x5c, 0x56, 0x19, 0x34, 0xe0, 0x89];
    for (requested, full) in [(token.as_slice(), false), (key.as_slice(), true)] {
        let root = identity_key(MAIN, true, requested, full);
        let mut context = DependencyContext::new(root).unwrap();
        let reference = context
            .references()
            .iter()
            .find(|r| r.identity.name == "ExternalLibrary")
            .unwrap()
            .clone();
        assert_eq!(reference.identity.public_key_token, token);
        context
            .add_reference(
                "signed-identity.dll",
                identity_key(DEPENDENCY, false, &key, true),
            )
            .unwrap();
        assert!(matches!(
            context.resolve(reference.token).unwrap(),
            AssemblyResolution::Resolved { .. }
        ));
    }
    let mut wrong = token;
    wrong[0] ^= 1;
    let mut context = DependencyContext::new(identity_key(MAIN, true, &wrong, false)).unwrap();
    let reference = context
        .references()
        .iter()
        .find(|r| r.identity.name == "ExternalLibrary")
        .unwrap()
        .token;
    context
        .add_reference("wrong-key.dll", identity_key(DEPENDENCY, false, &key, true))
        .unwrap();
    assert!(matches!(
        context.resolve(reference).unwrap(),
        AssemblyResolution::Missing { .. }
    ));
}

#[test]
fn malformed_key_flags_and_token_lengths_are_not_normalized_into_valid_identities() {
    for (key, full) in [(vec![], true), (vec![1; 7], false)] {
        let context = DependencyContext::new(identity_key(MAIN, true, &key, full)).unwrap();
        assert_eq!(context.reference_failures().len(), 1);
        let token = context.reference_failures()[0].token;
        assert!(matches!(
            context.resolve(token).unwrap(),
            AssemblyResolution::Invalid { .. }
        ));
    }
    let mut context = DependencyContext::new(MAIN.to_vec()).unwrap();
    assert!(
        context
            .add_reference(
                "invalid-definition-key.dll",
                identity_key(DEPENDENCY, false, &[1; 8], false)
            )
            .is_err()
    );
    assert!(
        context
            .add_reference(
                "missing-definition-key.dll",
                identity_key(DEPENDENCY, false, &[], true)
            )
            .is_err()
    );
}

#[test]
fn same_identity_with_different_content_is_conflicting_not_first_file_wins() {
    let mut context = DependencyContext::new(MAIN.to_vec()).unwrap();
    let token = context
        .references()
        .iter()
        .find(|r| r.identity.name == "ExternalLibrary")
        .unwrap()
        .token;
    context
        .add_reference("release.dll", DEPENDENCY.to_vec())
        .unwrap();
    context
        .add_reference(
            "debug.dll",
            include_bytes!("fixtures/external-library-debug.bin").to_vec(),
        )
        .unwrap();
    match context.resolve(token).unwrap() {
        AssemblyResolution::Conflicting { requested, sources } => {
            assert_eq!(requested.name, "ExternalLibrary");
            assert_eq!(sources, ["release.dll", "debug.dll"]);
        }
        other => panic!("Different explicit assemblies were silently conflated: {other:?}"),
    }
}

#[test]
fn immutable_byte_cache_obeys_assembly_and_total_byte_limits() {
    let mut context = DependencyContext::with_limits(
        MAIN.to_vec(),
        DependencyLimits {
            max_assemblies: 2,
            max_bytes: MAIN.len() + DEPENDENCY.len(),
            ..DependencyLimits::default()
        },
    )
    .unwrap();
    context
        .add_reference("first.dll", DEPENDENCY.to_vec())
        .unwrap();
    context
        .add_reference("first.dll", DEPENDENCY.to_vec())
        .unwrap();
    context
        .add_reference("same-bytes.dll", DEPENDENCY.to_vec())
        .unwrap();
    let token = context
        .references()
        .iter()
        .find(|r| r.identity.name == "ExternalLibrary")
        .unwrap()
        .token;
    assert!(matches!(
        context.resolve(token).unwrap(),
        AssemblyResolution::Resolved { .. }
    ));
    assert!(
        context
            .add_reference("another.dll", MAIN.to_vec())
            .unwrap_err()
            .to_string()
            .contains("limit")
    );
    let mut changed = DEPENDENCY.to_vec();
    changed[0] ^= 1;
    assert!(
        context
            .add_reference("first.dll", changed)
            .unwrap_err()
            .to_string()
            .contains("changed")
    );
    assert!(matches!(
        context.resolve(token).unwrap(),
        AssemblyResolution::Resolved { .. }
    ));
    let mut bytes_limited = DependencyContext::with_limits(
        MAIN.to_vec(),
        DependencyLimits {
            max_assemblies: 5,
            max_bytes: MAIN.len(),
            ..DependencyLimits::default()
        },
    )
    .unwrap();
    assert!(
        bytes_limited
            .add_reference("too-large.dll", DEPENDENCY.to_vec())
            .unwrap_err()
            .to_string()
            .contains("byte limit")
    );
    assert!(
        DependencyContext::with_limits(
            MAIN.to_vec(),
            DependencyLimits {
                max_assemblies: 0,
                max_bytes: usize::MAX,
                ..DependencyLimits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn bounded_reader_cannot_register_a_truncated_prefix_as_a_cached_snapshot() {
    let mut context = DependencyContext::with_limits(
        MAIN.to_vec(),
        DependencyLimits {
            max_bytes: MAIN.len() + 2 * DEPENDENCY.len() - 1,
            ..DependencyLimits::default()
        },
    )
    .unwrap();
    context
        .add_reference("complete.dll", DEPENDENCY.to_vec())
        .unwrap();
    let remaining = context.remaining_bytes();
    assert_eq!(remaining, DEPENDENCY.len() - 1);
    let mut longer_file = DEPENDENCY.to_vec();
    longer_file.push(0x42);
    let error = context
        .add_reference_reader("truncated-alias.dll", std::io::Cursor::new(longer_file))
        .expect_err("an overflow sentinel must be rejected before cache lookup");
    assert!(error.to_string().contains("read limit"));
    assert_eq!(context.remaining_bytes(), remaining);
    let details = context
        .decompile_type("ExternalCases.Cells", OutputStyle::Details)
        .unwrap();
    assert!(
        !details.text.contains("truncated-alias.dll"),
        "{}",
        details.text
    );
    // Complete bytes supplied directly still provide proof of an exact alias.
    context
        .add_reference("complete-copy.dll", DEPENDENCY.to_vec())
        .unwrap();
    assert_eq!(context.remaining_bytes(), remaining);
}

#[test]
fn dependency_reference_tables_are_bounded_before_collecting_identities() {
    let error = DependencyContext::with_limits(
        MAIN.to_vec(),
        DependencyLimits {
            max_references_per_image: 1,
            ..DependencyLimits::default()
        },
    )
    .err()
    .expect("two references must exceed a one-reference bound");
    assert!(error.to_string().contains("reference limit"));
}

#[test]
fn wrong_version_culture_or_file_name_cannot_override_metadata_identity() {
    let pe = cildec::PeImage::parse(DEPENDENCY).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().row_bytes(cildec::TableId::Assembly, 1).unwrap();
    assert_eq!(row.len(), 22);
    let offset = row.as_ptr() as usize - DEPENDENCY.as_ptr() as usize;
    let name = md
        .strings()
        .str(md.tables().assembly(1).unwrap().name)
        .unwrap()
        .as_bytes();
    let name_offset = name.as_ptr() as usize - DEPENDENCY.as_ptr() as usize;
    for mismatch in ["version", "culture", "name"] {
        let mut wrong = DEPENDENCY.to_vec();
        match mismatch {
            "version" => wrong[offset + 4..offset + 6].copy_from_slice(&1u16.to_le_bytes()),
            "culture" => wrong[offset + 20..offset + 22].copy_from_slice(&row[18..20]),
            "name" => wrong[name_offset] = b'X',
            _ => unreachable!(),
        }
        let mut context = DependencyContext::new(MAIN.to_vec()).unwrap();
        let token = context
            .references()
            .iter()
            .find(|r| r.identity.name == "ExternalLibrary")
            .unwrap()
            .token;
        context.add_reference("ExternalLibrary.dll", wrong).unwrap();
        assert!(
            matches!(
                context.resolve(token).unwrap(),
                AssemblyResolution::Missing { .. }
            ),
            "{mismatch}"
        );
        assert!(
            context
                .add_reference("broken.dll", b"not a managed PE".to_vec())
                .is_err()
        );
        context
            .add_reference("matching.dll", DEPENDENCY.to_vec())
            .unwrap();
        assert!(matches!(
            context.resolve(token).unwrap(),
            AssemblyResolution::Resolved { .. }
        ));
    }
}

#[test]
fn context_decompilation_keeps_bodies_and_reports_dependency_provenance_separately() {
    let mut context = DependencyContext::new(MAIN.to_vec()).unwrap();
    context
        .add_reference("fixture-library.dll", DEPENDENCY.to_vec())
        .unwrap();
    let details = context
        .decompile_type("ExternalCases.Cells", OutputStyle::Details)
        .unwrap();
    let isolated = ilspy_lite::decompile_type(MAIN, "ExternalCases.Cells").unwrap();
    // Supplied references may now admit bodies that are gated without them, so the
    // two runs need not agree method for method. What must hold is that the bodies
    // stay bodies and that provenance remains a separate trailing report.
    let (_, report) = details
        .text
        .split_once("\n.dependency-main ")
        .expect("dependency details carry a provenance report");
    assert!(!isolated.text.contains(".dependency-main"));
    assert!(details.summary.failed <= isolated.summary.failed);
    assert!(details.summary.decompiled >= isolated.summary.decompiled);
    assert!(report.contains("status=resolved") && report.contains("fixture-library.dll"));
    assert!(report.contains("status=missing") && report.contains("System.Runtime"));
    assert!(
        report.contains(".dependency-candidate ")
            && details.text.contains("identity=\"ExternalLibrary,")
    );
    let readable = context
        .decompile_type("ExternalCases.Cells", OutputStyle::Readable)
        .unwrap();
    assert_eq!(readable.summary, details.summary);
    assert!(readable.text.contains("public class Cells"));
    // Provenance moved to the details view.
    assert!(
        !readable.text.contains("Dependency unresolved:"),
        "{}",
        readable.text
    );
    assert!(!readable.text.contains(".dependency "));
}

#[test]
fn cli_explicit_reference_reports_identity_and_keeps_missing_dependencies_nonfatal() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
    let main = format!("{fixture}external-release.bin");
    let dependency = format!("{fixture}external-library-release.bin");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .args([
            "decompile",
            &main,
            "--type",
            "ExternalCases.Cells",
            "--reference",
            &dependency,
            "--no-auto-references",
            "--details",
        ])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(
        text.contains("status=resolved") && text.contains("external-library-release.bin"),
        "{text}"
    );
    assert!(text.contains("status=missing") && text.contains("System.Runtime"));
    assert!(!text.contains("IL FALLBACK"));
    let method = ilspy_lite::list_methods(MAIN)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "DirectFlag")
        .unwrap()
        .token;
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .args([
            "decompile",
            &main,
            "--method",
            &format!("0x{method:08X}"),
            "--details",
            "--reference",
            &dependency,
            "--no-auto-references",
        ])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("status=resolved")
    );
}

#[test]
fn cli_reference_directories_try_later_exact_identity_and_explicit_files_take_precedence() {
    let root = std::env::temp_dir().join(format!("ilspy-dependency-dirs-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    // A matching filename is not a matching assembly.
    std::fs::write(first.join("ExternalLibrary.dll"), MAIN).unwrap();
    std::fs::write(second.join("ExternalLibrary.dll"), DEPENDENCY).unwrap();
    let main = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/external-release.bin"
    );
    let explicit = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/external-library-debug.bin"
    );
    for use_explicit in [false, true] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"));
        command
            .args([
                "decompile",
                main,
                "--type",
                "ExternalCases.Cells",
                "--no-auto-references",
                "--details",
                "--reference-dir",
            ])
            .arg(&first)
            .arg("--reference-dir")
            .arg(&second);
        if use_explicit {
            command.args(["--reference", explicit]);
        }
        let result = command.output().unwrap();
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let text = String::from_utf8(result.stdout).unwrap();
        let resolved = text
            .lines()
            .find(|l| l.contains("status=resolved") && l.contains("ExternalLibrary"))
            .unwrap();
        assert!(
            resolved.contains(if use_explicit {
                "external-library-debug.bin"
            } else {
                "second"
            }),
            "{resolved}"
        );
        assert!(!text.contains("status=conflicting"), "{text}");
    }
    std::fs::write(
        first.join("ExternalLibrary.dll"),
        include_bytes!("fixtures/external-library-debug.bin"),
    )
    .unwrap();
    for (leading, later, expected) in [(&first, &second, "first"), (&second, &first, "second")] {
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
            .args([
                "decompile",
                main,
                "--type",
                "ExternalCases.Cells",
                "--reference-dir",
            ])
            .arg(leading)
            .arg("--reference-dir")
            .arg(later)
            .args(["--no-auto-references", "--details"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(0));
        let text = String::from_utf8(result.stdout).unwrap();
        let selected = text
            .lines()
            .find(|l| l.contains("status=resolved") && l.contains("ExternalLibrary"))
            .unwrap();
        assert!(selected.contains(expected), "{selected}");
    }
    std::fs::write(first.join("ExternalLibrary.dll"), b"broken PE").unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .args([
            "decompile",
            main,
            "--type",
            "ExternalCases.Cells",
            "--reference-dir",
        ])
        .arg(&first)
        .arg("--reference-dir")
        .arg(&second)
        .args(["--no-auto-references", "--details"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("Dependency input rejected:")
    );
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("dependency warning:")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn metadata_reference_names_cannot_escape_a_supplied_directory() {
    let pe = cildec::PeImage::parse(MAIN).unwrap();
    let md = pe.metadata().unwrap();
    let row = md
        .tables()
        .iter::<cildec::tables::AssemblyRefRow>()
        .find(|(_, r)| md.strings().str(r.as_ref().unwrap().name).unwrap() == "ExternalLibrary")
        .unwrap()
        .1
        .unwrap();
    let name = md.strings().str(row.name).unwrap().as_bytes();
    let offset = name.as_ptr() as usize - MAIN.as_ptr() as usize;
    let mut bytes = MAIN.to_vec();
    bytes[offset..offset + name.len()].fill(0);
    bytes[offset..offset + 10].copy_from_slice(b"../outside");
    let root =
        std::env::temp_dir().join(format!("ilspy-reference-traversal-{}", std::process::id()));
    let directory = root.join("references");
    std::fs::create_dir_all(&directory).unwrap();
    let input = root.join("input.bin");
    std::fs::write(&input, bytes).unwrap();
    std::fs::write(root.join("outside.dll"), DEPENDENCY).unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .arg("decompile")
        .arg(input)
        .args(["--type", "ExternalCases.Cells", "--reference-dir"])
        .arg(directory)
        .args(["--no-auto-references", "--details"])
        .output()
        .unwrap();
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("unsafe assembly filename"), "{text}");
    assert!(!text.contains("outside.dll"), "{text}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unreadable_reference_identity_does_not_hide_independently_decompilable_bodies() {
    let bytes = identity_key(MAIN, true, &[], true);
    let isolated = ilspy_lite::decompile_type(&bytes, "ExternalCases.Cells").unwrap();
    let path = std::env::temp_dir().join(format!(
        "ilspy-invalid-reference-{}.bin",
        std::process::id()
    ));
    std::fs::write(&path, &bytes).unwrap();
    let dependency = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/external-library-release.bin"
    );
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .arg("decompile")
        .arg(&path)
        .args([
            "--type",
            "ExternalCases.Cells",
            "--reference",
            dependency,
            "--no-auto-references",
            "--details",
        ])
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(
        result.status.code(),
        Some(i32::from(isolated.summary.exit_code())),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.starts_with(&isolated.text), "{text}");
    assert!(
        text.contains("status=invalid") && text.contains("raw-row="),
        "{text}"
    );
}

#[test]
fn cli_finds_neighboring_dependencies_by_default_and_can_disable_discovery() {
    let root =
        std::env::temp_dir().join(format!("ilspy-neighbor-reference-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let main = root.join("input.dll");
    std::fs::write(&main, MAIN).unwrap();
    std::fs::write(root.join("ExternalLibrary.dll"), DEPENDENCY).unwrap();
    for automatic in [true, false] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"));
        command
            .arg("decompile")
            .arg(&main)
            .args(["--type", "ExternalCases.Cells", "--details"]);
        if !automatic {
            command.arg("--no-auto-references");
        }
        let result = command.output().unwrap();
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let text = String::from_utf8(result.stdout).unwrap();
        if automatic {
            let resolution = text
                .lines()
                .find(|l| l.contains("status=resolved") && l.contains("ExternalLibrary"))
                .expect("neighboring exact dependency must resolve without a manual path");
            assert!(resolution.contains("ExternalLibrary.dll"));
        } else {
            assert_eq!(
                text,
                ilspy_lite::decompile_type(MAIN, "ExternalCases.Cells")
                    .unwrap()
                    .text
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_bytes_resolve_an_assembly_reference_without_searching_the_filesystem() {
    let mut context = DependencyContext::new(MAIN.to_vec()).unwrap();
    let reference = context
        .references()
        .iter()
        .find(|r| r.identity.name == "ExternalLibrary")
        .unwrap()
        .clone();
    assert!(matches!(
        context.resolve(reference.token).unwrap(),
        AssemblyResolution::Missing { .. }
    ));
    // This source is provenance, not an instruction to open a file.
    context
        .add_reference("not-on-disk/ExternalLibrary.dll", DEPENDENCY.to_vec())
        .unwrap();
    let resolved = context.resolve(reference.token).unwrap();
    match resolved {
        AssemblyResolution::Resolved { identity, source } => {
            assert_eq!(identity.name, "ExternalLibrary");
            assert_eq!(identity.version, [0, 0, 0, 0]);
            assert!(identity.culture.is_empty());
            assert!(identity.public_key_token.is_empty());
            assert_eq!(source, "not-on-disk/ExternalLibrary.dll");
        }
        other => panic!("Explicit matching dependency was not resolved: {other:?}"),
    }
}
