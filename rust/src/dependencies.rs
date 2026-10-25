// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{Result, metadata::Image};
use cildec::TableId;
use sha1::Digest;
use std::collections::BTreeMap;

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
        self.name.eq_ignore_ascii_case(&other.name)
            && self.version == other.version
            && self.culture.eq_ignore_ascii_case(&other.culture)
            && self.public_key_token == other.public_key_token
            && self.content_type == other.content_type
    }
}

#[derive(Clone, Debug)]
pub struct AssemblyReference {
    pub token: u32,
    pub identity: AssemblyIdentity,
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
}

#[derive(Clone, Copy, Debug)]
pub struct DependencyLimits {
    pub max_assemblies: usize,
    pub max_bytes: usize,
}

impl Default for DependencyLimits {
    fn default() -> Self {
        Self {
            max_assemblies: 256,
            max_bytes: 512 * 1024 * 1024,
        }
    }
}

/// An explicitly supplied metadata universe. Source labels are provenance only;
/// this interface never opens files, searches directories or executes assemblies.
pub struct DependencyContext {
    main: Vec<u8>,
    references: Vec<AssemblyReference>,
    candidates: Vec<(String, Vec<u8>, AssemblyIdentity)>,
    sources: BTreeMap<String, usize>,
    limits: DependencyLimits,
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
        for rid in 1..=image.md.tables().row_count(TableId::AssemblyRef) {
            let row = image.md.tables().assembly_ref(rid)?;
            references.push(AssemblyReference {
                token: 0x23000000 | rid,
                identity: AssemblyIdentity {
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
                },
            });
        }
        Ok(Self {
            main: bytes,
            references,
            candidates: Vec::new(),
            sources: BTreeMap::new(),
            limits,
        })
    }

    pub fn references(&self) -> &[AssemblyReference] {
        &self.references
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
        let used = self.main.len()
            + self
                .candidates
                .iter()
                .map(|(_, bytes, _)| bytes.len())
                .sum::<usize>();
        if bytes.len() > self.limits.max_bytes - used {
            return Err("dependency byte limit exceeded".into());
        }
        let image = Image::parse(&bytes)?;
        if image.md.tables().row_count(TableId::Assembly) != 1 {
            return Err("dependency needs exactly one Assembly definition".into());
        }
        let row = image.md.tables().assembly(1)?;
        let public_key = image.md.blobs().get(row.public_key)?;
        if !public_key.is_empty() && row.flags & 1 == 0 {
            return Err("Assembly definition key requires the full-public-key flag".into());
        }
        let identity = AssemblyIdentity {
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
        };
        self.sources
            .insert(source.to_string(), self.candidates.len());
        self.candidates.push((source.to_string(), bytes, identity));
        Ok(())
    }

    pub fn resolve(&self, reference_token: u32) -> Result<AssemblyResolution> {
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
