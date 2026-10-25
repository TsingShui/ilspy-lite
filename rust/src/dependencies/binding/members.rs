// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::{DefinitionLocation, DependencyContext, ResolvedType};
use crate::Result;
use cildec::{CodedIndex, MethodSig, Param, TableId, Token, Type};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedMethod {
    pub(crate) module: usize,
    pub definition: DefinitionLocation,
    pub owner: ResolvedType,
    pub name: String,
    pub flags: u16,
    pub reference_signature: Vec<u8>,
    pub definition_signature: Vec<u8>,
    /// Fully scoped signature identity; this is metadata notation, not C#.
    pub signature_identity: String,
    pub method_arguments: Vec<String>,
    pub instantiation_signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedField {
    pub definition: DefinitionLocation,
    pub owner: ResolvedType,
    pub name: String,
    pub flags: u16,
    pub reference_signature: Vec<u8>,
    pub definition_signature: Vec<u8>,
    pub signature_identity: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyAccessor {
    pub definition: DefinitionLocation,
    pub owner: ResolvedType,
    pub name: String,
    pub role: String,
    pub signature: Vec<u8>,
    pub method: ResolvedMethod,
}

impl DependencyContext {
    pub fn resolve_field(&self, token: u32) -> Result<ResolvedField> {
        let image = self.binding_image(0)?;
        let token = Token(token);
        let (module, owner, name, raw) = match token.table() {
            Some(TableId::Field) => {
                let row = image.md.tables().field(token.rid())?;
                let owner = image
                    .md
                    .tables()
                    .type_of_field(cildec::Rid::new(token.rid()))?
                    .ok_or("field has no owner")?;
                (
                    0,
                    self.describe_type(0, owner.get())?,
                    image.md.strings().str(row.name)?,
                    image.md.blobs().get(row.signature)?,
                )
            }
            Some(TableId::MemberRef) => {
                let row = image.md.tables().member_ref(token.rid())?;
                let parent = image.metadata_coded_token(
                    CodedIndex::MemberRefParent,
                    image
                        .md
                        .tables()
                        .raw_column(TableId::MemberRef, token.rid(), 0)?,
                )?;
                let (module, owner) = self.bind_type_in(0, parent, &mut BTreeSet::new())?;
                (
                    module,
                    owner,
                    image.md.strings().str(row.name)?,
                    image.md.blobs().get(row.signature)?,
                )
            }
            _ => return Err("field binding needs a Field or MemberRef".into()),
        };
        let signature = crate::signature_guard::field(raw)?;
        let signature_key =
            |module, signature: &cildec::FieldSig<'_>, args: &[String]| -> Result<String> {
                Ok(format!(
                    "{:?}:{:?}",
                    self.modifier_keys(module, &signature.custom_mods, args, 0)?,
                    self.type_key(module, &signature.type_, args, 0)?
                ))
            };
        let requested = signature_key(0, &signature, &[])?;
        let instantiated = signature_key(0, &signature, &owner.generic_arguments)?;
        let target = self.binding_image(module)?;
        let selected = target.selected_types(&owner.name)?;
        let root = selected.first().ok_or("field owner selection is empty")?;
        if root.token != owner.definition.token {
            return Err("field owner identity changed".into());
        }
        let mut found = None;
        for &candidate in &root.fields {
            let row = target.md.tables().field(candidate & 0xffffff)?;
            if target.md.strings().str(row.name)? != name {
                continue;
            }
            let bytes = target.md.blobs().get(row.signature)?;
            if signature_key(module, &crate::signature_guard::field(bytes)?, &[])? != requested {
                continue;
            }
            if found.is_some() {
                return Err("ambiguous field definition in resolved type".into());
            }
            found = Some(ResolvedField {
                definition: DefinitionLocation {
                    token: candidate,
                    ..owner.definition.clone()
                },
                owner: owner.clone(),
                name: name.to_string(),
                flags: row.flags,
                reference_signature: raw.to_vec(),
                definition_signature: bytes.to_vec(),
                signature_identity: instantiated.clone(),
            });
        }
        found.ok_or_else(|| "field definition missing for exact name and scoped signature".into())
    }

    pub fn resolve_property_accessor(&self, token: u32) -> Result<Option<PropertyAccessor>> {
        let method = self.resolve_method(token)?;
        let module = method.module;
        let image = self.binding_image(module)?;
        let types = image.selected_types(&method.owner.name)?;
        let owner = types.first().ok_or("property owner selection is empty")?;
        if owner.token != method.owner.definition.token {
            return Err("property owner identity changed".into());
        }
        let signature = crate::signature_guard::method(&method.definition_signature)?;
        let mut found = None;
        for (rid, row) in image
            .md
            .tables()
            .iter::<cildec::tables::MethodSemanticsRow>()
        {
            let row = row?;
            if row.method.get() != method.definition.token & 0xffffff {
                continue;
            }
            let association = image.metadata_coded_token(
                CodedIndex::HasSemantics,
                image
                    .md
                    .tables()
                    .raw_column(TableId::MethodSemantics, rid, 2)?,
            )?;
            if association.table() != Some(TableId::Property) {
                continue;
            }
            if !owner.properties.contains(&association.0) {
                return Err("property accessor has a different declaring owner".into());
            }
            let role = match row.semantics {
                1 => "set",
                2 => "get",
                _ => return Err("unsupported or combined property accessor semantics".into()),
            };
            let property = image.md.tables().property(association.rid())?;
            let bytes = image.md.blobs().get(property.type_signature)?;
            let property_sig = crate::signature_guard::property(bytes)?;
            if property_sig.has_this != signature.has_this
                || signature.explicit_this
                || signature.calling_convention != cildec::CallConv::Default
                || !signature.vararg_params.is_empty()
                || method.flags & 0x0800 == 0
            {
                return Err("property/accessor flags or calling convention conflict".into());
            }
            // Validate declared positions before instantiation can conflate a
            // property VAR with a distinct concrete accessor type.
            let args = &[];
            let params = property_sig
                .params
                .iter()
                .map(|p| self.parameter_key(module, p, args, 0))
                .collect::<Result<Vec<_>>>()?;
            let mut actual = signature
                .params
                .iter()
                .map(|p| self.parameter_key(module, p, args, 0))
                .collect::<Result<Vec<_>>>()?;
            let value = self.parameter_key(module, &property_sig.return_type, args, 0)?;
            let returned = self.parameter_key(module, &signature.return_type, args, 0)?;
            let valid = if role == "get" {
                actual == params && returned == value
            } else {
                actual.pop() == Some(value)
                    && actual == params
                    && signature.return_type.type_ == Type::Void
                    && !signature.return_type.by_ref
                    && signature.return_type.custom_mods.is_empty()
            };
            if !valid {
                return Err("property signature conflicts with accessor signature".into());
            }
            if found.is_some() {
                return Err("ambiguous property accessor relationship".into());
            }
            found = Some(PropertyAccessor {
                definition: DefinitionLocation {
                    token: association.0,
                    ..method.owner.definition.clone()
                },
                owner: method.owner.clone(),
                name: image.md.strings().str(property.name)?.to_string(),
                role: role.to_string(),
                signature: bytes.to_vec(),
                method: method.clone(),
            });
        }
        Ok(found)
    }

    pub fn resolve_method(&self, token: u32) -> Result<ResolvedMethod> {
        let token = Token(token);
        if token.table() == Some(TableId::MethodSpec) {
            let image = self.binding_image(0)?;
            let row = image.md.tables().method_spec(token.rid())?;
            let target = image.metadata_coded_token(
                CodedIndex::MethodDefOrRef,
                image
                    .md
                    .tables()
                    .raw_column(TableId::MethodSpec, token.rid(), 0)?,
            )?;
            let raw = image.md.blobs().get(row.instantiation)?;
            let instantiation = crate::signature_guard::method_spec(raw)?;
            let mut method = self.resolve_method(target.0)?;
            let definition = crate::signature_guard::method(&method.definition_signature)?;
            let cildec::CallConv::Generic(count) = definition.calling_convention else {
                return Err("method instantiation target is not generic".into());
            };
            if instantiation.args.len() != count as usize {
                return Err("generic method argument count differs from signature".into());
            }
            method.method_arguments = instantiation
                .args
                .iter()
                .map(|a| self.type_key(0, a, &[], 0))
                .collect::<Result<Vec<_>>>()?;
            method.instantiation_signature = raw.to_vec();
            return Ok(method);
        }
        let image = self.binding_image(0)?;
        let (module, owner, name, raw) = match token.table() {
            Some(TableId::MethodDef) => {
                let row = image.md.tables().method_def(token.rid())?;
                let ty = image
                    .md
                    .tables()
                    .type_of_method(cildec::Rid::new(token.rid()))?
                    .ok_or("method definition has no owner")?;
                (
                    0,
                    self.describe_type(0, ty.get())?,
                    image.md.strings().str(row.name)?,
                    image.md.blobs().get(row.signature)?,
                )
            }
            Some(TableId::MemberRef) => {
                let row = image.md.tables().member_ref(token.rid())?;
                let parent = image.metadata_coded_token(
                    CodedIndex::MemberRefParent,
                    image
                        .md
                        .tables()
                        .raw_column(TableId::MemberRef, token.rid(), 0)?,
                )?;
                let (module, owner) = self.bind_type_in(0, parent, &mut BTreeSet::new())?;
                (
                    module,
                    owner,
                    image.md.strings().str(row.name)?,
                    image.md.blobs().get(row.signature)?,
                )
            }
            _ => return Err("method binding needs a MethodDef or MemberRef".into()),
        };
        let signature = crate::signature_guard::method(raw)
            .map_err(|e| format!("method reference signature {raw:02X?}: {e}"))?;
        // Selection uses the declared signature. Substituting first would merge
        // Echo(!0) with Echo(int32) when the owner's argument is int32.
        let requested = self.method_signature_key(0, &signature, &[], 0)?;
        let instantiated = self.method_signature_key(0, &signature, &owner.generic_arguments, 0)?;
        let target = self.binding_image(module)?;
        let selected = target.selected_types(&owner.name)?;
        let root = selected.first().ok_or("method owner selection is empty")?;
        if root.token != owner.definition.token {
            return Err("method owner identity changed".into());
        }
        let mut found = None;
        for &candidate in &root.methods {
            let row = target.md.tables().method_def(candidate & 0xffffff)?;
            if target.md.strings().str(row.name)? != name {
                continue;
            }
            let bytes = target.md.blobs().get(row.signature)?;
            let definition = crate::signature_guard::method(bytes)
                .map_err(|e| format!("method definition signature {bytes:02X?}: {e}"))?;
            if self.method_signature_key(module, &definition, &[], 0)? != requested {
                continue;
            }
            if row.is_static() == definition.has_this {
                return Err("method static flag conflicts with signature".into());
            }
            if found.is_some() {
                return Err("ambiguous method definition in resolved type".into());
            }
            found = Some(ResolvedMethod {
                module,
                definition: DefinitionLocation {
                    token: candidate,
                    ..owner.definition.clone()
                },
                owner: owner.clone(),
                name: name.to_string(),
                flags: row.flags,
                reference_signature: raw.to_vec(),
                definition_signature: bytes.to_vec(),
                signature_identity: instantiated.clone(),
                method_arguments: Vec::new(),
                instantiation_signature: Vec::new(),
            });
        }
        found.ok_or_else(|| "method definition missing for exact name and scoped signature".into())
    }

    fn method_signature_key(
        &self,
        module: usize,
        sig: &MethodSig<'_>,
        args: &[String],
        depth: usize,
    ) -> Result<String> {
        if depth > 32 {
            return Err("excessive method signature binding depth".into());
        }
        let parameter = self.parameter_key(module, &sig.return_type, args, depth + 1)?;
        let parameters = sig
            .params
            .iter()
            .map(|p| self.parameter_key(module, p, args, depth + 1))
            .collect::<Result<Vec<_>>>()?;
        let varargs = sig
            .vararg_params
            .iter()
            .map(|p| self.parameter_key(module, p, args, depth + 1))
            .collect::<Result<Vec<_>>>()?;
        Ok(format!(
            "{:?}:{}:{}:{parameter:?}:{parameters:?}:{varargs:?}",
            sig.calling_convention, sig.has_this, sig.explicit_this
        ))
    }

    fn parameter_key(
        &self,
        module: usize,
        param: &Param<'_>,
        args: &[String],
        depth: usize,
    ) -> Result<String> {
        Ok(format!(
            "{}:{:?}:{:?}",
            param.by_ref,
            self.modifier_keys(module, &param.custom_mods, args, depth)?,
            self.type_key(module, &param.type_, args, depth)?
        ))
    }

    fn modifier_keys(
        &self,
        module: usize,
        mods: &[cildec::CustomMod],
        args: &[String],
        depth: usize,
    ) -> Result<Vec<(bool, String)>> {
        mods.iter()
            .map(|m| {
                Ok((
                    m.required,
                    self.type_key(module, &Type::Class(m.type_token), args, depth + 1)?,
                ))
            })
            .collect()
    }

    pub(super) fn type_key(
        &self,
        module: usize,
        ty: &Type<'_>,
        arguments: &[String],
        depth: usize,
    ) -> Result<String> {
        if depth > 32 {
            return Err("excessive type signature binding depth".into());
        }
        Ok(match ty {
            Type::Class(token) | Type::ValueType(token) => {
                if token.table() == Some(TableId::TypeSpec) {
                    let image = self.binding_image(module)?;
                    let row = image.md.tables().type_spec(token.rid())?;
                    let sig =
                        crate::signature_guard::type_spec(image.md.blobs().get(row.signature)?)?;
                    return self.type_key(module, &sig.type_, arguments, depth + 1);
                }
                let (_, bound) = self.bind_type_in(module, *token, &mut BTreeSet::new())?;
                format!(
                    "{:?}:{:?}:0x{:08X}",
                    matches!(ty, Type::ValueType(_)),
                    bound.definition.assembly,
                    bound.definition.token
                )
            }
            Type::GenericInst {
                def,
                args,
                is_value_type,
            } => {
                if !matches!(def.table(), Some(TableId::TypeDef | TableId::TypeRef)) {
                    return Err("generic signature needs a TypeDef or TypeRef definition".into());
                }
                let (_, bound) = self.bind_type_in(module, *def, &mut BTreeSet::new())?;
                let args = args
                    .iter()
                    .map(|a| self.type_key(module, a, arguments, depth + 1))
                    .collect::<Result<Vec<_>>>()?;
                format!(
                    "{is_value_type}:{:?}:0x{:08X}:{args:?}",
                    bound.definition.assembly, bound.definition.token
                )
            }
            Type::Ptr(mods, inner) | Type::SzArray(mods, inner) | Type::Modified(mods, inner) => {
                let kind = match ty {
                    Type::Ptr(..) => "ptr",
                    Type::SzArray(..) => "szarray",
                    _ => "modified",
                };
                format!(
                    "{kind}:{:?}:{:?}",
                    self.modifier_keys(module, mods, arguments, depth + 1)?,
                    self.type_key(module, inner, arguments, depth + 1)?
                )
            }
            Type::Array(inner, shape) => format!(
                "array:{shape:?}:{:?}",
                self.type_key(module, inner, arguments, depth + 1)?
            ),
            Type::ByRef(inner) => format!(
                "byref:{:?}",
                self.type_key(module, inner, arguments, depth + 1)?
            ),
            Type::Pinned(inner) => format!(
                "pinned:{:?}",
                self.type_key(module, inner, arguments, depth + 1)?
            ),
            Type::FnPtr(sig) => format!(
                "fnptr:{:?}",
                self.method_signature_key(module, sig, arguments, depth + 1)?
            ),
            Type::Var(n) => {
                if arguments.is_empty() {
                    format!("var:{n}")
                } else {
                    arguments
                        .get(*n as usize)
                        .ok_or("generic type parameter position out of range")?
                        .clone()
                }
            }
            Type::MVar(n) => format!("mvar:{n}"),
            Type::Void
            | Type::Boolean
            | Type::Char
            | Type::I1
            | Type::U1
            | Type::I2
            | Type::U2
            | Type::I4
            | Type::U4
            | Type::I8
            | Type::U8
            | Type::R4
            | Type::R8
            | Type::String
            | Type::Object
            | Type::TypedByRef
            | Type::IntPtr
            | Type::UIntPtr => format!("{ty:?}"),
            _ => return Err("unsupported type signature binding form".into()),
        })
    }
}
