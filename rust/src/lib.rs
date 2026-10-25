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
    decompile_method(&metadata::Image::parse(bytes)?, token)
}

fn decompile_method(image: &metadata::Image<'_>, token: u32) -> Result<DecompiledMethod> {
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
    match lift::emit(image, token, &body) {
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

/// Decompile an exact metadata type and its owned nested types using one image.
/// Each unsupported or failed method remains visible; it does not suppress peers.
pub fn decompile_type(bytes: &[u8], name: &str) -> Result<DecompiledType> {
    let image = metadata::Image::parse(bytes)?;
    let types = image.selected_types(name)?;
    let mut text =
        String::from("// Class-level C#-like analysis; not a compilable type declaration.\n");
    let mut summary = Summary::default();
    for ty in types {
        text.push_str(&format!(
            "// type token 0x{:08X} {}\n",
            ty.token,
            metadata::escape_line(&ty.name)
        ));
        match image.type_context(ty.token) {
            Ok((context, failures)) => {
                text.push_str(&context);
                summary.metadata_failed += failures;
            }
            Err(error) => {
                summary.metadata_failed += 1;
                text.push_str(&format!(
                    "// FAILED metadata for type 0x{:08X}: {}\n",
                    ty.token,
                    metadata::escape_line(&error.to_string())
                ));
            }
        }
        for token in ty.fields {
            match image.field_declaration(token) {
                Ok((declaration, failures)) => {
                    text.push_str(&declaration);
                    summary.metadata_failed += failures;
                }
                Err(error) => {
                    summary.metadata_failed += 1;
                    text.push_str(&format!(
                        ".field 0x{token:08X}\n// FAILED metadata: {}\n",
                        metadata::escape_line(&error.to_string())
                    ));
                }
            }
        }
        for token in ty.methods {
            let result = match decompile_method(&image, token) {
                Ok(result) => result,
                Err(error) => DecompiledMethod {
                    status: Status::Failed,
                    text: format!(
                        "// FAILED token 0x{token:08X}: {}\n",
                        metadata::escape_line(&error.to_string())
                    ),
                },
            };
            summary.record(&result.status);
            text.push_str(&result.text);
            text.push('\n');
        }
    }
    Ok(DecompiledType { text, summary })
}

pub struct DecompiledType {
    pub text: String,
    pub summary: Summary,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub decompiled: usize,
    pub declarations: usize,
    pub partial: usize,
    pub failed: usize,
    /// Metadata failures are separate from the number of failed method bodies.
    pub metadata_failed: usize,
}

impl Summary {
    pub fn record(&mut self, status: &Status) {
        match status {
            Status::Decompiled => self.decompiled += 1,
            Status::Declaration => self.declarations += 1,
            Status::Partial => self.partial += 1,
            Status::Failed => self.failed += 1,
        }
    }

    pub fn exit_code(&self) -> u8 {
        if self.failed > 0 || self.metadata_failed > 0 {
            1
        } else if self.partial > 0 {
            2
        } else {
            0
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
