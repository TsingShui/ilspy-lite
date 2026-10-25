// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{Result, metadata::Image};

/// Tokens are scoped to the one input Image. No cross-image token equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ty {
    Void,
    Bool,
    Char,
    I1,
    U1,
    I2,
    U2,
    I4,
    U4,
    I8,
    U8,
    Class(u32),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackKind {
    Void,
    I4,
    I8,
    Ref,
}

impl Ty {
    pub fn from_sig(t: &cildec::Type<'_>) -> Result<Self> {
        use cildec::Type as T;
        Ok(match t {
            T::Void => Self::Void,
            T::Boolean => Self::Bool,
            T::Char => Self::Char,
            T::I1 => Self::I1,
            T::U1 => Self::U1,
            T::I2 => Self::I2,
            T::U2 => Self::U2,
            T::I4 => Self::I4,
            T::U4 => Self::U4,
            T::I8 => Self::I8,
            T::U8 => Self::U8,
            T::Class(t) if t.table() == Some(cildec::TableId::TypeDef) => Self::Class(t.0),
            _ => return Err("unsupported signature type needs IL fallback".into()),
        })
    }
    pub fn from_param(p: &cildec::Param<'_>) -> Result<Self> {
        if p.by_ref || !p.custom_mods.is_empty() {
            return Err("byref/modified parameter needs IL fallback".into());
        }
        Self::from_sig(&p.type_)
    }
    pub fn cs(self, image: &Image<'_>) -> Result<String> {
        Ok(match self {
            Self::Void => "void",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::I1 => "sbyte",
            Self::U1 => "byte",
            Self::I2 => "short",
            Self::U2 => "ushort",
            Self::I4 => "int",
            Self::U4 => "uint",
            Self::I8 => "long",
            Self::U8 => "ulong",
            Self::Class(t) => return image.class_name(t),
            Self::Null => "object",
        }
        .into())
    }
    pub fn kind(self) -> StackKind {
        match self {
            Self::Void => StackKind::Void,
            Self::I8 | Self::U8 => StackKind::I8,
            Self::Class(_) | Self::Null => StackKind::Ref,
            _ => StackKind::I4,
        }
    }
    pub fn stack(self) -> Self {
        match self {
            Self::Void | Self::Class(_) | Self::Null => self,
            Self::I8 | Self::U8 => Self::I8,
            _ => Self::I4,
        }
    }
    pub fn load(self, expression: &str, image: &Image<'_>) -> Result<String> {
        Ok(match self {
            Self::I4 | Self::I8 | Self::Class(_) | Self::Null => expression.into(),
            Self::Bool => format!("({expression} ? 1 : 0)"),
            _ => format!("unchecked(({})({expression}))", self.stack().cs(image)?),
        })
    }
    pub fn store(self, expression: &str, image: &Image<'_>) -> Result<String> {
        Ok(match self {
            Self::I4 | Self::I8 | Self::Class(_) | Self::Null => expression.into(),
            Self::Bool => format!("({expression} != 0)"),
            _ => format!("unchecked(({})({expression}))", self.cs(image)?),
        })
    }
}
