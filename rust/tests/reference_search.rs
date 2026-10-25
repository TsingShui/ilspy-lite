// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::discovery_paths::DiscoveryEnvironment;
use ilspy_lite::{
    AssemblyResolution, DependencyContext, ReferenceSearchOptions, ReferenceVersionPolicy,
};

#[test]
fn exact_identity_beats_version_alternatives_and_substitution_requires_explicit_policy() {
    let main = include_bytes!("fixtures/external-release.bin");
    let dependency = include_bytes!("fixtures/external-library-release.bin");
    let md = cildec::PeImage::parse(dependency)
        .unwrap()
        .metadata()
        .unwrap();
    let row = md.tables().row_bytes(cildec::TableId::Assembly, 1).unwrap();
    let offset = row.as_ptr() as usize - dependency.as_ptr() as usize;
    let mut newer = dependency.to_vec();
    newer[offset + 10..offset + 12].copy_from_slice(&1u16.to_le_bytes());
    let root = std::env::temp_dir().join(format!("ilspy-version-search-{}", std::process::id()));
    let shared = root.join("dotnet/shared/Microsoft.NETCore.App/10.0.12");
    std::fs::create_dir_all(&shared).unwrap();
    let input = root.join("owned.dll");
    let explicit = root.join("newer.dll");
    std::fs::write(&input, main).unwrap();
    std::fs::write(&explicit, newer).unwrap();
    std::fs::write(shared.join("ExternalLibrary.dll"), dependency).unwrap();
    for (policy, exact_available) in [
        (ReferenceVersionPolicy::Exact, false),
        (ReferenceVersionPolicy::SameMajor, false),
        (ReferenceVersionPolicy::SameMajor, true),
    ] {
        let mut context = DependencyContext::new(main.to_vec()).unwrap();
        let requested = context
            .references()
            .iter()
            .find(|r| r.identity.name == "ExternalLibrary")
            .unwrap()
            .token;
        context
            .discover_references(
                &input,
                &ReferenceSearchOptions {
                    automatic: exact_available,
                    files: vec![explicit.clone()],
                    version_policy: policy,
                    environment: DiscoveryEnvironment {
                        dotnet_roots: vec![root.join("dotnet")],
                        ..DiscoveryEnvironment::default()
                    },
                    ..ReferenceSearchOptions::default()
                },
            )
            .unwrap();
        match context.resolve(requested).unwrap() {
            AssemblyResolution::Missing { .. } => assert_eq!(policy, ReferenceVersionPolicy::Exact),
            AssemblyResolution::Substituted {
                requested,
                selected,
                source,
                ..
            } => {
                assert!(!exact_available);
                assert_eq!(requested.version, [0; 4]);
                assert_eq!(selected.version, [0, 0, 0, 1]);
                assert!(source.ends_with("newer.dll"));
                let md = cildec::PeImage::parse(main).unwrap().metadata().unwrap();
                let base = md
                    .tables()
                    .iter::<cildec::tables::TypeRefRow>()
                    .find(|(_, row)| {
                        md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Base"
                    })
                    .unwrap()
                    .0;
                let bound = context.resolve_type(0x01000000 | base).unwrap();
                assert!(
                    !bound.exact_identity,
                    "replacement metadata is not proof of the original contract"
                );
                let output = context
                    .decompile_type("ExternalCases.Cells", ilspy_lite::OutputStyle::Readable)
                    .unwrap()
                    .text;
                assert!(
                    output.contains("Dependency version substituted"),
                    "{output}"
                );
            }
            AssemblyResolution::Resolved { identity, source } => {
                assert!(exact_available);
                assert_eq!(identity.version, [0; 4]);
                assert!(source.ends_with("ExternalLibrary.dll"));
            }
            other => panic!("unexpected policy result {other:?}"),
        }
    }
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
        .arg("decompile")
        .arg(&input)
        .args(["--type", "ExternalCases.Cells", "--reference"])
        .arg(&explicit)
        .args([
            "--no-auto-references",
            "--reference-version-policy",
            "same-major",
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
        text.contains("status=substituted") && text.contains("policy=SameMajor"),
        "{text}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn an_unreadable_named_candidate_cannot_authorize_version_substitution() {
    use std::os::unix::fs::PermissionsExt;
    let main = include_bytes!("fixtures/external-release.bin");
    let dependency = include_bytes!("fixtures/external-library-release.bin");
    let md = cildec::PeImage::parse(dependency)
        .unwrap()
        .metadata()
        .unwrap();
    let row = md.tables().row_bytes(cildec::TableId::Assembly, 1).unwrap();
    let at = row.as_ptr() as usize - dependency.as_ptr() as usize;
    let mut newer = dependency.to_vec();
    newer[at + 10..at + 12].copy_from_slice(&1u16.to_le_bytes());
    let root =
        std::env::temp_dir().join(format!("ilspy-unreadable-reference-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let exact = root.join("ExternalLibrary.dll");
    let explicit = root.join("newer.bin");
    std::fs::write(&exact, dependency).unwrap();
    std::fs::write(&explicit, newer).unwrap();
    std::fs::set_permissions(&exact, std::fs::Permissions::from_mode(0o0)).unwrap();
    let denied = std::fs::File::open(&exact).is_err();
    let mut context = DependencyContext::new(main.to_vec()).unwrap();
    let reference = context
        .references()
        .iter()
        .find(|r| r.identity.name == "ExternalLibrary")
        .unwrap()
        .token;
    let search = context.discover_references(
        &root.join("input.dll"),
        &ReferenceSearchOptions {
            automatic: false,
            files: vec![explicit],
            directories: vec![root.clone()],
            version_policy: ReferenceVersionPolicy::SameMajor,
            ..ReferenceSearchOptions::default()
        },
    );
    std::fs::set_permissions(&exact, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert!(
        denied,
        "owned permission fixture must actually deny reading on this host"
    );
    search.unwrap();
    assert!(
        context
            .search_diagnostics()
            .iter()
            .any(|s| s.contains("Dependency input rejected"))
    );
    assert!(
        matches!(
            context.resolve(reference).unwrap(),
            AssemblyResolution::Missing { .. }
        ),
        "incomplete exact search cannot prove substitution is warranted"
    );
}

#[test]
fn missing_candidate_attempts_consume_the_configured_probe_budget() {
    let root = std::env::temp_dir().join(format!("ilspy-probe-budget-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let mut context =
        DependencyContext::new(include_bytes!("fixtures/external-release.bin").to_vec()).unwrap();
    context
        .discover_references(
            &root.join("input.dll"),
            &ReferenceSearchOptions {
                automatic: false,
                directories: vec![root.clone(); 5000],
                max_probes: 3,
                ..ReferenceSearchOptions::default()
            },
        )
        .unwrap();
    assert!(
        context
            .search_diagnostics()
            .iter()
            .any(|s| s.contains("probe limit")),
        "missing files still require bounded inspection"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn discovery_completeness_is_structured_not_inferred_from_warning_words() {
    use ilspy_lite::discovery_paths::candidates_for;
    let root = std::env::temp_dir().join(format!(
        "ilspy-discovery-completeness-{}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let input = root.join("owned.dll");
    let context =
        DependencyContext::new(include_bytes!("fixtures/external-release.bin").to_vec()).unwrap();
    let identity = &context
        .references()
        .iter()
        .find(|r| r.identity.name == "ExternalLibrary")
        .unwrap()
        .identity;
    let absent_optional = candidates_for(&input, identity, None, &DiscoveryEnvironment::default());
    assert!(!absent_optional.incomplete);
    std::fs::write(input.with_extension("deps.json"), b"not JSON").unwrap();
    let malformed = candidates_for(&input, identity, None, &DiscoveryEnvironment::default());
    assert!(malformed.incomplete);
    assert!(
        malformed
            .diagnostics
            .iter()
            .any(|d| d.contains("malformed deps"))
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn target_framework_comes_from_static_attribute_evidence_not_the_host_runtime() {
    let context =
        DependencyContext::new(include_bytes!("fixtures/binding-release.bin").to_vec()).unwrap();
    assert_eq!(
        context.target_framework_hint().unwrap().as_deref(),
        Some(".NETCoreApp,Version=v10.0")
    );
    let context =
        DependencyContext::new(include_bytes!("fixtures/external-release.bin").to_vec()).unwrap();
    assert_eq!(context.target_framework_hint().unwrap(), None);
}

#[test]
fn reference_pack_origin_never_claims_a_runtime_implementation() {
    let root = std::env::temp_dir().join(format!(
        "ilspy-reference-contract-search-{}",
        std::process::id()
    ));
    let contract = root.join("dotnet/packs/Microsoft.NETCore.App.Ref/10.0.12/ref/net10.0");
    std::fs::create_dir_all(&contract).unwrap();
    let main = include_bytes!("fixtures/external-release.bin");
    std::fs::write(
        contract.join("ExternalLibrary.dll"),
        include_bytes!("fixtures/external-library-release.bin"),
    )
    .unwrap();
    let mut context = DependencyContext::new(main.to_vec()).unwrap();
    context
        .discover_references(
            &root.join("input.dll"),
            &ReferenceSearchOptions {
                environment: DiscoveryEnvironment {
                    dotnet_roots: vec![root.join("dotnet")],
                    ..DiscoveryEnvironment::default()
                },
                ..ReferenceSearchOptions::default()
            },
        )
        .unwrap();
    let output = context
        .decompile_type("ExternalCases.Cells", ilspy_lite::OutputStyle::Details)
        .unwrap()
        .text;
    assert!(output.contains("kind=ReferenceContract"), "{output}");
    assert!(output.contains("origin=ReferencePack"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn forwarder_dependencies_are_discovered_from_their_own_reference_scope() {
    let main = include_bytes!("fixtures/binding-release.bin");
    let md = cildec::PeImage::parse(main).unwrap().metadata().unwrap();
    let reference = md
        .tables()
        .iter::<cildec::tables::AssemblyRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "BindingLibrary")
        .unwrap()
        .0;
    let spare = md
        .tables()
        .iter::<cildec::tables::MethodDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "get_VirtualFlag")
        .unwrap()
        .1
        .unwrap()
        .name;
    let name = md.strings().str(spare).unwrap().as_bytes();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::AssemblyRef, reference)
        .unwrap();
    assert_eq!(row.len(), 20);
    let offset = row.as_ptr() as usize - main.as_ptr() as usize;
    let text = name.as_ptr() as usize - main.as_ptr() as usize;
    let mut input = main.to_vec();
    input[text..text + name.len()].fill(0);
    input[text..text + 13].copy_from_slice(b"BindingFacade");
    input[offset + 14..offset + 16].copy_from_slice(&(spare.0 as u16).to_le_bytes());
    let root = std::env::temp_dir().join(format!("ilspy-forwarder-search-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("input.dll");
    std::fs::write(&path, &input).unwrap();
    std::fs::write(
        root.join("BindingFacade.dll"),
        include_bytes!("fixtures/binding-facade-release.bin"),
    )
    .unwrap();
    std::fs::write(
        root.join("BindingLibrary.dll"),
        include_bytes!("fixtures/binding-library-release.bin"),
    )
    .unwrap();
    let peer = md
        .tables()
        .iter::<cildec::tables::TypeRefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Peer")
        .unwrap()
        .0;
    let mut context = DependencyContext::new(input).unwrap();
    context
        .discover_references(&path, &ReferenceSearchOptions::default())
        .unwrap();
    let resolved = context.resolve_type(0x01000000 | peer).unwrap();
    assert_eq!(resolved.definition.assembly.name, "BindingLibrary");
    assert_eq!(resolved.forwarders.len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn installed_layout_candidates_are_identity_checked_without_requiring_a_dotnet_process() {
    let root = std::env::temp_dir().join(format!("ilspy-reference-search-{}", std::process::id()));
    let app = root.join("app");
    let installed = root.join("dotnet");
    std::fs::create_dir_all(&app).unwrap();
    let input = app.join("owned.dll");
    let main = include_bytes!("fixtures/external-release.bin");
    let dependency = include_bytes!("fixtures/external-library-release.bin");
    std::fs::write(&input, main).unwrap();
    let target = installed.join("shared/Microsoft.NETCore.App/10.0.12");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("ExternalLibrary.dll"), dependency).unwrap();
    let mut context = DependencyContext::new(main.to_vec()).unwrap();
    let requested = context
        .references()
        .iter()
        .find(|r| r.identity.name == "ExternalLibrary")
        .unwrap()
        .token;
    context
        .discover_references(
            &input,
            &ReferenceSearchOptions {
                environment: DiscoveryEnvironment {
                    dotnet_roots: vec![installed],
                    ..DiscoveryEnvironment::default()
                },
                ..ReferenceSearchOptions::default()
            },
        )
        .unwrap();
    assert!(matches!(
        context.resolve(requested).unwrap(),
        AssemblyResolution::Resolved { .. }
    ));
    let text = context
        .decompile_type("ExternalCases.Cells", ilspy_lite::OutputStyle::Details)
        .unwrap()
        .text;
    assert!(text.contains("SharedFramework"), "{text}");
    assert!(!root.join("dotnet/dotnet").exists());
    let mut missing = DependencyContext::new(main.to_vec()).unwrap();
    missing
        .discover_references(&input, &ReferenceSearchOptions::default())
        .unwrap();
    assert!(matches!(
        missing.resolve(requested).unwrap(),
        AssemblyResolution::Missing { .. }
    ));
    std::fs::remove_dir_all(root).unwrap();
}
