// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{Result, ir::Ty, metadata::Image};
use cildec::{Operand, Rid};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn rewrite(image: &Image<'_>, caller: u32, body: &str) -> String {
    if !body.contains(" - 1);") {
        return body.into();
    }
    let Ok(locations) = stable_i4_locations(image, caller) else {
        return body.into();
    };
    if locations.is_empty() {
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
            if !locations.contains(location)
                || *lines.get(i + 1)? != format!("{location} = unchecked({snapshot} - 1);")
            {
                return None;
            }
            let only_immediate_use = lines.iter().enumerate().all(|(n, line)| {
                (i..i + 3).contains(&n) || !super::identifiers(line).iter().any(|id| id == snapshot)
            });
            if only_immediate_use
                && lines.get(i + 2) == Some(&format!("return ({snapshot} == 0);").as_str())
            {
                return Some((3, format!("return unchecked({location}-- == 0);")));
            }
            // Keep the immutable old value at its original position when later
            // reads, calls or control flow still need that snapshot. No EH is
            // admitted by lowering, so a failed store cannot expose its local.
            Some((2, format!("int {snapshot} = unchecked({location}--);")))
        })();
        if let Some((consumed, replacement)) = candidate {
            out.push_str(&replacement);
            out.push('\n');
            i += consumed;
        } else {
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
        }
    }
    out
}

// Signed I4 locals without address exposure and unambiguous owned-this fields.
// Property calls are still exact helper calls at this presentation stage.
fn stable_i4_locations(image: &Image<'_>, caller: u32) -> Result<BTreeSet<String>> {
    let body = image
        .body(caller)?
        .ok_or("post-decrement caller has no body")?;
    let instructions = body
        .instructions()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut eligible = BTreeSet::new();
    let context = image.method_type_context(caller, &image.signature(caller)?)?;
    for (index, ty) in image.local_types(&body, context)?.into_iter().enumerate() {
        if ty == Ty::I4
            && !instructions.iter().any(|i| {
                i.opcode.canonical().name() == "ldloca"
                    && matches!(i.operand, Operand::Var(slot) if usize::from(slot) == index)
            })
        {
            eligible.insert(format!("l{index}"));
        }
    }
    if image.row(caller)?.is_static()
        || instructions.iter().any(|i| {
            matches!(i.opcode.canonical().name(), "starg" | "ldarga")
                && matches!(i.operand, Operand::Var(0))
        })
    {
        return Ok(eligible);
    }
    let owner = image
        .md
        .tables()
        .type_of_method(Rid::new(caller & 0xffffff))?
        .ok_or("post-decrement caller has no owner")?;
    let mut names = BTreeMap::new();
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
    eligible.retain(|name| !name.starts_with("this.") || names.get(name) == Some(&1));
    Ok(eligible)
}
