// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::{AssemblyResolution, DependencyContext, ReferenceVersionPolicy};
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
}

impl Default for ReferenceSearchOptions {
    fn default() -> Self {
        Self {
            automatic: true,
            files: Vec::new(),
            directories: Vec::new(),
            environment: DiscoveryEnvironment::default(),
            version_policy: ReferenceVersionPolicy::Exact,
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
        for path in &options.files {
            self.search_file(path, "ExplicitFile", &mut seen);
        }
        for directory in &options.directories {
            if !directory.is_dir() {
                self.search_note(format!("reference directory unavailable: {directory:?}"));
            }
        }
        let framework = match self.target_framework_hint() {
            Ok(hint) => hint,
            Err(error) => {
                self.search_note(format!("target framework hint unavailable: {error}"));
                None
            }
        };
        let requests = self.references.clone();
        for reference in requests {
            if !matches!(
                self.resolve_exact(reference.token)?,
                AssemblyResolution::Missing { .. }
            ) {
                continue;
            }
            if !safe_name(&reference.identity.name) {
                self.search_note(format!(
                    "unsafe assembly filename: {:?}",
                    reference.identity.name
                ));
                continue;
            }
            let mut paths = Vec::new();
            for directory in &options.directories {
                for extension in ["dll", "exe"] {
                    paths.push((
                        directory.join(format!("{}.{extension}", reference.identity.name)),
                        "ExplicitDirectory".to_string(),
                    ));
                }
            }
            if options.automatic {
                let found = candidates_for(
                    input,
                    &reference.identity,
                    framework.as_deref(),
                    &options.environment,
                );
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
                if !matches!(
                    self.resolve_exact(reference.token)?,
                    AssemblyResolution::Missing { .. }
                ) {
                    break;
                }
                if path.is_file() {
                    self.search_file(&path, &origin, &mut seen);
                }
            }
        }
        Ok(())
    }

    fn search_file(&mut self, path: &Path, origin: &str, seen: &mut BTreeSet<PathBuf>) {
        if seen.len() >= 4096 {
            self.search_note("dependency file probe limit exceeded".into());
            return;
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
            return;
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
            Err(error) => self.search_note(format!(
                "Dependency input rejected: {source:?}: {}",
                error.to_string().escape_default()
            )),
        }
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
                identity.public_key_token == [0xb7, 0x7a, 0x5c, 0x56, 0x19, 0x34, 0xe0, 0x89]
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
