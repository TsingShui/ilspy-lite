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
    Native,
    UNative,
    String,
    CoreException(u32),
    Class(u32),
    I4Array,
    U4Array,
    I4Address,
    U4Address,
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackKind {
    Void,
    I4,
    I8,
    Native,
    Ref,
    Address,
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
            T::IntPtr => Self::Native,
            T::UIntPtr => Self::UNative,
            T::String => Self::String,
            T::Class(t) if t.table() == Some(cildec::TableId::TypeDef) => Self::Class(t.0),
            T::Class(t) if t.table() == Some(cildec::TableId::TypeRef) => Self::CoreException(t.0),
            T::SzArray(mods, element) if mods.is_empty() => match element.as_ref() {
                T::I4 => Self::I4Array,
                T::U4 => Self::U4Array,
                _ => return Err("unsupported array element type needs IL fallback".into()),
            },
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
            Self::Native => "nint",
            Self::UNative => "nuint",
            Self::String => "string",
            Self::Class(t) => return image.class_name(t),
            Self::CoreException(t) => return image.core_exception_name(t),
            Self::I4Array => "int[]",
            Self::U4Array => "uint[]",
            Self::I4Address => "ref int",
            Self::U4Address => "ref uint",
            Self::Null => "object",
        }
        .into())
    }
    pub fn kind(self) -> StackKind {
        match self {
            Self::Void => StackKind::Void,
            Self::I8 | Self::U8 => StackKind::I8,
            Self::Native | Self::UNative => StackKind::Native,
            Self::I4Address | Self::U4Address => StackKind::Address,
            Self::String
            | Self::CoreException(_)
            | Self::Class(_)
            | Self::I4Array
            | Self::U4Array
            | Self::Null => StackKind::Ref,
            _ => StackKind::I4,
        }
    }
    pub fn stack(self) -> Self {
        match self {
            Self::Void
            | Self::String
            | Self::CoreException(_)
            | Self::Class(_)
            | Self::I4Array
            | Self::U4Array
            | Self::I4Address
            | Self::U4Address
            | Self::Null => self,
            Self::I8 | Self::U8 => Self::I8,
            Self::Native | Self::UNative => Self::Native,
            _ => Self::I4,
        }
    }
    pub fn load(self, expression: &str, image: &Image<'_>) -> Result<String> {
        Ok(match self {
            Self::CoreException(_) => {
                self.cs(image)?;
                expression.into()
            }
            Self::String => expression.into(),
            Self::I4
            | Self::I8
            | Self::Native
            | Self::Class(_)
            | Self::I4Array
            | Self::U4Array
            | Self::Null => expression.into(),
            Self::I4Address | Self::U4Address => {
                return Err("managed element address is not an ordinary load".into());
            }
            Self::Bool => format!("({expression} ? 1 : 0)"),
            _ => format!("unchecked(({})({expression}))", self.stack().cs(image)?),
        })
    }
    pub fn store(self, expression: &str, image: &Image<'_>) -> Result<String> {
        Ok(match self {
            Self::CoreException(_) => {
                self.cs(image)?;
                expression.into()
            }
            Self::String => expression.into(),
            Self::I4
            | Self::I8
            | Self::Native
            | Self::Class(_)
            | Self::I4Array
            | Self::U4Array
            | Self::Null => expression.into(),
            Self::I4Address | Self::U4Address => {
                return Err("managed element address cannot be stored as a value".into());
            }
            Self::Bool => format!("({expression} != 0)"),
            _ => format!("unchecked(({})({expression}))", self.cs(image)?),
        })
    }
}
