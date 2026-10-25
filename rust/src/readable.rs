// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{
    DecompiledMethod, Result, Status,
    metadata::{Image, SelectedType},
};
use cildec::{Param, Rid, TableId, Token, Type};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn type_view(
    image: &Image<'_>,
    types: &[SelectedType],
    bodies: &BTreeMap<u32, DecompiledMethod>,
) -> String {
    let mut out = String::new();
    let root = &types[0];
    let namespace = image
        .md
        .tables()
        .type_def(root.token & 0xffffff)
        .ok()
        .and_then(|row| image.md.strings().str(row.type_namespace).ok())
        .unwrap_or("");
    if !namespace.is_empty() {
        out.push_str(&format!("namespace {namespace};\n\n"));
    }
    let mut emitted = BTreeSet::new();
    render_type(image, root, types, bodies, 0, &mut emitted, &mut out);
    out
}

fn render_type(
    image: &Image<'_>,
    ty: &SelectedType,
    types: &[SelectedType],
    bodies: &BTreeMap<u32, DecompiledMethod>,
    indent: usize,
    emitted: &mut BTreeSet<u32>,
    out: &mut String,
) {
    if !emitted.insert(ty.token) {
        return;
    }
    let header = type_header(image, ty).unwrap_or_else(|error| {
        format!(
            "class __type_{:08X} /* {} */",
            ty.token,
            crate::metadata::escape_line(&error.to_string())
        )
    });
    line(out, indent, &header);
    line(out, indent, "{");
    for &token in &ty.fields {
        match field_header(image, token, ty.token) {
            Ok(header) => line(out, indent + 1, &header),
            Err(_) => line(
                out,
                indent + 1,
                &format!("// Unreadable field 0x{token:08X}; see --details."),
            ),
        }
    }
    if !ty.fields.is_empty() && !ty.methods.is_empty() {
        out.push('\n');
    }
    for (index, &token) in ty.methods.iter().enumerate() {
        let result = &bodies[&token];
        render_method(image, token, ty.token, result, indent + 1, out);
        if index + 1 != ty.methods.len() {
            out.push('\n');
        }
    }
    // Accessors remain methods; no automatic-property or event semantics are invented.
    for (&token, kind) in ty
        .properties
        .iter()
        .map(|t| (t, "property"))
        .chain(ty.events.iter().map(|t| (t, "event")))
    {
        let relationship = accessor_summary(image, token, ty.token)
            .unwrap_or_else(|_| format!("// Unreadable {kind} relationship; see --details."));
        line(out, indent + 1, &relationship);
    }
    for nested in types {
        if image
            .md
            .tables()
            .enclosing_type(Rid::new(nested.token & 0xffffff))
            .ok()
            .flatten()
            .is_some_and(|p| p.get() == ty.token & 0xffffff)
        {
            out.push('\n');
            render_type(image, nested, types, bodies, indent + 1, emitted, out);
        }
    }
    line(out, indent, "}");
}

fn accessor_summary(image: &Image<'_>, token: u32, owner: u32) -> Result<String> {
    let tables = image.md.tables();
    let event = token >> 24 == 0x14;
    let (kind, signature) = if event {
        let row = tables.event(token & 0xffffff)?;
        (
            "event",
            format!(
                "{} {}",
                named_type(image, row.event_type)?,
                ident(image.md.strings().str(row.name)?)
            ),
        )
    } else {
        let row = tables.property(token & 0xffffff)?;
        let sig = cildec::PropertySig::parse(image.md.blobs().get(row.type_signature)?)?;
        let name = if sig.params.is_empty() {
            ident(image.md.strings().str(row.name)?)
        } else {
            format!(
                "this[{}]",
                sig.params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| Ok(format!("{} index{i}", cs_param(image, p, owner, 0)?)))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ")
            )
        };
        (
            "property",
            format!("{} {name}", cs_param(image, &sig.return_type, owner, 0)?),
        )
    };
    let mut methods = Vec::new();
    for rid in tables.semantics_for(Token(token))? {
        let row = tables.method_semantics(rid)?;
        let kind = match (event, row.semantics) {
            (false, 1) => "set",
            (false, 2) => "get",
            (true, 8) => "add",
            (true, 16) => "remove",
            (true, 32) => "raise",
            (_, 4) => "other",
            _ => "unknown",
        };
        let method = image.row(0x06000000 | row.method.get())?;
        methods.push(format!(
            "{kind}: {};",
            ident(image.md.strings().str(method.name)?)
        ));
    }
    Ok(format!("// {kind} {signature} {{ {} }}", methods.join(" ")))
}

