// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::{DependencyContext, ReferenceVersionPolicy};
use crate::{
    Result,
    discovery_paths::{DiscoveryEnvironment, candidates_for},
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// Filesystem lookup is opt-in at this explicit interface. Default roots are
/// empty for deterministic embedding; the CLI injects host-discovered roots.
#[derive(Clone, Debug)]
pub struct ReferenceSearchOptions {
    pub automatic: bool,
    pub files: Vec<PathBuf>,
    pub directories: Vec<PathBuf>,
    pub environment: DiscoveryEnvironment,
    pub version_policy: ReferenceVersionPolicy,
    /// Loader inspection attempts (including absent files and supplied dirs).
    /// Automatic path enumeration has its own documented finite bounds.
    pub max_loader_probes: usize,
}

impl Default for ReferenceSearchOptions {
    fn default() -> Self {
        Self {
            automatic: true,
            files: Vec::new(),
            directories: Vec::new(),
            environment: DiscoveryEnvironment::default(),
            version_policy: ReferenceVersionPolicy::IlSpy,
            max_loader_probes: 4096,
        }
    }
}

struct ProbeBudget {
    used: usize,
    limit: usize,
}
impl ProbeBudget {
    fn take(&mut self) -> bool {
        if self.used >= self.limit {
            false
        } else {
            self.used += 1;
            true
        }
    }
}

impl DependencyContext {
    /// Probe local files only. Every candidate still passes the byte-context
    /// identity, conflict and complete-snapshot checks before it can bind.
    pub fn discover_references(
        &mut self,
        input: &Path,
        options: &ReferenceSearchOptions,
    ) -> Result<()> {
        self.version_policy = options.version_policy;
        let mut seen = BTreeSet::new();
        let mut budget = ProbeBudget {
            used: 0,
            limit: options.max_loader_probes.min(4096),
        };
        for path in &options.files {
            if !self.search_file(path, "ExplicitFile", &mut seen, &mut budget) {
                return Ok(());
            }
        }
        for directory in &options.directories {
            if !budget.take() {
                self.search_failure("dependency file probe limit exceeded".into());
                return Ok(());
            }
            match std::fs::metadata(directory) {
                Ok(meta) if meta.is_dir() => {}
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => self.search_failure(
                    format!("cannot inspect reference directory {directory:?}: {error}"),
                ),
                _ => self.search_note(format!("reference directory unavailable: {directory:?}")),
            }
        }
        let framework = match self.target_framework_hint() {
            Ok(hint) => hint,
            Err(error) => {
                self.search_failure(format!("target framework hint unavailable: {error}"));
                None
            }
        };
        let mut queue = std::collections::VecDeque::from([(0usize, input.to_path_buf(), 0usize)]);
        let mut visited = BTreeSet::new();
        let mut work = 0usize;
        'closure: while let Some((module, referrer, depth)) = queue.pop_front() {
            if !visited.insert(module) {
                continue;
            }
            if depth > 32 {
                self.search_failure("dependency closure depth limit exceeded".into());
                continue;
            }
            let image = self.binding_image(module)?;
            let count = super::reference_count(&image, self.limits)?;
            if work + count as usize > 4096 {
                drop(image);
                self.search_failure("dependency closure request limit exceeded".into());
                break 'closure;
            }
            work += count as usize;
            let forwarded =
                if options.version_policy == ReferenceVersionPolicy::IlSpy && module != 0 {
                    let mut targets = BTreeSet::new();
                    for (rid, row) in image.md.tables().iter::<cildec::tables::ExportedTypeRow>() {
                        row?;
                        let target = image.metadata_coded_token(
                            cildec::CodedIndex::Implementation,
                            image
                                .md
                                .tables()
                                .raw_column(cildec::TableId::ExportedType, rid, 4)?,
                        )?;
                        if target.table() == Some(cildec::TableId::AssemblyRef) {
                            targets.insert(target.rid());
                        }
                    }
                    Some(targets)
                } else {
                    None
                };
            let mut requests = Vec::new();
            let mut invalid = Vec::new();
            for rid in 1..=count {
                if forwarded
                    .as_ref()
                    .is_some_and(|targets| !targets.contains(&rid))
                {
                    continue;
                }
                match super::reference_identity(&image, rid) {
                    Ok(identity) => requests.push((rid, identity)),
                    Err(error) => invalid.push((
                        rid,
                        error.to_string(),
                        image
                            .md
                            .tables()
                            .row_bytes(cildec::TableId::AssemblyRef, rid)?
                            .to_vec(),
                    )),
                }
            }
            // Drop borrowed image metadata before extending immutable snapshots.
            drop(image);
            for (rid, reason, raw) in invalid {
                self.search_evidence.push(format!(".dependency-request referrer={referrer:?} token=0x{:08X} status=invalid reason={reason:?} raw-row={raw:02X?}", 0x23000000 | rid));
            }
            for (rid, identity) in requests {
                if !safe_name(&identity.name) {
                    self.search_failure(format!(
                        "unsafe assembly filename in {referrer:?}: {:?}",
                        identity.name
                    ));
                    continue;
                }
                if !self.search_satisfied(&identity) {
                    if budget.used >= budget.limit {
                        self.search_failure("dependency file probe limit exceeded".into());
                        break 'closure;
                    }
                    let mut paths = Vec::new();
                    if options.automatic && options.version_policy == ReferenceVersionPolicy::IlSpy
                    {
                        let directory = input.parent().unwrap_or(Path::new("."));
                        for extension in ["dll", "exe"] {
                            paths.push((
                                directory.join(format!("{}.{extension}", identity.name)),
                                "Neighbor".to_string(),
                            ));
                        }
                    }
                    for directory in &options.directories {
                        for extension in ["dll", "exe"] {
                            paths.push((
                                directory.join(format!("{}.{extension}", identity.name)),
                                "ExplicitDirectory".to_string(),
                            ));
                        }
                    }
                    if options.automatic {
                        let found = if options.version_policy == ReferenceVersionPolicy::IlSpy {
                            crate::discovery_paths::candidates_for_ilspy(
                                input,
                                &identity,
                                framework.as_deref(),
                                &options.environment,
                            )
                        } else {
                            candidates_for(
                                &referrer,
                                &identity,
                                framework.as_deref(),
                                &options.environment,
                            )
                        };
                        self.discovery_incomplete |= found.incomplete;
                        for diagnostic in found.diagnostics {
                            self.search_note(diagnostic);
                        }
                        paths.extend(
                            found
                                .candidates
                                .into_iter()
                                .map(|c| (c.path, format!("{:?}", c.origin))),
                        );
                    }
                    for (path, origin) in paths {
                        if self.search_satisfied(&identity) {
                            break;
                        }
                        if !self.search_file(&path, &origin, &mut seen, &mut budget) {
                            break 'closure;
                        }
                    }
                }
                let selected = self.matching_modules(&identity);
                let mut selection = String::new();
                let status = match selected.as_slice() {
                    [target] => {
                        let source = if *target == 0 {
                            input.to_path_buf()
                        } else {
                            PathBuf::from(&self.candidates[*target - 1].0)
                        };
                        let (identity_selected, provenance) = self.identity_source(*target)?;
                        selection = format!(
                            " selected={:?} source={provenance:?} kind={:?}",
                            identity_selected.to_string(),
                            self.kinds[target]
                        );
                        queue.push_back((*target, source, depth + 1));
                        if identity.matches(&identity_selected) {
                            "resolved"
                        } else {
                            "substituted"
                        }
                    }
                    [] => "missing",
                    _ => "conflicting",
                };
                self.search_evidence.push(format!(".dependency-request referrer={referrer:?} token=0x{:08X} status={status} requested={:?}{selection}", 0x23000000 | rid, identity.to_string()));
            }
        }
        Ok(())
    }

    fn search_satisfied(&self, identity: &super::AssemblyIdentity) -> bool {
        if self.version_policy == ReferenceVersionPolicy::IlSpy {
            !self.matching_modules(identity).is_empty()
        } else {
            !self.exact_matching_modules(identity).is_empty()
        }
    }

    fn search_file(
        &mut self,
        path: &Path,
        origin: &str,
        seen: &mut BTreeSet<PathBuf>,
        budget: &mut ProbeBudget,
    ) -> bool {
        if !budget.take() {
            self.search_failure("dependency file probe limit exceeded".into());
            return false;
        }
        match std::fs::metadata(path) {
            Ok(meta) if meta.is_file() => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound && origin != "ExplicitFile" =>
            {
                return true;
            }
            Err(error) => {
                self.search_failure(format!("Dependency input rejected: {path:?}: {error}"));
                return true;
            }
            _ => {
                self.search_failure(format!(
                    "Dependency input rejected: {path:?}: not a regular file"
                ));
                return true;
            }
        }
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let source = canonical
            .to_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("{canonical:?}"));
        if !seen.insert(canonical.clone()) {
            if origin == "ReferencePack" {
                if let Some(index) = self.sources.get(&source) {
                    self.kinds
                        .insert(index + 1, super::AssemblyKind::ReferenceContract);
                    self.search_evidence.push(format!(
                        ".dependency-probe source={source:?} origin={origin}"
                    ));
                }
            }
            return true;
        }
        let read = (|| -> Result<()> {
            self.add_reference_reader(&source, std::fs::File::open(&canonical)?)
        })();
        match read {
            Ok(()) => {
                if origin == "ReferencePack" {
                    if let Some(index) = self.sources.get(&source) {
                        self.kinds
                            .insert(index + 1, super::AssemblyKind::ReferenceContract);
                    }
                }
                self.search_evidence.push(format!(
                    ".dependency-probe source={source:?} origin={origin}"
                ));
            }
            Err(error) => self.search_failure(format!(
                "Dependency input rejected: {source:?}: {}",
                error.to_string().escape_default()
            )),
        }
        true
    }

    fn search_failure(&mut self, note: String) {
        self.discovery_incomplete = true;
        self.search_note(note);
    }

    fn search_note(&mut self, note: String) {
        if self.search_diagnostics.len() < 128 && !self.search_diagnostics.contains(&note) {
            self.search_diagnostics.push(note);
        }
    }

    /// A target hint is read from the main assembly attribute blob. It selects
    /// probe order only; it is not evidence of the analyzer's or target's runtime.
    pub fn target_framework_hint(&self) -> Result<Option<String>> {
        let image = self.binding_image(0)?;
        let mut found = None;
        for (rid, row) in image
            .md
            .tables()
            .iter::<cildec::tables::CustomAttributeRow>()
        {
            let row = row?;
            let parent = image.metadata_coded_token(
                cildec::CodedIndex::HasCustomAttribute,
                image
                    .md
                    .tables()
                    .raw_column(cildec::TableId::CustomAttribute, rid, 0)?,
            )?;
            if parent.0 != 0x20000001 || row.constructor.table() != Some(cildec::TableId::MemberRef)
            {
                continue;
            }
            let ctor = image.md.tables().member_ref(row.constructor.rid())?;
            if image.md.strings().str(ctor.name)? != ".ctor"
                || ctor.class.table() != Some(cildec::TableId::TypeRef)
            {
                continue;
            }
            let ty = image.md.tables().type_ref(ctor.class.rid())?;
            if image.md.strings().str(ty.type_namespace)? != "System.Runtime.Versioning"
                || image.md.strings().str(ty.type_name)? != "TargetFrameworkAttribute"
                || ty.resolution_scope.table() != Some(cildec::TableId::AssemblyRef)
            {
                continue;
            }
            let identity = super::reference_identity(&image, ty.resolution_scope.rid())?;
            if !core_contract(&identity)
                || image.md.blobs().get(ctor.signature)? != [0x20, 1, 1, 0x0e]
            {
                continue;
            }
            let blob = image.md.blobs().get(row.value)?;
            if !blob.starts_with(&[1, 0]) {
                return Err("invalid TargetFrameworkAttribute prolog".into());
            }
            let (size, width) = cildec::compressed::read_u32(&blob[2..])?;
            if size > 4096 {
                return Err("target framework hint exceeds length limit".into());
            }
            let start = 2 + width;
            let value = blob
                .get(start..start + size as usize)
                .ok_or("truncated target framework hint")?;
            if blob.len() < start + size as usize + 2 {
                return Err("target framework attribute missing named argument count".into());
            }
            let value = std::str::from_utf8(value)?.to_string();
            if found.replace(value).is_some() {
                return Err("ambiguous target framework attributes".into());
            }
        }
        Ok(found)
    }

    pub fn search_diagnostics(&self) -> &[String] {
        &self.search_diagnostics
    }
}

