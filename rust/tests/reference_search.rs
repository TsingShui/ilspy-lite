// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::discovery_paths::DiscoveryEnvironment;
use ilspy_lite::{AssemblyResolution, DependencyContext, ReferenceSearchOptions};

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
