// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::super::DependencyContext;
use super::PropertyAccessor;
use crate::{Result, metadata::identifier};
use cildec::{CodedIndex, Operand, TableId, Token};

impl DependencyContext {
    /// Prove a bounded source lookup, independently of metadata member resolution.
    /// Only unchanged original-this direct calls are eligible. A base qualifier
    /// preserves exact nonvirtual dispatch even when the accessor is virtual.
    pub(crate) fn property_presentation(
        &self,
        caller: u32,
        target: u32,
    ) -> Result<Option<(PropertyAccessor, &'static str)>> {
        let Some(property) = self.resolve_property_accessor(target)? else {
            return Ok(None);
        };
        let main = self.binding_image(0)?;
        let caller_row = main.row(caller)?;
        if caller_row.is_static() || !property.owner.exact_identity {
            return Err("property syntax needs an exact instance lookup".into());
        }
        let caller_owner = main
            .md
            .tables()
            .type_of_method(cildec::Rid::new(caller & 0xffffff))?
            .ok_or("property caller has no owner")?;
        let caller_owner = 0x02000000 | caller_owner.get();
        let body = main.body(caller)?.ok_or("property caller has no body")?;
        let instructions = body
            .instructions()
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if instructions.iter().any(|i| {
            matches!(i.opcode.canonical().name(), "starg" | "ldarga")
                && matches!(i.operand, Operand::Var(0))
        }) {
            return Err("property syntax needs unchanged original this".into());
        }
        let calls = instructions
            .iter()
            .filter(|i| matches!(i.operand, Operand::Method(t) if t.0 == target))
            .collect::<Vec<_>>();
        if calls.is_empty() || calls.iter().any(|i| i.opcode.canonical().name() != "call") {
            return Err("property syntax needs exact direct call dispatch".into());
        }
        if main.md.strings().str(caller_row.name)? == ".ctor" {
            let initialized = instructions.first().is_some_and(|i| {
                i.opcode.canonical().name() == "ldarg" && matches!(i.operand, Operand::Var(0))
            }) && instructions.get(1).is_some_and(|i| {
                i.opcode.canonical().name() == "call"
                    && match i.operand {
                        Operand::Method(t) => main
                            .method_ref(t, Default::default())
                            .is_ok_and(|m| m.constructor && m.args.is_empty()),
                        _ => false,
                    }
            });
            if !initialized {
                return Err("constructor property syntax needs prior initialization".into());
            }
        }
        identifier(&property.name)?;
        if crate::symbol_names::aliased(&property.name) {
            return Err("property alias is not a proven source name".into());
        }
        if property.owner.flags & 0x20 != 0 || !property.owner.generic_arguments.is_empty() {
            return Err("property syntax needs a nongeneric class owner".into());
        }
        let method = &property.method;
        let signature = crate::signature_guard::method(&method.definition_signature)?;
        let property_signature = crate::signature_guard::property(&property.signature)?;
        if !signature.has_this
            || method.flags & (0x10 | 0x400) != 0
            || !property_signature.params.is_empty()
            || signature.return_type.by_ref
            || !signature.return_type.custom_mods.is_empty()
            || signature
                .params
                .iter()
                .any(|p| p.by_ref || !p.custom_mods.is_empty())
            || method.name != format!("{}_{}", property.role, property.name)
        {
            return Err("property source accessor shape is not proven".into());
        }
        if !self.property_type_is_exact(method.module, &property_signature.return_type.type_)? {
            return Err("property source signature requires exact type identities".into());
        }
        let requested_signature = crate::signature_guard::method(&method.reference_signature)?;
        for (module, signature) in [(0, &requested_signature), (method.module, &signature)] {
            for parameter in signature
                .params
                .iter()
                .chain(std::iter::once(&signature.return_type))
            {
                if !self.property_type_is_exact(module, &parameter.type_)? {
                    return Err("property source signature requires exact type identities".into());
                }
            }
        }
        let caller_type = self.resolve_type(caller_owner)?;
        let qualifier = if method.module == 0 && property.owner.definition.token == caller_owner {
            if method.flags & 0x40 != 0 {
                return Err("direct virtual this accessor needs exact call notation".into());
            }
            "this"
        } else {
            let extends = main.md.tables().type_def(caller_owner & 0xffffff)?.extends;
            let base = self.resolve_type(extends.0)?;
            if !base.exact_identity || base.definition != property.owner.definition {
                return Err("property syntax target is not the immediate base definition".into());
            }
            "base"
        };
        let same_assembly = caller_type.definition.assembly == property.owner.definition.assembly;
        let accessible = match method.flags & 7 {
            4..=6 => true,
            2 | 3 => same_assembly,
            1 => qualifier == "this",
            _ => false,
        };
        if !accessible {
            return Err("property source accessor is not accessible".into());
        }
        let image = self.binding_image(method.module)?;
        let types = image.selected_types(&property.owner.name)?;
        let owner = types.first().ok_or("property source owner is missing")?;
        if !image
            .md
            .tables()
            .generic_params(Token(owner.token))?
            .is_empty()
        {
            return Err("open generic property syntax needs additional context".into());
        }
        let mut matching_properties = 0;
        for &token in &owner.properties {
            let row = image.md.tables().property(token & 0xffffff)?;
            if image.md.strings().str(row.name)? == property.name {
                matching_properties += 1;
            }
        }
        if matching_properties != 1 {
            return Err("property source lookup is ambiguous".into());
        }
        for &token in &owner.fields {
            if image
                .md
                .strings()
                .str(image.md.tables().field(token & 0xffffff)?.name)?
                == property.name
            {
                return Err("field hides the source property name".into());
            }
        }
        for &token in &owner.methods {
            if image.md.strings().str(image.row(token)?.name)? == property.name {
                return Err("method hides the source property name".into());
            }
        }
        for &token in &owner.events {
            if image
                .md
                .strings()
                .str(image.md.tables().event(token & 0xffffff)?.name)?
                == property.name
            {
                return Err("event hides the source property name".into());
            }
        }
        for (rid, row) in image.md.tables().iter::<cildec::tables::TypeDefRow>() {
            let row = row?;
            if image
                .md
                .tables()
                .enclosing_type(cildec::Rid::new(rid))?
                .is_some_and(|p| p.get() == owner.token & 0xffffff)
                && image.md.strings().str(row.type_name)?.split('`').next()
                    == Some(property.name.as_str())
            {
                return Err("nested type hides the source property name".into());
            }
        }
        let role = if property.role == "get" { 2 } else { 1 };
        let mut same_role = 0;
        for (rid, row) in image
            .md
            .tables()
            .iter::<cildec::tables::MethodSemanticsRow>()
        {
            let row = row?;
            let association = image.metadata_coded_token(
                CodedIndex::HasSemantics,
                image
                    .md
                    .tables()
                    .raw_column(TableId::MethodSemantics, rid, 2)?,
            )?;
            if association.0 == property.definition.token && row.semantics == role {
                same_role += 1;
            }
        }
        if same_role != 1 {
            return Err("source property accessor role is ambiguous".into());
        }
        Ok(Some((property, qualifier)))
    }

