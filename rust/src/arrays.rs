// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Vector shape and element operations keep storage checks separate from value layout.

use crate::{Result, ir::Ty, metadata::Image};

pub(crate) fn scalar(image: &Image<'_>, vector: Ty, opcode: &str) -> Result<(Ty, Ty)> {
    let element = element(image, vector)?;
    let storage = match element {
        Ty::Enum(value) => value.storage.ty(),
        other => other,
    };
    let requested = match opcode {
        "ldelem.i1" if matches!(storage, Ty::I1 | Ty::U1 | Ty::Bool) => Ty::I1,
        "ldelem.u1" if matches!(storage, Ty::I1 | Ty::U1 | Ty::Bool) => Ty::U1,
        "stelem.i1" if matches!(storage, Ty::I1 | Ty::U1 | Ty::Bool) => Ty::I4,
        "ldelem.i2" if matches!(storage, Ty::I2 | Ty::U2 | Ty::Char) => Ty::I2,
        "ldelem.u2" if matches!(storage, Ty::I2 | Ty::U2 | Ty::Char) => Ty::U2,
        "stelem.i2" if matches!(storage, Ty::I2 | Ty::U2 | Ty::Char) => Ty::I4,
        "ldelem.i8" | "stelem.i8" if matches!(storage, Ty::I8 | Ty::U8) => Ty::I8,
        "ldelem.r4" | "stelem.r4" if storage == Ty::R4 => Ty::R4,
        _ => return Err("scalar element opcode differs from vector storage width or kind".into()),
    };
    Ok((element, requested))
}

pub(crate) fn operand(
    image: &Image<'_>,
    vector: Ty,
    token: cildec::Token,
    context: crate::signature_types::TypeContext,
) -> Result<Ty> {
    let element = element(image, vector)?;
    image.location_operand(image.address_of(element)?, token, context)
}

pub(crate) fn element(image: &Image<'_>, ty: Ty) -> Result<Ty> {
    match ty {
        Ty::I4Array => Ok(Ty::I4),
        Ty::U4Array => Ok(Ty::U4),
        Ty::StructuralRef(id) => {
            let (bytes, context) = image.signature_types.borrow().signature(id)?;
            let signature = crate::signature_guard::type_spec(&bytes)?;
            match signature.type_ {
                cildec::Type::SzArray(modifiers, element) if modifiers.is_empty() => {
                    Ty::from_sig_scoped(&element, image, context)
                }
                _ => Err("operation needs an ordinary vector reference".into()),
            }
        }
        _ => Err("operation needs an ordinary vector reference".into()),
    }
}

/// What an array intrinsic does with the element it names.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    Get,
    Set,
}

/// A resolved array intrinsic: the element kind, the rank that fixes its argument
/// count, and the operation.
pub(crate) struct Intrinsic {
    pub element: Ty,
    pub rank: usize,
    pub access: Access,
}

/// Element and rank of a non-vector array reference. C# can only spell shapes with
/// zero lower bounds and no declared sizes, so any other shape is refused.
pub(crate) fn general_shape(image: &Image<'_>, ty: Ty) -> Result<(Ty, usize)> {
    let Ty::StructuralRef(id) = ty else {
        return Err("operation needs a general array reference".into());
    };
    let (bytes, context) = image.signature_types.borrow().signature(id)?;
    let signature = crate::signature_guard::type_spec(&bytes)?;
    match signature.type_ {
        cildec::Type::Array(element, shape) => {
            if !shape.sizes.is_empty()
                || shape.lo_bounds.iter().any(|bound| *bound != 0)
                || shape.lo_bounds.len() > shape.rank as usize
            {
                return Err("array shape needs zero lower bounds and no declared sizes".into());
            }
            Ok((
                Ty::from_sig_scoped(&element, image, context)?,
                shape.rank as usize,
            ))
        }
        _ => Err("operation needs a general array reference".into()),
    }
}

/// Any array reference, vector or general, for operations that only need length.
pub(crate) fn reference(image: &Image<'_>, ty: Ty) -> Result<()> {
    match ty {
        Ty::I4Array | Ty::U4Array => Ok(()),
        Ty::StructuralRef(id) => {
            let (bytes, _) = image.signature_types.borrow().signature(id)?;
            let signature = crate::signature_guard::type_spec(&bytes)?;
            if matches!(
                &signature.type_,
                cildec::Type::SzArray(modifiers, _) if modifiers.is_empty()
            ) || matches!(&signature.type_, cildec::Type::Array(..))
            {
                Ok(())
            } else {
                Err("operation needs an array reference".into())
            }
        }
        _ => Err("operation needs an array reference".into()),
    }
}
