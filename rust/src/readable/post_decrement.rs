// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{Result, metadata::Image};
use cildec::{Operand, Rid};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn rewrite(image: &Image<'_>, caller: u32, body: &str) -> String {
    if !body.contains(" - 1);") || !body.contains("return (") {
        return body.into();
    }
    let Ok(fields) = original_this_i4_fields(image, caller) else {
        return body.into();
    };
    if fields.is_empty() {
        return body.into();
    }
    let lines = body.lines().collect::<Vec<_>>();
    let mut out = String::new();
    let mut i = 0;
    while i < lines.len() {
        let candidate = (|| {
            let (snapshot, location) = lines.get(i)?.strip_prefix("int ")?.split_once(" = ")?;
            if !super::generated_value(snapshot) {
                return None;
            }
            let location = location.strip_suffix(';')?;
            if !fields.contains(location)
                || *lines.get(i + 1)? != format!("{location} = unchecked({snapshot} - 1);")
                || *lines.get(i + 2)? != format!("return ({snapshot} == 0);")
                || lines.iter().enumerate().any(|(n, line)| {
                    !(i..i + 3).contains(&n)
                        && super::identifiers(line).iter().any(|id| id == snapshot)
                })
            {
                return None;
            }
            Some(format!("return unchecked({location}-- == 0);"))
        })();
        if let Some(replacement) = candidate {
            out.push_str(&replacement);
            out.push('\n');
            i += 3;
        } else {
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
        }
    }
    out
}

// Field-only, signed I4, no receiver/address mutation, and no ambiguous alias.
// Property calls are still exact helper calls at this presentation stage.
fn original_this_i4_fields(image: &Image<'_>, caller: u32) -> Result<BTreeSet<String>> {
    if image.row(caller)?.is_static() {
        return Ok(BTreeSet::new());
    }
    let body = image
        .body(caller)?
        .ok_or("post-decrement caller has no body")?;
    for instruction in body.instructions() {
        let instruction = instruction?;
        if matches!(instruction.opcode.canonical().name(), "starg" | "ldarga")
            && matches!(instruction.operand, Operand::Var(0))
        {
            return Ok(BTreeSet::new());
        }
    }
    let owner = image
        .md
        .tables()
        .type_of_method(Rid::new(caller & 0xffffff))?
        .ok_or("post-decrement caller has no owner")?;
    let mut names = BTreeMap::new();
    let mut eligible = BTreeSet::new();
    for (rid, row) in image.md.tables().iter::<cildec::tables::FieldRow>() {
        if image.md.tables().type_of_field(Rid::new(rid))? != Some(owner) {
            continue;
        }
        let row = row?;
        let original = image.md.strings().str(row.name)?;
        let name = if crate::metadata::identifier(original).is_err()
            || original.starts_with("__il_field_")
        {
            format!("__il_field_{:08X}", 0x04000000 | rid)
        } else {
            super::ident(original)
        };
        let location = format!("this.{name}");
        *names.entry(location.clone()).or_insert(0usize) += 1;
        if row.flags & (0x10 | 0x40) == 0 && image.md.blobs().get(row.signature)? == [0x06, 0x08] {
            eligible.insert(location);
        }
    }
    eligible.retain(|name| names.get(name) == Some(&1));
    Ok(eligible)
}
