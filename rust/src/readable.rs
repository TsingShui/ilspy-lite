// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{
    DecompiledMethod, Result, Status,
    metadata::{Image, SelectedType},
    symbol_names::{ident, keyword},
};
mod post_decrement;
mod property;

use cildec::{Param, Rid, TableId, Token, Type};
use std::collections::{BTreeMap, BTreeSet};

struct View<'a, 'b> {
    image: &'a Image<'b>,
    dependencies: Option<&'a crate::DependencyContext>,
}

pub(crate) fn type_view(
    image: &Image<'_>,
    types: &[SelectedType],
    bodies: &BTreeMap<u32, DecompiledMethod>,
    dependencies: Option<&crate::DependencyContext>,
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
        out.push_str(&format!(
            "namespace {};\n\n",
            crate::symbol_names::namespace(namespace)
        ));
    }
    let mut emitted = BTreeSet::new();
    let view = View {
        image,
        dependencies,
    };
    render_type(&view, root, types, bodies, 0, &mut emitted, &mut out);
    out
}

fn render_type(
    view: &View<'_, '_>,
    ty: &SelectedType,
    types: &[SelectedType],
    bodies: &BTreeMap<u32, DecompiledMethod>,
    indent: usize,
    emitted: &mut BTreeSet<u32>,
    out: &mut String,
) {
    let image = view.image;
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
        render_method(view, token, ty.token, result, indent + 1, out);
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
            render_type(view, nested, types, bodies, indent + 1, emitted, out);
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
                named_type_with_owners(image, row.event_type, owner, 0)?,
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
        crate::symbol_names::type_ident(name, !generics.is_empty(), false),
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
    let name = crate::symbol_names::field(token, original, false);
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
            crate::symbol_names::type_ident(
                image.md.strings().str(ty.type_name)?,
                !image.md.tables().generic_params(Token(owner))?.is_empty(),
                false
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
            crate::symbol_names::method(image, Token(token), name, false),
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
    view: &View<'_, '_>,
    token: u32,
    owner: u32,
    result: &DecompiledMethod,
    indent: usize,
    out: &mut String,
) {
    let image = view.image;
    let (mut header, names) = method_header(image, token, owner)
        .unwrap_or_else(|_| (format!("void __method_{token:08X}()"), BTreeMap::new()));
    let mut body = if result.status == Status::Decompiled {
        method_body(&result.text, &names)
    } else {
        String::new()
    };
    if result.status == Status::Decompiled {
        body = post_decrement::rewrite(image, token, &body);
    }
    if let Some(context) = view.dependencies {
        body = property::rewrite(context, token, &body);
    }
    if let Some((initializer, remaining)) =
        constructor_initializer(image, token, owner, &names, &body)
    {
        header.push_str(&initializer);
        body = remaining.to_string();
    }
    if let Ok(row) = image.row(token) {
        if let Ok(raw) = image.md.strings().str(row.name) {
            if let Some(comment) = crate::symbol_names::method_comment(raw) {
                line(out, indent, &comment);
            }
        }
    }
    if result.status == Status::Declaration {
        line(out, indent, &format!("{header};"));
        return;
    }
    line(out, indent, &header);
    line(out, indent, "{");
    for evidence in result.text.lines().filter(|l| {
        l.starts_with("// structural signature ")
            || l.starts_with("// generic body context ")
            || l.starts_with("// enum storage ")
    }) {
        line(out, indent + 1, evidence);
    }
    if result.status != Status::Decompiled {
        let mut evidence = result
            .text
            .lines()
            .skip_while(|l| !l.starts_with("// IL FALLBACK:") && !l.starts_with("// FAILED"));
        let notice = evidence
            .next()
            .unwrap_or("// FAILED: unreadable method body");
        line(
            out,
            indent + 1,
            &notice
                .replacen("IL FALLBACK:", "IL fallback:", 1)
                .replacen("FAILED", "Failed", 1),
        );
        // Keep the entire fallback body, including locals/EH/raw undecoded
        // evidence, without rewriting IL operands as C# variables.
        for entry in evidence {
            if !entry.starts_with("// // token ") && !entry.starts_with("// .method-flags ") {
                line(out, indent + 1, entry);
            }
        }
    } else {
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

// Only the first rendered operation can occupy an initializer. Parameter types
// must match the target exactly so this does not introduce overload guesses.
fn constructor_initializer<'b>(
    image: &Image<'_>,
    token: u32,
    owner: u32,
    names: &BTreeMap<String, String>,
    body: &'b str,
) -> Option<(String, &'b str)> {
    use crate::ir::Ty;
    if image.md.strings().str(image.row(token).ok()?.name).ok()? != ".ctor" {
        return None;
    }
    let (first, remaining) = body.split_once('\n')?;
    let call = first
        .strip_prefix("__cil_ctor_call_")
        .or_else(|| first.strip_prefix("__cil_core_object_ctor_"))?;
    let (target, arguments) = call.strip_suffix(");")?.split_once('(')?;
    if target.len() != 8 {
        return None;
    }
    let target = image
        .method_ref(
            Token(u32::from_str_radix(target, 16).ok()?),
            Default::default(),
        )
        .ok()?;
    let target_owner = match (target.receiver, target.core_object_constructor) {
        (Some(Ty::Class(t) | Ty::ExternalClass(t)), None) if target.constructor => t,
        (None, Some(t)) => t,
        _ => return None,
    };
    let kind = if target_owner == owner {
        "this"
    } else if image.md.tables().type_def(owner & 0xffffff).ok()?.extends.0 == target_owner {
        "base"
    } else {
        return None;
    };
    let arguments = arguments.split(", ").collect::<Vec<_>>();
    if arguments.first() != Some(&"this") || arguments.len() != target.args.len() + 1 {
        return None;
    }
    let signature = image.signature(token).ok()?;
    for (&argument, &expected) in arguments[1..].iter().zip(&target.args) {
        if expected == Ty::I4 && argument.parse::<i32>().is_ok()
            || expected == Ty::Bool && matches!(argument, "true" | "false")
        {
            continue;
        }
        let (slot, _) = names
            .iter()
            .find(|(slot, name)| slot.as_str() != "a0" && name.as_str() == argument)?;
        let index = slot
            .strip_prefix('a')?
            .parse::<usize>()
            .ok()?
            .checked_sub(1)?;
        if Ty::from_param(signature.params.get(index)?, image).ok()? != expected {
            return None;
        }
    }
    Some((
        format!(" : {kind}({})", arguments[1..].join(", ")),
        remaining,
    ))
}

pub(crate) fn method_view(
    image: &Image<'_>,
    token: u32,
    result: &DecompiledMethod,
    dependencies: Option<&crate::DependencyContext>,
) -> String {
    let owner = image
        .md
        .tables()
        .type_of_method(Rid::new(token & 0xffffff))
        .ok()
        .flatten()
        .map_or(0, |r| 0x02000000 | r.get());
    let mut out = String::new();
    render_method(
        &View {
            image,
            dependencies,
        },
        token,
        owner,
        result,
        0,
        &mut out,
    );
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
    let lines: Vec<String> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .map(str::to_owned)
        .collect();
    // Runtime regions and acquired locations keep explicit snapshot lifetimes.
    // A ref binding is a mutable location, not an immutable value that can be
    // substituted into a later expression after an alias write or call.
    let lines = if lines
        .iter()
        .any(|line| line == "try {" || line.starts_with("ref "))
    {
        lines
    } else {
        let lines = compact_lines(lines);
        compact_lines(lines)
    };
    lines
        .into_iter()
        .map(|line| inline_literals(&rewrite(&line, names)) + "\n")
        .collect()
}

/// Rewrite `__cil_ldstr_TOKEN("literal")` to the literal itself.
///
/// The helper only records the original module string-literal load position; the
/// value it yields is exactly the literal, so the readable view prints the
/// literal and the token evidence stays in `--details`.
fn inline_literals(line: &str) -> String {
    const PREFIX: &str = "__cil_ldstr_";
    let Some(at) = line.find(PREFIX) else {
        return line.to_owned();
    };
    let rest = &line[at + PREFIX.len()..];
    let Some(open) = rest.find('(') else {
        return line.to_owned();
    };
    if !rest[..open]
        .bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_lowercase())
    {
        return line.to_owned();
    }
    let body = &rest[open + 1..];
    if !body.starts_with('"') {
        return line.to_owned();
    }
    // The literal is a single quoted run with backslash escapes, so the closing
    // quote is the first unescaped one.
    let mut end = 1usize;
    let bytes = body.as_bytes();
    while end < bytes.len() {
        match bytes[end] {
            b'\\' => end += 2,
            b'"' => break,
            _ => end += 1,
        }
    }
    if end >= bytes.len() || !body[end + 1..].trim_start().starts_with(')') {
        return line.to_owned();
    }
    let literal = &body[..end + 1];
    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..at]);
    out.push_str(literal);
    let tail = &body[end + 1..];
    let after = tail.find(')').map(|i| &tail[i + 1..]).unwrap_or("");
    out.push_str(after);
    out
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
                let replacement = if next == &format!("return ({name} != 0);")
                    || next == &format!("return __cil_bool_store({name});")
                {
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
        if let Some((name, expr)) = value_declaration(&lines[i])
            && let Some(value) = expr
                .strip_prefix("__cil_bool_load(")
                .and_then(|s| s.strip_suffix(')'))
            && lines[i + 1] == format!("return __cil_bool_store({name});")
            && lines
                .iter()
                .enumerate()
                .filter(|(n, _)| *n != i)
                .map(|(_, line)| identifiers(line).iter().filter(|id| *id == name).count())
                .sum::<usize>()
                == 1
        {
            lines[i + 1] = format!("return {value};");
            lines.remove(i);
            continue;
        }
        if lines[i] == "return (1 != 0);" || lines[i] == "return __cil_bool_store(1);" {
            lines[i] = "return true;".into();
        }
        if lines[i] == "return (0 != 0);" || lines[i] == "return __cil_bool_store(0);" {
            lines[i] = "return false;".into();
        }
        i += 1;
    }
    for line in &mut lines {
        if line == "return (1 != 0);" || line == "return __cil_bool_store(1);" {
            *line = "return true;".into();
        }
        if line == "return (0 != 0);" || line == "return __cil_bool_store(0);" {
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
    // Generated declarations start with a type. Preserve that syntactic slot
    // separately even after an earlier pass removed its global:: qualifier.
    let type_end = if value_declaration(text).is_some() {
        text.split_once(" = ").and_then(|(left, _)| left.rfind(' '))
    } else {
        None
    };
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
            if member_slot || type_end.is_some_and(|end| start < end) {
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
                while let Some(&(_, ch)) = chars.peek() {
                    if ch == '.' || ch == '_' || ch == '@' || ch.is_ascii_alphanumeric() {
                        let (start, _) = chars.next().unwrap();
                        let mut end = start + ch.len_utf8();
                        if ch == '@' || ch == '_' || ch.is_ascii_alphabetic() {
                            while let Some(&(i, c)) = chars.peek() {
                                if c == '_' || c.is_ascii_alphanumeric() {
                                    chars.next();
                                    end = i + c.len_utf8();
                                } else {
                                    break;
                                }
                            }
                        }
                        let part = &text[start..end];
                        out.push_str(
                            part.strip_prefix('@')
                                .filter(|s| !keyword(s))
                                .unwrap_or(part),
                        );
                    } else {
                        break;
                    }
                }
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
    crate::symbol_names::type_name(image, token, false)
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
            crate::metadata::escape_units(
                b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]))
            )
        ),
        0x12 if b == [0, 0, 0, 0] => "null".into(),
        _ => return Err("constant requires detailed notation".into()),
    }))
}
