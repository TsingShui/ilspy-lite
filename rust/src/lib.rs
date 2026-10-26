// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

mod cfg;
mod dependencies;
pub mod discovery_paths;
mod eh;
mod ir;
mod lift;
mod locations;
mod metadata;
mod readable;
mod signature_guard;
mod signature_types;
mod symbol_names;

pub use dependencies::{
    AssemblyIdentity, AssemblyKind, AssemblyReference, AssemblyResolution, DefinitionLocation,
    DependencyContext, DependencyLimits, ImageIdentity, PropertyAccessor, ReferenceFailure,
    ReferenceSearchOptions, ReferenceVersionPolicy, ResolvedField, ResolvedMethod, ResolvedType,
    TypeRelation, TypeRelations, image_identity,
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

/// Scan every MethodDef using the ordinary decompilation path and one parsed image.
/// Failed methods are recorded independently; output I/O errors stop the scan.
pub fn scan_methods(
    bytes: &[u8],
    record: impl FnMut(u32, Status, Option<String>) -> Result<()>,
) -> Result<Summary> {
    scan_methods_with_context(bytes, None, record)
}

/// Scan every MethodDef with an explicit reference set.
///
/// The context must have been built from the same bytes. Its references are the
/// only facts lowering may use; a method's status here is the status the same
/// context produces for that token on its own.
pub fn scan_methods_with_context(
    bytes: &[u8],
    context: Option<&DependencyContext>,
    record: impl FnMut(u32, Status, Option<String>) -> Result<()>,
) -> Result<Summary> {
    let image = metadata::Image::parse(bytes)?;
    scan_range(&image, 1, &image, context, record)
}

type ScanRow = (u32, Status, Option<String>);
type ScanChunk = Result<Vec<ScanRow>>;

/// Scan every MethodDef with one interpreter per worker.
///
/// Each worker parses the image and rebuilds the reference context itself, so the
/// fact caches stay thread-local; results are emitted in MethodDef order and are
/// identical to the sequential scan for the same references. `threads` above 1 is
/// only useful for large images: the wall time of one image is otherwise bounded
/// by that image.
pub fn scan_methods_parallel(
    bytes: &[u8],
    threads: usize,
    context: Option<&(dyn Fn() -> Result<Option<DependencyContext>> + Sync)>,
    mut record: impl FnMut(u32, Status, Option<String>) -> Result<()>,
) -> Result<Summary> {
    let counted = metadata::Image::parse(bytes)?;
    let count = counted.md.tables().row_count(cildec::TableId::MethodDef);
    let workers = threads.max(1).min(count.max(1) as usize);
    if workers <= 1 {
        let owned = match context {
            Some(factory) => factory()?,
            None => None,
        };
        let image = metadata::Image::parse(bytes)?;
        return scan_range(&image, 1, &image, owned.as_ref(), record);
    }
    let chunk = count.div_ceil(workers as u32);
    let mut collected: Vec<ScanChunk> = Vec::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for index in 0..workers as u32 {
            let start = index * chunk + 1;
            let end = ((index + 1) * chunk).min(count);
            if start > end {
                break;
            }
            handles.push(
                scope.spawn(move || -> Result<Vec<(u32, Status, Option<String>)>> {
                    let image = metadata::Image::parse(bytes)?;
                    let owned = match context {
                        Some(factory) => factory()?,
                        None => None,
                    };
                    let mut rows = Vec::with_capacity((end - start + 1) as usize);
                    for rid in start..=end {
                        let token = 0x06000000 | rid;
                        let (status, reason) = scan_status(&image, token, owned.as_ref());
                        rows.push((token, status, reason));
                    }
                    Ok(rows)
                }),
            );
        }
        for handle in handles {
            match handle.join() {
                Ok(result) => collected.push(result),
                Err(_) => {
                    collected.push(Err("scan worker panicked".into()));
                }
            }
        }
    });
    let mut rows = Vec::with_capacity(count as usize);
    for chunk in collected {
        rows.extend(chunk?);
    }
    rows.sort_unstable_by_key(|row| row.0);
    let mut summary = Summary::default();
    for (token, status, reason) in rows {
        summary.record(&status);
        record(token, status, reason)?;
    }
    Ok(summary)
}

fn scan_range(
    image: &metadata::Image<'_>,
    first: u32,
    counted: &metadata::Image<'_>,
    context: Option<&DependencyContext>,
    mut record: impl FnMut(u32, Status, Option<String>) -> Result<()>,
) -> Result<Summary> {
    let mut summary = Summary::default();
    let count = counted.md.tables().row_count(cildec::TableId::MethodDef);
    for rid in first..=count {
        let token = 0x06000000 | rid;
        let (status, reason) = scan_status(image, token, context);
        summary.record(&status);
        record(token, status, reason)?;
    }
    Ok(summary)
}

