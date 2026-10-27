// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! `volatile.` marks the immediately following memory access as acquire/release.

use crate::Result;
use cildec::Instruction;
use std::collections::BTreeSet;

/// Offsets of field accesses that a `volatile.` prefix marks. Indirect and block
/// forms are refused here because their C# spelling needs an address view this
/// module does not model.
pub(crate) fn accesses(instructions: &[Instruction]) -> Result<BTreeSet<u32>> {
    let mut offsets = BTreeSet::new();
    for (index, instruction) in instructions.iter().enumerate() {
        if instruction.opcode.name() != "volatile." {
            continue;
        }
        let target = instructions
            .get(index + 1)
            .ok_or("volatile prefix has no following instruction")?;
        match target.opcode.name() {
            "ldfld" | "stfld" | "ldsfld" | "stsfld" => {}
            "ldobj" | "stobj" | "cpblk" | "initblk" | "ldind" => {
                return Err("volatile indirect access needs IL fallback".into());
            }
            name if name.starts_with("ldind.") || name.starts_with("stind.") => {
                return Err("volatile indirect access needs IL fallback".into());
            }
            _ => return Err("volatile prefix needs an immediately following field access".into()),
        }
        if !offsets.insert(target.offset) {
            return Err("duplicate volatile prefix".into());
        }
    }
    Ok(offsets)
}
