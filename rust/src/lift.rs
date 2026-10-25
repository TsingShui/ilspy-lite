// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{
    Result,
    ir::{StackKind, Ty},
    metadata::{Image, identifier},
};
use cildec::{FlowControl, Instruction, MethodBody, Operand};
use std::collections::VecDeque;

pub(crate) fn emit(image: &Image<'_>, token: u32, body: &MethodBody<'_>) -> Result<String> {
    if !body.exception_handlers.is_empty() || !body.unknown_sections.is_empty() {
        return Err("exception/data sections need IL fallback".into());
    }
    let row = image.row(token)?;
    // Synchronized and unmanaged implementation semantics cannot be erased.
    if row.impl_flags & !0x0108 != 0 {
        return Err("method implementation flags need IL fallback".into());
    }
    if !body.diagnostics().is_empty() {
        return Err("method-body parser diagnostics need IL fallback".into());
    }
    let name = image.md.strings().str(row.name)?;
    identifier(name)?;
    let sig = image.signature(token)?;
    if sig.explicit_this
        || sig.calling_convention != cildec::CallConv::Default
        || !sig.vararg_params.is_empty()
    {
        return Err("explicit-this/generic/vararg method needs IL fallback".into());
    }
    if sig.has_this == row.is_static() {
        return Err("inconsistent instance signature".into());
    }
    let mut args = sig
        .params
        .iter()
        .map(Ty::from_param)
        .collect::<Result<Vec<_>>>()?;
    if sig.has_this {
        args.insert(0, image.receiver(token)?);
    }
    let ret = Ty::from_param(&sig.return_type)?;
    for ty in args.iter().chain(std::iter::once(&ret)) {
        ty.cs(image)?;
    }
    let locals = image.local_types(body)?;
    if !locals.is_empty() && !body.flags.init_locals() {
        return Err("uninitialized local analysis is not implemented".into());
    }
    let instructions = body
        .instructions()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if instructions.len() > 100_000 {
        return Err("instruction limit exceeded".into());
    }
    let blocks = crate::cfg::build(&instructions, body.code.len())?;
    let mut entries: Vec<Option<Vec<Ty>>> = vec![None; blocks.len()];
    entries[0] = Some(Vec::new());
    let mut queue = VecDeque::from([0]);
    while let Some(n) = queue.pop_front() {
        let stack = entries[n]
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(s, &ty)| Value {
                name: format!("s{n}_{s}"),
                ty,
            })
            .collect();
        let mut w = Writer {
            image,
            stack,
            lines: Vec::new(),
            next: 0,
            args: args.clone(),
            locals: locals.clone(),
            ret,
        };
        for ins in &instructions[blocks[n].start..blocks[n].end] {
            w.instruction(ins)?;
            if w.stack.len() > usize::from(body.max_stack) {
                return Err("declared maxstack exceeded".into());
            }
        }
        let exit: Vec<_> = w.stack.iter().map(|v| v.ty).collect();
        for &s in &blocks[n].successors {
            if let Some(existing) = &entries[s] {
                if existing != &exit {
                    return Err("incompatible evaluation stacks at join".into());
                }
            } else {
                entries[s] = Some(exit.clone());
                queue.push_back(s);
            }
        }
    }
    if entries.iter().any(Option::is_none) {
        return Err("unreachable basic block requires IL fallback".into());
    }
    let mut writer = Writer {
        image,
        stack: Vec::new(),
        lines: Vec::new(),
        next: 0,
        args,
        locals,
        ret,
    };
    for (n, b) in blocks.iter().enumerate() {
        writer.stack = entries[n]
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(s, &ty)| Value {
                name: format!("s{n}_{s}"),
                ty,
            })
            .collect();
        writer.lines.push(format!("IL_{:04X}: ;", b.offset));
        for ins in &instructions[b.start..b.end] {
            writer.instruction(ins)?;
        }
        let last = &instructions[b.end - 1];
        let edge = |successor: usize, stack: &[Value]| -> Result<Vec<String>> {
            let mut lines = Vec::new();
            for (slot, value) in stack.iter().enumerate() {
                lines.push(format!(
                    "{} e{n}_{successor}_{slot} = {};",
                    value.ty.cs(image)?,
                    value.name
                ));
            }
            for slot in 0..stack.len() {
                lines.push(format!("s{successor}_{slot} = e{n}_{successor}_{slot};"));
            }
            lines.push(format!("goto IL_{:04X};", blocks[successor].offset));
            Ok(lines)
        };
        match last.info().flow {
            FlowControl::Return | FlowControl::Throw => {}
            FlowControl::CondBranch => {
                let condition = writer.lines.pop().ok_or("missing branch expression")?;
                if last.opcode.canonical().name() == "switch" {
                    writer.lines.push(format!("switch ({condition}) {{"));
                    for (case, &successor) in
                        b.successors[..b.successors.len() - 1].iter().enumerate()
                    {
                        writer.lines.push(format!("case {case}: {{"));
                        writer.lines.extend(edge(successor, &writer.stack)?);
                        writer.lines.push("}".into());
                    }
                    writer.lines.push("default: {".into());
                    writer
                        .lines
                        .extend(edge(*b.successors.last().unwrap(), &writer.stack)?);
                    writer.lines.push("} }".into());
                } else {
                    writer.lines.push(format!("if ({condition}) {{"));
                    writer.lines.extend(edge(b.successors[0], &writer.stack)?);
                    writer.lines.push("} else {".into());
                    writer.lines.extend(edge(b.successors[1], &writer.stack)?);
                    writer.lines.push("}".into());
                }
            }
            _ => writer.lines.extend(edge(b.successors[0], &writer.stack)?),
        }
    }
    let parameters = writer
        .args
        .iter()
        .enumerate()
        .map(|(i, t)| Ok(format!("{} a{i}", t.cs(image)?)))
        .collect::<Result<Vec<_>>>()?
        .join(", ");
    let mut out = if sig.has_this {
        "// Explicit receiver a0; no implicit entry null check.\n".into()
    } else {
        String::new()
    };
    if writer
        .args
        .iter()
        .chain(&writer.locals)
        .chain(std::iter::once(&writer.ret))
        .any(|ty| ty.kind() == StackKind::Native)
    {
        out.push_str(
            "// nint/nuint use the target runtime pointer width, not the analyzer host width.\n",
        );
    }
    out.push_str(&format!(
        "public static {} @{name}({parameters})\n{{\n",
        ret.cs(image)?
    ));
    for (i, t) in writer.locals.iter().enumerate() {
        out.push_str(&format!("    {} l{i} = default;\n", t.cs(image)?));
    }
    for (n, types) in entries.iter().enumerate() {
        for (s, t) in types.as_ref().unwrap().iter().enumerate() {
            out.push_str(&format!("    {} s{n}_{s} = default;\n", t.cs(image)?));
        }
    }
    for line in writer.lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("}\n");
    Ok(out)
}

