// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

mod binding;
mod classification;
mod search;
pub use binding::{
    DefinitionLocation, PropertyAccessor, ResolvedField, ResolvedMethod, ResolvedType,
    TypeRelation, TypeRelations,
};
pub use classification::AssemblyKind;
pub use search::ReferenceSearchOptions;

use crate::{DecompiledMethod, DecompiledType, OutputStyle, Result, metadata::Image};
use cildec::TableId;
use sha1::Digest;
use std::collections::BTreeMap;
use std::io::Read;

/// Metadata identity, not verification of a publisher or strong-name signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyIdentity {
    pub name: String,
    pub version: [u16; 4],
    pub culture: String,
    pub public_key_token: Vec<u8>,
    pub content_type: u32,
}

impl AssemblyIdentity {
    fn matches(&self, other: &Self) -> bool {
        self.version == other.version && self.same_contract_scope(other)
    }

    fn same_contract_scope(&self, other: &Self) -> bool {
        self.name.eq_ignore_ascii_case(&other.name)
            && self.culture.eq_ignore_ascii_case(&other.culture)
            && self.public_key_token == other.public_key_token
            && self.content_type == other.content_type
    }
}

impl std::fmt::Display for AssemblyIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let token = if self.public_key_token.is_empty() {
            "null".to_string()
        } else {
            self.public_key_token
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        };
        write!(
            f,
            "{}, Version={}.{}.{}.{}, Culture={}, PublicKeyToken={}, ContentType=0x{:X}",
            self.name.escape_default(),
            self.version[0],
            self.version[1],
            self.version[2],
            self.version[3],
            if self.culture.is_empty() {
                "neutral".to_string()
            } else {
                self.culture.escape_default().to_string()
            },
            token,
            self.content_type
        )
    }
}

#[derive(Clone, Debug)]
pub struct AssemblyReference {
    pub token: u32,
    pub identity: AssemblyIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceFailure {
    pub token: u32,
    pub reason: String,
    pub raw_row: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReferenceVersionPolicy {
    #[default]
    Exact,
    /// ILSpy loaded-reference binding: short assembly name, highest loaded version.
    IlSpy,
    /// Explicit opt-in metadata substitution to the nearest nonolder same-major
    /// identity. This does not certify API compatibility or runtime behavior.
    SameMajor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssemblyResolution {
    Resolved {
        identity: AssemblyIdentity,
        source: String,
    },
    Missing {
        requested: AssemblyIdentity,
    },
    Conflicting {
        requested: AssemblyIdentity,
        sources: Vec<String>,
    },
    Invalid {
        failure: ReferenceFailure,
    },
    Substituted {
        requested: AssemblyIdentity,
        selected: AssemblyIdentity,
        source: String,
        policy: ReferenceVersionPolicy,
    },
}

/// Facts about an instance method declared in another assembly.
///
/// Only facts that are already settled by the referenced definition are recorded:
/// the declared name, the declaring type, and the virtual/abstract flags. Which
/// override a call reaches remains a target-runtime decision, exactly as the
/// original `callvirt` left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalCall {
    pub name: String,
    pub declaring_type: String,
    pub is_virtual: bool,
    pub is_abstract: bool,
    pub(crate) property: Option<ExternalProperty>,
}

/// Metadata-proven source property for a virtual instance accessor call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExternalProperty {
    pub owner_name: String,
    pub name: String,
    pub getter: bool,
}

/// Facts read from the defining image for `newobj` on a foreign constructor.
///
/// The owner name and the constructor's shape come from the definition; the
/// parameter types the caller needs stay caller-scoped and are read from the
/// reference signature by `allocation_constructor`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExternalAllocation {
    pub owner_token: u32,
    pub owner_name: String,
    pub owner_sealed: bool,
    pub constructed: bool,
    pub arity: usize,
    pub parameter_count: usize,
}

/// Outcome of asking the context about one foreign constructor. `Unavailable`
/// leaves the caller byte-identical to a run without dependency facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExternalAllocationLookup {
    Fact(ExternalAllocation),
    Refused(String),
    Unavailable,
}

/// Identity and storage flags read from the selected defining image of a foreign field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExternalField {
    pub name: String,
    pub owner_name: String,
    pub flags: u16,
}

/// Outcome of asking the context about one foreign field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExternalFieldLookup {
    Fact(ExternalField),
    Refused(String),
    Unavailable,
}

/// A value type proven through the supplied references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExternalValue {
    /// Enum storage plus the defining assembly's own name for the type.
    Enum(crate::ir::EnumType, String),
    /// A struct whose whole-value copy is admitted; the caller keeps the identity
    /// the input signature already spells.
    Struct,
}