fn type_header(image: &Image<'_>, ty: &SelectedType) -> Result<String> {
    let row = image.md.tables().type_def(ty.token & 0xffffff)?;
    let name = image.md.strings().str(row.type_name)?;
    let access = match row.flags & 7 {
        1 | 2 => "public",
        3 => "private",
        4 => "protected",
        7 => "protected internal",
        6 => "private protected",
        _ => "internal",
    };
    let kind = if row.flags & 0x20 != 0 {
        "interface"
    } else if cildec::Names::new(&image.md)
        .try_token(row.extends)
        .is_ok_and(|n| n.ends_with("System.ValueType"))
    {
        "struct"
    } else {
        "class"
    };
    let modifiers = if kind != "class" {
        ""
    } else if row.flags & 0x180 == 0x180 {
        "static "
    } else if row.flags & 0x80 != 0 {
        "abstract "
    } else if row.flags & 0x100 != 0 {
        "sealed "
    } else {
        ""
    };
    let generics = generic_names(image, ty.token)?;
    let suffix = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };
    let mut bases = Vec::new();
    if !row.extends.is_null() {
        let base = named_type(image, row.extends)?;
        if !matches!(base.as_str(), "System.Object" | "System.ValueType") {
            bases.push(base);
        }
    }
    for rid in image
        .md
        .tables()
        .interface_impls(Rid::new(ty.token & 0xffffff))?
    {
        bases.push(named_type(
            image,
            image.md.tables().interface_impl(rid)?.interface,
        )?);
    }
    Ok(format!(
        "{access} {modifiers}{kind} {}{suffix}{}{}",
        ident(name.split('`').next().unwrap_or(name)),
        if bases.is_empty() {
            String::new()
        } else {
            format!(" : {}", bases.join(", "))
        },
        constraints(image, ty.token, ty.token, 0)?
    ))
}

fn generic_names(image: &Image<'_>, owner: u32) -> Result<Vec<String>> {
    image
        .md
        .tables()
        .generic_params(Token(owner))?
        .into_iter()
        .map(|rid| {
            Ok(ident(
                image
                    .md
                    .strings()
                    .str(image.md.tables().generic_param(rid)?.name)?,
            ))
        })
        .collect()
}

fn constraints(image: &Image<'_>, owner: u32, type_owner: u32, method: u32) -> Result<String> {
    let mut text = String::new();
    for rid in image.md.tables().generic_params(Token(owner))? {
        let row = image.md.tables().generic_param(rid)?;
        let mut values = Vec::new();
        if row.flags & 8 != 0 {
            values.push("struct".into());
        } else if row.flags & 4 != 0 {
            values.push("class".into());
        }
        for c in image.md.tables().generic_param_constraints(Rid::new(rid))? {
            let token = image.md.tables().generic_param_constraint(c)?.constraint;
            let value = named_type_with_owners(image, token, type_owner, method)?;
            if value != "System.ValueType" {
                values.push(value);
            }
        }
        if row.flags & 16 != 0 && row.flags & 8 == 0 {
            values.push("new()".into());
        }
        if !values.is_empty() {
            text.push_str(&format!(
                " where {} : {}",
                ident(image.md.strings().str(row.name)?),
                values.join(", ")
            ));
        }
    }
    Ok(text)
}

fn field_header(image: &Image<'_>, token: u32, owner: u32) -> Result<String> {
    let row = image.md.tables().field(token & 0xffffff)?;
    let sig = cildec::FieldSig::parse(image.md.blobs().get(row.signature)?)?;
    let original = image.md.strings().str(row.name)?;
    let name =
        if crate::metadata::identifier(original).is_err() || original.starts_with("__il_field_") {
            format!("__il_field_{token:08X}")
        } else {
            ident(original)
        };
    let modifier = if row.flags & 0x40 != 0 {
        "const "
    } else if row.flags & 0x20 != 0 {
        if row.flags & 0x10 != 0 {
            "static readonly "
        } else {
            "readonly "
        }
    } else if row.flags & 0x10 != 0 {
        "static "
    } else {
        ""
    };
    let value = if row.flags & 0x40 != 0 {
        default_value(image, token)?
            .map(|s| format!(" = {s}"))
            .unwrap_or_else(|| " /* unresolved constant; see --details */".into())
    } else {
        String::new()
    };
    Ok(format!(
        "{} {modifier}{} {name}{value};",
        access(row.flags),
        cs_type(image, &sig.type_, owner, 0)?
    ))
}

