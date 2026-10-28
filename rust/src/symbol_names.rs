// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{Result, metadata::Image};
use cildec::{Rid, TableId, Token};

/// Presentation names never participate in metadata identity or member lookup.
pub(crate) fn ident(raw: &str) -> String {
    let name = bare(raw);
    if keyword(&name) {
        format!("@{name}")
    } else {
        name
    }
}

pub(crate) fn aliased(raw: &str) -> bool {
    !simple(raw) || reserved(raw)
}

pub(crate) fn verbatim(raw: &str) -> String {
    format!("@{}", bare(raw))
}

pub(crate) fn method(image: &Image<'_>, token: Token, raw: &str, escaped: bool) -> String {
    let name = lambda_name(raw)
        .and_then(|candidate| scoped_lambda_name(image, token, &candidate).ok())
        .unwrap_or_else(|| bare(raw));
    if escaped || keyword(&name) {
        format!("@{name}")
    } else {
        name
    }
}

pub(crate) fn method_comment(raw: &str) -> Option<String> {
    lambda_name(raw).map(|_| format!("// Metadata name: {raw}"))
}

fn lambda_name(raw: &str) -> Option<String> {
    let (owner, index) = raw.strip_prefix('<')?.split_once(">b__")?;
    if index
        .split('_')
        .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let owner = match owner {
        ".ctor" => "Ctor",
        ".cctor" => "StaticCtor",
        name if simple(name) && !reserved(name) => name,
        _ => return None,
    };
    Some(format!("{owner}Lambda{index}"))
}

// Resolve only an exact in-module signature for presentation. Foreign scopes
// retain their opaque spelling; aliases never choose the actual call target.
fn local_method(image: &Image<'_>, token: Token) -> Result<Token> {
    if token.table() == Some(TableId::MethodDef) {
        return Ok(token);
    }
    if token.table() != Some(TableId::MemberRef) {
        return Err("method presentation requires a definition or member reference".into());
    }
    let tables = image.md.tables();
    let reference = tables.member_ref(token.rid())?;
    let owner = match reference.class.table() {
        Some(TableId::TypeDef) => reference.class,
        Some(TableId::TypeRef) => image
            .local_type_definition(reference.class)?
            .ok_or("foreign method presentation scope")?,
        _ => return Err("unresolved method presentation scope".into()),
    };
    let name = image.md.strings().str(reference.name)?;
    let signature = image.md.blobs().get(reference.signature)?;
    let mut found = None;
    for rid in tables.method_range(Rid::new(owner.rid()))? {
        let row = tables.method_def(rid)?;
        if image.md.strings().str(row.name)? == name
            && image.md.blobs().get(row.signature)? == signature
        {
            if found.is_some() {
                return Err("ambiguous method presentation definition".into());
            }
            found = Some(Token(0x06000000 | rid));
        }
    }
    found.ok_or_else(|| "missing method presentation definition".into())
}