/// Outcome of asking the context about one value type.
///
/// `Unavailable` means the references did not answer the question at all, which
/// must leave the caller exactly as it was without a context. `Refused` means the
/// references answered and the definition failed the proof; that reason is worth
/// reporting in place of a generic one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExternalValueLookup {
    Value(ExternalValue),
    Refused(String),
    Unavailable,
}

type ExternalValueCache =
    std::cell::RefCell<BTreeMap<u32, std::result::Result<ExternalValueLookup, String>>>;

#[derive(Clone, Copy, Debug)]
pub struct DependencyLimits {
    pub max_assemblies: usize,
    pub max_bytes: usize,
    pub max_references_per_image: u32,
}

impl Default for DependencyLimits {
    fn default() -> Self {
        Self {
            max_assemblies: 256,
            max_bytes: 512 * 1024 * 1024,
            max_references_per_image: 65536,
        }
    }
}

/// An explicitly supplied metadata universe. Source labels are provenance only;
/// this interface never opens files, searches directories or executes assemblies.
pub struct DependencyContext {
    main: Vec<u8>,
    references: Vec<AssemblyReference>,
    reference_failures: Vec<ReferenceFailure>,
    candidates: Vec<(String, Vec<u8>, AssemblyIdentity)>,
    sources: BTreeMap<String, usize>,
    limits: DependencyLimits,
    search_evidence: Vec<String>,
    search_diagnostics: Vec<String>,
    version_policy: ReferenceVersionPolicy,
    kinds: BTreeMap<usize, AssemblyKind>,
    discovery_incomplete: bool,
    enum_types: ExternalValueCache,
    reference_registry: std::cell::RefCell<Option<binding::reference_relations::CachedRegistry>>,
}