fn access(flags: u16) -> &'static str {
    match flags & 7 {
        1 => "private",
        2 => "private protected",
        3 => "internal",
        4 => "protected",
        5 => "protected internal",
        6 => "public",
        _ => "private",
    }
}

fn method_header(
    image: &Image<'_>,
    token: u32,
    owner: u32,
) -> Result<(String, BTreeMap<String, String>)> {
    let row = image.row(token)?;
    let name = image.md.strings().str(row.name)?;
    let sig = image.signature(token)?;
    let generic = generic_names(image, token)?;
    let mut names = BTreeMap::new();
    if sig.has_this {
        names.insert("a0".into(), "this".into());
    }
    let parameter_rows = image.md.tables().param_range(Rid::new(token & 0xffffff))?;
    let mut used = BTreeSet::new();
    let mut params = Vec::new();
    for (i, param) in sig.params.iter().enumerate() {
        let original = parameter_rows.clone().find_map(|rid| {
            let row = image.md.tables().param(rid).ok()?;
            (usize::from(row.sequence) == i + 1).then_some((rid, row))
        });
        let proposed = original
            .and_then(|(_, row)| image.md.strings().str(row.name).ok())
            .filter(|s| !s.is_empty())
            .map(ident)
            .unwrap_or_else(|| format!("arg{i}"));
        let mut name = if proposed.starts_with(['v', 'l', 's', 'e'])
            && proposed[1..].starts_with(|c: char| c.is_ascii_digit())
        {
            format!("arg{i}")
        } else {
            proposed
        };
        let base = name.clone();
        let mut suffix = 1;
        while !used.insert(name.clone()) {
            name = format!("{base}_{suffix}");
            suffix += 1;
        }
        let index = i + usize::from(sig.has_this);
        names.insert(format!("a{index}"), name.clone());
        let default = original
            .and_then(|(rid, _)| default_value(image, 0x08000000 | rid).ok().flatten())
            .map(|v| format!(" = {v}"))
            .unwrap_or_default();
        let parameter_type = if param.by_ref && original.is_some_and(|(_, row)| row.flags & 3 == 2)
        {
            format!("out {}", cs_type(image, &param.type_, owner, token)?)
        } else {
            cs_param(image, param, owner, token)?
        };
        params.push(format!("{parameter_type} {name}{default}"));
    }
    let modifiers = if row.flags & 0x10 != 0 {
        "static "
    } else if row.flags & 0x400 != 0 {
        "abstract "
    } else if row.flags & 0x40 != 0 {
        if row.flags & 0x100 == 0 {
            "override "
        } else {
            "virtual "
        }
    } else {
        ""
    };
    let modifiers = format!(
        "{modifiers}{}",
        if row.flags & 0x2000 != 0 {
            "extern "
        } else {
            ""
        }
    );
    let header = if name == ".ctor" || name == ".cctor" {
        let ty = image.md.tables().type_def(owner & 0xffffff)?;
        format!(
            "{} {}{}({})",
            if name == ".cctor" {
                ""
            } else {
                access(row.flags)
            },
            if name == ".cctor" { "static " } else { "" },
            ident(
                image
                    .md
                    .strings()
                    .str(ty.type_name)?
                    .split('`')
                    .next()
                    .unwrap()
            ),
            params.join(", ")
        )
        .trim()
        .to_string()
    } else {
        format!(
            "{} {modifiers}{} {}{}({}){}",
            access(row.flags),
            cs_param(image, &sig.return_type, owner, token)?,
            ident(name),
            if generic.is_empty() {
                String::new()
            } else {
                format!("<{}>", generic.join(", "))
            },
            params.join(", "),
            constraints(image, token, owner, token)?
        )
    };
    Ok((header, names))
}

