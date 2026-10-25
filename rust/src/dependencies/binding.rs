// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

mod members;
mod presentation;
pub use members::{PropertyAccessor, ResolvedField, ResolvedMethod};

use super::{
    AssemblyIdentity, DependencyContext, ReferenceVersionPolicy, assembly_identity,
    reference_identity,
};
use crate::{Result, metadata::Image};
use cildec::{CodedIndex, TableId, Token};
use std::collections::{BTreeMap, BTreeSet};

/// A token is meaningful only together with its defining assembly snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub assembly: AssemblyIdentity,
    pub source: String,
    pub token: u32,
    pub kind: super::AssemblyKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedType {
    pub definition: DefinitionLocation,
    pub name: String,
    pub flags: u32,
    /// False when any assembly hop used an explicit metadata-version substitute.
    pub exact_identity: bool,
    /// Scoped metadata signature identities in the definition's generic positions.
    pub generic_arguments: Vec<String>,
    /// ExportedType rows followed before reaching this actual definition.
    pub forwarders: Vec<DefinitionLocation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRelation {
    pub reference_token: u32,
    pub resolved: Option<ResolvedType>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRelations {
    pub owner: ResolvedType,
    pub base_type: Option<TypeRelation>,
    pub interfaces: Vec<TypeRelation>,
}

impl DependencyContext {
    /// Return actual relation rows without turning a missing base/interface into
    /// a fabricated definition or discarding the other readable relationships.
    pub fn type_relations(&self, token: u32) -> Result<TypeRelations> {
        let (module, owner) = self.bind_type_in(0, Token(token), &mut BTreeSet::new())?;
        let image = self.binding_image(module)?;
        let rid = owner.definition.token & 0xffffff;
        let base = image.metadata_coded_token(
            CodedIndex::TypeDefOrRef,
            image.md.tables().raw_column(TableId::TypeDef, rid, 3)?,
        )?;
        let relation = |token: Token| -> TypeRelation {
            let result = self.bind_type_in(module, token, &mut BTreeSet::new());
            match result {
                Ok((_, mut bound)) => {
                    if token.table() == Some(TableId::TypeSpec)
                        && !owner.generic_arguments.is_empty()
                    {
                        let specialized = (|| -> Result<Vec<String>> {
                            let row = image.md.tables().type_spec(token.rid())?;
                            let sig = crate::signature_guard::type_spec(
                                image.md.blobs().get(row.signature)?,
                            )?;
                            let cildec::Type::GenericInst { args, .. } = sig.type_ else {
                                return Err("relation TypeSpec is not a named instantiation".into());
                            };
                            args.iter()
                                .map(|a| self.type_key(module, a, &owner.generic_arguments, 0))
                                .collect()
                        })();
                        match specialized {
                            Ok(args) => bound.generic_arguments = args,
                            Err(error) => {
                                return TypeRelation {
                                    reference_token: token.0,
                                    resolved: None,
                                    error: Some(error.to_string()),
                                };
                            }
                        }
                    }
                    TypeRelation {
                        reference_token: token.0,
                        resolved: Some(bound),
                        error: None,
                    }
                }
                Err(error) => TypeRelation {
                    reference_token: token.0,
                    resolved: None,
                    error: Some(error.to_string()),
                },
            }
        };
        let mut interfaces = Vec::new();
        for (interface_rid, row) in image.md.tables().iter::<cildec::tables::InterfaceImplRow>() {
            let row = row?;
            if row.class.get() != rid {
                continue;
            }
            let reference = image.metadata_coded_token(
                CodedIndex::TypeDefOrRef,
                image
                    .md
                    .tables()
                    .raw_column(TableId::InterfaceImpl, interface_rid, 1)?,
            )?;
            interfaces.push(relation(reference));
        }
        let base_type = if base.is_null() {
            None
        } else {
            Some(relation(base))
        };
        Ok(TypeRelations {
            owner,
            base_type,
            interfaces,
        })
    }

    /// Resolve an input-image type without loading or executing its assembly.
    pub fn resolve_type(&self, token: u32) -> Result<ResolvedType> {
        self.bind_type_in(0, Token(token), &mut BTreeSet::new())
            .map(|(_, ty)| ty)
    }

    pub(super) fn binding_image(&self, module: usize) -> Result<Image<'_>> {
        let bytes = if module == 0 {
            &self.main
        } else {
            &self
                .candidates
                .get(module - 1)
                .ok_or("unknown defining snapshot")?
                .1
        };
        Image::parse(bytes)
    }

    pub(super) fn identity_source(&self, module: usize) -> Result<(AssemblyIdentity, String)> {
        if module == 0 {
            return Ok((
                assembly_identity(&self.binding_image(0)?)?,
                "<main>".to_string(),
            ));
        }
        let (source, _, identity) = self
            .candidates
            .get(module - 1)
            .ok_or("unknown defining snapshot")?;
        Ok((identity.clone(), source.clone()))
    }

    pub(super) fn exact_matching_modules(&self, requested: &AssemblyIdentity) -> Vec<usize> {
        let mut found = Vec::new();
        if self
            .binding_image(0)
            .and_then(|image| assembly_identity(&image))
            .is_ok_and(|identity| identity.matches(requested))
        {
            found.push(0);
        }
        for (index, (_, bytes, identity)) in self.candidates.iter().enumerate() {
            if identity.matches(requested) && !(found.first() == Some(&0) && *bytes == self.main) {
                found.push(index + 1);
            }
        }
        found
    }

    pub(super) fn matching_modules(&self, requested: &AssemblyIdentity) -> Vec<usize> {
        let mut found = self.exact_matching_modules(requested);
        if found.is_empty()
            && self.version_policy == ReferenceVersionPolicy::SameMajor
            && !self.discovery_incomplete
        {
            let version = self
                .candidates
                .iter()
                .filter(|(_, _, identity)| {
                    requested.same_contract_scope(identity)
                        && requested.version[0] == identity.version[0]
                        && requested.version <= identity.version
                })
                .map(|(_, _, identity)| identity.version)
                .min();
            if let Some(version) = version {
                found.extend(
                    self.candidates
                        .iter()
                        .enumerate()
                        .filter(|(_, (_, _, identity))| {
                            requested.same_contract_scope(identity) && identity.version == version
                        })
                        .map(|(index, _)| index + 1),
                );
            }
        }
        found
    }

    fn binding_assembly(&self, module: usize, token: Token) -> Result<usize> {
        if token.table() != Some(TableId::AssemblyRef) || token.rid() == 0 {
            return Err("type binding requires a non-null AssemblyRef".into());
        }
        let requested = reference_identity(&self.binding_image(module)?, token.rid())?;
        match self.matching_modules(&requested).as_slice() {
            [only] => Ok(*only),
            [] => Err(format!("type binding requires a resolved assembly: {requested}").into()),
            _ => Err(
                format!("type binding has conflicting assembly definitions: {requested}").into(),
            ),
        }
    }

    fn bind_type_in(
        &self,
        module: usize,
        token: Token,
        visiting: &mut BTreeSet<(usize, u32)>,
    ) -> Result<(usize, ResolvedType)> {
        if token.rid() == 0 || visiting.len() >= 32 || !visiting.insert((module, token.0)) {
            return Err("null, cyclic or excessive type reference".into());
        }
        let result = (|| {
            let image = self.binding_image(module)?;
            match token.table() {
                Some(TableId::TypeDef) => Ok((module, self.describe_type(module, token.rid())?)),
                Some(TableId::TypeRef) => {
                    let reference = image.md.tables().type_ref(token.rid())?;
                    // Validate the full coded RID before cildec's token conversion
                    // can truncate it to 24 bits.
                    let scope = image.metadata_coded_token(
                        CodedIndex::ResolutionScope,
                        image
                            .md
                            .tables()
                            .raw_column(TableId::TypeRef, token.rid(), 0)?,
                    )?;
                    let namespace = image.md.strings().str(reference.type_namespace)?;
                    let name = image.md.strings().str(reference.type_name)?;
                    let mut exact_identity = true;
                    let mut inherited_forwarders = Vec::new();
                    let (target, parent) = match scope.table() {
                        Some(TableId::AssemblyRef) => {
                            let target = self.binding_assembly(module, scope)?;
                            exact_identity = reference_identity(&image, scope.rid())?
                                .matches(&self.identity_source(target)?.0);
                            (target, None)
                        }
                        Some(TableId::Module) if scope.rid() == 1 => {
                            image.md.tables().module(1)?;
                            (module, None)
                        }
                        Some(TableId::TypeRef) => {
                            if !namespace.is_empty() {
                                return Err("nested TypeRef has a namespace".into());
                            }
                            let (parent_module, parent) =
                                self.bind_type_in(module, scope, visiting)?;
                            exact_identity = parent.exact_identity;
                            inherited_forwarders = parent.forwarders;
                            (parent_module, Some(parent.definition.token & 0xffffff))
                        }
                        _ => return Err("unsupported type resolution scope".into()),
                    };
                    let (target, mut resolved) =
                        self.lookup_named_type(target, namespace, name, parent, visiting)?;
                    resolved.exact_identity &= exact_identity;
                    inherited_forwarders.append(&mut resolved.forwarders);
                    resolved.forwarders = inherited_forwarders;
                    Ok((target, resolved))
                }
                Some(TableId::TypeSpec) => {
                    let row = image.md.tables().type_spec(token.rid())?;
                    let signature =
                        crate::signature_guard::type_spec(image.md.blobs().get(row.signature)?)?;
                    let cildec::Type::GenericInst { def, args, .. } = signature.type_ else {
                        return Err("type owner binding needs a named generic TypeSpec".into());
                    };
                    if !matches!(def.table(), Some(TableId::TypeDef | TableId::TypeRef)) {
                        return Err("generic definition must be a TypeDef or TypeRef".into());
                    }
                    let (target, mut bound) = self.bind_type_in(module, def, visiting)?;
                    let definition = self.binding_image(target)?;
                    let count = definition
                        .md
                        .tables()
                        .iter::<cildec::tables::GenericParamRow>()
                        .map(|(_, r)| r)
                        .collect::<std::result::Result<Vec<_>, _>>()?
                        .iter()
                        .filter(|r| r.owner.0 == bound.definition.token)
                        .count();
                    if args.len() != count {
                        return Err(
                            "generic argument count differs from definition positions".into()
                        );
                    }
                    bound.generic_arguments = args
                        .iter()
                        .map(|a| self.type_key(module, a, &[], 1))
                        .collect::<Result<Vec<_>>>()?;
                    Ok((target, bound))
                }
                _ => Err("type binding needs a TypeDef, TypeRef or named generic TypeSpec".into()),
            }
        })();
        visiting.remove(&(module, token.0));
        result
    }

    fn lookup_named_type(
        &self,
        module: usize,
        namespace: &str,
        name: &str,
        parent: Option<u32>,
        visiting: &mut BTreeSet<(usize, u32)>,
    ) -> Result<(usize, ResolvedType)> {
        let image = self.binding_image(module)?;
        let parents = nested_parents(&image)?;
        let mut found = None;
        for (rid, row) in image.md.tables().iter::<cildec::tables::TypeDefRow>() {
            let row = row?;
            if parents.get(&rid).copied() == parent
                && image.md.strings().str(row.type_name)? == name
                && (parent.is_some() || image.md.strings().str(row.type_namespace)? == namespace)
            {
                if found.is_some() {
                    return Err("ambiguous type definition in resolved assembly".into());
                }
                found = Some(rid);
            }
        }
        let mut export = None;
        if parent.is_none() {
            for (rid, row) in image.md.tables().iter::<cildec::tables::ExportedTypeRow>() {
                let row = row?;
                if image.md.strings().str(row.type_name)? == name
                    && image.md.strings().str(row.type_namespace)? == namespace
                {
                    if export.is_some() || found.is_some() {
                        return Err("ambiguous type definition or forwarding records".into());
                    }
                    if rid > 0x00ffffff {
                        return Err("forwarder RID exceeds token width".into());
                    }
                    export = Some((rid, row));
                }
            }
        }
        if let Some(rid) = found {
            return Ok((module, self.describe_type(module, rid)?));
        }
        let (rid, row) = export.ok_or("type definition missing from resolved assembly")?;
        let key = (module, 0x27000000 | rid);
        if visiting.len() >= 32 || !visiting.insert(key) {
            return Err("cyclic or excessive type forwarder chain".into());
        }
        let result = (|| {
            let implementation = image.metadata_coded_token(
                CodedIndex::Implementation,
                image
                    .md
                    .tables()
                    .raw_column(TableId::ExportedType, rid, 4)?,
            )?;
            if row.flags & 0x00200000 == 0 || implementation.table() != Some(TableId::AssemblyRef) {
                return Err("exported type needs an assembly-scoped forwarder; multi-module exports remain unresolved".into());
            }
            let target = self.binding_assembly(module, implementation)?;
            let (definition, mut bound) =
                self.lookup_named_type(target, namespace, name, None, visiting)?;
            bound.exact_identity &= reference_identity(&image, implementation.rid())?
                .matches(&self.identity_source(target)?.0);
            let (assembly, source) = self.identity_source(module)?;
            bound.forwarders.insert(
                0,
                DefinitionLocation {
                    assembly,
                    source,
                    token: key.1,
                    kind: self.kinds[&module],
                },
            );
            Ok((definition, bound))
        })();
        visiting.remove(&key);
        result
    }

    fn describe_type(&self, module: usize, rid: u32) -> Result<ResolvedType> {
        let image = self.binding_image(module)?;
        let flags = image.md.tables().type_def(rid)?.flags;
        let parents = nested_parents(&image)?;
        let mut names = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = rid;
        let namespace = loop {
            if !seen.insert(current) || seen.len() > 32 {
                return Err("cyclic or excessive nested type definition".into());
            }
            let row = image.md.tables().type_def(current)?;
            names.push(image.md.strings().str(row.type_name)?.to_string());
            if let Some(&parent) = parents.get(&current) {
                current = parent;
            } else {
                break image.md.strings().str(row.type_namespace)?;
            }
        };
        names.reverse();
        let name = names.join("/");
        let (assembly, source) = self.identity_source(module)?;
        Ok(ResolvedType {
            definition: DefinitionLocation {
                assembly,
                source,
                token: 0x02000000 | rid,
                kind: self.kinds[&module],
            },
            name: if namespace.is_empty() {
                name
            } else {
                format!("{namespace}.{name}")
            },
            flags,
            exact_identity: true,
            generic_arguments: Vec::new(),
            forwarders: Vec::new(),
        })
    }
}

fn nested_parents(image: &Image<'_>) -> Result<BTreeMap<u32, u32>> {
    let count = image.md.tables().row_count(TableId::TypeDef);
    if count > 0x00ffffff {
        return Err("type definition RID exceeds token width".into());
    }
    let mut parents = BTreeMap::new();
    for (_, row) in image.md.tables().iter::<cildec::tables::NestedClassRow>() {
        let row = row?;
        let (child, parent) = (row.nested_class.get(), row.enclosing_class.get());
        if child == 0
            || child > count
            || parent == 0
            || parent > count
            || parents.insert(child, parent).is_some()
        {
            return Err("invalid or duplicate nested type owner".into());
        }
    }
    Ok(parents)
}
