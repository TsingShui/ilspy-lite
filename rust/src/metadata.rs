// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{MethodInfo, Result};
use cildec::tables::{MethodDefRow, NestedClassRow, TypeDefRow};
use cildec::{Metadata, MethodBody, MethodSig, Names, PeImage, Rid, Strictness, TableId, Token};

type StructTypes = std::cell::RefCell<
    std::collections::BTreeMap<
        u32,
        std::result::Result<Option<crate::signature_types::TypeId>, String>,
    >,
>;

/// What a sealed value definition actually is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueDefinitionKind {
    Enum,
    Struct,
    Other,
}

/// Value types a method names, split by how they are admitted.
#[derive(Default)]
pub(crate) struct SignatureValueTypes {
    /// Plain `valuetype` identities, which need a storage fact.
    pub values: Vec<Token>,
    /// Constructed value types, with the argument count the site supplies.
    pub value_definitions: Vec<(Token, usize)>,
}

type InterfaceEdges =
    std::cell::OnceCell<std::result::Result<std::collections::BTreeMap<u32, Vec<u32>>, String>>;

mod method_instantiations;
mod reference_calls;

pub(crate) struct Image<'a> {
    pub pe: PeImage<'a>,
    pub md: Metadata<'a>,
    core_definition: std::cell::OnceCell<std::result::Result<bool, String>>,
    interface_edges: InterfaceEdges,
    location_types: std::cell::RefCell<crate::locations::LocationTypes>,
    struct_types: StructTypes,
    pub(crate) signature_types: std::cell::RefCell<crate::signature_types::SignatureTypes>,
    enum_types: std::cell::RefCell<
        std::collections::BTreeMap<u32, std::result::Result<crate::ir::EnumType, String>>,
    >,
    external_type_names: std::cell::RefCell<Vec<String>>,
    open_self: std::cell::RefCell<std::collections::BTreeMap<crate::signature_types::TypeId, u32>>,
    external_structs: std::cell::RefCell<
        std::collections::BTreeMap<
            u32,
            std::result::Result<crate::signature_types::TypeId, String>,
        >,
    >,
    external_value_defs:
        std::cell::RefCell<std::collections::BTreeMap<u32, std::result::Result<u32, String>>>,
    external_fields: std::cell::RefCell<
        std::collections::BTreeMap<
            u32,
            std::result::Result<crate::dependencies::ExternalField, String>,
        >,
    >,
    external_allocations: std::cell::RefCell<
        std::collections::BTreeMap<
            u32,
            std::result::Result<crate::dependencies::ExternalAllocation, String>,
        >,
    >,
    external_calls: std::cell::RefCell<
        std::collections::BTreeMap<
            u32,
            std::result::Result<crate::dependencies::ExternalCall, String>,
        >,
    >,
}