impl DependencyContext {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        Self::with_limits(bytes, DependencyLimits::default())
    }

    pub fn with_limits(bytes: Vec<u8>, limits: DependencyLimits) -> Result<Self> {
        if limits.max_assemblies == 0 {
            return Err("dependency assembly count limit excludes main image".into());
        }
        if bytes.len() > limits.max_bytes {
            return Err("main image exceeds dependency byte limit".into());
        }
        let image = Image::parse(&bytes)?;
        let mut references = Vec::new();
        let mut reference_failures = Vec::new();
        let count = reference_count(&image, limits)?;
        for rid in 1..=count {
            let token = 0x23000000 | rid;
            let identity = reference_identity(&image, rid);
            match identity {
                Ok(identity) => references.push(AssemblyReference { token, identity }),
                Err(error) => reference_failures.push(ReferenceFailure {
                    token,
                    reason: error.to_string(),
                    raw_row: image
                        .md
                        .tables()
                        .row_bytes(TableId::AssemblyRef, rid)?
                        .to_vec(),
                }),
            }
        }
        let (kind, diagnostic) = match classification::classify(&image) {
            Ok(kind) => (kind, None),
            Err(error) => (
                AssemblyKind::Unclassified,
                Some(format!(
                    "main reference classification unavailable: {error}"
                )),
            ),
        };
        Ok(Self {
            main: bytes,
            references,
            reference_failures,
            candidates: Vec::new(),
            sources: BTreeMap::new(),
            enum_types: std::cell::RefCell::new(BTreeMap::new()),
            reference_registry: std::cell::RefCell::new(None),
            limits,
            search_evidence: Vec::new(),
            search_diagnostics: diagnostic.into_iter().collect(),
            version_policy: ReferenceVersionPolicy::Exact,
            kinds: BTreeMap::from([(0, kind)]),
            discovery_incomplete: false,
        })
    }

    /// Analyze the main image and retain dependency provenance separately from
    /// the original body/metadata status accounting.
    pub fn decompile_type(&self, name: &str, style: OutputStyle) -> Result<DecompiledType> {
        let mut result = crate::decompile_type_with_context(&self.main, name, style, Some(self))?;
        self.append_report(&mut result.text, style)?;
        Ok(result)
    }

    pub fn decompile(&self, token: u32, style: OutputStyle) -> Result<DecompiledMethod> {
        let mut result = crate::decompile_with_context(&self.main, token, style, Some(self))?;
        self.append_report(&mut result.text, style)?;
        Ok(result)
    }

    /// Facts about an instance method defined outside the main image, resolved
    /// only through the references this context was explicitly given.
    ///
    /// Static targets, interface members, constructors and generic instantiations
    /// are refused: rendering those as an ordinary instance call would either not
    /// compile or bind to a different member than the original call site.
    pub(crate) fn external_call(&self, token: u32) -> Result<ExternalCall> {
        let method = self.resolve_method(token)?;
        if method.flags & 0x0010 != 0 {
            return Err("external call target is not an instance method".into());
        }
        if method.owner.flags & 0x0020 != 0 {
            return Err("external call target is declared on an interface".into());
        }
        if !method.method_arguments.is_empty() || !method.instantiation_signature.is_empty() {
            return Err("external call target is a generic instantiation".into());
        }
        let signature = crate::signature_guard::method(&method.reference_signature)?;
        if let cildec::CallConv::Generic(count) = signature.calling_convention
            && count != 0
        {
            return Err("external call target declares generic parameters".into());
        }
        if method.name.starts_with(".ctor") || method.name.starts_with(".cctor") {
            return Err("external call target is a constructor".into());
        }
        // Special names require a unique metadata association and a separately
        // proven source lookup. Prefixes alone never establish a property.
        let special = method.flags & 0x0800 != 0
            || ["get_", "set_", "add_", "remove_"]
                .iter()
                .any(|prefix| method.name.starts_with(prefix));
        let property = if special {
            Some(self.external_property_presentation(token)?
                .ok_or("external call target is a special-name accessor without a supported property association")?)
        } else {
            None
        };
        if !is_plain_identifier(&method.name) {
            return Err("external call target name is not a plain identifier".into());
        }
        Ok(ExternalCall {
            name: method.name.clone(),
            declaring_type: method.owner.name.clone(),
            is_virtual: method.flags & 0x0040 != 0,
            is_abstract: method.flags & 0x0400 != 0,
            property,
        })
    }

    /// Admission of a value definition from a referenced assembly.
    ///
    /// The referenced definition must pass the same proof a local struct copy
    /// needs, with its own generic parameters open because the instantiation
    /// supplies them, and it must declare exactly the arity the call site used.
    /// A reference that cannot be resolved is reported as unavailable so the
    /// caller keeps the blocker it would have had without a context.
    /// Facts for `newobj` whose constructor belongs to a referenced image.
    ///
    /// Errors before the defining image is reached are `Unavailable`; once it is
    /// read, a failed proof is `Refused` with the precise reason.
    pub(crate) fn external_allocation(&self, token: u32) -> ExternalAllocationLookup {
        use cildec::Type;
        let method = match self.resolve_method(token) {
            Ok(method) => method,
            Err(_) => return ExternalAllocationLookup::Unavailable,
        };
        if method.name != ".ctor" {
            return ExternalAllocationLookup::Refused(
                "allocation reference is not a constructor".into(),
            );
        }
        if method.flags & 0x0010 != 0 {
            return ExternalAllocationLookup::Refused("constructor is static".into());
        }
        if method.flags & 0x0440 != 0 {
            return ExternalAllocationLookup::Refused("constructor is abstract or virtual".into());
        }
        let signature = match crate::signature_guard::method(&method.reference_signature) {
            Ok(signature) => signature,
            Err(_) => {
                return ExternalAllocationLookup::Refused(
                    "constructor signature is not usable".into(),
                );
            }
        };
        if signature.calling_convention != cildec::CallConv::Default
            || !signature.vararg_params.is_empty()
        {
            return ExternalAllocationLookup::Refused(
                "constructor signature is vararg or not an ordinary instance .ctor".into(),
            );
        }
        if !signature.return_type.is_void() {
            return ExternalAllocationLookup::Refused(
                "constructor signature does not return void".into(),
            );
        }
        for parameter in &signature.params {
            if parameter.by_ref
                || !parameter.custom_mods.is_empty()
                || matches!(parameter.type_, Type::TypedByRef)
            {
                return ExternalAllocationLookup::Refused(
                    "constructor parameter is byref or modified".into(),
                );
            }
        }
        let image = match self.image_for_source(&method.owner.definition.source) {
            Ok(image) => image,
            Err(_) => return ExternalAllocationLookup::Unavailable,
        };
        let definition = cildec::Token(method.owner.definition.token);
        if definition.table() != Some(cildec::TableId::TypeDef) {
            return ExternalAllocationLookup::Unavailable;
        }
        let flags = match image.md.tables().type_def(definition.rid()) {
            Ok(row) => row.flags,
            Err(_) => return ExternalAllocationLookup::Unavailable,
        };
        if flags & 0x0020 != 0 {
            return ExternalAllocationLookup::Refused("owner is an interface".into());
        }
        if flags & 0x0080 != 0 {
            return ExternalAllocationLookup::Refused("owner is abstract or a static class".into());
        }
        let is_value = matches!(image.has_core_base(definition, "ValueType"), Ok(true))
            || matches!(image.has_core_base(definition, "Enum"), Ok(true));
        if is_value {
            return ExternalAllocationLookup::Refused(
                "owner is a value type; value initialisation is not an object identity".into(),
            );
        }
        let declared = match image.md.tables().generic_params(definition) {
            Ok(parameters) => parameters.len(),
            Err(_) => return ExternalAllocationLookup::Unavailable,
        };
        let constructed = !method.owner.generic_arguments.is_empty();
        if declared == 0 && constructed {
            return ExternalAllocationLookup::Refused(
                "owner instantiation supplies arguments but the definition is not generic".into(),
            );
        }
        if declared != 0 && !constructed {
            return ExternalAllocationLookup::Refused(
                "owner is an open generic definition without an instantiation".into(),
            );
        }
        if constructed && method.owner.generic_arguments.len() != declared {
            return ExternalAllocationLookup::Refused(format!(
                "owner instantiation supplies {} arguments but the definition declares {declared}",
                method.owner.generic_arguments.len()
            ));
        }
        let owner_name = match crate::symbol_names::type_name(&image, definition, true) {
            Ok(name) if !name.is_empty() => name,
            _ => return ExternalAllocationLookup::Refused("owner name is not usable".into()),
        };
        ExternalAllocationLookup::Fact(ExternalAllocation {
            owner_token: method.owner.definition.token,
            owner_name,
            owner_sealed: flags & 0x0100 != 0,
            constructed,
            arity: declared,
            parameter_count: signature.params.len(),
        })
    }

    /// Facts for a stored field whose owner is referenced. Static storage needs
    /// no instance receiver layout; instance storage retains its class checks.
    ///
    /// Only the field's identity and flags come from the definition; its type is
    /// read from the caller-scoped `MemberRef` signature by `Image::field`.
    pub(crate) fn external_field(&self, token: u32) -> ExternalFieldLookup {
        let field = match self.resolve_field(token) {
            Ok(field) => field,
            Err(_) => return ExternalFieldLookup::Unavailable,
        };
        if field.flags & 0x0010 != 0 {
            if field.flags & 0x1040 != 0 {
                return ExternalFieldLookup::Refused(
                    "static field has no ordinary stored representation".into(),
                );
            }
            let image = match self.image_for_source(&field.owner.definition.source) {
                Ok(image) => image,
                Err(_) => return ExternalFieldLookup::Unavailable,
            };
            let owner = match image
                .md
                .tables()
                .type_def(field.owner.definition.token & 0xffffff)
            {
                Ok(row) => row,
                Err(_) => return ExternalFieldLookup::Unavailable,
            };
            if owner.flags & 0x20 != 0 {
                return ExternalFieldLookup::Refused(
                    "static interface storage needs a separate proof".into(),
                );
            }
            return ExternalFieldLookup::Fact(ExternalField {
                name: field.name,
                owner_name: field.owner.name,
                flags: field.flags,
            });
        }
        if field.flags & 0x0040 != 0 {
            return ExternalFieldLookup::Refused("field is a literal with no storage".into());
        }
        if field.flags & 0x1000 != 0 {
            return ExternalFieldLookup::Refused("field has a marshalled layout".into());
        }
        let image = match self.image_for_source(&field.owner.definition.source) {
            Ok(image) => image,
            Err(_) => return ExternalFieldLookup::Unavailable,
        };
        let definition = cildec::Token(field.owner.definition.token);
        if definition.table() != Some(cildec::TableId::TypeDef) {
            return ExternalFieldLookup::Unavailable;
        }
        let flags = match image.md.tables().type_def(definition.rid()) {
            Ok(row) => row.flags,
            Err(_) => return ExternalFieldLookup::Unavailable,
        };
        if flags & 0x0020 != 0 {
            return ExternalFieldLookup::Refused("owner is an interface".into());
        }
        let is_value = matches!(image.has_core_base(definition, "ValueType"), Ok(true))
            || matches!(image.has_core_base(definition, "Enum"), Ok(true));
        if is_value {
            return ExternalFieldLookup::Refused(
                "owner is a value type; its fields need managed-location semantics".into(),
            );
        }
        let owner_name = match crate::symbol_names::type_name(&image, definition, true) {
            Ok(name) if !name.is_empty() => name,
            _ => return ExternalFieldLookup::Refused("owner name is not usable".into()),
        };
        ExternalFieldLookup::Fact(ExternalField {
            name: field.name,
            owner_name,
            flags: field.flags,
        })
    }

    pub(crate) fn external_value_definition(
        &self,
        token: u32,
        arity: usize,
    ) -> ExternalValueLookup {
        let Ok(resolved) = self.resolve_type(token) else {
            return ExternalValueLookup::Unavailable;
        };
        let Ok(image) = self.image_for_source(&resolved.definition.source) else {
            return ExternalValueLookup::Unavailable;
        };
        let definition = cildec::Token(resolved.definition.token);
        if definition.table() != Some(cildec::TableId::TypeDef) {
            return ExternalValueLookup::Unavailable;
        }
        let declared = match image.md.tables().generic_params(definition) {
            Ok(params) => params.len() as u32,
            Err(_) => return ExternalValueLookup::Unavailable,
        };
        if declared != arity as u32 {
            return ExternalValueLookup::Refused(format!(
                "constructed value type supplies {arity} arguments but the referenced definition declares {declared}"
            ));
        }
        match image.value_definition_checks(definition, true) {
            Ok(()) => ExternalValueLookup::Value(ExternalValue::Struct),
            Err(error) => ExternalValueLookup::Refused(error.to_string()),
        }
    }

    /// Storage of an enum defined outside the main image, resolved only through
    /// the references this context was explicitly given.
    pub(crate) fn external_value_type(&self, token: u32) -> Result<ExternalValueLookup> {
        if let Some(cached) = self.enum_types.borrow().get(&token) {
            return cached.clone().map_err(Into::into);
        }
        let result = self.external_value_uncached(token);
        self.enum_types.borrow_mut().insert(
            token,
            match &result {
                Ok(lookup) => Ok(lookup.clone()),
                Err(error) => Err(error.to_string()),
            },
        );
        result
    }

    fn external_value_uncached(&self, token: u32) -> Result<ExternalValueLookup> {
        let Ok(resolved) = self.resolve_type(token) else {
            return Ok(ExternalValueLookup::Unavailable);
        };
        let Ok(image) = self.image_for_source(&resolved.definition.source) else {
            return Ok(ExternalValueLookup::Unavailable);
        };
        let definition = cildec::Token(resolved.definition.token);
        let kind = match image.value_definition_kind(definition) {
            Ok(kind) => kind,
            Err(_) => return Ok(ExternalValueLookup::Unavailable),
        };
        Ok(match kind {
            crate::metadata::ValueDefinitionKind::Enum => match image.checked_enum_type(definition)
            {
                Ok(enum_type) => {
                    // The name comes from the defining image, not from the caller's token.
                    let name = crate::symbol_names::type_name(&image, definition, true)?;
                    ExternalValueLookup::Value(ExternalValue::Enum(enum_type, name))
                }
                Err(error) => ExternalValueLookup::Refused(error.to_string()),
            },
            crate::metadata::ValueDefinitionKind::Struct => {
                match image.value_definition_checks(definition, true) {
                    Ok(()) => ExternalValueLookup::Value(ExternalValue::Struct),
                    Err(error) => ExternalValueLookup::Refused(error.to_string()),
                }
            }
            crate::metadata::ValueDefinitionKind::Other => ExternalValueLookup::Unavailable,
        })
    }

    fn image_for_source(&self, source: &str) -> Result<Image<'_>> {
        if source == "<main>" {
            return self.binding_image(0);
        }
        let index = *self
            .sources
            .get(source)
            .ok_or("unknown defining source label")?;
        self.binding_image(index + 1)
    }

    /// Metadata-only reference requests, selected snapshots and discovery provenance.
    pub fn dependency_report(&self) -> Result<String> {
        let mut text = String::new();
        self.append_report(&mut text, OutputStyle::Details)?;
        Ok(text)
    }

    fn append_report(&self, text: &mut String, style: OutputStyle) -> Result<()> {
        let kind = self.kinds[&0];
        if style == OutputStyle::Readable {
            let notice = match kind {
                AssemblyKind::ReferenceContract => {
                    "// Reference assembly API contract; body stubs are not runtime implementations.\n"
                }
                AssemblyKind::Unclassified => {
                    "// Input assembly role is unclassified; executable implementation identity is not proven.\n"
                }
                AssemblyKind::ImplementationImage => "",
            };
            text.insert_str(0, notice);
        }
        if style == OutputStyle::Details {
            match self.identity_source(0) {
                Ok((identity, _)) => text.push_str(&format!(
                    "\n.dependency-main kind={kind:?} identity={:?}\n",
                    identity.to_string()
                )),
                Err(error) => text.push_str(&format!(
                    "\n.dependency-main kind={kind:?} identity-unavailable={:?}\n",
                    error.to_string()
                )),
            }
            for evidence in &self.search_evidence {
                text.push_str(&format!("\n{evidence}\n"));
            }
            for diagnostic in &self.search_diagnostics {
                text.push_str(&format!("\n.dependency-search-diagnostic {diagnostic:?}\n"));
            }
            for (source, index) in &self.sources {
                let identity = &self.candidates[*index].2;
                text.push_str(&format!(
                    "\n.dependency-candidate source={source:?} identity={:?} kind={:?}\n",
                    identity.to_string(),
                    self.kinds[&(index + 1)]
                ));
            }
        }
        let mut tokens = self
            .references
            .iter()
            .map(|r| r.token)
            .chain(self.reference_failures.iter().map(|r| r.token))
            .collect::<Vec<_>>();
        tokens.sort_unstable();
        let mut missing = Vec::new();
        for token in tokens {
            let (state, evidence, warning) = match self.resolve(token)? {
                AssemblyResolution::Resolved { identity, source } => {
                    let requested = &self
                        .references
                        .iter()
                        .find(|r| r.token == token)
                        .ok_or("resolved reference lost its request identity")?
                        .identity;
                    (
                        "resolved",
                        format!(
                            "requested={:?} selected={:?} source={source:?}",
                            requested.to_string(),
                            identity.to_string()
                        ),
                        None,
                    )
                }
                AssemblyResolution::Missing { requested } => (
                    "missing",
                    format!(
                        "requested={:?} no matching supplied identity",
                        requested.to_string()
                    ),
                    Some(format!(
                        "Dependency unresolved: {requested}; reference identity retained."
                    )),
                ),
                AssemblyResolution::Conflicting { requested, sources } => (
                    "conflicting",
                    format!("requested={:?} sources={sources:?}", requested.to_string()),
                    Some(format!(
                        "Dependency conflicting: {requested}; no candidate selected."
                    )),
                ),
                AssemblyResolution::Substituted {
                    requested,
                    selected,
                    source,
                    policy,
                } => (
                    "substituted",
                    format!(
                        "requested={:?} selected={:?} source={source:?} policy={policy:?} evidence=metadata-only",
                        requested.to_string(),
                        selected.to_string()
                    ),
                    Some(format!(
                        "Dependency identity substituted: {requested} -> {selected}; {policy:?}, metadata only, not original-runtime behavior proof."
                    )),
                ),
                AssemblyResolution::Invalid { failure } => (
                    "invalid",
                    format!(
                        "reason={:?} raw-row={:02X?}",
                        failure.reason, failure.raw_row
                    ),
                    Some(format!(
                        "Dependency reference invalid: 0x{token:08X}: {}",
                        failure.reason.escape_default()
                    )),
                ),
            };
            if style == OutputStyle::Details {
                text.push_str(&format!(
                    "\n.dependency 0x{token:08X} status={state} {evidence}\n"
                ));
            } else if let Some(warning) = warning {
                if state == "missing" {
                    missing.push(warning);
                } else {
                    text.push_str(&format!("\n// {warning}\n"));
                }
            }
        }
        if missing.len() > 3 {
            text.push_str(&format!("\n// Dependencies unresolved: {}; exact references retained; see --details for identities and provenance.\n", missing.len()));
        } else {
            for warning in missing {
                text.push_str(&format!("\n// {warning}\n"));
            }
        }
        Ok(())
    }

    /// Remaining owned-image budget, for bounded reads by an external loader.
    pub fn remaining_bytes(&self) -> usize {
        self.limits.max_bytes
            - self.main.len()
            - self
                .candidates
                .iter()
                .map(|(_, bytes, _)| bytes.len())
                .sum::<usize>()
    }

    /// References with readable identities; malformed rows remain separately
    /// available through reference_failures() and resolve(token).
    pub fn references(&self) -> &[AssemblyReference] {
        &self.references
    }

    pub fn reference_failures(&self) -> &[ReferenceFailure] {
        &self.reference_failures
    }

    /// Read an explicitly supplied stream; this method does not open paths.
    pub fn add_reference_reader(&mut self, source: &str, reader: impl Read) -> Result<()> {
        let mut bytes = Vec::new();
        let limit = self.remaining_bytes().min(512 * 1024 * 1024);
        reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        // The extra byte is an overflow sentinel, never a complete snapshot.
        // Reject before content deduplication could mistake a prefix for an alias.
        if bytes.len() > limit {
            return Err("dependency read limit exceeded; complete snapshot required".into());
        }
        self.add_reference(source, bytes)
    }

    pub fn add_reference(&mut self, source: &str, bytes: Vec<u8>) -> Result<()> {
        if let Some(&index) = self.sources.get(source) {
            return if self.candidates[index].1 == bytes {
                Ok(())
            } else {
                Err("dependency source changed within immutable context".into())
            };
        }
        if let Some(index) = self
            .candidates
            .iter()
            .position(|(_, cached, _)| *cached == bytes)
        {
            self.sources.insert(source.to_string(), index);
            return Ok(());
        }
        if self.candidates.len() + 1 >= self.limits.max_assemblies {
            return Err("dependency assembly count limit exceeded".into());
        }
        if bytes.len() > self.remaining_bytes() {
            return Err("dependency byte limit exceeded".into());
        }
        let image = Image::parse(&bytes)?;
        reference_count(&image, self.limits)?;
        let identity = assembly_identity(&image)?;
        let kind = match classification::classify(&image) {
            Ok(kind) => kind,
            Err(error) => {
                self.search_diagnostics.push(format!(
                    "reference classification unavailable for {source:?}: {error}"
                ));
                AssemblyKind::Unclassified
            }
        };
        self.kinds.insert(self.candidates.len() + 1, kind);
        self.sources
            .insert(source.to_string(), self.candidates.len());
        self.candidates.push((source.to_string(), bytes, identity));
        Ok(())
    }

    pub fn resolve(&self, reference_token: u32) -> Result<AssemblyResolution> {
        if self.version_policy == ReferenceVersionPolicy::IlSpy {
            if let Some(failure) = self
                .reference_failures
                .iter()
                .find(|r| r.token == reference_token)
            {
                return Ok(AssemblyResolution::Invalid {
                    failure: failure.clone(),
                });
            }
            let requested = self
                .references
                .iter()
                .find(|r| r.token == reference_token)
                .ok_or("unknown assembly reference")?
                .identity
                .clone();
            let Some(module) = self.matching_modules(&requested).first().copied() else {
                return Ok(AssemblyResolution::Missing { requested });
            };
            let (selected, source) = self.identity_source(module)?;
            return Ok(if requested.matches(&selected) {
                AssemblyResolution::Resolved {
                    identity: selected,
                    source,
                }
            } else {
                AssemblyResolution::Substituted {
                    requested,
                    selected,
                    source,
                    policy: self.version_policy,
                }
            });
        }
        let exact = self.resolve_exact(reference_token)?;
        let AssemblyResolution::Missing { requested } = exact else {
            return Ok(exact);
        };
        if self.version_policy == ReferenceVersionPolicy::Exact || self.discovery_incomplete {
            return Ok(AssemblyResolution::Missing { requested });
        }
        let selected = self
            .candidates
            .iter()
            .filter(|(_, _, identity)| {
                requested.same_contract_scope(identity)
                    && requested.version[0] == identity.version[0]
                    && requested.version <= identity.version
            })
            .map(|(_, _, identity)| identity.version)
            .min();
        let Some(version) = selected else {
            return Ok(AssemblyResolution::Missing { requested });
        };
        let candidates = self
            .candidates
            .iter()
            .filter(|(_, _, identity)| {
                requested.same_contract_scope(identity) && identity.version == version
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Ok(AssemblyResolution::Conflicting {
                requested,
                sources: candidates.iter().map(|(s, _, _)| s.clone()).collect(),
            });
        }
        let (source, _, selected) = candidates[0];
        Ok(AssemblyResolution::Substituted {
            requested,
            selected: selected.clone(),
            source: source.clone(),
            policy: self.version_policy,
        })
    }

    fn resolve_exact(&self, reference_token: u32) -> Result<AssemblyResolution> {
        if let Some(failure) = self
            .reference_failures
            .iter()
            .find(|r| r.token == reference_token)
        {
            return Ok(AssemblyResolution::Invalid {
                failure: failure.clone(),
            });
        }
        let reference = self
            .references
            .iter()
            .find(|r| r.token == reference_token)
            .ok_or("unknown AssemblyRef token in dependency context")?;
        let candidates = self
            .candidates
            .iter()
            .filter(|(_, _, identity)| reference.identity.matches(identity))
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [] => Ok(AssemblyResolution::Missing {
                requested: reference.identity.clone(),
            }),
            [(source, _, identity)] => Ok(AssemblyResolution::Resolved {
                identity: identity.clone(),
                source: source.clone(),
            }),
            _ => Ok(AssemblyResolution::Conflicting {
                requested: reference.identity.clone(),
                sources: candidates
                    .iter()
                    .map(|(source, _, _)| source.clone())
                    .collect(),
            }),
        }
    }
}

