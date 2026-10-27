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
fn external_direct_base_constructor_preserves_original_receiver_and_scoped_target() {
    for bytes in IMAGES {
        let ctor = token(bytes, ".ctor");
        let result = decompile(bytes, ctor).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(
            result.text.contains("__cil_ctor_call_0A"),
            "{}",
            result.text
        );
        assert!(result.text.contains("ExternalLibrary.Base::.ctor"));
        assert!(result.text.contains("exact direct-base constructor"));
        assert!(
            result.text.contains("no allocation") && result.text.contains("no added null check")
        );
        assert!(!result.text.contains("__cil_core_object_ctor_"));
        assert!(!result.text.contains("unresolved core type identity"));
        assert!(!result.text.contains("new "));
    }
}

#[test]
fn external_constructor_rejects_null_receivers_callvirt_and_unrelated_owners() {
    for bytes in IMAGES {
        let ctor = token(bytes, ".ctor");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md.tables().method_def(ctor & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let instructions = body.instructions().map(Result::unwrap).collect::<Vec<_>>();
        let load = instructions
            .iter()
            .find(|i| i.opcode.name() == "ldarg.0")
            .unwrap();
        let call = instructions
            .iter()
            .find(|i| i.opcode.name() == "call")
            .unwrap();
        let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut null_receiver = bytes.to_vec();
        null_receiver[start + load.offset as usize] = 0x14; // ldnull as the receiver.
        assert_eq!(
            decompile(&null_receiver, ctor).unwrap().status,
            Status::Partial
        );
        // A reference constructor invoked with callvirt is the same non-virtual call
        // the runtime performs: no dispatch and no added null check.
        let mut virtual_call = bytes.to_vec();
        virtual_call[start + call.offset as usize] = 0x6f;
        let result = decompile(&virtual_call, ctor).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(!result.text.contains("__cil_callvirt_"), "{}", result.text);
        let target = call.operand.token().unwrap();
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::MemberRef, target.rid())
            .unwrap();
        assert_eq!(raw.len(), 6);
        let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        let actor = md
            .tables()
            .iter::<cildec::tables::TypeRefRow>()
            .find(|(_, r)| md.strings().str(r.as_ref().unwrap().type_name).unwrap() == "Actor")
            .unwrap()
            .0;
        let mut patched = bytes.to_vec();
        patched[offset..offset + 2].copy_from_slice(&(((actor << 3) | 1) as u16).to_le_bytes());
        let result = decompile(&patched, ctor).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(!result.text.contains("unresolved core type identity"));
        let ordinary = token(bytes, "DirectConstant");
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(ordinary & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let call = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "call")
            .unwrap();
        let offset =
            body.code.as_ptr() as usize - bytes.as_ptr() as usize + call.offset as usize + 1;
        let mut patched = bytes.to_vec();
        patched[offset..offset + 4].copy_from_slice(&target.0.to_le_bytes());
        let result = decompile(&patched, ordinary).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(
            result.text.contains("outside constructor context"),
            "{}",
            result.text
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
    let result = decompile(&patched, method).unwrap();
    assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    assert!(result.text.contains("__cil_callvirt_"), "{}", result.text);
    assert!(
        !result.text.contains("__cil_memberref_call_"),
        "{}",
        result.text
    );
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
