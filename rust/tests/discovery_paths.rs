// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::AssemblyIdentity;
#[path = "../src/discovery_paths.rs"]
mod discovery_paths;

use discovery_paths::{CandidateOrigin, DiscoveryEnvironment, candidates_for};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ilspy-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn file(&self, path: &str) -> PathBuf {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"inert path enumeration sentinel").unwrap();
        path
    }
    fn input(&self) -> PathBuf {
        self.0.join("App.dll")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn identity() -> AssemblyIdentity {
    AssemblyIdentity {
        name: "Widget".into(),
        version: [4, 0, 0, 0],
        culture: String::new(),
        public_key_token: vec![0x12; 8],
        content_type: 0,
    }
}
fn paths(result: &discovery_paths::DiscoveryResult) -> Vec<&Path> {
    result.candidates.iter().map(|c| c.path.as_path()).collect()
}

#[test]
fn isolated_missing_sdk_and_neighbor_dll_before_exe() {
    let f = Fixture::new();
    let environment = DiscoveryEnvironment::default();
    assert!(environment.dotnet_roots.is_empty());
    assert!(environment.nuget_roots.is_empty());
    assert!(environment.mono_roots.is_empty());
    assert!(environment.windows_dir.is_none());
    assert!(
        candidates_for(&f.input(), &identity(), None, &environment)
            .candidates
            .is_empty()
    );
    let exe = f.file("Widget.exe");
    let dll = f.file("Widget.dll");
    let result = candidates_for(&f.input(), &identity(), None, &environment);
    assert_eq!(paths(&result), vec![dll.as_path(), exe.as_path()]);
    assert!(
        result
            .candidates
            .iter()
            .all(|c| c.origin == CandidateOrigin::Neighbor)
    );
}

#[test]
fn unsafe_names_are_rejected_before_any_probe() {
    let f = Fixture::new();
    f.file("Widget.dll");
    for name in [
        "",
        ".",
        "..",
        "../Widget",
        "a/b",
        "a\\b",
        "C:Widget",
        "a\0b",
        "a\nb",
        "Widget.",
        "Widget ",
        "CON",
        "NUL.txt",
        "a*b",
        "a?b",
    ] {
        let mut requested = identity();
        requested.name = name.into();
        let result = candidates_for(
            &f.input(),
            &requested,
            None,
            &DiscoveryEnvironment::default(),
        );
        assert!(result.candidates.is_empty(), "{name:?}");
        assert!(
            result.diagnostics.iter().any(|s| s.contains("unsafe")),
            "{name:?}"
        );
    }
}

fn deps(f: &Fixture, text: &str) {
    std::fs::write(f.0.join("App.deps.json"), text).unwrap();
}

#[test]
fn deps_exact_target_and_cache_paths_preserve_case_and_origin() {
    let f = Fixture::new();
    let dll = f.file("MixedCache/Package.Name/1.2.3/Lib/Net8/Widget.dll");
    let exe = f.file("MixedCache/Package.Name/1.2.3/Lib/Net8/Widget.exe");
    f.file("MixedCache/Wrong/1.0/Widget.dll");
    deps(
        &f,
        r#"{"targets":{
      ".NETCoreApp,Version=v8.0":{"Package.Name/1.2.3":{"runtime":{"Lib/Net8/Widget.exe":{},"Lib/Net8/Widget.dll":{}}}},
      "wrong":{"Wrong/1.0":{"runtime":{"Widget.dll":{}}}}
    },"libraries":{"Package.Name/1.2.3":{"type":"package","path":"Package.Name/1.2.3"},"Wrong/1.0":{"type":"package"}}}"#,
    );
    let env = DiscoveryEnvironment {
        nuget_roots: vec![f.0.join("MissingCache"), f.0.join("MixedCache")],
        ..Default::default()
    };
    let result = candidates_for(
        &f.input(),
        &identity(),
        Some(".NETCoreApp,Version=v8.0"),
        &env,
    );
    assert_eq!(paths(&result), vec![dll.as_path(), exe.as_path()]);
    assert!(
        result
            .candidates
            .iter()
            .all(|c| c.origin == CandidateOrigin::NuGet)
    );
}