pub(crate) fn is_core_reference(image: &Image<'_>, scope: cildec::Token) -> Result<bool> {
    if scope.table() != Some(TableId::AssemblyRef) || scope.rid() == 0 {
        return Ok(false);
    }
    let mut identity = reference_identity(image, scope.rid())?;
    // Base-type proof follows the ordinary assembly name comparison policy.
    let Some(name) = [
        "System.Runtime",
        "System.Private.CoreLib",
        "mscorlib",
        "netstandard",
    ]
    .into_iter()
    .find(|name| identity.name.eq_ignore_ascii_case(name)) else {
        return Ok(false);
    };
    identity.name = name.into();
    Ok(search::core_contract(&identity))
}

pub(crate) fn is_core_definition(image: &Image<'_>) -> Result<bool> {
    if image.md.tables().row_count(TableId::Assembly) == 0 {
        return Ok(false);
    }
    Ok(search::core_contract(&assembly_identity(image)?))
}

/// One image's own identity together with the references it declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageIdentity {
    pub identity: AssemblyIdentity,
    pub references: Vec<AssemblyIdentity>,
}

/// Report the identity of one image and the references it declares, without
/// loading or resolving any other file. An image without exactly one Assembly
/// definition (a module or a netmodule) has no identity and is rejected.
pub fn image_identity(bytes: &[u8]) -> Result<ImageIdentity> {
    let image = Image::parse(bytes)?;
    let identity = assembly_identity(&image)?;
    let count = reference_count(&image, DependencyLimits::default())?;
    let mut references = Vec::with_capacity(count as usize);
    for rid in 1..=count {
        references.push(reference_identity(&image, rid)?);
    }
    Ok(ImageIdentity {
        identity,
        references,
    })
}

