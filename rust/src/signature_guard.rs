// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Checked blob bodies for dependency identity binding, not a CLR validity verifier.
//! cildec 0.1.0 ignores suffixes and masks token RIDs to 24 bits. Walk its supported
//! structural grammar before constructing tokens, without allocating a second AST.
//! PINNED/byref/modified locals, reserved flags, misplaced sentinels, and generic
//! non-default conventions (whose arity cildec discards) are outside this seam.
//! Type legality, generic context, and token resolution remain the caller's job.

use cildec::signature::{calling_convention as cc, element_type as et};

const MAX_BYTES: usize = 1 << 20;
const MAX_WORK: usize = 1 << 20;
const MAX_DEPTH: u32 = 32;
const OPTIONS: cildec::SignatureOptions = cildec::SignatureOptions {
    recursion_limit: MAX_DEPTH,
};

pub fn method(bytes: &[u8]) -> crate::Result<cildec::MethodSig<'_>> {
    check(bytes, Kind::Method)?;
    Ok(cildec::MethodSig::parse_with(bytes, OPTIONS)?)
}

pub fn field(bytes: &[u8]) -> crate::Result<cildec::FieldSig<'_>> {
    check(bytes, Kind::Field)?;
    Ok(cildec::FieldSig::parse_with(bytes, OPTIONS)?)
}

pub fn property(bytes: &[u8]) -> crate::Result<cildec::PropertySig<'_>> {
    check(bytes, Kind::Property)?;
    Ok(cildec::PropertySig::parse_with(bytes, OPTIONS)?)
}

pub fn type_spec(bytes: &[u8]) -> crate::Result<cildec::TypeSpecSig<'_>> {
    check(bytes, Kind::TypeSpec)?;
    Ok(cildec::TypeSpecSig::parse_with(bytes, OPTIONS)?)
}

pub fn method_spec(bytes: &[u8]) -> crate::Result<cildec::MethodSpecSig<'_>> {
    check(bytes, Kind::MethodSpec)?;
    Ok(cildec::MethodSpecSig::parse_with(bytes, OPTIONS)?)
}

/// Callers reject local flags/modifiers before requesting this plain-local seam.
pub fn plain_locals(bytes: &[u8]) -> crate::Result<()> {
    check(bytes, Kind::PlainLocals)
}

enum Kind {
    PlainLocals,
    Method,
    Field,
    Property,
    TypeSpec,
    MethodSpec,
}

