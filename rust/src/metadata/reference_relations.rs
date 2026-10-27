// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Instantiated reference relationships. A proof changes no runtime operation.

use super::Image;
use crate::{
    Result,
    ir::{StackKind, Ty},
    signature_types::{GenericScope, TypeContext},
};
use cildec::{TableId, Token, Type};

/// A selected definition, including its image, assembly identity and full name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DefinitionId {
    pub module: usize,
    pub token: u32,
    pub assembly: crate::dependencies::AssemblyIdentity,
    pub name: String,
    pub arity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Term {
    Primitive(u8),
    Parameter(u32),
    Named {
        token: Token,
        args: Vec<Term>,
        value: bool,
    },
    ForeignNamed {
        definition: DefinitionId,
        args: Vec<Term>,
        value: bool,
    },
    Variable {
        scope: GenericScope,
        position: u32,
        method: bool,
    },
    Vector(Box<Term>),
}

#[derive(Clone, Default)]
pub(crate) struct Facts {
    pub aliases: std::collections::BTreeMap<u32, u32>,
    pub identities: std::collections::BTreeMap<u32, DefinitionId>,
    pub definitions: std::collections::BTreeMap<u32, Definition>,
    pub foreign_definitions: std::collections::BTreeMap<DefinitionId, Definition>,
}

#[derive(Clone)]
pub(crate) struct Definition {
    pub interface: bool,
    pub exception: bool,
    pub variance: Vec<u16>,
    pub parents: Vec<Term>,
}

pub(super) fn assignable(image: &Image<'_>, actual: Ty, expected: Ty) -> Result<bool> {
    if actual.kind() != StackKind::Ref || expected.kind() != StackKind::Ref {
        return Ok(false);
    }
    let mut proof = Proof {
        image,
        remaining: 2048,
    };
    let actual = proof.stack_type(actual)?;
    let expected = proof.stack_type(expected)?;
    proof.prove(&actual, &expected, 0)
}

fn instantiate(term: &Term, args: &[Term], depth: usize) -> Result<Term> {
    if depth >= 32 {
        return Err("reference template depth exceeded".into());
    }
    Ok(match term {
        Term::Parameter(position) => args
            .get(*position as usize)
            .cloned()
            .ok_or("reference template argument missing")?,
        Term::Named {
            token,
            args: nested,
            value,
        } => Term::Named {
            token: *token,
            args: nested
                .iter()
                .map(|a| instantiate(a, args, depth + 1))
                .collect::<Result<Vec<_>>>()?,
            value: *value,
        },
        Term::ForeignNamed {
            definition,
            args: nested,
            value,
        } => Term::ForeignNamed {
            definition: definition.clone(),
            args: nested
                .iter()
                .map(|a| instantiate(a, args, depth + 1))
                .collect::<Result<Vec<_>>>()?,
            value: *value,
        },
        Term::Vector(element) => Term::Vector(Box::new(instantiate(element, args, depth + 1)?)),
        _ => term.clone(),
    })
}

#[derive(Clone, Copy)]
enum NominalRef<'a> {
    Caller(Token),
    Foreign(&'a DefinitionId),
}

fn named(term: &Term) -> Option<(NominalRef<'_>, &[Term], bool)> {
    match term {
        Term::Named { token, args, value } => Some((NominalRef::Caller(*token), args, *value)),
        Term::ForeignNamed {
            definition,
            args,
            value,
        } => Some((NominalRef::Foreign(definition), args, *value)),
        _ => None,
    }
}

struct Proof<'i, 'a> {
    image: &'i Image<'a>,
    remaining: usize,
}

impl Proof<'_, '_> {
    fn step(&mut self, depth: usize) -> Result<()> {
        if depth >= 48 || self.remaining == 0 {
            return Err("reference relationship depth/work limit exceeded".into());
        }
        self.remaining -= 1;
        Ok(())
    }

