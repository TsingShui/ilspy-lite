// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::{AssemblyIdentity, AssemblyResolution, DependencyContext};
use crate::{Result, metadata::Image};
use cildec::{TableId, Token};
use std::collections::BTreeSet;

/// A token is meaningful only together with its defining assembly snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub assembly: AssemblyIdentity,
    pub source: String,
    pub token: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedType {
    pub definition: DefinitionLocation,
    pub name: String,
    pub flags: u32,
}

impl DependencyContext {
    /// Resolve an input-image TypeRef without loading or executing its assembly.
    pub fn resolve_type(&self, token: u32) -> Result<ResolvedType> {
        let input = Image::parse(&self.main)?;
        let token = Token(token);
        if token.table() != Some(TableId::TypeRef) || token.rid() == 0 {
            return Err("type binding needs a non-null TypeRef".into());
        }
        let reference = input.md.tables().type_ref(token.rid())?;
        if reference.resolution_scope.table() != Some(TableId::AssemblyRef) {
            return Err("type binding needs an AssemblyRef scope".into());
        }
        let namespace = input.md.strings().str(reference.type_namespace)?;
        let name = input.md.strings().str(reference.type_name)?;
        let AssemblyResolution::Resolved { identity, source } =
            self.resolve(reference.resolution_scope.0)?
        else {
            return Err("type binding requires an unambiguous resolved assembly identity".into());
        };
        let index = *self
            .sources
            .get(&source)
            .ok_or("resolved assembly snapshot is missing")?;
        let definition = Image::parse(&self.candidates[index].1)?;
        let nested = definition
            .md
            .tables()
            .iter::<cildec::tables::NestedClassRow>()
            .map(|(_, row)| row.map(|row| row.nested_class.get()))
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        let mut found = None;
        for (rid, row) in definition.md.tables().iter::<cildec::tables::TypeDefRow>() {
            let row = row?;
            if !nested.contains(&rid)
                && definition.md.strings().str(row.type_namespace)? == namespace
                && definition.md.strings().str(row.type_name)? == name
            {
                if found.is_some() {
                    return Err("ambiguous type definition in resolved assembly".into());
                }
                found = Some((rid, row.flags));
            }
        }
        let (rid, flags) = found.ok_or("type definition missing from resolved assembly")?;
        if rid > 0x00ffffff {
            return Err("type definition RID exceeds token width".into());
        }
        Ok(ResolvedType {
            definition: DefinitionLocation {
                assembly: identity,
                source,
                token: 0x02000000 | rid,
            },
            name: if namespace.is_empty() {
                name.to_string()
            } else {
                format!("{namespace}.{name}")
            },
            flags,
        })
    }
}
