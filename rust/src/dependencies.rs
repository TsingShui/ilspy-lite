// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

mod binding;
mod search;
pub use binding::{
    DefinitionLocation, PropertyAccessor, ResolvedField, ResolvedMethod, ResolvedType,
};
pub use search::ReferenceSearchOptions;

use crate::{DecompiledMethod, DecompiledType, OutputStyle, Result, metadata::Image};
use cildec::TableId;
use sha1::Digest;
use std::collections::BTreeMap;
use std::io::Read;

/// Metadata identity, not verification of a publisher or strong-name signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyIdentity {
    pub name: String,
    pub version: [u16; 4],
    pub culture: String,
    pub public_key_token: Vec<u8>,
    pub content_type: u32,
}

impl AssemblyIdentity {
    fn matches(&self, other: &Self) -> bool {
        self.version == other.version && self.same_contract_scope(other)
    }

    fn same_contract_scope(&self, other: &Self) -> bool {
        self.name.eq_ignore_ascii_case(&other.name)
            && self.culture.eq_ignore_ascii_case(&other.culture)
            && self.public_key_token == other.public_key_token
            && self.content_type == other.content_type
    }
}

impl std::fmt::Display for AssemblyIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let token = if self.public_key_token.is_empty() {
            "null".to_string()
        } else {
            self.public_key_token
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        };
        write!(
            f,
            "{}, Version={}.{}.{}.{}, Culture={}, PublicKeyToken={}, ContentType=0x{:X}",
            self.name.escape_default(),
            self.version[0],
            self.version[1],
            self.version[2],
            self.version[3],
            if self.culture.is_empty() {
                "neutral".to_string()
            } else {
                self.culture.escape_default().to_string()
            },
            token,
            self.content_type
        )
    }
}