    fn nominal(&self, token: Token) -> Token {
        let token = Token(
            self.image
                .reference_facts
                .borrow()
                .aliases
                .get(&token.0)
                .copied()
                .unwrap_or(token.0),
        );
        if token.table() == Some(TableId::TypeRef)
            && !self.image.reference_attempted.borrow().contains(&token.0)
        {
            self.image.reference_queries.borrow_mut().insert(token.0);
        }
        token
    }

    fn stack_type(&mut self, ty: Ty) -> Result<Term> {
        match ty {
            Ty::Object => Ok(Term::Primitive(0x1c)),
            Ty::String => Ok(Term::Primitive(0x0e)),
            Ty::I4Array => Ok(Term::Vector(Box::new(Term::Primitive(8)))),
            Ty::U4Array => Ok(Term::Vector(Box::new(Term::Primitive(9)))),
            Ty::StructuralRef(id) | Ty::GenericValue(id) | Ty::Value(id) => {
                let (bytes, context) = self.image.signature_types.borrow().signature(id)?;
                let sig = crate::signature_guard::type_spec(&bytes)?;
                self.term(&sig.type_, context, None, 0)
            }
            Ty::Class(token) => {
                let owner = Token(token);
                let params = self.image.md.tables().generic_params(owner)?;
                let scope = GenericScope {
                    owner: token,
                    count: params.len() as u32,
                };
                let args = (0..scope.count)
                    .map(|position| Term::Variable {
                        scope,
                        position,
                        method: false,
                    })
                    .collect();
                Ok(Term::Named {
                    token: owner,
                    args,
                    value: false,
                })
            }
            Ty::ExternalClass(token) => {
                self.term(&Type::Class(Token(token)), TypeContext::default(), None, 0)
            }
            _ => Err("reference relationship needs an admitted reference endpoint".into()),
        }
    }

    fn term(
        &mut self,
        ty: &Type<'_>,
        context: TypeContext,
        arguments: Option<&[Term]>,
        depth: usize,
    ) -> Result<Term> {
        self.step(depth)?;
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
        match ty {
            Type::Var(position) | Type::MVar(position) => {
                let method = matches!(ty, Type::MVar(_));
                if !method && let Some(arguments) = arguments {
                    return arguments
                        .get(*position as usize)
                        .cloned()
                        .ok_or_else(|| "reference base variable exceeds owner arity".into());
                }
                let scope = if method {
                    context.method_scope
                } else {
                    context.type_scope
                };
                if scope.owner == 0 || *position >= scope.count {
                    return Err("reference variable has no exact caller scope".into());
                }
                Ok(Term::Variable {
                    scope,
                    position: *position,
                    method,
                })
            }
            Type::Class(token) | Type::ValueType(token) => {
                let value = matches!(ty, Type::ValueType(_));
                let token = self.image.local_type_definition(*token)?.unwrap_or(*token);
                if !value {
                    let intrinsic = match token.table() {
                        Some(TableId::TypeDef) => self.image.local_core_type_name(token)?,
                        Some(TableId::TypeRef) => self.image.core_type_name(token).ok(),
                        _ => None,
                    };
                    if intrinsic == Some("Object") {
                        return Ok(Term::Primitive(0x1c));
                    }
                    if intrinsic == Some("String") {
                        return Ok(Term::Primitive(0x0e));
                    }
                }
                Ok(Term::Named {
                    token: self.nominal(token),
                    args: Vec::new(),
                    value,
                })
            }
            Type::GenericInst {
                is_value_type,
                def,
                args,
            } => {
                let token = self.image.local_type_definition(*def)?.unwrap_or(*def);
                let args = args
                    .iter()
                    .map(|arg| self.term(arg, context, arguments, depth + 1))
                    .collect::<Result<Vec<_>>>()?;
                if token.table() == Some(TableId::TypeDef)
                    && self.image.md.tables().generic_params(token)?.len() != args.len()
                {
                    return Err("reference instantiation argument count differs".into());
                }
                Ok(Term::Named {
                    token: self.nominal(token),
                    args,
                    value: *is_value_type,
                })
            }
            Type::SzArray(modifiers, element) if modifiers.is_empty() => Ok(Term::Vector(
                Box::new(self.term(element, context, arguments, depth + 1)?),
            )),
            _ => Err("unsupported reference relationship argument shape".into()),
        }
    }