/// A name that can be written after `@` without changing what it binds to.
fn is_plain_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    match bytes.next() {
        Some(first) if first == b'_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

fn assembly_identity(image: &Image<'_>) -> Result<AssemblyIdentity> {
    if image.md.tables().row_count(TableId::Assembly) != 1 {
        return Err("dependency needs exactly one Assembly definition".into());
    }
    let row = image.md.tables().assembly(1)?;
    let public_key = image.md.blobs().get(row.public_key)?;
    if !public_key.is_empty() && row.flags & 1 == 0 {
        return Err("Assembly definition key requires the full-public-key flag".into());
    }
    Ok(AssemblyIdentity {
        name: image.md.strings().str(row.name)?.to_string(),
        version: [
            row.major_version,
            row.minor_version,
            row.build_number,
            row.revision_number,
        ],
        culture: image.md.strings().str(row.culture)?.to_string(),
        public_key_token: key_token(public_key, row.flags & 1 != 0)?,
        content_type: row.flags & 0x0e00,
    })
}

fn reference_identity(image: &Image<'_>, rid: u32) -> Result<AssemblyIdentity> {
    let row = image.md.tables().assembly_ref(rid)?;
    Ok(AssemblyIdentity {
        name: image.md.strings().str(row.name)?.to_string(),
        version: [
            row.major_version,
            row.minor_version,
            row.build_number,
            row.revision_number,
        ],
        culture: image.md.strings().str(row.culture)?.to_string(),
        public_key_token: key_token(
            image.md.blobs().get(row.public_key_or_token)?,
            row.flags & 1 != 0,
        )?,
        content_type: row.flags & 0x0e00,
    })
}

fn reference_count(image: &Image<'_>, limits: DependencyLimits) -> Result<u32> {
    let count = image.md.tables().row_count(TableId::AssemblyRef);
    if count > 0x00ffffff || count > limits.max_references_per_image {
        return Err("dependency reference limit exceeded".into());
    }
    Ok(count)
}

fn key_token(bytes: &[u8], full_key: bool) -> Result<Vec<u8>> {
    if bytes.is_empty() {
        return if full_key {
            Err("full-public-key flag without a key".into())
        } else {
            Ok(Vec::new())
        };
    }
    if full_key {
        // ECMA-335 assembly identity uses the reversed final eight SHA-1 bytes.
        // This is a legacy identifier calculation, not signature authentication.
        return Ok(sha1::Sha1::digest(bytes)
            .iter()
            .rev()
            .take(8)
            .copied()
            .collect());
    }
    if bytes.len() != 8 {
        return Err("assembly public-key token must contain eight bytes".into());
    }
    Ok(bytes.to_vec())
}