pub(super) fn core_contract(identity: &super::AssemblyIdentity) -> bool {
    identity.culture.is_empty()
        && identity.content_type == 0
        && match identity.name.as_str() {
            "System.Runtime" => {
                identity.public_key_token == [0xb0, 0x3f, 0x5f, 0x7f, 0x11, 0xd5, 0x0a, 0x3a]
            }
            "mscorlib" => {
                // Desktop and Silverlight/mobile core libraries use different
                // Microsoft keys; both define the ECMA core types.
                identity.public_key_token == [0xb7, 0x7a, 0x5c, 0x56, 0x19, 0x34, 0xe0, 0x89]
                    || identity.public_key_token == [0x7c, 0xec, 0x85, 0xd7, 0xbe, 0xa7, 0x79, 0x8e]
            }
            "System.Private.CoreLib" => {
                identity.public_key_token == [0x7c, 0xec, 0x85, 0xd7, 0xbe, 0xa7, 0x79, 0x8e]
            }
            "netstandard" => {
                identity.public_key_token == [0xcc, 0x7b, 0x13, 0xff, 0xcd, 0x2d, 0xdd, 0x51]
            }
            _ => false,
        }
}

fn safe_name(name: &str) -> bool {
    !name.is_empty() && !matches!(name, "." | "..") && !name.contains(['/', '\\', ':', '\0'])
}
