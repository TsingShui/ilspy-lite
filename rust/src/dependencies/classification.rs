// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::reference_identity;
use crate::{Result, metadata::Image};
use cildec::{CodedIndex, TableId};

/// This classifies static metadata evidence, not execution compatibility or
/// publisher authentication. Absence of a reference marker is not a body proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssemblyKind {
    ImplementationImage,
    ReferenceContract,
    Unclassified,
}

pub(super) fn classify(image: &Image<'_>) -> Result<AssemblyKind> {
    for (rid, row) in image
        .md
        .tables()
        .iter::<cildec::tables::CustomAttributeRow>()
    {
        let row = row?;
        let parent = image.metadata_coded_token(
            CodedIndex::HasCustomAttribute,
            image
                .md
                .tables()
                .raw_column(TableId::CustomAttribute, rid, 0)?,
        )?;
        if parent.0 != 0x20000001 {
            continue;
        }
        let token = image.metadata_coded_token(
            CodedIndex::CustomAttributeType,
            image
                .md
                .tables()
                .raw_column(TableId::CustomAttribute, rid, 1)?,
        )?;
        let (name, namespace, type_name, signature) = match token.table() {
            Some(TableId::MemberRef) => {
                let constructor = image.md.tables().member_ref(token.rid())?;
                let owner = image.metadata_coded_token(
                    CodedIndex::MemberRefParent,
                    image
                        .md
                        .tables()
                        .raw_column(TableId::MemberRef, token.rid(), 0)?,
                )?;
                if owner.table() != Some(TableId::TypeRef) {
                    continue;
                }
                let ty = image.md.tables().type_ref(owner.rid())?;
                let scope = image.metadata_coded_token(
                    CodedIndex::ResolutionScope,
                    image
                        .md
                        .tables()
                        .raw_column(TableId::TypeRef, owner.rid(), 0)?,
                )?;
                if scope.table() != Some(TableId::AssemblyRef)
                    || !super::search::core_contract(&reference_identity(image, scope.rid())?)
                {
                    continue;
                }
                (
                    constructor.name,
                    ty.type_namespace,
                    ty.type_name,
                    constructor.signature,
                )
            }
            Some(TableId::MethodDef) => {
                if !image.is_core_definition()? {
                    continue;
                }
                let constructor = image.md.tables().method_def(token.rid())?;
                if constructor.is_static()
                    || constructor.flags & 0x1800 != 0x1800
                    || !image.md.tables().generic_params(token)?.is_empty()
                {
                    continue;
                }
                let owner = image
                    .md
                    .tables()
                    .type_of_method(cildec::Rid::new(token.rid()))?
                    .ok_or("reference marker constructor has no owner")?;
                let ty = image.md.tables().type_def(owner.get())?;
                if ty.is_nested()
                    || !image
                        .md
                        .tables()
                        .generic_params(cildec::Token::new(TableId::TypeDef, owner.get()))?
                        .is_empty()
                {
                    continue;
                }
                (
                    constructor.name,
                    ty.type_namespace,
                    ty.type_name,
                    constructor.signature,
                )
            }
            _ => continue,
        };
        if image.md.strings().str(name)? != ".ctor"
            || image.md.strings().str(namespace)? != "System.Runtime.CompilerServices"
            || image.md.strings().str(type_name)? != "ReferenceAssemblyAttribute"
        {
            continue;
        }
        if image.md.blobs().get(signature)? != [0x20, 0, 1]
            || image.md.blobs().get(row.value)? != [1, 0, 0, 0]
        {
            return Err("invalid reference assembly marker evidence".into());
        }
        return Ok(AssemblyKind::ReferenceContract);
    }
    Ok(AssemblyKind::ImplementationImage)
}
