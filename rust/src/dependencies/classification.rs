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
        if parent.0 != 0x20000001 || row.constructor.table() != Some(TableId::MemberRef) {
            continue;
        }
        let constructor = image.md.tables().member_ref(row.constructor.rid())?;
        if image.md.strings().str(constructor.name)? != ".ctor"
            || constructor.class.table() != Some(TableId::TypeRef)
        {
            continue;
        }
        let ty = image.md.tables().type_ref(constructor.class.rid())?;
        if image.md.strings().str(ty.type_namespace)? != "System.Runtime.CompilerServices"
            || image.md.strings().str(ty.type_name)? != "ReferenceAssemblyAttribute"
            || ty.resolution_scope.table() != Some(TableId::AssemblyRef)
        {
            continue;
        }
        let identity = reference_identity(image, ty.resolution_scope.rid())?;
        if !super::search::core_contract(&identity) {
            continue;
        }
        if image.md.blobs().get(constructor.signature)? != [0x20, 0, 1]
            || image.md.blobs().get(row.value)? != [1, 0, 0, 0]
        {
            return Err("invalid reference assembly marker evidence".into());
        }
        return Ok(AssemblyKind::ReferenceContract);
    }
    Ok(AssemblyKind::ImplementationImage)
}
