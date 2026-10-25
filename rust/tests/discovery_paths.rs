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
