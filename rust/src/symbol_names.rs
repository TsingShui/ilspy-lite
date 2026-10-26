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

fn bare(raw: &str) -> String {
    if simple(raw) && !reserved(raw) {
        raw.into()
    } else {
        format!(
            "__name_{}",
            raw.bytes().map(|b| format!("{b:02X}")).collect::<String>()
        )
    }
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

pub(crate) fn field(token: u32, raw: &str, escaped: bool) -> String {
    if aliased(raw) {
        format!("__il_field_{token:08X}")
    } else if escaped {
        verbatim(raw)
    } else {
        ident(raw)
    }
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