#[derive(Clone, Debug)]
pub struct AssemblyReference {
    pub token: u32,
    pub identity: AssemblyIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceFailure {
    pub token: u32,
    pub reason: String,
    pub raw_row: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReferenceVersionPolicy {
    #[default]
    Exact,
    /// Explicit opt-in metadata substitution to the nearest nonolder same-major
    /// identity. This does not certify API compatibility or runtime behavior.
    SameMajor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssemblyResolution {
    Resolved {
        identity: AssemblyIdentity,
        source: String,
    },
    Missing {
        requested: AssemblyIdentity,
    },
    Conflicting {
        requested: AssemblyIdentity,
        sources: Vec<String>,
    },
    Invalid {
        failure: ReferenceFailure,
    },
    Substituted {
        requested: AssemblyIdentity,
        selected: AssemblyIdentity,
        source: String,
        policy: ReferenceVersionPolicy,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct DependencyLimits {
    pub max_assemblies: usize,
    pub max_bytes: usize,
    pub max_references_per_image: u32,
}

impl Default for DependencyLimits {
    fn default() -> Self {
        Self {
            max_assemblies: 256,
            max_bytes: 512 * 1024 * 1024,
            max_references_per_image: 65536,
        }
    }
}

/// An explicitly supplied metadata universe. Source labels are provenance only;
/// this interface never opens files, searches directories or executes assemblies.
pub struct DependencyContext {
    main: Vec<u8>,
    references: Vec<AssemblyReference>,
    reference_failures: Vec<ReferenceFailure>,
    candidates: Vec<(String, Vec<u8>, AssemblyIdentity)>,
    sources: BTreeMap<String, usize>,
    limits: DependencyLimits,
    search_evidence: Vec<String>,
    search_diagnostics: Vec<String>,
    version_policy: ReferenceVersionPolicy,
}

impl DependencyContext {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        Self::with_limits(bytes, DependencyLimits::default())
    }

    pub fn with_limits(bytes: Vec<u8>, limits: DependencyLimits) -> Result<Self> {
        if limits.max_assemblies == 0 {
            return Err("dependency assembly count limit excludes main image".into());
        }
        if bytes.len() > limits.max_bytes {
            return Err("main image exceeds dependency byte limit".into());
        }
        let image = Image::parse(&bytes)?;
        let mut references = Vec::new();
        let mut reference_failures = Vec::new();
        let count = reference_count(&image, limits)?;
        for rid in 1..=count {
            let token = 0x23000000 | rid;
            let identity = reference_identity(&image, rid);
            match identity {
                Ok(identity) => references.push(AssemblyReference { token, identity }),
                Err(error) => reference_failures.push(ReferenceFailure {
                    token,
                    reason: error.to_string(),
                    raw_row: image
                        .md
                        .tables()
                        .row_bytes(TableId::AssemblyRef, rid)?
                        .to_vec(),
                }),
            }
        }
        Ok(Self {
            main: bytes,
            references,
            reference_failures,
            candidates: Vec::new(),
            sources: BTreeMap::new(),
            limits,
            search_evidence: Vec::new(),
            search_diagnostics: Vec::new(),
            version_policy: ReferenceVersionPolicy::Exact,
        })
    }

    /// Analyze the main image and retain dependency provenance separately from
    /// the original body/metadata status accounting.
    pub fn decompile_type(&self, name: &str, style: OutputStyle) -> Result<DecompiledType> {
        let mut result = crate::decompile_type_with_style(&self.main, name, style)?;
        self.append_report(&mut result.text, style)?;
        Ok(result)
    }

    pub fn decompile(&self, token: u32, style: OutputStyle) -> Result<DecompiledMethod> {
        let mut result = crate::decompile_with_style(&self.main, token, style)?;
        self.append_report(&mut result.text, style)?;
        Ok(result)
    }

    fn append_report(&self, text: &mut String, style: OutputStyle) -> Result<()> {
        if style == OutputStyle::Details {
            for evidence in &self.search_evidence {
                text.push_str(&format!("\n{evidence}\n"));
            }
            for diagnostic in &self.search_diagnostics {
                text.push_str(&format!("\n.dependency-search-diagnostic {diagnostic:?}\n"));
            }
            for (source, index) in &self.sources {
                let identity = &self.candidates[*index].2;
                text.push_str(&format!(
                    "\n.dependency-candidate source={source:?} identity={:?}\n",
                    identity.to_string()
                ));
            }
        }
        let mut tokens = self
            .references
            .iter()
            .map(|r| r.token)
            .chain(self.reference_failures.iter().map(|r| r.token))
            .collect::<Vec<_>>();
        tokens.sort_unstable();
        for token in tokens {
            let (state, evidence, warning) = match self.resolve(token)? {
                AssemblyResolution::Resolved { identity, source } => {
                    let requested = &self
                        .references
                        .iter()
                        .find(|r| r.token == token)
                        .ok_or("resolved reference lost its request identity")?
                        .identity;
                    (
                        "resolved",
                        format!(
                            "requested={:?} selected={:?} source={source:?}",
                            requested.to_string(),
                            identity.to_string()
                        ),
                        None,
                    )
                }
                AssemblyResolution::Missing { requested } => (
                    "missing",
                    format!(
                        "requested={:?} no matching supplied identity",
                        requested.to_string()
                    ),
                    Some(format!(
                        "Dependency unresolved: {requested}; reference identity retained."
                    )),
                ),
                AssemblyResolution::Conflicting { requested, sources } => (
                    "conflicting",
                    format!("requested={:?} sources={sources:?}", requested.to_string()),
                    Some(format!(
                        "Dependency conflicting: {requested}; no candidate selected."
                    )),
                ),
                AssemblyResolution::Substituted {
                    requested,
                    selected,
                    source,
                    policy,
                } => (
                    "substituted",
                    format!(
                        "requested={:?} selected={:?} source={source:?} policy={policy:?} evidence=metadata-only",
                        requested.to_string(),
                        selected.to_string()
                    ),
                    Some(format!(
                        "Dependency version substituted: {requested} -> {selected}; {policy:?}, metadata only, not original-runtime behavior proof."
                    )),
                ),
                AssemblyResolution::Invalid { failure } => (
                    "invalid",
                    format!(
                        "reason={:?} raw-row={:02X?}",
                        failure.reason, failure.raw_row
                    ),
                    Some(format!(
                        "Dependency reference invalid: 0x{token:08X}: {}",
                        failure.reason.escape_default()
                    )),
                ),
            };
            if style == OutputStyle::Details {
                text.push_str(&format!(
                    "\n.dependency 0x{token:08X} status={state} {evidence}\n"
                ));
            } else if let Some(warning) = warning {
                text.push_str(&format!("\n// {warning}\n"));
            }
        }
        Ok(())
    }

    /// Remaining owned-image budget, for bounded reads by an external loader.
    pub fn remaining_bytes(&self) -> usize {
        self.limits.max_bytes
            - self.main.len()
            - self
                .candidates
                .iter()
                .map(|(_, bytes, _)| bytes.len())
                .sum::<usize>()
    }

    /// References with readable identities; malformed rows remain separately
    /// available through reference_failures() and resolve(token).
    pub fn references(&self) -> &[AssemblyReference] {
        &self.references
    }

    pub fn reference_failures(&self) -> &[ReferenceFailure] {
        &self.reference_failures
    }

    /// Read an explicitly supplied stream; this method does not open paths.
    pub fn add_reference_reader(&mut self, source: &str, reader: impl Read) -> Result<()> {
        let mut bytes = Vec::new();
        let limit = self.remaining_bytes().min(512 * 1024 * 1024);
        reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        // The extra byte is an overflow sentinel, never a complete snapshot.
        // Reject before content deduplication could mistake a prefix for an alias.
        if bytes.len() > limit {
            return Err("dependency read limit exceeded; complete snapshot required".into());
        }
        self.add_reference(source, bytes)
    }

    pub fn add_reference(&mut self, source: &str, bytes: Vec<u8>) -> Result<()> {
        if let Some(&index) = self.sources.get(source) {
            return if self.candidates[index].1 == bytes {
                Ok(())
            } else {
                Err("dependency source changed within immutable context".into())
            };
        }
        if let Some(index) = self
            .candidates
            .iter()
            .position(|(_, cached, _)| *cached == bytes)
        {
            self.sources.insert(source.to_string(), index);
            return Ok(());
        }
        if self.candidates.len() + 1 >= self.limits.max_assemblies {
            return Err("dependency assembly count limit exceeded".into());
        }
        if bytes.len() > self.remaining_bytes() {
            return Err("dependency byte limit exceeded".into());
        }
        let image = Image::parse(&bytes)?;
        reference_count(&image, self.limits)?;
        let identity = assembly_identity(&image)?;
        self.sources
            .insert(source.to_string(), self.candidates.len());
        self.candidates.push((source.to_string(), bytes, identity));
        Ok(())
    }

    pub fn resolve(&self, reference_token: u32) -> Result<AssemblyResolution> {
        let exact = self.resolve_exact(reference_token)?;
        let AssemblyResolution::Missing { requested } = exact else {
            return Ok(exact);
        };
        if self.version_policy == ReferenceVersionPolicy::Exact {
            return Ok(AssemblyResolution::Missing { requested });
        }
        let selected = self
            .candidates
            .iter()
            .filter(|(_, _, identity)| {
                requested.same_contract_scope(identity)
                    && requested.version[0] == identity.version[0]
                    && requested.version <= identity.version
            })
            .map(|(_, _, identity)| identity.version)
            .min();
        let Some(version) = selected else {
            return Ok(AssemblyResolution::Missing { requested });
        };
        let candidates = self
            .candidates
            .iter()
            .filter(|(_, _, identity)| {
                requested.same_contract_scope(identity) && identity.version == version
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Ok(AssemblyResolution::Conflicting {
                requested,
                sources: candidates.iter().map(|(s, _, _)| s.clone()).collect(),
            });
        }
        let (source, _, selected) = candidates[0];
        Ok(AssemblyResolution::Substituted {
            requested,
            selected: selected.clone(),
            source: source.clone(),
            policy: self.version_policy,
        })
    }

    fn resolve_exact(&self, reference_token: u32) -> Result<AssemblyResolution> {
        if let Some(failure) = self
            .reference_failures
            .iter()
            .find(|r| r.token == reference_token)
        {
            return Ok(AssemblyResolution::Invalid {
                failure: failure.clone(),
            });
        }
        let reference = self
            .references
            .iter()
            .find(|r| r.token == reference_token)
            .ok_or("unknown AssemblyRef token in dependency context")?;
        let candidates = self
            .candidates
            .iter()
            .filter(|(_, _, identity)| reference.identity.matches(identity))
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [] => Ok(AssemblyResolution::Missing {
                requested: reference.identity.clone(),
            }),
            [(source, _, identity)] => Ok(AssemblyResolution::Resolved {
                identity: identity.clone(),
                source: source.clone(),
            }),
            _ => Ok(AssemblyResolution::Conflicting {
                requested: reference.identity.clone(),
                sources: candidates
                    .iter()
                    .map(|(source, _, _)| source.clone())
                    .collect(),
            }),
        }
    }
}

fn assembly_identity(image: &Image<'_>) -> Result<AssemblyIdentity> {
    if image.md.tables().row_count(TableId::Assembly) != 1 {
        return Err("dependency needs exactly one Assembly definition".into());
    }
    let row = image.md.tables().assembly(1)?;
    let public_key = image.md.blobs().get(row.public_key)?;
    if !public_key.is_empty() && row.flags & 1 == 0 {
        return Err("Assembly definition key requires the full-public-key flag".into());
    }
    Ok(AssemblyIdentity {
        name: image.md.strings().str(row.name)?.to_string(),
        version: [
            row.major_version,
            row.minor_version,
            row.build_number,
            row.revision_number,
        ],
        culture: image.md.strings().str(row.culture)?.to_string(),
        public_key_token: key_token(public_key, row.flags & 1 != 0)?,
        content_type: row.flags & 0x0e00,
    })
}

fn reference_identity(image: &Image<'_>, rid: u32) -> Result<AssemblyIdentity> {
    let row = image.md.tables().assembly_ref(rid)?;
    Ok(AssemblyIdentity {
        name: image.md.strings().str(row.name)?.to_string(),
        version: [
            row.major_version,
            row.minor_version,
            row.build_number,
            row.revision_number,
        ],
        culture: image.md.strings().str(row.culture)?.to_string(),
        public_key_token: key_token(
            image.md.blobs().get(row.public_key_or_token)?,
            row.flags & 1 != 0,
        )?,
        content_type: row.flags & 0x0e00,
    })
}

fn reference_count(image: &Image<'_>, limits: DependencyLimits) -> Result<u32> {
    let count = image.md.tables().row_count(TableId::AssemblyRef);
    if count > 0x00ffffff || count > limits.max_references_per_image {
        return Err("dependency reference limit exceeded".into());
    }
    Ok(count)
}

fn key_token(bytes: &[u8], full_key: bool) -> Result<Vec<u8>> {
    if bytes.is_empty() {
        return if full_key {
            Err("full-public-key flag without a key".into())
        } else {
            Ok(Vec::new())
        };
    }
    if full_key {
        // ECMA-335 assembly identity uses the reversed final eight SHA-1 bytes.
        // This is a legacy identifier calculation, not signature authentication.
        return Ok(sha1::Sha1::digest(bytes)
            .iter()
            .rev()
            .take(8)
            .copied()
            .collect());
    }
    if bytes.len() != 8 {
        return Err("assembly public-key token must contain eight bytes".into());
    }
    Ok(bytes.to_vec())
}
