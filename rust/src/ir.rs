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
    R4,
    R8,
    F4,
    String,
    Object,
    ExternalClass(u32),
    StructuralRef(crate::signature_types::TypeId),
    GenericValue(crate::signature_types::TypeId),
    Value(crate::signature_types::TypeId),
    ValueAddress(crate::signature_types::TypeId),
    ManagedAddress(crate::locations::LocationId),
    I8Address,
    U8Address,
    Enum(EnumType),
    /// Local named reference: a class or admitted nongeneric interface.
    Class(u32),
    I4Array,
    U4Array,
    I4Address,
    U4Address,
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EnumStorage {
    I1,
    U1,
    I2,
    U2,
    I4,
    U4,
    I8,
    U8,
}

impl EnumStorage {
    pub fn ty(self) -> Ty {
        match self {
            Self::I1 => Ty::I1,
            Self::U1 => Ty::U1,
            Self::I2 => Ty::I2,
            Self::U2 => Ty::U2,
            Self::I4 => Ty::I4,
            Self::U4 => Ty::U4,
            Self::I8 => Ty::I8,
            Self::U8 => Ty::U8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EnumType {
    pub token: u32,
    pub storage: EnumStorage,
    /// Index into the current image's external value-type names when the storage
    /// fact came from another assembly. The token then belongs to that assembly
    /// and must never be resolved against the image being decompiled.
    pub external: Option<u32>,
}

impl EnumType {
    pub fn name(self) -> String {
        format!("__cil_enum_{:08X}", self.token)
    }

    /// The defining assembly's own name for a cross-image enum, or the local
    /// definition name for a same-image one.
    pub fn display(self, image: &Image<'_>) -> Result<String> {
        match self.external {
            Some(index) => image.external_type_name(index),
            None => crate::symbol_names::type_name(image, cildec::Token(self.token), true),
        }
    }

    /// A same-image enum keeps its input-scoped alias; a cross-image enum is
    /// named through the defining assembly instead of a foreign token.
    pub fn render(self, image: &Image<'_>) -> Result<String> {
        match self.external {
            Some(_) => self.display(image),
            None => Ok(self.name()),
        }
    }
    pub fn evidence(self, image: &Image<'_>) -> Result<String> {
        Ok(format!(
            "// enum storage token=0x{:08X} alias={} underlying={}: {}; exact input-scoped value type; loads normalize to CLI integer stack bits and stores truncate to the declared width.\n",
            self.token,
            self.name(),
            self.storage.ty().cs(image)?,
            crate::metadata::escape_line(&self.display(image)?)
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackKind {
    Void,
    I4,
    I8,
    Native,
    Float,
    Ref,
    Address,
    GenericValue,
    Value,
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
            T::R4 => Self::R4,
            T::R8 => Self::R8,
            T::String => Self::String,
            T::Object => Self::Object,
            T::Class(t) if t.table() == Some(cildec::TableId::TypeDef) => Self::Class(t.0),
            T::Class(t) if t.table() == Some(cildec::TableId::TypeRef) => Self::ExternalClass(t.0),
            T::SzArray(mods, element) if mods.is_empty() => match element.as_ref() {
                T::I4 => Self::I4Array,
                T::U4 => Self::U4Array,
                _ => return Err("unsupported array element type needs IL fallback".into()),
            },
            _ => return Err("unsupported signature type needs IL fallback".into()),
        })
    }
    pub fn from_sig_in(t: &cildec::Type<'_>, image: &Image<'_>) -> Result<Self> {
        Self::from_sig_scoped(t, image, crate::signature_types::TypeContext::default())
    }
    pub fn from_sig_scoped(
        t: &cildec::Type<'_>,
        image: &Image<'_>,
        context: crate::signature_types::TypeContext,
    ) -> Result<Self> {
        if let cildec::Type::ValueType(token) = t {
            return match image.enum_type(*token) {
                Ok(ty) => Ok(Self::Enum(ty)),
                Err(error) => match image.struct_type(*token)? {
                    Some(ty) => Ok(Self::Value(ty)),
                    None => Err(error),
                },
            };
        }
        if matches!(t, cildec::Type::Var(_) | cildec::Type::MVar(_)) {
            return Ok(Self::GenericValue(image.intern_signature_type(t, context)?));
        }
        // A constructed value type is copied whole by the CLI; the definition has
        // to pass the same proof a struct copy needs, with its generic parameters
        // supplied by the instantiation.
        if let cildec::Type::GenericInst {
            is_value_type: true,
            def,
            args,
            ..
        } = t
        {
            image.admit_generic_value_definition(*def, args.len())?;
            return Ok(Self::Value(image.intern_signature_type(t, context)?));
        }
        if let cildec::Type::GenericInst {
            is_value_type: false,
            def,
            args,
        } = t
        {
            let id = image.intern_signature_type(t, context)?;
            let open_self = args.iter().enumerate().all(|(index, argument)| {
                matches!(argument, cildec::Type::Var(position) if *position as usize == index)
            });
            if open_self
                && let Some(definition) = image.local_type_definition(*def)?
                && definition.0 == context.type_scope.owner
                && args.len() == context.type_scope.count as usize
            {
                image.register_open_self(id, definition.0);
            }
            return Ok(Self::StructuralRef(id));
        }
        match Self::from_sig(t) {
            Ok(ty) => Ok(ty),
            Err(error) => {
                if matches!(t, cildec::Type::SzArray(modifiers, _) if modifiers.is_empty()) {
                    Ok(Self::StructuralRef(
                        image.intern_signature_type(t, context)?,
                    ))
                } else {
                    Err(error)
                }
            }
        }
    }
    pub fn from_param(p: &cildec::Param<'_>, image: &Image<'_>) -> Result<Self> {
        Self::from_param_scoped(p, image, crate::signature_types::TypeContext::default())
    }
    pub fn from_param_scoped(
        p: &cildec::Param<'_>,
        image: &Image<'_>,
        context: crate::signature_types::TypeContext,
    ) -> Result<Self> {
        if !p.custom_mods.is_empty() {
            return Err("byref/modified parameter needs IL fallback".into());
        }
        let ty = Self::from_sig_scoped(&p.type_, image, context)?;
        if p.by_ref {
            image.address_of(ty)
        } else {
            Ok(ty)
        }
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
            Self::R4 => "float",
            Self::R8 => "double",
            Self::F4 => "__cil_F",
            Self::String => "string",
            Self::Object => "object",
            Self::Enum(t) => return t.render(image),
            Self::Class(t) => return image.class_name(t),
            Self::ExternalClass(t) => return image.external_class_name(t),
            Self::StructuralRef(t) | Self::GenericValue(t) | Self::Value(t) => {
                return image.signature_types.borrow().name(t);
            }
            Self::ValueAddress(t) => {
                return Ok(format!("ref {}", image.signature_types.borrow().name(t)?));
            }
            Self::ManagedAddress(_) => {
                return Ok(format!("ref {}", image.address_element(self)?.cs(image)?));
            }
            Self::I4Array => "int[]",
            Self::U4Array => "uint[]",
            Self::I4Address => "ref int",
            Self::U4Address => "ref uint",
            Self::I8Address => "ref long",
            Self::U8Address => "ref ulong",
            Self::Null => "object",
        }
        .into())
    }
    pub fn kind(self) -> StackKind {
        match self {
            Self::Void => StackKind::Void,
            Self::I8 | Self::U8 => StackKind::I8,
            Self::Native | Self::UNative => StackKind::Native,
            Self::R4 | Self::R8 | Self::F4 => StackKind::Float,
            Self::I4Address | Self::U4Address | Self::ManagedAddress(_) => StackKind::Address,
            Self::GenericValue(_) => StackKind::GenericValue,
            Self::Value(_) => StackKind::Value,
            Self::ValueAddress(_) | Self::I8Address | Self::U8Address => StackKind::Address,
            Self::Enum(t) => t.storage.ty().kind(),
            Self::String
            | Self::Object
            | Self::ExternalClass(_)
            | Self::StructuralRef(_)
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
            | Self::Object
            | Self::ExternalClass(_)
            | Self::StructuralRef(_)
            | Self::Class(_)
            | Self::I4Array
            | Self::U4Array
            | Self::I4Address
            | Self::U4Address
            | Self::GenericValue(_)
            | Self::Value(_)
            | Self::ValueAddress(_)
            | Self::ManagedAddress(_)
            | Self::I8Address
            | Self::U8Address
            | Self::Null => self,
            Self::I8 | Self::U8 => Self::I8,
            Self::Native | Self::UNative => Self::Native,
            Self::R4 | Self::F4 => Self::F4,
            Self::R8 => Self::R8,
            Self::Enum(t) => t.storage.ty().stack(),
            _ => Self::I4,
        }
    }
    pub fn load(self, expression: &str, image: &Image<'_>) -> Result<String> {
        Ok(match self {
            Self::R4 => format!("__cil_load_r4({expression})"),
            Self::Enum(t) => {
                let storage = t.storage.ty();
                storage.load(
                    &format!("unchecked(({})({expression}))", storage.cs(image)?),
                    image,
                )?
            }
            Self::F4
            | Self::R8
            | Self::Object
            | Self::StructuralRef(_)
            | Self::GenericValue(_)
            | Self::Value(_) => expression.into(),
            // Copying an acquired managed pointer keeps its original location.
            Self::ValueAddress(_) | Self::ManagedAddress(_) => expression.into(),
            Self::ExternalClass(_) => {
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
            Self::I4Address | Self::U4Address | Self::I8Address | Self::U8Address => {
                return Err("managed element address is not an ordinary load".into());
            }
            Self::Bool => format!("__cil_bool_load({expression})"),
            _ => format!("unchecked(({})({expression}))", self.stack().cs(image)?),
        })
    }
    pub fn store(self, expression: &str, image: &Image<'_>) -> Result<String> {
        Ok(match self {
            Self::R4 => format!("__cil_store_r4({expression})"),
            Self::Enum(t) => format!(
                "unchecked(({})({}))",
                t.render(image)?,
                t.storage.ty().store(expression, image)?
            ),
            Self::F4
            | Self::Object
            | Self::StructuralRef(_)
            | Self::GenericValue(_)
            | Self::Value(_) => expression.into(),
            Self::ExternalClass(_) => {
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
            Self::I4Address
            | Self::U4Address
            | Self::I8Address
            | Self::U8Address
            | Self::ValueAddress(_)
            | Self::ManagedAddress(_) => {
                return Err("managed address cannot be stored as an ordinary value".into());
            }
            Self::Bool => format!("__cil_bool_store({expression})"),
            _ => format!("unchecked(({})({expression}))", self.cs(image)?),
        })
    }
}