fn render_method(
    image: &Image<'_>,
    token: u32,
    owner: u32,
    result: &DecompiledMethod,
    indent: usize,
    out: &mut String,
) {
    let (header, names) = method_header(image, token, owner)
        .unwrap_or_else(|_| (format!("void __method_{token:08X}()"), BTreeMap::new()));
    if result.status == Status::Declaration {
        line(out, indent, &format!("{header};"));
        return;
    }
    line(out, indent, &header);
    line(out, indent, "{");
    if result.status != Status::Decompiled {
        let reason = result
            .text
            .lines()
            .find_map(|l| {
                l.strip_prefix("// IL FALLBACK:")
                    .or_else(|| l.strip_prefix("// FAILED:"))
            })
            .unwrap_or("unreadable method body");
        line(
            out,
            indent + 1,
            &format!(
                "// {}: {}; see --details for complete evidence.",
                if result.status == Status::Partial {
                    "IL fallback"
                } else {
                    "Failed"
                },
                reason.trim()
            ),
        );
    } else {
        let body = method_body(&result.text, &names);
        let mut nesting = 0usize;
        for statement in body.lines() {
            let leading = statement.chars().take_while(|c| *c == '}').count();
            nesting = nesting.saturating_sub(leading);
            line(out, indent + 1 + nesting, statement);
            let braces = lexical_braces(statement);
            let remaining_closes = braces.1.saturating_sub(leading);
            nesting = nesting
                .saturating_add(braces.0)
                .saturating_sub(remaining_closes);
        }
    }
    line(out, indent, "}");
}

pub(crate) fn method_view(image: &Image<'_>, token: u32, result: &DecompiledMethod) -> String {
    let owner = image
        .md
        .tables()
        .type_of_method(Rid::new(token & 0xffffff))
        .ok()
        .flatten()
        .map_or(0, |r| 0x02000000 | r.get());
    let mut out = String::new();
    render_method(image, token, owner, result, 0, &mut out);
    out
}

fn lexical_braces(text: &str) -> (usize, usize) {
    let mut braces = (0, 0);
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\"' || c == '\'' {
            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    chars.next();
                } else if ch == c {
                    break;
                }
            }
        } else if c == '{' {
            braces.0 += 1;
        } else if c == '}' {
            braces.1 += 1;
        }
    }
    braces
}

fn method_body(text: &str, names: &BTreeMap<String, String>) -> String {
    let Some((_, body)) = text.split_once("\n{\n") else {
        return "// Unreadable rendered body; see --details.".into();
    };
    let body = body.strip_suffix("}\n").unwrap_or(body);
    let lines = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .map(str::to_owned)
        .collect();
    let mut lines = compact_lines(lines);
    // Edge structuring can expose further adjacent immutable copy chains.
    lines = compact_lines(lines);
    lines
        .into_iter()
        .map(|line| rewrite(&line, names) + "\n")
        .collect()
}

fn generated_value(name: &str) -> bool {
    name.strip_prefix('v')
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

fn identifiers(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if c == '\"' || c == '\'' {
            while let Some((_, ch)) = chars.next() {
                if ch == '\\' {
                    chars.next();
                } else if ch == c {
                    break;
                }
            }
        } else if c == '@' || c == '_' || c.is_ascii_alphabetic() {
            let mut end = start + c.len_utf8();
            while let Some(&(i, ch)) = chars.peek() {
                if ch == '_' || ch.is_ascii_alphanumeric() {
                    chars.next();
                    end = i + ch.len_utf8();
                } else {
                    break;
                }
            }
            result.push(text[start..end].to_owned());
        }
    }
    result
}

fn value_declaration(line: &str) -> Option<(&str, &str)> {
    let (left, right) = line.split_once(" = ")?;
    let name = left.split_whitespace().last()?;
    if !generated_value(name) || left.split_whitespace().count() < 2 {
        return None;
    }
    Some((name, right.strip_suffix(';')?))
}

// Only generated immutable snapshots can be substituted. Calls/field reads,
// allocation, managed addresses and checked or runtime operations stay at their
// original statement, even when their result is unused.
fn control_barrier(line: &str) -> bool {
    line.ends_with(": ;")
        || line.starts_with("goto ")
        || line.starts_with("if ")
        || line.starts_with("switch ")
        || line.starts_with("case ")
        || line.starts_with("default:")
        || line.starts_with('}')
        || line == "{"
}

