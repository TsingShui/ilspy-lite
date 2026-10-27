// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Image-scoped structural identities with owner-qualified generic variables. No layout, member
//! substitution, variance, allocation, or concrete value layouts are inferred.

use crate::{Result, metadata::Image};
use cildec::{Names, TableId, Token, Type};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeId {
    arena: u64,
    index: u32,
}

/// Generic variables are meaningful only in the declaration that owns them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GenericScope {
    pub owner: u32,
    pub count: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TypeContext {
    pub type_scope: GenericScope,
    pub method_scope: GenericScope,
}

impl TypeContext {
    pub fn evidence(self) -> String {
        if self.type_scope.count == 0 && self.method_scope.count == 0 {
            return String::new();
        }
        format!(
            "// generic body context type-owner=0x{:08X} type-arity={} method-owner=0x{:08X} method-arity={}; variables use owner/position, not names; original generic constraints and runtime entry binding apply.\n",
            self.type_scope.owner,
            self.type_scope.count,
            self.method_scope.owner,
            self.method_scope.count
        )
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SignatureKey {
    bytes: Vec<u8>,
    type_scope: GenericScope,
    method_scope: GenericScope,
}

pub(crate) struct SignatureTypes {
    arena: u64,
    entries: Vec<Entry>,
    identities: BTreeMap<SignatureKey, TypeId>,
    aliases: BTreeMap<String, TypeId>,
}

struct Entry {
    alias: String,
    signature: SignatureKey,
    description: String,
}

impl SignatureTypes {
    pub fn new() -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let arena = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| "signature arena identities exhausted")?;
        Ok(Self {
            arena,
            entries: Vec::new(),
            identities: BTreeMap::new(),
            aliases: BTreeMap::new(),
        })
    }

    pub fn intern(&mut self, signature: SignatureKey, description: String) -> Result<TypeId> {
        if let Some(&id) = self.identities.get(&signature) {
            return Ok(id);
        }
        if self.entries.len() >= 16384 {
            return Err("structural signature identity limit exceeded".into());
        }
        // Hashes name aliases only. Equality uses the complete canonical key,
        // and distinct keys with the same alias are rejected, never merged.
        let mut hash = Sha1::new();
        hash.update(&signature.bytes);
        if signature.type_scope.count != 0 || signature.method_scope.count != 0 {
            for scope in [signature.type_scope, signature.method_scope] {
                hash.update(scope.owner.to_le_bytes());
                hash.update(scope.count.to_le_bytes());
            }
        }
        let category = match signature.bytes.first() {
            Some(0x13 | 0x1e) => "generic",
            Some(0x11) => "value",
            Some(0x15) if signature.bytes.get(1) == Some(&0x11) => "value",
            _ => "ref",
        };
        let alias = format!("__cil_{category}_{:x}", hash.finalize());
        if self.aliases.contains_key(&alias) {
            return Err("structural signature alias collision".into());
        }
        let id = TypeId {
            arena: self.arena,
            index: self.entries.len() as u32,
        };
        self.aliases.insert(alias.clone(), id);
        self.identities.insert(signature.clone(), id);
        self.entries.push(Entry {
            alias,
            signature,
            description,
        });
        Ok(id)
    }

    fn entry(&self, id: TypeId) -> Result<&Entry> {
        if id.arena != self.arena {
            return Err("structural signature belongs to a different input image".into());
        }
        self.entries
            .get(id.index as usize)
            .ok_or_else(|| "unknown structural signature identity".into())
    }

    pub fn signature(&self, id: TypeId) -> Result<(Vec<u8>, TypeContext)> {
        let key = &self.entry(id)?.signature;
        Ok((
            key.bytes.clone(),
            TypeContext {
                type_scope: key.type_scope,
                method_scope: key.method_scope,
            },
        ))
    }

    pub fn is_vector(&self, id: TypeId) -> Result<bool> {
        Ok(self.entry(id)?.signature.bytes.first() == Some(&0x1d))
    }

    pub fn name(&self, id: TypeId) -> Result<String> {
        Ok(self.entry(id)?.alias.clone())
    }

    pub fn evidence(&self, id: TypeId) -> Result<String> {
        let entry = self.entry(id)?;
        let bytes = entry
            .signature
            .bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<String>();
        let semantics = match entry.signature.bytes.first() {
            Some(0x13 | 0x1e) => {
                "generic value copy in the original target type; no boxing, reference/integer category or layout inference"
            }
            Some(0x14) => {
                "general array reference identity (element and rank); no element, layout or bounds inference"
            }
            Some(0x11) => {
                "whole struct value snapshot/copy, preserving embedded references; no boxing, construction, host layout or address inference"
            }
            // 0x15 0x11 is a constructed value type: the CLI copies it whole, and
            // the instantiation's arguments are part of its identity.
            Some(0x15) if entry.signature.bytes.get(1) == Some(&0x11) => {
                "whole constructed value snapshot/copy in the original instantiation; no boxing, construction, host layout or address inference"
            }
            _ => "exact reference identity, no member/layout inference",
        };
        Ok(format!(
            "// structural signature {} bytes={bytes}: {}; type-owner=0x{:08X} method-owner=0x{:08X}; tokens scoped to this input image; {semantics}.\n",
            entry.alias,
            crate::metadata::escape_line(&entry.description),
            entry.signature.type_scope.owner,
            entry.signature.method_scope.owner
        ))
    }
}

/// Encode a structural type and expose its normalized bytes with the generic
/// scopes it depends on, for identity comparisons between metadata operands.
pub(crate) fn encoded_type(
    image: &Image<'_>,
    ty: &Type<'_>,
    context: TypeContext,
) -> Result<(Vec<u8>, GenericScope, GenericScope)> {
    let (key, _) = encode_type(image, ty, context)?;
    Ok((key.bytes, key.type_scope, key.method_scope))
}