    fn reference(&self, term: &Term) -> Result<bool> {
        Ok(match term {
            Term::Named { value, .. } | Term::ForeignNamed { value, .. } => !value,
            Term::Primitive(14 | 28) | Term::Vector(_) => true,
            Term::Variable {
                scope, position, ..
            } => self
                .image
                .md
                .tables()
                .generic_params(Token(scope.owner))?
                .into_iter()
                .map(|rid| self.image.md.tables().generic_param(rid))
                .collect::<std::result::Result<Vec<_>, _>>()?
                .iter()
                .any(|p| u32::from(p.number) == *position && p.flags & 4 != 0),
            _ => false,
        })
    }

    fn same_nominal(&self, a: NominalRef<'_>, b: NominalRef<'_>) -> bool {
        let facts = self.image.reference_facts.borrow();
        match (a, b) {
            (NominalRef::Caller(a), NominalRef::Caller(b)) => {
                a == b
                    || facts
                        .identities
                        .get(&a.0)
                        .is_some_and(|identity| facts.identities.get(&b.0) == Some(identity))
            }
            (NominalRef::Caller(a), NominalRef::Foreign(b))
            | (NominalRef::Foreign(b), NominalRef::Caller(a)) => {
                facts.identities.get(&a.0) == Some(b)
            }
            (NominalRef::Foreign(a), NominalRef::Foreign(b)) => a == b,
        }
    }

