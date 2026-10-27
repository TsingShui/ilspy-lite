// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::{Image, MethodRef, substitute_arguments};
use crate::{Result, ir::Ty, signature_types::TypeContext};
use cildec::{CodedIndex, Rid, TableId, Token};

fn local_instance_method(
    image: &Image<'_>,
    owner: Token,
    name: &str,
    signature: &cildec::MethodSig<'_>,
) -> Result<Token> {
    let mut found = None;
    for rid in image.md.tables().method_range(Rid::new(owner.rid()))? {
        let candidate = image.md.tables().method_def(rid)?;
        if image.md.strings().str(candidate.name)? == name
            && image.md.blobs().get(candidate.signature)? == signature.raw
        {
            if found.is_some() {
                return Err("generic instance method target is ambiguous".into());
            }
            if candidate.is_static() {
                return Err("generic instance method target is static".into());
            }
            found = Some(Token(0x06000000 | rid));
        }
    }
    let target = found.ok_or("generic instance method needs an exact local declaration")?;
    image.method_type_context(target.0, signature)?;
    Ok(target)
}

/// Bind the signature but keep the exact MethodSpec as the runtime call target.
pub(super) fn resolve(image: &Image<'_>, token: Token, context: TypeContext) -> Result<MethodRef> {
    let tables = image.md.tables();
    let row = tables.method_spec(token.rid())?;
    let target = image.metadata_coded_token(
        CodedIndex::MethodDefOrRef,
        tables.raw_column(TableId::MethodSpec, token.rid(), 0)?,
    )?;
    let instantiation =
        crate::signature_guard::method_spec(image.md.blobs().get(row.instantiation)?)?;
    let (name, owner, signature) = match target.table() {
        Some(TableId::MethodDef) => {
            let row = image.row(target.0)?;
            let owner = tables
                .type_of_method(Rid::new(target.rid()))?
                .ok_or("generic method target has no declaring type")?;
            let signature = image.signature(target.0)?;
            if row.is_static() == signature.has_this || row.flags & 0x1000 != 0 {
                return Err("generic method target has inconsistent instance/special flags".into());
            }
            image.method_type_context(target.0, &signature)?;
            (
                image.md.strings().str(row.name)?,
                Token(0x02000000 | owner.get()),
                signature,
            )
        }
        Some(TableId::MemberRef) => {
            let row = tables.member_ref(target.rid())?;
            (
                image.md.strings().str(row.name)?,
                row.class,
                crate::signature_guard::method(image.md.blobs().get(row.signature)?)?,
            )
        }
        _ => return Err("method instantiation needs MethodDef or MemberRef".into()),
    };
    let cildec::CallConv::Generic(arity) = signature.calling_convention else {
        return Err("method instantiation target is not generic".into());
    };
    if arity == 0 || arity as usize != instantiation.args.len() {
        return Err("generic method argument count differs from signature".into());
    }
    if name.is_empty()
        || name.starts_with('.')
        || signature.explicit_this
        || !signature.vararg_params.is_empty()
    {
        return Err("generic method target needs an ordinary non-vararg signature".into());
    }
    // Even unused generic arguments participate in runtime identity and constraints.
    for argument in &instantiation.args {
        let ty = Ty::from_sig_scoped(argument, image, context)?;
        if ty == Ty::Void || ty.kind() == crate::ir::StackKind::Address {
            return Err("generic method argument needs an admitted nonvoid value type".into());
        }
        ty.cs(image)?;
    }
    let mut type_arguments = None;
    let mut generic_virtual =
        target.table() == Some(TableId::MethodDef) && image.row(target.0)?.flags & 0x40 != 0;
    let receiver = if owner.table() == Some(TableId::TypeSpec) {
        let spec = image.type_spec(owner)?;
        let cildec::Type::GenericInst {
            is_value_type: false,
            args,
            ..
        } = &spec.type_
        else {
            return Err("generic method owner needs a reference-class instantiation".into());
        };
        if signature.has_this {
            let bound = image
                .constructed_member_owner(owner, context)?
                .ok_or("generic instance method needs a constructed local owner")?;
            let declaration = local_instance_method(image, bound.definition, name, &signature)?;
            generic_virtual = image.row(declaration.0)?.flags & 0x40 != 0;
            type_arguments = Some(args.clone());
            Some(bound.receiver)
        } else {
            // This proves only signature identity, not layout or a runtime body.
            Ty::from_sig_scoped(&spec.type_, image, context)?.cs(image)?;
            type_arguments = Some(args.clone());
            None
        }
    } else if owner.table() == Some(TableId::TypeDef) {
        let arity = tables.generic_params(owner)?.len();
        if arity != 0
            && (owner.0 != context.type_scope.owner || arity != context.type_scope.count as usize)
        {
            return Err(
                "generic method definition owner needs an instantiated declaring type".into(),
            );
        }
        if signature.has_this {
            let declaration = if target.table() == Some(TableId::MethodDef) {
                target
            } else {
                local_instance_method(image, owner, name, &signature)?
            };
            generic_virtual = image.row(declaration.0)?.flags & 0x40 != 0;
            let ty = image.receiver(declaration.0)?;
            if ty.kind() != crate::ir::StackKind::Ref {
                return Err("generic instance method needs a reference receiver".into());
            }
            Some(ty)
        } else {
            None
        }
    } else if owner.table() == Some(TableId::TypeRef) {
        if !signature.has_this {
            image.external_class_context(owner.0)?;
            None
        } else {
            // ILSpy specializes the declaration found on the selected owner
            // definition. A virtual instantiated method still dispatches by
            // runtime lookup, so it keeps the generic-virtual refusal.
            let fact = match image.external_method_instantiation(token.0) {
                Some(Ok(fact)) => fact,
                Some(Err(reason)) => return Err(reason.into()),
                None => {
                    return Err("external generic instance dispatch needs dependency facts".into());
                }
            };
            if fact.generic_virtual {
                return Err("generic virtual dispatch needs runtime lookup-order modeling".into());
            }
            Some(image.external_receiver_type(owner)?)
        }
    } else {
        return Err("external generic instance dispatch needs dependency facts".into());
    };
    let parameter = |parameter: &cildec::Param<'_>| -> Result<Ty> {
        if !parameter.custom_mods.is_empty() {
            return Err("byref/modified parameter needs IL fallback".into());
        }
        let substituted = substitute_arguments(
            &parameter.type_,
            type_arguments.as_deref(),
            Some(&instantiation.args),
        )?;
        let ty = Ty::from_sig_scoped(&substituted, image, context)?;
        if parameter.by_ref {
            image.address_of(ty)
        } else {
            Ok(ty)
        }
    };
    Ok(MethodRef {
        token: token.0,
        name: name.into(),
        receiver,
        target: format!("__cil_methodspec_call_{:08X}", token.0),
        args: signature
            .params
            .iter()
            .map(&parameter)
            .collect::<Result<Vec<_>>>()?,
        ret: parameter(&signature.return_type)?,
        core_object_constructor: None,
        tick_count: false,
        constructor: false,
        generic_virtual,
        exact_reference: true,
    })
}
