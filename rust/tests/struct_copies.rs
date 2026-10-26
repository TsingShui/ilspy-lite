// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/struct-copy-release.bin"),
    include_bytes!("fixtures/struct-copy-debug.bin"),
];

fn method(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn struct_base_identity_uses_existing_case_insensitive_core_assembly_matching() {
    let bytes = IMAGES[0];
    let token = method(bytes, "StructCopyCases.PairOps", "Identity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let pair = md.tables().type_def(2).unwrap();
    assert_eq!(md.strings().str(pair.type_name).unwrap(), "Pair");
    let base = md.tables().type_ref(pair.extends.rid()).unwrap();
    let scope = md
        .tables()
        .assembly_ref(base.resolution_scope.rid())
        .unwrap();
    let name = md.strings().str(scope.name).unwrap();
    assert_eq!(name, "System.Runtime");
    let offset = name.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset] = b's';
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Decompiled, "{}", result.text);
}

#[test]
fn impossible_instance_field_types_cannot_become_opaque_struct_copies() {
    let bytes = IMAGES[0];
    let token = method(bytes, "StructCopyCases.PairOps", "Identity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md
        .tables()
        .field_range(cildec::Rid::new(2))
        .unwrap()
        .next()
        .unwrap();
    let field = md.tables().field(rid).unwrap();
    let signature = md.blobs().get(field.signature).unwrap();
    assert_eq!(signature, &[6, 8]);
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    for kind in [1, 0x16] {
        // Void and TypedByRef are not ordinary stored fields.
        let mut patched = bytes.to_vec();
        patched[offset + 1] = kind;
        let result = decompile(&patched, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("struct field"));
    }
}

#[test]
fn struct_values_are_not_null_integers_or_other_nominal_structs() {
    let bytes = IMAGES[0];
    let token = method(bytes, "StructCopyCases.PairOps", "Identity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(token & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.code, &[2, 0x2a]);
    let offset = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    for opcode in [0x14, 0x17] {
        let mut patched = bytes.to_vec();
        patched[offset] = opcode;
        let result = decompile(&patched, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("incompatible value/receiver types"));
    }
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature, &[0, 1, 0x11, 8, 0x11, 8]);
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    assert_eq!(
        md.strings()
            .str(md.tables().type_def(6).unwrap().type_name)
            .unwrap(),
        "Frozen"
    );
    patched[offset + 3] = 24; // Frozen return, Pair argument, no conversion.
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("incompatible value/receiver types"));
}

#[test]
fn struct_scan_preserves_every_method_and_status() {
    for bytes in IMAGES {
        let mut records = Vec::new();
        ilspy_lite::scan_methods(bytes, |token, status, _| {
            records.push((token, status));
            Ok(())
        })
        .unwrap();
        let methods = list_methods(bytes).unwrap();
        assert_eq!(records.len(), methods.len());
        for (definition, (token, status)) in methods.iter().zip(records) {
            assert_eq!(definition.token, token);
            assert_eq!(decompile(bytes, token).unwrap().status, status);
        }
    }
}

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring). There
// are no counts; a promotion must drop its row and a regression must add one.
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "StructCopyCases.WithCtor",
        ".ctor",
        "external allocation needs dependency facts; only local classes and local instantiations are admitted",
    ),
    (
        "StructCopyCases.UnsupportedOps",
        "UnsupportedExternal",
        "enum storage requires an exact local definition; external/value signatures need IL fallback",
    ),
    (
        "StructCopyCases.UnsupportedOps",
        "UnsupportedNullable",
        "constructed value type needs a local definition; external instantiations need dependency facts",
    ),
    (
        "StructCopyCases.UnsupportedOps",
        "UnsupportedBox",
        "box/unbox needs an admitted core scalar",
    ),
    (
        "StructCopyCases.UnsupportedOps",
        "UnsupportedRefLike",
        "byref-like struct copies need location/lifetime semantics",
    ),
];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}
