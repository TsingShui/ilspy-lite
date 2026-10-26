// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Typed managed-location descriptors. Storage identity lives in each acquired
//! ref binding, not in the element-type descriptor shared by multiple locations.

use crate::{Result, ir::Ty};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocationId {
    arena: u64,
    index: u32,
}

pub(crate) struct LocationTypes {
    arena: u64,
    elements: Vec<Ty>,
}

impl LocationTypes {
    pub fn new() -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let arena = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| "managed-location arena identities exhausted")?;
        Ok(Self {
            arena,
            elements: Vec::new(),
        })
    }

    pub fn intern(&mut self, element: Ty) -> Result<LocationId> {
        if let Some(index) = self.elements.iter().position(|ty| *ty == element) {
            return Ok(LocationId {
                arena: self.arena,
                index: index as u32,
            });
        }
        if self.elements.len() >= 16384 {
            return Err("managed-location element-type limit exceeded".into());
        }
        let index = self.elements.len() as u32;
        self.elements.push(element);
        Ok(LocationId {
            arena: self.arena,
            index,
        })
    }

    pub fn element(&self, id: LocationId) -> Result<Ty> {
        if id.arena != self.arena {
            return Err("managed location belongs to a different input image".into());
        }
        self.elements
            .get(id.index as usize)
            .copied()
            .ok_or_else(|| "unknown managed-location element type".into())
    }
}