#[derive(Clone)]
struct Value {
    name: String,
    ty: Ty,
}
struct Writer<'i, 'a> {
    image: &'i Image<'a>,
    stack: Vec<Value>,
    lines: Vec<String>,
    next: usize,
    args: Vec<Ty>,
    locals: Vec<Ty>,
    ret: Ty,
}
impl Writer<'_, '_> {
    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| "evaluation stack underflow".into())
    }
    fn push(&mut self, ty: Ty, expression: String) -> Result<()> {
        let name = format!("v{}", self.next);
        self.next += 1;
        self.lines
            .push(format!("{} {name} = {expression};", ty.cs(self.image)?));
        self.stack.push(Value { name, ty });
        Ok(())
    }
    fn instruction(&mut self, i: &Instruction) -> Result<()> {
        let op = i.opcode.canonical().name();
        self.lines
            .push(format!("// IL_{:04X}: {}", i.offset, i.opcode.name()));
        match op {
            "nop" => {}
            "ldnull" => self.stack.push(Value {
                name: "null".into(),
                ty: Ty::Null,
            }),
            "ldc.i4" => {
                let Operand::I32(n) = i.operand else {
                    return Err("bad i4 operand".into());
                };
                self.push(Ty::I4, n.to_string())?;
            }
            "ldc.i8" => {
                let Operand::I64(n) = i.operand else {
                    return Err("bad i8 operand".into());
                };
                self.push(Ty::I8, format!("{n}L"))?;
            }
            "ldarg" | "ldloc" => {
                let idx = var(i)?;
                let (ty, expr) = if op == "ldarg" {
                    (
                        *self.args.get(idx).ok_or("argument out of range")?,
                        format!("a{idx}"),
                    )
                } else {
                    (
                        *self.locals.get(idx).ok_or("local out of range")?,
                        format!("l{idx}"),
                    )
                };
                self.push(ty.stack(), ty.load(&expr, self.image)?)?;
            }
            "starg" | "stloc" => {
                let idx = var(i)?;
                let value = self.pop()?;
                let (ty, dest) = if op == "starg" {
                    (
                        *self.args.get(idx).ok_or("argument out of range")?,
                        format!("a{idx}"),
                    )
                } else {
                    (
                        *self.locals.get(idx).ok_or("local out of range")?,
                        format!("l{idx}"),
                    )
                };
                self.image.assignable(value.ty, ty.stack())?;
                self.lines
                    .push(format!("{dest} = {};", ty.store(&value.name, self.image)?));
            }
            "ldelem.i4" | "ldelem.u4" => {
                let index = self.pop()?;
                expect(index.ty, Ty::I4)?;
                let array = self.pop()?;
                let element = array_element(array.ty)?;
                let target = format!("{}[{}]", array.name, index.name);
                self.push(Ty::I4, element.load(&target, self.image)?)?;
            }
            "stelem.i4" => {
                let value = self.pop()?;
                expect(value.ty, Ty::I4)?;
                let index = self.pop()?;
                expect(index.ty, Ty::I4)?;
                let array = self.pop()?;
                let element = array_element(array.ty)?;
                self.lines.push(format!(
                    "{}[{}] = {};",
                    array.name,
                    index.name,
                    element.store(&value.name, self.image)?
                ));
            }
            "dup" => {
                let v = self.stack.last().ok_or("dup on empty stack")?.clone();
                self.stack.push(v);
            }
            "pop" => {
                self.pop()?;
            }
            "add" | "add.ovf" | "add.ovf.un" | "sub" | "sub.ovf" | "sub.ovf.un" | "mul"
            | "mul.ovf" | "mul.ovf.un" | "and" | "or" | "xor" => {
                let b = self.pop()?;
                let a = self.pop()?;
                expect(a.ty, b.ty)?;
                let symbol = match op.split('.').next().unwrap_or("") {
                    "add" => "+",
                    "sub" => "-",
                    "mul" => "*",
                    "and" => "&",
                    "or" => "|",
                    _ => "^",
                };
                let unsigned = op.ends_with(".un");
                let cast = if a.ty == Ty::I8 { "ulong" } else { "uint" };
                let (x, y) = if unsigned {
                    (
                        format!("unchecked(({cast}){})", a.name),
                        format!("unchecked(({cast}){})", b.name),
                    )
                } else {
                    (a.name, b.name)
                };
                let mode = if op.contains(".ovf") {
                    "checked"
                } else {
                    "unchecked"
                };
                let expression = format!("{mode}({x} {symbol} {y})");
                self.push(
                    a.ty,
                    if unsigned {
                        format!("unchecked(({}){expression})", a.ty.cs(self.image)?)
                    } else {
                        expression
                    },
                )?;
            }
            "div" | "div.un" | "rem" | "rem.un" => {
                let b = self.pop()?;
                let a = self.pop()?;
                expect(a.ty, b.ty)?;
                let symbol = if op.starts_with("div") { "/" } else { "%" };
                let (x, y) = if op.ends_with(".un") {
                    let ty = if a.ty == Ty::I8 { "ulong" } else { "uint" };
                    (
                        format!("unchecked(({ty}){})", a.name),
                        format!("unchecked(({ty}){})", b.name),
                    )
                } else {
                    (a.name, b.name)
                };
                self.push(
                    a.ty,
                    if op.ends_with(".un") {
                        format!("unchecked(({})({x} {symbol} {y}))", a.ty.cs(self.image)?)
                    } else {
                        format!("unchecked({x} {symbol} {y})")
                    },
                )?;
            }
            "neg" | "not" => {
                let a = self.pop()?;
                integer(a.ty)?;
                let symbol = if op == "neg" { "-" } else { "~" };
                self.push(a.ty, format!("unchecked({symbol}{})", a.name))?;
            }
            "shl" | "shr" | "shr.un" => {
                let b = self.pop()?;
                let a = self.pop()?;
                expect(b.ty, Ty::I4)?;
                integer(a.ty)?;
                let symbol = if op == "shl" {
                    "<<"
                } else if op == "shr.un" {
                    ">>>"
                } else {
                    ">>"
                };
                self.push(a.ty, format!("({} {symbol} {})", a.name, b.name))?;
            }
            "ceq" | "cgt" | "cgt.un" | "clt" | "clt.un" => {
                let b = self.pop()?;
                let a = self.pop()?;
                let cmp = comparison(op, &a, &b)?;
                self.push(Ty::I4, format!("({cmp} ? 1 : 0)"))?;
            }
            "br" => {}
            "brtrue" | "brfalse" => {
                let a = self.pop()?;
                integer(a.ty)?;
                self.lines.push(format!(
                    "{} {} 0",
                    a.name,
                    if op == "brtrue" { "!=" } else { "==" }
                ));
            }
            "beq" | "bne.un" | "bge" | "bge.un" | "bgt" | "bgt.un" | "ble" | "ble.un" | "blt"
            | "blt.un" => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.lines.push(comparison(op, &a, &b)?);
            }
            "switch" => {
                let a = self.pop()?;
                expect(a.ty, Ty::I4)?;
                self.lines.push(a.name);
            }
            "call" => {
                let t = i.operand.token().ok_or("missing call token")?;
                let method = self.image.method_ref(t)?;
                let mut values = Vec::new();
                for &ty in method.args.iter().rev() {
                    let v = self.pop()?;
                    self.image.assignable(v.ty, ty.stack())?;
                    values.push(ty.store(&v.name, self.image)?);
                }
                values.reverse();
                if let Some(owner) = method.receiver {
                    let receiver = self.pop()?;
                    self.image.assignable(receiver.ty, owner)?;
                    values.insert(0, receiver.name);
                    self.lines.push("// __cil_direct: exact IL call target; no virtual dispatch or added null check.".into());
                }
                let expression = format!("{}({})", method.target, values.join(", "));
                self.lines.push(format!(
                    "// call token 0x{:08X}: {}",
                    method.token,
                    crate::metadata::escape_line(&cildec::Names::new(&self.image.md).try_token(t)?)
                ));
                if method.ret == Ty::Void {
                    self.lines.push(format!("{expression};"));
                } else {
                    self.push(
                        method.ret.stack(),
                        method.ret.load(&expression, self.image)?,
                    )?;
                }
            }
            "ldfld" | "stfld" => {
                let t = i.operand.token().ok_or("missing field token")?;
                let field = self.image.field(t)?;
                if field.is_static {
                    return Err("instance access to static field".into());
                }
                let value = if op == "stfld" {
                    Some(self.pop()?)
                } else {
                    None
                };
                let receiver = self.pop()?;
                self.image.assignable(receiver.ty, field.owner)?;
                // Keep base casts: a derived type can hide the named field.
                let target = if receiver.ty == field.owner {
                    format!("{}.{}", receiver.name, field.name)
                } else {
                    format!(
                        "(({}){}).{}",
                        field.owner.cs(self.image)?,
                        receiver.name,
                        field.name
                    )
                };
                self.lines.push(format!(
                    "// field token 0x{:08X}: {}",
                    field.token,
                    crate::metadata::escape_line(&cildec::Names::new(&self.image.md).try_token(t)?)
                ));
                if let Some(v) = value {
                    if field.readonly {
                        return Err("readonly field write needs IL fallback".into());
                    }
                    self.image.assignable(v.ty, field.ty.stack())?;
                    self.lines.push(format!(
                        "{target} = {};",
                        field.ty.store(&v.name, self.image)?
                    ));
                } else {
                    self.push(field.ty.stack(), field.ty.load(&target, self.image)?)?;
                }
            }
            "ldsfld" | "stsfld" => {
                let t = i.operand.token().ok_or("missing field token")?;
                let (field, ty) = self.image.static_field(t)?;
                self.lines.push(format!("// field token 0x{:08X}", t.0));
                if op == "ldsfld" {
                    self.push(ty.stack(), ty.load(&field, self.image)?)?;
                } else {
                    let v = self.pop()?;
                    if ty.kind() == StackKind::Native {
                        self.image.assignable(v.ty, ty.stack())?;
                    } else {
                        expect(v.ty, ty.stack())?;
                    }
                    self.lines
                        .push(format!("{field} = {};", ty.store(&v.name, self.image)?));
                }
            }
            "ret" => {
                if self.ret == Ty::Void {
                    self.lines.push("return;".into());
                } else {
                    let value = self.pop()?;
                    self.image.assignable(value.ty, self.ret.stack())?;
                    self.lines.push(format!(
                        "return {};",
                        self.ret.store(&value.name, self.image)?
                    ));
                }
                if !self.stack.is_empty() {
                    return Err("return leaves excess stack values".into());
                }
            }
            _ => {
                return Err(format!(
                    "unsupported opcode {} at IL_{:04X}",
                    i.opcode.name(),
                    i.offset
                )
                .into());
            }
        }
        Ok(())
    }
}
fn comparison(op: &str, a: &Value, b: &Value) -> Result<String> {
    expect(a.ty, b.ty)?;
    let symbol = match op.split('.').next().unwrap_or("") {
        "ceq" | "beq" => "==",
        "bne" => "!=",
        "cgt" | "bgt" => ">",
        "clt" | "blt" => "<",
        "bge" => ">=",
        "ble" => "<=",
        _ => return Err("unknown comparison".into()),
    };
    let (x, y) = if op.ends_with(".un") {
        let ty = if a.ty == Ty::I8 { "ulong" } else { "uint" };
        (
            format!("unchecked(({ty}){})", a.name),
            format!("unchecked(({ty}){})", b.name),
        )
    } else {
        (a.name.clone(), b.name.clone())
    };
    Ok(format!("{x} {symbol} {y}"))
}
fn var(i: &Instruction) -> Result<usize> {
    if let Operand::Var(n) = i.operand {
        Ok(n as usize)
    } else {
        Err("expected variable operand".into())
    }
}
fn array_element(ty: Ty) -> Result<Ty> {
    match ty {
        Ty::I4Array => Ok(Ty::I4),
        Ty::U4Array => Ok(Ty::U4),
        _ => Err("expected int[] or uint[] evaluation-stack value".into()),
    }
}
fn integer(ty: Ty) -> Result<()> {
    if matches!(ty.kind(), StackKind::I4 | StackKind::I8) {
        Ok(())
    } else {
        Err("integer operation on non-integer value".into())
    }
}
fn expect(actual: Ty, expected: Ty) -> Result<()> {
    integer(actual)?;
    integer(expected)?;
    if actual == expected {
        Ok(())
    } else {
        Err("incompatible stack categories".into())
    }
}
