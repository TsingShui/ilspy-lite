// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/array-release.bin"),
    include_bytes!("fixtures/array-debug.bin"),
];

fn with_code(name: &str, code: &[u8]) -> (Vec<u8>, u32) {
    let bytes = IMAGES[0];
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == name)
        .unwrap();
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let row = pe
        .metadata()
        .unwrap()
        .tables()
        .method_def(method.token & 0xffffff)
        .unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.kind, cildec::HeaderKind::Tiny);
    assert!(code.len() <= body.code.len());
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[start - 1] = ((code.len() as u8) << 2) | 2;
    patched[start..start + code.len()].copy_from_slice(code);
    (patched, method.token)
}

#[test]
fn invalid_array_operands_and_element_addresses_remain_fallback() {
    for (name, code) in [
        ("ReadSigned", vec![0x16, 0x03, 0x94, 0x2a]), // numeric receiver
        ("ReadSigned", vec![0x02, 0x02, 0x94, 0x2a]), // reference index
        ("ReadSigned", vec![0x02, 0x02, 0x58, 0x2a]), // reference arithmetic
        ("Copy", vec![0x02, 0x16, 0x03, 0x9e, 0x2a]), // reference stored as I4
        ("EffectfulStore", vec![0x02, 0x16, 0x8f, 1, 0, 0, 1, 0x2a]), // ldelema
    ] {
        let (bytes, token) = with_code(name, &code);
        let result = decompile(&bytes, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("IL FALLBACK"));
    }
}

#[test]
fn arrays_do_not_become_class_receivers_or_interchangeable_element_types() {
    let bytes = IMAGES[1];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().field(1).unwrap();
    assert_eq!(md.strings().str(row.name).unwrap(), "Values");
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature, &[0x06, 0x1d, 0x09]); // FIELD, SZARRAY, U4
    let start = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    for element in [0x08, 0x0b] {
        // int[] assignment mismatch; unsupported ulong[]
        let mut patched = bytes.to_vec();
        patched[start + 2] = element;
        let method = list_methods(&patched)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "OldThenWrite")
            .unwrap();
        assert_eq!(
            decompile(&patched, method.token).unwrap().status,
            Status::Partial
        );
    }
    let field_token = (0x04000001u32).to_le_bytes();
    let mut code = vec![0x02, 0x7b]; // uint[] is not an ArrayCases.Cells receiver
    code.extend(field_token);
    code.push(0x2a);
    let (patched, token) = with_code("Copy", &code);
    assert_eq!(decompile(&patched, token).unwrap().status, Status::Partial);
}

#[test]
fn arrays_keep_field_snapshots_locals_and_same_type_joins() {
    for bytes in IMAGES {
        for name in ["OldThenWrite", "Choose", "ReadChoice"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            assert!(result.text.contains("uint[]"), "{}", result.text);
        }
    }
}

#[test]
fn null_array_return_is_a_reference_not_an_integer() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "Null")
            .unwrap();
        let result = decompile(bytes, method.token).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    }
}

#[test]
fn integer_array_writes_keep_store_order() {
    for bytes in IMAGES {
        for name in ["WriteSigned", "WriteUnsigned"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            assert!(result.text.contains("v0[v1] = "), "{}", result.text);
        }
    }
}

#[test]
fn integer_array_reads_are_lowered_with_stable_values() {
    for bytes in IMAGES {
        for name in ["ReadSigned", "ReadUnsigned"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            assert!(result.text.contains("v0[v1]"), "{}", result.text);
            assert!(result.text.contains("[] v0 = a0;"), "{}", result.text);
        }
    }
}