fn scan_status(
    image: &metadata::Image<'_>,
    token: u32,
    context: Option<&DependencyContext>,
) -> (Status, Option<String>) {
    match decompile_method(image, token, context) {
        Ok(result) => {
            let reason = match result.status {
                Status::Partial => result
                    .text
                    .lines()
                    .find_map(|line| line.strip_prefix("// IL FALLBACK: "))
                    .map(String::from),
                Status::Failed => result
                    .text
                    .lines()
                    .find_map(|line| line.strip_prefix("// FAILED: "))
                    .map(String::from),
                _ => None,
            };
            (result.status, reason)
        }
        Err(error) => (Status::Failed, Some(error.to_string())),
    }
}

/// Decode one method without executing it.
pub fn disassemble(bytes: &[u8], token: u32) -> Result<String> {
    metadata::Image::parse(bytes)?.disassemble(token)
}

/// Lower a supported method, or return a marked IL fallback without a fabricated body.
pub fn decompile(bytes: &[u8], token: u32) -> Result<DecompiledMethod> {
    decompile_method(&metadata::Image::parse(bytes)?, token, None)
}

/// Choose a presentation without changing method status or lowering admission.
pub fn decompile_with_style(
    bytes: &[u8],
    token: u32,
    style: OutputStyle,
) -> Result<DecompiledMethod> {
    decompile_with_context(bytes, token, style, None)
}

pub(crate) fn decompile_with_context(
    bytes: &[u8],
    token: u32,
    style: OutputStyle,
    context: Option<&DependencyContext>,
) -> Result<DecompiledMethod> {
    let image = metadata::Image::parse(bytes)?;
    let mut result = decompile_method(&image, token, context)?;
    if style == OutputStyle::Readable {
        result.text = readable::method_view(&image, token, &result, context);
    }
    Ok(result)
}

/// Prefetch storage facts for the value types this method can load or store.
///
/// Only cross-image identities need a fact, and only the ones named by this
/// method's signature and local slots are asked about. A fact that resolves is
/// cached on the image, where the enum lookup finds it under the same token; a
/// type that does not resolve keeps its existing blocker unchanged.
fn prime_external_enums(
    image: &metadata::Image<'_>,
    token: u32,
    body: &cildec::MethodBody<'_>,
    context: &DependencyContext,
) {
    let Ok(types) = image.value_type_tokens(token, body) else {
        return;
    };
    for candidate in types.values {
        if candidate.table() != Some(cildec::TableId::TypeRef) || image.has_cached_enum(candidate) {
            continue;
        }
        use crate::dependencies::{ExternalValue, ExternalValueLookup};
        match context.external_value_type(candidate.0) {
            Ok(ExternalValueLookup::Value(ExternalValue::Enum(enum_type, name))) => {
                let external = image.intern_external_type_name(&name);
                image.cache_enum(
                    candidate,
                    crate::ir::EnumType {
                        external: Some(external),
                        ..enum_type
                    },
                );
            }
            Ok(ExternalValueLookup::Value(ExternalValue::Struct)) => {
                let identity = image
                    .intern_signature_type(&cildec::Type::ValueType(candidate), Default::default())
                    .map_err(|error| error.to_string());
                image.cache_external_struct_type(candidate, identity);
            }
            Ok(ExternalValueLookup::Refused(reason)) => {
                image.cache_external_struct_type(candidate, Err(reason));
            }
            // A context that could not answer must leave the caller byte-identical.
            Ok(ExternalValueLookup::Unavailable) | Err(_) => {}
        }
    }
}

/// Prefetch facts for the call targets this body names.
///
/// Only MemberRef operands are asked about; a MethodDef target is local and a
/// MethodSpec target is a generic instantiation, which stays gated regardless.
/// A refused fact is recorded with its reason so the call site reports why it
/// cannot be rendered instead of silently keeping a placeholder.
fn prime_external_calls(
    image: &metadata::Image<'_>,
    body: &cildec::MethodBody<'_>,
    context: &DependencyContext,
) {
    let mut seen = std::collections::BTreeSet::new();
    for instruction in body.instructions() {
        let Ok(instruction) = instruction else {
            break;
        };
        let Some(candidate) = instruction.operand.token() else {
            continue;
        };
        if candidate.table() != Some(cildec::TableId::MemberRef) || !seen.insert(candidate.0) {
            continue;
        }
        if image.external_call(candidate.0).is_some() {
            continue;
        }
        image.cache_external_call(
            candidate.0,
            context
                .external_call(candidate.0)
                .map_err(|error| error.to_string()),
        );
    }
}

