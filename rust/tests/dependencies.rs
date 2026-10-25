// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{AssemblyResolution, DependencyContext, DependencyLimits};

const MAIN: &[u8] = include_bytes!("fixtures/external-release.bin");
const DEPENDENCY: &[u8] = include_bytes!("fixtures/external-library-release.bin");

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
                max_bytes: usize::MAX
            }
        )
        .is_err()
    );
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
