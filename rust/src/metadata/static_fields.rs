// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::Image;
use crate::{Result, ir::Ty, signature_types::TypeContext};
use cildec::{TableId, Token};

pub(crate) struct StaticField {
    pub ty: Ty,
    readonly: bool,
    initializer: Option<u32>,
    token: u32,
    named: Option<String>,
}

impl StaticField {
    pub(super) fn named(token: Token, parts: (String, Ty, bool, u32)) -> Self {
        Self {
            token: token.0,
            ty: parts.1,
            readonly: parts.2,
            initializer: Some(parts.3),
            named: Some(parts.0),
        }
    }

    pub(crate) fn load(&self, image: &Image<'_>) -> Result<String> {
        let value = match &self.named {
            Some(name) => name.clone(),
            None => format!("__cil_ldsfld_{:08X}()", self.token),
        };
        self.ty.load(&value, image)
    }

    pub(crate) fn address(&self) -> String {
        match &self.named {
            Some(name) if !self.readonly => name.clone(),
            _ => format!("__cil_ldsflda_{:08X}()", self.token),
        }
    }

    pub(crate) fn writable_in(&self, initializer: Option<u32>) -> bool {
        !self.readonly || (self.initializer.is_some() && self.initializer == initializer)
    }

    pub(crate) fn store(&self, value: &str, image: &Image<'_>) -> Result<String> {
        let value = self.ty.store(value, image)?;
        Ok(match &self.named {
            Some(name) => format!("{name} = {value};"),
            None => format!("__cil_stsfld_{:08X}({value});", self.token),
        })
    }
}

fn field_type<'a>(
    image: &Image<'a>,
    bytes: &[u8],
    context: TypeContext,
    arguments: Option<&[cildec::Type<'a>]>,
) -> Result<Ty> {
    let signature = crate::signature_guard::field(bytes)?;
    if !signature.custom_mods.is_empty()
        && !image.volatile_only_modifiers(&signature.custom_mods)?
    {
        return Err("modified field needs IL fallback".into());
    }
    let type_ = match arguments {
        Some(args) => super::substitute_type_arguments(&signature.type_, args)?,
        None => signature.type_,
    };
    let ty = Ty::from_sig_scoped(&type_, image, context)?;
    ty.cs(image)?;
    Ok(ty)
}

fn foreign<'a>(
    image: &Image<'a>,
    token: Token,
    context: TypeContext,
    arguments: Option<&[cildec::Type<'a>]>,
) -> Result<StaticField> {
    let fact = match image.external_field(token.0) {
        Some(Ok(fact)) => fact,
        Some(Err(reason)) => {
            return Err(format!("external static field needs dependency facts: {reason}").into());
        }
        None => return Err("external static field needs dependency facts".into()),
    };
    if fact.flags & 0x1050 != 0x10 {
        return Err("static field needs an ordinary stored static definition".into());
    }
    let row = image.md.tables().member_ref(token.rid())?;
    let ty = field_type(
        image,
        image.md.blobs().get(row.signature)?,
        context,
        arguments,
    )?;
    Ok(StaticField {
        ty,
        readonly: fact.flags & 0x20 != 0,
        initializer: None,
        token: token.0,
        named: None,
    })
}

pub(super) fn referenced(
    image: &Image<'_>,
    token: Token,
    context: TypeContext,
) -> Result<Option<StaticField>> {
    if token.table() != Some(TableId::MemberRef) {
        return Ok(None);
    }
    let member = image.md.tables().member_ref(token.rid())?;
    if member.class.table() != Some(TableId::TypeSpec) {
        if matches!(
            member.class.table(),
            Some(TableId::TypeRef | TableId::TypeDef)
        ) && image.local_type_definition(member.class)?.is_none()
        {
            return foreign(image, token, context, None).map(Some);
        }
        return Ok(None);
    }
    let spec = image.type_spec(member.class)?;
    let cildec::Type::GenericInst {
        def,
        args,
        is_value_type,
    } = &spec.type_
    else {
        return Err("static field needs a generic declaring type".into());
    };
    let Some(definition) = image.local_type_definition(*def)? else {
        return foreign(image, token, context, Some(args)).map(Some);
    };
    let owner = image.md.tables().type_def(definition.rid())?;
    let value_owner = matches!(image.has_core_base(definition, "ValueType"), Ok(true))
        || matches!(image.has_core_base(definition, "Enum"), Ok(true));
    if owner.flags & 0x20 != 0 || value_owner != *is_value_type {
        return Err("static field declaring kind differs from its definition".into());
    }
    if args.is_empty() || image.md.tables().generic_params(definition)?.len() != args.len() {
        return Err("static field declaring arity differs from its definition".into());
    }
    image.generic_metadata(definition.0, Some(args.len() as u32))?;
    let name = image.md.strings().str(member.name)?;
    let field = image.find_field(definition, name)?;
    let row = image.md.tables().field(field.rid())?;
    if row.flags & 0x50 != 0x10 {
        return Err("static field needs a stored static definition".into());
    }
    let resolve = |signature| field_type(image, signature, context, Some(args));
    let declared = resolve(image.md.blobs().get(row.signature)?)?;
    let requested = resolve(image.md.blobs().get(member.signature)?)?;
    if declared != requested {
        return Err("static field reference signature differs from its declaration".into());
    }
    Ok(Some(StaticField {
        ty: requested,
        readonly: row.flags & 0x20 != 0,
        initializer: image
            .identity_member_owner(definition, args, context)?
            .then_some(definition.0),
        token: token.0,
        named: None,
    }))
}
