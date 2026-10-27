// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

// Each suite includes the shared contract helper as its own module, so a suite
// stays compilable on its own; clippy reports that deliberate repetition.
#![allow(clippy::duplicate_mod)]

//! One test binary for the whole suite.
//!
//! Every suite is a module here instead of its own test target, so a change to
//! the library relinks one binary rather than one per file. A single module is
//! still selectable by test name: `cargo test --test all -- <substring>` or
//! `cargo nextest run -E 'test(<substring>)'`.
//!
//! `discovery_paths` and `signature_guard` stay separate targets because they
//! compile a source file from `src/` into their own crate root, which cannot be
//! nested under this one.

#[path = "addresses.rs"]
mod addresses;
#[path = "array_references.rs"]
mod array_references;
#[path = "arrays.rs"]
mod arrays;
#[path = "assembly.rs"]
mod assembly;
#[path = "casts.rs"]
mod casts;
#[path = "class_attachment_owners.rs"]
mod class_attachment_owners;
#[path = "class_attributes.rs"]
mod class_attributes;
#[path = "class_endpoint_evidence.rs"]
mod class_endpoint_evidence;
#[path = "class_events.rs"]
mod class_events;
#[path = "class_fields.rs"]
mod class_fields;
#[path = "class_generics.rs"]
mod class_generics;
#[path = "class_marshalling.rs"]
mod class_marshalling;
#[path = "class_method_impls.rs"]
mod class_method_impls;
#[path = "class_methods.rs"]
mod class_methods;
#[path = "class_ownership.rs"]
mod class_ownership;
#[path = "class_pinvoke_evidence.rs"]
mod class_pinvoke_evidence;
#[path = "class_properties.rs"]
mod class_properties;
#[path = "class_security.rs"]
mod class_security;
#[path = "class_type_reference_evidence.rs"]
mod class_type_reference_evidence;
#[path = "class_wide_owners.rs"]
mod class_wide_owners;
#[path = "classes.rs"]
mod classes;
#[path = "cli.rs"]
mod cli;
#[path = "constructors.rs"]
mod constructors;
#[path = "conversions.rs"]
mod conversions;
#[path = "core_types.rs"]
mod core_types;
#[path = "dependencies.rs"]
mod dependencies;
#[path = "dependency_binding.rs"]
mod dependency_binding;
#[path = "dependency_output.rs"]
mod dependency_output;
#[path = "enums.rs"]
mod enums;
#[path = "equality.rs"]
mod equality;
#[path = "exception_regions.rs"]
mod exception_regions;
mod exception_types;
#[path = "exceptions.rs"]
mod exceptions;
#[path = "external.rs"]
mod external;
#[path = "external_calls.rs"]
mod external_calls;
#[path = "external_enums.rs"]
mod external_enums;
#[path = "external_properties.rs"]
mod external_properties;
#[path = "external_values.rs"]
mod external_values;
#[path = "managed_locations.rs"]
mod managed_locations;
#[path = "reference_calls.rs"]
mod reference_calls;
mod reference_relations;
mod static_fields;
mod typed_arrays;

#[path = "abstract_members.rs"]
mod abstract_members;
mod constrained_calls;
#[path = "external_allocations.rs"]
mod external_allocations;
#[path = "floats.rs"]
mod floats;
#[path = "generic_binding.rs"]
mod generic_binding;
#[path = "generic_declaring.rs"]
mod generic_declaring;
#[path = "generic_references.rs"]
mod generic_references;
#[path = "generic_values.rs"]
mod generic_values;
#[path = "interface_references.rs"]
mod interface_references;
#[path = "method_instantiations.rs"]
mod method_instantiations;
#[path = "naming.rs"]
mod naming;
#[path = "native.rs"]
mod native;
#[path = "new_objects.rs"]
mod new_objects;
#[path = "objects.rs"]
mod objects;
#[path = "post_decrement.rs"]
mod post_decrement;
#[path = "property_output.rs"]
mod property_output;
#[path = "readability.rs"]
mod readability;
#[path = "readable_output.rs"]
mod readable_output;
#[path = "reference_search.rs"]
mod reference_search;
#[path = "references.rs"]
mod references;
#[path = "robustness.rs"]
mod robustness;
#[path = "scoped_references.rs"]
mod scoped_references;
#[path = "static_constructors.rs"]
mod static_constructors;
#[path = "strings.rs"]
mod strings;
#[path = "struct_copies.rs"]
mod struct_copies;
#[path = "struct_receivers.rs"]
mod struct_receivers;
#[path = "token_values.rs"]
mod token_values;
mod value_calls;
#[path = "virtual_calls.rs"]
mod virtual_calls;