/// Prefetch the facts for instance fields read or written on a foreign type.
///
/// Only operands of field opcodes are asked about, so a method reference never
/// reaches the field resolver.
fn prime_external_fields(
    image: &metadata::Image<'_>,
    body: &cildec::MethodBody<'_>,
    context: &DependencyContext,
) {
    let mut seen = std::collections::BTreeSet::new();
    for instruction in body.instructions() {
        let Ok(instruction) = instruction else {
            break;
        };
        if !matches!(
            instruction.opcode.name(),
            "ldfld" | "ldflda" | "stfld" | "ldsfld" | "ldsflda" | "stsfld"
        ) {
            continue;
        }
        let Some(candidate) = instruction.operand.token() else {
            continue;
        };
        if candidate.table() != Some(cildec::TableId::MemberRef) || !seen.insert(candidate.0) {
            continue;
        }
        if image.external_field(candidate.0).is_some() {
            continue;
        }
        image.cache_external_field(
            candidate.0,
            match context.external_field(candidate.0) {
                crate::dependencies::ExternalFieldLookup::Fact(fact) => Ok(fact),
                crate::dependencies::ExternalFieldLookup::Refused(reason) => Err(reason),
                crate::dependencies::ExternalFieldLookup::Unavailable => continue,
            },
        );
    }
}

/// Prefetch the facts for `newobj` on a foreign constructor.
///
/// Only `MemberRef` operands named `.ctor` are asked about; local constructors and
/// generic instantiations keep their existing paths. Asking during priming keeps a
/// method's admission independent of which other method was lowered first.
fn prime_external_allocations(
    image: &metadata::Image<'_>,
    body: &cildec::MethodBody<'_>,
    context: &DependencyContext,
) {
    let mut seen = std::collections::BTreeSet::new();
    for instruction in body.instructions() {
        let Ok(instruction) = instruction else {
            break;
        };
        let Some(candidate) = instruction.operand.token() else {
            continue;
        };
        if candidate.table() != Some(cildec::TableId::MemberRef) || !seen.insert(candidate.0) {
            continue;
        }
        if image.external_allocation(candidate.0).is_some() {
            continue;
        }
        let Ok(row) = image.md.tables().member_ref(candidate.rid()) else {
            continue;
        };
        if image.md.strings().str(row.name).ok() != Some(".ctor") {
            continue;
        }
        image.cache_external_allocation(
            candidate.0,
            match context.external_allocation(candidate.0) {
                crate::dependencies::ExternalAllocationLookup::Fact(fact) => Ok(fact),
                crate::dependencies::ExternalAllocationLookup::Refused(reason) => Err(reason),
                crate::dependencies::ExternalAllocationLookup::Unavailable => continue,
            },
        );
    }
}

/// Prefetch admission for constructed value types whose definition is external.
///
/// The proof is about the definition, so one fact covers every instantiation of
/// it; the recorded arity keeps a mismatched instantiation from borrowing it.
fn prime_external_value_definitions(
    image: &metadata::Image<'_>,
    token: u32,
    body: &cildec::MethodBody<'_>,
    context: &DependencyContext,
) {
    use crate::dependencies::ExternalValueLookup;
    let Ok(types) = image.value_type_tokens(token, body) else {
        return;
    };
    for (definition, arity) in types.value_definitions {
        if definition.table() != Some(cildec::TableId::TypeRef)
            || image.has_external_value_definition(definition)
        {
            continue;
        }
        image.cache_external_value_definition(
            definition,
            match context.external_value_definition(definition.0, arity) {
                ExternalValueLookup::Value(_) => Ok(arity as u32),
                ExternalValueLookup::Refused(reason) => Err(reason),
                ExternalValueLookup::Unavailable => continue,
            },
        );
    }
}

fn decompile_method(
    image: &metadata::Image<'_>,
    token: u32,
    context: Option<&DependencyContext>,
) -> Result<DecompiledMethod> {
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
    if let Some(context) = context {
        prime_external_enums(image, token, &body, context);
        prime_external_calls(image, &body, context);
        prime_external_allocations(image, &body, context);
        prime_external_fields(image, &body, context);
        prime_external_value_definitions(image, token, &body, context);
    }
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
    decompile_type_with_context(bytes, name, style, None)
}

pub(crate) fn decompile_type_with_context(
    bytes: &[u8],
    name: &str,
    style: OutputStyle,
    context: Option<&DependencyContext>,
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
            let result = match decompile_method(&image, token, context) {
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
        text = readable::type_view(&image, &types, &bodies, context);
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