    fn identical(&self, actual: &Term, expected: &Term, depth: usize) -> Result<bool> {
        if depth >= 48 {
            return Err("reference identity depth exceeded".into());
        }
        if actual == expected {
            return Ok(true);
        }
        if let (Some((a, aa, av)), Some((b, ba, bv))) = (named(actual), named(expected)) {
            if av != bv || aa.len() != ba.len() || !self.same_nominal(a, b) {
                return Ok(false);
            }
            for (a, b) in aa.iter().zip(ba) {
                if !self.identical(a, b, depth + 1)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        if let (Term::Vector(a), Term::Vector(b)) = (actual, expected) {
            return self.identical(a, b, depth + 1);
        }
        Ok(false)
    }

    fn same_or_variant(&mut self, actual: &Term, expected: &Term, depth: usize) -> Result<bool> {
        if self.identical(actual, expected, depth)? {
            return Ok(true);
        }
        if *expected == Term::Primitive(0x1c) && self.reference(actual)? {
            return Ok(true);
        }
        let (Some((a, aa, false)), Some((b, ba, false))) = (named(actual), named(expected)) else {
            return Ok(false);
        };
        if !self.same_nominal(a, b) || aa.len() != ba.len() {
            return Ok(false);
        }
        let flags = match a {
            NominalRef::Caller(token) if token.table() == Some(TableId::TypeDef) => {
                if self.image.md.tables().type_def(token.rid())?.flags & 0x20 == 0 {
                    return Ok(false);
                }
                self.image
                    .md
                    .tables()
                    .generic_params(token)?
                    .into_iter()
                    .enumerate()
                    .map(|(index, rid)| {
                        let p = self.image.md.tables().generic_param(rid)?;
                        if usize::from(p.number) != index {
                            return Err("invalid reference variance parameter positions".into());
                        }
                        Ok(p.flags)
                    })
                    .collect::<Result<Vec<_>>>()?
            }
            _ => {
                let facts = self.image.reference_facts.borrow();
                let fact = match a {
                    NominalRef::Caller(token) => facts.definitions.get(&token.0),
                    NominalRef::Foreign(definition) => facts.foreign_definitions.get(definition),
                };
                let Some(fact) = fact else { return Ok(false) };
                if !fact.interface {
                    return Ok(false);
                }
                fact.variance.clone()
            }
        };
        if flags.len() != aa.len() {
            return Err("reference variance arity differs".into());
        }
        for ((a, b), flags) in aa.iter().zip(ba).zip(flags) {
            if flags & 3 == 3 {
                return Err("invalid reference variance parameter positions/flags".into());
            }
            if self.identical(a, b, depth + 1)? {
                continue;
            }
            if !self.reference(a)? || !self.reference(b)? {
                return Ok(false);
            }
            let valid = match flags & 3 {
                1 => self.prove(a, b, depth + 1)?,
                2 => self.prove(b, a, depth + 1)?,
                _ => false,
            };
            if !valid {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn prove(&mut self, actual: &Term, expected: &Term, depth: usize) -> Result<bool> {
        self.step(depth)?;
        let mut pending = vec![(actual.clone(), Vec::<u32>::new())];
        let mut found = false;
        while let Some((candidate, mut active)) = pending.pop() {
            self.step(depth)?;
            found |= self.same_or_variant(&candidate, expected, depth)?;
            let (token, args) = match candidate {
                Term::Named {
                    token,
                    args,
                    value: false,
                } => (token, args),
                Term::ForeignNamed {
                    definition,
                    args,
                    value: false,
                } => {
                    if let Some(fact) = self
                        .image
                        .reference_facts
                        .borrow()
                        .foreign_definitions
                        .get(&definition)
                        .cloned()
                    {
                        if fact.variance.len() != args.len() {
                            return Err("foreign reference arity differs".into());
                        }
                        for parent in &fact.parents {
                            let parent = instantiate(parent, &args, 0)?;
                            found |= self.prove(&parent, expected, depth + 1)?;
                        }
                    }
                    continue;
                }
                _ => continue,
            };
            if token.table() != Some(TableId::TypeDef) {
                if let Some(fact) = self
                    .image
                    .reference_facts
                    .borrow()
                    .definitions
                    .get(&token.0)
                    .cloned()
                {
                    if fact.variance.len() != args.len() {
                        return Err("foreign reference arity differs".into());
                    }
                    for parent in &fact.parents {
                        let parent = instantiate(parent, &args, 0)?;
                        found |= self.prove(&parent, expected, depth + 1)?;
                    }
                }
                continue;
            }
            if active.contains(&token.0) {
                return Err("cyclic reference relationship hierarchy".into());
            }
            if active.len() >= 64 {
                return Err("reference hierarchy depth exceeded".into());
            }
            active.push(token.0);
            self.image.md.tables().type_def(token.rid())?;
            if self.image.md.tables().generic_params(token)?.len() != args.len() {
                return Err("reference owner argument count differs".into());
            }
            let mut edges = Vec::new();
            let base = self.image.metadata_coded_token(
                cildec::CodedIndex::TypeDefOrRef,
                self.image
                    .md
                    .tables()
                    .raw_column(TableId::TypeDef, token.rid(), 3)?,
            )?;
            if !base.is_null() {
                edges.push(base);
            }
            for (rid, edge) in self
                .image
                .md
                .tables()
                .iter::<cildec::tables::InterfaceImplRow>()
            {
                let edge = edge?;
                if edge.class.get() == token.rid() {
                    edges.push(
                        self.image.metadata_coded_token(
                            cildec::CodedIndex::TypeDefOrRef,
                            self.image
                                .md
                                .tables()
                                .raw_column(TableId::InterfaceImpl, rid, 1)?,
                        )?,
                    );
                }
            }
            for edge in edges {
                let next = if edge.table() == Some(TableId::TypeSpec) {
                    let spec = self.image.type_spec(edge)?;
                    self.term(&spec.type_, TypeContext::default(), Some(&args), depth + 1)?
                } else {
                    self.term(
                        &Type::Class(edge),
                        TypeContext::default(),
                        Some(&args),
                        depth + 1,
                    )?
                };
                if !self.reference(&next)? {
                    return Err("reference hierarchy edge names a value".into());
                }
                pending.push((next, active.clone()));
            }
        }
        Ok(found)
    }
}