fn compact_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let mut changed = false;
        for i in 0..lines.len() {
            let Some((name, expression)) = value_declaration(&lines[i]) else {
                continue;
            };
            let name = name.to_owned();
            let expression = expression.to_owned();
            let uses = lines
                .iter()
                .enumerate()
                .filter(|(n, _)| *n != i)
                .map(|(_, l)| identifiers(l).iter().filter(|v| *v == &name).count())
                .sum::<usize>();
            let literal = expression == "null"
                || expression.parse::<i32>().is_ok()
                || expression
                    .strip_suffix('L')
                    .is_some_and(|n| n.parse::<i64>().is_ok());
            let last_use = lines
                .iter()
                .enumerate()
                .filter(|(n, l)| *n != i && identifiers(l).contains(&name))
                .map(|(n, _)| n)
                .max()
                .unwrap_or(i);
            let forward_region =
                last_use >= i && lines[i + 1..=last_use].iter().all(|l| !control_barrier(l));
            let stable = generated_value(&expression) && forward_region
                || expression.strip_prefix('a').is_some_and(|n| {
                    !n.is_empty()
                        && n.bytes().all(|b| b.is_ascii_digit())
                        && !lines
                            .iter()
                            .any(|l| l.starts_with(&format!("{expression} = ")))
                });
            // Adjacent pure arithmetic can move into its one immediate use. No
            // mutable local/argument or effectful nested operation is admitted.
            let words = identifiers(&expression);
            let arithmetic = expression.starts_with("unchecked(")
                && words.iter().all(|w| {
                    w == "unchecked"
                        || w == "int"
                        || w == "long"
                        || w == "uint"
                        || w == "ulong"
                        || generated_value(w)
                        || w.strip_prefix('a').is_some_and(|n| {
                            !n.is_empty()
                                && n.bytes().all(|b| b.is_ascii_digit())
                                && !lines.iter().any(|l| l.starts_with(&format!("{w} = ")))
                        })
                })
                && !expression.contains(['/', '%', '[', '.', '?']);
            let adjacent = uses == 1
                && lines.get(i + 1).is_some_and(|l| {
                    identifiers(l).contains(&name)
                        && (l.starts_with("return ")
                            || l.split_once(" = ").is_some_and(|(target, value)| {
                                value == format!("{name};")
                                    && !target.contains(['[', '(', ')'])
                                    && identifiers(target).iter().all(|w| {
                                        w.starts_with('@')
                                            || target.trim().ends_with(w)
                                            || w == "this"
                                            || generated_value(w)
                                            || w.starts_with('a')
                                    })
                            }))
                });
            if !(literal || stable || arithmetic && adjacent) {
                continue;
            }
            if uses == 0 && !literal && !stable {
                continue;
            }
            let replacement = expression;
            let map = BTreeMap::from([(name, replacement)]);
            for (n, line) in lines.iter_mut().enumerate() {
                if n != i {
                    *line = rewrite(line, &map);
                }
            }
            lines.remove(i);
            changed = true;
            break;
        }
        if !changed {
            break;
        }
    }
    // Boolean 0/1 encodings may collapse at their one adjacent Boolean use.
    // Do not turn arbitrary integer truth tests into Boolean expressions.
    let mut i = 0;
    while i + 1 < lines.len() {
        if let Some((name, expr)) = value_declaration(&lines[i]) {
            if let Some(condition) = expr
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(" ? 1 : 0)"))
            {
                let uses = lines
                    .iter()
                    .enumerate()
                    .filter(|(n, _)| *n != i)
                    .map(|(_, l)| identifiers(l).iter().filter(|id| *id == name).count())
                    .sum::<usize>();
                let next = &lines[i + 1];
                let replacement = if next == &format!("return ({name} != 0);") {
                    Some(format!("return ({condition});"))
                } else if next == &format!("if ({name} == 0) {{") {
                    Some(format!("if (!({condition})) {{"))
                } else if next == &format!("if ({name} != 0) {{") {
                    Some(format!("if ({condition}) {{"))
                } else {
                    None
                };
                if uses == 1
                    && let Some(replacement) = replacement
                {
                    lines[i + 1] = replacement;
                    lines.remove(i);
                    continue;
                }
            }
        }
        if lines[i] == "return (1 != 0);" {
            lines[i] = "return true;".into();
        }
        if lines[i] == "return (0 != 0);" {
            lines[i] = "return false;".into();
        }
        i += 1;
    }
    for line in &mut lines {
        if line == "return (1 != 0);" {
            *line = "return true;".into();
        }
        if line == "return (0 != 0);" {
            *line = "return false;".into();
        }
    }
    // Eliminate a local written once only when its stored source is already an
    // immutable snapshot. This preserves the read point across field mutations.
    let mut i = 0;
    while i < lines.len() {
        let Some((local, value)) = lines[i].split_once(" = ") else {
            i += 1;
            continue;
        };
        if local
            .strip_prefix('l')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        {
            let value = value.strip_suffix(';').unwrap_or("");
            let last_use = lines
                .iter()
                .enumerate()
                .filter(|(n, l)| {
                    *n != i
                        && identifiers(l).contains(&local.to_owned())
                        && !l.ends_with(" = default;")
                })
                .map(|(n, _)| n)
                .max()
                .unwrap_or(i);
            let no_earlier_use = lines[..i]
                .iter()
                .all(|l| !identifiers(l).contains(&local.to_owned()) || l.ends_with(" = default;"));
            let region =
                last_use >= i && lines[i + 1..=last_use].iter().all(|l| !control_barrier(l));
            if generated_value(value)
                && no_earlier_use
                && region
                && lines
                    .iter()
                    .filter(|l| l.starts_with(&format!("{local} = ")))
                    .count()
                    == 1
            {
                let map = BTreeMap::from([(local.to_owned(), value.to_owned())]);
                let declaration = lines
                    .iter()
                    .find(|l| l.ends_with(&format!(" {local} = default;")))
                    .cloned()
                    .unwrap_or_default();
                let local = local.to_owned();
                let mut retained = Vec::new();
                for (n, line) in lines.iter().enumerate() {
                    if n == i || line == &declaration {
                        continue;
                    }
                    retained.push(rewrite(line, &map));
                }
                if !retained.iter().any(|l| identifiers(l).contains(&local)) {
                    lines = retained;
                    i = 0;
                    continue;
                }
            }
        }
        i += 1;
    }
    // Replace an edge to a return-only block with that return. No stack copies,
    // field/call effects or block-local values may be pulled through the edge.
    loop {
        let mut replacements = BTreeMap::new();
        for (index, label) in lines.iter().enumerate() {
            let Some(label) = label.strip_suffix(": ;") else {
                continue;
            };
            let end = lines[index + 1..]
                .iter()
                .position(|l| l.ends_with(": ;"))
                .map_or(lines.len(), |n| index + 1 + n);
            if end == index + 2 && lines[index + 1].starts_with("return ") {
                let expression = &lines[index + 1];
                if identifiers(expression).iter().all(|w| {
                    w == "return"
                        || w == "true"
                        || w == "false"
                        || w == "null"
                        || w.strip_prefix('a')
                            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                }) {
                    replacements.insert(format!("goto {label};"), expression.clone());
                }
            }
        }
        let mut changed = false;
        for line in &mut lines {
            if let Some(replacement) = replacements.get(line) {
                *line = replacement.clone();
                changed = true;
            }
        }
        if changed {
            let targets = lines
                .iter()
                .filter_map(|l| l.strip_prefix("goto ").and_then(|s| s.strip_suffix(';')))
                .collect::<BTreeSet<_>>();
            let mut remove = Vec::new();
            for (index, line) in lines.iter().enumerate() {
                if let Some(label) = line.strip_suffix(": ;") {
                    let previous = index.checked_sub(1).and_then(|n| lines.get(n));
                    let no_fallthrough = previous.is_some_and(|l| {
                        l.starts_with("return ")
                            || l.starts_with("goto ")
                            || l.starts_with("throw ")
                            || l == "} }"
                            || l == "}"
                    });
                    if !targets.contains(label)
                        && no_fallthrough
                        && lines
                            .get(index + 1)
                            .is_some_and(|l| l.starts_with("return "))
                    {
                        remove.push(index);
                        remove.push(index + 1);
                    }
                }
            }
            for index in remove.into_iter().rev() {
                lines.remove(index);
            }
        }
        if !changed {
            break;
        }
    }
    // A two-way edge with one return arm and one immediate continuation is an
    // early-return if. This only removes edge syntax, never moves instructions.
    let mut i = 0;
    while i + 6 < lines.len() {
        let condition = lines[i]
            .strip_prefix("if (")
            .and_then(|s| s.strip_suffix(") {"));
        if let Some(condition) = condition {
            let target = lines[i + 1]
                .strip_prefix("goto ")
                .and_then(|s| s.strip_suffix(';'));
            if let Some(target) = target
                && lines[i + 2] == "} else {"
                && lines[i + 3].starts_with("return ")
                && lines[i + 4] == "}"
                && lines[i + 5] == format!("{target}: ;")
                && lines
                    .iter()
                    .filter(|l| *l == &format!("goto {target};"))
                    .count()
                    == 1
            {
                let condition = condition
                    .strip_prefix("!(")
                    .and_then(|s| s.strip_suffix(')'))
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("!({condition})"));
                let returned = lines[i + 3].clone();
                lines.splice(
                    i..i + 6,
                    [format!("if ({condition}) {{"), returned, "}".into()],
                );
                continue;
            }
        }
        i += 1;
    }
    // A goto immediately followed by its target is just fall-through. Keep all
    // other labels and edges; no control-flow or loop reconstruction is assumed.
    let mut i = 0;
    while i + 1 < lines.len() {
        if lines[i]
            .strip_prefix("goto ")
            .and_then(|s| s.strip_suffix(';'))
            .is_some_and(|target| lines[i + 1] == format!("{target}: ;"))
        {
            lines.remove(i);
        } else {
            i += 1;
        }
    }
    let targets = lines
        .iter()
        .filter_map(|l| {
            l.trim()
                .strip_prefix("goto ")
                .and_then(|s| s.strip_suffix(';'))
        })
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    lines.retain(|line| !line.ends_with(": ;") || targets.contains(line.trim_end_matches(": ;")));
    lines
}

