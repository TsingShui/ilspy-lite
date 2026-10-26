// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Bounded exception regions over the typed CFG. Accepted regions render as
//! runtime try/catch/finally constructs, not as annotations on flattened gotos.

use crate::{Result, cfg::Block, ir::Ty, metadata::Image};
use cildec::{HandlerKind, Instruction, MethodBody, Operand};
use std::collections::BTreeMap;

pub(crate) struct Clause {
    pub index: usize,
    pub catch: Option<(Ty, String)>,
    pub try_start: u32,
    pub try_end: u32,
    pub handler_start: u32,
    pub handler_end: u32,
}

struct Group {
    start: u32,
    try_end: u32,
    end: u32,
    clauses: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Try(usize),
    Handler(usize),
}

pub(crate) struct Regions {
    pub clauses: Vec<Clause>,
    groups: Vec<Group>,
}

pub(crate) fn boundaries(body: &MethodBody<'_>, instructions: &[Instruction]) -> Result<Vec<u32>> {
    if body.exception_handlers.len() > 64 {
        return Err("EH clause limit exceeded".into());
    }
    let starts: std::collections::BTreeSet<_> = instructions.iter().map(|i| i.offset).collect();
    let code_end = body.code.len() as u32;
    let mut result = Vec::new();
    for clause in &body.exception_handlers {
        match clause.kind {
            HandlerKind::Filter { .. } => {
                return Err("EH filters need filter-region lowering".into());
            }
            HandlerKind::Fault => {
                return Err("EH fault handlers need exceptional-only cleanup".into());
            }
            HandlerKind::Catch(_) | HandlerKind::Finally => {}
        }
        for boundary in [
            clause.try_offset,
            clause.try_end(),
            clause.handler_offset,
            clause.handler_end(),
        ] {
            if boundary != code_end && !starts.contains(&boundary) {
                return Err("EH clause boundary is not an instruction start".into());
            }
            result.push(boundary);
        }
    }
    result.sort_unstable();
    result.dedup();
    Ok(result)
}

pub(crate) fn parse(
    image: &Image<'_>,
    body: &MethodBody<'_>,
    instructions: &[Instruction],
) -> Result<Regions> {
    let mut clauses = Vec::new();
    let mut grouped: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (index, raw) in body.exception_handlers.iter().enumerate() {
        if raw.try_offset >= raw.try_end() || raw.handler_offset >= raw.handler_end() {
            return Err("EH clause has an empty range".into());
        }
        let catch = match raw.kind {
            HandlerKind::Catch(token) => {
                Some((Ty::ExternalClass(token.0), image.handler_type_name(token)?))
            }
            HandlerKind::Finally => None,
            _ => return Err("EH handler kind is not admitted".into()),
        };
        grouped
            .entry((raw.try_offset, raw.try_end()))
            .or_default()
            .push(index);
        clauses.push(Clause {
            index,
            catch,
            try_start: raw.try_offset,
            try_end: raw.try_end(),
            handler_start: raw.handler_offset,
            handler_end: raw.handler_end(),
        });
    }
    let mut groups = Vec::new();
    for ((start, try_end), members) in grouped {
        let mut next = try_end;
        for &index in &members {
            let clause = &clauses[index];
            if clause.handler_start != next {
                return Err("EH handlers must follow their try contiguously in table order".into());
            }
            next = clause.handler_end;
        }
        if members.len() != 1 && members.iter().any(|&i| clauses[i].catch.is_none()) {
            return Err("EH shared try must contain catches only".into());
        }
        groups.push(Group {
            start,
            try_end,
            end: next,
            clauses: members,
        });
    }
    // Each nested group must fit wholly in one protected/handler range of its
    // parent. Merely overlapping the parent's total envelope is not sufficient.
    for (i, a) in groups.iter().enumerate() {
        for b in &groups[i + 1..] {
            if a.end <= b.start || b.end <= a.start {
                continue;
            }
            let fits = |inner: &Group, outer: &Group| {
                (outer.start <= inner.start && inner.end <= outer.try_end)
                    || outer.clauses.iter().any(|&c| {
                        let c = &clauses[c];
                        c.handler_start <= inner.start && inner.end <= c.handler_end
                    })
            };
            if !fits(a, b) && !fits(b, a) {
                return Err("EH regions overlap without nesting in one parent region".into());
            }
        }
    }
    let regions = Regions { clauses, groups };
    for instruction in instructions {
        let offset = instruction.offset;
        let path = regions.path(offset);
        if path.len() > 8 {
            return Err("EH nesting depth exceeded".into());
        }
        let op = instruction.opcode.canonical().name();
        match op {
            "leave" => {
                if path.is_empty() || path.iter().any(|part| regions.is_finally(*part)) {
                    return Err("EH leave must originate in a try or catch".into());
                }
                let Operand::BranchTarget(target) = instruction.operand else {
                    return Err("EH leave needs a branch target".into());
                };
                let target_path = regions.path(target);
                if target_path.len() >= path.len() || !path.starts_with(&target_path) {
                    return Err("EH leave must exit to an enclosing region".into());
                }
            }
            "endfinally" => {
                if !path.last().is_some_and(|part| regions.is_finally(*part)) {
                    return Err("EH endfinally must terminate its own finally handler".into());
                }
            }
            "rethrow" => {
                if !path.iter().rev().any(|part| match *part {
                    Part::Handler(index) => regions.clauses[index].catch.is_some(),
                    _ => false,
                }) || path.iter().any(|part| regions.is_finally(*part))
                {
                    return Err("EH rethrow needs an enclosing catch handler".into());
                }
            }
            "endfilter" => return Err("EH endfilter needs filter-region lowering".into()),
            "ret" if !path.is_empty() => {
                return Err("EH return cannot bypass a protected-region exit".into());
            }
            _ => {}
        }
        if op != "leave" {
            let same_region = |target| -> Result<()> {
                if regions.path(target) != path {
                    return Err("EH plain branch crosses a region boundary".into());
                }
                Ok(())
            };
            match &instruction.operand {
                Operand::BranchTarget(target) => same_region(*target)?,
                Operand::Switch(targets) => {
                    for &target in targets {
                        same_region(target)?;
                    }
                }
                _ => {}
            }
        }
        if !instruction.info().flow.is_unconditional_transfer() {
            let target = instruction.next_offset();
            let next = regions.path(target);
            if next != path && !regions.enters_try(offset, target) {
                return Err("EH fallthrough crosses a protected or handler boundary".into());
            }
        }
    }
    Ok(regions)
}

