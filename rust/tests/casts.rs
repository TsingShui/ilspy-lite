// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/cast-release.bin"),
    include_bytes!("fixtures/cast-debug.bin"),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "CastCases.Ops" && m.simple_name == name)
        .unwrap()
        .token
}

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "CastCases.Ops",
        "UnsupportedGeneric",
        "cast needs a local class definition; external/interface casts need more facts",
    ),
    (
        "CastCases.Ops",
        "UnsupportedInterface",
        "callvirt on an opaque external receiver needs dependency facts",
    ),
];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}

#[test]
fn core_object_cast_keeps_a_reference_runtime_check() {
    for bytes in IMAGES {
        let method = token(bytes, "ToStringValue");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let (object, _) = md
            .tables()
            .iter::<cildec::tables::TypeRefRow>()
            .find(|(_, r)| md.strings().str(r.as_ref().unwrap().type_name).unwrap() == "Object")
            .unwrap();
        let row = md.tables().method_def(method & 0xffffff).unwrap();
        let signature = md.blobs().get(row.signature).unwrap();
        assert_eq!(signature, [0, 1, 0x0e, 0x1c]);
        let sig_at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let at = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let opcode = body.code.iter().position(|b| *b == 0x74).unwrap();
        for operation in [0x74, 0x75] {
            let mut patched = bytes.to_vec();
            patched[sig_at + 2] = 0x1c;
            patched[at + opcode] = operation;
            patched[at + opcode + 1..at + opcode + 5]
                .copy_from_slice(&(0x01000000 | object).to_le_bytes());
            let result = decompile(&patched, method).unwrap();
            assert_eq!(
                result.status,
                ilspy_lite::Status::Decompiled,
                "{}",
                result.text
            );
            assert!(
                result.text.contains(if operation == 0x74 {
                    "((object)(object)"
                } else {
                    " as object)"
                }),
                "{}",
                result.text
            );
        }
    }
}

#[test]
fn core_cast_targets_require_the_original_core_identity() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let (_, string) = md
            .tables()
            .iter::<cildec::tables::TypeRefRow>()
            .find(|(_, r)| md.strings().str(r.as_ref().unwrap().type_name).unwrap() == "String")
            .unwrap();
        let scope = string.unwrap().resolution_scope;
        let assembly = md.tables().assembly_ref(scope.rid()).unwrap();
        let key = md.blobs().get(assembly.public_key_or_token).unwrap();
        let key_at = key.as_ptr() as usize - bytes.as_ptr() as usize;
        let name = md.strings().str(assembly.name).unwrap();
        let name_at = name.as_ptr() as usize - bytes.as_ptr() as usize;
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::AssemblyRef, scope.rid())
            .unwrap();
        let flags_at = raw.as_ptr() as usize - bytes.as_ptr() as usize + 8;
        for (at, value) in [(key_at, key[0] ^ 0xff), (name_at, b'X'), (flags_at + 1, 2)] {
            let mut patched = bytes.to_vec();
            patched[at] = value;
            let result = decompile(&patched, token(bytes, "ToStringValue")).unwrap();
            assert_eq!(
                result.status,
                ilspy_lite::Status::Partial,
                "{}",
                result.text
            );
        }
    }
}

#[test]
fn casts_keep_the_runtime_check_instead_of_being_erased() {
    for bytes in IMAGES {
        let downcast = decompile(bytes, token(bytes, "Downcast")).unwrap();
        assert!(
            downcast.text.contains("CastCases.Derived"),
            "{}",
            downcast.text
        );
        let as_form = decompile(bytes, token(bytes, "As")).unwrap();
        assert!(as_form.text.contains(" as "), "{}", as_form.text);
        assert!(!as_form.text.contains("IL FALLBACK"), "{}", as_form.text);
    }
}
