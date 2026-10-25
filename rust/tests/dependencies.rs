// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{AssemblyResolution, DependencyContext, DependencyLimits};

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
        assert!(DependencyContext::new(identity_key(MAIN, true, &key, full)).is_err());
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
