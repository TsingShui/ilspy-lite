// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::{Image, MethodRef, substitute_type_arguments};
use crate::{Result, ir::Ty, signature_types::TypeContext};
use cildec::{TableId, Token};

/// Exact calls carry the caller's requested owner/signature, without selecting
/// an external definition or proving a source-language member lookup.
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
        is_value_type: false,
        def,
        args,
    } = &spec.type_
    else {
        return Ok(None);
    };
    // A local class instantiation is resolved against its definition so the
    // declared signature is checked; a local *interface* instantiation has no
    // class members behind it, so it keeps the exact-token dispatch path and a
    // callvirt on it becomes the interface call the IL names.
    if let Some(definition) = image.local_type_definition(*def)?
        && image.md.tables().type_def(definition.rid())?.flags & 0x0020 == 0
    {
        return Ok(None);
    }
    let signature = crate::signature_guard::method(image.md.blobs().get(member.signature)?)?;
    if signature.explicit_this
        || signature.calling_convention != cildec::CallConv::Default
        || !signature.vararg_params.is_empty()
    {
        return Err("constructed reference call needs an ordinary nongeneric signature".into());
    }
    let name = image.md.strings().str(member.name)?;
    if name.is_empty() || name.starts_with('.') {
        return Err(
            "constructed reference call cannot infer constructor/allocation semantics".into(),
        );
    }
    let owner = Ty::from_sig_scoped(&spec.type_, image, context)?;
    owner.cs(image)?;
    let parameter = |parameter: &cildec::Param<'_>| -> Result<Ty> {
        if !parameter.custom_mods.is_empty() {
            return Err("byref/modified parameter needs IL fallback".into());
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
        target: format!("__cil_reference_call_{:08X}", token.0),
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
