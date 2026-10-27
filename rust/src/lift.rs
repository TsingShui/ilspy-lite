// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{
    Result,
    ir::{StackKind, Ty},
    metadata::Image,
};
use cildec::{FlowControl, Instruction, MethodBody, Operand};
use std::collections::{BTreeSet, HashSet, VecDeque};

pub(crate) fn emit(image: &Image<'_>, token: u32, body: &MethodBody<'_>) -> Result<String> {
    if !body.unknown_sections.is_empty() {
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
    // A `.cctor` is an ordinary static void body whose statements run exactly once
    // at type initialization, so it is lowered like any other body and rendered as
    // the type's static constructor. An empty name or a runtime-special method
    // other than a constructor has no writable C# member at all.
    if name.is_empty() || row.flags & 0x1000 != 0 && !matches!(name, ".ctor" | ".cctor") {
        return Err("empty/runtime-special method needs IL fallback".into());
    }
    let sig = image.signature(token)?;
    if sig.explicit_this
        || !matches!(
            sig.calling_convention,
            cildec::CallConv::Default | cildec::CallConv::Generic(_)
        )
        || !sig.vararg_params.is_empty()
    {
        return Err("explicit-this/vararg method needs IL fallback".into());
    }
    if sig.has_this == row.is_static() {
        return Err("inconsistent instance signature".into());
    }
    let context = image.method_type_context(token, &sig)?;
    let mut args = sig
        .params
        .iter()
        .map(|p| Ty::from_param_scoped(p, image, context))
        .collect::<Result<Vec<_>>>()?;
    if sig.has_this {
        args.insert(0, image.receiver(token)?);
    }
    let ret = Ty::from_param_scoped(&sig.return_type, image, context)?;
    if ret.kind() == StackKind::Address {
        return Err("byref return needs escape/lifetime analysis".into());
    }
    let constructor_owner = if name == ".ctor" {
        if !sig.has_this
            || row.flags & 0x1800 != 0x1800
            || row.flags & 0x440 != 0
            || ret != Ty::Void
        {
            return Err("invalid instance constructor context needs IL fallback".into());
        }
        Some(match args[0] {
            Ty::ValueAddress(id) => Ty::Value(id),
            other => other,
        })
    } else {
        None
    };
    // Only the declaring type's own static constructor may assign its
    // `static readonly` fields.
    let static_constructor = if name == ".cctor" {
        if sig.has_this || ret != Ty::Void {
            return Err("invalid static constructor context needs IL fallback".into());
        }
        let owner = image
            .md
            .tables()
            .type_of_method(cildec::Rid::new(token & 0xffffff))?
            .ok_or("static constructor has no declaring type")?;
        Some(0x02000000 | owner.get())
    } else {
        None
    };
    for ty in args.iter().chain(std::iter::once(&ret)) {
        ty.cs(image)?;
    }
    let locals = image.local_types(body, context)?;
    let instructions = body
        .instructions()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if instructions.len() > 100_000 {
        return Err("instruction limit exceeded".into());
    }
    if constructor_owner.is_some()
        && instructions
            .iter()
            .any(|i| i.opcode.canonical().name() == "starg" && matches!(i.operand, Operand::Var(0)))
    {
        return Err("constructor receiver reassignment needs IL fallback".into());
    }
    let constrained_calls = crate::constrained::calls(&instructions)?;
    let volatile_accesses = crate::volatile::accesses(&instructions)?;
    let tail_calls = crate::tail::check(&instructions)?;
    let boundaries = crate::eh::boundaries(body, &instructions)?;
    let blocks = crate::cfg::build(&instructions, body.code.len(), &boundaries)?;
    if !locals.is_empty() && !body.flags.init_locals() {
        require_definite_local_assignment(&blocks, &instructions, locals.len(), body, image)?;
    }
    if blocks
        .iter()
        .any(|block| constrained_calls.contains_key(&block.offset))
    {
        return Err("control flow cannot enter between constrained prefix and callvirt".into());
    }
    let regions = crate::eh::parse(image, body, &instructions)?;
    let mut entries: Vec<Option<Vec<Ty>>> = vec![None; blocks.len()];
    // Stack index carrying the constructor's original `this` into each block, so
    // a base-constructor call after a join still proves it uses that receiver.
    let mut this_slots: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); blocks.len()];
    entries[0] = Some(Vec::new());
    let mut queue = VecDeque::from([0]);
    loop {
        while let Some(n) = queue.pop_front() {
            let mut original_seed: HashSet<String> = HashSet::new();
            let stack: Vec<Value> = entries[n]
                .as_ref()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(s, &ty)| Value {
                    name: format!("s{n}_{s}"),
                    ty,
                })
                .collect();
            for index in &this_slots[n] {
                if *index < stack.len() {
                    original_seed.insert(format!("s{n}_{index}"));
                }
            }
            let mut w = Writer {
                image,
                analyzing: true,
                constrained_calls: &constrained_calls,
                volatile_accesses: &volatile_accesses,
                tail_calls: &tail_calls,
                type_context: context,
                stack,
                lines: Vec::new(),
                next: 0,
                args: args.clone(),
                locals: locals.clone(),
                ret,
                constructor_owner,
                static_constructor,
                original_this: original_seed,
                structural_types: BTreeSet::new(),
                enum_types: BTreeSet::new(),
            };
            for ins in &instructions[blocks[n].start..blocks[n].end] {
                w.instruction(ins)?;
                if w.stack.len() > usize::from(body.max_stack) {
                    return Err("declared maxstack exceeded".into());
                }
            }
            let exit: Vec<_> = w.stack.iter().map(|v| v.ty).collect();
            for &s in &blocks[n].successors {
                if let Some(existing) = &mut entries[s] {
                    if existing.len() != exit.len() {
                        return Err("incompatible evaluation stacks at join".into());
                    }
                    let mut changed = false;
                    for (stored, &incoming) in existing.iter_mut().zip(&exit) {
                        if *stored == incoming {
                            continue;
                        }
                        if let Some(merged) = join_reference_type(image, *stored, incoming) {
                            if merged != *stored {
                                *stored = merged;
                                changed = true;
                            }
                        } else {
                            return Err("incompatible evaluation stacks at join".into());
                        }
                    }
                    let exit_this: BTreeSet<usize> = w
                        .original_this
                        .iter()
                        .filter_map(|name| w.stack.iter().position(|value| &value.name == name))
                        .collect();
                    this_slots[s].extend(exit_this);
                    if changed {
                        queue.push_back(s);
                    }
                } else {
                    entries[s] = Some(exit.clone());
                    this_slots[s] = w
                        .original_this
                        .iter()
                        .filter_map(|name| w.stack.iter().position(|value| &value.name == name))
                        .collect();
                    queue.push_back(s);
                }
            }
        }
        let seeded = regions.seed_handlers(&blocks, &mut entries);
        if seeded.is_empty() {
            break;
        }
        queue.extend(seeded);
    }
    if entries.iter().any(Option::is_none) {
        return Err("unreachable basic block requires IL fallback".into());
    }
    let mut writer = Writer {
        image,
        analyzing: false,
        constrained_calls: &constrained_calls,
        volatile_accesses: &volatile_accesses,
        tail_calls: &tail_calls,
        type_context: context,
        stack: Vec::new(),
        lines: Vec::new(),
        next: 0,
        args,
        locals,
        ret,
        constructor_owner,
        static_constructor,
        original_this: HashSet::new(),
        structural_types: BTreeSet::new(),
        enum_types: BTreeSet::new(),
    };
    let mut previous = None;
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
        writer.original_this = this_slots[n]
            .iter()
            .filter(|index| **index < writer.stack.len())
            .map(|index| format!("s{n}_{index}"))
            .collect();
        writer.lines.extend(regions.transition(previous, b.offset));
        previous = Some(b.offset);
        writer.lines.push(format!("IL_{:04X}: ;", b.offset));
        if let Some(variable) = regions.catch_variable(b.offset) {
            writer.lines.push(format!("s{n}_0 = {variable};"));
        }
        for ins in &instructions[b.start..b.end] {
            writer.instruction(ins)?;
        }
        let last = &instructions[b.end - 1];
        let edge = |successor: usize, stack: &[Value]| -> Result<Vec<String>> {
            let mut lines = Vec::new();
            for (slot, value) in stack.iter().enumerate() {
                lines.push(format!(
                    "{} e{n}_{successor}_{slot} = {}{};",
                    entries[successor].as_ref().unwrap()[slot].cs(image)?,
                    if value.ty.kind() == StackKind::Address {
                        "ref "
                    } else {
                        ""
                    },
                    if value.ty == Ty::Null {
                        "null"
                    } else {
                        &value.name
                    }
                ));
            }
            for (slot, value) in stack.iter().enumerate() {
                lines.push(format!(
                    "s{successor}_{slot} = {}e{n}_{successor}_{slot};",
                    if value.ty.kind() == StackKind::Address {
                        "ref "
                    } else {
                        ""
                    }
                ));
            }
            let target = blocks[successor].offset;
            if regions.enters_try(last.offset, target) {
                if successor != n + 1 || !stack.is_empty() {
                    return Err("EH entry must fall through with an empty stack".into());
                }
            } else {
                lines.push(format!("goto IL_{target:04X};"));
            }
            Ok(lines)
        };
        if last.opcode.canonical().name() == "endfinally" {
            let index = regions
                .finally_end(last.offset)
                .ok_or("EH endfinally outside finally")?;
            writer
                .lines
                .push(format!("goto __cil_finally_end_{index};"));
        }
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
    writer
        .lines
        .extend(regions.transition(previous, body.code.len() as u32));
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
        || instructions
            .iter()
            .any(|i| i.opcode.canonical().name() == "ldlen")
    {
        out.push_str(
            "// nint/nuint use the target runtime pointer width, not the analyzer host width.\n",
        );
    }
    let storage_types = writer
        .args
        .iter()
        .chain(&writer.locals)
        .chain(std::iter::once(&writer.ret))
        .map(|ty| {
            if ty.kind() == StackKind::Address {
                image.address_element(*ty)
            } else {
                Ok(*ty)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    if storage_types.iter().any(|ty| ty.kind() == StackKind::Float)
        || instructions
            .iter()
            .any(|i| matches!(i.opcode.canonical().name(), "conv.r4" | "ldc.r4"))
    {
        out.push_str("// __cil_F is a nominal R4 evaluation/local value; CLI F precision is target-runtime-defined, not analyzer-host-defined.\n");
        out.push_str("// conv.r4 rounds to binary32 (nearest); local/argument snapshots do not add a storage rounding. Field storage and return/call boundaries stay explicit.\n");
    }
    out.push_str(&context.evidence());
    let external_types = storage_types
        .iter()
        .filter_map(|ty| {
            if let Ty::ExternalClass(token) = ty {
                Some(*token)
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    for token in external_types {
        out.push_str(&format!("// external class token 0x{token:08X}: {} = {}; opaque identity, no dependency loaded.\n",
            image.external_class_name(token)?, image.external_class_context(token)?));
    }
    let structural_types = storage_types
        .iter()
        .filter_map(|ty| match ty {
            Ty::StructuralRef(id) | Ty::GenericValue(id) | Ty::Value(id) | Ty::ValueAddress(id) => {
                Some(*id)
            }
            _ => None,
        })
        .chain(writer.structural_types.iter().copied())
        .collect::<BTreeSet<_>>();
    for id in structural_types {
        out.push_str(&image.signature_types.borrow().evidence(id)?);
    }
    let enum_types = storage_types
        .iter()
        .filter_map(|ty| if let Ty::Enum(t) = ty { Some(*t) } else { None })
        .chain(writer.enum_types.iter().copied())
        .collect::<BTreeSet<_>>();
    for ty in enum_types {
        out.push_str(&ty.evidence(image)?);
    }
    let body_name = if constructor_owner.is_some() {
        out.push_str("// Instance constructor initialization body on an existing receiver; does not allocate or certify a complete constructor chain.\n");
        format!("__cil_ctor_body_{token:08X}")
    } else {
        if let Some(comment) = crate::symbol_names::method_comment(name) {
            out.push_str(&comment);
            out.push('\n');
        }
        crate::symbol_names::method(image, cildec::Token(token), name, true)
    };
    out.push_str(&format!(
        "public static {} {body_name}({parameters})\n{{\n",
        ret.cs(image)?
    ));
    for (i, t) in writer.locals.iter().enumerate() {
        let local = if *t == Ty::R4 { t.stack() } else { *t };
        if local.kind() == StackKind::Address {
            out.push_str(&format!(
                "    scoped {} l{i} = ref System.Runtime.CompilerServices.Unsafe.NullRef<{}>();\n",
                local.cs(image)?,
                image.address_element(local)?.cs(image)?
            ));
        } else {
            out.push_str(&format!("    {} l{i} = default;\n", local.cs(image)?));
        }
    }
    for (n, types) in entries.iter().enumerate() {
        for (s, t) in types.as_ref().unwrap().iter().enumerate() {
            if t.kind() == StackKind::Address {
                out.push_str(&format!(
                    "    scoped {} s{n}_{s} = ref System.Runtime.CompilerServices.Unsafe.NullRef<{}>();\n",
                    t.cs(image)?,
                    image.address_element(*t)?.cs(image)?
                ));
            } else {
                out.push_str(&format!("    {} s{n}_{s} = default;\n", t.cs(image)?));
            }
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
    /// Fixpoint analysis pass: provenance-dependent checks run in the final
    /// emission pass, where synthetic slot provenance has settled.
    analyzing: bool,
    constrained_calls: &'i std::collections::BTreeMap<u32, cildec::Token>,
    volatile_accesses: &'i std::collections::BTreeSet<u32>,
    tail_calls: &'i std::collections::BTreeSet<u32>,
    type_context: crate::signature_types::TypeContext,
    stack: Vec<Value>,
    lines: Vec<String>,
    next: usize,
    args: Vec<Ty>,
    locals: Vec<Ty>,
    ret: Ty,
    constructor_owner: Option<Ty>,
    /// Declaring type of the `.cctor` being lowered, which is the only member
    /// allowed to assign that type's `static readonly` fields.
    static_constructor: Option<u32>,
    original_this: HashSet<String>,
    structural_types: BTreeSet<crate::signature_types::TypeId>,
    enum_types: BTreeSet<crate::ir::EnumType>,
}
impl Writer<'_, '_> {
    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| "evaluation stack underflow".into())
    }
    fn track_type(&mut self, ty: Ty) -> Result<()> {
        if matches!(ty, Ty::ManagedAddress(_)) {
            return self.track_type(self.image.address_element(ty)?);
        }
        match ty {
            Ty::StructuralRef(id) | Ty::GenericValue(id) | Ty::Value(id) | Ty::ValueAddress(id) => {
                self.structural_types.insert(id);
            }
            Ty::Enum(t) => {
                self.enum_types.insert(t);
            }
            _ => {}
        }
        Ok(())
    }
    fn push(&mut self, ty: Ty, expression: String) -> Result<()> {
        self.track_type(ty)?;
        let name = format!("v{}", self.next);
        self.next += 1;
        let binding = if ty.kind() == StackKind::Address {
            "ref "
        } else {
            ""
        };
        self.lines.push(format!(
            "{} {name} = {binding}{expression};",
            ty.cs(self.image)?
        ));
        self.stack.push(Value { name, ty });
        Ok(())
    }
    /// Element access on a general array: `Get`, `Set` and `Address` name element
    /// slots, so this keeps the operand order and the runtime's null/bounds checks
    /// instead of dispatching to a method.
    fn array_intrinsic(&mut self, intrinsic: crate::arrays::Intrinsic) -> Result<()> {
        use crate::arrays::Access;
        let value = if intrinsic.access == Access::Set {
            Some(self.pop()?)
        } else {
            None
        };
        let mut indices = Vec::with_capacity(intrinsic.rank);
        for _ in 0..intrinsic.rank {
            let index = self.pop()?;
            if index.ty != Ty::I4 {
                return Err("array index needs a 32-bit integer value".into());
            }
            indices.push(index.name);
        }
        indices.reverse();
        let target = self.pop()?;
        let (element, rank) = crate::arrays::general_shape(self.image, target.ty)?;
        if element != intrinsic.element || rank != intrinsic.rank {
            return Err("array intrinsic does not match the array reference".into());
        }
        let name = format!("{}[{}]", target.name, indices.join(", "));
        match intrinsic.access {
            Access::Get => self.push(element, name),
            Access::Set => {
                let value = value.ok_or("array Set needs a value")?;
                if value.ty.kind() == StackKind::Address {
                    return Err("array Set needs a value snapshot, not a managed location".into());
                }
                self.image.assignable(value.ty, element.stack())?;
                self.lines.push(format!(
                    "{} = {};",
                    name,
                    element.store(&value.name, self.image)?
                ));
                Ok(())
            }
        }
    }

    fn constrained_call(
        &mut self,
        token: cildec::Token,
        constraint: cildec::Token,
        method: &crate::metadata::MethodRef,
    ) -> Result<()> {
        if method.generic_virtual {
            return Err("generic virtual dispatch needs runtime lookup-order modeling".into());
        }
        if method.receiver.is_none()
            || method.constructor
            || method.core_object_constructor.is_some()
            || method.tick_count
        {
            return Err("constrained call needs an ordinary instance method".into());
        }
        for ty in method.args.iter().chain(std::iter::once(&method.ret)) {
            self.track_type(*ty)?;
        }
        let mut values = Vec::new();
        for &ty in method.args.iter().rev() {
            let value = self.pop()?;
            if value.ty.kind() == StackKind::Address && ty.kind() != StackKind::Address {
                return Err(
                    "constrained argument needs a value snapshot, not a managed location".into(),
                );
            }
            self.image.assignable(value.ty, ty.stack())?;
            values.push(if ty.kind() == StackKind::Address {
                format!("ref {}", value.name)
            } else if ty == Ty::R4 {
                format!("__cil_call_arg_r4({})", value.name)
            } else {
                ty.store(&value.name, self.image)?
            });
        }
        values.reverse();
        let receiver = self.pop()?;
        let element = self
            .image
            .location_operand(receiver.ty, constraint, self.type_context)?;
        self.track_type(element)?;
        values.insert(0, format!("ref {}", receiver.name));
        self.lines.push(format!("// constrained type 0x{:08X}, callvirt method 0x{:08X}: exact runtime dispatch through this acquired location, including value boxing fallback or reference-slot dereference; no copied receiver.", constraint.0, token.0));
        let call = format!(
            "__cil_constrained_{:08X}_{:08X}({})",
            constraint.0,
            token.0,
            values.join(", ")
        );
        if method.ret == Ty::Void {
            self.lines.push(format!("{call};"));
        } else {
            let call = if method.ret == Ty::R4 {
                format!("__cil_call_ret_r4({call})")
            } else {
                call
            };
            self.push(method.ret.stack(), call)?;
        }
        Ok(())
    }

    fn instruction(&mut self, i: &Instruction) -> Result<()> {
        let op = i.opcode.canonical().name();
        self.lines
            .push(format!("// IL_{:04X}: {}", i.offset, i.opcode.name()));
        match op {
            "nop" | "constrained." | "volatile." | "tail." => {}
            "ldnull" => self.stack.push(Value {
                name: "null".into(),
                ty: Ty::Null,
            }),
            "ldstr" => {
                let Operand::String(token) = i.operand else {
                    return Err("missing user-string token".into());
                };
                let literal = self.image.string_literal(token)?;
                self.lines.push(format!(
                    "// string token 0x{:08X}: UTF-16 literal {literal}",
                    token.0
                ));
                self.lines.push("// __cil_ldstr: load using the original module string-literal policy; exact code units and load position, not a fresh string allocation.".into());
                self.push(
                    Ty::String,
                    format!("__cil_ldstr_{:08X}({literal})", token.0),
                )?;
            }
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
            "ldc.r4" => {
                let Operand::R32(value) = i.operand else {
                    return Err("bad r4 operand".into());
                };
                self.push(
                    Ty::F4,
                    format!("__cil_ldc_r4_bits(0x{:08X}u)", value.to_bits()),
                )?;
            }
            "ldc.r8" => {
                let Operand::R64(value) = i.operand else {
                    return Err("bad r8 operand".into());
                };
                self.push(
                    Ty::R8,
                    format!("__cil_ldc_r8_bits(0x{:016X}uL)", value.to_bits()),
                )?;
            }
            "conv.i1" | "conv.u1" | "conv.i2" | "conv.u2" => {
                let value = self.pop()?;
                if !matches!(
                    value.ty.kind(),
                    StackKind::I4 | StackKind::I8 | StackKind::Native
                ) {
                    return Err(format!("{op} source category needs IL fallback").into());
                }
                let storage = match op {
                    "conv.i1" => Ty::I1,
                    "conv.u1" => Ty::U1,
                    "conv.i2" => Ty::I2,
                    "conv.u2" => Ty::U2,
                    _ => unreachable!(),
                };
                let truncated = format!("unchecked(({})({}))", storage.cs(self.image)?, value.name);
                self.push(Ty::I4, storage.load(&truncated, self.image)?)?;
            }
            "conv.i" => {
                let value = self.pop()?;
                if value.ty.kind() == StackKind::Pointer {
                    self.push(Ty::Native, format!("unchecked((nint)({}))", value.name))?;
                    return Ok(());
                }
                if !matches!(
                    value.ty.kind(),
                    StackKind::I4 | StackKind::I8 | StackKind::Native
                ) {
                    return Err("conv.i source category needs IL fallback".into());
                }
                self.lines.push(
                    "// Sign-extend or truncate to the target runtime's native signed width."
                        .into(),
                );
                self.push(Ty::Native, format!("unchecked((nint)({}))", value.name))?;
            }
            "conv.u4" => {
                let value = self.pop()?;
                if value.ty.kind() == StackKind::Pointer {
                    self.push(Ty::U4, format!("unchecked((uint)(nuint)({}))", value.name))?;
                    return Ok(());
                }
                if !matches!(
                    value.ty.kind(),
                    StackKind::I4 | StackKind::I8 | StackKind::Native
                ) {
                    return Err("conv.u4 source category needs IL fallback".into());
                }
                self.lines.push("// Low 32 bits zero-extended into the 32-bit integer stack category; no sign extension.".into());
                self.push(Ty::U4, format!("unchecked((uint)({}))", value.name))?;
            }
            "conv.u" => {
                let value = self.pop()?;
                if value.ty.kind() == StackKind::Pointer {
                    self.push(Ty::UNative, format!("unchecked((nuint)({}))", value.name))?;
                    return Ok(());
                }
                if !matches!(
                    value.ty.kind(),
                    StackKind::I4 | StackKind::I8 | StackKind::Native
                ) {
                    return Err("conv.u source category needs IL fallback".into());
                }
                self.lines.push("// Zero-extend to the target runtime's native unsigned width; no sign extension.".into());
                self.push(Ty::UNative, format!("unchecked((nuint)({}))", value.name))?;
            }
            "conv.i4" | "conv.i8" | "conv.u8" => {
                let value = self.pop()?;
                let kind = value.ty.kind();
                if kind == StackKind::Pointer {
                    // A pointer converts to an integer through its native width;
                    // the truncation of conv.i4 is kept explicitly.
                    let expression = match op {
                        "conv.i4" => format!("unchecked((int)(nint)({}))", value.name),
                        "conv.u8" => format!("unchecked((long)(ulong)(nuint)({}))", value.name),
                        _ => format!("unchecked((long)(nint)({}))", value.name),
                    };
                    let target = if op == "conv.i4" { Ty::I4 } else { Ty::I8 };
                    self.push(target, expression)?;
                    return Ok(());
                }
                if !matches!(kind, StackKind::I4 | StackKind::I8 | StackKind::Native) {
                    return Err(format!("{op} source category needs IL fallback").into());
                }
                let target = if op == "conv.i4" { Ty::I4 } else { Ty::I8 };
                let expression = if op == "conv.u8" {
                    // Reinterpret at source width before widening, including native width.
                    match kind {
                        StackKind::I4 => format!("unchecked((long)(uint)({}))", value.name),
                        StackKind::Native => {
                            format!("unchecked((long)(ulong)(nuint)({}))", value.name)
                        }
                        _ => value.name,
                    }
                } else {
                    format!("unchecked(({})({}))", target.cs(self.image)?, value.name)
                };
                self.push(target, expression)?;
            }
            "conv.r.un" => {
                let value = self.pop()?;
                let suffix = match value.ty.kind() {
                    StackKind::I4 => "u4",
                    StackKind::I8 => "u8",
                    _ => return Err("conv.r.un source category needs IL fallback".into()),
                };
                self.lines.push("// Unsigned integer converted to CLI F (binary64, round to nearest); the source bits are read unsigned.".into());
                self.push(Ty::R8, format!("__cil_conv_r_{suffix}({})", value.name))?;
            }
            "conv.r8" => {
                let value = self.pop()?;
                match value.ty.kind() {
                    StackKind::I4 | StackKind::I8 | StackKind::Native => {
                        self.lines.push("// Integer widened to binary64 in CLI F; unsigned storage keeps its own sign rule in the source value.".into());
                        self.push(Ty::R8, format!("__cil_conv_r8({})", value.name))?;
                    }
                    StackKind::Float => {
                        self.lines.push(
                            "// Widen to binary64; values already binary64 are unchanged.".into(),
                        );
                        self.push(Ty::R8, format!("__cil_conv_r8({})", value.name))?;
                    }
                    _ => return Err("conv.r8 source category needs IL fallback".into()),
                }
            }
            "conv.r4" => {
                let value = self.pop()?;
                if value.ty.kind() == StackKind::Float {
                    self.lines.push("// Round to binary32 (nearest); NaN stays NaN and the sign of zero is preserved.".into());
                    self.push(Ty::F4, format!("__cil_conv_r4_float({})", value.name))?;
                    return Ok(());
                }
                if !matches!(
                    value.ty.kind(),
                    StackKind::I4 | StackKind::I8 | StackKind::Native
                ) {
                    return Err(
                        "conv.r4 needs a signed integer stack source in the admitted model".into(),
                    );
                }
                self.lines.push("// Signed integer value rounded to binary32 (nearest), then represented as CLI F; declared unsigned storage does not change the input sign.".into());
                self.push(Ty::F4, format!("__cil_conv_r4({})", value.name))?;
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
                let loaded = if ty.kind() == StackKind::Address {
                    expr
                } else if ty == Ty::R4 {
                    if op == "ldarg" {
                        format!("__cil_arg_r4({expr})")
                    } else {
                        expr
                    }
                } else {
                    ty.load(&expr, self.image)?
                };
                self.push(ty.stack(), loaded)?;
                if op == "ldarg" && idx == 0 && self.constructor_owner.is_some() {
                    self.original_this
                        .insert(self.stack.last().unwrap().name.clone());
                }
            }
            "ldarga" | "ldloca" => {
                let idx = var(i)?;
                if op == "ldarga" && idx == 0 && self.constructor_owner.is_some() {
                    return Err(
                        "constructor receiver slot address needs original-this provenance".into(),
                    );
                }
                let (ty, expr) = if op == "ldarga" {
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
                let address = self.image.address_of(ty)?;
                self.lines.push(format!(
                    "// Acquire the stable managed location of {}; later field access reads/writes that slot, not a copy.",
                    expr
                ));
                self.push(address, expr)?;
            }
            "ldtoken" => {
                let token = i
                    .operand
                    .token()
                    .ok_or("ldtoken needs a metadata token operand")?;
                // LdTypeToken/LdMemberToken: the operand decides the handle kind.
                let kind = self.image.token_handle_kind(token)?;
                let handle = self.image.handle_type(kind)?;
                self.lines.push(format!(
                    "// ldtoken {} 0x{:08X}: exact runtime handle for this metadata row; operand identity preserved, no reflection call.",
                    kind.label(),
                    token.0
                ));
                self.push(
                    handle,
                    format!("__cil_ldtoken_{}_{:08X}", kind.label(), token.0),
                )?;
            }
            "ldftn" => {
                let token = i
                    .operand
                    .token()
                    .ok_or("ldftn needs a method token operand")?;
                if !matches!(
                    token.table(),
                    Some(
                        cildec::TableId::MethodDef
                            | cildec::TableId::MemberRef
                            | cildec::TableId::MethodSpec
                    )
                ) {
                    return Err("ldftn operand is not a method token".into());
                }
                if self.image.token_handle_kind(token)? != crate::metadata::HandleKind::Method {
                    return Err("ldftn operand is not a method token".into());
                }
                // LdFtn: a native function pointer value. No receiver is popped;
                // delegate construction consumes it as the IntPtr argument.
                self.lines.push(format!(
                    "// ldftn 0x{:08X}: exact function pointer to the named method; no receiver change, no virtual dispatch.",
                    token.0
                ));
                self.push(Ty::Native, format!("__cil_ldftn_{:08X}", token.0))?;
            }
            "initobj" => {
                let address = self.pop()?;
                let token = i.operand.token().ok_or("initobj needs a storage token")?;
                self.image
                    .location_operand(address.ty, token, self.type_context)?;
                self.lines.push("// initobj: zero-initialize the acquired storage in place; no constructor or allocation.".into());
                self.lines.push(format!("{} = default;", address.name));
            }
            "ldobj" | "stobj" | "cpobj" => {
                let token = i
                    .operand
                    .token()
                    .ok_or("object memory operation needs a storage token")?;
                if op == "ldobj" {
                    let address = self.pop()?;
                    let element =
                        self.image
                            .location_operand(address.ty, token, self.type_context)?;
                    self.track_type(element)?;
                    self.push(element.stack(), element.load(&address.name, self.image)?)?;
                } else if op == "stobj" {
                    let value = self.pop()?;
                    let address = self.pop()?;
                    let element =
                        self.image
                            .location_operand(address.ty, token, self.type_context)?;
                    if value.ty.kind() == StackKind::Address {
                        return Err(
                            "stobj needs a value snapshot, not another managed location".into()
                        );
                    }
                    self.image.assignable(value.ty, element.stack())?;
                    self.lines.push(format!(
                        "{} = {};",
                        address.name,
                        element.store(&value.name, self.image)?
                    ));
                } else {
                    let source = self.pop()?;
                    let destination = self.pop()?;
                    let element =
                        self.image
                            .location_operand(source.ty, token, self.type_context)?;
                    if self
                        .image
                        .location_operand(destination.ty, token, self.type_context)?
                        != element
                    {
                        return Err("cpobj needs identical storage elements".into());
                    }
                    self.push(element.stack(), element.load(&source.name, self.image)?)?;
                    let copied = self.pop()?;
                    self.lines.push(format!(
                        "{} = {};",
                        destination.name,
                        element.store(&copied.name, self.image)?
                    ));
                }
            }
            "ldsflda" => {
                let token = i.operand.token().ok_or("missing static field token")?;
                let field = self.image.static_field(token, self.type_context)?;
                self.track_type(field.ty)?;
                self.push(self.image.address_of(field.ty)?, field.address())?;
            }
            "ldflda" => {
                let t = i.operand.token().ok_or("missing field token")?;
                let field =
                    self.image
                        .field(t, self.type_context, op != "ldsfld" && op != "stsfld")?;
                self.track_type(field.ty)?;
                if field.is_static {
                    return Err("instance address of a static field".into());
                }
                let address = self.image.address_of(field.ty)?;
                let receiver = self.pop()?;
                let target = match field.owner {
                    Ty::Value(id) => {
                        if receiver.ty != Ty::ValueAddress(id) {
                            return Err(
                                "struct field address needs the exact managed location of that value type"
                                    .into(),
                            );
                        }
                        format!("{}.{}", receiver.name, field.name)
                    }
                    owner => {
                        self.image.assignable(receiver.ty, owner)?;
                        format!("{}.{}", receiver.name, field.name)
                    }
                };
                // Initonly fields need an exact address operation, not a source-level writable borrow.
                let target = if field.readonly {
                    format!(
                        "__cil_ldflda_{:08X}({}{})",
                        t.0,
                        if receiver.ty.kind() == StackKind::Address {
                            "ref "
                        } else {
                            ""
                        },
                        receiver.name
                    )
                } else {
                    target
                };
                self.lines.push(format!(
                    "// field token 0x{:08X}: {}",
                    field.token,
                    crate::metadata::escape_line(&cildec::Names::new(&self.image.md).try_token(t)?)
                ));
                self.lines
                    .push("// Stable managed location of a field; no value is read here.".into());
                self.push(address, target)?;
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
                if op == "stloc"
                    && ty.kind() == StackKind::Address
                    && value.ty.kind() == StackKind::Address
                {
                    self.image.assignable(value.ty, ty)?;
                    self.lines.push(format!("{dest} = ref {};", value.name));
                    return Ok(());
                }
                if ty.kind() == StackKind::Address || value.ty.kind() == StackKind::Address {
                    return Err(
                        "managed argument/local slot reassignment needs lifetime analysis".into(),
                    );
                }
                self.image.assignable(value.ty, ty.stack())?;
                let stored = if ty == Ty::R4 {
                    if op == "starg" {
                        format!("__cil_set_arg_r4({})", value.name)
                    } else {
                        value.name
                    }
                } else {
                    ty.store(&value.name, self.image)?
                };
                self.lines.push(format!("{dest} = {stored};"));
            }
            "newobj" => {
                let token = i.operand.token().ok_or("missing constructor token")?;
                if let Some((array, rank, lengths)) =
                    self.image.array_allocation(token, self.type_context)?
                {
                    if rank < 2 {
                        return Err(
                            "general rank-1 array allocation is not a vector; C# has no equivalent"
                                .into(),
                        );
                    }
                    self.track_type(array)?;
                    let element = crate::arrays::general_shape(self.image, array)?.0;
                    let element_name = element.cs(self.image)?;
                    let mut values = Vec::with_capacity(lengths.len());
                    for ty in lengths.iter().rev() {
                        let value = self.pop()?;
                        self.image.assignable(value.ty, ty.stack())?;
                        values.push(value.name);
                    }
                    values.reverse();
                    self.lines.push("// Array allocation with one length per dimension; the runtime zero-initialises every element.".into());
                    self.push(array, format!("new {element_name}[{}]", values.join(", ")))?;
                    return Ok(());
                }
                if let Some((value, parameters)) =
                    self.image.value_constructor(token, self.type_context)?
                {
                    self.track_type(value)?;
                    let mut values = Vec::new();
                    for parameter in parameters.iter().rev() {
                        self.track_type(*parameter)?;
                        let argument = self.pop()?;
                        self.image.assignable(argument.ty, parameter.stack())?;
                        values.push(if parameter.kind() == StackKind::Address {
                            format!("ref {}", argument.name)
                        } else {
                            parameter.store(&argument.name, self.image)?
                        });
                    }
                    values.reverse();
                    self.lines.push("// Value newobj: initialize separate value storage, call the exact constructor, then copy the completed value; no reference-class identity is allocated.".into());
                    self.push(
                        value,
                        format!("__cil_value_newobj_{:08X}({})", token.0, values.join(", ")),
                    )?;
                    return Ok(());
                }
                {
                    let (allocated, args) =
                        match self.image.exception_constructor(token, self.type_context)? {
                            Some(constructor) => constructor,
                            None => self
                                .image
                                .allocation_constructor(token, self.type_context)?,
                        };
                    for ty in std::iter::once(&allocated).chain(args.iter()) {
                        self.track_type(*ty)?;
                    }
                    let mut values = Vec::new();
                    for ty in args.iter().rev() {
                        let value = self.pop()?;
                        if value.ty.kind() == StackKind::Address && ty.kind() != StackKind::Address
                        {
                            return Err("constructor argument needs a value snapshot, not a managed location".into());
                        }
                        self.image.assignable(value.ty, ty.stack())?;
                        values.push(if ty.kind() == StackKind::Address {
                            format!("ref {}", value.name)
                        } else {
                            value.name
                        });
                    }
                    values.reverse();
                    self.lines.push(format!(
                        "// newobj token 0x{:08X}: {}",
                        token.0,
                        crate::metadata::escape_line(
                            &cildec::Names::new(&self.image.md).try_token(token)?
                        )
                    ));
                    if let Some(Ok(fact)) = self.image.external_allocation(token.0) {
                        self.lines.push(format!(
                            "// external allocation owner 0x{:08X}: {}{}; {}, {}; one fresh identity here, then .ctor with {} parameter(s) runs in original argument order; field initialisers are the defining image's, not visible here.",
                            fact.owner_token,
                            fact.owner_name,
                            if fact.constructed { "<>" } else { "" },
                            if fact.constructed { "constructed class" } else { "class" },
                            if fact.owner_sealed { "sealed" } else { "not sealed" },
                            fact.parameter_count
                        ));
                    }
                    if allocated.kind() == StackKind::Ref {
                        self.lines.push("// __cil_newobj: one fresh object identity allocated here, then the exact constructor runs in original argument order; later reads observe the same instance.".into());
                    } else {
                        self.lines.push("// __cil_newobj: zero-initialize separate value storage here, run the exact constructor in original argument order, then copy the completed value; no object identity or shared storage is created.".into());
                    }
                    self.push(
                        allocated,
                        format!("__cil_newobj_{:08X}({})", token.0, values.join(", ")),
                    )?;
                    return Ok(());
                }
            }
            "newarr" => {
                let token = i.operand.token().ok_or("missing element type token")?;
                let array = self.image.array_allocation_type(token, self.type_context)?;
                let length = self.pop()?;
                expect(length.ty, Ty::I4)?;
                self.lines.push(format!(
                    "// element type token 0x{:08X}: {}",
                    token.0,
                    crate::metadata::escape_line(
                        &cildec::Names::new(&self.image.md).try_token(token)?
                    )
                ));
                self.lines.push("// Allocate a fresh zero-initialized array here; runtime length/allocation failures remain at this instruction.".into());
                self.track_type(array)?;
                let expression = match array {
                    Ty::I4Array => format!("new int[{}]", length.name),
                    Ty::U4Array => format!("new uint[{}]", length.name),
                    _ => format!("__cil_newarr_{:08X}({})", token.0, length.name),
                };
                self.push(array, expression)?;
            }
            "ldlen" => {
                let array = self.pop()?;
                crate::arrays::reference(self.image, array.ty)?;
                // Read now: null checking and length evaluation must not be delayed.
                // The signed native temporary retains the native-unsigned bit pattern.
                self.push(
                    Ty::Native,
                    format!("unchecked((nint)({}.LongLength))", array.name),
                )?;
            }
            "ldelema" => {
                let token = i.operand.token().ok_or("missing element type token")?;
                let index = self.pop()?;
                expect(index.ty, Ty::I4)?;
                let array = self.pop()?;
                let legacy = matches!(array.ty, Ty::I4Array | Ty::U4Array);
                let element = if legacy {
                    let element = self.image.array_element_type(token)?;
                    if array_element(array.ty)? != element {
                        return Err("ldelema token does not match the array element type".into());
                    }
                    element
                } else {
                    crate::arrays::operand(self.image, array.ty, token, self.type_context)?
                };
                self.lines.push(format!(
                    "// element type token 0x{:08X}: {}",
                    token.0,
                    crate::metadata::escape_line(
                        &cildec::Names::new(&self.image.md).try_token(token)?
                    )
                ));
                self.lines.push("// Stable managed element address; null/bounds checks occur here, without reading its value.".into());
                let address = self.image.address_of(element)?;
                let expression = if legacy {
                    format!("{}[{}]", array.name, index.name)
                } else {
                    format!(
                        "__cil_ldelema_{:08X}({}, {})",
                        token.0, array.name, index.name
                    )
                };
                self.push(address, expression)?;
            }
            "ldind.i1" | "ldind.u1" | "ldind.i2" | "ldind.u2" | "ldind.i4" | "ldind.u4"
            | "ldind.i8" | "ldind.u8" | "ldind.i" | "ldind.r4" | "ldind.ref" => {
                let address = self.pop()?;
                let pointer = matches!(address.ty.kind(), StackKind::Pointer);
                let element = if pointer {
                    self.image.pointee(address.ty)?
                } else {
                    self.image.address_element(address.ty)?
                };
                let access = if pointer {
                    format!("(*{})", address.name)
                } else {
                    address.name.clone()
                };
                let storage = match element {
                    Ty::Enum(e) => e.storage.ty(),
                    other => other,
                };
                let requested = match op {
                    "ldind.i1" if matches!(storage, Ty::I1 | Ty::U1) => Ty::I1,
                    "ldind.u1" if matches!(storage, Ty::I1 | Ty::U1 | Ty::Bool) => Ty::U1,
                    "ldind.i2" if matches!(storage, Ty::I2 | Ty::U2 | Ty::Char) => Ty::I2,
                    "ldind.u2" if matches!(storage, Ty::I2 | Ty::U2 | Ty::Char) => Ty::U2,
                    "ldind.i4" | "ldind.u4" if matches!(storage, Ty::I4 | Ty::U4) => Ty::I4,
                    "ldind.i8" | "ldind.u8" if matches!(storage, Ty::I8 | Ty::U8) => Ty::I8,
                    "ldind.i" if matches!(storage, Ty::Native | Ty::UNative) => Ty::Native,
                    "ldind.r4" if storage == Ty::R4 => Ty::R4,
                    "ldind.r8" if storage == Ty::R8 => Ty::R8,
                    "ldind.ref" if element.kind() == StackKind::Ref => element,
                    _ => {
                        return Err(
                            "indirect load width/kind differs from its storage element".into()
                        );
                    }
                };
                let expression = if element == Ty::Bool {
                    element.load(&access, self.image)?
                } else if op == "ldind.ref" || requested == Ty::R4 {
                    requested.load(&access, self.image)?
                } else {
                    requested.load(
                        &format!("unchecked(({})({}))", requested.cs(self.image)?, access),
                        self.image,
                    )?
                };
                self.push(requested.stack(), expression)?;
            }
            "stind.i1" | "stind.i2" | "stind.i4" | "stind.i8" | "stind.i" | "stind.r4"
            | "stind.r8" | "stind.ref" => {
                let value = self.pop()?;
                let address = self.pop()?;
                let pointer = matches!(address.ty.kind(), StackKind::Pointer);
                let element = if pointer {
                    self.image.pointee(address.ty)?
                } else {
                    self.image.address_element(address.ty)?
                };
                let access = if pointer {
                    format!("(*{})", address.name)
                } else {
                    address.name.clone()
                };
                let storage = match element {
                    Ty::Enum(e) => e.storage.ty(),
                    other => other,
                };
                let valid = match op {
                    "stind.i1" => matches!(storage, Ty::I1 | Ty::U1 | Ty::Bool),
                    "stind.i2" => matches!(storage, Ty::I2 | Ty::U2 | Ty::Char),
                    "stind.i4" => matches!(storage, Ty::I4 | Ty::U4),
                    "stind.i8" => matches!(storage, Ty::I8 | Ty::U8),
                    "stind.i" => matches!(storage, Ty::Native | Ty::UNative),
                    "stind.r4" => storage == Ty::R4,
                    "stind.r8" => storage == Ty::R8,
                    "stind.ref" => element.kind() == StackKind::Ref,
                    _ => false,
                };
                if !valid || value.ty.kind() == StackKind::Address {
                    return Err("indirect store width/kind differs from its storage element".into());
                }
                self.image.assignable(value.ty, element.stack())?;
                self.lines.push(format!(
                    "{} = {};",
                    access,
                    element.store(&value.name, self.image)?
                ));
            }
            "ldelem.i1" | "ldelem.u1" | "ldelem.i2" | "ldelem.u2" | "ldelem.i8" | "ldelem.r4"
            | "stelem.i1" | "stelem.i2" | "stelem.i8" | "stelem.r4" => {
                let value = if op.starts_with("stelem") {
                    Some(self.pop()?)
                } else {
                    None
                };
                let index = self.pop()?;
                expect(index.ty, Ty::I4)?;
                let array = self.pop()?;
                let (element, requested) = crate::arrays::scalar(self.image, array.ty, op)?;
                self.track_type(element)?;
                let target = format!("{}[{}]", array.name, index.name);
                if let Some(value) = value {
                    self.image.assignable(value.ty, requested.stack())?;
                    self.lines.push(format!(
                        "{target} = {};",
                        element.store(&value.name, self.image)?
                    ));
                } else {
                    let loaded = element.load(&target, self.image)?;
                    let expression = if requested == Ty::R4 {
                        loaded
                    } else {
                        requested.load(
                            &format!("unchecked(({})({loaded}))", requested.cs(self.image)?),
                            self.image,
                        )?
                    };
                    self.push(requested.stack(), expression)?;
                }
            }
            "ldelem" | "stelem" => {
                let token = i.operand.token().ok_or("missing element token")?;
                let value = if op == "stelem" {
                    Some(self.pop()?)
                } else {
                    None
                };
                let index = self.pop()?;
                expect(index.ty, Ty::I4)?;
                let array = self.pop()?;
                let element =
                    crate::arrays::operand(self.image, array.ty, token, self.type_context)?;
                self.track_type(element)?;
                if let Some(value) = value {
                    self.image.assignable(value.ty, element.stack())?;
                    self.lines.push(format!(
                        "__cil_stelem_{:08X}({}, {}, {});",
                        token.0,
                        array.name,
                        index.name,
                        element.store(&value.name, self.image)?
                    ));
                } else {
                    self.push(
                        element.stack(),
                        element.load(
                            &format!(
                                "__cil_ldelem_{:08X}({}, {})",
                                token.0, array.name, index.name
                            ),
                            self.image,
                        )?,
                    )?;
                }
            }
            "stelem.ref" => {
                let value = self.pop()?;
                let index = self.pop()?;
                expect(index.ty, Ty::I4)?;
                let array = self.pop()?;
                let element = crate::arrays::element(self.image, array.ty)?;
                if element.kind() != StackKind::Ref {
                    return Err("reference element store needs a reference vector element".into());
                }
                self.image.assignable(value.ty, element)?;
                self.lines.push(format!(
                    "{}[{}] = {};",
                    array.name,
                    index.name,
                    element.store(&value.name, self.image)?
                ));
            }
            "ldelem.ref" => {
                let index = self.pop()?;
                expect(index.ty, Ty::I4)?;
                let array = self.pop()?;
                let element = crate::arrays::element(self.image, array.ty)?;
                if element.kind() != StackKind::Ref {
                    return Err("reference element load needs a reference vector element".into());
                }
                self.push(element, format!("{}[{}]", array.name, index.name))?;
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
            "castclass" | "isinst" => {
                let token = i.operand.token().ok_or("missing cast type token")?;
                // A value-type target is a boxed-value test: the runtime checks the
                // box's type and the result is still the object reference (the same
                // box on success, null for a failed isinst). The declared value type
                // is used only inside the C# type test.
                let boxed = self
                    .image
                    .boxed_value_type(token, self.type_context)
                    .ok()
                    .filter(|ty| matches!(ty, Ty::Value(_) | Ty::Enum(_)));
                let target = if boxed.is_some() {
                    Ty::Object
                } else {
                    self.image.cast_class(token, self.type_context)?
                };
                let value = self.pop()?;
                if value.ty.kind() != StackKind::Ref && value.ty != Ty::Null {
                    return Err("cast needs an admitted reference operand".into());
                }
                self.lines.push(format!(
                    "// {} token 0x{:08X}: {}",
                    op,
                    token.0,
                    crate::metadata::escape_line(
                        &cildec::Names::new(&self.image.md).try_token(token)?
                    )
                ));
                let name = target.cs(self.image)?;
                // Both forms are exact: a C# reference cast performs castclass
                // (null passes through, incompatibility throws InvalidCast), and
                // the as-expression performs isinst (null on failure, no throw).
                let expression = if let Some(declared) = boxed {
                    let declared = declared.cs(self.image)?;
                    self.lines.push("// Boxed-value test: the runtime checks the box's exact type; the result stays an object reference (the same box on success, null for a failed isinst).".into());
                    if op == "isinst" {
                        format!(
                            "((object)({} is {declared} ? {} : null))",
                            value.name, value.name
                        )
                    } else {
                        format!(
                            "((object)({} is {declared} ? {} : throw new global::System.InvalidCastException()))",
                            value.name, value.name
                        )
                    }
                } else if op == "isinst" {
                    self.lines.push("// isinst: runtime type test yielding null on failure; no exception and no user code.".into());
                    format!("((object){} as {})", value.name, name)
                } else {
                    self.lines.push("// castclass: runtime type check yielding the same reference or throwing InvalidCastException; null passes through. The object hop is an identity conversion that keeps the check legal for statically unrelated reference types.".into());
                    format!("(({})(object){})", name, value.name)
                };
                self.push(target.stack(), expression)?;
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
                let pointer = |ty: Ty| matches!(ty.kind(), StackKind::Pointer);
                if matches!(op, "add" | "sub") && (pointer(a.ty) || pointer(b.ty)) {
                    // C# pointer arithmetic: pointer +/- integer scales by the
                    // pointee, pointer - pointer yields the element delta. Both
                    // are exactly the CLI operations on unmanaged pointers.
                    let offset = |ty: Ty| {
                        if matches!(ty, Ty::I4 | Ty::Native) {
                            Ok(())
                        } else {
                            Err("pointer offset needs a 32-bit or native integer".to_string())
                        }
                    };
                    if op == "add" {
                        if pointer(a.ty) && pointer(b.ty) {
                            return Err("pointer addition needs one integer offset".into());
                        }
                        // IL adds a byte offset; C# `T* + n` scales by the
                        // pointee, so the addition happens on byte pointers and
                        // the result is cast back to the declared pointer type.
                        let pointee =
                            self.image
                                .pointee(if pointer(a.ty) { a.ty } else { b.ty })?;
                        let pointer_type = self.image.pointer(pointee)?.cs(self.image)?;
                        if pointer(a.ty) {
                            offset(b.ty)?;
                            self.push(
                                a.ty,
                                format!("(({pointer_type})((byte*)({}) + ({})))", a.name, b.name),
                            )?;
                        } else {
                            offset(a.ty)?;
                            self.push(
                                b.ty,
                                format!("(({pointer_type})((byte*)({}) + ({})))", b.name, a.name),
                            )?;
                        }
                        return Ok(());
                    }
                    if !pointer(a.ty) {
                        return Err("integer minus pointer needs IL fallback".into());
                    }
                    if pointer(b.ty) {
                        if a.ty != b.ty {
                            return Err("pointer difference needs identical pointee types".into());
                        }
                        // IL subtracts byte addresses; C# pointer subtraction
                        // divides by the pointee size, so byte pointers keep the
                        // exact count.
                        self.push(
                            Ty::Native,
                            format!(
                                "unchecked((nint)((byte*)({}) - (byte*)({})))",
                                a.name, b.name
                            ),
                        )?;
                    } else {
                        offset(b.ty)?;
                        let pointer_type = a.ty.cs(self.image)?;
                        self.push(
                            a.ty,
                            format!("(({pointer_type})((byte*)({}) - ({})))", a.name, b.name),
                        )?;
                    }
                    return Ok(());
                }
                // The CLI widens a narrower integer operand to the wider one and
                // computes in that width; C# has no implicit mixed-width arithmetic
                // for these types, so the widening is written out explicitly.
                let (a, b) = widen_integers(self.image, a, b)?;
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
                if op == "div" && a.ty == Ty::F4 && b.ty == Ty::F4 {
                    self.lines.push("// Floating division in target CLI F precision; IEEE special values, no integer divide checks or extra storage rounding.".into());
                    self.push(Ty::F4, format!("__cil_div_F({}, {})", a.name, b.name))?;
                    return Ok(());
                }
                if op.ends_with(".un")
                    && (matches!(a.ty.kind(), StackKind::Float)
                        || matches!(b.ty.kind(), StackKind::Float))
                {
                    return Err("float division has no unsigned form in the CLI".into());
                }
                if op == "div" && a.ty == Ty::R8 && b.ty == Ty::R8 {
                    self.lines.push("// Binary64 division in target CLI F; IEEE special values, no integer divide checks or extra storage rounding.".into());
                    self.push(Ty::R8, format!("__cil_div_F8({}, {})", a.name, b.name))?;
                    return Ok(());
                }
                let (a, b) = widen_integers(self.image, a, b)?;
                expect(a.ty, b.ty)?;
                let symbol = if op.starts_with("div") { "/" } else { "%" };
                let (x, y) = if op.ends_with(".un") {
                    let ty = if a.ty == Ty::I8 {
                        "ulong"
                    } else if matches!(a.ty, Ty::Native | Ty::UNative) {
                        "nuint"
                    } else {
                        "uint"
                    };
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
                if matches!(a.ty.kind(), StackKind::Float) {
                    if op == "not" {
                        return Err("bitwise not on a float needs IL fallback".into());
                    }
                } else {
                    integer(a.ty)?;
                }
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
                let cmp = comparison(op, &a, &b, self.image)?;
                self.push(Ty::I4, format!("({cmp} ? 1 : 0)"))?;
            }
            "br" => {}
            "brtrue" | "brfalse" => {
                let a = self.pop()?;
                if a.ty.kind() == StackKind::Ref {
                    self.lines.push(format!(
                        "{} {}",
                        a.name,
                        if op == "brtrue" {
                            "is not null"
                        } else {
                            "is null"
                        }
                    ));
                } else {
                    integer(a.ty)?;
                    self.lines.push(format!(
                        "{} {} 0",
                        a.name,
                        if op == "brtrue" { "!=" } else { "==" }
                    ));
                }
            }
            "beq" | "bne.un" | "bge" | "bge.un" | "bgt" | "bgt.un" | "ble" | "ble.un" | "blt"
            | "blt.un" => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.lines.push(comparison(op, &a, &b, self.image)?);
            }
            "switch" => {
                let a = self.pop()?;
                expect(a.ty, Ty::I4)?;
                self.lines.push(a.name);
            }
            "call" | "callvirt" => {
                let t = i.operand.token().ok_or("missing call token")?;
                if self.tail_calls.contains(&i.offset) {
                    self.lines.push("// tail. prefix: the CLI tail-call request has no C# equivalent, so this call is emitted normally; arguments, result, side effects and exceptions are unchanged.".into());
                }
                if let Some(intrinsic) = self.image.array_access(t, self.type_context)? {
                    return self.array_intrinsic(intrinsic);
                }
                let mut method = self.image.method_ref(t, self.type_context)?;
                let virtual_call = op == "callvirt";
                if let Some(&constraint) = self.constrained_calls.get(&i.offset) {
                    return self.constrained_call(t, constraint, &method);
                }
                let mut dispatch = false;
                if virtual_call {
                    if method.generic_virtual {
                        return Err(
                            "generic virtual dispatch needs runtime lookup-order modeling".into(),
                        );
                    }
                    // A static intrinsic has no receiver to check.
                    if method.tick_count {
                        return Err("intrinsic callvirt needs IL fallback".into());
                    }
                    // Constructors are never virtual. Some compilers emit callvirt on
                    // a reference constructor; the runtime performs the same non-virtual
                    // receiver call, so the constructor checks and target are kept and
                    // nothing dispatches. A value-type constructor expects the managed
                    // cell, and callvirt would treat that cell as an object reference,
                    // so it stays refused.
                    if method.constructor || method.core_object_constructor.is_some() {
                        let reference_receiver = match method.receiver {
                            Some(owner) => owner.kind() == StackKind::Ref,
                            None => method.core_object_constructor.is_some(),
                        };
                        if !reference_receiver {
                            return Err("constructor/intrinsic callvirt needs IL fallback".into());
                        }
                    } else {
                        let owner = method
                            .receiver
                            .ok_or("callvirt needs an instance receiver")?;
                        if owner.kind() != StackKind::Ref {
                            return Err(
                                "callvirt on a value-type receiver needs boxing/constrained dispatch"
                                    .into(),
                            );
                        }
                        // Exact reference dispatch needs the original token and checked
                        // stack signature, not a guessed external implementation. Source
                        // property/member syntax has a separate dependency-backed proof.
                        if !matches!(
                            owner,
                            Ty::Class(_) | Ty::StructuralRef(_) | Ty::ExternalClass(_)
                        ) {
                            return Err("callvirt needs a nominal reference receiver".into());
                        }
                        method.target = format!("__cil_callvirt_{:08X}", t.0);
                        dispatch = true;
                    }
                }
                let dispatch_note = if dispatch {
                    "callvirt receiver null check then target-runtime dispatch to the declared method"
                } else {
                    "exact IL call; no virtual dispatch or added null check"
                };
                for ty in method.args.iter().chain(std::iter::once(&method.ret)) {
                    self.track_type(*ty)?;
                }
                let mut values = Vec::new();
                for &ty in method.args.iter().rev() {
                    let v = self.pop()?;
                    if v.ty.kind() == StackKind::Address && ty.kind() != StackKind::Address {
                        return Err(
                            "call argument needs a value snapshot, not a managed location".into(),
                        );
                    }
                    if let Err(error) = self.image.assignable(v.ty, ty.stack()) {
                        // Two reference types whose assignability proof is missing:
                        // the explicit cast emitted below performs the runtime check
                        // the IL verifier already guarantees for this argument, and
                        // keeps reference identity. Any other category keeps the
                        // refusal.
                        if v.ty.kind() != StackKind::Ref || ty.kind() != StackKind::Ref {
                            return Err(error);
                        }
                        self.lines.push("// Reference argument: explicit cast preserves identity; on verifier-valid IL the runtime value already fits the parameter.".into());
                    }
                    values.push(if ty.kind() == StackKind::Address {
                        format!("ref {}", v.name)
                    } else if ty.kind() == StackKind::Ref && v.ty != ty {
                        let formal = if let Ty::Class(token) = ty {
                            crate::symbol_names::type_name(self.image, cildec::Token(token), true)?
                        } else {
                            ty.cs(self.image)?
                        };
                        format!("(({formal})({}))", v.name)
                    } else if ty == Ty::R4 {
                        format!("__cil_call_arg_r4({})", v.name)
                    } else {
                        ty.store(&v.name, self.image)?
                    });
                }
                values.reverse();
                let mut property_expression = None;
                if let Some(core_owner) = method.core_object_constructor {
                    let owner = self
                        .constructor_owner
                        .ok_or("core constructor call outside constructor context")?;
                    let receiver = self.pop()?;
                    if receiver.ty != owner || !self.original_this.contains(&receiver.name) {
                        return Err("core constructor call needs unchanged original this".into());
                    }
                    let Ty::Class(owner) = owner else {
                        return Err("core constructor call needs a reference class".into());
                    };
                    if self.image.md.tables().type_def(owner & 0xffffff)?.extends.0 != core_owner {
                        return Err("core constructor call is not the immediate base".into());
                    }
                    values.insert(0, receiver.name);
                    self.lines.push("// Core Object constructor initialization: exact IL call, no allocation, no virtual dispatch, no added null check.".into());
                } else if let Some(owner) = method.receiver {
                    let receiver = self.pop()?;
                    if owner.kind() != StackKind::Ref {
                        let element = if owner.kind() == StackKind::Address {
                            self.image.address_element(owner)?
                        } else {
                            owner
                        };
                        if receiver.ty.kind() != StackKind::Address
                            || self.image.address_element(receiver.ty)? != element
                        {
                            return Err(
                                "struct instance call needs the exact managed location of that value type"
                                    .into(),
                            );
                        }
                    } else {
                        self.image.assignable(receiver.ty, owner)?;
                    }
                    if method.constructor && owner.kind() != StackKind::Ref {
                        self.lines.push("// Exact value constructor call on the acquired managed cell; no allocation, synthetic zeroing, or receiver copy.".into());
                    } else if method.constructor {
                        let current = self
                            .constructor_owner
                            .ok_or("constructor call outside constructor context")?;
                        if !receiver_matches(receiver.ty, current)
                            || !self.original_this.contains(&receiver.name)
                        {
                            return Err(
                                "constructor call needs same owning constructor and original this"
                                    .into(),
                            );
                        }
                        if current == owner {
                            self.lines.push("// Exact same-type constructor delegation on the same original receiver; no allocation or added null check.".into());
                        } else {
                            if !matches!(current, Ty::Class(_) | Ty::StructuralRef(_)) {
                                return Err("constructor call needs a reference-class owner".into());
                            }
                            if !self.image.immediate_constructor_base(current, owner)? {
                                return Err("constructor call is not the immediate base".into());
                            }
                            self.lines.push("// Initialize the existing original receiver via an exact direct-base constructor; no allocation, no virtual dispatch, no added null check.".into());
                        }
                    } else if virtual_call {
                        self.lines.push("// __cil_callvirt: receiver null check first, then target-runtime virtual/interface dispatch to the declared method; the receiver and arguments are evaluated once, in order.".into());
                    } else {
                        self.lines.push("// __cil_direct: exact IL call target; no virtual dispatch or added null check.".into());
                    }
                    // An external instance method is written on the receiver, so the
                    // receiver is the call's object rather than its first argument.
                    let mut explicit_receiver = true;
                    // A constructed local class reaches this arm with its definition
                    // already verified, so a virtual call can be written on the
                    // receiver: C# emits callvirt, keeping the null check, dispatch
                    // and evaluation order of the original instruction.
                    if virtual_call
                        && matches!(owner, Ty::StructuralRef(_))
                        && !method.exact_reference
                        && t.table() != Some(cildec::TableId::MethodSpec)
                    {
                        method.target = format!("{}.@{}", receiver.name, method.name);
                        self.lines.push(
                            "// constructed dispatch: the call is written on the receiver, so the receiver null check, target-runtime dispatch and evaluation order are the ones the original call site had."
                                .into(),
                        );
                        explicit_receiver = false;
                    }
                    if matches!(owner, Ty::ExternalClass(_))
                        && let Some(Ok(call)) = self.image.external_call(t.0)
                    {
                        if virtual_call && let Some(property) = &call.property {
                            let access = format!(
                                "(({}){}).{}",
                                property.owner_name, receiver.name, property.name
                            );
                            property_expression =
                                Some(match (property.getter, values.as_slice()) {
                                    (true, []) => access,
                                    (false, [value]) if method.ret == Ty::Void => {
                                        format!("{access} = {value}")
                                    }
                                    _ => return Err(
                                        "external property call shape differs from its association"
                                            .into(),
                                    ),
                                });
                            self.lines.push(format!(
                                "// property callvirt 0x{:08X}: declared owner {}; exact metadata association; arguments evaluated before receiver null check and runtime dispatch.",
                                t.0, property.owner_name
                            ));
                            explicit_receiver = false;
                        } else if call.property.is_none() {
                            method.target = format!("{}.@{}", receiver.name, call.name);
                            self.lines.push(format!(
                                "// external dispatch 0x{:08X}: instance method declared by {} (virtual={}, abstract={}); the call is written on the receiver, so the receiver null check, target-runtime dispatch, evaluation order and argument copies are the ones the original call site had.",
                                t.0,
                                crate::metadata::escape_line(&call.declaring_type),
                                call.is_virtual,
                                call.is_abstract
                            ));
                            explicit_receiver = false;
                        }
                        // A direct accessor call keeps its exact-call intrinsic:
                        // property syntax would add a null check and virtual dispatch.
                    }
                    // A value-type instance call passes a managed pointer, so the
                    // receiver argument is explicit here rather than an implicit copy.
                    if explicit_receiver {
                        values.insert(
                            0,
                            if owner.kind() != StackKind::Ref {
                                format!("ref {}", receiver.name)
                            } else {
                                receiver.name
                            },
                        );
                    }
                }
                if method.target.starts_with("__cil_static_call_") {
                    self.lines.push("// Exact constructed static call: original token and instantiated declaring type, original generic type initialization and target-runtime effects; arguments evaluated once in order, no receiver or virtual dispatch.".into());
                }
                if method.tick_count {
                    self.lines.push("// Exact core Environment.TickCount: signed I4 clock read at this instruction, evaluated once.".into());
                }
                if let (Some(Ty::ExternalClass(owner)), Some(cildec::TableId::MemberRef)) =
                    (method.receiver, t.table())
                {
                    let reference = self.image.md.tables().member_ref(t.rid())?;
                    self.lines.push(format!("// memberref target 0x{:08X}: owner={} method-name={} signature-bytes={:02X?}; {dispatch_note}; resolution/access/body effects belong to the original target runtime.",
                        t.0, self.image.external_class_context(owner)?, crate::metadata::escape_line(self.image.md.strings().str(reference.name)?), self.image.md.blobs().get(reference.signature)?));
                }
                let expression = property_expression
                    .unwrap_or_else(|| format!("{}({})", method.target, values.join(", ")));
                self.lines.push(format!(
                    "// call token 0x{:08X}: {}",
                    method.token,
                    crate::metadata::escape_line(&cildec::Names::new(&self.image.md).try_token(t)?)
                ));
                if method.ret == Ty::Void {
                    self.lines.push(format!("{expression};"));
                } else if method.ret.kind() == StackKind::Address {
                    // A byref result is bound as a ref local: reading or writing
                    // through the alias keeps the callee's referent and the same
                    // dereference point as the original pointer.
                    self.lines.push("// byref call result: bind the returned managed reference as a ref local; later reads/writes alias the callee's referent.".into());
                    self.push(method.ret, expression)?;
                } else {
                    let result = if method.ret == Ty::R4 {
                        format!("__cil_call_result_r4({expression})")
                    } else {
                        method.ret.load(&expression, self.image)?
                    };
                    self.push(method.ret.stack(), result)?;
                }
            }
            "ldfld" | "stfld" => {
                let t = i.operand.token().ok_or("missing field token")?;
                let field =
                    self.image
                        .field(t, self.type_context, op != "ldsfld" && op != "stsfld")?;
                self.track_type(field.ty)?;
                if field.is_static {
                    return Err("instance access to static field".into());
                }
                let value = if op == "stfld" {
                    Some(self.pop()?)
                } else {
                    None
                };
                let receiver = self.pop()?;
                // Keep base casts: a derived type can hide the named field.
                let target = match field.owner {
                    Ty::Value(id) => {
                        // A managed location of this exact value type is writable;
                        // a copied value on the stack would only mutate a temporary.
                        let managed = receiver.ty == Ty::ValueAddress(id);
                        if !managed && receiver.ty != Ty::Value(id) {
                            return Err(
                                "struct field access needs the exact managed location of that value type"
                                    .into(),
                            );
                        }
                        if op == "stfld" && !managed {
                            return Err(
                                "struct field write needs a managed location, not a copied value"
                                    .into(),
                            );
                        }
                        format!("{}.{}", receiver.name, field.name)
                    }
                    owner => {
                        self.image.assignable(receiver.ty, owner)?;
                        if receiver.ty == owner
                            || matches!((receiver.ty, owner), (Ty::StructuralRef(id), Ty::Class(definition))
                            if self.image.open_self_definition(id) == Some(definition))
                        {
                            format!("{}.{}", receiver.name, field.name)
                        } else {
                            // A cast keeps a hidden base field explicit, but a generic
                            // base would have to be named with its instantiation
                            // (`Holder<T>`); the definition name alone is an open
                            // generic and erases the argument. Until the constructed
                            // owner can be named here, this access stays gated.
                            if self.image.generic_definition_name(owner)? {
                                return Err(
                                    "base field access needs the constructed owner name".into()
                                );
                            }
                            format!(
                                "(({}){}).{}",
                                owner.cs(self.image)?,
                                receiver.name,
                                field.name
                            )
                        }
                    }
                };
                self.lines.push(format!(
                    "// field token 0x{:08X}: {}",
                    field.token,
                    crate::metadata::escape_line(&cildec::Names::new(&self.image.md).try_token(t)?)
                ));
                let volatile = self.volatile_accesses.contains(&i.offset);
                if volatile && !volatile_supported(field.ty) {
                    return Err(
                        "volatile access needs a scalar or reference field with a Volatile overload"
                            .into(),
                    );
                }
                if let Some(v) = value {
                    if field.readonly
                        && !self.analyzing
                        && (self.constructor_owner != Some(field.owner)
                            || !self.original_this.contains(&receiver.name))
                    {
                        return Err("readonly field write needs IL fallback".into());
                    }
                    self.image.assignable(v.ty, field.ty.stack())?;
                    if volatile {
                        self.lines.push(format!(
                            "global::System.Threading.Volatile.Write(ref {target}, {});",
                            field.ty.store(&v.name, self.image)?
                        ));
                    } else {
                        self.lines.push(format!(
                            "{target} = {};",
                            field.ty.store(&v.name, self.image)?
                        ));
                    }
                } else if volatile {
                    self.lines
                        .push("// Volatile field read: acquire semantics, no plain load.".into());
                    self.push(
                        field.ty.stack(),
                        format!("global::System.Threading.Volatile.Read(ref {target})"),
                    )?;
                } else {
                    self.push(field.ty.stack(), field.ty.load(&target, self.image)?)?;
                }
            }
            "ldsfld" | "stsfld" => {
                let t = i.operand.token().ok_or("missing field token")?;
                let field = self.image.static_field(t, self.type_context)?;
                let ty = field.ty;
                self.track_type(ty)?;
                self.lines.push(format!("// field token 0x{:08X}", t.0));
                let volatile = self.volatile_accesses.contains(&i.offset);
                if volatile && !volatile_supported(ty) {
                    return Err(
                        "volatile access needs a scalar or reference field with a Volatile overload"
                            .into(),
                    );
                }
                if op == "ldsfld" && volatile {
                    self.lines
                        .push("// Volatile static read: acquire semantics, no plain load.".into());
                    self.push(
                        ty.stack(),
                        format!(
                            "global::System.Threading.Volatile.Read(ref {})",
                            field.address()
                        ),
                    )?;
                } else if op == "ldsfld" {
                    self.push(ty.stack(), field.load(self.image)?)?;
                } else if volatile {
                    if !field.writable_in(self.static_constructor) {
                        return Err("readonly static field write needs IL fallback".into());
                    }
                    let v = self.pop()?;
                    self.image.assignable(v.ty, ty.stack())?;
                    self.lines.push(format!(
                        "global::System.Threading.Volatile.Write(ref {}, {});",
                        field.address(),
                        ty.store(&v.name, self.image)?
                    ));
                } else {
                    // A `static readonly` field is assignable only by the static
                    // constructor of its own declaring type; every other write would
                    // not be legal C# for the same storage.
                    if !field.writable_in(self.static_constructor) {
                        return Err("readonly static field write needs IL fallback".into());
                    }
                    let v = self.pop()?;
                    self.image.assignable(v.ty, ty.stack())?;
                    self.lines.push(field.store(&v.name, self.image)?);
                }
            }
            "throw" => {
                let value = self.pop()?;
                if value.ty != Ty::Null {
                    self.image.exception_reference(value.ty)?;
                }
                self.lines.push(format!("throw {};", value.name));
                // CLI throw empties the evaluation stack, including values below the object.
                self.stack.clear();
            }
            "ret" => {
                if self.ret == Ty::Void {
                    self.lines.push("return;".into());
                } else {
                    let value = self.pop()?;
                    if value.ty.kind() == StackKind::Address {
                        return Err(
                            "ordinary return needs a value snapshot, not a managed location".into(),
                        );
                    }
                    self.image.assignable(value.ty, self.ret.stack())?;
                    let returned = if self.ret == Ty::R4 {
                        format!("__cil_return_r4({})", value.name)
                    } else {
                        self.ret.store(&value.name, self.image)?
                    };
                    self.lines.push(format!("return {returned};"));
                }
                if !self.stack.is_empty() {
                    return Err("return leaves excess stack values".into());
                }
            }
            "box" => {
                let token = i.operand.token().ok_or("missing box type")?;
                let declared = self.image.boxed_value_type(token, self.type_context)?;
                self.track_type(declared)?;
                let value = self.pop()?;
                if declared.kind() == StackKind::Ref {
                    if value.ty.kind() != StackKind::Ref && value.ty != Ty::Null {
                        return Err("box of a reference type needs a reference operand".into());
                    }
                    self.lines.push("// box of a proved reference type preserves the reference, including null; no value box is allocated.".into());
                    self.push(Ty::Object, format!("((object)({}))", value.name))?;
                    return Ok(());
                }
                // An enum lives on the stack as its storage integer; boxing it must
                // box the enum, not that integer, so the declared name is applied.
                let expression = match declared {
                    crate::ir::Ty::Enum(t) => {
                        // The enum reaches the stack as its storage integer, so the
                        // integer category is the check; the declared name is then
                        // applied so the box carries the enum type.
                        if value.ty.stack() != declared.stack() {
                            return Err("box operand type differs from its declared type".into());
                        }
                        format!("((object)(({})({})))", t.display(self.image)?, value.name)
                    }
                    _ => {
                        // Scalars reach the stack as normalized integers, so the
                        // category is the check and the declared name is the cast.
                        if value.ty.stack() != declared.stack() {
                            return Err("box operand type differs from its declared type".into());
                        }
                        if declared == Ty::Bool && value.ty != Ty::Bool {
                            format!("((object)({}))", declared.store(&value.name, self.image)?)
                        } else {
                            format!("((object)(({})({})))", declared.cs(self.image)?, value.name)
                        }
                    }
                };
                self.lines.push("// box: the runtime converts this exact declared type to object. A non-nullable value is copied into a fresh box here; generic references retain identity and nullable values use CLR nullable boxing.".into());
                self.push(crate::ir::Ty::Object, expression)?;
            }
            "unbox.any" => {
                let token = i.operand.token().ok_or("missing unbox type")?;
                if let Some(target) = self
                    .image
                    .reference_unbox_target(token, self.type_context)?
                {
                    let value = self.pop()?;
                    if value.ty.kind() != StackKind::Ref && value.ty != Ty::Null {
                        return Err(
                            "reference unbox.any needs an admitted reference operand".into()
                        );
                    }
                    self.lines.push("// Reference-type unbox.any: exact runtime castclass check; null passes through, wrong type throws InvalidCastException, and reference identity is preserved.".into());
                    self.push(
                        target,
                        format!("(({})(object){})", target.cs(self.image)?, value.name),
                    )?;
                    return Ok(());
                }
                let declared = self.image.boxed_value_type(token, self.type_context)?;
                self.track_type(declared)?;
                let value = self.pop()?;
                if value.ty.kind() != StackKind::Ref && value.ty != Ty::Null {
                    return Err("unbox.any needs an object reference".into());
                }
                self.lines.push("// unbox.any: exact runtime conversion to the declared type. Non-nullable values reject null and wrong boxed types and copy the value; generic references and nullable values retain CLR null behavior.".into());
                // The stack carries an enum as its storage integer, so the cast names
                // the enum for the runtime check and the conversion to that storage
                // is written explicitly; the stack type stays the declared storage.
                let (text, stack_ty) = match declared {
                    crate::ir::Ty::Enum(t) => {
                        let storage = t.storage.ty();
                        (
                            format!(
                                "(({})(({})({})))",
                                // The CLI value is the normalized integer category
                                // (int32/int64), so converting to the narrow or unsigned
                                // storage type here would not be assignable.
                                storage.stack().cs(self.image)?,
                                t.display(self.image)?,
                                value.name
                            ),
                            // `unbox.any` leaves the normalized integer category on
                            // the CLI stack, not the narrow storage width.
                            storage.stack(),
                        )
                    }
                    other => (
                        format!("(({})(object)({}))", other.cs(self.image)?, value.name),
                        other,
                    ),
                };
                self.push(stack_ty, text)?;
            }
            "leave" | "leave.s" => {
                self.lines.push("// leave: discards the evaluation stack; enclosing finally regions run innermost first, then control continues at the label.".into());
                self.stack.clear();
            }
            "endfinally" => {
                if !self.stack.is_empty() {
                    return Err("endfinally with a non-empty stack needs IL fallback".into());
                }
                // The emitter transfers to the end of this lexical finally; the
                // runtime resumes its original normal or exceptional continuation.
            }
            "endfilter" => {
                let value = self.pop()?;
                if !matches!(value.ty, Ty::Bool | Ty::I4 | Ty::U4) {
                    return Err("endfilter needs an integer or boolean result".into());
                }
                return Err("EH filters need filter-region lowering".into());
            }
            "rethrow" => {
                if !self.stack.is_empty() {
                    return Err("rethrow with a non-empty stack needs IL fallback".into());
                }
                self.lines.push("// rethrow: `throw;` re-raises the original exception object, so identity and the recorded trace are preserved and no catch selection runs again for this group.".into());
                self.lines.push("throw;".into());
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
fn comparison(
    op: &str,
    a: &Value,
    b: &Value,
    image: &crate::metadata::Image<'_>,
) -> Result<String> {
    // Reference equality is identity comparison. It never dereferences, never
    // calls an overloaded operator and never runs a user-defined equality body.
    let pointer_like = |ty: Ty| matches!(ty.kind(), StackKind::Pointer);
    // A pointer compared with the null constant is a null-pointer test in C#.
    // A pointer compared with an integer is the null-pointer-constant form; the
    // cast is bit-preserving and the equality is the same identity test.
    if matches!(op, "ceq" | "bne" | "beq") {
        let integer_like =
            |ty: Ty| matches!(ty.kind(), StackKind::I4 | StackKind::I8 | StackKind::Native);
        if pointer_like(a.ty) && integer_like(b.ty) {
            let target = a.ty.cs(image)?;
            let comparison = format!(
                "({} {symbol} ({target})({}))",
                a.name,
                b.name,
                symbol = if op == "ceq" { "==" } else { "!=" }
            );
            return Ok(comparison);
        }
        if pointer_like(b.ty) && integer_like(a.ty) {
            let target = b.ty.cs(image)?;
            let comparison = format!(
                "(({target})({}) {} {})",
                a.name,
                if op == "ceq" { "==" } else { "!=" },
                b.name
            );
            return Ok(comparison);
        }
    }
    if b.ty == Ty::Null && pointer_like(a.ty) && matches!(op, "ceq" | "cgt.un" | "beq") {
        return Ok(if op == "ceq" {
            format!("({} == null)", a.name)
        } else {
            format!("({} != null)", a.name)
        });
    }
    let reference_like = |ty: Ty| ty == Ty::Null || ty.kind() == StackKind::Ref;
    if reference_like(a.ty) && reference_like(b.ty) {
        let base = op.split('.').next().unwrap_or("");
        // Roslyn emits `cgt.un` for "is not null": an unsigned comparison against
        // the null constant is exactly a non-null identity test.
        if op == "cgt.un" && b.ty == Ty::Null && a.ty != Ty::Null {
            return Ok(format!(
                "!global::System.Object.ReferenceEquals({}, null)",
                a.name
            ));
        }
        if !matches!(base, "ceq" | "beq" | "bne") {
            return Err("ordered comparison of references needs IL fallback".into());
        }
        // `ReferenceEquals` takes both operands as `object`, so two references of
        // different static types need no common-type proof: the comparison is
        // identity on the values, not on their declared types.
        let identity = format!(
            "global::System.Object.ReferenceEquals({}, {})",
            a.name, b.name
        );
        return Ok(if base == "bne" {
            format!("!{identity}")
        } else {
            identity
        });
    }
    // CLI F comparisons treat NaN as unordered: the `.un` forms are true when
    // either operand is NaN, which is exactly the negation of the opposite
    // ordered relation. Converting the operands to integers would compare
    // representations instead of values.
    if (matches!(a.ty.kind(), StackKind::Float) || matches!(b.ty.kind(), StackKind::Float))
        && op.ends_with(".un")
    {
        return Ok(match op.split('.').next().unwrap_or("") {
            "cgt" | "bgt" => format!("!({} <= {})", a.name, b.name),
            "clt" | "blt" => format!("!({} >= {})", a.name, b.name),
            "bge" => format!("!({} < {})", a.name, b.name),
            "ble" => format!("!({} > {})", a.name, b.name),
            "bne" | "beq" => format!(
                "({} {} {})",
                a.name,
                if op.starts_with("bne") { "!=" } else { "==" },
                b.name
            ),
            _ => return Err("unknown unordered float comparison".into()),
        });
    }
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
// A struct receiver is a managed location of that exact value type.
/// Types with a `System.Threading.Volatile` overload. Enums, chars and structs
/// have no overload, so their volatile access keeps a stated refusal instead of a
/// plain read/write that would drop the acquire/release ordering.
fn volatile_supported(ty: Ty) -> bool {
    matches!(
        ty,
        Ty::Bool
            | Ty::I1
            | Ty::U1
            | Ty::I2
            | Ty::U2
            | Ty::I4
            | Ty::U4
            | Ty::I8
            | Ty::U8
            | Ty::Native
            | Ty::UNative
            | Ty::R4
            | Ty::R8
    ) || ty.kind() == StackKind::Ref
}

/// Merge two stack values at a control-flow join. The verifier merges reference
/// types to a common supertype; a null with a reference becomes that reference.
/// Anything else (mixed integer widths, addresses, value types) has no single C#
/// slot type here and stays a stated refusal.
fn join_reference_type(image: &crate::metadata::Image<'_>, stored: Ty, incoming: Ty) -> Option<Ty> {
    let reference = |ty: Ty| ty == Ty::Null || ty.kind() == StackKind::Ref;
    if stored == incoming {
        return Some(stored);
    }
    if !reference(stored) || !reference(incoming) {
        return None;
    }
    if stored == Ty::Null {
        return Some(incoming);
    }
    if incoming == Ty::Null {
        return Some(stored);
    }
    if image.assignable(stored, incoming).is_ok() {
        return Some(incoming);
    }
    if image.assignable(incoming, stored).is_ok() {
        return Some(stored);
    }
    // A shared local base class is the most precise common type; otherwise the
    // CLI's reference merge is object, and later uses that need a specific type
    // will keep their own refusal.
    if let (Ty::Class(a), Ty::Class(b)) = (stored, incoming)
        && let Ok(chain) = image.class_chain(a)
    {
        for ancestor in chain {
            let base = Ty::Class(ancestor);
            // An open generic definition is not a value type in C# and would
            // drop the instantiation; only a closed definition can merge here.
            if image.generic_definition_name(base).unwrap_or(false) {
                continue;
            }
            if image.assignable(Ty::Class(b), base).is_ok() {
                return Some(base);
            }
        }
    }
    Some(Ty::Object)
}

fn receiver_matches(actual: Ty, expected: Ty) -> bool {
    actual == expected
        || matches!((actual, expected), (Ty::ValueAddress(a), Ty::Value(b)) if a == b)
}
/// Like ILSpy's definite-assignment visitor, propagate possible uninitialized
/// slots by union at joins. Taking an address is a use, even for an out call.
/// Whether a `ldloca` is immediately consumed as a write of the local's storage.
/// Only the instructions that demonstrably write through the address are admitted;
/// any other use (including an intervening argument load) stays a read.
fn address_write(
    image: &crate::metadata::Image<'_>,
    instructions: &[Instruction],
    index: usize,
) -> bool {
    let Some(next) = instructions.get(index + 1) else {
        return false;
    };
    match next.opcode.canonical().name() {
        "initobj" | "stobj" | "cpobj" => true,
        name if name.starts_with("stind.") => true,
        // A value-type constructor called on the freshly taken address
        // initializes that storage (`ldloca; call S::.ctor`).
        "call" | "callvirt" => next
            .operand
            .token()
            .and_then(|token| member_name(image, token))
            .is_some_and(|name| name == ".ctor"),
        _ => false,
    }
}

fn member_name(image: &crate::metadata::Image<'_>, token: cildec::Token) -> Option<String> {
    use cildec::TableId;
    match token.table()? {
        TableId::MethodDef => Some(
            image
                .md
                .strings()
                .str(image.md.tables().method_def(token.rid()).ok()?.name)
                .ok()?
                .to_string(),
        ),
        TableId::MemberRef => Some(
            image
                .md
                .strings()
                .str(image.md.tables().member_ref(token.rid()).ok()?.name)
                .ok()?
                .to_string(),
        ),
        _ => None,
    }
}

/// Apply the writes of one block to a may-be-unassigned state.
fn apply_block_writes(
    image: &crate::metadata::Image<'_>,
    instructions: &[Instruction],
    block: &crate::cfg::Block,
    state: &mut [bool],
) -> Result<()> {
    for index in block.start..block.end {
        let instruction = &instructions[index];
        match instruction.opcode.canonical().name() {
            "stloc" => {
                let slot = state
                    .get_mut(var(instruction)?)
                    .ok_or("local out of range")?;
                *slot = false;
            }
            "ldloca" if address_write(image, instructions, index) => {
                let slot = state
                    .get_mut(var(instruction)?)
                    .ok_or("local out of range")?;
                *slot = false;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Refuse a read of a local that may still be unassigned after the block's writes
/// were applied in order.
fn check_block_reads(
    image: &crate::metadata::Image<'_>,
    instructions: &[Instruction],
    block: &crate::cfg::Block,
    state: &mut [bool],
) -> Result<()> {
    for index in block.start..block.end {
        let instruction = &instructions[index];
        match instruction.opcode.canonical().name() {
            "stloc" => {
                let slot = state
                    .get_mut(var(instruction)?)
                    .ok_or("local out of range")?;
                *slot = false;
            }
            "ldloca" if address_write(image, instructions, index) => {
                let slot = state
                    .get_mut(var(instruction)?)
                    .ok_or("local out of range")?;
                *slot = false;
            }
            "ldloc" | "ldloca" if *state.get(var(instruction)?).ok_or("local out of range")? => {
                return Err("local may be used before definite assignment".into());
            }
            _ => {}
        }
    }
    Ok(())
}

fn require_definite_local_assignment(
    blocks: &[crate::cfg::Block],
    instructions: &[Instruction],
    local_count: usize,
    body: &cildec::MethodBody<'_>,
    image: &crate::metadata::Image<'_>,
) -> Result<()> {
    // Handler entry states depend on every point of the protected region: an
    // exception can leave the region at any instruction. Seeding a handler with
    // the union of the region blocks' states is the conservative merge, and it is
    // what C# itself requires before a local may be read in a handler/finally.
    let handlers: Vec<(usize, usize, usize)> = body
        .exception_handlers
        .iter()
        .filter_map(|handler| {
            let start = handler.try_offset as usize;
            let end = start + handler.try_length as usize;
            blocks
                .iter()
                .position(|block| block.offset == handler.handler_offset)
                .map(|index| (index, start, end))
        })
        .collect();
    let mut entries: Vec<Option<Vec<bool>>> = vec![None; blocks.len()];
    entries[0] = Some(vec![true; local_count]);
    for _round in 0..32 {
        let mut pending = VecDeque::new();
        if _round == 0 {
            pending.push_back(0);
        }
        while let Some(index) = pending.pop_front() {
            let mut uninitialized = entries[index].as_ref().unwrap().clone();
            let block = &blocks[index];
            apply_block_writes(image, instructions, block, &mut uninitialized)?;
            for &successor in &block.successors {
                if let Some(existing) = &mut entries[successor] {
                    let mut changed = false;
                    for (old, &incoming) in existing.iter_mut().zip(&uninitialized) {
                        if incoming && !*old {
                            *old = true;
                            changed = true;
                        }
                    }
                    if changed {
                        pending.push_back(successor);
                    }
                } else {
                    entries[successor] = Some(uninitialized.clone());
                    pending.push_back(successor);
                }
            }
        }
        let mut grew = false;
        for &(handler, start, end) in &handlers {
            let mut seed = vec![false; local_count];
            let mut reached = false;
            for (index, block) in blocks.iter().enumerate() {
                if (block.offset as usize) < start || (block.offset as usize) >= end {
                    continue;
                }
                if let Some(state) = &entries[index] {
                    reached = true;
                    for (slot, &incoming) in seed.iter_mut().zip(state) {
                        *slot |= incoming;
                    }
                }
            }
            if !reached {
                return Err("uninitialized local handler region has no reachable block".into());
            }
            let existing = entries[handler].get_or_insert_with(|| vec![false; local_count]);
            for (old, &incoming) in existing.iter_mut().zip(&seed) {
                if incoming && !*old {
                    *old = true;
                    grew = true;
                }
            }
            if grew {
                // The handler itself and everything after it must be re-propagated.
                let mut queue = VecDeque::from([handler]);
                while let Some(index) = queue.pop_front() {
                    let mut uninitialized = entries[index].as_ref().unwrap().clone();
                    let block = &blocks[index];
                    apply_block_writes(image, instructions, block, &mut uninitialized)?;
                    for &successor in &block.successors {
                        let existing =
                            entries[successor].get_or_insert_with(|| uninitialized.clone());
                        let mut changed = false;
                        for (old, &incoming) in existing.iter_mut().zip(&uninitialized) {
                            if incoming && !*old {
                                *old = true;
                                changed = true;
                            }
                        }
                        if changed {
                            queue.push_back(successor);
                        }
                    }
                }
            }
        }
        if !grew {
            break;
        }
    }
    for (index, block) in blocks.iter().enumerate() {
        let Some(mut uninitialized) = entries[index].clone() else {
            continue;
        };
        check_block_reads(image, instructions, block, &mut uninitialized)?;
    }
    Ok(())
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
    // Native int and native uint share the integer stack category; the CLI
    // performs the same wrapping arithmetic on them.
    if matches!(ty.kind(), StackKind::I4 | StackKind::I8 | StackKind::Native) {
        Ok(())
    } else {
        Err("integer operation on non-integer value".into())
    }
}
/// Widen a narrower integer operand to the wider one, preserving its own sign
/// rule (unsigned storage zero-extends). Equal or non-integer operands are
/// returned unchanged so the caller's existing checks still decide.
fn widen_integers(
    image: &crate::metadata::Image<'_>,
    a: Value,
    b: Value,
) -> Result<(Value, Value)> {
    let rank = |ty: Ty| match ty.kind() {
        StackKind::I4 => Some(1),
        StackKind::I8 => Some(2),
        StackKind::Native => Some(2),
        _ => None,
    };
    let (Some(left), Some(right)) = (rank(a.ty), rank(b.ty)) else {
        return Ok((a, b));
    };
    if a.ty.kind() == b.ty.kind() {
        return Ok((a, b));
    }
    if left == right {
        // int64 and native int are the same width; mixing their spellings has no
        // single C# result type, so keep the refusal explicit.
        return Err("mixed int64/native integer arithmetic needs IL fallback".into());
    }
    let (narrow, wide) = if left < right {
        (a.clone(), b)
    } else {
        (b.clone(), a)
    };
    let target = wide.ty.cs(image)?;
    let converted = if matches!(narrow.ty, Ty::U4 | Ty::U8 | Ty::UNative) {
        format!(
            "unchecked(({target})({}){})",
            narrow.ty.cs(image)?,
            narrow.name
        )
    } else {
        format!("unchecked(({target})({}))", narrow.name)
    };
    let widened = Value {
        name: converted,
        ty: wide.ty.stack(),
    };
    Ok(if left < right {
        (widened, wide)
    } else {
        (wide, widened)
    })
}

fn expect(actual: Ty, expected: Ty) -> Result<()> {
    if matches!(actual.kind(), StackKind::Pointer) || matches!(expected.kind(), StackKind::Pointer)
    {
        return if actual == expected {
            Ok(())
        } else {
            Err("pointer operation needs identical pointee types".into())
        };
    }
    // Float operands share one stack category; equality is exact identity, so
    // binary32 and binary64 never mix without an explicit conversion.
    if matches!(actual.kind(), StackKind::Float) || matches!(expected.kind(), StackKind::Float) {
        return if actual == expected {
            Ok(())
        } else {
            Err("float operation needs identical operand types".into())
        };
    }
    integer(actual)?;
    integer(expected)?;
    if actual == expected {
        Ok(())
    } else {
        Err("incompatible stack categories".into())
    }
}
