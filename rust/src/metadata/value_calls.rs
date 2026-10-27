// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::{Image, MethodRef, substitute_type_arguments};
use crate::{Result, ir::Ty, signature_types::TypeContext};
use cildec::{TableId, Token};

pub(super) fn constructor(
    image: &Image<'_>,
    token: Token,
    context: TypeContext,
) -> Result<Option<(Ty, Vec<Ty>)>> {
    if token.table() == Some(TableId::MethodDef) {
        let row = image.row(token.0)?;
        if image.md.strings().str(row.name)? != ".ctor" {
            return Ok(None);
        }
        let owner = image
            .md
            .tables()
            .type_of_method(cildec::Rid::new(token.rid()))?
            .ok_or("value constructor has no declaring type")?;
        let owner = Token(0x02000000 | owner.get());
        if image.generic_declaring_context(owner)? {
            return Ok(None);
        }
        // Constructing a core scalar (`new IntPtr(5)`) keeps the scalar stack
        // model; the emitted operation is the exact constructor call.
        if let Some(scalar) = crate::ir::core_scalar(image, owner)? {
            let signature = image.signature(token.0)?;
            let declaration = image.method_type_context(token.0, &signature)?;
            return Ok(Some((
                scalar,
                parameters(image, &signature, None, declaration)?,
            )));
        }
        let Some(id) = image.struct_type(owner)? else {
            return Ok(None);
        };
        if row.is_static() || row.flags & 0x1800 != 0x1800 || row.flags & 0x440 != 0 {
            return Err(
                "value constructor needs an ordinary instance constructor definition".into(),
            );
        }
        let signature = image.signature(token.0)?;
        let declaration = image.method_type_context(token.0, &signature)?;
        return Ok(Some((
            Ty::Value(id),
            parameters(image, &signature, None, declaration)?,
        )));
    }
    if token.table() != Some(TableId::MemberRef) {
        return Ok(None);
    }
    let member = image.md.tables().member_ref(token.rid())?;
    if image.md.strings().str(member.name)? != ".ctor"
        || member.class.table() != Some(TableId::TypeSpec)
    {
        return Ok(None);
    }
    let spec = image.type_spec(member.class)?;
    let cildec::Type::GenericInst {
        is_value_type: true,
        args,
        ..
    } = &spec.type_
    else {
        return Ok(None);
    };
    let signature = crate::signature_guard::method(image.md.blobs().get(member.signature)?)?;
    let owner = Ty::from_sig_scoped(&spec.type_, image, context)?;
    if !matches!(owner, Ty::Value(_)) {
        return Err("value constructor needs an admitted value identity".into());
    }
    Ok(Some((
        owner,
        parameters(image, &signature, Some(args), context)?,
    )))
}

fn parameters(
    image: &Image<'_>,
    signature: &cildec::MethodSig<'_>,
    arguments: Option<&[cildec::Type<'_>]>,
    context: TypeContext,
) -> Result<Vec<Ty>> {
    if !signature.has_this
        || signature.explicit_this
        || signature.calling_convention != cildec::CallConv::Default
        || !signature.vararg_params.is_empty()
        || signature.return_type.by_ref
        || signature
            .return_type
            .custom_mods
            .iter()
            .any(|modifier| modifier.required)
        || !matches!(signature.return_type.type_, cildec::Type::Void)
    {
        return Err("value constructor needs an ordinary instance void signature".into());
    }
    signature
        .params
        .iter()
        .map(|p| image.member_parameter(p, arguments, context))
        .collect()
}

/// Direct value calls retain the requested instantiation and actual this location.
pub(super) fn constructed(
    image: &Image<'_>,
    token: Token,
    context: TypeContext,
) -> Result<Option<MethodRef>> {
    if token.table() != Some(TableId::MemberRef) {
        return Ok(None);
    }
    let member = image.md.tables().member_ref(token.rid())?;
    if member.class.table() != Some(TableId::TypeSpec) {
        return Ok(None);
    }
    let spec = image.type_spec(member.class)?;
    let cildec::Type::GenericInst {
        is_value_type: true,
        args,
        ..
    } = &spec.type_
    else {
        return Ok(None);
    };
    let name = image.md.strings().str(member.name)?;
    if name.starts_with('.') {
        return Ok(None);
    }
    let signature = crate::signature_guard::method(image.md.blobs().get(member.signature)?)?;
    if name.is_empty()
        || signature.explicit_this
        || signature.calling_convention != cildec::CallConv::Default
        || !signature.vararg_params.is_empty()
    {
        return Err("constructed value call needs an ordinary nongeneric signature".into());
    }
    let owner = Ty::from_sig_scoped(&spec.type_, image, context)?;
    if !matches!(owner, Ty::Value(_)) {
        return Err("value member owner needs an admitted value identity".into());
    }
    let parameter = |parameter: &cildec::Param<'_>| -> Result<Ty> {
        if !parameter.custom_mods.is_empty() {
            return Err("modified value-call parameter needs IL fallback".into());
        }
        let substituted = substitute_type_arguments(&parameter.type_, args)?;
        let ty = Ty::from_sig_scoped(&substituted, image, context)?;
        if parameter.by_ref {
            image.address_of(ty)
        } else {
            Ok(ty)
        }
    };
    Ok(Some(MethodRef {
        token: token.0,
        name: name.into(),
        receiver: signature.has_this.then_some(owner),
        target: format!("__cil_value_call_{:08X}", token.0),
        args: signature
            .params
            .iter()
            .map(&parameter)
            .collect::<Result<Vec<_>>>()?,
        ret: parameter(&signature.return_type)?,
        core_object_constructor: None,
        tick_count: false,
        constructor: false,
        generic_virtual: false,
        exact_reference: true,
    }))
}