pub(crate) fn encode_type(
    image: &Image<'_>,
    ty: &Type<'_>,
    context: TypeContext,
) -> Result<(SignatureKey, String)> {
    if !matches!(
        ty,
        Type::GenericInst { .. }
            | Type::Array(..)
            | Type::SzArray(..)
            | Type::Var(_)
            | Type::MVar(_)
            | Type::ValueType(_)
    ) {
        return Err("unsupported structural stack type shape".into());
    }
    let mut encoder = Encoder {
        image,
        bytes: Vec::new(),
        remaining: 16384,
        context,
        type_scope: GenericScope::default(),
        method_scope: GenericScope::default(),
    };
    encoder.ty(ty, 0)?;
    Ok((
        SignatureKey {
            bytes: encoder.bytes,
            type_scope: encoder.type_scope,
            method_scope: encoder.method_scope,
        },
        Names::new(&image.md).type_(ty),
    ))
}

struct Encoder<'i, 'a> {
    image: &'i Image<'a>,
    bytes: Vec<u8>,
    remaining: usize,
    context: TypeContext,
    type_scope: GenericScope,
    method_scope: GenericScope,
}

impl Encoder<'_, '_> {
    fn unsigned(&mut self, value: u32) -> Result<()> {
        if value < 0x80 {
            self.bytes.push(value as u8);
        } else if value < 0x4000 {
            self.bytes
                .extend_from_slice(&((value as u16) | 0x8000).to_be_bytes());
        } else if value <= 0x1fff_ffff {
            self.bytes
                .extend_from_slice(&(value | 0xc000_0000).to_be_bytes());
        } else {
            return Err("signature integer exceeds compressed width".into());
        }
        Ok(())
    }

    fn nominal(&mut self, token: Token, value: bool, arity: usize) -> Result<()> {
        let token = self.image.signature_type_token(token, value, arity)?;
        let tag = match token.table() {
            Some(TableId::TypeDef) => 0,
            Some(TableId::TypeRef) => 1,
            _ => return Err("structural nominal type requires TypeDef or TypeRef".into()),
        };
        if token.rid() == 0 || token.rid() > Token::RID_MASK {
            return Err("invalid structural type token".into());
        }
        self.unsigned((token.rid() << 2) | tag)
    }

    fn ty(&mut self, ty: &Type<'_>, depth: usize) -> Result<()> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or("structural signature work limit exceeded")?;
        if depth >= 32 || self.bytes.len() >= 1 << 20 {
            return Err("structural signature depth/byte limit exceeded".into());
        }
        let primitive = match ty {
            Type::Boolean => 0x02,
            Type::Char => 0x03,
            Type::I1 => 0x04,
            Type::U1 => 0x05,
            Type::I2 => 0x06,
            Type::U2 => 0x07,
            Type::I4 => 0x08,
            Type::U4 => 0x09,
            Type::I8 => 0x0a,
            Type::U8 => 0x0b,
            Type::R4 => 0x0c,
            Type::R8 => 0x0d,
            Type::String => 0x0e,
            Type::IntPtr => 0x18,
            Type::UIntPtr => 0x19,
            Type::Object => 0x1c,
            _ => 0,
        };
        if primitive != 0 {
            self.bytes.push(primitive);
            return Ok(());
        }
        match ty {
            Type::Var(position) | Type::MVar(position) => {
                let method = matches!(ty, Type::MVar(_));
                let scope = if method {
                    self.context.method_scope
                } else {
                    self.context.type_scope
                };
                if *position >= scope.count || scope.owner == 0 {
                    return Err(
                        "generic variable has no owner or exceeds declared positions".into(),
                    );
                }
                if method {
                    self.method_scope = scope;
                } else {
                    self.type_scope = scope;
                }
                self.bytes.push(if method { 0x1e } else { 0x13 });
                self.unsigned(*position)?;
            }
            Type::Class(token) | Type::ValueType(token) => {
                let value = matches!(ty, Type::ValueType(_));
                self.bytes.push(if value { 0x11 } else { 0x12 });
                self.nominal(*token, value, 0)?;
            }
            Type::GenericInst {
                is_value_type,
                def,
                args,
            } if !args.is_empty() => {
                self.bytes
                    .extend_from_slice(&[0x15, if *is_value_type { 0x11 } else { 0x12 }]);
                self.nominal(*def, *is_value_type, args.len())?;
                self.unsigned(u32::try_from(args.len())?)?;
                for arg in args {
                    self.ty(arg, depth + 1)?;
                }
            }
            Type::SzArray(modifiers, inner) if modifiers.is_empty() => {
                self.bytes.push(0x1d);
                self.ty(inner, depth + 1)?;
            }
            // ELEMENT_TYPE_ARRAY with no declared sizes and zero-based dimensions:
            // C# spells exactly these shapes as `T[,]`.
            Type::Array(element, shape)
                if shape.sizes.is_empty() && shape.lo_bounds.iter().all(|bound| *bound == 0) =>
            {
                self.bytes.push(0x14);
                self.ty(element, depth + 1)?;
                self.unsigned(shape.rank)?;
                self.unsigned(0)?;
                self.unsigned(u32::try_from(shape.lo_bounds.len())?)?;
                for _ in &shape.lo_bounds {
                    self.unsigned(0)?;
                }
            }
            _ => {
                return Err(
                    "open/modified/unsupported structural signature needs IL fallback".into(),
                );
            }
        }
        Ok(())
    }
}