fn check(bytes: &[u8], kind: Kind) -> crate::Result<()> {
    if bytes.len() > MAX_BYTES {
        return Err("signature exceeds 1 MiB byte limit".into());
    }
    let mut s = Cursor {
        bytes,
        pos: 0,
        depth: 0,
        work: MAX_WORK,
    };
    match kind {
        Kind::Method => s.method()?,
        Kind::PlainLocals => {
            s.expect(0x07)?;
            let count = s.count()?;
            for _ in 0..count {
                s.ty()?;
            }
        }
        Kind::Field => {
            s.expect(cc::FIELD)?;
            s.mods()?;
            s.ty()?;
        }
        Kind::Property => {
            let flags = s.byte()?;
            if flags != cc::PROPERTY && flags != cc::PROPERTY | cc::HASTHIS {
                return Err(s.error("unsupported property flags"));
            }
            let count = s.count()?;
            s.param()?;
            for _ in 0..count {
                s.param()?;
            }
        }
        Kind::TypeSpec => s.ty()?,
        Kind::MethodSpec => {
            s.expect(cc::GENERICINST)?;
            let count = s.count()?;
            for _ in 0..count {
                s.ty()?;
            }
        }
    }
    if s.pos != bytes.len() {
        return Err(s.error("trailing signature bytes"));
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    depth: u32,
    work: usize,
}

impl Cursor<'_> {
    fn error(&self, message: &str) -> Box<dyn std::error::Error + Send + Sync> {
        format!("signature at byte {}: {message}", self.pos).into()
    }

    // Charge bytes and recursive entries, bounding both wide and deep walks.
    fn charge(&mut self, amount: usize) -> crate::Result<()> {
        self.work = self
            .work
            .checked_sub(amount)
            .ok_or_else(|| self.error("signature work limit exceeded"))?;
        Ok(())
    }

    fn enter(&mut self) -> crate::Result<()> {
        self.charge(1)?;
        if self.depth >= MAX_DEPTH {
            return Err(self.error("signature depth limit exceeded"));
        }
        self.depth += 1;
        Ok(())
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn byte(&mut self) -> crate::Result<u8> {
        self.charge(1)?;
        let byte = self
            .peek()
            .ok_or_else(|| self.error("truncated signature"))?;
        self.pos += 1;
        Ok(byte)
    }

    fn expect(&mut self, expected: u8) -> crate::Result<()> {
        if self.byte()? != expected {
            return Err(self.error("unexpected signature byte"));
        }
        Ok(())
    }

    fn unsigned(&mut self) -> crate::Result<u32> {
        let (value, used) = cildec::compressed::read_u32(&self.bytes[self.pos..])?;
        if (used == 2 && value < 0x80) || (used == 4 && value < 0x4000) {
            return Err(self.error("noncanonical compressed unsigned integer"));
        }
        self.charge(used)?;
        self.pos += used;
        Ok(value)
    }

    fn signed(&mut self) -> crate::Result<()> {
        let (value, used) = cildec::compressed::read_i32(&self.bytes[self.pos..])?;
        // Signed widths cannot be inferred from the unsigned rotated value.
        if (used == 2 && (-64..=63).contains(&value))
            || (used == 4 && (-8192..=8191).contains(&value))
        {
            return Err(self.error("noncanonical compressed signed integer"));
        }
        self.charge(used)?;
        self.pos += used;
        Ok(())
    }

    fn count(&mut self) -> crate::Result<u32> {
        let count = self.unsigned()?;
        if count as usize > self.bytes.len() - self.pos {
            return Err(self.error("signature count exceeds remaining bytes"));
        }
        Ok(count)
    }

    fn token(&mut self) -> crate::Result<()> {
        let coded = self.unsigned()?;
        let rid = coded >> 2;
        if coded & 3 == 3 || rid == 0 || rid > 0x00ff_ffff {
            return Err(self.error("invalid or unrepresentable TypeDefOrRefOrSpec token"));
        }
        Ok(())
    }

    fn mods(&mut self) -> crate::Result<bool> {
        let mut count = 0;
        while matches!(self.peek(), Some(et::CMOD_REQD | et::CMOD_OPT)) {
            self.byte()?;
            self.token()?;
            count += 1;
            if count > MAX_DEPTH {
                return Err(self.error("custom modifier limit exceeded"));
            }
        }
        Ok(count != 0)
    }

    fn param(&mut self) -> crate::Result<()> {
        self.mods()?;
        if self.peek() == Some(et::BYREF) {
            self.byte()?;
        }
        self.ty()
    }

    fn ty(&mut self) -> crate::Result<()> {
        self.enter()?;
        if self.mods()? {
            // Match cildec's Modified wrapper depth, including nested fnptrs.
            self.ty()?;
        } else {
            match self.byte()? {
                et::VOID
                | et::BOOLEAN
                | et::CHAR
                | et::I1
                | et::U1
                | et::I2
                | et::U2
                | et::I4
                | et::U4
                | et::I8
                | et::U8
                | et::R4
                | et::R8
                | et::STRING
                | et::TYPEDBYREF
                | et::I
                | et::U
                | et::OBJECT => {}
                et::CLASS | et::VALUETYPE => self.token()?,
                et::VAR | et::MVAR => {
                    self.unsigned()?;
                }
                et::PTR | et::SZARRAY => {
                    self.mods()?;
                    self.ty()?;
                }
                et::BYREF => self.ty()?,
                et::ARRAY => {
                    self.ty()?;
                    self.unsigned()?; // rank; no allocation or token conversion
                    let sizes = self.count()?;
                    for _ in 0..sizes {
                        self.unsigned()?;
                    }
                    let bounds = self.count()?;
                    for _ in 0..bounds {
                        self.signed()?;
                    }
                }
                et::GENERICINST => {
                    if !matches!(self.byte()?, et::CLASS | et::VALUETYPE) {
                        return Err(self.error("invalid generic instantiation kind"));
                    }
                    self.token()?;
                    let count = self.count()?;
                    for _ in 0..count {
                        self.ty()?;
                    }
                }
                et::FNPTR => self.method()?,
                _ => return Err(self.error("unsupported signature element type")),
            }
        }
        self.depth -= 1;
        Ok(())
    }

    fn method(&mut self) -> crate::Result<()> {
        self.enter()?;
        let flags = self.byte()?;
        let convention = flags & cc::MASK;
        if flags & 0x80 != 0
            || (flags & cc::EXPLICITTHIS != 0 && flags & cc::HASTHIS == 0)
            || !matches!(
                convention,
                cc::DEFAULT
                    | cc::C
                    | cc::STDCALL
                    | cc::THISCALL
                    | cc::FASTCALL
                    | cc::VARARG
                    | cc::UNMANAGED
                    | cc::NATIVEVARARG
            )
        {
            return Err(self.error("unsupported method convention or flags"));
        }
        if flags & cc::GENERIC != 0 {
            if convention != cc::DEFAULT {
                return Err(self.error("cildec cannot preserve generic non-default convention"));
            }
            self.unsigned()?;
        }
        let count = self.count()?;
        self.param()?;
        let mut sentinel = false;
        for _ in 0..count {
            if self.peek() == Some(et::SENTINEL) {
                if sentinel || !matches!(convention, cc::VARARG | cc::NATIVEVARARG) {
                    return Err(self.error("misplaced signature sentinel"));
                }
                self.byte()?;
                sentinel = true;
            }
            self.param()?;
        }
        self.depth -= 1;
        Ok(())
    }
}