// Rewrite generated C# identifiers without touching string/character literals.
// Fully qualified names remain qualified; this is not source-name binding.
fn rewrite(text: &str, names: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if c == '"' || c == '\'' {
            out.push(c);
            while let Some((_, ch)) = chars.next() {
                out.push(ch);
                if ch == '\\' {
                    if let Some((_, escaped)) = chars.next() {
                        out.push(escaped);
                    }
                } else if ch == c {
                    break;
                }
            }
        } else if c == '@' || c == '_' || c.is_ascii_alphabetic() {
            let mut end = start + c.len_utf8();
            while let Some(&(i, ch)) = chars.peek() {
                if ch == '_' || ch.is_ascii_alphanumeric() {
                    chars.next();
                    end = i + ch.len_utf8();
                } else {
                    break;
                }
            }
            let word = &text[start..end];
            let member_slot =
                text[..start].trim_end().ends_with('.') || text[..start].trim_end().ends_with("::");
            if member_slot {
                out.push_str(
                    word.strip_prefix('@')
                        .filter(|s| !keyword(s))
                        .unwrap_or(word),
                );
                continue;
            }
            if word == "global" && text[end..].starts_with("::") {
                chars.next();
                chars.next();
                continue;
            }
            out.push_str(names.get(word).map(String::as_str).unwrap_or_else(|| {
                if let Some(word) = word.strip_prefix('@') {
                    if keyword(word) {
                        &text[start..end]
                    } else {
                        word
                    }
                } else {
                    word
                }
            }));
        } else {
            out.push(c);
        }
    }
    out
}

