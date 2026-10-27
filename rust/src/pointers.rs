// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Pointer descriptors carry the pointee type, so a pointer's C# spelling and
//! dereference width come from the signature that declared it.

use crate::{Result, ir::Ty};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PointerId {
    arena: u64,
    index: u32,
}

pub(crate) struct PointerTypes {
    arena: u64,
    pointees: Vec<Ty>,
}

impl PointerTypes {
    pub fn new() -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let arena = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| "pointer arena identities exhausted")?;
        Ok(Self {
            arena,
            pointees: Vec::new(),
        })
    }

    pub fn intern(&mut self, pointee: Ty) -> Result<PointerId> {
        if let Some(index) = self.pointees.iter().position(|ty| *ty == pointee) {
            return Ok(PointerId {
                arena: self.arena,
                index: index as u32,
            });
        }
        if self.pointees.len() >= 16384 {
            return Err("pointer pointee-type limit exceeded".into());
        }
        let index = self.pointees.len() as u32;
        self.pointees.push(pointee);
        Ok(PointerId {
            arena: self.arena,
            index,
        })
    }

    pub fn pointee(&self, id: PointerId) -> Result<Ty> {
        if id.arena != self.arena {
            return Err("pointer belongs to a different input image".into());
        }
        self.pointees
            .get(id.index as usize)
            .copied()
            .ok_or_else(|| "unknown pointer pointee type".into())
    }
}
