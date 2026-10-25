// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{MethodInfo, Result};
use cildec::tables::{MethodDefRow, NestedClassRow, TypeDefRow};
use cildec::{Metadata, MethodBody, MethodSig, Names, PeImage, Rid, Strictness, TableId, Token};

pub(crate) struct Image<'a> {
    pub pe: PeImage<'a>,
    pub md: Metadata<'a>,
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
        Ok(Self { pe, md })
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
        Ok((text, failures))
    }

    fn type_reference(&self, token: Token, depth: usize) -> Result<String> {
        if depth == 32 || token.rid() == 0 {
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
        let name = self.md.strings().str(row.name)?;
        let alias = field_name(token, name);
        let flags = field_flags(row.flags);
        let blob = self.md.blobs().get(row.signature);
        let raw_signature = match &blob {
            Ok(bytes) => format!("signature-bytes={bytes:02X?}"),
            Err(_) => format!("signature-blob-index=0x{:08X}", row.signature.get()),
        };
        let mut failures = 0;
        let signature = (|| -> Result<String> {
            let sig = cildec::FieldSig::parse(blob?)?;
            let mut references = Vec::new();
            self.signature_references(&sig.type_, 0, &mut references)?;
            for modifier in &sig.custom_mods {
                references.push(self.type_reference(modifier.type_token, 0)?);
            }
            Ok(format!(
                "{} {alias} // name={} {raw_signature} parsed={:?} references=[{}]",
                Names::new(&self.md).field_sig(&sig),
                escape_line(name),
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
                    ".field 0x{token:08X} flags=0x{:04X} {flags} <unresolved-signature> {alias} // name={} {raw_signature}\n// FAILED metadata: {}\n",
                    row.flags,
                    escape_line(name),
                    escape_line(&error.to_string())
                )
            }
        };
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
        let marshals = self
            .md
            .tables()
            .rows_for_parent(TableId::FieldMarshal, Token(token))?;
        if marshals.len() > 1 || (row.flags & 0x1000 != 0) != !marshals.is_empty() {
            failures += 1;
            text.push_str("// FAILED metadata: inconsistent field marshal association\n");
        }
        for rid in marshals {
            let marshal = self
                .md
                .tables()
                .row::<cildec::tables::FieldMarshalRow>(rid)?;
            text.push_str(&format!(
                ".field-marshal 0x{:08X} parent=0x{token:08X} ",
                0x0d000000 | rid
            ));
            match self.md.blobs().get(marshal.native_type) {
                Ok(bytes) => text.push_str(&format!("bytes={bytes:02X?}\n")),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "blob-index=0x{:08X}\n// FAILED metadata: {}\n",
                        marshal.native_type.get(),
                        escape_line(&error.to_string())
                    ));
                }
            }
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
        let name = self.md.strings().str(row.name)?;
        let blob = self.md.blobs().get(row.signature);
        let evidence = match &blob {
            Ok(bytes) => format!("signature-bytes={bytes:02X?}"),
            Err(_) => format!("signature-blob-index=0x{:08X}", row.signature.get()),
        };
        let signature: Result<MethodSig<'_>> = blob
            .map_err(Into::into)
            .and_then(|bytes| MethodSig::parse(bytes).map_err(Into::into));
        let mut failures = 0;
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
                    escape_line(name),
                    escape_line(&error.to_string())
                ));
            }
        }
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
            let import = tables.row::<cildec::tables::ImplMapRow>(rid)?;
            text.push_str(&format!(
                ".pinvoke 0x{:08X} method=0x{token:08X} flags=0x{:04X} scope=0x{:08X} ",
                0x1c000000 | rid,
                import.mapping_flags,
                0x1a000000 | import.import_scope.get()
            ));
            let target = (|| -> Result<String> {
                let module = tables.module_ref(import.import_scope.get())?;
                Ok(format!(
                    "name={} module={}\n",
                    escape_line(self.md.strings().str(import.import_name)?),
                    escape_line(self.md.strings().str(module.name)?)
                ))
            })();
            match target {
                Ok(target) => text.push_str(&target),
                Err(error) => {
                    failures += 1;
                    text.push_str(&format!(
                        "name-index=0x{:08X}\n// FAILED metadata: {}\n",
                        import.import_name.get(),
                        escape_line(&error.to_string())
                    ));
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
        Ok(MethodSig::parse(self.md.blobs().get(row.signature)?)?)
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
                cildec::Operand::String(t) => format!(
                    "0x{:08X} {:?}",
                    t.0,
                    self.md
                        .user_strings()
                        .get(*t)?
                        .code_units()
                        .collect::<Vec<_>>()
                ),
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

    pub fn local_types(&self, body: &MethodBody<'a>) -> Result<Vec<crate::ir::Ty>> {
        let Some(t) = body.local_var_sig else {
            return Ok(Vec::new());
        };
        if t.table() != Some(TableId::StandAloneSig) {
            return Err("invalid local signature token".into());
        }
        let row = self.md.tables().stand_alone_sig(t.rid())?;
        let sig = cildec::LocalVarSig::parse(self.md.blobs().get(row.signature)?)?;
        sig.locals
            .iter()
            .map(|l| {
                if l.pinned || l.by_ref || !l.custom_mods.is_empty() {
                    return Err("pinned/byref/modified local needs IL fallback".into());
                }
                crate::ir::Ty::from_sig(&l.type_)
            })
            .collect()
    }

    /// Resolve local class identity; external ancestry is not an assignability proof.
    pub fn class_name(&self, token: u32) -> Result<String> {
        self.class_chain(token)?;
        cs_owner(&Names::new(&self.md).type_def(Rid::new(token & 0xffffff))?)
    }

    fn class_chain(&self, token: u32) -> Result<Vec<u32>> {
        let mut chain = Vec::new();
        let mut current = Token(token);
        for _ in 0..64 {
            if current.table() != Some(TableId::TypeDef) || chain.contains(&current.0) {
                return Err("unresolved/cyclic reference hierarchy needs IL fallback".into());
            }
            let row = self.md.tables().type_def(current.rid())?;
            let name = Names::new(&self.md).type_def(Rid::new(current.rid()))?;
            if row.flags & 0x20 != 0 || name.contains('`') {
                return Err("interface/generic declaring type needs IL fallback".into());
            }
            chain.push(current.0);
            current = row.extends;
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

    pub fn receiver(&self, method: u32) -> Result<crate::ir::Ty> {
        let owner = self
            .md
            .tables()
            .type_of_method(Rid::new(method & 0xffffff))?
            .ok_or("missing declaring type")?;
        let token = 0x02000000 | owner.get();
        self.class_name(token)?;
        Ok(crate::ir::Ty::Class(token))
    }

    pub fn assignable(&self, actual: crate::ir::Ty, expected: crate::ir::Ty) -> Result<()> {
        use crate::ir::Ty;
        if actual == expected
            || (actual == Ty::Null && matches!(expected, Ty::Class(_) | Ty::I4Array | Ty::U4Array))
        {
            return Ok(());
        }
        if let (Ty::Class(a), Ty::Class(b)) = (actual, expected) {
            if self.class_chain(a)?.contains(&b) {
                return Ok(());
            }
        }
        Err("incompatible value/receiver types".into())
    }

    pub fn field(&self, t: Token) -> Result<FieldRef> {
        if t.table() != Some(TableId::Field) {
            return Err("nonlocal/constructed field needs IL fallback".into());
        }
        let row = self.md.tables().field(t.rid())?;
        let owner = self
            .md
            .tables()
            .type_of_field(Rid::new(t.rid()))?
            .ok_or("missing field owner")?;
        let owner = crate::ir::Ty::Class(0x02000000 | owner.get());
        let sig = cildec::FieldSig::parse(self.md.blobs().get(row.signature)?)?;
        if !sig.custom_mods.is_empty() {
            return Err("modified field needs IL fallback".into());
        }
        let ty = crate::ir::Ty::from_sig(&sig.type_)?;
        ty.cs(self)?;
        let original = self.md.strings().str(row.name)?;
        let name = field_name(t.0, original);
        Ok(FieldRef {
            token: t.0,
            owner,
            ty,
            name,
            is_static: row.flags & 0x10 != 0,
            readonly: row.flags & 0x20 != 0,
        })
    }

    pub fn method_ref(&self, t: Token) -> Result<MethodRef> {
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
                    Names::new(&self.md).type_def(owner)?,
                    self.signature(t.0)?,
                )
            }
            Some(TableId::MemberRef) => {
                let row = self.md.tables().member_ref(t.rid())?;
                (
                    self.md.strings().str(row.name)?.to_string(),
                    Names::new(&self.md).try_token(row.class)?,
                    MethodSig::parse(self.md.blobs().get(row.signature)?)?,
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
        identifier(&name)?;
        let receiver = if sig.has_this {
            if t.table() != Some(TableId::MethodDef) {
                return Err("external instance call needs IL fallback".into());
            }
            let row = self.row(t.0)?;
            if row.is_static() {
                return Err("inconsistent call signature".into());
            }
            Some(self.receiver(t.0)?)
        } else {
            None
        };
        let args = sig
            .params
            .iter()
            .map(crate::ir::Ty::from_param)
            .collect::<Result<Vec<_>>>()?;
        let ret = crate::ir::Ty::from_param(&sig.return_type)?;
        Ok(MethodRef {
            token: t.0,
            receiver,
            target: if receiver.is_some() {
                format!("__cil_direct_{:08X}", t.0)
            } else {
                format!("{}.@{name}", cs_owner(&owner)?)
            },
            args,
            ret,
        })
    }

    pub fn static_field(&self, t: Token) -> Result<(String, crate::ir::Ty)> {
        if t.table() != Some(TableId::Field) {
            return Err("external field needs IL fallback".into());
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
        let name = self.md.strings().str(row.name)?;
        identifier(name)?;
        let sig = cildec::FieldSig::parse(self.md.blobs().get(row.signature)?)?;
        if !sig.custom_mods.is_empty() {
            return Err("modified field needs IL fallback".into());
        }
        Ok((
            format!(
                "{}.@{name}",
                cs_owner(&Names::new(&self.md).type_def(owner)?)?
            ),
            crate::ir::Ty::from_sig(&sig.type_)?,
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
    pub receiver: Option<crate::ir::Ty>,
    pub target: String,
    pub args: Vec<crate::ir::Ty>,
    pub ret: crate::ir::Ty,
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
    if identifier(original).is_ok() && !original.starts_with("__il_field_") {
        format!("@{original}")
    } else {
        format!("__il_field_{token:08X}")
    }
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
fn cs_owner(name: &str) -> Result<String> {
    // Assembly identity remains in the adjacent IL-token comment and raw IL.
    let name = if name.starts_with('[') {
        name.split_once(']')
            .ok_or("invalid assembly-qualified name")?
            .1
    } else {
        name
    };
    let name = name.replace('/', ".");
    for part in name.split('.') {
        identifier(part)?;
    }
    Ok(format!(
        "global::{}",
        name.split('.')
            .map(|p| format!("@{p}"))
            .collect::<Vec<_>>()
            .join(".")
    ))
}
pub(crate) fn escape_line(s: &str) -> String {
    s.chars().flat_map(|c| c.escape_default()).collect()
}
