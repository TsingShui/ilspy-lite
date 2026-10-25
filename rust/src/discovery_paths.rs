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

const MAX_DIRECTORY_READS: usize = 512;
const MAX_DIRECTORY_ENTRIES: usize = 1024;
const MAX_TOTAL_ENTRIES: usize = 32768;
const MAX_ROOTS: usize = 64;
const MAX_MANIFEST_ITEMS: usize = 4096;
const MAX_DEPS_BYTES: u64 = 1024 * 1024;
const MAX_PROBES: usize = 16384;
const MAX_CANDIDATES: usize = 1024;
const MAX_DIAGNOSTICS: usize = 128;

#[derive(Default)]
struct Search {
    result: DiscoveryResult,
    seen: HashSet<(PathBuf, bool)>,
    probes: usize,
    directories: usize,
    entries: usize,
}

impl Search {
    fn diagnostic(&mut self, message: impl Into<String>) {
        let message = message.into();
        if self.result.diagnostics.contains(&message) {
            return;
        }
        if self.result.diagnostics.len() < MAX_DIAGNOSTICS - 1 {
            self.result.diagnostics.push(message);
        } else if self.result.diagnostics.len() == MAX_DIAGNOSTICS - 1 {
            self.result
                .diagnostics
                .push("diagnostic limit reached".into());
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
        match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return,
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    self.diagnostic(format!("cannot inspect {}: {error}", path.display()));
                }
                return;
            }
        }
        match path.canonicalize() {
            Ok(canonical) => {
                // Preserve contract provenance even when a ref pack aliases an implementation.
                if !self
                    .seen
                    .insert((canonical, origin == CandidateOrigin::ReferencePack))
                {
                    self.diagnostic(format!(
                        "duplicate canonical candidate omitted: {}",
                        path.display()
                    ));
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
/// Roots retain injected order. Shared/ref versions matching an explicit major/minor
/// hint precede ascending alternatives; releases precede naturally ordered previews.
/// Scans are nonrecursive and bounded: 64 roots per family, 512 directory reads,
/// 1024 entries per directory, 32768 total entries, 16384 probes, 1024 candidates.
/// Deps JSON is limited to 1 MiB and 4096 manifest item visits. Limits and filesystem
/// errors are diagnostic; a truncated result is not evidence that no exact identity exists.
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
    let hint = target_framework.and_then(FrameworkHint::parse);
    search.dotnet(requested, hint.as_ref(), environment);
    search.framework_and_gac(requested, hint.as_ref(), environment);
    search.mono(requested, environment);
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
        let mut items = 0;
        for root in env.nuget_roots.iter().take(MAX_ROOTS) {
            let canonical_root = root.canonicalize().ok();
            for (name, entry) in selected {
                items += 1;
                if items > MAX_MANIFEST_ITEMS {
                    self.diagnostic("deps manifest item limit reached");
                    return;
                }
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
                        items += 1;
                        if items > MAX_MANIFEST_ITEMS {
                            self.diagnostic("deps manifest item limit reached");
                            return;
                        }
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
                        match (&canonical_root, candidate.canonicalize()) {
                            (Some(root), Ok(path)) if path.starts_with(root) => {
                                self.probe(candidate, CandidateOrigin::NuGet)
                            }
                            (Some(_), Ok(_)) => {
                                self.diagnostic("deps asset symlink escapes NuGet root")
                            }
                            (_, Err(error)) if error.kind() != std::io::ErrorKind::NotFound => {
                                self.diagnostic(format!(
                                    "cannot inspect deps asset {}: {error}",
                                    candidate.display()
                                ));
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

#[derive(Debug)]
struct Version {
    numbers: [u32; 4],
    preview: Option<String>,
}

impl Version {
    fn parse(value: &str) -> Option<Self> {
        if !safe_component(value) {
            return None;
        }
        let (numeric, preview) = value
            .split_once('-')
            .map_or((value, None), |(n, p)| (n, Some(p)));
        if preview.is_some_and(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c))
        }) {
            return None;
        }
        let parts: Vec<_> = numeric.split('.').collect();
        if !(2..=4).contains(&parts.len()) {
            return None;
        }
        let mut numbers = [0; 4];
        for (index, part) in parts.iter().enumerate() {
            if part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            numbers[index] = part.parse().ok()?;
        }
        Some(Self {
            numbers,
            preview: preview.map(str::to_owned),
        })
    }

    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.numbers
            .cmp(&other.numbers)
            .then_with(|| match (&self.preview, &other.preview) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (Some(a), Some(b)) => natural_cmp(a, b),
            })
    }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a = a.as_bytes();
    let mut b = b.as_bytes();
    while !a.is_empty() && !b.is_empty() {
        let order = if a[0].is_ascii_digit() && b[0].is_ascii_digit() {
            let an = a.iter().take_while(|c| c.is_ascii_digit()).count();
            let bn = b.iter().take_while(|c| c.is_ascii_digit()).count();
            let av = a[..an].iter().position(|c| *c != b'0').unwrap_or(an);
            let bv = b[..bn].iter().position(|c| *c != b'0').unwrap_or(bn);
            let order = (an - av)
                .cmp(&(bn - bv))
                .then_with(|| a[av..an].cmp(&b[bv..bn]));
            a = &a[an..];
            b = &b[bn..];
            order
        } else {
            let order = a[0].cmp(&b[0]);
            a = &a[1..];
            b = &b[1..];
            order
        };
        if !order.is_eq() {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

struct FrameworkHint {
    family: String,
    version: Version,
}

impl FrameworkHint {
    fn parse(value: &str) -> Option<Self> {
        let mut parts = value.split(',').map(str::trim);
        let family = parts.next()?;
        if !matches!(
            family,
            ".NETCoreApp" | ".NET" | ".NETStandard" | ".NETFramework"
        ) {
            return None;
        }
        let version = parts.find_map(|p| p.strip_prefix("Version=v"))?;
        Some(Self {
            family: family.to_owned(),
            version: Version::parse(version)?,
        })
    }

    fn tfm(&self) -> Option<String> {
        let [major, minor, _, _] = self.version.numbers;
        let prefix = match self.family.as_str() {
            ".NETStandard" => "netstandard",
            ".NETCoreApp" if major < 5 => "netcoreapp",
            ".NETCoreApp" | ".NET" => "net",
            _ => return None,
        };
        Some(format!("{prefix}{major}.{minor}"))
    }
}

impl Search {
    // Overfull directories are rejected whole instead of retaining a filesystem-order prefix.
    fn directories(&mut self, path: &Path) -> Vec<PathBuf> {
        if self.directories >= MAX_DIRECTORY_READS || self.entries >= MAX_TOTAL_ENTRIES {
            self.diagnostic("directory scan limit reached");
            return Vec::new();
        }
        self.directories += 1;
        let read = match std::fs::read_dir(path) {
            Ok(read) => read,
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    self.diagnostic(format!("cannot scan {}: {error}", path.display()));
                }
                return Vec::new();
            }
        };
        let mut result = Vec::new();
        for (index, entry) in read.enumerate() {
            self.entries += 1;
            if index == MAX_DIRECTORY_ENTRIES || self.entries > MAX_TOTAL_ENTRIES {
                self.diagnostic(format!("directory entry limit reached: {}", path.display()));
                return Vec::new();
            }
            match entry {
                Ok(entry) => match std::fs::metadata(entry.path()) {
                    Ok(metadata)
                        if metadata.is_dir()
                            && entry.file_name().to_str().is_some_and(safe_component) =>
                    {
                        result.push(entry.path())
                    }
                    Ok(_) => {}
                    Err(error) => self.diagnostic(format!(
                        "cannot inspect {}: {error}",
                        entry.path().display()
                    )),
                },
                Err(error) => self.diagnostic(format!("cannot read directory entry: {error}")),
            }
        }
        result.sort();
        result
    }

    fn versions(&mut self, path: &Path, prefix: &str, hint: Option<&Version>) -> Vec<PathBuf> {
        let mut versions: Vec<_> = self
            .directories(path)
            .into_iter()
            .filter_map(|path| {
                let version = Version::parse(path.file_name()?.to_str()?.strip_prefix(prefix)?)?;
                Some((path, version))
            })
            .collect();
        versions.sort_by(|(ap, a), (bp, b)| {
            let matches = |v: &Version| hint.is_some_and(|h| h.numbers[..2] == v.numbers[..2]);
            matches(b)
                .cmp(&matches(a))
                .then_with(|| a.cmp(b))
                .then_with(|| ap.cmp(bp))
        });
        versions.into_iter().map(|(path, _)| path).collect()
    }

    fn dotnet(
        &mut self,
        requested: &AssemblyIdentity,
        hint: Option<&FrameworkHint>,
        env: &DiscoveryEnvironment,
    ) {
        if env.dotnet_roots.len() > MAX_ROOTS {
            self.diagnostic("dotnet root limit reached");
        }
        for root in env.dotnet_roots.iter().take(MAX_ROOTS) {
            for pack in [
                "Microsoft.NETCore.App",
                "Microsoft.WindowsDesktop.App",
                "Microsoft.AspNetCore.App",
                "Microsoft.AspNetCore.All",
            ] {
                for version in self.versions(
                    &root.join("shared").join(pack),
                    "",
                    hint.map(|h| &h.version),
                ) {
                    self.pair(&version, &requested.name, CandidateOrigin::SharedFramework);
                }
            }
            for pack in [
                "Microsoft.NETCore.App.Ref",
                "NETStandard.Library.Ref",
                "Microsoft.WindowsDesktop.App.Ref",
                "Microsoft.AspNetCore.App.Ref",
            ] {
                for version in
                    self.versions(&root.join("packs").join(pack), "", hint.map(|h| &h.version))
                {
                    let reference = version.join("ref");
                    if let Some(hint) = hint {
                        if let Some(tfm) = hint.tfm() {
                            self.pair(
                                &reference.join(tfm),
                                &requested.name,
                                CandidateOrigin::ReferencePack,
                            );
                        }
                    } else {
                        for tfm in self.directories(&reference) {
                            let supported =
                                tfm.file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|name| {
                                        ["netstandard", "netcoreapp", "net"].iter().any(|prefix| {
                                            name.strip_prefix(prefix)
                                                .and_then(Version::parse)
                                                .is_some()
                                        })
                                    });
                            if supported {
                                self.pair(&tfm, &requested.name, CandidateOrigin::ReferencePack);
                            }
                        }
                    }
                }
            }
        }
    }

    fn framework_and_gac(
        &mut self,
        requested: &AssemblyIdentity,
        hint: Option<&FrameworkHint>,
        env: &DiscoveryEnvironment,
    ) {
        let mut gac_roots = Vec::new();
        if let Some(windows) = &env.windows_dir {
            for (root, prefix) in [
                (windows.join("assembly"), ""),
                (windows.join("Microsoft.NET/assembly"), "v4.0_"),
            ] {
                for folder in ["GAC_MSIL", "GAC_32", "GAC_64", "GAC"] {
                    gac_roots.push((root.join(folder), prefix, CandidateOrigin::Gac));
                }
            }
        }
        if env.mono_roots.len() > MAX_ROOTS {
            self.diagnostic("Mono root limit reached");
        }
        gac_roots.extend(
            env.mono_roots
                .iter()
                .take(MAX_ROOTS)
                .map(|root| (root.join("gac"), "", CandidateOrigin::Mono)),
        );
        self.gac(requested, &gac_roots);
        let Some(windows) = &env.windows_dir else {
            return;
        };
        for kind in ["Framework", "Framework64"] {
            let base = windows.join("Microsoft.NET").join(kind);
            for version in self.versions(
                &base,
                "v",
                hint.filter(|h| h.family == ".NETFramework")
                    .map(|h| &h.version),
            ) {
                self.pair(&version, &requested.name, CandidateOrigin::Framework);
                self.pair(
                    &version.join("Facades"),
                    &requested.name,
                    CandidateOrigin::Framework,
                );
            }
        }
    }

    fn gac(&mut self, requested: &AssemblyIdentity, roots: &[(PathBuf, &str, CandidateOrigin)]) {
        if requested.public_key_token.is_empty() {
            return;
        }
        if requested.public_key_token.len() != 8 {
            self.diagnostic("GAC requires an eight-byte public key token");
            return;
        }
        let token: String = requested
            .public_key_token
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let suffix = format!("_{}_{token}", requested.culture);
        let [major, minor, build, revision] = requested.version;
        for (root, prefix, origin) in roots {
            self.probe(
                root.join(&requested.name)
                    .join(format!(
                        "{prefix}{major}.{minor}.{build}.{revision}{suffix}"
                    ))
                    .join(format!("{}.dll", requested.name)),
                *origin,
            );
        }
        for (root, prefix, origin) in roots {
            if *origin == CandidateOrigin::Mono {
                continue;
            }
            let mut versions: Vec<_> = self
                .directories(&root.join(&requested.name))
                .into_iter()
                .filter_map(|path| {
                    let text = path
                        .file_name()?
                        .to_str()?
                        .strip_prefix(prefix)?
                        .strip_suffix(&suffix)?;
                    if text.split('.').count() != 4 {
                        return None;
                    }
                    let version = Version::parse(text)?;
                    if version.preview.is_some() || version.numbers[0] != u32::from(major) {
                        return None;
                    }
                    Some((path, version))
                })
                .collect();
            versions.sort_by(|(ap, a), (bp, b)| a.cmp(b).then_with(|| ap.cmp(bp)));
            for (version, _) in versions {
                self.probe(version.join(format!("{}.dll", requested.name)), *origin);
            }
        }
    }

    fn mono(&mut self, requested: &AssemblyIdentity, env: &DiscoveryEnvironment) {
        for root in env.mono_roots.iter().take(MAX_ROOTS) {
            for version in self.versions(root, "", None) {
                self.pair(&version, &requested.name, CandidateOrigin::Mono);
                self.pair(
                    &version.join("Facades"),
                    &requested.name,
                    CandidateOrigin::Mono,
                );
            }
        }
    }
}

/// Collect existing local installation/cache roots without starting any executable.
/// This is a filesystem heuristic, not .NET host runtime selection. Inject roots
/// directly when reproducibility or installations outside these layouts matter.
pub fn environment_from_host() -> DiscoveryEnvironment {
    let mut result = DiscoveryEnvironment::default();
    let arch = match std::env::consts::ARCH {
        "x86_64" => "X64",
        "x86" => "X86",
        "aarch64" => "ARM64",
        "arm" => "ARM",
        other => other,
    }
    .to_ascii_uppercase();
    for key in [format!("DOTNET_ROOT_{arch}"), "DOTNET_ROOT".to_owned()] {
        if let Some(root) = std::env::var_os(key).filter(|s| !s.is_empty()) {
            host_dotnet_root(&mut result.dotnet_roots, Path::new(&root));
        }
    }
    if cfg!(windows) {
        for key in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = std::env::var_os(key).filter(|s| !s.is_empty()) {
                host_dotnet_root(
                    &mut result.dotnet_roots,
                    &PathBuf::from(root).join("dotnet"),
                );
            }
        }
    } else {
        for file in [
            format!("/etc/dotnet/install_location_{}", arch.to_ascii_lowercase()),
            "/etc/dotnet/install_location".to_owned(),
        ] {
            if let Ok(bytes) = bounded_read(Path::new(&file), 4096) {
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    if let Some(first) = text
                        .lines()
                        .next()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                    {
                        host_dotnet_root(&mut result.dotnet_roots, Path::new(first));
                    }
                }
            }
        }
        for root in [
            "/usr/share/dotnet",
            "/usr/lib/dotnet",
            "/usr/local/share/dotnet",
        ] {
            host_dotnet_root(&mut result.dotnet_roots, Path::new(root));
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        let executable = if cfg!(windows) {
            "dotnet.exe"
        } else {
            "dotnet"
        };
        for root in std::env::split_paths(&path)
            .filter(|p| !p.as_os_str().is_empty())
            .take(MAX_ROOTS)
        {
            let file = root.join(executable);
            if file.is_file() {
                if let Ok(file) = file.canonicalize() {
                    if let Some(parent) = file.parent() {
                        push_host_root(&mut result.dotnet_roots, parent);
                    }
                }
            }
        }
    }
    if let Some(root) = std::env::var_os("NUGET_PACKAGES").filter(|s| !s.is_empty()) {
        push_host_root(&mut result.nuget_roots, Path::new(&root));
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    if let Some(home) = home.filter(|s| !s.is_empty()) {
        push_host_root(
            &mut result.nuget_roots,
            &PathBuf::from(home).join(".nuget/packages"),
        );
    }
    result.windows_dir = std::env::var_os("WINDIR")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .and_then(|p| p.canonicalize().ok());
    if let Some(prefixes) = std::env::var_os("MONO_GAC_PREFIX") {
        for prefix in std::env::split_paths(&prefixes)
            .filter(|p| !p.as_os_str().is_empty())
            .take(MAX_ROOTS)
        {
            push_host_root(&mut result.mono_roots, &prefix.join("lib/mono"));
        }
    }
    for root in [
        "/usr/lib/mono",
        "/usr/local/lib/mono",
        "/Library/Frameworks/Mono.framework/Versions/Current/lib/mono",
    ] {
        push_host_root(&mut result.mono_roots, Path::new(root));
    }
    result
}

fn host_dotnet_root(roots: &mut Vec<PathBuf>, root: &Path) {
    let executable = if cfg!(windows) {
        "dotnet.exe"
    } else {
        "dotnet"
    };
    if root.join(executable).is_file()
        || root.join("shared").is_dir()
        || root.join("packs").is_dir()
    {
        push_host_root(roots, root);
    }
}

fn push_host_root(roots: &mut Vec<PathBuf>, root: &Path) {
    if roots.len() < MAX_ROOTS && root.is_dir() {
        if let Ok(root) = root.canonicalize() {
            if !roots.contains(&root) {
                roots.push(root);
            }
        }
    }
}
