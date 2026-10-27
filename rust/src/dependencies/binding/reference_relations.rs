// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Project exact dependency relationships into the caller's token vocabulary.

use super::DependencyContext;
use crate::{
    Result,
    dependencies::ReferenceVersionPolicy,
    ir::Ty,
    metadata::reference_relations::{Definition, Facts, Term},
};
use cildec::{CodedIndex, TableId, Token, Type};
use std::collections::{BTreeMap, BTreeSet};

type Locations = BTreeMap<(usize, u32), Token>;

#[derive(Clone)]
pub(crate) struct CachedRegistry {
    stamp: (usize, super::super::ReferenceVersionPolicy, bool),
    aliases: BTreeMap<u32, u32>,
    locations: Locations,
    roots: Vec<(usize, Token, Token)>,
}

impl DependencyContext {
    fn relationship_definition_usable(&self, definition: &super::ResolvedType) -> bool {
        definition.exact_identity || self.version_policy == ReferenceVersionPolicy::IlSpy
    }

    pub(crate) fn reference_assignment_facts(&self, requested: &BTreeSet<u32>) -> Result<Facts> {
        let stamp = (
            self.candidates.len(),
            self.version_policy,
            self.discovery_incomplete,
        );
        let previous = self.reference_registry.borrow().clone();
        let registry = if let Some(previous) = previous.filter(|r| r.stamp == stamp) {
            previous
        } else {
            let built = self.build_reference_registry(stamp)?;
            *self.reference_registry.borrow_mut() = Some(built.clone());
            built
        };
        let mut facts = Facts {
            aliases: registry.aliases,
            definitions: BTreeMap::new(),
        };
        let requested: BTreeSet<_> = requested
            .iter()
            .map(|token| facts.aliases.get(token).copied().unwrap_or(*token))
            .collect();
        for (module, definition, caller) in registry.roots {
            if requested.contains(&caller.0)
                && let Ok(fact) = self.reference_definition(module, definition, &registry.locations)
            {
                facts.definitions.insert(caller.0, fact);
            }
        }
        Ok(facts)
    }

    fn build_reference_registry(
        &self,
        stamp: (usize, super::super::ReferenceVersionPolicy, bool),
    ) -> Result<CachedRegistry> {
        let main = self.binding_image(0)?;
        let mut aliases = BTreeMap::new();
        let mut locations = Locations::new();
        for (rid, row) in main.md.tables().iter::<cildec::tables::TypeDefRow>() {
            row?;
            locations.insert((0, 0x02000000 | rid), Token(0x02000000 | rid));
        }
        let mut roots = Vec::new();
        for (rid, _) in main.md.tables().iter::<cildec::tables::TypeRefRow>() {
            let token = Token(0x01000000 | rid);
            if let Ok((module, owner)) = self.bind_type_in(0, token, &mut BTreeSet::new())
                && self.relationship_definition_usable(&owner)
            {
                let representative = *locations
                    .entry((module, owner.definition.token))
                    .or_insert(token);
                aliases.insert(token.0, representative.0);
                if module != 0 && representative == token {
                    roots.push((module, Token(owner.definition.token), representative));
                }
            }
        }
        Ok(CachedRegistry {
            stamp,
            aliases,
            locations,
            roots,
        })
    }

