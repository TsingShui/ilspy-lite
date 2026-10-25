// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

mod cfg;
mod ir;
mod lift;
mod metadata;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// List every MethodDef, including compiler-generated methods.
pub fn list_methods(bytes: &[u8]) -> Result<Vec<MethodInfo>> {
    metadata::Image::parse(bytes)?.methods()
}

/// Decode one method without executing it.
pub fn disassemble(bytes: &[u8], token: u32) -> Result<String> {
    metadata::Image::parse(bytes)?.disassemble(token)
}

/// Lower a supported method, or return a marked IL fallback without a fabricated body.
pub fn decompile(bytes: &[u8], token: u32) -> Result<DecompiledMethod> {
    let image = metadata::Image::parse(bytes)?;
    let name = image.name(token)?;
    let Some(body) = image.body(token)? else {
        return Ok(DecompiledMethod {
            status: Status::Declaration,
            text: image.disassemble(token)?,
        });
    };
    let banner = format!(
        "// C#-like reverse-engineering output; not guaranteed to compile.\n// token 0x{token:08X} {}\n",
        metadata::escape_line(&name)
    );
    match lift::emit(&image, token, &body) {
        Ok(source) => Ok(DecompiledMethod {
            status: Status::Decompiled,
            text: format!("{banner}{source}"),
        }),
        Err(error) => {
            let raw = match image.disassemble(token) {
                Ok(raw) => raw,
                Err(decode_error) => {
                    return Ok(DecompiledMethod {
                        status: Status::Failed,
                        text: format!(
                            "{banner}// FAILED: {}\n// Raw IL bytes (undecoded): {:02X?}\n// Header: maxstack={} flags={:?} locals={:?}\n// EH: {:?}\n",
                            metadata::escape_line(&decode_error.to_string()),
                            body.code,
                            body.max_stack,
                            body.flags,
                            body.local_var_sig,
                            body.exception_handlers
                        ),
                    });
                }
            };
            let commented = raw.lines().map(|s| format!("// {s}\n")).collect::<String>();
            Ok(DecompiledMethod {
                status: Status::Partial,
                text: format!(
                    "{banner}// IL FALLBACK: {}\n{commented}",
                    metadata::escape_line(&error.to_string())
                ),
            })
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Decompiled,
    Declaration,
    Partial,
    Failed,
}
pub struct DecompiledMethod {
    pub status: Status,
    pub text: String,
}

#[derive(Debug)]
pub struct MethodInfo {
    pub token: u32,
    pub name: String,
    pub declaring_type: String,
    pub simple_name: String,
}