#[test]
fn deps_runtime_target_is_explicit_evidence_not_an_invented_rid() {
    let f = Fixture::new();
    let dll = f.file("Cache/Pkg/1.0/runtimes/test/Widget.dll");
    let env = DiscoveryEnvironment {
        nuget_roots: vec![f.0.join("Cache")],
        ..Default::default()
    };
    let body = r#""targets":{".NETCoreApp,Version=v8.0/test-rid":{"Pkg/1.0":{"runtime":{"runtimes/test/Widget.dll":{}}}}},"libraries":{"Pkg/1.0":{"type":"package"}}"#;
    deps(&f, &format!("{{{body}}}"));
    let missing = candidates_for(
        &f.input(),
        &identity(),
        Some(".NETCoreApp,Version=v8.0"),
        &env,
    );
    assert!(missing.candidates.is_empty());
    assert!(missing.diagnostics.iter().any(|s| s.contains("target")));
    deps(
        &f,
        &format!("{{\"runtimeTarget\":{{\"name\":\".NETCoreApp,Version=v8.0/test-rid\"}},{body}}}"),
    );
    assert_eq!(
        paths(&candidates_for(
            &f.input(),
            &identity(),
            Some(".NETCoreApp,Version=v8.0"),
            &env
        )),
        vec![dll.as_path()]
    );
    assert_eq!(
        paths(&candidates_for(&f.input(), &identity(), None, &env)),
        vec![dll.as_path()]
    );
    assert!(
        candidates_for(
            &f.input(),
            &identity(),
            Some(".NETCoreApp,Version=v9.0"),
            &env
        )
        .candidates
        .is_empty()
    );
}

