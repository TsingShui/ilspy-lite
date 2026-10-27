// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! The `tail.` prefix requests a CLI tail call. C# has no equivalent request, so
//! the following call is emitted normally, as ILSpy does; the prefix must still
//! name a call.

use crate::Result;
use cildec::Instruction;

pub(crate) fn check(
    instructions: &[Instruction],
) -> Result<std::collections::BTreeSet<u32>> {
    let mut calls = std::collections::BTreeSet::new();
    for (index, instruction) in instructions.iter().enumerate() {
        if instruction.opcode.name() != "tail." {
            continue;
        }
        let target = instructions
            .get(index + 1)
            .ok_or("tail prefix has no following instruction")?;
        if !matches!(
            target.opcode.name(),
            "call" | "callvirt" | "calli" | "jmp"
        ) {
            return Err("tail prefix needs an immediately following call".into());
        }
        if matches!(target.opcode.name(), "call" | "callvirt") {
            calls.insert(target.offset);
        }
    }
    Ok(calls)
}