fn scoped_lambda_name(image: &Image<'_>, token: Token, candidate: &str) -> Result<String> {
    use cildec::tables::{EventMapRow, NestedClassRow, PropertyMapRow};
    let token = local_method(image, token)?;
    let tables = image.md.tables();
    let owner = tables
        .type_of_method(Rid::new(token.rid()))?
        .ok_or("missing method presentation owner")?;
    let ty = tables.type_def(owner.get())?;
    let mut used = std::collections::BTreeSet::new();
    used.insert(type_ident(
        image.md.strings().str(ty.type_name)?,
        !tables
            .generic_params(Token(0x02000000 | owner.get()))?
            .is_empty(),
        false,
    ));
    for rid in tables.method_range(owner)? {
        if rid != token.rid() {
            let row = tables.method_def(rid)?;
            let raw = image.md.strings().str(row.name)?;
            used.insert(lambda_name(raw).unwrap_or_else(|| ident(raw)));
        }
    }
    for rid in tables.field_range(owner)? {
        let row = tables.field(rid)?;
        used.insert(field(
            0x04000000 | rid,
            image.md.strings().str(row.name)?,
            false,
        ));
    }
    for (_, row) in tables.iter::<NestedClassRow>() {
        let row = row?;
        if row.enclosing_class == owner {
            let nested = tables.type_def(row.nested_class.get())?;
            used.insert(type_ident(
                image.md.strings().str(nested.type_name)?,
                !tables
                    .generic_params(Token(0x02000000 | row.nested_class.get()))?
                    .is_empty(),
                false,
            ));
        }
    }
    for (rid, row) in tables.iter::<PropertyMapRow>() {
        let row = row?;
        if row.parent == owner {
            let end = if rid < tables.row_count(TableId::PropertyMap) {
                tables.property_map(rid + 1)?.property_list.get()
            } else {
                tables.row_count(TableId::Property) + 1
            };
            for property in row.property_list.get()..end {
                used.insert(ident(
                    image.md.strings().str(tables.property(property)?.name)?,
                ));
            }
        }
    }
    for (rid, row) in tables.iter::<EventMapRow>() {
        let row = row?;
        if row.parent == owner {
            let end = if rid < tables.row_count(TableId::EventMap) {
                tables.event_map(rid + 1)?.event_list.get()
            } else {
                tables.row_count(TableId::Event) + 1
            };
            for event in row.event_list.get()..end {
                used.insert(ident(image.md.strings().str(tables.event(event)?.name)?));
            }
        }
    }
    if !used.contains(candidate) {
        return Ok(candidate.into());
    }
    let mut name = format!("{candidate}__{:08X}", token.0);
    while used.contains(&name) {
        name.push('_');
    }
    Ok(name)
}

fn bare(raw: &str) -> String {
    // An explicit interface implementation is named `Namespace.Interface.Member`.
    // Every part is an identifier, so the dotted spelling is already the C#
    // source form and needs no encoding.
    if let Some(dotted) = dotted_identifiers(raw) {
        return dotted;
    }
    if simple(raw) && !reserved(raw) {
        raw.into()
    } else {
        format!(
            "__name_{}",
            raw.bytes().map(|b| format!("{b:02X}")).collect::<String>()
        )
    }
}

fn dotted_identifiers(raw: &str) -> Option<String> {
    let parts = raw.split('.').collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let mut spelled = Vec::with_capacity(parts.len());
    for part in parts {
        if part.is_empty() || !simple(part) || reserved(part) {
            return None;
        }
        spelled.push(ident(part));
    }
    Some(spelled.join("."))
}

fn simple(raw: &str) -> bool {
    !raw.is_empty()
        && raw
            .bytes()
            .enumerate()
            .all(|(i, b)| b == b'_' || b.is_ascii_alphabetic() || i > 0 && b.is_ascii_digit())
}

fn reserved(raw: &str) -> bool {
    ["__name_", "__cil_", "__il_"]
        .iter()
        .any(|prefix| raw.starts_with(prefix))
}

/// Readable spelling of a field name.
///
/// A compiler-generated backing field is `<Property>k__BackingField`; the C#
/// property already carries that name, so the field is spelled from it instead
/// of from an opaque token. Anything else unusual keeps the token alias, which
/// stays unique and traceable.
pub(crate) fn field(token: u32, raw: &str, escaped: bool) -> String {
    if let Some(property) = backing_field_property(raw) {
        return format!("__backing_{property}");
    }
    if aliased(raw) {
        format!("__il_field_{token:08X}")
    } else if escaped {
        verbatim(raw)
    } else {
        ident(raw)
    }
}

fn backing_field_property(raw: &str) -> Option<String> {
    let property = raw.strip_prefix('<')?.strip_suffix(">k__BackingField")?;
    let property = property.rsplit('.').next().unwrap_or(property);
    (!property.is_empty() && simple(property)).then(|| property.to_owned())
}

pub(crate) fn namespace(raw: &str) -> String {
    raw.split('.').map(ident).collect::<Vec<_>>().join(".")
}

fn arity_stem(raw: &str) -> Option<&str> {
    raw.rsplit_once('`')
        .filter(|(_, arity)| {
            !arity.is_empty()
                && arity.bytes().all(|b| b.is_ascii_digit())
                && arity.parse::<u32>().is_ok_and(|n| n > 0)
        })
        .map(|(stem, _)| stem)
}