impl<'a> Image<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() > 512 * 1024 * 1024 {
            return Err("image exceeds 512 MiB limit".into());
        }
        let pe = PeImage::parse_with(bytes, Strictness::Strict)?;
        let md = pe.metadata()?;
        if !pe.diagnostics().is_empty() || !md.diagnostics().is_empty() {
            return Err("image/metadata parser reported diagnostics".into());
        }
        // Token identity must not be confused with logical pointer-table indices.
        if TableId::ALL
            .iter()
            .any(|t| t.is_ptr_table() && md.tables().row_count(*t) != 0)
        {
            return Err("metadata pointer tables are not supported".into());
        }
        Ok(Self {
            pe,
            md,
            core_definition: std::cell::OnceCell::new(),
            interface_edges: std::cell::OnceCell::new(),
            location_types: std::cell::RefCell::new(crate::locations::LocationTypes::new()?),
            struct_types: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            signature_types: std::cell::RefCell::new(crate::signature_types::SignatureTypes::new()?),
            enum_types: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            external_type_names: std::cell::RefCell::new(Vec::new()),
            open_self: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            external_structs: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            external_value_defs: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            external_fields: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            external_allocations: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            external_calls: std::cell::RefCell::new(std::collections::BTreeMap::new()),
        })
    }

    pub fn methods(&self) -> Result<Vec<MethodInfo>> {
        let names = Names::new(&self.md);
        self.md
            .tables()
            .iter::<MethodDefRow>()
            .map(|(rid, row)| {
                let row = row?;
                let owner = self
                    .md
                    .tables()
                    .type_of_method(Rid::new(rid))?
                    .ok_or_else(|| "method has no declaring type".to_string())?;
                Ok(MethodInfo {
                    token: 0x06000000 | rid,
                    name: names.method_def_full(Rid::new(rid))?,
                    declaring_type: names.type_def(owner)?,
                    simple_name: self.md.strings().str(row.name)?.to_string(),
                })
            })
            .collect()
    }

    pub fn selected_types(&self, name: &str) -> Result<Vec<SelectedType>> {
        let tables = self.md.tables();
        let rows = tables
            .iter::<TypeDefRow>()
            .map(|(_, row)| row.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        let method_limit = tables.row_count(TableId::MethodDef) + 1;
        let mut prior = 1;
        for row in &rows {
            let start = row.method_list.get();
            if start < prior || start > method_limit {
                return Err("invalid or overlapping type method ownership ranges".into());
            }
            prior = start;
        }
        let field_limit = tables.row_count(TableId::Field) + 1;
        let mut prior = 1;
        for row in &rows {
            let start = row.field_list.get();
            if start < prior || start > field_limit {
                return Err("invalid or overlapping type field ownership ranges".into());
            }
            prior = start;
        }
        let param_limit = tables.row_count(TableId::Param) + 1;
        let mut prior = 1;
        for (_, method) in tables.iter::<MethodDefRow>() {
            let start = method?.param_list.get();
            if start < prior || start > param_limit {
                return Err("invalid or overlapping method parameter ownership ranges".into());
            }
            prior = start;
        }
        let mut properties = vec![Vec::new(); rows.len()];
        let maps = tables
            .iter::<cildec::tables::PropertyMapRow>()
            .map(|(_, row)| row.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        let property_limit = tables.row_count(TableId::Property) + 1;
        if property_limit > 1 && maps.first().map(|m| m.property_list.get()) != Some(1) {
            return Err("unowned property metadata rows".into());
        }
        let mut owners = std::collections::BTreeSet::new();
        for (index, map) in maps.iter().enumerate() {
            let owner = map.parent.get();
            let start = map.property_list.get();
            let end = maps
                .get(index + 1)
                .map_or(property_limit, |m| m.property_list.get());
            if owner == 0
                || owner as usize > rows.len()
                || !owners.insert(owner)
                || start == 0
                || start > end
                || end > property_limit
            {
                return Err("invalid property ownership range".into());
            }
            properties[(owner - 1) as usize] = (start..end).map(|p| 0x17000000 | p).collect();
        }
        let mut events = vec![Vec::new(); rows.len()];
        let maps = tables
            .iter::<cildec::tables::EventMapRow>()
            .map(|(_, row)| row.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        let event_limit = tables.row_count(TableId::Event) + 1;
        if event_limit > 1 && maps.first().map(|m| m.event_list.get()) != Some(1) {
            return Err("unowned event metadata rows".into());
        }
        let mut owners = std::collections::BTreeSet::new();
        for (index, map) in maps.iter().enumerate() {
            let owner = map.parent.get();
            let start = map.event_list.get();
            let end = maps
                .get(index + 1)
                .map_or(event_limit, |m| m.event_list.get());
            if owner == 0
                || owner as usize > rows.len()
                || !owners.insert(owner)
                || start == 0
                || start > end
                || end > event_limit
            {
                return Err("invalid event ownership range".into());
            }
            events[(owner - 1) as usize] = (start..end).map(|e| 0x14000000 | e).collect();
        }
        let mut parents = vec![None; rows.len()];
        for (_, row) in tables.iter::<NestedClassRow>() {
            let row = row?;
            let nested = row.nested_class.get();
            let parent = row.enclosing_class.get();
            if nested == 0 || parent == 0 || parent as usize > rows.len() {
                return Err("invalid nested type ownership".into());
            }
            let slot = parents
                .get_mut((nested - 1) as usize)
                .ok_or("invalid nested type token")?;
            if slot.replace(parent).is_some() {
                return Err("duplicate nested type ownership".into());
            }
        }
        let mut names = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            if row.is_nested() != parents[index].is_some() {
                return Err("inconsistent nested type visibility/ownership".into());
            }
            let mut parts = Vec::new();
            let mut path = Vec::new();
            let mut current = Some(index as u32 + 1);
            while let Some(rid) = current {
                if path.len() == 64 || path.contains(&rid) {
                    return Err("cyclic or excessive nested type hierarchy".into());
                }
                path.push(rid);
                let row = rows[(rid - 1) as usize];
                let ns = self.md.strings().str(row.type_namespace)?;
                let local = self.md.strings().str(row.type_name)?;
                parts.push(if ns.is_empty() {
                    local.to_string()
                } else {
                    format!("{ns}.{local}")
                });
                current = parents[(rid - 1) as usize];
            }
            parts.reverse();
            names.push(parts.join("/"));
        }
        let mut matches = names.iter().enumerate().filter(|(_, n)| n.as_str() == name);
        let root = matches
            .next()
            .ok_or("no type matches the exact metadata name")?
            .0 as u32
            + 1;
        if matches.next().is_some() {
            return Err("ambiguous metadata type name".into());
        }
        let mut selected = Vec::new();
        let mut pending = vec![root];
        while let Some(rid) = pending.pop() {
            let row = rows[(rid - 1) as usize];
            let end = rows
                .get(rid as usize)
                .map_or(tables.row_count(TableId::MethodDef) + 1, |next| {
                    next.method_list.get()
                });
            let start = row.method_list.get();
            // cildec's display-oriented list_range clamps invalid list columns.
            // Selection must fail instead of silently losing malformed members.
            if start == 0 || start > end || end > tables.row_count(TableId::MethodDef) + 1 {
                return Err("invalid selected type method range".into());
            }
            selected.push(SelectedType {
                token: 0x02000000 | rid,
                name: names[(rid - 1) as usize].clone(),
                methods: (start..end).map(|m| 0x06000000 | m).collect(),
                properties: properties[(rid - 1) as usize].clone(),
                events: events[(rid - 1) as usize].clone(),
                fields: (row.field_list.get()
                    ..rows
                        .get(rid as usize)
                        .map_or(field_limit, |next| next.field_list.get()))
                    .map(|f| 0x04000000 | f)
                    .collect(),
            });
            for (index, parent) in parents.iter().enumerate().rev() {
                if *parent == Some(rid) {
                    pending.push(index as u32 + 1);
                }
            }
        }
        Ok(selected)
    }

    pub fn type_context(&self, token: u32) -> Result<(String, usize)> {
        let rid = token & 0xffffff;
        let row = self.md.tables().type_def(rid)?;
        let mut text = format!(
            ".type-flags 0x{:08X} {}\n",
            row.flags,
            type_flags(row.flags)
        );
        let parent = self.md.tables().enclosing_type(Rid::new(rid))?;
        let mut failures = 0;
        let mut relation = |kind: &str, target: Token| match self.type_reference(target, 0) {
            Ok(description) => text.push_str(&format!(".{kind} {description}\n")),
            Err(error) => {
                failures += 1;
                text.push_str(&format!(
                    ".{kind} 0x{:08X}\n// FAILED metadata: {}\n",
                    target.0,
                    escape_line(&error.to_string())
                ));
            }
        };
        if let Some(parent) = parent {
            relation("enclosing-type", Token(0x02000000 | parent.get()));
        }
        if row.extends.is_null() {
            text.push_str(".extends none\n");
        } else {
            relation("extends", row.extends);
        }
        for interface in self.md.tables().interface_impls(Rid::new(rid))? {
            let row = self.md.tables().interface_impl(interface)?;
            match self.type_reference(row.interface, 0) {
                Ok(description) => text.push_str(&format!(
                    ".implements {description} // InterfaceImpl 0x{:08X}\n",
                    0x09000000 | interface
                )),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        ".implements 0x{:08X} // InterfaceImpl 0x{:08X}\n// FAILED metadata: {}\n",
                        row.interface.0,
                        0x09000000 | interface,
                        escape_line(&error.to_string())
                    ));
                }
            }
        }
        let tables = self.md.tables();
        let owned_layouts = tables.rows_for_parent(TableId::ClassLayout, Token(token))?;
        if owned_layouts.len() > 1 {
            failures += 1;
            text.push_str("// FAILED metadata: duplicate class layout records\n");
        }
        for layout_rid in 1..=tables.row_count(TableId::ClassLayout) {
            let layout = tables.class_layout(layout_rid)?;
            let parent_rid = tables.raw_column(TableId::ClassLayout, layout_rid, 2)?;
            let valid_owner = parent_rid != 0
                && parent_rid <= Token::RID_MASK
                && parent_rid <= tables.row_count(TableId::TypeDef);
            if valid_owner && Token::new(TableId::TypeDef, parent_rid).0 != token {
                continue;
            }
            let raw = self
                .md
                .tables()
                .row_bytes(TableId::ClassLayout, layout_rid)?;
            let parent = if valid_owner {
                format!(
                    "parent=0x{:08X}",
                    Token::new(TableId::TypeDef, parent_rid).0
                )
            } else {
                format!("parent-rid=0x{parent_rid:08X}")
            };
            text.push_str(&format!(
                ".class-layout 0x{:08X} {parent} packing={} size={} raw-row-bytes={raw:02X?}\n",
                0x0f000000 | layout_rid,
                layout.packing_size,
                layout.class_size
            ));
            text.push_str("// size=0 means runtime-computed; packing=0 means default; full target layout/ancestry validation is not performed.\n");
            if !valid_owner {
                failures += 1;
                text.push_str("// FAILED metadata: ownership=unowned-invalid; ClassLayout owner is null/out-of-range; record is image context, not attached to this type\n");
            } else if row.flags & 0x20 != 0
                || !matches!(row.layout_kind(), 1 | 2)
                || !matches!(layout.packing_size, 0 | 1 | 2 | 4 | 8 | 16 | 32 | 64 | 128)
            {
                failures += 1;
                text.push_str("// FAILED metadata: invalid class layout flags or packing\n");
            }
        }
        match self.generic_metadata(token, None) {
            Ok((generics, errors)) => {
                text.push_str(&generics);
                failures += errors;
            }
            Err(error) => {
                failures += 1;
                text.push_str(&format!(
                    "// FAILED metadata: {}\n",
                    escape_line(&error.to_string())
                ));
            }
        }
        Ok((text, failures))
    }

    pub(crate) fn metadata_coded_token(
        &self,
        kind: cildec::CodedIndex,
        encoded: u32,
    ) -> Result<Token> {
        if encoded >> kind.tag_bits() > Token::RID_MASK {
            return Err("coded-index RID exceeds metadata token width".into());
        }
        Ok(kind.decode(encoded, 0)?)
    }

    fn metadata_token_exists(&self, token: Token) -> bool {
        token.rid() != 0
            && token
                .table()
                .is_some_and(|table| token.rid() <= self.md.tables().row_count(table))
    }

    /// The selected type's member and relation tokens that can own attached records.
    fn attached_parents(&self, ty: &SelectedType) -> Result<std::collections::BTreeSet<u32>> {
        let tables = self.md.tables();
        let mut parents = std::collections::BTreeSet::from([ty.token]);
        parents.extend(
            ty.fields
                .iter()
                .chain(&ty.methods)
                .chain(&ty.properties)
                .chain(&ty.events)
                .copied(),
        );
        // Image-level attributes affect every type; retain them as contextual evidence.
        for table in [TableId::Module, TableId::Assembly] {
            for rid in 1..=tables.row_count(table) {
                parents.insert(Token::new(table, rid).0);
            }
        }
        for &method in &ty.methods {
            let row = tables.method_def(method & 0xffffff)?;
            let end = if method & 0xffffff == tables.row_count(TableId::MethodDef) {
                tables.row_count(TableId::Param) + 1
            } else {
                tables.method_def((method & 0xffffff) + 1)?.param_list.get()
            };
            parents.extend((row.param_list.get()..end).map(|rid| 0x08000000 | rid));
        }
        let owners = std::iter::once(ty.token)
            .chain(ty.methods.iter().copied())
            .collect::<Vec<_>>();
        for owner in owners {
            for rid in tables.generic_params(Token(owner))? {
                parents.insert(0x2a000000 | rid);
                parents.extend(
                    tables
                        .generic_param_constraints(Rid::new(rid))?
                        .into_iter()
                        .map(|rid| 0x2c000000 | rid),
                );
            }
        }
        parents.extend(
            tables
                .interface_impls(Rid::new(ty.token & 0xffffff))?
                .into_iter()
                .map(|rid| 0x09000000 | rid),
        );
        // DeclSecurity rows can themselves carry custom attributes.
        for owner in std::iter::once(ty.token)
            .chain(ty.methods.iter().copied())
            .chain((1..=tables.row_count(TableId::Assembly)).map(|rid| 0x20000000 | rid))
        {
            parents.extend(
                tables
                    .rows_for_parent(TableId::DeclSecurity, Token(owner))?
                    .into_iter()
                    .map(|rid| 0x0e000000 | rid),
            );
        }
        Ok(parents)
    }

    pub fn marshalling(&self, ty: &SelectedType) -> Result<(String, usize)> {
        let tables = self.md.tables();
        let parents = self.attached_parents(ty)?;
        let mut counts = std::collections::BTreeMap::<u32, usize>::new();
        let mut text = String::new();
        let mut failures = 0;
        for rid in 1..=tables.row_count(TableId::FieldMarshal) {
            let encoded = tables.raw_column(TableId::FieldMarshal, rid, 0)?;
            let parent = self.metadata_coded_token(cildec::CodedIndex::HasFieldMarshal, encoded);
            if parent
                .as_ref()
                .is_ok_and(|p| self.metadata_token_exists(*p) && !parents.contains(&p.0))
            {
                continue;
            }
            let blob = tables.raw_column(TableId::FieldMarshal, rid, 1)?;
            let raw = tables.row_bytes(TableId::FieldMarshal, rid)?;
            text.push_str(&format!(".field-marshal 0x{:08X} ", 0x0d000000 | rid));
            match parent {
                Ok(parent) => {
                    text.push_str(&format!("parent=0x{:08X} ", parent.0));
                    if self.metadata_token_exists(parent) {
                        *counts.entry(parent.0).or_default() += 1;
                    } else {
                        failures += 1;
                        text.push_str("// FAILED metadata: ownership=unowned-invalid; null/out-of-range marshal parent; record is image context ");
                    }
                }
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!("parent-encoded=0x{encoded:08X} // FAILED metadata: ownership=unowned-invalid; {} ", escape_line(&error.to_string())));
                }
            }
            text.push_str(&format!("native-type-blob-index=0x{blob:08X} "));
            match self.md.blobs().get(cildec::BlobIndex(blob)) {
                Ok(bytes) => text.push_str(&format!("bytes={bytes:02X?} ")),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "// FAILED metadata: {} ",
                        escape_line(&error.to_string())
                    ));
                }
            }
            text.push_str(&format!("raw-row-bytes={raw:02X?} payload=uninterpreted; interop marshaling is runtime-owned; native conversions, allocation and cleanup are not recovered body operations\n"));
        }
        for parent in parents {
            let token = Token(parent);
            let flag = match token.table() {
                Some(TableId::Field) => tables.field(token.rid())?.flags & 0x1000 != 0,
                Some(TableId::Param) => tables.param(token.rid())?.flags & 0x2000 != 0,
                _ => continue,
            };
            let count = counts.get(&parent).copied().unwrap_or(0);
            if count > 1 || flag != (count != 0) {
                failures += 1;
                text.push_str(&format!("// FAILED metadata: inconsistent marshal association parent=0x{parent:08X} flag={flag} records={count}\n"));
            }
        }
        Ok((text, failures))
    }

    pub fn method_implementations(&self, ty: &SelectedType) -> Result<(String, usize)> {
        let tables = self.md.tables();
        let mut text = String::new();
        let mut failures = 0;
        for rid in 1..=tables.row_count(TableId::MethodImpl) {
            let class = tables.raw_column(TableId::MethodImpl, rid, 0)?;
            let valid_owner = class != 0
                && class <= Token::RID_MASK
                && class <= tables.row_count(TableId::TypeDef);
            if valid_owner && Token::new(TableId::TypeDef, class).0 != ty.token {
                continue;
            }
            let body = tables.raw_column(TableId::MethodImpl, rid, 1)?;
            let declaration = tables.raw_column(TableId::MethodImpl, rid, 2)?;
            let raw = tables.row_bytes(TableId::MethodImpl, rid)?;
            // Decode endpoints independently; an invalid declaration cannot conceal
            // the implementing body or turn it into an absent mapping.
            let body = self.metadata_coded_token(cildec::CodedIndex::MethodDefOrRef, body);
            let declaration =
                self.metadata_coded_token(cildec::CodedIndex::MethodDefOrRef, declaration);
            let owner = if valid_owner {
                format!("class=0x{:08X}", Token::new(TableId::TypeDef, class).0)
            } else {
                format!("class-rid=0x{class:08X}")
            };
            text.push_str(&format!(".method-impl 0x{:08X} {owner} ", 0x19000000 | rid));
            for (kind, token, column) in [("body", &body, 1), ("declaration", &declaration, 2)] {
                match token {
                    Ok(token) => text.push_str(&format!("{kind}=0x{:08X} ", token.0)),
                    Err(_) => text.push_str(&format!(
                        "{kind}-encoded=0x{:08X} ",
                        tables.raw_column(TableId::MethodImpl, rid, column)?
                    )),
                }
            }
            text.push_str(&format!("raw-row-bytes={raw:02X?}; explicit metadata mapping, not a name-based override guess; dispatch/access/ancestry validation is not performed\n"));
            if !valid_owner {
                failures += 1;
                text.push_str("// FAILED metadata: ownership=unowned-invalid; MethodImpl owner is null/out-of-range; record is image context, not attached to this type\n");
            }
            for (kind, token) in [("body", body), ("declaration", declaration)] {
                match token.and_then(|token| self.metadata_method_identity(token)) {
                    Ok((identity, errors)) => {
                        failures += errors;
                        text.push_str(&format!(".method-impl-{kind} {identity}\n"));
                    }
                    Err(error) => {
                        failures += 1;
                        text.push_str(&format!(
                            "// FAILED metadata MethodImpl {kind}: {}\n",
                            escape_line(&error.to_string())
                        ));
                    }
                }
            }
        }
        Ok((text, failures))
    }

    pub fn security_declarations(&self, ty: &SelectedType) -> Result<(String, usize)> {
        let tables = self.md.tables();
        let parents = std::iter::once(ty.token)
            .chain(ty.methods.iter().copied())
            .chain((1..=tables.row_count(TableId::Assembly)).map(|rid| 0x20000000 | rid))
            .collect::<std::collections::BTreeSet<_>>();
        let mut text = String::new();
        let mut failures = 0;
        for rid in 1..=tables.row_count(TableId::DeclSecurity) {
            let encoded = tables.raw_column(TableId::DeclSecurity, rid, 1)?;
            let parent = self.metadata_coded_token(cildec::CodedIndex::HasDeclSecurity, encoded);
            if parent
                .as_ref()
                .is_ok_and(|p| self.metadata_token_exists(*p) && !parents.contains(&p.0))
            {
                continue;
            }
            let action = tables.raw_column(TableId::DeclSecurity, rid, 0)?;
            let blob = tables.raw_column(TableId::DeclSecurity, rid, 2)?;
            let raw = tables.row_bytes(TableId::DeclSecurity, rid)?;
            text.push_str(&format!(".decl-security 0x{:08X} ", 0x0e000000 | rid));
            match parent {
                Ok(parent) => {
                    text.push_str(&format!("parent=0x{:08X} ", parent.0));
                    if !self.metadata_token_exists(parent) {
                        failures += 1;
                        text.push_str("// FAILED metadata: ownership=unowned-invalid; null/out-of-range parent; record is image context ");
                    }
                }
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "parent-encoded=0x{encoded:08X} // FAILED metadata: ownership=unowned-invalid; {} ",
                        escape_line(&error.to_string())
                    ));
                }
            }
            text.push_str(&format!(
                "action=0x{action:04X} permission-set-blob-index=0x{blob:08X} "
            ));
            match self.md.blobs().get(cildec::BlobIndex(blob)) {
                Ok(bytes) => text.push_str(&format!("permission-set-bytes={bytes:02X?} ")),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "// FAILED metadata: {} ",
                        escape_line(&error.to_string())
                    ));
                }
            }
            text.push_str(&format!("raw-row-bytes={raw:02X?} payload=uninterpreted; permission-set format/action policy are not interpreted; security enforcement is runtime-owned, not a property of the recovered body\n"));
        }
        Ok((text, failures))
    }

    pub fn custom_attributes(&self, ty: &SelectedType) -> Result<(String, usize)> {
        let parents = self.attached_parents(ty)?;
        let tables = self.md.tables();
        let mut text = String::new();
        let mut failures = 0;
        // Raw parent/constructor columns are independent: one broken constructor must
        // not hide its parent, payload or later attached records.
        for rid in 1..=tables.row_count(TableId::CustomAttribute) {
            let raw_parent = tables.raw_column(TableId::CustomAttribute, rid, 0)?;
            let parent =
                self.metadata_coded_token(cildec::CodedIndex::HasCustomAttribute, raw_parent);
            if parent.as_ref().is_ok_and(|parent| {
                self.metadata_token_exists(*parent) && !parents.contains(&parent.0)
            }) {
                continue;
            }
            let raw = tables.row_bytes(TableId::CustomAttribute, rid)?;
            let constructor = tables.raw_column(TableId::CustomAttribute, rid, 1)?;
            let blob = tables.raw_column(TableId::CustomAttribute, rid, 2)?;
            text.push_str(&format!(".custom-attribute 0x{:08X} ", 0x0c000000 | rid));
            match parent {
                Ok(parent) => {
                    text.push_str(&format!("parent=0x{:08X} ", parent.0));
                    if !self.metadata_token_exists(parent) {
                        failures += 1;
                        text.push_str("// FAILED metadata: ownership=unowned-invalid; null/out-of-range parent; record is image context ");
                    }
                }
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "parent-encoded=0x{raw_parent:08X} // FAILED metadata: ownership=unowned-invalid; {} ",
                        escape_line(&error.to_string())
                    ));
                }
            }
            let constructor =
                self.metadata_coded_token(cildec::CodedIndex::CustomAttributeType, constructor);
            match &constructor {
                Ok(token) => text.push_str(&format!("constructor=0x{:08X} ", token.0)),
                Err(_) => text.push_str(&format!(
                    "constructor-encoded=0x{:08X} ",
                    tables.raw_column(TableId::CustomAttribute, rid, 1)?
                )),
            }
            text.push_str(&format!("value-blob-index=0x{blob:08X} "));
            match self.md.blobs().get(cildec::BlobIndex(blob)) {
                Ok(bytes) => text.push_str(&format!("value-bytes={bytes:02X?} ")),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "// FAILED metadata: {} ",
                        escape_line(&error.to_string())
                    ));
                }
            }
            text.push_str(&format!("raw-row-bytes={raw:02X?} payload=uninterpreted; attribute arguments and runtime effects are not decoded or executed\n"));
            match constructor.and_then(|token| self.metadata_method_identity(token)) {
                Ok((identity, errors)) => {
                    failures += errors;
                    text.push_str(&format!(".attribute-constructor {identity}\n"));
                }
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "// FAILED metadata: {}\n",
                        escape_line(&error.to_string())
                    ));
                }
            }
        }
        Ok((text, failures))
    }

    /// Metadata identity only: no lowering admission, dependency loading or execution.
    fn metadata_method_identity(&self, token: Token) -> Result<(String, usize)> {
        let tables = self.md.tables();
        let table = token.table().ok_or("invalid metadata endpoint table")?;
        if !matches!(table, TableId::MethodDef | TableId::MemberRef) || token.rid() == 0 {
            return Err("metadata method reference needs MethodDef or MemberRef".into());
        }
        let raw = tables.row_bytes(table, token.rid())?;
        let (name_index, blob_index, owner, owner_evidence) = if table == TableId::MethodDef {
            let owner: Result<Token> = tables
                .type_of_method(Rid::new(token.rid()))
                .map_err(Into::into)
                .and_then(|owner| {
                    owner
                        .map(|rid| Token::new(TableId::TypeDef, rid.get()))
                        .ok_or_else(|| "missing metadata method owner".into())
                });
            (
                tables.raw_column(table, token.rid(), 3)?,
                tables.raw_column(table, token.rid(), 4)?,
                owner,
                format!(
                    "method-owner-derived-from-TypeDef-lists; param-list-index=0x{:08X}",
                    tables.raw_column(table, token.rid(), 5)?
                ),
            )
        } else {
            let encoded = tables.raw_column(table, token.rid(), 0)?;
            (
                tables.raw_column(table, token.rid(), 1)?,
                tables.raw_column(table, token.rid(), 2)?,
                self.metadata_coded_token(cildec::CodedIndex::MemberRefParent, encoded),
                format!("owner-encoded=0x{encoded:08X}"),
            )
        };
        let mut failures = 0;
        let mut errors = Vec::new();
        let name = match self.md.strings().str(cildec::StringIndex(name_index)) {
            Ok(name) => escape_line(name),
            Err(error) => {
                failures += 1;
                errors.push(format!("name: {error}"));
                "<unreadable-name>".into()
            }
        };
        let (owner_name, owner_description) = match owner {
            Ok(owner) => {
                let owner_name = match Names::new(&self.md).try_token(owner) {
                    Ok(name) => escape_line(&name),
                    Err(error) => {
                        failures += 1;
                        errors.push(format!("owner name: {error}"));
                        "<unreadable-owner>".into()
                    }
                };
                let description = match self.type_reference(owner, 0) {
                    Ok(description) => description,
                    Err(error) => {
                        failures += 1;
                        errors.push(format!("owner: {error}"));
                        format!("0x{:08X}", owner.0)
                    }
                };
                (owner_name, description)
            }
            Err(error) => {
                failures += 1;
                errors.push(format!("owner: {error}"));
                ("<unreadable-owner>".into(), "unresolved".into())
            }
        };
        let mut text = format!(
            "0x{:08X} {owner_name}::{name} owner={owner_description} name-index=0x{name_index:08X} {owner_evidence} signature-blob-index=0x{blob_index:08X} raw-row-bytes={raw:02X?}",
            token.0
        );
        // Keep row and blob evidence before parsing. A malformed signature or named
        // reference cannot erase the independently readable endpoint identity.
        match self.md.blobs().get(cildec::BlobIndex(blob_index)) {
            Ok(bytes) => {
                text.push_str(&format!(" signature-bytes={bytes:02X?}"));
                match MethodSig::parse(bytes) {
                    Ok(signature) => {
                        text.push_str(&format!(" parsed={signature:?}"));
                        let mut references = Vec::new();
                        for parameter in std::iter::once(&signature.return_type)
                            .chain(&signature.params)
                            .chain(&signature.vararg_params)
                        {
                            if let Err(error) =
                                self.signature_references(&parameter.type_, 0, &mut references)
                            {
                                failures += 1;
                                errors.push(format!("signature reference: {error}"));
                            }
                        }
                        text.push_str(&format!(" references=[{}]", references.join("; ")));
                    }
                    Err(error) => {
                        failures += 1;
                        errors.push(format!("signature: {error}"));
                    }
                }
            }
            Err(error) => {
                failures += 1;
                errors.push(format!("signature blob: {error}"));
            }
        }
        for error in errors {
            text.push_str(&format!(" // FAILED metadata: {}", escape_line(&error)));
        }
        Ok((text, failures))
    }

    fn type_reference(&self, token: Token, depth: usize) -> Result<String> {
        self.render_type_reference(token, depth).map_err(|error| {
            format!(
                "{} // type reference interpretation failed: {error}",
                self.raw_reference_evidence(token, depth)
            )
            .into()
        })
    }

    /// Failure evidence is independent of display-name and signature interpretation.
    /// It never turns an unreadable identity into an admitted lowering type.
    fn raw_reference_evidence(&self, token: Token, depth: usize) -> String {
        let mut text = format!("type-reference-evidence=0x{:08X} ", token.0);
        if depth >= 32 {
            text.push_str("reference-depth-limit; identity retained without recursion");
            return text;
        }
        let tables = self.md.tables();
        let Some(table) = token.table() else {
            return format!("{text}invalid-table");
        };
        let raw = match tables.row_bytes(table, token.rid()) {
            Ok(raw) => raw,
            Err(error) => {
                return format!("{text}row-unavailable: {}", escape_line(&error.to_string()));
            }
        };
        text.push_str(&format!("raw-row-bytes={raw:02X?} "));
        let mut string =
            |label: &str, index: cildec::StringIndex| match self.md.strings().str(index) {
                Ok(value) => text.push_str(&format!("{label}={} ", escape_line(value))),
                Err(error) => text.push_str(&format!(
                    "{label}-index=0x{:08X} unreadable({}) ",
                    index.0,
                    escape_line(&error.to_string())
                )),
            };
        match table {
            TableId::TypeRef => {
                // Decode the scope only after independently preserving the raw row
                // and both heap indexes; bad scope decoding cannot hide the name.
                let name = tables.raw_column(table, token.rid(), 1).unwrap();
                let ns = tables.raw_column(table, token.rid(), 2).unwrap();
                string("name", cildec::StringIndex(name));
                string("namespace", cildec::StringIndex(ns));
                let encoded = tables.raw_column(table, token.rid(), 0).unwrap();
                text.push_str(&format!("resolution-scope-encoded=0x{encoded:08X} "));
                match self.metadata_coded_token(cildec::CodedIndex::ResolutionScope, encoded) {
                    Ok(scope) => {
                        text.push_str(&format!("resolution-scope=0x{:08X} ", scope.0));
                        if !scope.is_null() {
                            text.push_str(&self.raw_reference_evidence(scope, depth + 1));
                        } else {
                            text.push_str("unresolved-scope; no dependency guessed");
                        }
                    }
                    Err(error) => text.push_str(&format!(
                        "scope-unreadable: {}",
                        escape_line(&error.to_string())
                    )),
                }
            }
            TableId::TypeDef => {
                string(
                    "name",
                    cildec::StringIndex(tables.raw_column(table, token.rid(), 1).unwrap()),
                );
                string(
                    "namespace",
                    cildec::StringIndex(tables.raw_column(table, token.rid(), 2).unwrap()),
                );
                text.push_str(&format!(
                    "flags=0x{:08X} extends-encoded=0x{:08X} field-list={} method-list={} ",
                    tables.raw_column(table, token.rid(), 0).unwrap(),
                    tables.raw_column(table, token.rid(), 3).unwrap(),
                    tables.raw_column(table, token.rid(), 4).unwrap(),
                    tables.raw_column(table, token.rid(), 5).unwrap()
                ));
                if let Ok(Some(owner)) = tables.enclosing_type(Rid::new(token.rid())) {
                    text.push_str(&self.raw_reference_evidence(
                        Token::new(TableId::TypeDef, owner.get()),
                        depth + 1,
                    ));
                }
            }
            TableId::AssemblyRef => {
                let row = tables.assembly_ref(token.rid()).unwrap();
                string("name", row.name);
                string("culture", row.culture);
                text.push_str(&format!(
                    "assembly-ref=0x{:08X} version={}.{}.{}.{} flags=0x{:08X} ",
                    token.0,
                    row.major_version,
                    row.minor_version,
                    row.build_number,
                    row.revision_number,
                    row.flags
                ));
                for (label, index) in [
                    ("key-or-token", row.public_key_or_token),
                    ("hash", row.hash_value),
                ] {
                    match self.md.blobs().get(index) {
                        Ok(bytes) => text.push_str(&format!("{label}={bytes:02X?} ")),
                        Err(error) => text.push_str(&format!(
                            "{label}-blob-index=0x{:08X} unreadable({}) ",
                            index.0,
                            escape_line(&error.to_string())
                        )),
                    }
                }
            }
            TableId::TypeSpec => {
                let index = tables.raw_column(table, token.rid(), 0).unwrap();
                text.push_str(&format!("signature-blob-index=0x{index:08X} "));
                match self.md.blobs().get(cildec::BlobIndex(index)) {
                    Ok(bytes) => text.push_str(&format!("signature-bytes={bytes:02X?}")),
                    Err(error) => text.push_str(&format!(
                        "signature-unreadable: {}",
                        escape_line(&error.to_string())
                    )),
                }
            }
            TableId::ModuleRef => string(
                "name",
                cildec::StringIndex(tables.raw_column(table, token.rid(), 0).unwrap()),
            ),
            TableId::Module => {
                string(
                    "name",
                    cildec::StringIndex(tables.raw_column(table, token.rid(), 1).unwrap()),
                );
                text.push_str(&format!(
                    "generation={} mvid-index={} enc-id-index={} enc-base-id-index={}",
                    tables.raw_column(table, token.rid(), 0).unwrap(),
                    tables.raw_column(table, token.rid(), 2).unwrap(),
                    tables.raw_column(table, token.rid(), 3).unwrap(),
                    tables.raw_column(table, token.rid(), 4).unwrap()
                ));
            }
            _ => text.push_str("unexpected-type-reference-table; raw row retained"),
        }
        text
    }

    fn render_type_reference(&self, token: Token, depth: usize) -> Result<String> {
        if depth >= 32 || token.rid() == 0 {
            return Err("invalid or excessive type reference".into());
        }
        let tables = self.md.tables();
        let names = Names::new(&self.md);
        let description = match token.table() {
            Some(TableId::TypeDef) => {
                let row = tables.type_def(token.rid())?;
                self.md.strings().str(row.type_name)?;
                self.md.strings().str(row.type_namespace)?;
                names.type_def(Rid::new(token.rid()))?
            }
            Some(TableId::TypeRef) => {
                let row = tables.type_ref(token.rid())?;
                let name = self.md.strings().str(row.type_name)?;
                let ns = self.md.strings().str(row.type_namespace)?;
                let local = if ns.is_empty() {
                    name.into()
                } else {
                    format!("{ns}.{name}")
                };
                let scope = row.resolution_scope;
                match scope.table() {
                    Some(TableId::AssemblyRef) if scope.rid() != 0 => {
                        let a = tables.assembly_ref(scope.rid())?;
                        format!(
                            "[{}]{local} // assembly-ref 0x{:08X} version={}.{}.{}.{} flags=0x{:08X} culture={:?} key-or-token={:02X?}",
                            self.md.strings().str(a.name)?,
                            scope.0,
                            a.major_version,
                            a.minor_version,
                            a.build_number,
                            a.revision_number,
                            a.flags,
                            self.md.strings().str(a.culture)?,
                            self.md.blobs().get(a.public_key_or_token)?
                        )
                    }
                    Some(TableId::TypeRef) if scope.rid() != 0 => {
                        format!(
                            "{local} // enclosing-reference {}",
                            self.type_reference(scope, depth + 1)?
                        )
                    }
                    Some(TableId::ModuleRef) if scope.rid() != 0 => {
                        let module = tables.module_ref(scope.rid())?;
                        format!(
                            "{local} // module-ref 0x{:08X} {:?}",
                            scope.0,
                            self.md.strings().str(module.name)?
                        )
                    }
                    Some(TableId::Module) if scope.rid() != 0 => {
                        let module = tables.module(scope.rid())?;
                        format!(
                            "{local} // module 0x{:08X} {:?}",
                            scope.0,
                            self.md.strings().str(module.name)?
                        )
                    }
                    _ if scope.is_null() => {
                        format!("{local} // unresolved scope 0x{:08X}", scope.0)
                    }
                    _ => return Err("invalid type reference scope".into()),
                }
            }
            Some(TableId::TypeSpec) => {
                let row = tables.type_spec(token.rid())?;
                let blob = self.md.blobs().get(row.signature)?;
                let signature = cildec::TypeSpecSig::parse(blob)
                    .map_err(|error| format!("signature-bytes={blob:02X?}: {error}"))?;
                let mut references = Vec::new();
                self.signature_references(&signature.type_, depth + 1, &mut references)
                    .map_err(|error| format!("signature-bytes={blob:02X?}: {error}"))?;
                format!(
                    "{} // signature-bytes={blob:02X?} parsed={:?} references=[{}]",
                    names.type_(&signature.type_),
                    signature.type_,
                    references.join("; ")
                )
            }
            _ => return Err("expected TypeDef, TypeRef or TypeSpec".into()),
        };
        Ok(format!("0x{:08X} {}", token.0, escape_line(&description)))
    }

    fn signature_references(
        &self,
        ty: &cildec::Type<'_>,
        depth: usize,
        references: &mut Vec<String>,
    ) -> Result<()> {
        use cildec::Type as T;
        if depth >= 32 {
            return Err("excessive type signature reference depth".into());
        }
        match ty {
            T::Class(token) | T::ValueType(token) => {
                references.push(self.type_reference(*token, depth + 1)?);
            }
            T::GenericInst { def, args, .. } => {
                references.push(self.type_reference(*def, depth + 1)?);
                for arg in args {
                    self.signature_references(arg, depth + 1, references)?;
                }
            }
            T::Ptr(mods, inner) | T::SzArray(mods, inner) | T::Modified(mods, inner) => {
                for modifier in mods {
                    references.push(self.type_reference(modifier.type_token, depth + 1)?);
                }
                self.signature_references(inner, depth + 1, references)?;
            }
            T::ByRef(inner) | T::Array(inner, _) | T::Pinned(inner) => {
                self.signature_references(inner, depth + 1, references)?;
            }
            T::FnPtr(sig) => {
                for param in std::iter::once(&sig.return_type)
                    .chain(&sig.params)
                    .chain(&sig.vararg_params)
                {
                    for modifier in &param.custom_mods {
                        references.push(self.type_reference(modifier.type_token, depth + 1)?);
                    }
                    self.signature_references(&param.type_, depth + 1, references)?;
                }
            }
            // Primitive and generic-parameter types contain no metadata token.
            _ => {}
        }
        Ok(())
    }

    pub fn field_declaration(&self, token: u32) -> Result<(String, usize)> {
        let row = self.md.tables().field(token & 0xffffff)?;
        let mut failures = 0;
        let mut name_diagnostic = String::new();
        let (alias, name_evidence) = match self.md.strings().str(row.name) {
            Ok(name) => (
                field_name(token, name),
                format!("name={}", escape_line(name)),
            ),
            Err(error) => {
                failures += 1;
                name_diagnostic = format!(
                    "// FAILED metadata field name: {}\n",
                    escape_line(&error.to_string())
                );
                (
                    format!("__il_field_{token:08X}"),
                    format!("name-index=0x{:08X}", row.name.get()),
                )
            }
        };
        let flags = field_flags(row.flags);
        let blob = self.md.blobs().get(row.signature);
        let raw_signature = match &blob {
            Ok(bytes) => format!("signature-bytes={bytes:02X?}"),
            Err(_) => format!("signature-blob-index=0x{:08X}", row.signature.get()),
        };
        let signature = (|| -> Result<String> {
            let sig = cildec::FieldSig::parse(blob?)?;
            let mut references = Vec::new();
            self.signature_references(&sig.type_, 0, &mut references)?;
            for modifier in &sig.custom_mods {
                references.push(self.type_reference(modifier.type_token, 0)?);
            }
            Ok(format!(
                "{} {alias} // {name_evidence} {raw_signature} parsed={:?} references=[{}]",
                Names::new(&self.md).field_sig(&sig),
                sig,
                references.join("; ")
            ))
        })();
        let mut text = match signature {
            Ok(signature) => format!(
                ".field 0x{token:08X} flags=0x{:04X} {flags} {signature}\n",
                row.flags
            ),
            Err(error) => {
                failures += 1;
                format!(
                    ".field 0x{token:08X} flags=0x{:04X} {flags} <unresolved-signature> {alias} // {name_evidence} {raw_signature}\n// FAILED metadata: {}\n",
                    row.flags,
                    escape_line(&error.to_string())
                )
            }
        };
        text.push_str(&name_diagnostic);
        let (constants, errors) = self.constant_metadata(token, row.flags & 0x8000 != 0)?;
        text.push_str(&constants);
        failures += errors;
        for rid in self
            .md
            .tables()
            .rows_for_parent(TableId::FieldLayout, Token(token))?
        {
            let layout = self
                .md
                .tables()
                .row::<cildec::tables::FieldLayoutRow>(rid)?;
            text.push_str(&format!(
                ".field-layout 0x{:08X} parent=0x{token:08X} offset={}\n",
                0x10000000 | rid,
                layout.offset
            ));
        }
        let initial_data = self
            .md
            .tables()
            .rows_for_parent(TableId::FieldRva, Token(token))?;
        if initial_data.len() > 1 || (row.flags & 0x100 != 0) != !initial_data.is_empty() {
            failures += 1;
            text.push_str("// FAILED metadata: inconsistent field RVA association\n");
        }
        for rid in initial_data {
            let data = self.md.tables().row::<cildec::tables::FieldRvaRow>(rid)?;
            text.push_str(&format!(
                ".field-rva 0x{:08X} parent=0x{token:08X} rva=0x{:08X} ",
                0x1d000000 | rid,
                data.rva
            ));
            let extent = self.field_data_size(row.signature);
            match extent.and_then(|size| {
                size.map(|size| self.pe.rva_slice(data.rva, size).map_err(Into::into))
                    .transpose()
            }) {
                Ok(Some(bytes)) => text.push_str(&format!("bytes={bytes:02X?}\n")),
                Ok(None) => text.push_str("// data extent unresolved; no bytes guessed\n"),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "\n// FAILED metadata: {}\n",
                        escape_line(&error.to_string())
                    ));
                }
            }
        }
        text.push('\n');
        Ok((text, failures))
    }

    fn field_data_size(&self, signature: cildec::BlobIndex) -> Result<Option<usize>> {
        use cildec::Type as T;
        let sig = cildec::FieldSig::parse(self.md.blobs().get(signature)?)?;
        Ok(match sig.type_ {
            T::Boolean | T::I1 | T::U1 => Some(1),
            T::Char | T::I2 | T::U2 => Some(2),
            T::I4 | T::U4 | T::R4 => Some(4),
            T::I8 | T::U8 | T::R8 => Some(8),
            T::ValueType(token) if token.table() == Some(TableId::TypeDef) => {
                let row = self.md.tables().type_def(token.rid())?;
                // Only a fieldless explicit-layout value type has a proven data extent.
                if row.layout_kind() == 2
                    && self
                        .md
                        .tables()
                        .field_range(Rid::new(token.rid()))?
                        .is_empty()
                {
                    self.md
                        .tables()
                        .class_layout_of(Rid::new(token.rid()))?
                        .filter(|layout| layout.class_size != 0)
                        .map(|layout| layout.class_size as usize)
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    pub fn property_declaration(
        &self,
        token: u32,
        owner: &SelectedType,
    ) -> Result<(String, usize)> {
        let row = self.md.tables().property(token & 0xffffff)?;
        let mut failures = 0;
        let mut name_diagnostic = String::new();
        let name = match self.md.strings().str(row.name) {
            Ok(name) => escape_line(name),
            Err(error) => {
                failures += 1;
                name_diagnostic =
                    format!("// FAILED metadata: {}\n", escape_line(&error.to_string()));
                format!("<unreadable name-index=0x{:08X}>", row.name.get())
            }
        };
        let blob = self.md.blobs().get(row.type_signature);
        let evidence = match &blob {
            Ok(bytes) => format!("signature-bytes={bytes:02X?}"),
            Err(_) => format!("signature-blob-index=0x{:08X}", row.type_signature.get()),
        };
        let signature = (|| -> Result<String> {
            let sig = cildec::PropertySig::parse(blob?)?;
            let names = Names::new(&self.md);
            let params = sig
                .params
                .iter()
                .map(|p| escape_line(&names.param(p)))
                .collect::<Vec<_>>()
                .join(", ");
            let mut references = Vec::new();
            for param in std::iter::once(&sig.return_type).chain(&sig.params) {
                self.signature_references(&param.type_, 0, &mut references)?;
                for modifier in &param.custom_mods {
                    references.push(self.type_reference(modifier.type_token, 0)?);
                }
            }
            Ok(format!(
                "{} {} {name}({params}) // {evidence} parsed={sig:?} references=[{}]",
                if sig.has_this { "instance" } else { "static" },
                escape_line(&names.param(&sig.return_type)),
                references.join("; ")
            ))
        })();
        let mut text = format!(".property 0x{token:08X} flags=0x{:04X} ", row.flags);
        match signature {
            Ok(signature) => text.push_str(&format!("{signature}\n")),
            Err(error) => {
                failures += 1;
                text.push_str(&format!(
                    "{name} // {evidence}\n// FAILED metadata: {}\n",
                    escape_line(&error.to_string())
                ));
            }
        }
        text.push_str(&name_diagnostic);
        let (accessors, errors) = self.accessor_metadata(token, owner)?;
        text.push_str(&accessors);
        failures += errors;
        match self.constant_metadata(token, row.flags & 0x1000 != 0) {
            Ok((constants, errors)) => {
                text.push_str(&constants);
                failures += errors;
            }
            Err(error) => {
                failures += 1;
                text.push_str(&format!(
                    "// FAILED metadata: {}\n",
                    escape_line(&error.to_string())
                ));
            }
        }
        Ok((text, failures))
    }

    pub fn event_declaration(&self, token: u32, owner: &SelectedType) -> Result<(String, usize)> {
        let row = self
            .md
            .tables()
            .row::<cildec::tables::EventRow>(token & 0xffffff)?;
        let mut text = format!(".event 0x{token:08X} flags=0x{:04X} ", row.event_flags);
        let mut failures = 0;
        let mut name_diagnostic = String::new();
        let name = self.md.strings().str(row.name);
        match name {
            Ok(name) => text.push_str(&format!("name={} ", escape_line(name))),
            Err(error) => {
                failures += 1;
                text.push_str(&format!("name-index=0x{:08X} ", row.name.get()));
                name_diagnostic =
                    format!("// FAILED metadata: {}\n", escape_line(&error.to_string()));
            }
        }
        match self.type_reference(row.event_type, 0) {
            Ok(description) => text.push_str(&format!("type={description}\n")),
            Err(error) => {
                failures += 1;
                text.push_str(&format!(
                    "type=0x{:08X}\n// FAILED metadata: {}\n",
                    row.event_type.0,
                    escape_line(&error.to_string())
                ));
            }
        }
        text.push_str(&name_diagnostic);
        let (accessors, errors) = self.accessor_metadata(token, owner)?;
        text.push_str(&accessors);
        failures += errors;
        Ok((text, failures))
    }

    fn accessor_metadata(&self, token: u32, owner: &SelectedType) -> Result<(String, usize)> {
        let is_event = token >> 24 == 0x14;
        let mut text = String::new();
        let mut failures = 0;
        let mut singles = std::collections::BTreeSet::new();
        for rid in self.md.tables().semantics_for(Token(token))? {
            let relation = self.md.tables().method_semantics(rid)?;
            let method = 0x06000000 | relation.method.get();
            let kind = match (is_event, relation.semantics) {
                (false, 1) => "setter",
                (false, 2) => "getter",
                (true, 8) => "adder",
                (true, 16) => "remover",
                (true, 32) => "fire",
                (_, 4) => "other",
                _ => "unknown",
            };
            text.push_str(&format!(".accessor 0x{:08X} association=0x{token:08X} semantics=0x{:04X} {kind} method=0x{method:08X}",
                0x18000000 | rid, relation.semantics));
            if !owner.methods.contains(&method)
                || kind == "unknown"
                || (kind != "other" && !singles.insert(relation.semantics))
            {
                failures += 1;
                text.push_str(&format!(
                    "\n// FAILED metadata: invalid or duplicate {} accessor relationship\n",
                    if is_event { "event" } else { "property" }
                ));
            } else {
                match self.name(method) {
                    Ok(name) => text.push_str(&format!(" {}\n", escape_line(&name))),
                    Err(error) => {
                        failures += 1;
                        text.push_str(&format!(
                            "\n// FAILED metadata: {}\n",
                            escape_line(&error.to_string())
                        ));
                    }
                }
            }
        }
        Ok((text, failures))
    }

    pub fn method_declaration(&self, token: u32, owner: &str) -> Result<(String, usize)> {
        let row = self.row(token)?;
        let mut failures = 0;
        let mut name_diagnostic = String::new();
        let name = match self.md.strings().str(row.name) {
            Ok(name) => name.to_string(),
            Err(error) => {
                failures += 1;
                name_diagnostic = format!(
                    "// FAILED metadata method name: {}\n",
                    escape_line(&error.to_string())
                );
                format!("<unreadable name-index=0x{:08X}>", row.name.get())
            }
        };
        let blob = self.md.blobs().get(row.signature);
        let evidence = match &blob {
            Ok(bytes) => format!("signature-bytes={bytes:02X?}"),
            Err(_) => format!("signature-blob-index=0x{:08X}", row.signature.get()),
        };
        let signature: Result<MethodSig<'_>> = blob
            .map_err(Into::into)
            .and_then(|bytes| MethodSig::parse(bytes).map_err(Into::into));
        let mut text = format!(
            ".method 0x{token:08X} flags=0x{:04X} impl-flags=0x{:04X} rva=0x{:08X} {} ",
            row.flags,
            row.impl_flags,
            row.rva,
            method_flags(row.flags)
        );
        match &signature {
            Ok(sig) => {
                let mut references = Vec::new();
                let checked = (|| -> Result<()> {
                    for param in std::iter::once(&sig.return_type)
                        .chain(&sig.params)
                        .chain(&sig.vararg_params)
                    {
                        self.signature_references(&param.type_, 0, &mut references)?;
                        for modifier in &param.custom_mods {
                            references.push(self.type_reference(modifier.type_token, 0)?);
                        }
                    }
                    Ok(())
                })();
                text.push_str(&format!(
                    "{} // {evidence} parsed={sig:?} references=[{}]\n",
                    escape_line(&Names::new(&self.md).method_sig(sig, &format!("{owner}::{name}"))),
                    references.join("; ")
                ));
                if let Err(error) = checked {
                    failures += 1;
                    text.push_str(&format!(
                        "// FAILED metadata: {}\n",
                        escape_line(&error.to_string())
                    ));
                }
            }
            Err(error) => {
                failures += 1;
                text.push_str(&format!(
                    "{}::{} // {evidence}\n// FAILED metadata: {}\n",
                    escape_line(owner),
                    escape_line(&name),
                    escape_line(&error.to_string())
                ));
            }
        }
        text.push_str(&name_diagnostic);
        text.push_str(&format!(
            ".implementation 0x{token:08X} {}\n",
            implementation_flags(row.impl_flags)
        ));
        let tables = self.md.tables();
        let start = row.param_list.get();
        let end = if token & 0xffffff == tables.row_count(TableId::MethodDef) {
            tables.row_count(TableId::Param) + 1
        } else {
            tables.method_def((token & 0xffffff) + 1)?.param_list.get()
        };
        if start == 0 || start > end || end > tables.row_count(TableId::Param) + 1 {
            failures += 1;
            text.push_str(&format!(
                "// FAILED metadata: invalid parameter range {start}..{end}\n"
            ));
        } else {
            let mut sequences = std::collections::BTreeSet::new();
            for rid in start..end {
                let param = tables.param(rid)?;
                let name = match self.md.strings().str(param.name) {
                    Ok(name) => escape_line(name),
                    Err(error) => {
                        failures += 1;
                        text.push_str(&format!(
                            "// FAILED metadata: {}\n",
                            escape_line(&error.to_string())
                        ));
                        format!("<unreadable name-index=0x{:08X}>", param.name.get())
                    }
                };
                text.push_str(&format!(
                    ".param 0x{:08X} method=0x{token:08X} sequence={} flags=0x{:04X} name={}\n",
                    0x08000000 | rid,
                    param.sequence,
                    param.flags,
                    name
                ));
                if !sequences.insert(param.sequence)
                    || signature
                        .as_ref()
                        .is_ok_and(|sig| usize::from(param.sequence) > sig.params.len())
                {
                    failures += 1;
                    text.push_str("// FAILED metadata: invalid or duplicate parameter sequence\n");
                }
                match self.constant_metadata(0x08000000 | rid, param.flags & 0x1000 != 0) {
                    Ok((constants, errors)) => {
                        text.push_str(&constants);
                        failures += errors;
                    }
                    Err(error) => {
                        failures += 1;
                        text.push_str(&format!(
                            "// FAILED metadata: {}\n",
                            escape_line(&error.to_string())
                        ));
                    }
                }
            }
        }
        let imports = tables.rows_for_parent(TableId::ImplMap, Token(token))?;
        if imports.len() > 1 || (row.flags & 0x2000 != 0) != !imports.is_empty() {
            failures += 1;
            text.push_str("// FAILED metadata: inconsistent method import association\n");
        }
        for rid in imports {
            let flags = tables.raw_column(TableId::ImplMap, rid, 0)?;
            let name_index = tables.raw_column(TableId::ImplMap, rid, 2)?;
            let scope = tables.raw_column(TableId::ImplMap, rid, 3)?;
            let raw = tables.row_bytes(TableId::ImplMap, rid)?;
            let valid_scope = scope != 0
                && scope <= Token::RID_MASK
                && scope <= tables.row_count(TableId::ModuleRef);
            let scope_identity = if valid_scope {
                format!("scope=0x{:08X}", Token::new(TableId::ModuleRef, scope).0)
            } else {
                format!("scope-rid=0x{scope:08X}")
            };
            text.push_str(&format!(
                ".pinvoke 0x{:08X} method=0x{token:08X} flags=0x{flags:04X} {scope_identity} ",
                0x1c000000 | rid
            ));
            let mut errors = Vec::new();
            match self.md.strings().str(cildec::StringIndex(name_index)) {
                Ok(name) => text.push_str(&format!("name={} ", escape_line(name))),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!("name-index=0x{name_index:08X} "));
                    errors.push(format!("import name: {error}"));
                }
            }
            // Module identity is independent of the entry point name. Preserve its
            // raw row/index even when either heap lookup fails.
            if valid_scope {
                let module_raw = tables.row_bytes(TableId::ModuleRef, scope)?;
                let module_name = tables.raw_column(TableId::ModuleRef, scope, 0)?;
                match self.md.strings().str(cildec::StringIndex(module_name)) {
                    Ok(name) => text.push_str(&format!("module={} ", escape_line(name))),
                    Err(error) => {
                        failures += 1;
                        text.push_str(&format!("module-name-index=0x{module_name:08X} "));
                        errors.push(format!("module name: {error}"));
                    }
                }
                text.push_str(&format!(
                    "module-raw-row-bytes={module_raw:02X?} module-name-index=0x{module_name:08X} "
                ));
            } else {
                failures += 1;
                errors.push("module scope RID is null/out-of-range".into());
            }
            text.push_str(&format!("raw-row-bytes={raw:02X?}; external native binding and interop execution are runtime-owned\n"));
            for error in errors {
                text.push_str(&format!("// FAILED metadata: {}\n", escape_line(&error)));
            }
        }
        let arity = signature
            .as_ref()
            .ok()
            .map(|sig| sig.calling_convention.generic_param_count());
        match self.generic_metadata(token, arity) {
            Ok((generics, errors)) => {
                text.push_str(&generics);
                failures += errors;
            }
            Err(error) => {
                failures += 1;
                text.push_str(&format!(
                    "// FAILED metadata: {}\n",
                    escape_line(&error.to_string())
                ));
            }
        }
        Ok((text, failures))
    }

    fn generic_metadata(&self, owner: u32, expected: Option<u32>) -> Result<(String, usize)> {
        let tables = self.md.tables();
        let parameters = tables.generic_params(Token(owner))?;
        let mut text = String::new();
        let mut failures = 0;
        if expected.is_some_and(|arity| arity as usize != parameters.len()) {
            failures += 1;
            text.push_str(
                "// FAILED metadata: generic parameter count differs from method signature\n",
            );
        }
        for (number, rid) in parameters.into_iter().enumerate() {
            let row = tables.generic_param(rid)?;
            text.push_str(&format!(
                ".generic 0x{:08X} owner=0x{owner:08X} number={} flags=0x{:04X} {} ",
                0x2a000000 | rid,
                row.number,
                row.flags,
                generic_flags(row.flags)
            ));
            match self.md.strings().str(row.name) {
                Ok(name) => text.push_str(&format!("name={}\n", escape_line(name))),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "name-index=0x{:08X}\n// FAILED metadata: {}\n",
                        row.name.get(),
                        escape_line(&error.to_string())
                    ));
                }
            }
            if usize::from(row.number) != number {
                failures += 1;
                text.push_str(
                    "// FAILED metadata: invalid or duplicate generic parameter number\n",
                );
            }
            for constraint in tables.generic_param_constraints(Rid::new(rid))? {
                let row = tables.generic_param_constraint(constraint)?;
                text.push_str(&format!(
                    ".constraint 0x{:08X} owner=0x{:08X} type=",
                    0x2c000000 | constraint,
                    0x2a000000 | rid
                ));
                match self.type_reference(row.constraint, 0) {
                    Ok(description) => text.push_str(&format!("{description}\n")),
                    Err(error) => {
                        failures += 1;
                        text.push_str(&format!(
                            "0x{:08X}\n// FAILED metadata: {}\n",
                            row.constraint.0,
                            escape_line(&error.to_string())
                        ));
                    }
                }
            }
        }
        Ok((text, failures))
    }

    fn constant_metadata(&self, parent: u32, has_default: bool) -> Result<(String, usize)> {
        let constants = self
            .md
            .tables()
            .rows_for_parent(TableId::Constant, Token(parent))?;
        let mut text = String::new();
        let mut failures = 0;
        if constants.len() > 1 || has_default != !constants.is_empty() {
            failures += 1;
            text.push_str("// FAILED metadata: inconsistent constant association\n");
        }
        for rid in constants {
            let constant = self.md.tables().row::<cildec::tables::ConstantRow>(rid)?;
            let raw = self.md.blobs().get(constant.value);
            let evidence = match &raw {
                Ok(bytes) => format!("bytes={bytes:02X?}"),
                Err(_) => format!("value-blob-index=0x{:08X}", constant.value.get()),
            };
            let value = raw
                .map_err(Into::into)
                .and_then(|raw| constant_value(constant.type_raw, raw));
            text.push_str(&format!(
                ".constant 0x{:08X} parent=0x{parent:08X} type=0x{:04X} ",
                0x0b000000 | rid,
                constant.type_raw
            ));
            match value {
                Ok(value) => text.push_str(&format!("value={value} {evidence}\n")),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "{evidence}\n// FAILED metadata: {}\n",
                        escape_line(&error.to_string())
                    ));
                }
            }
        }
        Ok((text, failures))
    }

    pub fn row(&self, token: u32) -> Result<MethodDefRow> {
        if token >> 24 != 6 || token & 0xffffff == 0 {
            return Err("expected nonzero MethodDef token (0x06xxxxxx)".into());
        }
        Ok(self.md.tables().method_def(token & 0xffffff)?)
    }

    pub fn signature(&self, token: u32) -> Result<MethodSig<'a>> {
        let row = self.row(token)?;
        crate::signature_guard::method(self.md.blobs().get(row.signature)?)
    }

    pub fn body(&self, token: u32) -> Result<Option<MethodBody<'a>>> {
        let row = self.row(token)?;
        if row.rva == 0 {
            if row.is_abstract()
                || row.is_pinvoke()
                || row.is_runtime()
                || row.impl_flags & 0x1000 != 0
            {
                return Ok(None);
            }
            return Err("ordinary IL method has no body RVA".into());
        }
        if !row.is_il() {
            return Err("non-CIL native method body is not a declaration".into());
        }
        if row.is_abstract() || row.is_pinvoke() {
            return Err("body conflicts with method flags".into());
        }
        Ok(MethodBody::from_image(&self.pe, &row)?)
    }

    pub fn name(&self, token: u32) -> Result<String> {
        Ok(Names::new(&self.md)
            .method_def_full(Rid::new(self.row(token).map(|_| token & 0xffffff)?))?)
    }

    pub fn disassemble(&self, token: u32) -> Result<String> {
        let mut out = format!(
            "// token 0x{token:08X} {}\n",
            escape_line(&self.name(token)?)
        );
        let row = self.row(token)?;
        out.push_str(&format!(
            ".method-flags 0x{:04X} .impl-flags 0x{:04X}\n",
            row.flags, row.impl_flags
        ));
        let Some(body) = self.body(token)? else {
            out.push_str("// no IL body (abstract, extern, or runtime supplied)\n");
            return Ok(out);
        };
        out.push_str(&self.body_evidence(&body)?);
        Ok(out)
    }

    pub fn body_evidence(&self, body: &MethodBody<'_>) -> Result<String> {
        let mut out = format!(
            ".maxstack {}\n.initlocals {}\n",
            body.max_stack,
            body.flags.init_locals()
        );
        for (chunk, bytes) in body.code.chunks(24).enumerate() {
            out.push_str(&format!(
                ".code-bytes IL_{:04X}: {:02X?}\n",
                chunk * 24,
                bytes
            ));
        }
        if let Some(locals) = body.local_var_sig {
            let row = self.md.tables().stand_alone_sig(locals.rid())?;
            let sig = cildec::LocalVarSig::parse(self.md.blobs().get(row.signature)?)?;
            for (i, local) in sig.locals.iter().enumerate() {
                out.push_str(&format!(
                    ".local [{i}] {}\n",
                    escape_line(&Names::new(&self.md).local(local))
                ));
            }
        }
        for ins in body.instructions() {
            let ins = ins?;
            let operand = match &ins.operand {
                cildec::Operand::None => String::new(),
                cildec::Operand::Var(n) => n.to_string(),
                cildec::Operand::I8(n) => n.to_string(),
                cildec::Operand::I32(n) => n.to_string(),
                cildec::Operand::I64(n) => n.to_string(),
                cildec::Operand::R32(n) => format!("{n:?}"),
                cildec::Operand::R64(n) => format!("{n:?}"),
                cildec::Operand::BranchTarget(n) => format!("IL_{n:04X}"),
                cildec::Operand::Switch(ns) => ns
                    .iter()
                    .map(|n| format!("IL_{n:04X}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                cildec::Operand::String(t) => {
                    let body = self.user_string_bytes(*t)?;
                    let (flag, bytes) = body
                        .split_last()
                        .map_or((None, &[][..]), |(flag, bytes)| (Some(*flag), bytes));
                    let units = bytes
                        .chunks_exact(2)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                        .collect::<Vec<_>>();
                    format!(
                        "0x{:08X} {units:?} utf16-bytes={bytes:02X?} terminal-flag={flag:?}",
                        t.0
                    )
                }
                o => match o.token() {
                    Some(t) => format!(
                        "0x{:08X} {}",
                        t.0,
                        escape_line(&Names::new(&self.md).try_token(t)?)
                    ),
                    None => format!("{o:?}"),
                },
            };
            out.push_str(&format!(
                "IL_{:04X}: {} {}\n",
                ins.offset,
                ins.opcode.name(),
                operand
            ));
        }
        for eh in &body.exception_handlers {
            let kind = match eh.kind {
                cildec::HandlerKind::Catch(t) => {
                    format!("catch 0x{:08X} {}", t.0, Names::new(&self.md).try_token(t)?)
                }
                cildec::HandlerKind::Finally => "finally".into(),
                cildec::HandlerKind::Fault => "fault".into(),
                cildec::HandlerKind::Filter { filter_offset } => {
                    format!("filter IL_{filter_offset:04X}")
                }
            };
            out.push_str(&format!(".eh-raw {eh:?}\n"));
            out.push_str(&format!(
                ".try IL_{:04X}..IL_{:04X} {} IL_{:04X}..IL_{:04X}\n",
                eh.try_offset,
                eh.try_end(),
                escape_line(&kind),
                eh.handler_offset,
                eh.handler_end()
            ));
        }
        for section in &body.unknown_sections {
            out.push_str(&format!(
                ".section {} {:02X?}\n",
                section.kind, section.data
            ));
        }
        for diagnostic in body.diagnostics() {
            out.push_str(&format!("// diagnostic: {diagnostic}\n"));
        }
        Ok(out)
    }

    fn user_string_bytes(&self, token: cildec::UserStringToken) -> Result<&'a [u8]> {
        if !token.has_us_table_byte() || token.offset() == 0 {
            return Err("expected nonzero user-string token (0x70xxxxxx)".into());
        }
        let heap = self
            .md
            .stream("#US")
            .ok_or("missing user-string heap")?
            .data;
        let rest = heap
            .get(token.offset() as usize..)
            .ok_or("user-string offset out of range")?;
        let (length, prefix) = cildec::read_compressed_u32(rest)?;
        let end = prefix
            .checked_add(length as usize)
            .ok_or("user-string extent overflow")?;
        Ok(rest
            .get(prefix..end)
            .ok_or("user-string extent out of range")?)
    }

    pub fn string_literal(&self, token: cildec::UserStringToken) -> Result<String> {
        let body = self.user_string_bytes(token)?;
        let (&flag, bytes) = body
            .split_last()
            .ok_or("invalid user-string encoding: missing terminal flag")?;
        if bytes.len() % 2 != 0 {
            return Err("invalid user-string encoding: incomplete UTF-16 code unit".into());
        }
        let units = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]));
        let mut literal = String::from("\"");
        let mut special = false;
        for unit in units {
            special |=
                unit > 0xff || matches!(unit, 0x01..=0x08 | 0x0e..=0x1f | 0x27 | 0x2d | 0x7f);
            match unit {
                0x22 => literal.push_str("\\\""),
                0x5c => literal.push_str("\\\\"),
                0x20..=0x7e => literal.push(char::from(unit as u8)),
                _ => literal.push_str(&format!("\\u{unit:04X}")),
            }
        }
        if flag != u8::from(special) {
            return Err(
                "invalid user-string encoding: terminal flag differs from code units".into(),
            );
        }
        literal.push('"');
        Ok(literal)
    }

    pub fn local_types(
        &self,
        body: &MethodBody<'a>,
        context: crate::signature_types::TypeContext,
    ) -> Result<Vec<crate::ir::Ty>> {
        let Some(t) = body.local_var_sig else {
            return Ok(Vec::new());
        };
        if t.table() != Some(TableId::StandAloneSig) {
            return Err("invalid local signature token".into());
        }
        let row = self.md.tables().stand_alone_sig(t.rid())?;
        let bytes = self.md.blobs().get(row.signature)?;
        let sig = cildec::LocalVarSig::parse(bytes)?;
        if sig
            .locals
            .iter()
            .any(|l| l.pinned || l.by_ref || !l.custom_mods.is_empty())
        {
            return Err("pinned/byref/modified local needs IL fallback".into());
        }
        crate::signature_guard::plain_locals(bytes)?;
        sig.locals
            .iter()
            .map(|l| crate::ir::Ty::from_sig_scoped(&l.type_, self, context))
            .collect()
    }

    pub(crate) fn nested_type_parents(&self) -> Result<std::collections::BTreeMap<u32, u32>> {
        let count = self.md.tables().row_count(TableId::TypeDef);
        if count > Token::RID_MASK {
            return Err("type definition RID exceeds token width".into());
        }
        let mut parents = std::collections::BTreeMap::new();
        for (_, row) in self.md.tables().iter::<NestedClassRow>() {
            let row = row?;
            let (child, parent) = (row.nested_class.get(), row.enclosing_class.get());
            if child == 0
                || child > count
                || parent == 0
                || parent > count
                || parents.insert(child, parent).is_some()
            {
                return Err("invalid or duplicate nested type owner".into());
            }
        }
        Ok(parents)
    }

    pub(crate) fn find_type_definition(
        &self,
        namespace: &str,
        name: &str,
        parent: Option<u32>,
    ) -> Result<Option<u32>> {
        let parents = self.nested_type_parents()?;
        let mut found = None;
        for (rid, row) in self.md.tables().iter::<TypeDefRow>() {
            let row = row?;
            if parents.get(&rid).copied() == parent
                && self.md.strings().str(row.type_name)? == name
                && (parent.is_some() || self.md.strings().str(row.type_namespace)? == namespace)
                && found.replace(rid).is_some()
            {
                return Err("ambiguous type definition in resolved assembly".into());
            }
        }
        Ok(found)
    }

    /// Canonicalize only exact in-module references; foreign scopes stay opaque.
    pub(crate) fn local_type_definition(&self, token: Token) -> Result<Option<Token>> {
        fn resolve(image: &Image<'_>, token: Token, seen: &mut Vec<u32>) -> Result<Option<Token>> {
            if token.rid() == 0 || seen.len() == 32 || seen.contains(&token.0) {
                return Err("null, cyclic or excessive local type reference".into());
            }
            if token.table() == Some(TableId::TypeDef) {
                image.md.tables().type_def(token.rid())?;
                return Ok(Some(token));
            }
            if token.table() != Some(TableId::TypeRef) {
                return Err("local type lookup requires TypeDef or TypeRef".into());
            }
            seen.push(token.0);
            let row = image.md.tables().type_ref(token.rid())?;
            let scope = image.metadata_coded_token(
                cildec::CodedIndex::ResolutionScope,
                image
                    .md
                    .tables()
                    .raw_column(TableId::TypeRef, token.rid(), 0)?,
            )?;
            let namespace = image.md.strings().str(row.type_namespace)?;
            let name = image.md.strings().str(row.type_name)?;
            let parent = match scope.table() {
                Some(TableId::Module) if scope.rid() == 1 => {
                    image.md.tables().module(1)?;
                    None
                }
                Some(TableId::TypeRef) => {
                    if !namespace.is_empty() {
                        return Err("nested TypeRef has a namespace".into());
                    }
                    let Some(parent) = resolve(image, scope, seen)? else {
                        return Ok(None);
                    };
                    Some(parent.rid())
                }
                Some(TableId::AssemblyRef | TableId::ModuleRef) => return Ok(None),
                _ => return Err("unsupported local type resolution scope".into()),
            };
            let rid = image
                .find_type_definition(namespace, name, parent)?
                .ok_or("type definition missing from current module")?;
            Ok(Some(Token(0x02000000 | rid)))
        }
        resolve(self, token, &mut Vec::new())
    }

    pub(crate) fn method_type_context(
        &self,
        token: u32,
        signature: &MethodSig<'_>,
    ) -> Result<crate::signature_types::TypeContext> {
        use crate::signature_types::{GenericScope, TypeContext};
        fn scope(image: &Image<'_>, owner: Token) -> Result<GenericScope> {
            let tables = image.md.tables();
            let parameters = tables.generic_params(owner)?;
            if parameters.len() > 4096 {
                return Err("generic context arity exceeds 4096 positions".into());
            }
            for (position, rid) in parameters.iter().copied().enumerate() {
                let row = tables.generic_param(rid)?;
                let raw_owner = image.metadata_coded_token(
                    cildec::CodedIndex::TypeOrMethodDef,
                    tables.raw_column(TableId::GenericParam, rid, 2)?,
                )?;
                if raw_owner != owner
                    || usize::from(row.number) != position
                    || row.flags & !0x1f != 0
                    || row.flags & 0x0c == 0x0c
                    || row.flags & 3 == 3
                    || (owner.table() == Some(TableId::MethodDef) && row.flags & 3 != 0)
                {
                    return Err("invalid generic context owner/position/flags".into());
                }
            }
            Ok(GenericScope {
                owner: owner.0,
                count: parameters.len() as u32,
            })
        }
        let method = Token(token);
        self.row(token)?;
        let owner = self
            .md
            .tables()
            .type_of_method(Rid::new(method.rid()))?
            .ok_or("generic method context has no declaring type")?;
        let method_scope = scope(self, method)?;
        let expected = match signature.calling_convention {
            cildec::CallConv::Default => 0,
            cildec::CallConv::Generic(arity) if arity != 0 => arity,
            _ => return Err("unsupported generic method context calling convention".into()),
        };
        if method_scope.count != expected {
            return Err("generic context parameter count differs from method signature".into());
        }
        Ok(TypeContext {
            type_scope: scope(self, Token(0x02000000 | owner.get()))?,
            method_scope,
        })
    }

    pub(crate) fn intern_signature_type(
        &self,
        ty: &cildec::Type<'_>,
        context: crate::signature_types::TypeContext,
    ) -> Result<crate::signature_types::TypeId> {
        let (signature, description) = crate::signature_types::encode_type(self, ty, context)?;
        self.signature_types
            .borrow_mut()
            .intern(signature, description)
    }

    pub(crate) fn signature_type_token(
        &self,
        token: Token,
        value: bool,
        arity: usize,
    ) -> Result<Token> {
        let token = if token.table() == Some(TableId::TypeRef) {
            self.local_type_definition(token)?.unwrap_or(token)
        } else {
            token
        };
        match token.table() {
            Some(TableId::TypeRef) => {
                self.type_reference(token, 0)?;
            }
            Some(TableId::TypeDef) => {
                let row = self.md.tables().type_def(token.rid())?;
                if self.md.strings().str(row.type_name)?.is_empty()
                    || self.md.tables().generic_params(token)?.len() != arity
                {
                    return Err("structural type definition has invalid name/generic arity".into());
                }
                let core = self.local_core_type_name(token)?;
                let is_value = !matches!(core, Some("ValueType" | "Enum"))
                    && (self.has_core_base(token, "ValueType")?
                        || self.has_core_base(token, "Enum")?);
                if is_value != value {
                    return Err(
                        "structural signature class/value kind disagrees with local definition"
                            .into(),
                    );
                }
            }
            _ => return Err("structural type definition must be TypeDef or TypeRef".into()),
        }
        Ok(token)
    }

    pub(crate) fn enum_type(&self, token: Token) -> Result<crate::ir::EnumType> {
        // A cross-image storage fact is cached under the TypeRef it was asked
        // about, so it must be honoured before the TypeDef requirement below.
        if let Some(cached) = self.enum_types.borrow().get(&token.0) {
            return cached.clone().map_err(Into::into);
        }
        let token = if token.table() == Some(TableId::TypeRef) {
            self.local_type_definition(token)?.unwrap_or(token)
        } else {
            token
        };
        if token.table() != Some(TableId::TypeDef) || token.rid() == 0 {
            return Err("enum storage requires an exact local definition; external/value signatures need IL fallback".into());
        }
        if let Some(cached) = self.enum_types.borrow().get(&token.0) {
            return cached.clone().map_err(Into::into);
        }
        let result = self.checked_enum_type(token).map_err(|e| e.to_string());
        self.enum_types.borrow_mut().insert(token.0, result.clone());
        result.map_err(Into::into)
    }

    /// Value-type tokens named by a method's own signature and local slots.
    /// Callers use this to ask an explicit dependency context about exactly the
    /// types this method can load or store, rather than walking every TypeRef.
    pub(crate) fn value_type_tokens(
        &self,
        token: u32,
        body: &cildec::MethodBody<'_>,
    ) -> Result<SignatureValueTypes> {
        let mut tokens = SignatureValueTypes::default();
        fn walk(ty: &cildec::Type<'_>, out: &mut SignatureValueTypes) {
            match ty {
                cildec::Type::ValueType(token) => out.values.push(*token),
                cildec::Type::GenericInst {
                    is_value_type: true,
                    def,
                    args,
                } => {
                    out.value_definitions.push((*def, args.len()));
                    for arg in args {
                        walk(arg, out);
                    }
                }
                cildec::Type::GenericInst { def, args, .. } => {
                    out.values.push(*def);
                    for arg in args {
                        walk(arg, out);
                    }
                }
                cildec::Type::Array(element, _) => walk(element, out),
                cildec::Type::SzArray(_, element) => walk(element, out),
                cildec::Type::ByRef(inner)
                | cildec::Type::Ptr(_, inner)
                | cildec::Type::Modified(_, inner)
                | cildec::Type::Pinned(inner) => walk(inner, out),
                _ => {}
            }
        }
        let row = self.row(token)?;
        let signature = crate::signature_guard::method(self.md.blobs().get(row.signature)?)?;
        walk(&signature.return_type.type_, &mut tokens);
        for param in &signature.params {
            walk(&param.type_, &mut tokens);
        }
        if let Some(t) = body.local_var_sig
            && t.table() == Some(TableId::StandAloneSig)
        {
            let row = self.md.tables().stand_alone_sig(t.rid())?;
            let signature = cildec::LocalVarSig::parse(self.md.blobs().get(row.signature)?)?;
            for local in &signature.locals {
                walk(&local.type_, &mut tokens);
            }
        }
        // A call, field access or allocation can name a value type that this
        // method never stores directly. Those signatures live in this image, so
        // they are collected here rather than left to whichever method happened
        // to be lowered first: admission must not depend on scan order.
        for instruction in body.instructions() {
            let Ok(instruction) = instruction else {
                break;
            };
            let (blob, field) = match instruction.operand {
                cildec::Operand::Method(target) => (self.member_signature(target)?, false),
                cildec::Operand::Field(target) => (self.member_signature(target)?, true),
                _ => continue,
            };
            let Some(blob) = blob else { continue };
            if field {
                if let Ok(signature) = crate::signature_guard::field(blob) {
                    walk(&signature.type_, &mut tokens);
                }
            } else if let Ok(signature) = crate::signature_guard::method(blob) {
                walk(&signature.return_type.type_, &mut tokens);
                for param in &signature.params {
                    walk(&param.type_, &mut tokens);
                }
            }
        }
        Ok(tokens)
    }

    /// The signature blob of a member operand, when the image owns one.
    ///
    /// A prefetch that cannot read a signature simply contributes no candidate;
    /// the lowering reports the original blocker for that member on its own.
    fn member_signature(&self, token: Token) -> Result<Option<&'a [u8]>> {
        Ok(match token.table() {
            Some(TableId::MemberRef) => Some(
                self.md
                    .blobs()
                    .get(self.md.tables().member_ref(token.rid())?.signature)?,
            ),
            Some(TableId::MethodDef) => Some(
                self.md
                    .blobs()
                    .get(self.md.tables().method_def(token.rid())?.signature)?,
            ),
            Some(TableId::Field) => Some(
                self.md
                    .blobs()
                    .get(self.md.tables().field(token.rid())?.signature)?,
            ),
            _ => None,
        })
    }

    /// Name of a value type defined in another assembly, recorded when its
    /// storage fact was admitted. Kept per image so a foreign token is never
    /// looked up in this image's type table.
    pub(crate) fn external_type_name(&self, index: u32) -> Result<String> {
        self.external_type_names
            .borrow()
            .get(index as usize)
            .cloned()
            .ok_or_else(|| "unknown external value type name".into())
    }

    pub(crate) fn intern_external_type_name(&self, name: &str) -> u32 {
        let mut names = self.external_type_names.borrow_mut();
        if let Some(index) = names.iter().position(|existing| existing == name) {
            return index as u32;
        }
        names.push(name.to_string());
        (names.len() - 1) as u32
    }

    /// Facts recorded for one call target from a referenced assembly, if the
    /// caller asked for them. `Some(Err(..))` records why the fact was refused.
    pub(crate) fn external_call(
        &self,
        token: u32,
    ) -> Option<std::result::Result<crate::dependencies::ExternalCall, String>> {
        self.external_calls.borrow().get(&token).cloned()
    }

    /// Facts for one foreign field, if the caller asked for them.
    pub(crate) fn external_field(
        &self,
        token: u32,
    ) -> Option<std::result::Result<crate::dependencies::ExternalField, String>> {
        self.external_fields.borrow().get(&token).cloned()
    }

    pub(crate) fn cache_external_field(
        &self,
        token: u32,
        value: std::result::Result<crate::dependencies::ExternalField, String>,
    ) {
        self.external_fields.borrow_mut().insert(token, value);
    }

    /// Facts for one foreign constructor, if the caller asked for them.
    pub(crate) fn external_allocation(
        &self,
        token: u32,
    ) -> Option<std::result::Result<crate::dependencies::ExternalAllocation, String>> {
        self.external_allocations.borrow().get(&token).cloned()
    }

    pub(crate) fn cache_external_allocation(
        &self,
        token: u32,
        value: std::result::Result<crate::dependencies::ExternalAllocation, String>,
    ) {
        self.external_allocations.borrow_mut().insert(token, value);
    }

    pub(crate) fn cache_external_call(
        &self,
        token: u32,
        value: std::result::Result<crate::dependencies::ExternalCall, String>,
    ) {
        self.external_calls.borrow_mut().insert(token, value);
    }

    /// True when this type would be named by an open generic definition, i.e. the
    /// text cannot be written as C# without its instantiation.
    pub(crate) fn generic_definition_name(&self, ty: crate::ir::Ty) -> Result<bool> {
        let crate::ir::Ty::Class(token) = ty else {
            return Ok(false);
        };
        if token & 0xffffff == 0 {
            return Ok(false);
        }
        self.generic_declaring_context(Token(token))
    }

    /// Whether a value definition is an enum, a struct, or neither.
    pub(crate) fn value_definition_kind(&self, token: Token) -> Result<ValueDefinitionKind> {
        if self.has_core_base(token, "Enum")? {
            Ok(ValueDefinitionKind::Enum)
        } else if self.has_core_base(token, "ValueType")? {
            Ok(ValueDefinitionKind::Struct)
        } else {
            Ok(ValueDefinitionKind::Other)
        }
    }

    /// A value type from a referenced assembly that passed the copy proof, keyed by
    /// the input-scoped TypeRef identity the signature used.
    pub(crate) fn external_struct_type(
        &self,
        token: Token,
    ) -> Option<std::result::Result<crate::signature_types::TypeId, String>> {
        self.external_structs.borrow().get(&token.0).cloned()
    }

    pub(crate) fn cache_external_struct_type(
        &self,
        token: Token,
        value: std::result::Result<crate::signature_types::TypeId, String>,
    ) {
        self.external_structs.borrow_mut().insert(token.0, value);
    }

    /// Admission of a value definition from a referenced assembly, recorded with
    /// the generic arity the referenced definition declares.
    pub(crate) fn has_external_value_definition(&self, token: Token) -> bool {
        self.external_value_defs.borrow().contains_key(&token.0)
    }

    pub(crate) fn cache_external_value_definition(
        &self,
        token: Token,
        value: std::result::Result<u32, String>,
    ) {
        self.external_value_defs.borrow_mut().insert(token.0, value);
    }

    /// Record that an interned reference instantiation is the open self type of a
    /// local definition, i.e. `C<!0..!n-1>`. Members of `C` name `this` that way,
    /// so the two representations must stay assignable to each other.
    pub(crate) fn register_open_self(&self, id: crate::signature_types::TypeId, definition: u32) {
        self.open_self.borrow_mut().insert(id, definition);
    }

    pub(crate) fn open_self_definition(&self, id: crate::signature_types::TypeId) -> Option<u32> {
        self.open_self.borrow().get(&id).copied()
    }

    pub(crate) fn has_cached_enum(&self, token: Token) -> bool {
        self.enum_types.borrow().contains_key(&token.0)
    }

    pub(crate) fn cache_enum(&self, token: Token, value: crate::ir::EnumType) {
        self.enum_types.borrow_mut().insert(token.0, Ok(value));
    }

    pub(crate) fn has_core_base(&self, token: Token, name: &str) -> Result<bool> {
        let tables = self.md.tables();
        let mut base = self.metadata_coded_token(
            cildec::CodedIndex::TypeDefOrRef,
            tables.raw_column(TableId::TypeDef, token.rid(), 3)?,
        )?;
        if base.table() == Some(TableId::TypeRef) {
            base = self.local_type_definition(base)?.unwrap_or(base);
        }
        Ok(match base.table() {
            Some(TableId::TypeDef) => {
                self.local_core_type_name(base)? == Some(name)
                    && !self.generic_declaring_context(base)?
            }
            Some(TableId::TypeRef) => {
                let ty = tables.type_ref(base.rid())?;
                let scope = self.metadata_coded_token(
                    cildec::CodedIndex::ResolutionScope,
                    tables.raw_column(TableId::TypeRef, base.rid(), 0)?,
                )?;
                self.md.strings().str(ty.type_namespace)? == "System"
                    && self.md.strings().str(ty.type_name)? == name
                    && crate::dependencies::is_core_reference(self, scope)?
            }
            _ => false,
        })
    }

    pub(crate) fn checked_enum_type(&self, token: Token) -> Result<crate::ir::EnumType> {
        use crate::ir::{EnumStorage, EnumType};
        let tables = self.md.tables();
        let row = tables.type_def(token.rid())?;
        if row.flags & (0x20 | 0x80 | 0x100) != 0x100 || self.generic_declaring_context(token)? {
            return Err("enum storage needs a sealed nongeneric value definition".into());
        }
        if !self.has_core_base(token, "Enum")? {
            return Err("value type has no exact core Enum base".into());
        }
        let mut storage = None;
        for rid in tables.field_range(Rid::new(token.rid()))? {
            let field = tables.field(rid)?;
            if field.flags & 0x10 != 0 {
                if field.flags & 0x60 != 0x40 {
                    return Err("enum static fields must be literal, not initonly".into());
                }
                continue;
            }
            if storage.is_some()
                || self.md.strings().str(field.name)? != "value__"
                || field.flags != 0x0606
            {
                return Err("enum needs one ordinary public special value__ instance field".into());
            }
            let signature = crate::signature_guard::field(self.md.blobs().get(field.signature)?)?;
            if !signature.custom_mods.is_empty() {
                return Err("modified enum storage needs IL fallback".into());
            }
            storage = Some(match signature.type_ {
                cildec::Type::I1 => EnumStorage::I1,
                cildec::Type::U1 => EnumStorage::U1,
                cildec::Type::I2 => EnumStorage::I2,
                cildec::Type::U2 => EnumStorage::U2,
                cildec::Type::I4 => EnumStorage::I4,
                cildec::Type::U4 => EnumStorage::U4,
                cildec::Type::I8 => EnumStorage::I8,
                cildec::Type::U8 => EnumStorage::U8,
                _ => return Err("enum storage must be an admitted fixed-width integer".into()),
            });
        }
        Ok(EnumType {
            token: token.0,
            storage: storage.ok_or("enum has no instance storage field")?,
            external: None,
        })
    }

    pub(crate) fn struct_type(
        &self,
        token: Token,
    ) -> Result<Option<crate::signature_types::TypeId>> {
        let token = if token.table() == Some(TableId::TypeRef) {
            self.local_type_definition(token)?.unwrap_or(token)
        } else {
            token
        };
        if token.table() != Some(TableId::TypeDef) || token.rid() == 0 {
            // A refusal recorded for this cross-image value type keeps its own
            // reason instead of being reported as an enum-storage failure.
            return match self.external_struct_type(token) {
                Some(Ok(id)) => Ok(Some(id)),
                Some(Err(reason)) => Err(reason.into()),
                None => Ok(None),
            };
        }
        if let Some(cached) = self.struct_types.borrow().get(&token.0) {
            return cached.clone().map_err(Into::into);
        }
        let result = self.checked_struct_type(token).map_err(|e| e.to_string());
        self.struct_types
            .borrow_mut()
            .insert(token.0, result.clone());
        result.map_err(Into::into)
    }

    /// The proof shared by local struct copies and constructed value types.
    ///
    /// open_fields admits fields whose signature is one of the definition's own
    /// generic parameters: an instantiation supplies a value for each of them, so
    /// the field is still a value field. Modified, byref, typed-reference and
    /// unreadable fields are refused either way.
    pub(crate) fn value_definition_checks(&self, token: Token, open_fields: bool) -> Result<()> {
        use cildec::CodedIndex;
        let tables = self.md.tables();
        if !self.has_core_base(token, "ValueType")? {
            return Err("value definition has no exact core ValueType base".into());
        }
        let row = tables.type_def(token.rid())?;
        if row.flags & (0x20 | 0x80 | 0x100) != 0x100 {
            return Err("value definition must be sealed and not an interface or abstract".into());
        }
        if matches!(
            self.local_core_type_name(token)?,
            Some(
                "Boolean"
                    | "Char"
                    | "SByte"
                    | "Byte"
                    | "Int16"
                    | "UInt16"
                    | "Int32"
                    | "UInt32"
                    | "Int64"
                    | "UInt64"
                    | "IntPtr"
                    | "UIntPtr"
                    | "Single"
                    | "Double"
                    | "TypedReference"
                    | "Void"
            )
        ) {
            return Err("core scalar/special value signature requires its own stack model".into());
        }
        for rid in tables.custom_attributes(token)? {
            let parent = self.metadata_coded_token(
                CodedIndex::HasCustomAttribute,
                tables.raw_column(TableId::CustomAttribute, rid, 0)?,
            )?;
            if parent != token {
                return Err("invalid struct custom attribute owner".into());
            }
            let constructor = self.metadata_coded_token(
                CodedIndex::CustomAttributeType,
                tables.raw_column(TableId::CustomAttribute, rid, 1)?,
            )?;
            let owner = match constructor.table() {
                Some(TableId::MethodDef) => Token(
                    0x02000000
                        | tables
                            .type_of_method(Rid::new(constructor.rid()))?
                            .ok_or("struct attribute constructor has no owner")?
                            .get(),
                ),
                Some(TableId::MemberRef) => self.metadata_coded_token(
                    CodedIndex::MemberRefParent,
                    tables.raw_column(TableId::MemberRef, constructor.rid(), 0)?,
                )?,
                _ => return Err("unreadable struct attribute constructor".into()),
            };
            let (namespace, name) = match owner.table() {
                Some(TableId::TypeDef) => {
                    let row = tables.type_def(owner.rid())?;
                    (row.type_namespace, row.type_name)
                }
                Some(TableId::TypeRef) => {
                    let row = tables.type_ref(owner.rid())?;
                    (row.type_namespace, row.type_name)
                }
                _ => {
                    return Err(
                        "constructed/unknown struct attribute owner needs IL fallback".into(),
                    );
                }
            };
            // An ambiguous lookalike is conservatively refused, not used as
            // positive identity or layout evidence.
            if self.md.strings().str(namespace)? == "System.Runtime.CompilerServices"
                && self.md.strings().str(name)? == "IsByRefLikeAttribute"
            {
                return Err("byref-like struct copies need location/lifetime semantics".into());
            }
        }
        for rid in tables.field_range(Rid::new(token.rid()))? {
            let field = tables.field(rid)?;
            if field.flags & 0x10 != 0 {
                continue;
            }
            let sig = crate::signature_guard::field(self.md.blobs().get(field.signature)?)?;
            if !sig.custom_mods.is_empty()
                || matches!(
                    sig.type_,
                    cildec::Type::Void | cildec::Type::ByRef(_) | cildec::Type::TypedByRef
                )
                || (!open_fields
                    && matches!(sig.type_, cildec::Type::Var(_) | cildec::Type::MVar(_)))
            {
                return Err(
                    "invalid/modified/open/managed-address struct field needs IL fallback".into(),
                );
            }
        }
        Ok(())
    }

    /// Proof for a constructed value type: the definition must be a sealed generic
    /// value definition and the instantiation must supply exactly its parameters.
    pub(crate) fn admit_generic_value_definition(&self, def: Token, arity: usize) -> Result<()> {
        let token = if def.table() == Some(TableId::TypeRef) {
            self.local_type_definition(def)?.unwrap_or(def)
        } else {
            def
        };
        if token.table() != Some(TableId::TypeDef) || token.rid() == 0 {
            return match self.external_value_defs.borrow().get(&def.0) {
                Some(Ok(declared)) if *declared as usize == arity => Ok(()),
                Some(Ok(declared)) => Err(format!(
                    "constructed value type instantiation supplies {arity} arguments but the definition declares {declared}"
                )
                .into()),
                Some(Err(reason)) => Err(reason.clone().into()),
                None => Err(
                    "constructed value type needs a local definition; external instantiations need dependency facts"
                        .into(),
                ),
            };
        }
        let declared = self.md.tables().generic_params(token)?.len();
        if arity == 0 || declared != arity {
            return Err("constructed value type argument count differs from the definition".into());
        }
        self.value_definition_checks(token, true)
    }

    fn checked_struct_type(&self, token: Token) -> Result<Option<crate::signature_types::TypeId>> {
        if !self.has_core_base(token, "ValueType")? {
            return Ok(None);
        }
        if self.generic_declaring_context(token)? {
            return Err("struct copies require a sealed nongeneric value definition".into());
        }
        self.value_definition_checks(token, false)?;
        Ok(Some(self.intern_signature_type(
            &cildec::Type::ValueType(token),
            Default::default(),
        )?))
    }

    /// Resolve local named reference identity, without guessing external ancestry.
    pub fn class_name(&self, token: u32) -> Result<String> {
        if !self.local_interface(token)? {
            self.class_chain(token)?;
        }
        crate::symbol_names::type_name(self, Token(token), true)
    }

    fn local_interface(&self, token: u32) -> Result<bool> {
        let token = Token(token);
        if token.table() != Some(TableId::TypeDef) || token.rid() == 0 {
            return Err("local reference identity needs a TypeDef".into());
        }
        let row = self.md.tables().type_def(token.rid())?;
        if row.flags & 0x20 == 0 {
            return Ok(false);
        }
        if row.flags & 0x80 == 0
            || self
                .md
                .tables()
                .raw_column(TableId::TypeDef, token.rid(), 3)?
                != 0
        {
            return Err("interface reference needs abstract flags and no class base".into());
        }
        if self.generic_declaring_context(token)? {
            return Err("generic interface reference needs IL fallback".into());
        }
        Ok(true)
    }

    /// Prove assignment only through local, nongeneric interface edges. Constructed
    /// and external edges are not erased to their definitions or guessed by name.
    fn implements_interface(&self, actual: u32, expected: u32) -> Result<bool> {
        let roots = if self.local_interface(actual)? {
            vec![actual]
        } else {
            self.class_chain(actual)?
        };
        let edges = self
            .interface_edges
            .get_or_init(|| {
                let mut edges = std::collections::BTreeMap::<u32, Vec<u32>>::new();
                for (_, row) in self.md.tables().iter::<cildec::tables::InterfaceImplRow>() {
                    let row = row.map_err(|error| error.to_string())?;
                    if row.interface.table() == Some(TableId::TypeDef) {
                        edges
                            .entry(0x02000000 | row.class.get())
                            .or_default()
                            .push(row.interface.0);
                    }
                }
                Ok(edges)
            })
            .as_ref()
            .map_err(|error| error.clone())?;
        let mut pending: Vec<_> = roots.into_iter().map(|token| (token, false)).collect();
        let mut active = std::collections::BTreeSet::new();
        let mut visited = std::collections::BTreeSet::new();
        let mut found = false;
        let mut work = 0;
        while let Some((token, leaving)) = pending.pop() {
            if leaving {
                active.remove(&token);
                visited.insert(token);
                continue;
            }
            if active.contains(&token) {
                return Err("cyclic local interface hierarchy needs IL fallback".into());
            }
            if visited.contains(&token) {
                continue;
            }
            work += 1;
            if work > 256 {
                return Err("local interface hierarchy work limit exceeded".into());
            }
            found |= token == expected;
            active.insert(token);
            pending.push((token, true));
            for &interface in edges.get(&token).into_iter().flatten() {
                if self.generic_declaring_context(Token(interface))? {
                    continue;
                }
                if !self.local_interface(interface)? {
                    return Err("InterfaceImpl edge must name an interface".into());
                }
                pending.push((interface, false));
            }
        }
        Ok(found)
    }

    fn generic_declaring_context(&self, token: Token) -> Result<bool> {
        let mut current = Some(token);
        let mut seen = Vec::new();
        while let Some(owner) = current {
            if owner.table() != Some(TableId::TypeDef)
                || seen.len() == 64
                || seen.contains(&owner.0)
            {
                return Err("invalid/cyclic generic declaring context".into());
            }
            seen.push(owner.0);
            self.md.tables().type_def(owner.rid())?;
            if !self.md.tables().generic_params(owner)?.is_empty() {
                return Ok(true);
            }
            current = self
                .md
                .tables()
                .enclosing_type(Rid::new(owner.rid()))?
                .map(|rid| Token(0x02000000 | rid.get()));
        }
        Ok(false)
    }

    pub(crate) fn is_core_definition(&self) -> Result<bool> {
        self.core_definition
            .get_or_init(|| {
                crate::dependencies::is_core_definition(self).map_err(|e| e.to_string())
            })
            .as_ref()
            .copied()
            .map_err(|e| e.clone().into())
    }

    fn local_core_type_name(&self, token: Token) -> Result<Option<&'a str>> {
        if token.table() != Some(TableId::TypeDef) || token.rid() == 0 {
            return Ok(None);
        }
        let row = self.md.tables().type_def(token.rid())?;
        if row.is_nested()
            || self.md.strings().str(row.type_namespace)? != "System"
            || !self.is_core_definition()?
        {
            return Ok(None);
        }
        Ok(Some(self.md.strings().str(row.type_name)?))
    }

    fn class_chain(&self, token: u32) -> Result<Vec<u32>> {
        let mut chain = Vec::new();
        let mut current = Token(token);
        for _ in 0..64 {
            if current.table() != Some(TableId::TypeDef) || chain.contains(&current.0) {
                return Err("unresolved/cyclic reference hierarchy needs IL fallback".into());
            }
            let row = self.md.tables().type_def(current.rid())?;
            if row.flags & 0x20 != 0 {
                return Err("interface declaring type needs IL fallback".into());
            }
            chain.push(current.0);
            let encoded = self
                .md
                .tables()
                .raw_column(TableId::TypeDef, current.rid(), 3)?;
            let base = self.metadata_coded_token(cildec::CodedIndex::TypeDefOrRef, encoded)?;
            // A class may itself be generic, and a base may be a constructed type
            // such as `Base<T>`. Inheritance, assignability and override identity
            // are properties of the definition, so the chain follows the definition
            // and leaves the instantiation to member binding.
            let base = if base.table() == Some(TableId::TypeSpec) {
                let spec = self.type_spec(base)?;
                let cildec::Type::GenericInst {
                    is_value_type: false,
                    def,
                    ..
                } = &spec.type_
                else {
                    return Err("constructed base needs member binding".into());
                };
                self.local_type_definition(*def)?.ok_or(
                    "constructed base needs a local definition; external constructed bases need dependency facts",
                )?
            } else {
                base
            };
            if base.rid() == 0 {
                if encoded == 0 && self.local_core_type_name(current)? == Some("Object") {
                    return Ok(chain);
                }
                return Err("unresolved reference hierarchy root needs IL fallback".into());
            }
            current = base;
            if matches!(
                self.local_core_type_name(current)?,
                Some("ValueType" | "Enum")
            ) {
                return Err("value-type receiver needs IL fallback".into());
            }
            if current.table() == Some(TableId::TypeRef) {
                let base = self.md.tables().type_ref(current.rid())?;
                if self.md.strings().str(base.type_namespace)? == "System"
                    && matches!(self.md.strings().str(base.type_name)?, "ValueType" | "Enum")
                {
                    return Err("value-type receiver needs IL fallback".into());
                }
                // Legal CLI value types derive directly from ValueType/Enum.
                // Do not assume any assignability beyond this unresolved base.
                return Ok(chain);
            }
        }
        Err("reference hierarchy depth exceeded".into())
    }

    /// Recognize admitted core identities without loading dependencies.
    /// A locally named System type is not a core type.
    /// The core scalar named by a `box`/`unbox.any` operand.
    ///
    /// Only core scalars are admitted: a boxed enum or struct must keep its own
    /// value identity, which needs a constructed-value admission this slice does not
    /// have, so those stay gated instead of being boxed as a plain integer.
    pub(crate) fn boxed_value_type(&self, token: Token) -> Result<crate::ir::Ty> {
        use crate::ir::Ty;
        // A local enum keeps its own boxed identity, so it is admitted with its
        // declared name; a struct or constructed value still needs a name renderer.
        if token.table() != Some(TableId::TypeRef) || self.core_type_name(token).is_err() {
            return match crate::ir::Ty::from_sig_scoped(
                &cildec::Type::ValueType(token),
                self,
                crate::signature_types::TypeContext::default(),
            ) {
                Ok(ty @ crate::ir::Ty::Enum(_)) => Ok(ty),
                _ => Err("box/unbox needs an admitted core scalar or local enum".into()),
            };
        }
        let name = self.core_type_name(token)?;
        Ok(match name {
            "Boolean" => Ty::Bool,
            "Char" => Ty::Char,
            "SByte" => Ty::I1,
            "Byte" => Ty::U1,
            "Int16" => Ty::I2,
            "UInt16" => Ty::U2,
            "Int32" => Ty::I4,
            "UInt32" => Ty::U4,
            "Int64" => Ty::I8,
            "UInt64" => Ty::U8,
            _ => return Err("box/unbox needs an admitted core scalar or local enum".into()),
        })
    }

    fn core_type_name(&self, token: Token) -> Result<&'a str> {
        if token.table() != Some(TableId::TypeRef) || token.rid() == 0 {
            return Err("operation needs a core TypeRef".into());
        }
        let row = self.md.tables().type_ref(token.rid())?;
        if row.resolution_scope.table() != Some(TableId::AssemblyRef)
            || row.resolution_scope.rid() == 0
            || self.md.strings().str(row.type_namespace)? != "System"
        {
            return Err("unresolved core type identity".into());
        }
        let assembly = self.md.tables().assembly_ref(row.resolution_scope.rid())?;
        let name = self.md.strings().str(assembly.name)?;
        let key = self.md.blobs().get(assembly.public_key_or_token)?;
        let core = match name {
            "System.Runtime" => key == [0xb0, 0x3f, 0x5f, 0x7f, 0x11, 0xd5, 0x0a, 0x3a],
            "System.Private.CoreLib" => key == [0x7c, 0xec, 0x85, 0xd7, 0xbe, 0xa7, 0x79, 0x8e],
            "mscorlib" => key == [0xb7, 0x7a, 0x5c, 0x56, 0x19, 0x34, 0xe0, 0x89],
            "netstandard" => key == [0xcc, 0x7b, 0x13, 0xff, 0xcd, 0x2d, 0xdd, 0x51],
            _ => false,
        };
        if !core
            || assembly.flags & 0x0e01 != 0
            || !self.md.strings().str(assembly.culture)?.is_empty()
        {
            return Err("type scope is not an admitted core identity".into());
        }
        Ok(self.md.strings().str(row.type_name)?)
    }

    pub fn array_element_type(&self, token: Token) -> Result<crate::ir::Ty> {
        use crate::ir::Ty;
        match self.core_type_name(token)? {
            "Int32" => Ok(Ty::I4),
            "UInt32" => Ok(Ty::U4),
            _ => Err("array element type needs IL fallback".into()),
        }
    }

    pub fn external_class_context(&self, token: u32) -> Result<String> {
        let t = Token(token);
        if t.table() != Some(TableId::TypeRef) || t.rid() == 0 {
            return Err("opaque external class needs a TypeRef".into());
        }
        let row = self.md.tables().type_ref(t.rid())?;
        let name = self.md.strings().str(row.type_name)?;
        if name.is_empty()
            || name.contains('`')
            || row.resolution_scope.table() != Some(TableId::AssemblyRef)
            || row.resolution_scope.rid() == 0
        {
            return Err("nested/generic/unresolved external class needs IL fallback".into());
        }
        self.type_reference(t, 0)
    }

    /// A bounded set of core catch types whose runtime identities are fixed.
    pub(crate) fn handler_type_name(&self, token: Token) -> Result<String> {
        let name = self
            .core_type_name(token)
            .map_err(|_| "EH catch type needs an admitted core exception identity")?;
        if !matches!(
            name,
            "Exception"
                | "SystemException"
                | "ArithmeticException"
                | "DivideByZeroException"
                | "OverflowException"
                | "ArgumentException"
                | "ArgumentNullException"
                | "ArgumentOutOfRangeException"
                | "NullReferenceException"
                | "InvalidOperationException"
                | "IndexOutOfRangeException"
        ) {
            return Err("EH catch type needs an admitted core exception identity".into());
        }
        Ok(format!("global::System.{name}"))
    }

    pub fn external_class_name(&self, token: u32) -> Result<String> {
        self.external_class_context(token)?;
        if let Ok(name) = self.handler_type_name(Token(token)) {
            return Ok(name);
        }
        Ok(format!("__cil_type_{token:08X}"))
    }

    pub fn core_exception_name(&self, token: u32) -> Result<String> {
        if self.core_type_name(Token(token))? != "ArgumentOutOfRangeException" {
            return Err(
                "exception type needs exact core ArgumentOutOfRangeException identity".into(),
            );
        }
        Ok("global::System.ArgumentOutOfRangeException".into())
    }

    pub fn exception_constructor(&self, token: Token) -> Result<crate::ir::Ty> {
        if token.table() != Some(TableId::MemberRef) || token.rid() == 0 {
            return Err("newobj needs an admitted core exception constructor".into());
        }
        let member = self.md.tables().member_ref(token.rid())?;
        self.core_exception_name(member.class.0)?;
        if self.md.strings().str(member.name)? != ".ctor"
            || self.md.blobs().get(member.signature)? != [0x20, 0x02, 0x01, 0x0e, 0x0e]
        {
            return Err("newobj needs exact instance void exception .ctor(string, string)".into());
        }
        Ok(crate::ir::Ty::ExternalClass(member.class.0))
    }

    /// Positional arguments represent a named receiver only in their original
    /// declaring scope or through a proved identity-mapped local base chain.
    fn identity_member_owner(
        &self,
        definition: Token,
        args: &[cildec::Type<'a>],
        context: crate::signature_types::TypeContext,
    ) -> Result<bool> {
        let arity = self.md.tables().generic_params(definition)?.len();
        if arity != args.len() || arity == 0 {
            return Err("constructed member arity differs from its definition".into());
        }
        if (context.type_scope.count as usize) < arity
            || !args.iter().enumerate().all(|(index, arg)|
                matches!(arg, cildec::Type::Var(position) if *position as usize == index))
        {
            return Ok(false);
        }
        let mut current = Token(context.type_scope.owner);
        let mut seen = Vec::new();
        for _ in 0..64 {
            if current == definition {
                return Ok(true);
            }
            if current.table() != Some(TableId::TypeDef) || current.rid() == 0 {
                return Ok(false);
            }
            if seen.contains(&current) {
                return Err("cyclic identity-mapped member hierarchy needs IL fallback".into());
            }
            seen.push(current);
            let base = self.md.tables().type_def(current.rid())?.extends;
            if base.table() != Some(TableId::TypeSpec) {
                return Ok(false);
            }
            let signature = self.type_spec(base)?;
            let cildec::Type::GenericInst {
                is_value_type: false,
                def,
                args: base_args,
            } = signature.type_
            else {
                return Ok(false);
            };
            if base_args.len() < arity
                || !base_args.iter().enumerate().all(|(index, arg)|
                    matches!(arg, cildec::Type::Var(position) if *position as usize == index))
            {
                return Ok(false);
            }
            let Some(base) = self.local_type_definition(def)? else {
                return Ok(false);
            };
            if self.md.tables().generic_params(base)?.len() != base_args.len() {
                return Err("constructed base arity differs from its definition".into());
            }
            current = base;
        }
        Err("identity-mapped member hierarchy depth exceeded".into())
    }

    /// Receiver and arguments for a locally defined constructed class member.
    /// Identity-mapped self/base owners retain their named representation; all
    /// other instantiations carry structural identity and caller-scoped arguments.
    fn constructed_member_owner(
        &self,
        parent: Token,
        context: crate::signature_types::TypeContext,
    ) -> Result<Option<ConstructedOwner<'a>>> {
        if parent.table() != Some(TableId::TypeSpec) {
            return Ok(None);
        }
        let spec = self.type_spec(parent)?;
        let cildec::Type::GenericInst {
            is_value_type: false,
            def,
            args,
        } = &spec.type_
        else {
            return Err("constructed member parent is not a class instantiation".into());
        };
        let definition = self.local_type_definition(*def)?.ok_or(
            "constructed member needs a local definition; external constructed members need dependency facts",
        )?;
        // Abstract classes still declare callable members and base constructors.
        // Only allocation requires a concrete owner; that check is separate.
        if self.md.tables().type_def(definition.rid())?.flags & 0x20 != 0 {
            return Err(
                "constructed member parent must be a class; interface dispatch needs IL fallback"
                    .into(),
            );
        }
        let open_self = self.identity_member_owner(definition, args, context)?;
        if open_self {
            return Ok(Some(ConstructedOwner {
                definition,
                receiver: crate::ir::Ty::Class(definition.0),
                arguments: None,
            }));
        }
        let receiver =
            crate::ir::Ty::StructuralRef(self.intern_signature_type(&spec.type_, context)?);
        Ok(Some(ConstructedOwner {
            definition,
            receiver,
            arguments: Some(args.clone()),
        }))
    }

    /// The single constructor of `definition` with this parameter count.
    fn find_constructor(&self, definition: Token, parameters: usize) -> Result<Token> {
        let tables = self.md.tables();
        let mut found = None;
        for rid in tables.method_range(Rid::new(definition.rid()))? {
            let row = tables.method_def(rid)?;
            if self.md.strings().str(row.name)? != ".ctor" || row.flags & 0x10 != 0 {
                continue;
            }
            let signature = crate::signature_guard::method(self.md.blobs().get(row.signature)?)?;
            if signature.params.len() != parameters {
                continue;
            }
            if found.is_some() {
                return Err("constructor overload selection needs IL fallback".into());
            }
            found = Some(Token(0x06000000 | rid));
        }
        found.ok_or_else(|| "constructed allocation names no constructor of its definition".into())
    }

    pub(crate) fn allocation_constructor(
        &self,
        token: Token,
        context: crate::signature_types::TypeContext,
    ) -> Result<(crate::ir::Ty, Vec<crate::ir::Ty>)> {
        let (token, allocated, arguments) = match token.table() {
            Some(TableId::MethodDef) if token.rid() != 0 => (token, None, None),
            Some(TableId::MemberRef) => {
                let row = self.md.tables().member_ref(token.rid())?;
                if self.md.strings().str(row.name)? != ".ctor" {
                    return Err("allocation needs a constructor reference".into());
                }
                let reference =
                    crate::signature_guard::method(self.md.blobs().get(row.signature)?)?;
                match self.constructed_member_owner(row.class, context)? {
                    Some(owner) => {
                        let constructor =
                            self.find_constructor(owner.definition, reference.params.len())?;
                        (constructor, Some(owner.receiver), owner.arguments)
                    }
                    None => {
                        return self.external_allocation_for(token, row.class, &reference);
                    }
                }
            }
            _ => return Err("allocation needs a local constructor definition".into()),
        };
        let row = self.row(token.0)?;
        if self.md.strings().str(row.name)? != ".ctor"
            || row.is_static()
            || row.flags & 0x1800 != 0x1800
            || row.flags & 0x440 != 0
        {
            return Err("allocation needs an ordinary instance constructor".into());
        }
        let owner = self
            .md
            .tables()
            .type_of_method(Rid::new(token.rid()))?
            .ok_or("constructor has no declaring type")?;
        let owner = Token(0x02000000 | owner.get());
        let definition = self.md.tables().type_def(owner.rid())?;
        // A sealed class is concrete and instantiable; a static class is
        // `abstract | sealed` and is still refused by the abstract bit.
        if definition.flags & (0x20 | 0x80) != 0 {
            return Err("allocation owner must be a concrete class".into());
        }
        match &arguments {
            Some(arguments) => {
                if self.md.tables().generic_params(owner)?.len() != arguments.len() {
                    return Err("allocation instantiation arity differs from its definition".into());
                }
            }
            None => {
                if self.generic_declaring_context(owner)? {
                    return Err("allocation owner must be a concrete nongeneric class".into());
                }
            }
        }
        self.class_name(owner.0)?;
        let sig = self.signature(token.0)?;
        if !sig.has_this
            || sig.calling_convention != cildec::CallConv::Default
            || !sig.vararg_params.is_empty()
            || crate::ir::Ty::from_param(&sig.return_type, self)? != crate::ir::Ty::Void
        {
            return Err("allocation constructor signature needs IL fallback".into());
        }
        let args = sig
            .params
            .iter()
            .map(|p| self.member_parameter(p, arguments.as_deref(), context))
            .collect::<Result<Vec<_>>>()?;
        Ok((allocated.unwrap_or(crate::ir::Ty::Class(owner.0)), args))
    }

    /// Admit `newobj` whose owner lives in a referenced image.
    ///
    /// The parameters are read in the caller's vocabulary so the same `assignable`
    /// checks apply as for a local constructor; only the owner's kind, name and
    /// shape come from the defining image.
    fn external_allocation_for(
        &self,
        token: Token,
        parent: Token,
        reference: &cildec::MethodSig<'_>,
    ) -> Result<(crate::ir::Ty, Vec<crate::ir::Ty>)> {
        let fact = match self.external_allocation(token.0) {
            Some(Ok(fact)) => fact,
            Some(Err(reason)) => {
                return Err(format!("external allocation needs dependency facts: {reason}").into());
            }
            None => {
                return Err(
                    "external allocation needs dependency facts; only local classes and local instantiations are admitted"
                        .into(),
                );
            }
        };
        if fact.parameter_count != reference.params.len() {
            return Err("external allocation argument count differs from its reference".into());
        }
        let spec = parent.table() == Some(TableId::TypeSpec);
        if fact.constructed != spec {
            return Err("external allocation owner shape differs from its reference".into());
        }
        let owner = crate::ir::Ty::ExternalClass(parent.0);
        let arguments = reference
            .params
            .iter()
            .map(|parameter| crate::ir::Ty::from_param(parameter, self))
            .collect::<Result<Vec<_>>>()?;
        Ok((owner, arguments))
    }

    /// Cast target with locally verified class facts or an admitted core identity.
    pub(crate) fn cast_class(
        &self,
        token: Token,
        context: crate::signature_types::TypeContext,
    ) -> Result<crate::ir::Ty> {
        if token.table() == Some(TableId::TypeSpec) {
            let spec = self.type_spec(token)?;
            if matches!(
                spec.type_,
                cildec::Type::GenericInst {
                    is_value_type: false,
                    ..
                } | cildec::Type::SzArray(..)
            ) {
                let ty = crate::ir::Ty::from_sig_scoped(&spec.type_, self, context)?;
                if ty.kind() == crate::ir::StackKind::Ref {
                    return Ok(ty);
                }
            }
            return Err("cast TypeSpec needs a proved reference shape".into());
        }
        match self.core_type_name(token) {
            Ok("String") => return Ok(crate::ir::Ty::String),
            Ok("Object") => return Ok(crate::ir::Ty::Object),
            _ => {}
        }
        if token.table() != Some(TableId::TypeDef) || token.rid() == 0 {
            return Err(
                "cast needs a local class definition; external/interface casts need more facts"
                    .into(),
            );
        }
        if self.generic_declaring_context(token)? {
            return Err("cast target must be a nongeneric reference type".into());
        }
        self.class_name(token.0)?;
        Ok(crate::ir::Ty::Class(token.0))
    }

    /// Reference-type unbox.any is castclass; it does not copy a scalar from a box.
    pub(crate) fn reference_unbox_target(
        &self,
        token: Token,
        context: crate::signature_types::TypeContext,
    ) -> Result<Option<crate::ir::Ty>> {
        let reference = match token.table() {
            Some(TableId::TypeSpec) => matches!(
                self.type_spec(token)?.type_,
                cildec::Type::GenericInst {
                    is_value_type: false,
                    ..
                } | cildec::Type::SzArray(..)
            ),
            Some(TableId::TypeRef) => matches!(self.core_type_name(token), Ok("String" | "Object")),
            Some(TableId::TypeDef) => self.class_name(token.0).is_ok(),
            _ => false,
        };
        if reference {
            self.cast_class(token, context).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn receiver(&self, method: u32) -> Result<crate::ir::Ty> {
        let owner = self
            .md
            .tables()
            .type_of_method(Rid::new(method & 0xffffff))?
            .ok_or("missing declaring type")?;
        let token = 0x02000000 | owner.get();
        if self.local_interface(token)? && self.md.strings().str(self.row(method)?.name)? == ".ctor"
        {
            return Err("interface instance constructors are invalid".into());
        }
        // A value-type instance method receives a managed pointer to the caller's
        // storage; its receiver slot is a location, never a copied value.
        if let Some(id) = self.struct_type(Token(token))? {
            return Ok(crate::ir::Ty::ValueAddress(id));
        }
        self.class_name(token)?;
        Ok(crate::ir::Ty::Class(token))
    }

    pub(crate) fn address_of(&self, element: crate::ir::Ty) -> Result<crate::ir::Ty> {
        use crate::ir::{StackKind, Ty};
        if element.kind() == StackKind::Address || matches!(element, Ty::Void | Ty::Null | Ty::F4) {
            return Err("managed location needs an ordinary stored element type".into());
        }
        element.cs(self)?;
        Ok(match element {
            Ty::I4 => Ty::I4Address,
            Ty::U4 => Ty::U4Address,
            Ty::I8 => Ty::I8Address,
            Ty::U8 => Ty::U8Address,
            Ty::Value(id) => Ty::ValueAddress(id),
            other => Ty::ManagedAddress(self.location_types.borrow_mut().intern(other)?),
        })
    }

    pub(crate) fn address_element(&self, address: crate::ir::Ty) -> Result<crate::ir::Ty> {
        use crate::ir::Ty;
        match address {
            Ty::I4Address => Ok(Ty::I4),
            Ty::U4Address => Ok(Ty::U4),
            Ty::I8Address => Ok(Ty::I8),
            Ty::U8Address => Ok(Ty::U8),
            Ty::ValueAddress(id) => Ok(Ty::Value(id)),
            Ty::ManagedAddress(id) => self.location_types.borrow().element(id),
            _ => Err("operation needs an acquired managed location".into()),
        }
    }

    fn primitive_storage_type(&self, token: Token) -> Result<Option<crate::ir::Ty>> {
        use crate::ir::Ty;
        let name = match token.table() {
            Some(TableId::TypeDef) => self.local_core_type_name(token)?,
            Some(TableId::TypeRef) => self.core_type_name(token).ok(),
            _ => None,
        };
        Ok(match name {
            Some("Boolean") => Some(Ty::Bool),
            Some("Char") => Some(Ty::Char),
            Some("SByte") => Some(Ty::I1),
            Some("Byte") => Some(Ty::U1),
            Some("Int16") => Some(Ty::I2),
            Some("UInt16") => Some(Ty::U2),
            Some("Int32") => Some(Ty::I4),
            Some("UInt32") => Some(Ty::U4),
            Some("Int64") => Some(Ty::I8),
            Some("UInt64") => Some(Ty::U8),
            Some("IntPtr") => Some(Ty::Native),
            Some("UIntPtr") => Some(Ty::UNative),
            Some("Single") => Some(Ty::R4),
            Some("Object") => Some(Ty::Object),
            Some("String") => Some(Ty::String),
            _ => None,
        })
    }

    fn external_receiver_type(&self, owner: Token) -> Result<crate::ir::Ty> {
        use crate::ir::{StackKind, Ty};
        if let Some(ty) = self.primitive_storage_type(owner)?
            && ty.kind() != StackKind::Ref
        {
            return Ok(ty);
        }
        if let Ok(enumeration) = self.enum_type(owner) {
            return Ok(Ty::Enum(enumeration));
        }
        if let Some(id) = self.struct_type(owner)? {
            return Ok(Ty::Value(id));
        }
        self.external_class_context(owner.0)?;
        Ok(Ty::ExternalClass(owner.0))
    }

    pub(crate) fn location_operand(
        &self,
        address: crate::ir::Ty,
        token: Token,
        context: crate::signature_types::TypeContext,
    ) -> Result<crate::ir::Ty> {
        use crate::ir::{StackKind, Ty};
        let element = self.address_element(address)?;
        let core = match token.table() {
            Some(TableId::TypeDef) => self.local_core_type_name(token)?,
            Some(TableId::TypeRef) => self.core_type_name(token).ok(),
            _ => None,
        };
        let primitive = match core {
            Some("Boolean") => Some(Ty::Bool),
            Some("Char") => Some(Ty::Char),
            Some("SByte") => Some(Ty::I1),
            Some("Byte") => Some(Ty::U1),
            Some("Int16") => Some(Ty::I2),
            Some("UInt16") => Some(Ty::U2),
            Some("Int32") => Some(Ty::I4),
            Some("UInt32") => Some(Ty::U4),
            Some("Int64") => Some(Ty::I8),
            Some("UInt64") => Some(Ty::U8),
            Some("IntPtr") => Some(Ty::Native),
            Some("UIntPtr") => Some(Ty::UNative),
            Some("Single") => Some(Ty::R4),
            Some("Object") => Some(Ty::Object),
            Some("String") => Some(Ty::String),
            _ => None,
        };
        let operand = match (primitive, token.table()) {
            (Some(ty), _) => ty,
            (_, Some(TableId::TypeSpec)) => {
                Ty::from_sig_scoped(&self.type_spec(token)?.type_, self, context)?
            }
            (_, Some(TableId::TypeDef | TableId::TypeRef)) => {
                let ty = if element.kind() == StackKind::Ref {
                    cildec::Type::Class(token)
                } else {
                    cildec::Type::ValueType(token)
                };
                Ty::from_sig_scoped(&ty, self, context)?
            }
            _ => return Err("location operand needs an exact storage type".into()),
        };
        if operand != element
            && !(operand.kind() == StackKind::Ref
                && element.kind() == StackKind::Ref
                && self.assignable(operand, element).is_ok()
                && self.assignable(element, operand).is_ok())
        {
            return Err("location operand differs from its exact storage element".into());
        }
        Ok(element)
    }

    fn intrinsic_reference_name(&self, ty: crate::ir::Ty) -> Result<Option<&'a str>> {
        use crate::ir::Ty;
        let name = match ty {
            Ty::String => Some("String"),
            Ty::Object => Some("Object"),
            Ty::Class(token) => self.local_core_type_name(Token(token))?,
            Ty::ExternalClass(token) => self.core_type_name(Token(token)).ok(),
            _ => None,
        };
        Ok(name.filter(|name| matches!(*name, "String" | "Object" | "Array")))
    }

    pub fn assignable(&self, actual: crate::ir::Ty, expected: crate::ir::Ty) -> Result<()> {
        use crate::ir::Ty;
        if actual == expected
            || (expected == Ty::Object && actual.kind() == crate::ir::StackKind::Ref)
            || (actual == Ty::Null
                && matches!(
                    expected,
                    Ty::String
                        | Ty::ExternalClass(_)
                        | Ty::StructuralRef(_)
                        | Ty::Class(_)
                        | Ty::I4Array
                        | Ty::U4Array
                ))
        {
            return Ok(());
        }
        if let Some(expected_name) = self.intrinsic_reference_name(expected)? {
            if expected_name == "Object" && actual.kind() == crate::ir::StackKind::Ref
                || self.intrinsic_reference_name(actual)? == Some(expected_name)
            {
                return Ok(());
            }
            if expected_name == "Array" {
                let vector = matches!(actual, Ty::I4Array | Ty::U4Array)
                    || if let Ty::StructuralRef(id) = actual {
                        self.signature_types.borrow().is_vector(id)?
                    } else {
                        false
                    };
                if vector {
                    return Ok(());
                }
            }
        }
        if actual.kind() == crate::ir::StackKind::Address
            && expected.kind() == crate::ir::StackKind::Address
        {
            if self.address_element(actual)? == self.address_element(expected)? {
                return Ok(());
            }
            return Err("managed location element identity differs".into());
        }
        if let (Ty::ValueAddress(a), Ty::Value(b)) = (actual, expected) {
            if a == b {
                return Ok(());
            }
        }
        if let (Ty::Class(a), Ty::Class(b)) = (actual, expected) {
            if self.local_interface(b)? {
                if self.implements_interface(a, b)? {
                    return Ok(());
                }
            } else if !self.local_interface(a)? && self.class_chain(a)?.contains(&b) {
                return Ok(());
            }
        }
        // `this` inside `C<T>` is the open self instantiation of `C`, so a member
        // typed `C<!0..>` and the receiver are the same type.
        if let (Ty::Class(a), Ty::StructuralRef(b)) = (actual, expected) {
            if self.open_self_definition(b) == Some(a) {
                return Ok(());
            }
        }
        if let (Ty::Class(a), Ty::ExternalClass(b)) = (actual, expected) {
            self.external_class_context(b)?;
            for owner in self.class_chain(a)? {
                if self.md.tables().type_def(owner & 0xffffff)?.extends.0 == b {
                    return Ok(());
                }
            }
        }
        Err("incompatible value/receiver types".into())
    }

    /// Resolve a field operand.
    ///
    /// A field of a generic type is usually named through a MemberRef whose parent
    /// is the constructed type, so the definition's signature names the declaring
    /// type's parameters and the site supplies their arguments.
    pub fn field(
        &self,
        t: Token,
        context: crate::signature_types::TypeContext,
    ) -> Result<FieldRef> {
        match t.table() {
            Some(TableId::Field) => {
                let row = self.md.tables().field(t.rid())?;
                let owner = self
                    .md
                    .tables()
                    .type_of_field(Rid::new(t.rid()))?
                    .ok_or("missing field owner")?;
                let owner_token = 0x02000000 | owner.get();
                let owner = match self.struct_type(Token(owner_token))? {
                    Some(id) => crate::ir::Ty::Value(id),
                    None => crate::ir::Ty::Class(owner_token),
                };
                let name = self.md.strings().str(row.name)?;
                let blob = self.md.blobs().get(row.signature)?;
                self.field_from_parts(
                    t.0,
                    name,
                    context,
                    FieldParts {
                        flags: row.flags,
                        signature: blob,
                        owner,
                        arguments: None,
                    },
                )
            }
            Some(TableId::MemberRef) => {
                let row = self.md.tables().member_ref(t.rid())?;
                let name = self.md.strings().str(row.name)?.to_string();
                match row.class.table() {
                    Some(TableId::TypeDef | TableId::TypeRef) => {
                        let Some(definition) = self.local_type_definition(row.class)? else {
                            return match self.external_field(t.0) {
                                Some(Ok(fact)) => {
                                    let blob = self.md.blobs().get(row.signature)?;
                                    self.field_from_parts(
                                        t.0,
                                        &fact.name,
                                        context,
                                        FieldParts {
                                            flags: fact.flags,
                                            signature: blob,
                                            owner: crate::ir::Ty::ExternalClass(row.class.0),
                                            arguments: None,
                                        },
                                    )
                                }
                                Some(Err(reason)) => Err(format!(
                                    "external member reference needs dependency facts: {reason}"
                                )
                                .into()),
                                None => {
                                    Err("external member reference needs dependency facts".into())
                                }
                            };
                        };
                        let field = self.find_field(definition, &name)?;
                        let owner = match self.struct_type(definition)? {
                            Some(id) => crate::ir::Ty::Value(id),
                            None => crate::ir::Ty::Class(definition.0),
                        };
                        let field_row = self.md.tables().field(field.rid())?;
                        let blob = self.md.blobs().get(field_row.signature)?;
                        self.field_from_parts(
                            field.0,
                            &name,
                            context,
                            FieldParts {
                                flags: field_row.flags,
                                signature: blob,
                                owner,
                                arguments: None,
                            },
                        )
                    }
                    Some(TableId::TypeSpec) => {
                        let spec = self.type_spec(row.class)?;
                        let cildec::Type::GenericInst {
                            is_value_type: false,
                            def,
                            args,
                        } = &spec.type_
                        else {
                            return Err(
                                "constructed member parent is not a class instantiation".into()
                            );
                        };
                        let definition = self.local_type_definition(*def)?.ok_or(
                            "constructed member needs a local definition; external constructed members need dependency facts",
                        )?;
                        let field = self.find_field(definition, &name)?;
                        let field_row = self.md.tables().field(field.rid())?;
                        let blob = self.md.blobs().get(field_row.signature)?;
                        // `C<!0..!n-1>` is the open self instantiation that instance
                        // members of `C` name; it is the declaring type itself, not a
                        // constructed one, so no argument substitution applies.
                        let open_self = self.identity_member_owner(definition, args, context)?;
                        if open_self {
                            let owner = match self.struct_type(definition)? {
                                Some(id) => crate::ir::Ty::Value(id),
                                None => crate::ir::Ty::Class(definition.0),
                            };
                            return self.field_from_parts(
                                field.0,
                                &name,
                                context,
                                FieldParts {
                                    flags: field_row.flags,
                                    signature: blob,
                                    owner,
                                    arguments: None,
                                },
                            );
                        }
                        let owner = crate::ir::Ty::StructuralRef(
                            self.intern_signature_type(&spec.type_, context)?,
                        );
                        self.field_from_parts(
                            field.0,
                            &name,
                            context,
                            FieldParts {
                                flags: field_row.flags,
                                signature: blob,
                                owner,
                                arguments: Some(args),
                            },
                        )
                    }
                    _ => Err("nonlocal/constructed field needs IL fallback".into()),
                }
            }
            _ => Err("nonlocal/constructed field needs IL fallback".into()),
        }
    }

    fn field_from_parts(
        &self,
        token: u32,
        name: &str,
        context: crate::signature_types::TypeContext,
        parts: FieldParts<'a>,
    ) -> Result<FieldRef> {
        let FieldParts {
            flags,
            signature,
            owner,
            arguments,
        } = parts;
        if flags & 0x10 == 0
            && let crate::ir::Ty::Class(token) = owner
            && self.local_interface(token)?
        {
            return Err("interface instance fields are invalid".into());
        }
        let signature = crate::signature_guard::field(signature)?;
        if !signature.custom_mods.is_empty() {
            return Err("modified field needs IL fallback".into());
        }
        let ty = match arguments {
            Some(arguments) => {
                let substituted = substitute_type_arguments(&signature.type_, arguments)?;
                crate::ir::Ty::from_sig_scoped(&substituted, self, context)?
            }
            None => crate::ir::Ty::from_sig_scoped(&signature.type_, self, context)?,
        };
        ty.cs(self)?;
        Ok(FieldRef {
            token,
            owner,
            ty,
            name: field_name(token, name),
            is_static: flags & 0x10 != 0,
            readonly: flags & 0x20 != 0,
        })
    }

    fn member_parameter(
        &self,
        parameter: &cildec::Param<'_>,
        arguments: Option<&[cildec::Type<'a>]>,
        context: crate::signature_types::TypeContext,
    ) -> Result<crate::ir::Ty> {
        if !parameter.custom_mods.is_empty() {
            return Err("byref/modified parameter needs IL fallback".into());
        }
        let ty = match arguments {
            Some(arguments) => {
                let substituted = substitute_type_arguments(&parameter.type_, arguments)?;
                crate::ir::Ty::from_sig_scoped(&substituted, self, context)?
            }
            None => crate::ir::Ty::from_sig_scoped(&parameter.type_, self, context)?,
        };
        if parameter.by_ref {
            self.address_of(ty)
        } else {
            Ok(ty)
        }
    }

    fn find_field(&self, definition: Token, name: &str) -> Result<Token> {
        let tables = self.md.tables();
        let mut found = None;
        for rid in tables.field_range(Rid::new(definition.rid()))? {
            let field = tables.field(rid)?;
            if self.md.strings().str(field.name)? != name {
                continue;
            }
            if found.is_some() {
                return Err("constructed definition declares the field name twice".into());
            }
            found = Some(Token(0x04000000 | rid));
        }
        found.ok_or_else(|| "constructed member names no field of its definition".into())
    }

    /// The local definition a type's `extends` column names, following a
    /// constructed base to its definition.
    pub(crate) fn base_definition(&self, base: Token) -> Result<Option<u32>> {
        if base.rid() == 0 {
            return Ok(None);
        }
        if base.table() != Some(TableId::TypeSpec) {
            return Ok(Some(base.0));
        }
        let spec = self.type_spec(base)?;
        let cildec::Type::GenericInst {
            is_value_type: false,
            def,
            ..
        } = &spec.type_
        else {
            return Err("constructed base needs member binding".into());
        };
        Ok(self
            .local_type_definition(*def)?
            .map(|definition| definition.0))
    }

    fn type_spec(&self, token: Token) -> Result<cildec::TypeSpecSig<'a>> {
        let row = self.md.tables().type_spec(token.rid())?;
        crate::signature_guard::type_spec(self.md.blobs().get(row.signature)?)
    }

    pub fn method_ref(
        &self,
        t: Token,
        context: crate::signature_types::TypeContext,
    ) -> Result<MethodRef> {
        if t.table() == Some(TableId::MethodSpec) {
            return method_instantiations::resolve(self, t, context);
        }
        let (name, owner, sig) = match t.table() {
            Some(TableId::MethodDef) => {
                let row = self.row(t.0)?;
                let owner = self
                    .md
                    .tables()
                    .type_of_method(Rid::new(t.rid()))?
                    .ok_or("missing method owner")?;
                (
                    self.md.strings().str(row.name)?.to_string(),
                    Token(0x02000000 | owner.get()),
                    self.signature(t.0)?,
                )
            }
            Some(TableId::MemberRef) => {
                let row = self.md.tables().member_ref(t.rid())?;
                (
                    self.md.strings().str(row.name)?.to_string(),
                    row.class,
                    crate::signature_guard::method(self.md.blobs().get(row.signature)?)?,
                )
            }
            _ => return Err("generic/indirect method reference needs IL fallback".into()),
        };
        if sig.explicit_this
            || sig.calling_convention != cildec::CallConv::Default
            || !sig.vararg_params.is_empty()
        {
            return Err("instance/generic/vararg call needs IL fallback".into());
        }
        if name == ".ctor" && t.table() == Some(TableId::MemberRef) {
            let reference = self.md.tables().member_ref(t.rid())?;
            // A base constructor of a constructed base (`Base<T>`) resolves against
            // the local definition and keeps the instantiation for its signature.
            if let Some(owner) = self.constructed_member_owner(reference.class, context)? {
                let signature =
                    crate::signature_guard::method(self.md.blobs().get(reference.signature)?)?;
                if !signature.has_this
                    || crate::ir::Ty::from_param(&signature.return_type, self)?
                        != crate::ir::Ty::Void
                {
                    return Err(
                        "constructed constructor needs an ordinary instance void signature".into(),
                    );
                }
                let args = signature
                    .params
                    .iter()
                    .map(|p| self.member_parameter(p, owner.arguments.as_deref(), context))
                    .collect::<Result<Vec<_>>>()?;
                return Ok(MethodRef {
                    token: t.0,
                    name: name.clone(),
                    receiver: Some(owner.receiver),
                    target: format!("__cil_ctor_call_{:08X}", t.0),
                    args,
                    ret: crate::ir::Ty::Void,
                    core_object_constructor: None,
                    tick_count: false,
                    constructor: true,
                    generic_virtual: false,
                    exact_reference: false,
                });
            }
            self.external_class_context(reference.class.0)?;
            let declaring_type = self.md.tables().type_ref(reference.class.rid())?;
            if self.md.strings().str(declaring_type.type_namespace)? == "System"
                && self.md.strings().str(declaring_type.type_name)? == "Object"
            {
                self.core_type_name(reference.class)?;
                if sig.raw != [0x20, 0x00, 0x01] {
                    return Err(
                        "core Object constructor needs exact instance void signature".into(),
                    );
                }
                return Ok(MethodRef {
                    token: t.0,
                    name: name.clone(),
                    receiver: None,
                    target: format!("__cil_core_object_ctor_{:08X}", t.0),
                    args: Vec::new(),
                    ret: crate::ir::Ty::Void,
                    core_object_constructor: Some(reference.class.0),
                    tick_count: false,
                    constructor: false,
                    generic_virtual: false,
                    exact_reference: false,
                });
            }
            if !sig.has_this
                || crate::ir::Ty::from_param(&sig.return_type, self)? != crate::ir::Ty::Void
            {
                return Err(
                    "external constructor needs an ordinary instance void signature".into(),
                );
            }
            return Ok(MethodRef {
                token: t.0,
                name: name.clone(),
                receiver: Some(crate::ir::Ty::ExternalClass(reference.class.0)),
                target: format!("__cil_ctor_call_{:08X}", t.0),
                args: sig
                    .params
                    .iter()
                    .map(|p| crate::ir::Ty::from_param(p, self))
                    .collect::<Result<Vec<_>>>()?,
                ret: crate::ir::Ty::Void,
                core_object_constructor: None,
                tick_count: false,
                constructor: true,
                generic_virtual: false,
                exact_reference: false,
            });
        }
        if name == ".ctor" && t.table() == Some(TableId::MethodDef) {
            let row = self.row(t.0)?;
            if !sig.has_this
                || row.is_static()
                || row.flags & 0x1800 != 0x1800
                || row.flags & 0x440 != 0
                || crate::ir::Ty::from_param(&sig.return_type, self)? != crate::ir::Ty::Void
            {
                return Err("invalid constructor call context needs IL fallback".into());
            }
            return Ok(MethodRef {
                token: t.0,
                name: name.clone(),
                receiver: Some(self.receiver(t.0)?),
                target: format!("__cil_ctor_call_{:08X}", t.0),
                args: sig
                    .params
                    .iter()
                    .map(|p| crate::ir::Ty::from_param(p, self))
                    .collect::<Result<Vec<_>>>()?,
                ret: crate::ir::Ty::Void,
                core_object_constructor: None,
                tick_count: false,
                constructor: true,
                generic_virtual: false,
                exact_reference: false,
            });
        }
        if name == ".ctor" || name == ".cctor" {
            return Err("constructor call needs IL fallback".into());
        }
        if name == "get_TickCount" && t.table() == Some(TableId::MemberRef) {
            let reference = self.md.tables().member_ref(t.rid())?;
            if self.core_type_name(reference.class)? != "Environment"
                || sig.raw != [0x00, 0x00, 0x08]
            {
                return Err(
                    "clock call needs exact core Environment.TickCount identity/signature".into(),
                );
            }
            return Ok(MethodRef {
                token: t.0,
                name: name.clone(),
                receiver: None,
                target: format!("__cil_tick_count_{:08X}", t.0),
                args: Vec::new(),
                ret: crate::ir::Ty::I4,
                core_object_constructor: None,
                tick_count: true,
                constructor: false,
                generic_virtual: false,
                exact_reference: false,
            });
        }
        if name.is_empty()
            || t.table() == Some(TableId::MethodDef) && self.row(t.0)?.flags & 0x1000 != 0
        {
            return Err("empty/runtime-special method reference needs IL fallback".into());
        }
        if let Some(method) = reference_calls::constructed(self, t, context)? {
            return Ok(method);
        }
        // A member named through a local class instantiation (`C<int>` or the open
        // self `C<!0>`): the definition is local and concrete, so dispatch is
        // verified here and the declared signature is read with the site's
        // arguments substituted.
        if t.table() == Some(TableId::MemberRef)
            && let Some(owner) =
                self.constructed_member_owner(self.md.tables().member_ref(t.rid())?.class, context)?
        {
            let reference = self.md.tables().member_ref(t.rid())?;
            let signature =
                crate::signature_guard::method(self.md.blobs().get(reference.signature)?)?;
            if signature.explicit_this
                || signature.calling_convention != cildec::CallConv::Default
                || !signature.vararg_params.is_empty()
            {
                return Err("instance/generic/vararg call needs IL fallback".into());
            }
            let args = signature
                .params
                .iter()
                .map(|p| self.member_parameter(p, owner.arguments.as_deref(), context))
                .collect::<Result<Vec<_>>>()?;
            let ret =
                self.member_parameter(&signature.return_type, owner.arguments.as_deref(), context)?;
            return Ok(MethodRef {
                token: t.0,
                name: name.clone(),
                receiver: sig.has_this.then_some(owner.receiver),
                target: if sig.has_this {
                    format!("__cil_direct_{:08X}", t.0)
                } else {
                    format!("__cil_static_call_{:08X}", t.0)
                },
                args,
                ret,
                core_object_constructor: None,
                tick_count: false,
                constructor: false,
                generic_virtual: false,
                exact_reference: false,
            });
        }
        let receiver = if sig.has_this {
            if t.table() == Some(TableId::MemberRef) {
                let reference = self.md.tables().member_ref(t.rid())?;
                Some(self.external_receiver_type(reference.class)?)
            } else {
                let row = self.row(t.0)?;
                if row.is_static() {
                    return Err("inconsistent call signature".into());
                }
                Some(self.receiver(t.0)?)
            }
        } else {
            None
        };
        let args = sig
            .params
            .iter()
            .map(|p| crate::ir::Ty::from_param(p, self))
            .collect::<Result<Vec<_>>>()?;
        let ret = crate::ir::Ty::from_param(&sig.return_type, self)?;
        if t.table() == Some(TableId::MemberRef)
            && args
                .iter()
                .chain(std::iter::once(&ret))
                .any(|ty| ty.kind() == crate::ir::StackKind::Float)
        {
            let reference = self.md.tables().member_ref(t.rid())?;
            if self.core_type_name(reference.class)? != "Math"
                || name != "Abs"
                || sig.raw != [0x00, 0x01, 0x0c, 0x0c]
            {
                return Err(
                    "external float call needs exact core Math.Abs(float32) identity/signature"
                        .into(),
                );
            }
            return Ok(MethodRef {
                token: t.0,
                name: name.clone(),
                receiver: None,
                target: format!("__cil_abs_r4_{:08X}", t.0),
                args,
                ret,
                core_object_constructor: None,
                tick_count: false,
                constructor: false,
                generic_virtual: false,
                exact_reference: false,
            });
        }
        Ok(MethodRef {
            token: t.0,
            name: name.clone(),
            receiver,
            target: if receiver.is_some() && t.table() == Some(TableId::MemberRef) {
                format!("__cil_memberref_call_{:08X}", t.0)
            } else if receiver.is_some() {
                format!("__cil_direct_{:08X}", t.0)
            } else {
                if if owner.table() == Some(TableId::TypeDef) {
                    self.generic_declaring_context(owner)?
                } else {
                    Names::new(&self.md).try_token(owner)?.contains('`')
                } {
                    return Err("generic static call owner needs IL fallback".into());
                }
                format!(
                    "{}.{}",
                    crate::symbol_names::type_name(self, owner, true)?,
                    crate::symbol_names::method(self, t, &name, true)
                )
            },
            args,
            ret,
            core_object_constructor: None,
            tick_count: false,
            constructor: false,
            generic_virtual: false,
            exact_reference: false,
        })
    }

    /// A static field of this image, rendered with its declaring type's name.
    ///
    /// The declaring type token is reported so a write can be checked against the
    /// constructor context: only the declaring type's own static constructor may
    /// assign a `static readonly` field.
    pub fn static_field(&self, t: Token) -> Result<(String, crate::ir::Ty, bool, u32)> {
        if t.table() == Some(TableId::MemberRef) {
            // A static field named through a member reference has a declaring type
            // this image does not define, so neither its storage nor its name can be
            // written without more facts.
            let parent = self.md.tables().member_ref(t.rid())?.class;
            return Err(match parent.table() {
                Some(TableId::TypeSpec) => {
                    "generic static field needs the constructed owner name".into()
                }
                _ => "external field needs IL fallback".into(),
            });
        }
        if t.table() != Some(TableId::Field) {
            return Err("nonlocal static field needs IL fallback".into());
        }
        let row = self.md.tables().field(t.rid())?;
        if row.flags & 0x10 == 0 {
            return Err("expected static field".into());
        }
        let owner = self
            .md
            .tables()
            .type_of_field(Rid::new(t.rid()))?
            .ok_or("missing field owner")?;
        let name = field_name(t.0, self.md.strings().str(row.name)?);
        let sig = crate::signature_guard::field(self.md.blobs().get(row.signature)?)?;
        if !sig.custom_mods.is_empty() {
            return Err("modified field needs IL fallback".into());
        }
        if self.generic_declaring_context(Token(0x02000000 | owner.get()))? {
            return Err("generic static field owner needs IL fallback".into());
        }
        Ok((
            format!(
                "{}.{name}",
                crate::symbol_names::type_name(self, Token(0x02000000 | owner.get()), true)?
            ),
            crate::ir::Ty::from_sig_in(&sig.type_, self)?,
            row.flags & 0x20 != 0,
            0x02000000 | owner.get(),
        ))
    }
}