fn ident(name: &str) -> String {
    if crate::metadata::identifier(name).is_err() {
        return format!(
            "__name_{}",
            name.bytes().map(|b| format!("{b:02X}")).collect::<String>()
        );
    }
    if keyword(name) {
        format!("@{name}")
    } else {
        name.into()
    }
}
fn keyword(name: &str) -> bool {
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
    )
}
fn line(out: &mut String, indent: usize, value: &str) {
    out.push_str(&"    ".repeat(indent));
    out.push_str(value);
    out.push('\n');
}

fn cs_param(image: &Image<'_>, param: &Param<'_>, owner: u32, method: u32) -> Result<String> {
    Ok(format!(
        "{}{}",
        if param.by_ref { "ref " } else { "" },
        cs_type(image, &param.type_, owner, method)?
    ))
}
fn named_type(image: &Image<'_>, token: Token) -> Result<String> {
    named_type_with_owners(image, token, 0, 0)
}
fn named_type_with_owners(
    image: &Image<'_>,
    token: Token,
    owner: u32,
    method: u32,
) -> Result<String> {
    if token.table() == Some(TableId::TypeSpec) {
        let row = image.md.tables().type_spec(token.rid())?;
        return cs_type(
            image,
            &cildec::TypeSpecSig::parse(image.md.blobs().get(row.signature)?)?.type_,
            owner,
            method,
        );
    }
    let names = cildec::Names::new(&image.md).try_token(token)?;
    let name = names
        .strip_prefix('[')
        .and_then(|s| s.split_once(']').map(|(_, n)| n))
        .unwrap_or(&names);
    Ok(name.replace('/', "."))
}
fn cs_type(image: &Image<'_>, ty: &Type<'_>, owner: u32, method: u32) -> Result<String> {
    Ok(match ty {
        Type::Void => "void".into(),
        Type::Boolean => "bool".into(),
        Type::Char => "char".into(),
        Type::I1 => "sbyte".into(),
        Type::U1 => "byte".into(),
        Type::I2 => "short".into(),
        Type::U2 => "ushort".into(),
        Type::I4 => "int".into(),
        Type::U4 => "uint".into(),
        Type::I8 => "long".into(),
        Type::U8 => "ulong".into(),
        Type::R4 => "float".into(),
        Type::R8 => "double".into(),
        Type::String => "string".into(),
        Type::Object => "object".into(),
        Type::IntPtr => "nint".into(),
        Type::UIntPtr => "nuint".into(),
        Type::Class(t) | Type::ValueType(t) => named_type_with_owners(image, *t, owner, method)?,
        Type::Var(n) | Type::MVar(n) => generic_names(
            image,
            if matches!(ty, Type::Var(_)) {
                owner
            } else {
                method
            },
        )?
        .get(*n as usize)
        .cloned()
        .unwrap_or_else(|| format!("T{n}")),
        Type::SzArray(_, t) => format!("{}[]", cs_type(image, t, owner, method)?),
        Type::Array(t, shape) => format!(
            "{}[{}]",
            cs_type(image, t, owner, method)?,
            ",".repeat(shape.rank.saturating_sub(1) as usize)
        ),
        Type::ByRef(t) => format!("ref {}", cs_type(image, t, owner, method)?),
        Type::Ptr(_, t) => format!("{}*", cs_type(image, t, owner, method)?),
        Type::Modified(_, t) | Type::Pinned(t) => cs_type(image, t, owner, method)?,
        Type::GenericInst { def, args, .. } => format!(
            "{}<{}>",
            named_type(image, *def)?.split('`').next().unwrap(),
            args.iter()
                .map(|t| cs_type(image, t, owner, method))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        ),
        _ => return Err("type requires detailed signature notation".into()),
    })
}
fn default_value(image: &Image<'_>, parent: u32) -> Result<Option<String>> {
    let rows = image
        .md
        .tables()
        .rows_for_parent(TableId::Constant, Token(parent))?;
    let Some(&rid) = rows.first() else {
        return Ok(None);
    };
    let row = image.md.tables().constant(rid)?;
    let b = image.md.blobs().get(row.value)?;
    Ok(Some(match row.type_raw {
        0x02 if b.len() == 1 => (b[0] != 0).to_string(),
        0x04 if b.len() == 1 => (b[0] as i8).to_string(),
        0x05 if b.len() == 1 => b[0].to_string(),
        0x06 if b.len() == 2 => i16::from_le_bytes(b.try_into()?).to_string(),
        0x07 if b.len() == 2 => u16::from_le_bytes(b.try_into()?).to_string(),
        0x08 if b.len() == 4 => i32::from_le_bytes(b.try_into()?).to_string(),
        0x09 if b.len() == 4 => format!("{}u", u32::from_le_bytes(b.try_into()?)),
        0x0a if b.len() == 8 => format!("{}L", i64::from_le_bytes(b.try_into()?)),
        0x0b if b.len() == 8 => format!("{}UL", u64::from_le_bytes(b.try_into()?)),
        0x0e if b.len() % 2 == 0 => format!(
            "\"{}\"",
            b.chunks_exact(2)
                .map(|c| format!("\\u{:04X}", u16::from_le_bytes([c[0], c[1]])))
                .collect::<String>()
        ),
        0x12 if b == [0, 0, 0, 0] => "null".into(),
        _ => return Err("constant requires detailed notation".into()),
    }))
}
