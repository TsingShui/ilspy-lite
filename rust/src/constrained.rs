// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::Result;
use cildec::{Instruction, TableId, Token};
use std::collections::BTreeMap;

/// Each prefix belongs to exactly one immediately following callvirt.
pub(crate) fn calls(instructions: &[Instruction]) -> Result<BTreeMap<u32, Token>> {
    let mut calls = BTreeMap::new();
    for (index, instruction) in instructions.iter().enumerate() {
        if instruction.opcode.name() != "constrained." {
            continue;
        }
        let target = instructions
            .get(index + 1)
            .ok_or("constrained prefix has no following instruction")?;
        if target.opcode.name() != "callvirt" {
            return Err("constrained prefix needs an immediately following callvirt".into());
        }
        let token = instruction
            .operand
            .token()
            .ok_or("constrained prefix needs a type token")?;
        if token.rid() == 0
            || !matches!(
                token.table(),
                Some(TableId::TypeDef | TableId::TypeRef | TableId::TypeSpec)
            )
        {
            return Err("constrained prefix needs a nonzero TypeDefOrRef token".into());
        }
        calls.insert(target.offset, token);
    }
    Ok(calls)
}