pub(crate) struct SelectedType {
    pub token: u32,
    pub name: String,
    pub methods: Vec<u32>,
    pub fields: Vec<u32>,
    pub properties: Vec<u32>,
    pub events: Vec<u32>,
}

/// A member reference resolved against a local class instantiation.
struct ConstructedOwner<'a> {
    definition: Token,
    receiver: crate::ir::Ty,
    /// Arguments the site supplied; `None` for the open self instantiation.
    arguments: Option<Vec<cildec::Type<'a>>>,
}

/// Everything a field resolution needs besides its token and name.
struct FieldParts<'a> {
    flags: u16,
    signature: &'a [u8],
    owner: crate::ir::Ty,
    /// Supplied arguments when the crate names a constructed declaring type.
    arguments: Option<&'a [cildec::Type<'a>]>,
}

/// Replace the declaring type's generic parameters with the arguments the site
/// supplied. Only `Var` positions of the declaring type are substituted; arguments
/// keep their own scope because they may name an enclosing method's parameters.
fn substitute_type_arguments<'a>(
    ty: &cildec::Type<'a>,
    arguments: &[cildec::Type<'a>],
) -> Result<cildec::Type<'a>> {
    substitute_arguments(ty, Some(arguments), None)
}

/// Substitute both declaration scopes in one traversal. Inserted arguments
/// already belong to the caller and must not be substituted a second time.
fn substitute_arguments<'a>(
    ty: &cildec::Type<'a>,
    type_arguments: Option<&[cildec::Type<'a>]>,
    method_arguments: Option<&[cildec::Type<'a>]>,
) -> Result<cildec::Type<'a>> {
    use cildec::Type;
    let recurse = |ty: &Type<'a>| substitute_arguments(ty, type_arguments, method_arguments);
    Ok(match ty {
        Type::Var(index) if type_arguments.is_some() => type_arguments
            .unwrap()
            .get(*index as usize)
            .cloned()
            .ok_or("constructed member argument missing")?,
        Type::MVar(index) if method_arguments.is_some() => method_arguments
            .unwrap()
            .get(*index as usize)
            .cloned()
            .ok_or("constructed method argument missing")?,
        Type::GenericInst {
            is_value_type,
            def,
            args,
        } => Type::GenericInst {
            is_value_type: *is_value_type,
            def: *def,
            args: args.iter().map(&recurse).collect::<Result<_>>()?,
        },
        Type::SzArray(modifiers, inner) => {
            Type::SzArray(modifiers.clone(), Box::new(recurse(inner)?))
        }
        Type::Array(inner, shape) => Type::Array(Box::new(recurse(inner)?), shape.clone()),
        Type::ByRef(inner) => Type::ByRef(Box::new(recurse(inner)?)),
        Type::Ptr(modifiers, inner) => Type::Ptr(modifiers.clone(), Box::new(recurse(inner)?)),
        Type::Modified(modifiers, inner) => {
            Type::Modified(modifiers.clone(), Box::new(recurse(inner)?))
        }
        Type::Pinned(inner) => Type::Pinned(Box::new(recurse(inner)?)),
        other => other.clone(),
    })
}

