// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

mod cfg;
mod dependencies;
mod ir;
mod lift;
mod metadata;
mod readable;

pub use dependencies::{
    AssemblyIdentity, AssemblyReference, AssemblyResolution, DependencyContext, DependencyLimits,
};

/// Readable presentation is distinct from the complete diagnostic evidence view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputStyle {
    Readable,
    Details,
}

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

/// Choose a presentation without changing method status or lowering admission.
pub fn decompile_with_style(
    bytes: &[u8],
    token: u32,
    style: OutputStyle,
) -> Result<DecompiledMethod> {
    let image = metadata::Image::parse(bytes)?;
    let mut result = decompile_method(&image, token)?;
    if style == OutputStyle::Readable {
        result.text = readable::method_view(&image, token, &result);
    }
    Ok(result)
}

fn decompile_method(image: &metadata::Image<'_>, token: u32) -> Result<DecompiledMethod> {
    let name = match image.name(token) {
        Ok(name) => name,
        Err(error) => {
            let Some(body) = image.body(token)? else {
                return Err(error);
            };
            return Ok(failed_body(
                image,
                &format!("// token 0x{token:08X}\n"),
                &body,
                &error.to_string(),
            ));
        }
    };
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
                    return Ok(failed_body(
                        image,
                        &banner,
                        &body,
                        &decode_error.to_string(),
                    ));
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

fn failed_body(
    image: &metadata::Image<'_>,
    banner: &str,
    body: &cildec::MethodBody<'_>,
    error: &str,
) -> DecompiledMethod {
    let mut text = format!(
        "{banner}// FAILED: {}\n// Raw IL bytes (undecoded): {:02X?}\n// Header: maxstack={} flags={:?} locals={:?}\n// EH: {:?}\n// Unknown sections: {:?}\n",
        metadata::escape_line(error),
        body.code,
        body.max_stack,
        body.flags,
        body.local_var_sig,
        body.exception_handlers,
        body.unknown_sections
    );
    match image.body_evidence(body) {
        Ok(evidence) => {
            for line in evidence.lines() {
                text.push_str(&format!("// {line}\n"));
            }
        }
        Err(error) => text.push_str(&format!(
            "// Body evidence decoding failed: {}\n",
            metadata::escape_line(&error.to_string())
        )),
    }
    DecompiledMethod {
        status: Status::Failed,
        text,
    }
}

/// Decompile an exact metadata type and its owned nested types using one image.
/// Each unsupported or failed method remains visible; it does not suppress peers.
pub fn decompile_type(bytes: &[u8], name: &str) -> Result<DecompiledType> {
    decompile_type_with_style(bytes, name, OutputStyle::Details)
}

/// Select the same class result with either readable presentation or full evidence.
pub fn decompile_type_with_style(
    bytes: &[u8],
    name: &str,
    style: OutputStyle,
) -> Result<DecompiledType> {
    let image = metadata::Image::parse(bytes)?;
    let types = image.selected_types(name)?;
    let mut text =
        String::from("// Class-level C#-like analysis; not a compilable type declaration.\n");
    let mut summary = Summary::default();
    let mut bodies = std::collections::BTreeMap::new();
    for ty in &types {
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
        match image.marshalling(ty) {
            Ok((marshalling, failures)) => {
                text.push_str(&marshalling);
                summary.metadata_failed += failures;
            }
            Err(error) => {
                summary.metadata_failed += 1;
                text.push_str(&format!(
                    "// FAILED metadata marshalling for type 0x{:08X}: {}\n",
                    ty.token,
                    metadata::escape_line(&error.to_string())
                ));
            }
        }
        match image.method_implementations(ty) {
            Ok((mappings, failures)) => {
                text.push_str(&mappings);
                summary.metadata_failed += failures;
            }
            Err(error) => {
                summary.metadata_failed += 1;
                text.push_str(&format!(
                    "// FAILED metadata method implementations for type 0x{:08X}: {}\n",
                    ty.token,
                    metadata::escape_line(&error.to_string())
                ));
            }
        }
        match image.security_declarations(ty) {
            Ok((security, failures)) => {
                text.push_str(&security);
                summary.metadata_failed += failures;
            }
            Err(error) => {
                summary.metadata_failed += 1;
                text.push_str(&format!(
                    "// FAILED metadata security for type 0x{:08X}: {}\n",
                    ty.token,
                    metadata::escape_line(&error.to_string())
                ));
            }
        }
        match image.custom_attributes(ty) {
            Ok((attributes, failures)) => {
                text.push_str(&attributes);
                summary.metadata_failed += failures;
            }
            Err(error) => {
                summary.metadata_failed += 1;
                text.push_str(&format!(
                    "// FAILED metadata attributes for type 0x{:08X}: {}\n",
                    ty.token,
                    metadata::escape_line(&error.to_string())
                ));
            }
        }
        for &token in &ty.properties {
            match image.property_declaration(token, ty) {
                Ok((declaration, failures)) => {
                    text.push_str(&declaration);
                    summary.metadata_failed += failures;
                }
                Err(error) => {
                    summary.metadata_failed += 1;
                    text.push_str(&format!(
                        ".property 0x{token:08X}\n// FAILED metadata: {}\n",
                        metadata::escape_line(&error.to_string())
                    ));
                }
            }
        }
        for &token in &ty.events {
            match image.event_declaration(token, ty) {
                Ok((declaration, failures)) => {
                    text.push_str(&declaration);
                    summary.metadata_failed += failures;
                }
                Err(error) => {
                    summary.metadata_failed += 1;
                    text.push_str(&format!(
                        ".event 0x{token:08X}\n// FAILED metadata: {}\n",
                        metadata::escape_line(&error.to_string())
                    ));
                }
            }
        }
        for &token in &ty.fields {
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
        for &token in &ty.methods {
            match image.method_declaration(token, &ty.name) {
                Ok((declaration, failures)) => {
                    text.push_str(&declaration);
                    summary.metadata_failed += failures;
                }
                Err(error) => {
                    summary.metadata_failed += 1;
                    text.push_str(&format!(
                        ".method 0x{token:08X}\n// FAILED metadata: {}\n",
                        metadata::escape_line(&error.to_string())
                    ));
                }
            }
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
            text.push_str(&format!(
                ".method-status 0x{token:08X} {:?}\n",
                result.status
            ));
            text.push_str(&result.text);
            text.push('\n');
            bodies.insert(token, result);
        }
    }
    if style == OutputStyle::Readable {
        text = readable::type_view(&image, &types, &bodies);
        if summary.metadata_failed != 0 {
            text.push_str(
                "\n// Metadata could not be fully read; use --details for retained evidence.\n",
            );
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
