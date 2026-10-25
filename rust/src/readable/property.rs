// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{DependencyContext, metadata::escape_line};
use std::collections::BTreeSet;

pub(super) fn rewrite(context: &DependencyContext, caller: u32, body: &str) -> String {
    let mut out = String::new();
    let mut reported = BTreeSet::new();
    for statement in body.lines() {
        let mut rendered = String::new();
        let mut changed = false;
        let mut notes = Vec::new();
        let mut position = 0;
        while position < statement.len() {
            let rest = &statement[position..];
            let first = rest.chars().next().unwrap();
            if matches!(first, '\'' | '"') {
                let end = literal_end(statement, position);
                rendered.push_str(&statement[position..end]);
                position = end;
                continue;
            }
            let prefix = ["__cil_memberref_call_", "__cil_direct_"]
                .into_iter()
                .find(|prefix| rest.starts_with(prefix));
            if let Some(prefix) = prefix {
                let token_start = position + prefix.len();
                if let Some(hex) = statement.get(token_start..token_start + 8) {
                    if let (Ok(target), Some((end, arguments))) = (
                        u32::from_str_radix(hex, 16),
                        call_arguments(statement, token_start + 8),
                    ) {
                        if arguments.first() == Some(&"this") {
                            match context.property_presentation(caller, target) {
                                Ok(Some((property, qualifier))) => {
                                    let access =
                                        format!("{qualifier}.{}", super::ident(&property.name));
                                    if property.role == "get" && arguments.len() == 1 {
                                        rendered.push_str(&access);
                                        changed = true;
                                        position = end;
                                        continue;
                                    }
                                    if property.role == "set"
                                        && arguments.len() == 2
                                        && position == 0
                                        && &statement[end..] == ";"
                                    {
                                        rendered.push_str(&format!("{access} = {};", arguments[1]));
                                        position = statement.len();
                                        continue;
                                    }
                                }
                                Ok(None) => {}
                                Err(error) if reported.insert(target) => notes.push(format!(
                                    "// Exact call 0x{target:08X} kept: {}.",
                                    escape_line(&error.to_string())
                                )),
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
            rendered.push(first);
            position += first.len_utf8();
        }
        if changed {
            if let Some(expression) = rendered
                .strip_prefix("return (")
                .and_then(|r| r.strip_suffix(");"))
            {
                if let Some((qualifier, name)) = expression.split_once('.') {
                    if matches!(qualifier, "base" | "this")
                        && crate::metadata::identifier(name.strip_prefix('@').unwrap_or(name))
                            .is_ok()
                    {
                        rendered = format!("return {expression};");
                    }
                }
            }
        }
        for note in notes {
            out.push_str(&note);
            out.push('\n');
        }
        out.push_str(&rendered);
        out.push('\n');
    }
    out
}

fn literal_end(text: &str, start: usize) -> usize {
    let quote = text.as_bytes()[start];
    let mut p = start + 1;
    while p < text.len() {
        let byte = text.as_bytes()[p];
        p += 1;
        if byte == b'\\' {
            p = (p + 1).min(text.len());
        } else if byte == quote {
            break;
        }
    }
    p
}

fn call_arguments(text: &str, start: usize) -> Option<(usize, Vec<&str>)> {
    if text.as_bytes().get(start) != Some(&b'(') {
        return None;
    }
    let mut depth = 0usize;
    let mut arguments = Vec::new();
    let mut segment = start + 1;
    let mut p = start;
    while p < text.len() {
        match text.as_bytes()[p] {
            b'\'' | b'"' => {
                p = literal_end(text, p);
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    arguments.push(text[segment..p].trim());
                    return Some((p + 1, arguments));
                }
            }
            b']' | b'}' => depth = depth.checked_sub(1)?,
            b',' if depth == 1 => {
                arguments.push(text[segment..p].trim());
                segment = p + 1;
            }
            _ => {}
        }
        p += 1;
    }
    None
}