pub(crate) struct FieldRef {
    pub token: u32,
    pub owner: crate::ir::Ty,
    pub ty: crate::ir::Ty,
    pub name: String,
    pub is_static: bool,
    pub readonly: bool,
}

pub(crate) struct MethodRef {
    pub token: u32,
    pub name: String,
    pub receiver: Option<crate::ir::Ty>,
    pub target: String,
    pub args: Vec<crate::ir::Ty>,
    pub ret: crate::ir::Ty,
    pub core_object_constructor: Option<u32>,
    pub tick_count: bool,
    pub constructor: bool,
    pub generic_virtual: bool,
    pub exact_reference: bool,
}

fn generic_flags(flags: u16) -> String {
    let mut names = vec![match flags & 3 {
        0 => "invariant",
        1 => "covariant",
        2 => "contravariant",
        _ => "reserved-variance",
    }];
    for (bit, name) in [
        (4, "reference-type"),
        (8, "nonnullable-value-type"),
        (16, "default-constructor"),
        (32, "allow-byref-like"),
    ] {
        if flags & bit != 0 {
            names.push(name);
        }
    }
    names.join(" ")
}

fn method_flags(flags: u16) -> String {
    let mut names = vec![
        [
            "privatescope",
            "private",
            "famandassem",
            "assembly",
            "family",
            "famorassem",
            "public",
            "reserved-access",
        ][(flags & 7) as usize],
    ];
    for (bit, name) in [
        (0x10, "static"),
        (0x20, "final"),
        (0x40, "virtual"),
        (0x80, "hidebysig"),
        (0x100, "newslot"),
        (0x200, "strict"),
        (0x400, "abstract"),
        (0x800, "specialname"),
        (0x1000, "rtspecialname"),
        (0x2000, "pinvokeimpl"),
        (0x4000, "hassecurity"),
        (0x8000, "reqsecobj"),
    ] {
        if flags & bit != 0 {
            names.push(name);
        }
    }
    names.join(" ")
}