#[test]
fn missing_malformed_oversized_and_traversing_deps_do_not_hide_neighbors() {
    let f = Fixture::new();
    let dll = f.file("Widget.dll");
    let env = DiscoveryEnvironment {
        nuget_roots: vec![f.0.join("Cache")],
        ..Default::default()
    };
    for text in [None, Some("{".to_string()), Some(" ".repeat(1024 * 1024 + 1)), Some(r#"{"targets":{"T":{"P/1":{"runtime":{"../../Widget.dll":{}}}}},"libraries":{"P/1":{"type":"package","path":"../Escape"}}}"#.to_string())] {
        if let Some(text) = text { deps(&f, &text); }
        let result = candidates_for(&f.input(), &identity(), Some("T"), &env);
        assert_eq!(paths(&result), vec![dll.as_path()]);
        assert!(!result.diagnostics.is_empty());
    }
}

#[test]
fn shared_versions_use_target_hint_then_stable_alternatives_without_sdk() {
    let f = Fixture::new();
    let mut expected = Vec::new();
    for version in [
        "8.0.0",
        "8.0.0-preview.2",
        "8.0.0-preview.10",
        "8.0.1",
        "6.0.0",
        "10.0.0",
    ] {
        expected.push(f.file(&format!(
            "dotnet/shared/Microsoft.NETCore.App/{version}/Widget.dll"
        )));
    }
    f.file("dotnet/shared/Microsoft.NETCore.App/8.0.2/Other.dll");
    f.file("dotnet/shared/Microsoft.NETCore.App/not-a-version/Widget.dll");
    let env = DiscoveryEnvironment {
        dotnet_roots: vec![f.0.join("missing"), f.0.join("dotnet")],
        ..Default::default()
    };
    let result = candidates_for(
        &f.input(),
        &identity(),
        Some(".NETCoreApp,Version=v8.0"),
        &env,
    );
    assert_eq!(
        paths(&result),
        expected.iter().map(|p| p.as_path()).collect::<Vec<_>>()
    );
    assert!(
        result
            .candidates
            .iter()
            .all(|c| c.origin == CandidateOrigin::SharedFramework)
    );
    let no_hint = candidates_for(&f.input(), &identity(), None, &env);
    assert_eq!(no_hint.candidates[0].path, expected[4]);
}

#[test]
fn reference_pack_provenance_is_separate_from_shared_implementation() {
    let f = Fixture::new();
    let shared = f.file("dotnet/shared/Microsoft.NETCore.App/8.0.0/Widget.dll");
    let contract = f.file("dotnet/packs/Microsoft.NETCore.App.Ref/8.0.1/ref/net8.0/Widget.dll");
    let alternate = f.file("dotnet/packs/Microsoft.NETCore.App.Ref/9.0.0/ref/net8.0/Widget.dll");
    f.file("dotnet/packs/Microsoft.NETCore.App.Ref/8.0.1/ref/net9.0/Widget.dll");
    let env = DiscoveryEnvironment {
        dotnet_roots: vec![f.0.join("dotnet")],
        ..Default::default()
    };
    let result = candidates_for(
        &f.input(),
        &identity(),
        Some(".NETCoreApp,Version=v8.0"),
        &env,
    );
    assert_eq!(
        paths(&result),
        vec![shared.as_path(), contract.as_path(), alternate.as_path()]
    );
    assert_eq!(result.candidates[1].origin, CandidateOrigin::ReferencePack);
    std::fs::remove_file(shared).unwrap();
    assert!(
        candidates_for(
            &f.input(),
            &identity(),
            Some(".NETCoreApp,Version=v8.0"),
            &env
        )
        .candidates
        .iter()
        .all(|c| c.origin == CandidateOrigin::ReferencePack)
    );
    let standard =
        f.file("dotnet/packs/NETStandard.Library.Ref/2.1.0/ref/netstandard2.1/Widget.dll");
    assert_eq!(
        paths(&candidates_for(
            &f.input(),
            &identity(),
            Some(".NETStandard,Version=v2.1"),
            &env
        )),
        vec![standard.as_path()]
    );
}

#[test]
fn gac_exact_paths_across_roots_precede_same_major_alternatives_and_keep_culture_token() {
    let f = Fixture::new();
    let exact = f.file("Windows/Microsoft.NET/assembly/GAC_64/Widget/v4.0_4.0.0.0_fr-FR_1212121212121212/Widget.dll");
    let low = f.file("Windows/assembly/GAC_MSIL/Widget/4.0.0.1_fr-FR_1212121212121212/Widget.dll");
    let high = f.file("Windows/assembly/GAC_MSIL/Widget/4.2.0.0_fr-FR_1212121212121212/Widget.dll");
    for dir in [
        "4.3.0.0__1212121212121212",
        "4.3.0.0_fr-FR_1313131313131313",
        "5.0.0.0_fr-FR_1212121212121212",
    ] {
        f.file(&format!(
            "Windows/assembly/GAC_MSIL/Widget/{dir}/Widget.dll"
        ));
    }
    let env = DiscoveryEnvironment {
        windows_dir: Some(f.0.join("Windows")),
        ..Default::default()
    };
    let mut requested = identity();
    requested.culture = "fr-FR".into();
    let result = candidates_for(&f.input(), &requested, None, &env);
    assert_eq!(
        paths(&result),
        vec![exact.as_path(), low.as_path(), high.as_path()]
    );
    assert!(
        result
            .candidates
            .iter()
            .all(|c| c.origin == CandidateOrigin::Gac)
    );
    requested.public_key_token.clear();
    assert!(
        candidates_for(&f.input(), &requested, None, &env)
            .candidates
            .is_empty()
    );
}

#[test]
fn framework_and_mono_roots_need_no_host_runtime() {
    let f = Fixture::new();
    let framework = f.file("Windows/Microsoft.NET/Framework/v4.0.30319/Widget.dll");
    let framework64 = f.file("Windows/Microsoft.NET/Framework64/v4.0.30319/Widget.exe");
    let mono_gac = f.file("lib/mono/gac/Widget/4.0.0.0__1212121212121212/Widget.dll");
    let mono = f.file("lib/mono/4.5/Widget.dll");
    let facade = f.file("lib/mono/4.5/Facades/Widget.dll");
    let env = DiscoveryEnvironment {
        windows_dir: Some(f.0.join("Windows")),
        mono_roots: vec![f.0.join("lib/mono")],
        ..Default::default()
    };
    let result = candidates_for(
        &f.input(),
        &identity(),
        Some(".NETFramework,Version=v4.8"),
        &env,
    );
    assert_eq!(
        paths(&result),
        vec![
            mono_gac.as_path(),
            framework.as_path(),
            framework64.as_path(),
            mono.as_path(),
            facade.as_path()
        ]
    );
    assert_eq!(result.candidates[1].origin, CandidateOrigin::Framework);
    assert_eq!(result.candidates[0].origin, CandidateOrigin::Mono);
}

#[test]
fn directory_and_candidate_limits_are_diagnostic_and_deterministic() {
    let f = Fixture::new();
    for i in 0..1025 {
        f.file(&format!(
            "dotnet/shared/Microsoft.NETCore.App/8.0.{i}/Widget.dll"
        ));
    }
    let neighbor = f.file("Widget.dll");
    let env = DiscoveryEnvironment {
        dotnet_roots: vec![f.0.join("dotnet")],
        ..Default::default()
    };
    let result = candidates_for(&f.input(), &identity(), None, &env);
    assert_eq!(paths(&result), vec![neighbor.as_path()]);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|s| s.contains("directory entry limit"))
    );
    std::fs::remove_dir_all(f.0.join("dotnet/shared/Microsoft.NETCore.App/8.0.1024")).unwrap();
    for i in 0..10 {
        f.file(&format!(
            "dotnet/shared/Microsoft.AspNetCore.App/8.0.{i}/Widget.dll"
        ));
    }
    let result = candidates_for(&f.input(), &identity(), None, &env);
    assert_eq!(result.candidates.len(), 1024);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|s| s.contains("candidate count limit"))
    );
    assert_eq!(
        paths(&result),
        paths(&candidates_for(&f.input(), &identity(), None, &env))
    );
}

