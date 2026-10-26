// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::Result;
use cildec::{Metadata, Rid, TableId, Token};

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

/// Construct the path from actual metadata rows, not a display full-name string.
pub(crate) fn type_name(md: &Metadata<'_>, token: Token, escaped: bool) -> Result<String> {
    let mut current = token;
    let mut seen = Vec::new();
    let mut parts = Vec::new();
    let namespace;
    loop {
        if current.rid() == 0 || seen.len() == 64 || seen.contains(&current.0) {
            return Err("invalid/cyclic/excessive type name path".into());
        }
        seen.push(current.0);
        let (name, ns, parent) = match current.table() {
            Some(TableId::TypeDef) => {
                let row = md.tables().type_def(current.rid())?;
                (
                    md.strings().str(row.type_name)?,
                    md.strings().str(row.type_namespace)?,
                    md.tables()
                        .enclosing_type(Rid::new(current.rid()))?
                        .map(|rid| Token(0x02000000 | rid.get())),
                )
            }
            Some(TableId::TypeRef) => {
                let row = md.tables().type_ref(current.rid())?;
                (
                    md.strings().str(row.type_name)?,
                    md.strings().str(row.type_namespace)?,
                    (row.resolution_scope.table() == Some(TableId::TypeRef))
                        .then_some(row.resolution_scope),
                )
            }
            _ => return Err("type name path requires TypeDef or TypeRef".into()),
        };
        if name.is_empty() {
            return Err("empty metadata type name".into());
        }
        let name = name.split('`').next().unwrap_or(name);
        parts.push(if escaped {
            format!("@{}", bare(name))
        } else {
            ident(name)
        });
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
