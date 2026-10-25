// Copyright (c) 2018 Siegfried Pammer
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this
// software and associated documentation files (the "Software"), to deal in the Software
// without restriction, including without limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons
// to whom the Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
// INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
// PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! Filesystem candidates only; callers must validate full metadata identities.
//! Layout rules derive from DotNetCorePathFinder and UniversalAssemblyResolver.

use crate::AssemblyIdentity;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct DiscoveryEnvironment {
    pub dotnet_roots: Vec<PathBuf>,
    pub nuget_roots: Vec<PathBuf>,
    pub windows_dir: Option<PathBuf>,
    /// Each root is a lib/mono directory containing version directories and gac.
    pub mono_roots: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateOrigin {
    Neighbor,
    NuGet,
    SharedFramework,
    /// Metadata contract provenance, not implementation evidence.
    ReferencePack,
    Framework,
    Gac,
    Mono,
}

#[derive(Clone, Debug)]
pub struct CandidatePath {
    pub path: PathBuf,
    pub origin: CandidateOrigin,
}

#[derive(Clone, Debug, Default)]
pub struct DiscoveryResult {
    pub candidates: Vec<CandidatePath>,
    pub diagnostics: Vec<String>,
}

const MAX_ROOTS: usize = 64;
const MAX_DEPS_BYTES: u64 = 1024 * 1024;
const MAX_PROBES: usize = 16384;
const MAX_CANDIDATES: usize = 1024;
const MAX_DIAGNOSTICS: usize = 128;

#[derive(Default)]
struct Search {
    result: DiscoveryResult,
    seen: HashSet<(PathBuf, bool)>,
    probes: usize,
}

impl Search {
    fn diagnostic(&mut self, message: impl Into<String>) {
        if self.result.diagnostics.len() < MAX_DIAGNOSTICS {
            self.result.diagnostics.push(message.into());
        }
    }

    fn probe(&mut self, path: PathBuf, origin: CandidateOrigin) {
        if self.probes >= MAX_PROBES {
            return;
        }
        self.probes += 1;
        if self.probes == MAX_PROBES {
            self.diagnostic("candidate probe limit reached");
        }
        if !path.is_file() {
            return;
        }
        match path.canonicalize() {
            Ok(canonical) => {
                // Preserve contract provenance even when a ref pack aliases an implementation.
                if !self
                    .seen
                    .insert((canonical, origin == CandidateOrigin::ReferencePack))
                {
                    return;
                }
                if self.result.candidates.len() == MAX_CANDIDATES {
                    self.diagnostic("candidate count limit reached");
                    return;
                }
                self.result.candidates.push(CandidatePath { path, origin });
            }
            Err(error) => {
                self.diagnostic(format!("cannot canonicalize {}: {error}", path.display()))
            }
        }
    }

    fn pair(&mut self, directory: &Path, name: &str, origin: CandidateOrigin) {
        for extension in ["dll", "exe"] {
            self.probe(directory.join(format!("{name}.{extension}")), origin);
        }
    }
}

fn safe_component(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 240
        || value == "."
        || value == ".."
        || value.ends_with(['.', ' '])
        || value
            .chars()
            .any(|c| c.is_control() || "<>:\\/|?*\"".contains(c))
    {
        return false;
    }
    let base = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

/// Enumerate local candidates without opening assembly contents or authorizing replacement.
pub fn candidates_for(
    input_path: &Path,
    requested: &AssemblyIdentity,
    target_framework: Option<&str>,
    environment: &DiscoveryEnvironment,
) -> DiscoveryResult {
    let mut search = Search::default();
    if !safe_component(&requested.name)
        || (!requested.culture.is_empty() && !safe_component(&requested.culture))
    {
        search.diagnostic("unsafe assembly name or culture component");
        return search.result;
    }
    search.pair(
        input_path.parent().unwrap_or(Path::new(".")),
        &requested.name,
        CandidateOrigin::Neighbor,
    );
    search.deps(input_path, requested, target_framework, environment);
    search.result
}

fn relative_asset(value: &str) -> Option<PathBuf> {
    let components: Vec<_> = value.split('/').collect();
    if components.len() > 32 || !components.iter().all(|part| safe_component(part)) {
        return None;
    }
    Some(components.iter().collect())
}

impl Search {
    fn deps(
        &mut self,
        input: &Path,
        requested: &AssemblyIdentity,
        target: Option<&str>,
        env: &DiscoveryEnvironment,
    ) {
        let path = input.with_extension("deps.json");
        let bytes = match bounded_read(&path, MAX_DEPS_BYTES) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.diagnostic(format!("deps {}: {error}", path.display()));
                return;
            }
        };
        let json: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(error) => {
                self.diagnostic(format!("malformed deps {}: {error}", path.display()));
                return;
            }
        };
        let Some(targets) = json.get("targets").and_then(|v| v.as_object()) else {
            self.diagnostic("deps missing targets object");
            return;
        };
        let selected = target.filter(|t| targets.contains_key(*t)).or_else(|| {
            let explicit = json.get("runtimeTarget")?.get("name")?.as_str()?;
            if target.is_none_or(|t| {
                explicit == t || explicit.strip_prefix(t).is_some_and(|s| s.starts_with('/'))
            }) {
                Some(explicit)
            } else {
                None
            }
        });
        let Some(selected) = selected
            .and_then(|key| targets.get(key))
            .and_then(|v| v.as_object())
        else {
            self.diagnostic("deps has no exact target or matching explicit runtimeTarget");
            return;
        };
        let Some(libraries) = json.get("libraries").and_then(|v| v.as_object()) else {
            self.diagnostic("deps missing libraries object");
            return;
        };
        if env.nuget_roots.len() > MAX_ROOTS {
            self.diagnostic("NuGet root limit reached");
        }
        for root in env.nuget_roots.iter().take(MAX_ROOTS) {
            for (name, entry) in selected {
                let Some(library) = libraries.get(name) else {
                    continue;
                };
                if library.get("type").and_then(|v| v.as_str()) != Some("package") {
                    continue;
                }
                let package = library.get("path").and_then(|v| v.as_str()).unwrap_or(name);
                let Some(package) = relative_asset(package) else {
                    self.diagnostic("unsafe deps package path");
                    continue;
                };
                if entry.get("runtimeTargets").is_some() {
                    self.diagnostic(
                        "deps runtimeTargets assets are not selected without RID policy",
                    );
                }
                let Some(runtime) = entry.get("runtime").and_then(|v| v.as_object()) else {
                    continue;
                };
                for extension in ["dll", "exe"] {
                    let filename = format!("{}.{}", requested.name, extension);
                    for asset in runtime.keys() {
                        let Some(asset) = relative_asset(asset) else {
                            self.diagnostic("unsafe deps runtime asset path");
                            continue;
                        };
                        if !asset
                            .file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.eq_ignore_ascii_case(&filename))
                        {
                            continue;
                        }
                        let candidate = root.join(&package).join(asset);
                        match (root.canonicalize(), candidate.canonicalize()) {
                            (Ok(root), Ok(path)) if path.starts_with(&root) => {
                                self.probe(candidate, CandidateOrigin::NuGet)
                            }
                            (Ok(_), Ok(_)) => {
                                self.diagnostic("deps asset symlink escapes NuGet root")
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(format!("not a regular file or exceeds {limit} byte limit"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("exceeds {limit} byte limit"));
    }
    Ok(bytes)
}
