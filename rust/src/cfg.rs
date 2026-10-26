// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::Result;
use cildec::{FlowControl, Instruction, Operand};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Block {
    pub start: usize,
    pub end: usize,
    pub offset: u32,
    pub successors: Vec<usize>,
}

pub(crate) fn build(
    instructions: &[Instruction],
    code_size: usize,
    boundaries: &[u32],
) -> Result<Vec<Block>> {
    if instructions.is_empty() {
        return Err("empty method body".into());
    }
    let positions: BTreeMap<u32, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, ins)| (ins.offset, i))
        .collect();
    let mut leaders = BTreeSet::from([0usize]);
    // Exception clause boundaries are leaders even when no branch targets them.
    for &boundary in boundaries {
        if boundary as usize == code_size {
            continue;
        }
        leaders.insert(
            *positions
                .get(&boundary)
                .ok_or("EH clause boundary is not an instruction start")?,
        );
    }
    for (i, ins) in instructions.iter().enumerate() {
        let mut target = |offset: u32| -> Result<()> {
            leaders.insert(
                *positions
                    .get(&offset)
                    .ok_or("branch target is outside code or inside an instruction")?,
            );
            Ok(())
        };
        match &ins.operand {
            Operand::BranchTarget(t) => target(*t)?,
            Operand::Switch(ts) => {
                for t in ts {
                    target(*t)?;
                }
            }
            _ => {}
        }
        if matches!(
            ins.info().flow,
            FlowControl::Branch
                | FlowControl::CondBranch
                | FlowControl::Return
                | FlowControl::Throw
        ) && i + 1 < instructions.len()
        {
            leaders.insert(i + 1);
        }
    }
    if instructions.last().unwrap().next_offset() as usize != code_size {
        return Err("incomplete instruction coverage".into());
    }
    let starts = leaders.into_iter().collect::<Vec<_>>();
    if starts.len() > 4096 {
        return Err("basic block limit exceeded".into());
    }
    let mut blocks: Vec<_> = starts
        .iter()
        .enumerate()
        .map(|(n, &start)| Block {
            start,
            end: starts.get(n + 1).copied().unwrap_or(instructions.len()),
            offset: instructions[start].offset,
            successors: Vec::new(),
        })
        .collect();
    let by_offset: BTreeMap<_, _> = blocks
        .iter()
        .enumerate()
        .map(|(n, b)| (b.offset, n))
        .collect();
    let count = blocks.len();
    for (n, b) in blocks.iter_mut().enumerate() {
        let ins = &instructions[b.end - 1];
        match &ins.operand {
            Operand::BranchTarget(t) => b.successors.push(by_offset[t]),
            Operand::Switch(ts) => {
                for t in ts {
                    b.successors.push(by_offset[t]);
                }
            }
            _ => {}
        }
        if !ins.info().flow.is_unconditional_transfer() {
            if n + 1 == count {
                return Err("control flow falls off the end of the method".into());
            }
            b.successors.push(n + 1);
        }
    }
    Ok(blocks)
}
