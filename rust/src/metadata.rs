// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use crate::{MethodInfo, Result};
use cildec::tables::MethodDefRow;
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
        out.push_str(&format!(
            ".maxstack {}\n.initlocals {}\n",
            body.max_stack,
            body.flags.init_locals()
        ));
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
        let name = if identifier(original).is_ok() && !original.starts_with("__il_field_") {
            format!("@{original}")
        } else {
            format!("__il_field_{:08X}", t.0)
        };
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