fn implementation_flags(flags: u16) -> String {
    let mut names = vec![
        ["cil", "native", "optil", "runtime"][(flags & 3) as usize],
        if flags & 4 == 0 {
            "managed"
        } else {
            "unmanaged"
        },
    ];
    for (bit, name) in [
        (0x8, "noinlining"),
        (0x10, "forwardref"),
        (0x20, "synchronized"),
        (0x40, "nooptimization"),
        (0x80, "preservesig"),
        (0x100, "aggressiveinlining"),
        (0x200, "aggressiveoptimization"),
        (0x1000, "internalcall"),
    ] {
        if flags & bit != 0 {
            names.push(name);
        }
    }
    names.join(" ")
}

fn constant_value(kind: u16, bytes: &[u8]) -> Result<String> {
    let invalid = || "invalid constant type or encoded value".into();
    Ok(match kind {
        0x02 if bytes.len() == 1 && bytes[0] <= 1 => format!("bool({})", bytes[0] != 0),
        0x03 if bytes.len() == 2 => {
            format!("char(0x{:04X})", u16::from_le_bytes(bytes.try_into()?))
        }
        0x04 if bytes.len() == 1 => format!("int8({})", bytes[0] as i8),
        0x05 if bytes.len() == 1 => format!("uint8({})", bytes[0]),
        0x06 if bytes.len() == 2 => format!("int16({})", i16::from_le_bytes(bytes.try_into()?)),
        0x07 if bytes.len() == 2 => format!("uint16({})", u16::from_le_bytes(bytes.try_into()?)),
        0x08 if bytes.len() == 4 => format!("int32({})", i32::from_le_bytes(bytes.try_into()?)),
        0x09 if bytes.len() == 4 => format!("uint32({})", u32::from_le_bytes(bytes.try_into()?)),
        0x0a if bytes.len() == 8 => format!("int64({})", i64::from_le_bytes(bytes.try_into()?)),
        0x0b if bytes.len() == 8 => format!("uint64({})", u64::from_le_bytes(bytes.try_into()?)),
        0x0c if bytes.len() == 4 => format!(
            "float32-bits(0x{:08X})",
            u32::from_le_bytes(bytes.try_into()?)
        ),
        0x0d if bytes.len() == 8 => format!(
            "float64-bits(0x{:016X})",
            u64::from_le_bytes(bytes.try_into()?)
        ),
        0x0e if bytes.len() % 2 == 0 => format!(
            "utf16[{}]",
            bytes
                .chunks_exact(2)
                .map(|b| format!("{:04X}", u16::from_le_bytes([b[0], b[1]])))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        0x12 if bytes == [0, 0, 0, 0] => "null".into(),
        _ => return Err(invalid()),
    })
}

fn field_name(token: u32, original: &str) -> String {
    crate::symbol_names::field(token, original, true)
}

fn field_flags(flags: u16) -> String {
    let mut names = vec![
        [
            "privatescope",
            "private",
            "famandassem",
            "assembly",
            "family",
            "famorassem",
            "public",
            "reserved-access",
        ][(flags & 7) as usize],
    ];
    for (bit, name) in [
        (0x10, "static"),
        (0x20, "initonly"),
        (0x40, "literal"),
        (0x80, "notserialized"),
        (0x100, "hasrva"),
        (0x200, "specialname"),
        (0x400, "rtspecialname"),
        (0x1000, "hasmarshal"),
        (0x2000, "pinvokeimpl"),
        (0x8000, "hasdefault"),
    ] {
        if flags & bit != 0 {
            names.push(name);
        }
    }
    names.join(" ")
}

fn type_flags(flags: u32) -> String {
    let visibility = [
        "notpublic",
        "public",
        "nested public",
        "nested private",
        "nested family",
        "nested assembly",
        "nested famandassem",
        "nested famorassem",
    ];
    let layout = ["auto", "sequential", "explicit", "reserved-layout"];
    let format = ["ansi", "unicode", "autochar", "customchar"];
    let mut names = vec![
        if flags & 0x20 != 0 {
            "interface"
        } else {
            "class"
        },
        visibility[(flags & 7) as usize],
        layout[((flags >> 3) & 3) as usize],
        format[((flags >> 16) & 3) as usize],
    ];
    for (bit, name) in [
        (0x80, "abstract"),
        (0x100, "sealed"),
        (0x400, "specialname"),
        (0x800, "rtspecialname"),
        (0x1000, "import"),
        (0x2000, "serializable"),
        (0x4000, "windowsruntime"),
        (0x40000, "hassecurity"),
        (0x100000, "beforefieldinit"),
        (0x200000, "typeforwarder"),
    ] {
        if flags & bit != 0 {
            names.push(name);
        }
    }
    names.join(" ")
}

pub(crate) fn identifier(s: &str) -> Result<()> {
    if s.is_empty()
        || !s
            .bytes()
            .enumerate()
            .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
    {
        return Err("non-simple identifier needs IL fallback".into());
    }
    Ok(())
}
pub(crate) fn escape_line(s: &str) -> String {
    s.chars().flat_map(|c| c.escape_default()).collect()
}
