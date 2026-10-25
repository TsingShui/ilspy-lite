// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Static binding exporter for tests/dependency-oracle.py. Does not execute inputs.
use ilspy_lite::{
    DefinitionLocation, DependencyContext, PropertyAccessor, ResolvedField, ResolvedMethod,
    ResolvedType, TypeRelation,
};
use serde_json::{Value, json};
use std::{env, fs};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

fn location(value: &DefinitionLocation) -> Value {
    json!({
        "assembly": {
            "name": value.assembly.name,
            "version": value.assembly.version,
            "culture": value.assembly.culture,
            "public_key_token": hex(&value.assembly.public_key_token),
            "content_type": value.assembly.content_type,
        },
        "source": value.source, "token": value.token, "kind": format!("{:?}", value.kind),
    })
}

fn ty(value: &ResolvedType) -> Value {
    json!({
        "definition": location(&value.definition), "name": value.name, "flags": value.flags,
        "exact_identity": value.exact_identity, "generic_arguments": value.generic_arguments,
        "forwarders": value.forwarders.iter().map(location).collect::<Vec<_>>(),
    })
}

fn method(value: &ResolvedMethod) -> Value {
    json!({
        "definition": location(&value.definition), "owner": ty(&value.owner),
        "name": value.name, "flags": value.flags,
        "reference_signature": hex(&value.reference_signature),
        "definition_signature": hex(&value.definition_signature),
        "signature_identity": value.signature_identity, "method_arguments": value.method_arguments,
        "instantiation_signature": hex(&value.instantiation_signature),
    })
}

fn field(value: &ResolvedField) -> Value {
    json!({
        "definition": location(&value.definition), "owner": ty(&value.owner),
        "name": value.name, "flags": value.flags,
        "reference_signature": hex(&value.reference_signature),
        "definition_signature": hex(&value.definition_signature),
        "signature_identity": value.signature_identity,
    })
}

fn property(value: &PropertyAccessor) -> Value {
    json!({
        "definition": location(&value.definition), "owner": ty(&value.owner),
        "name": value.name, "role": value.role, "signature": hex(&value.signature),
        "method": method(&value.method),
    })
}

fn relation(value: &TypeRelation) -> Value {
    json!({
        "reference_token": value.reference_token,
        "resolved": value.resolved.as_ref().map(ty), "error": value.error,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("expected main-image requests.json [reference-image ...]".into());
    }
    let mut context = DependencyContext::new(fs::read(&args[0])?)?;
    for reference in &args[2..] {
        context.add_reference(reference, fs::read(reference)?)?;
    }
    let requests: Vec<Value> = serde_json::from_slice(&fs::read(&args[1])?)?;
    let mut results = Vec::new();
    for request in requests {
        let token = u32::try_from(request["token"].as_u64().ok_or("expected token")?)?;
        let kind = request["kind"].as_str().ok_or("expected request kind")?;
        let result = match kind {
            "type" => context.resolve_type(token).map(|v| ty(&v)),
            "method" => context.resolve_method(token).map(|v| method(&v)),
            "field" => context.resolve_field(token).map(|v| field(&v)),
            "property" => context
                .resolve_property_accessor(token)
                .map(|v| v.as_ref().map(property).unwrap_or(Value::Null)),
            "relations" => context.type_relations(token).map(|v| {
                json!({
                    "owner": ty(&v.owner), "base_type": v.base_type.as_ref().map(relation),
                    "interfaces": v.interfaces.iter().map(relation).collect::<Vec<_>>(),
                })
            }),
            _ => return Err(format!("unknown request kind: {kind}").into()),
        };
        results.push(match result {
            Ok(value) => json!({"kind": kind, "token": token, "value": value}),
            Err(error) => json!({"kind": kind, "token": token, "error": error.to_string()}),
        });
    }
    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}
