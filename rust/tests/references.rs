// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, list_methods};
const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/reference-release.bin"),
    include_bytes!("fixtures/reference-debug.bin"),
];

fn replace_body(bytes: &[u8], name: &str, code: &[u8]) -> (Vec<u8>, u32) {
    let token = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ReferenceCases.Cell" && m.simple_name == name)
        .unwrap()
        .token;
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(token & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.kind, cildec::HeaderKind::Tiny);
    assert!(code.len() <= body.code.len());
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[start - 1] = ((code.len() as u8) << 2) | 2;
    patched[start..start + code.len()].copy_from_slice(code);
    (patched, token)
}

#[test]
fn references_cannot_be_used_as_integer_operands_or_fake_field_receivers() {
    for code in [
        vec![0x02, 0x02, 0x58, 0x2a],
        vec![0x02, 0x65, 0x2a],
        vec![0x16, 0x7b, 1, 0, 0, 4, 0x2a],
    ] {
        let (bytes, token) = replace_body(IMAGES[0], "Read", &code);
        let result = decompile(&bytes, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("IL FALLBACK"));
    }
}

#[test]
fn callvirt_is_not_silently_treated_as_direct_call() {
    let methods = list_methods(IMAGES[0]).unwrap();
    let token = methods
        .iter()
        .find(|m| m.simple_name == "get_Count")
        .unwrap()
        .token;
    let mut code = vec![0x02, 0x6f];
    code.extend(token.to_le_bytes());
    code.push(0x2a);
    let (bytes, token) = replace_body(IMAGES[0], "CountTwice", &code);
    assert_eq!(decompile(&bytes, token).unwrap().status, Status::Partial);
}

#[test]
fn null_return_keeps_a_reference_value() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "Null")
            .unwrap();
        assert_eq!(
            decompile(bytes, method.token).unwrap().status,
            Status::Decompiled
        );
    }
}

#[test]
fn reference_joins_keep_the_reference_type() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "Choose")
            .unwrap();
        assert_eq!(
            decompile(bytes, method.token).unwrap().status,
            Status::Decompiled
        );
    }
}

#[test]
fn value_type_receivers_remain_fallback() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ReferenceCases.ValueCell" && m.simple_name == "Read")
            .unwrap();
        assert_eq!(
            decompile(bytes, method.token).unwrap().status,
            Status::Partial
        );
    }
}

#[test]
fn local_class_with_external_base_can_read_its_own_fields() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ReferenceCases.ExternalBase" && m.simple_name == "Read")
            .unwrap();
        assert_eq!(
            decompile(bytes, method.token).unwrap().status,
            Status::Decompiled
        );
    }
}

#[test]
fn direct_instance_calls_and_backing_fields_preserve_identity() {
    for bytes in IMAGES {
        for (owner, name) in [
            ("ReferenceCases.Cell", "get_Count"),
            ("ReferenceCases.Cell", "CountTwice"),
            ("ReferenceCases.Child", "BaseConstant"),
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == owner && m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            assert!(result.text.contains(if name == "get_Count" {
                "__il_field_"
            } else {
                "__cil_direct_"
            }));
        }
    }
}

#[test]
fn instance_field_reads_and_writes_are_lowered() {
    for bytes in IMAGES {
        for name in ["Read", "Credit", "Nested", "StoreAfterRead"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ReferenceCases.Cell" && m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            assert!(result.text.contains("ReferenceCases"));
            assert!(result.text.contains("field token"));
        }
    }
}
