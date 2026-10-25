// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/exception-release.bin"),
    include_bytes!("fixtures/exception-debug.bin"),
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
fn exception_allocation_keeps_exact_target_arguments_and_position() {
    for bytes in IMAGES {
        for name in ["Make", "Literal", "Build", "Discard"] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("__cil_newobj_0A"));
            assert!(
                result
                    .text
                    .contains("System.ArgumentOutOfRangeException::.ctor")
            );
            assert!(
                result
                    .text
                    .contains("fresh object; exact constructor; parameter order retained")
            );
            if name == "Make" {
                assert!(result.text.contains("(v0, v1)"), "{}", result.text);
            }
        }
        assert_eq!(
            decompile(bytes, token(bytes, "Null")).unwrap().status,
            Status::Decompiled
        );
        let class = decompile_type(bytes, "ExceptionCases.Cells").unwrap();
        assert_eq!(class.summary.decompiled, 9); // Includes three throw bodies and core-base constructor.
        assert_eq!(class.summary.partial, 2);
        assert_eq!(class.summary.failed + class.summary.metadata_failed, 0);
    }
}

#[test]
fn throw_keeps_the_existing_object_and_terminates_with_an_empty_stack() {
    for bytes in IMAGES {
        for name in ["Raise", "ThrowExisting", "ThrowNull"] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("throw ") && result.text.contains("throw"));
            assert!(!result.text.contains("return;"));
            if name == "ThrowExisting" {
                assert!(!result.text.contains("__cil_newobj_"));
            }
        }
    }
    let bytes = IMAGES[0];
    let method = token(bytes, "Discard");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let body =
        cildec::MethodBody::from_image(&pe, &md.tables().method_def(method & 0xffffff).unwrap())
            .unwrap()
            .unwrap();
    assert_eq!(body.kind, cildec::HeaderKind::Tiny);
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    for (code, status) in [
        (&[0x17, 0x14, 0x7a][..], Status::Decompiled), // throw clears lower stack slots.
        (&[0x17, 0x7a][..], Status::Partial),          // Integer cannot be thrown.
        (&[0x03, 0x7a][..], Status::Partial), // General/string throw outside admitted subset.
        (&[0x7a][..], Status::Partial),       // Underflow.
    ] {
        let mut patched = bytes.to_vec();
        patched[start - 1] = ((code.len() as u8) << 2) | 2;
        patched[start..start + code.len()].copy_from_slice(code);
        let result = decompile(&patched, method).unwrap();
        assert_eq!(result.status, status, "{}", result.text);
    }
}

#[test]
fn only_the_exact_core_exception_type_and_two_string_constructor_are_admitted() {
    for bytes in IMAGES {
        for name in ["UnsupportedType", "UnsupportedOverload"] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("IL FALLBACK"));
        }
        let method = token(bytes, "Make");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let call = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "newobj")
            .unwrap();
        let reference = md
            .tables()
            .member_ref(call.operand.token().unwrap().rid())
            .unwrap();
        let owner = md.tables().type_ref(reference.class.rid()).unwrap();
        let assembly = md
            .tables()
            .assembly_ref(owner.resolution_scope.rid())
            .unwrap();
        let sig = md.blobs().get(reference.signature).unwrap();
        assert_eq!(sig, &[0x20, 0x02, 0x01, 0x0e, 0x0e]);
        for data in [
            md.strings().str(assembly.name).unwrap().as_bytes(),
            md.blobs().get(assembly.public_key_or_token).unwrap(),
            md.strings().str(owner.type_name).unwrap().as_bytes(),
            sig,
        ] {
            let start = data.as_ptr() as usize - bytes.as_ptr() as usize;
            let mut patched = bytes.to_vec();
            patched[start] ^= 1;
            let result = decompile(&patched, method).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
        }
        let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut patched = bytes.to_vec();
        patched[start] = 0x17; // I4 instead of first string argument.
        assert_eq!(decompile(&patched, method).unwrap().status, Status::Partial);
        let mut patched = bytes.to_vec();
        patched[start + call.offset as usize] = 0x28; // call is not allocation.
        assert_eq!(decompile(&patched, method).unwrap().status, Status::Partial);
    }
}