    /// Prove source lookup for a callvirt property on an external class.
    /// The receiver will be cast to this exact declaring owner before lookup.
    pub(crate) fn external_property_presentation(
        &self,
        token: u32,
    ) -> Result<Option<super::super::ExternalProperty>> {
        let Some(property) = self.resolve_property_accessor(token)? else {
            return Ok(None);
        };
        let method = &property.method;
        let image = self.binding_image(method.module)?;
        let owner_token = Token(property.owner.definition.token);
        if !property.owner.exact_identity
            || property.owner.flags & 0x20 != 0
            || !property.owner.generic_arguments.is_empty()
            || !image.md.tables().generic_params(owner_token)?.is_empty()
            || image.has_core_base(owner_token, "ValueType")?
            || image.has_core_base(owner_token, "Enum")?
        {
            return Err("external property needs an exact nongeneric reference-class owner".into());
        }
        let signature = crate::signature_guard::method(&method.definition_signature)?;
        let property_signature = crate::signature_guard::property(&property.signature)?;
        if !signature.has_this
            || method.flags & 0x10 != 0
            || method.flags & 7 != 6
            || !property_signature.params.is_empty()
            || signature.return_type.by_ref
            || !signature.return_type.custom_mods.is_empty()
            || signature
                .params
                .iter()
                .any(|p| p.by_ref || !p.custom_mods.is_empty())
            || method.name != format!("{}_{}", property.role, property.name)
        {
            return Err("external property needs a public non-indexed ordinary accessor".into());
        }
        identifier(&property.name)?;
        if crate::symbol_names::aliased(&property.name) {
            return Err("external property name is not a proven source identifier".into());
        }
        for (module, raw) in [
            (0, &method.reference_signature),
            (method.module, &method.definition_signature),
        ] {
            let signature = crate::signature_guard::method(raw)?;
            for parameter in signature
                .params
                .iter()
                .chain(std::iter::once(&signature.return_type))
            {
                if !self.property_type_is_exact(module, &parameter.type_)? {
                    return Err("external property signature needs exact supported types".into());
                }
            }
        }
        let types = image.selected_types(&property.owner.name)?;
        let owner = types.first().ok_or("external property owner is missing")?;
        if owner.token != owner_token.0 {
            return Err("external property owner identity changed".into());
        }
        let matching = owner
            .properties
            .iter()
            .filter(|&&token| {
                image
                    .md
                    .tables()
                    .property(token & 0xffffff)
                    .ok()
                    .and_then(|row| image.md.strings().str(row.name).ok())
                    == Some(property.name.as_str())
            })
            .count();
        if matching != 1 {
            return Err("external property source name is ambiguous".into());
        }
        for &token in &owner.fields {
            let row = image.md.tables().field(token & 0xffffff)?;
            if image.md.strings().str(row.name)? == property.name {
                return Err("field hides the external property name".into());
            }
        }
        for &token in &owner.methods {
            if image.md.strings().str(image.row(token)?.name)? == property.name {
                return Err("method hides the external property name".into());
            }
        }
        for &token in &owner.events {
            let row = image.md.tables().event(token & 0xffffff)?;
            if image.md.strings().str(row.name)? == property.name {
                return Err("event hides the external property name".into());
            }
        }
        for (rid, row) in image.md.tables().iter::<cildec::tables::TypeDefRow>() {
            let row = row?;
            if image
                .md
                .tables()
                .enclosing_type(cildec::Rid::new(rid))?
                .is_some_and(|parent| parent.get() == owner_token.rid())
                && image.md.strings().str(row.type_name)?.split('`').next()
                    == Some(property.name.as_str())
            {
                return Err("nested type hides the external property name".into());
            }
        }
        let role = if property.role == "get" { 2 } else { 1 };
        let mut same_role = 0;
        for (rid, row) in image
            .md
            .tables()
            .iter::<cildec::tables::MethodSemanticsRow>()
        {
            let row = row?;
            let association = image.metadata_coded_token(
                CodedIndex::HasSemantics,
                image
                    .md
                    .tables()
                    .raw_column(TableId::MethodSemantics, rid, 2)?,
            )?;
            if association.0 == property.definition.token && row.semantics == role {
                same_role += 1;
            }
        }
        if same_role != 1 {
            return Err("external property accessor role is ambiguous".into());
        }
        Ok(Some(super::super::ExternalProperty {
            owner_name: crate::symbol_names::type_name(&image, owner_token, true)?,
            name: crate::symbol_names::ident(&property.name),
            getter: property.role == "get",
        }))
    }

    fn property_type_is_exact(&self, module: usize, ty: &cildec::Type<'_>) -> Result<bool> {
        match ty {
            cildec::Type::Class(token) | cildec::Type::ValueType(token) => {
                let (module, bound) =
                    self.bind_type_in(module, *token, &mut std::collections::BTreeSet::new())?;
                if !bound.exact_identity {
                    return Err(format!("property signature used metadata-only type substitution: selected={} source={:?}; see --details for requested scopes; runtime behavior is not proven", bound.definition.assembly, bound.definition.source).into());
                }
                Ok(bound.generic_arguments.is_empty()
                    && self
                        .binding_image(module)?
                        .md
                        .tables()
                        .generic_params(Token(bound.definition.token))?
                        .is_empty())
            }
            cildec::Type::SzArray(modifiers, inner) => {
                if modifiers.is_empty() {
                    self.property_type_is_exact(module, inner)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(crate::ir::Ty::from_sig(ty).is_ok()),
        }
    }
}