#[cfg(unix)]
#[test]
fn symlink_duplicates_loops_and_cache_escapes_are_bounded() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let shared = f.file("dotnet/shared/Microsoft.NETCore.App/8.0.0/Widget.dll");
    symlink(
        "8.0.0",
        f.0.join("dotnet/shared/Microsoft.NETCore.App/8.0.1"),
    )
    .unwrap();
    symlink(
        "8.0.2",
        f.0.join("dotnet/shared/Microsoft.NETCore.App/8.0.2"),
    )
    .unwrap();
    symlink("dotnet", f.0.join("alias")).unwrap();
    std::fs::create_dir_all(f.0.join("Cache/P/1")).unwrap();
    symlink(&shared, f.0.join("Cache/P/1/Widget.dll")).unwrap();
    deps(
        &f,
        r#"{"targets":{"T":{"P/1":{"runtime":{"Widget.dll":{}}}}},"libraries":{"P/1":{"type":"package"}}}"#,
    );
    let env = DiscoveryEnvironment {
        dotnet_roots: vec![f.0.join("dotnet"), f.0.join("alias")],
        nuget_roots: vec![f.0.join("Cache")],
        ..Default::default()
    };
    let result = candidates_for(&f.input(), &identity(), Some("T"), &env);
    assert_eq!(paths(&result), vec![shared.as_path()]);
    assert!(result.diagnostics.iter().any(|s| s.contains("escapes")));
    assert!(result.diagnostics.iter().any(|s| s.contains("8.0.2")));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|s| s.contains("duplicate canonical candidate"))
    );
}

#[test]
fn host_environment_child() {
    // The parent test launches this test in a fresh process, avoiding global env mutation.
    if let Some(root) = std::env::var_os("ILSPY_DISCOVERY_HOST_FIXTURE") {
        let root = PathBuf::from(root);
        let environment = discovery_paths::environment_from_host();
        assert_eq!(
            environment.dotnet_roots[0],
            root.join("Arch").canonicalize().unwrap()
        );
        assert_eq!(
            environment.dotnet_roots[1],
            root.join("Generic").canonicalize().unwrap()
        );
        assert!(
            environment
                .dotnet_roots
                .contains(&root.join("FromPath").canonicalize().unwrap())
        );
        assert_eq!(
            environment.nuget_roots[0],
            root.join("Packages").canonicalize().unwrap()
        );
        assert_eq!(
            environment.nuget_roots[1],
            root.join("Home/.nuget/packages").canonicalize().unwrap()
        );
        assert_eq!(
            environment.windows_dir,
            Some(root.join("Windows").canonicalize().unwrap())
        );
        assert!(
            environment
                .mono_roots
                .contains(&root.join("Prefix/lib/mono").canonicalize().unwrap())
        );
    } else {
        assert!(
            discovery_paths::DiscoveryEnvironment::default()
                .dotnet_roots
                .is_empty()
        );
    }
}