    fn reference_definition(
        &self,
        module: usize,
        root: Token,
        locations: &Locations,
    ) -> Result<Definition> {
        let image = self.binding_image(module)?;
        let row = image.md.tables().type_def(root.rid())?;
        let variance = image
            .md
            .tables()
            .generic_params(root)?
            .into_iter()
            .enumerate()
            .map(|(index, rid)| {
                let p = image.md.tables().generic_param(rid)?;
                if usize::from(p.number) != index || p.flags & 3 == 3 {
                    return Err("invalid foreign generic parameter positions/variance".into());
                }
                Ok(p.flags)
            })
            .collect::<Result<Vec<_>>>()?;
        let arguments: Vec<Term> = (0..variance.len())
            .map(|i| Term::Parameter(i as u32))
            .collect();
        let mut pending = vec![(module, root, arguments, Vec::<(usize, u32)>::new(), true)];
        let mut parents = Vec::new();
        let mut exception = false;
        let mut work = 0;
        while let Some((module, token, arguments, mut active, class_path)) = pending.pop() {
            work += 1;
            if work > 256 || active.len() >= 32 || active.contains(&(module, token.0)) {
                return Err("cyclic/excessive dependency reference hierarchy".into());
            }
            let is_root = active.is_empty();
            active.push((module, token.0));
            let image = self.binding_image(module)?;
            let current = image.md.tables().type_def(token.rid())?;
            let class_path = class_path && current.flags & 0x20 == 0;
            if class_path && image.local_core_type_name(token)? == Some("Exception") {
                exception = true;
            }
            if image.md.tables().generic_params(token)?.len() != arguments.len() {
                return Err("foreign reference owner argument count differs".into());
            }
            if !is_root && let Some(&caller) = locations.get(&(module, token.0)) {
                parents.push(Term::Named {
                    token: caller,
                    args: arguments.clone(),
                    value: false,
                });
            }
            let base = image.metadata_coded_token(
                CodedIndex::TypeDefOrRef,
                image
                    .md
                    .tables()
                    .raw_column(TableId::TypeDef, token.rid(), 3)?,
            )?;
            let mut edges = Vec::new();
            if !base.is_null() {
                if class_path && image.core_exception_name(base.0).is_ok() {
                    exception = true;
                }
                edges.push((base, class_path));
            }
            for (rid, edge) in image.md.tables().iter::<cildec::tables::InterfaceImplRow>() {
                if edge?.class.get() == token.rid() {
                    edges.push((
                        image.metadata_coded_token(
                            CodedIndex::TypeDefOrRef,
                            image
                                .md
                                .tables()
                                .raw_column(TableId::InterfaceImpl, rid, 1)?,
                        )?,
                        false,
                    ));
                }
            }
            for (edge, edge_class_path) in edges {
                let (definition, args) = if edge.table() == Some(TableId::TypeSpec) {
                    let spec = image.type_spec(edge)?;
                    let Type::GenericInst {
                        is_value_type: false,
                        def,
                        args,
                    } = &spec.type_
                    else {
                        return Err("reference base requires a class instantiation".into());
                    };
                    let translated = args
                        .iter()
                        .map(|arg| self.reference_argument(module, arg, &arguments, locations, 0))
                        .collect::<Result<Vec<_>>>();
                    let Ok(translated) = translated else {
                        continue;
                    };
                    (*def, translated)
                } else {
                    (edge, Vec::new())
                };
                if let Ok((target, bound)) =
                    self.bind_type_in(module, definition, &mut BTreeSet::new())
                    && self.relationship_definition_usable(&bound)
                {
                    pending.push((
                        target,
                        Token(bound.definition.token),
                        args,
                        active.clone(),
                        edge_class_path,
                    ));
                }
            }
        }
        Ok(Definition {
            interface: row.flags & 0x20 != 0,
            exception,
            variance,
            parents,
        })
    }

    fn reference_argument(
        &self,
        module: usize,
        ty: &Type<'_>,
        arguments: &[Term],
        locations: &Locations,
        depth: usize,
    ) -> Result<Term> {
        if depth >= 32 {
            return Err("foreign reference argument depth exceeded".into());
        }
        let primitive = match ty {
            Type::Boolean => 2,
            Type::Char => 3,
            Type::I1 => 4,
            Type::U1 => 5,
            Type::I2 => 6,
            Type::U2 => 7,
            Type::I4 => 8,
            Type::U4 => 9,
            Type::I8 => 10,
            Type::U8 => 11,
            Type::R4 => 12,
            Type::R8 => 13,
            Type::String => 14,
            Type::IntPtr => 24,
            Type::UIntPtr => 25,
            Type::Object => 28,
            _ => 0,
        };
        if primitive != 0 {
            return Ok(Term::Primitive(primitive));
        }
        Ok(match ty {
            Type::Var(position) => arguments
                .get(*position as usize)
                .cloned()
                .ok_or("foreign base argument position exceeds owner")?,
            Type::Class(token) | Type::ValueType(token) => {
                let image = self.binding_image(module)?;
                if matches!(ty, Type::Class(_)) {
                    let nominal = if token.table() == Some(TableId::TypeDef) {
                        Ty::Class(token.0)
                    } else {
                        Ty::ExternalClass(token.0)
                    };
                    if image.intrinsic_reference_name(nominal)? == Some("Object") {
                        return Ok(Term::Primitive(28));
                    }
                    if image.intrinsic_reference_name(nominal)? == Some("String") {
                        return Ok(Term::Primitive(14));
                    }
                }
                let (target, bound) = self.bind_type_in(module, *token, &mut BTreeSet::new())?;
                if !self.relationship_definition_usable(&bound) {
                    return Err("reference argument uses a substituted definition".into());
                }
                let caller = *locations
                    .get(&(target, bound.definition.token))
                    .ok_or("foreign reference argument has no caller identity")?;
                Term::Named {
                    token: caller,
                    args: Vec::new(),
                    value: matches!(ty, Type::ValueType(_)),
                }
            }
            Type::GenericInst {
                is_value_type,
                def,
                args,
            } => {
                let (target, bound) = self.bind_type_in(module, *def, &mut BTreeSet::new())?;
                if !self.relationship_definition_usable(&bound) {
                    return Err("reference argument uses a substituted definition".into());
                }
                let caller = *locations
                    .get(&(target, bound.definition.token))
                    .ok_or("foreign constructed argument has no caller identity")?;
                Term::Named {
                    token: caller,
                    value: *is_value_type,
                    args: args
                        .iter()
                        .map(|arg| {
                            self.reference_argument(module, arg, arguments, locations, depth + 1)
                        })
                        .collect::<Result<Vec<_>>>()?,
                }
            }
            Type::SzArray(mods, element) if mods.is_empty() => Term::Vector(Box::new(
                self.reference_argument(module, element, arguments, locations, depth + 1)?,
            )),
            _ => return Err("unsupported foreign reference argument shape".into()),
        })
    }
}