pub(crate) fn type_ident(raw: &str, generic: bool, escaped: bool) -> String {
    let name = if generic {
        arity_stem(raw).unwrap_or(raw)
    } else {
        raw
    };
    if escaped { verbatim(name) } else { ident(name) }
}

/// Construct the path from actual metadata rows, not a display full-name string.
pub(crate) fn type_name(image: &Image<'_>, token: Token, escaped: bool) -> Result<String> {
    let md = &image.md;
    let mut current = if token.table() == Some(TableId::TypeRef) {
        image.local_type_definition(token)?.unwrap_or(token)
    } else {
        token
    };
    let mut seen = Vec::new();
    let mut parts = Vec::new();
    let namespace;
    loop {
        if current.rid() == 0 || seen.len() == 64 || seen.contains(&current.0) {
            return Err("invalid/cyclic/excessive type name path".into());
        }
        seen.push(current.0);
        let (name, ns, parent, generic) = match current.table() {
            Some(TableId::TypeDef) => {
                let row = md.tables().type_def(current.rid())?;
                (
                    md.strings().str(row.type_name)?,
                    md.strings().str(row.type_namespace)?,
                    md.tables()
                        .enclosing_type(Rid::new(current.rid()))?
                        .map(|rid| Token(0x02000000 | rid.get())),
                    !md.tables().generic_params(current)?.is_empty(),
                )
            }
            Some(TableId::TypeRef) => {
                let row = md.tables().type_ref(current.rid())?;
                (
                    md.strings().str(row.type_name)?,
                    md.strings().str(row.type_namespace)?,
                    (row.resolution_scope.table() == Some(TableId::TypeRef))
                        .then_some(row.resolution_scope),
                    arity_stem(md.strings().str(row.type_name)?).is_some(),
                )
            }
            _ => return Err("type name path requires TypeDef or TypeRef".into()),
        };
        if name.is_empty() {
            return Err("empty metadata type name".into());
        }
        parts.push(type_ident(name, generic, escaped));
        if let Some(parent) = parent {
            current = parent;
        } else {
            namespace = ns;
            break;
        }
    }
    parts.reverse();
    let mut path = if namespace.is_empty() {
        Vec::new()
    } else {
        namespace
            .split('.')
            .map(|part| {
                if escaped {
                    format!("@{}", bare(part))
                } else {
                    ident(part)
                }
            })
            .collect()
    };
    path.extend(parts);
    Ok(format!(
        "{}{}",
        if escaped { "global::" } else { "" },
        path.join(".")
    ))
}

pub(crate) fn keyword(name: &str) -> bool {
    matches!(
        name,
        "class"
            | "namespace"
            | "event"
            | "base"
            | "this"
            | "ref"
            | "out"
            | "in"
            | "return"
            | "new"
            | "default"
            | "switch"
            | "case"
            | "goto"
            | "if"
            | "else"
            | "while"
            | "for"
            | "do"
            | "break"
            | "continue"
            | "public"
            | "private"
            | "protected"
            | "internal"
            | "static"
            | "readonly"
            | "const"
            | "void"
            | "bool"
            | "int"
            | "uint"
            | "long"
            | "ulong"
            | "float"
            | "double"
            | "string"
            | "object"
            | "null"
            | "true"
            | "false"
            | "checked"
            | "unchecked"
            | "throw"
            | "try"
            | "catch"
            | "finally"
            | "using"
            | "struct"
            | "interface"
            | "enum"
            | "delegate"
            | "operator"
            | "params"
            | "virtual"
            | "override"
            | "sealed"
            | "abstract"
            | "extern"
            | "unsafe"
            | "fixed"
            | "sizeof"
            | "typeof"
            | "is"
            | "as"
            | "lock"
            | "get"
            | "set"
            | "byte"
            | "sbyte"
            | "char"
            | "short"
            | "ushort"
            | "decimal"
            | "foreach"
            | "volatile"
            | "explicit"
            | "implicit"
            | "stackalloc"
            | "__arglist"
            | "__makeref"
            | "__reftype"
            | "__refvalue"
    )
}