#[cfg(unix)]
#[test]
fn host_environment_uses_env_and_path_symlinks_without_executing_dotnet() {
    let f = Fixture::new();
    for directory in [
        "Arch/shared",
        "Generic/packs",
        "Packages",
        "Home/.nuget/packages",
        "Windows",
        "Prefix/lib/mono",
        "bin",
    ] {
        std::fs::create_dir_all(f.0.join(directory)).unwrap();
    }
    let executable = f.file("FromPath/dotnet");
    std::os::unix::fs::symlink(&executable, f.0.join("bin/dotnet")).unwrap();
    let arch = match std::env::consts::ARCH {
        "x86_64" => "X64",
        "x86" => "X86",
        "aarch64" => "ARM64",
        "arm" => "ARM",
        other => other,
    };
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "host_environment_child", "--nocapture"])
        .env_clear()
        .env("ILSPY_DISCOVERY_HOST_FIXTURE", &f.0)
        .env(format!("DOTNET_ROOT_{arch}"), f.0.join("Arch"))
        .env("DOTNET_ROOT", f.0.join("Generic"))
        .env("PATH", f.0.join("bin"))
        .env("NUGET_PACKAGES", f.0.join("Packages"))
        .env("HOME", f.0.join("Home"))
        .env("WINDIR", f.0.join("Windows"))
        .env("MONO_GAC_PREFIX", f.0.join("Prefix"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mono_gac_exact_precedes_windows_gac_version_alternatives() {
    let f = Fixture::new();
    let alternative =
        f.file("Windows/assembly/GAC_MSIL/Widget/4.1.0.0__1212121212121212/Widget.dll");
    let exact = f.file("lib/mono/gac/Widget/4.0.0.0__1212121212121212/Widget.dll");
    let env = DiscoveryEnvironment {
        windows_dir: Some(f.0.join("Windows")),
        mono_roots: vec![f.0.join("lib/mono")],
        ..Default::default()
    };
    let result = candidates_for(&f.input(), &identity(), None, &env);
    assert_eq!(paths(&result), vec![exact.as_path(), alternative.as_path()]);
}

#[test]
fn deps_manifest_work_is_bounded_separately_from_file_size() {
    let f = Fixture::new();
    std::fs::create_dir_all(f.0.join("Cache")).unwrap();
    let runtime: serde_json::Map<_, _> = (0..4097)
        .map(|i| (format!("Asset{i}.dll"), serde_json::json!({})))
        .collect();
    deps(&f, &serde_json::json!({"targets":{"T":{"P/1":{"runtime":runtime}}},"libraries":{"P/1":{"type":"package"}}}).to_string());
    let env = DiscoveryEnvironment {
        nuget_roots: vec![f.0.join("Cache")],
        ..Default::default()
    };
    let result = candidates_for(&f.input(), &identity(), Some("T"), &env);
    assert!(result.candidates.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|s| s.contains("manifest item limit"))
    );
}

#[cfg(unix)]
#[test]
fn deps_symlink_loop_is_diagnosed_without_stopping_other_families() {
    let f = Fixture::new();
    std::fs::create_dir_all(f.0.join("Cache/P/1")).unwrap();
    std::os::unix::fs::symlink("Widget.dll", f.0.join("Cache/P/1/Widget.dll")).unwrap();
    deps(
        &f,
        r#"{"targets":{"T":{"P/1":{"runtime":{"Widget.dll":{}}}}},"libraries":{"P/1":{"type":"package"}}}"#,
    );
    let shared = f.file("dotnet/shared/Microsoft.NETCore.App/8.0.0/Widget.dll");
    let env = DiscoveryEnvironment {
        nuget_roots: vec![f.0.join("Cache")],
        dotnet_roots: vec![f.0.join("dotnet")],
        ..Default::default()
    };
    let result = candidates_for(&f.input(), &identity(), Some("T"), &env);
    assert_eq!(paths(&result), vec![shared.as_path()]);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|s| s.contains("cannot inspect deps asset"))
    );
}
