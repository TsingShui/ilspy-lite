// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/external-release.bin"),
    include_bytes!("fixtures/external-debug.bin"),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn opaque_external_classes_keep_image_scoped_identity_without_loading_dependencies() {
    for bytes in IMAGES {
        for name in ["Unused", "Identity", "Other"] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("__cil_type_01"));
            assert!(
                result.text.contains("external class token 0x01")
                    && result.text.contains("assembly-ref")
            );
            assert!(result.text.contains("ExternalLibrary"));
        }
        let class = decompile_type(bytes, "ExternalCases.Cells").unwrap();
        assert_eq!(class.summary.metadata_failed, 0);
        assert!(
            class.text.contains("ExternalLibrary.Actor")
                && class.text.contains("ExternalLibrary.Other")
        );
    }
}

#[test]
fn direct_memberref_calls_keep_full_identity_signature_and_no_virtual_dispatch() {
    for bytes in IMAGES {
        for name in ["DirectFlag", "DirectConstant", "DirectVirtual"] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("__cil_memberref_call_0A"));
            assert!(
                result.text.contains("signature-bytes=") && result.text.contains("assembly-ref")
            );
            assert!(
                result
                    .text
                    .contains("exact IL call; no virtual dispatch or added null check")
            );
        }
    }
}

#[test]
fn memberref_calls_do_not_infer_wrong_receiver_or_callvirt_semantics() {
    let bytes = IMAGES[0];
    let method = token(bytes, "DirectFlag");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let body =
        cildec::MethodBody::from_image(&pe, &md.tables().method_def(method & 0xffffff).unwrap())
            .unwrap()
            .unwrap();
    let call = body
        .instructions()
        .map(Result::unwrap)
        .find(|i| i.opcode.name() == "call")
        .unwrap();
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[start] = 0x03; // Actor is not the external Base receiver.
    assert_eq!(decompile(&patched, method).unwrap().status, Status::Partial);
    let mut patched = bytes.to_vec();
    patched[start + call.offset as usize] = 0x6f;
    assert_eq!(decompile(&patched, method).unwrap().status, Status::Partial);
}

#[test]
fn differently_scoped_external_types_are_not_interchangeable() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let method = token(bytes, "Identity");
    let row = md.tables().method_def(method & 0xffffff).unwrap();
    let sig = md.blobs().get(row.signature).unwrap();
    assert_eq!(sig.len(), 6);
    let other = md
        .blobs()
        .get(
            md.tables()
                .method_def(token(bytes, "Other") & 0xffffff)
                .unwrap()
                .signature,
        )
        .unwrap();
    let start = sig.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[start + 3] = other[3]; // Return Other, argument Actor.
    let result = decompile(&patched, method).unwrap();
    assert_eq!(result.status, Status::Partial);
    assert!(result.text.contains("incompatible value/receiver types"));
}