impl Regions {
    fn is_finally(&self, part: Part) -> bool {
        matches!(part, Part::Handler(index) if self.clauses[index].catch.is_none())
    }

    fn path(&self, offset: u32) -> Vec<Part> {
        let mut path = Vec::new();
        for (index, group) in self.groups.iter().enumerate() {
            if group.start <= offset && offset < group.try_end {
                path.push((group.end - group.start, Part::Try(index)));
            }
            for &index in &group.clauses {
                let clause = &self.clauses[index];
                if clause.handler_start <= offset && offset < clause.handler_end {
                    path.push((group.end - group.start, Part::Handler(index)));
                }
            }
        }
        path.sort_by_key(|(length, _)| std::cmp::Reverse(*length));
        path.into_iter().map(|(_, part)| part).collect()
    }

    pub fn enters_try(&self, from: u32, to: u32) -> bool {
        let from_path = self.path(from);
        let target_path = self.path(to);
        target_path.len() > from_path.len()
            && target_path.starts_with(&from_path)
            && target_path[from_path.len()..]
                .iter()
                .all(|part| matches!(*part, Part::Try(index) if self.groups[index].start == to))
    }

    pub fn handler_at(&self, offset: u32) -> Option<&Clause> {
        self.clauses
            .iter()
            .find(|clause| clause.handler_start == offset)
    }

    pub fn catch_variable(&self, offset: u32) -> Option<String> {
        self.handler_at(offset)
            .filter(|clause| clause.catch.is_some())
            .map(|clause| format!("__cil_exception_{}", clause.index))
    }

    pub fn finally_end(&self, offset: u32) -> Option<usize> {
        match self.path(offset).last()? {
            Part::Handler(index) if self.clauses[*index].catch.is_none() => Some(*index),
            _ => None,
        }
    }

    /// Braces are emitted in the lift, before any readable transformations.
    pub fn transition(&self, from: Option<u32>, to: u32) -> Vec<String> {
        let previous = from.map(|offset| self.path(offset)).unwrap_or_default();
        let next = self.path(to);
        let common = previous
            .iter()
            .zip(&next)
            .take_while(|(a, b)| a == b)
            .count();
        let mut lines = Vec::new();
        for &part in previous[common..].iter().rev() {
            if let Part::Handler(index) = part
                && self.clauses[index].catch.is_none()
            {
                lines.push(format!("__cil_finally_end_{index}: ;"));
            }
            lines.push("}".into());
        }
        for &part in &next[common..] {
            match part {
                Part::Try(_) => lines.push("try {".into()),
                Part::Handler(index) => {
                    let clause = &self.clauses[index];
                    lines.push(match &clause.catch {
                        Some((_, name)) => format!("catch ({name} __cil_exception_{index}) {{"),
                        None => "finally {".into(),
                    });
                }
            }
        }
        lines
    }

    pub fn seed_handlers(&self, blocks: &[Block], entries: &mut [Option<Vec<Ty>>]) -> Vec<usize> {
        let mut seeded = Vec::new();
        for clause in &self.clauses {
            if !blocks.iter().enumerate().any(|(n, b)| {
                clause.try_start <= b.offset && b.offset < clause.try_end && entries[n].is_some()
            }) {
                continue;
            }
            if let Some(block) = blocks
                .iter()
                .position(|block| block.offset == clause.handler_start)
                && entries[block].is_none()
            {
                entries[block] = Some(
                    clause
                        .catch
                        .as_ref()
                        .map(|(ty, _)| vec![*ty])
                        .unwrap_or_default(),
                );
                seeded.push(block);
            }
        }
        seeded
    }
}
