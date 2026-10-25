// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/address-release.bin"),
    include_bytes!("fixtures/address-debug.bin"),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == name)
        .unwrap()
        .token
}

fn patched_body(bytes: &[u8], token: u32, code: &[u8]) -> Vec<u8> {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let row = pe
        .metadata()
        .unwrap()
        .tables()
        .method_def(token & 0xffffff)
        .unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.kind, cildec::HeaderKind::Tiny);
    assert!(code.len() <= body.code.len());
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[start - 1] = ((code.len() as u8) << 2) | 2;
    patched[start..start + code.len()].copy_from_slice(code);
    patched
}

fn element_token(bytes: &[u8], method: u32) -> cildec::Token {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let row = pe
        .metadata()
        .unwrap()
        .tables()
        .method_def(method & 0xffffff)
        .unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    body.instructions()
        .map(Result::unwrap)
        .find(|i| i.opcode.name() == "ldelema")
        .unwrap()
        .operand
        .token()
        .unwrap()
}

#[test]
fn element_addresses_are_taken_immediately_and_not_read_as_values() {
    for bytes in IMAGES {
        for (name, element) in [
            ("Take", "uint"),
            ("TakeSigned", "int"),
            ("Duplicate", "uint"),
            ("Retarget", "uint"),
        ] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            let address = result
                .text
                .lines()
                .find(|l| l.trim_start().starts_with(&format!("ref {element} v")))
                .unwrap();
            assert!(
                address.contains(" = ref ") && address.contains('['),
                "{address}"
            );
            assert!(
                result.text.contains("// element type token 0x01"),
                "{}",
                result.text
            );
            assert!(result.text.contains("ldelema"));
            assert_eq!(
                result
                    .text
                    .lines()
                    .filter(|l| l.contains(" = ref "))
                    .count(),
                1
            );
        }
        let result = decompile_type(bytes, "AddressCases.Cells").unwrap();
        assert_eq!(result.summary.decompiled, 15);
        assert_eq!(result.summary.partial, 3); // Two ref-return anchors and constructor.
        assert_eq!(result.summary.failed, 0);
        assert_eq!(result.summary.metadata_failed, 0);
    }
}

#[test]
fn indirect_u4_reads_snapshot_the_value_at_the_read_instruction() {
    for bytes in IMAGES {
        for name in [
            "ReadUnsigned",
            "ReadSignedBits",
            "ReadSnapshot",
            "ReadAfterWrite",
            "ReadAfterRetarget",
        ] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("ldind.u4") && result.text.contains("ref "));
            let read = result
                .text
                .lines()
                .find(|l| l.contains("int v") && l.contains("unchecked((int)(v"))
                .unwrap();
            assert!(
                !read.contains('['),
                "Address must be read, not acquired again: {read}"
            );
            if name == "ReadSnapshot" {
                assert!(
                    result.text.find("ldind.u4").unwrap() < result.text.find("stelem.i4").unwrap()
                );
            } else if name == "ReadAfterWrite" {
                assert!(
                    result.text.find("ldind.u4").unwrap() > result.text.find("stelem.i4").unwrap()
                );
            }
        }
    }
}

#[test]
fn indirect_i4_writes_use_the_previously_acquired_address() {
    for bytes in IMAGES {
        for name in [
            "WriteUnsigned",
            "WriteSignedBits",
            "UpdateViaDup",
            "WriteAfterRetarget",
            "WriteAfterCall",
        ] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("stind.i4"));
            let address = result
                .text
                .lines()
                .find(|l| l.contains(" = ref "))
                .unwrap()
                .split_whitespace()
                .nth(2)
                .unwrap();
            let write = result
                .text
                .lines()
                .find(|l| l.trim_start().starts_with(&format!("{address} = ")))
                .unwrap();
            assert!(!write.contains('['), "{write}");
            assert_eq!(
                result
                    .text
                    .lines()
                    .filter(|l| l.contains(" = ref "))
                    .count(),
                1
            );
            if name == "WriteSignedBits" {
                assert!(!write.contains("uint"), "{write}");
            } else {
                assert!(write.contains("unchecked((uint)(v"), "{write}");
            }
        }
    }
}

#[test]
fn indirect_i4_rejects_invalid_destination_and_value_categories() {
    for bytes in IMAGES {
        let method = token(bytes, "Take");
        for code in [
            &[0x17, 0x17, 0x54, 0x2a][..],
            &[0x14, 0x17, 0x54, 0x2a][..],
            &[0x03, 0x17, 0x54, 0x2a][..],
            &[0x54, 0x2a][..],
        ] {
            let result = decompile(&patched_body(bytes, method, code), method).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("IL FALLBACK"));
        }
        let mut code = vec![0x03, 0x04, 0x8f];
        code.extend(element_token(bytes, method).0.to_le_bytes());
        code.extend([0x17, 0x6a, 0x54, 0x2a]); // I8 value is not implicit I4.
        let result = decompile(&patched_body(bytes, method, &code), method).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(result.text.contains("IL FALLBACK"));
    }
}

#[test]
fn indirect_u4_rejects_ordinary_values_and_empty_stack() {
    for bytes in IMAGES {
        let method = token(bytes, "Take");
        for code in [
            &[0x17, 0x4b, 0x26, 0x2a][..],
            &[0x14, 0x4b, 0x26, 0x2a][..],
            &[0x03, 0x4b, 0x26, 0x2a][..],
            &[0x4b, 0x26, 0x2a][..],
        ] {
            let result = decompile(&patched_body(bytes, method, code), method).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("IL FALLBACK"));
        }
    }
}

#[test]
fn address_operations_and_cross_block_lifetimes_stay_outside_the_model() {
    for bytes in IMAGES {
        let method = token(bytes, "Take");
        let mut acquire = vec![0x03, 0x04, 0x8f]; // Array, I4 index, ldelema.
        acquire.extend(element_token(bytes, method).0.to_le_bytes());
        for (tail, reason) in [
            (
                &[0x2b, 0x00, 0x26, 0x2a][..],
                "managed element address crossing a block",
            ),
            (&[0x4a, 0x26, 0x2a][..], "unsupported opcode ldind.i4"),
            (&[0x17, 0x55, 0x2a][..], "unsupported opcode stind.i8"),
            (
                &[0x25, 0x58, 0x26, 0x2a][..],
                "integer operation on non-integer",
            ),
            (&[0x69, 0x26, 0x2a][..], "conv.i4 source category"),
            (&[0x2a][..], "return leaves excess stack"),
        ] {
            let mut code = acquire.clone();
            code.extend(tail);
            let patched = patched_body(bytes, method, &code);
            let result = decompile(&patched, method).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains(reason), "{}", result.text);
            assert!(result.text.contains("IL FALLBACK") && result.text.contains("ldelema"));
        }
        let mut code = vec![0x03, 0x04, 0xfe, 0x1e, 0x8f]; // readonly. ldelema remains unsupported.
        code.extend(element_token(bytes, method).0.to_le_bytes());
        code.extend([0x26, 0x2a]);
        let result = decompile(&patched_body(bytes, method, &code), method).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(result.text.contains("unsupported opcode readonly."));
    }
}

#[test]
fn element_identity_index_and_array_categories_are_checked() {
    for bytes in IMAGES {
        let method = token(bytes, "Take");
        let u4 = element_token(bytes, method).0;
        let i4 = element_token(bytes, token(bytes, "TakeSigned")).0;
        for (prefix, element) in [
            (&[0x03, 0x04][..], i4), // Different declared element.
            (&[0x03, 0x04][..], 0),
            (&[0x03, 0x04][..], method),   // MethodDef is not a type.
            (&[0x17, 0x04][..], u4),       // Integer is not an array.
            (&[0x14, 0x04][..], u4),       // Untyped null is not an admitted array.
            (&[0x03, 0x04, 0x6a][..], u4), // I8 index outside admitted model.
            (&[][..], u4),                 // Stack underflow.
        ] {
            let mut code = prefix.to_vec();
            code.push(0x8f);
            code.extend(element.to_le_bytes());
            code.extend([0x26, 0x2a]);
            let result = decompile(&patched_body(bytes, method, &code), method).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("IL FALLBACK") && result.text.contains("ldelema"));
        }
        let mut code = vec![0x03, 0x04, 0x8f];
        code.extend(0x01ffffff_u32.to_le_bytes());
        code.extend([0x26, 0x2a]);
        let result = decompile(&patched_body(bytes, method, &code), method).unwrap();
        assert_eq!(result.status, Status::Failed, "{}", result.text);
        assert!(result.text.contains("Raw IL bytes (undecoded)"));
    }
}

#[test]
fn primitive_names_do_not_substitute_for_core_scope_identity() {
    for bytes in IMAGES {
        let method = token(bytes, "Take");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md
            .tables()
            .type_ref(element_token(bytes, method).rid())
            .unwrap();
        let assembly = md
            .tables()
            .assembly_ref(row.resolution_scope.rid())
            .unwrap();
        let name = md.strings().str(assembly.name).unwrap();
        assert_eq!(name, "System.Runtime");
        let key = md.blobs().get(assembly.public_key_or_token).unwrap();
        for data in [name.as_bytes(), key] {
            let start = data.as_ptr() as usize - bytes.as_ptr() as usize;
            let mut patched = bytes.to_vec();
            patched[start] ^= 1;
            let result = decompile(&patched, method).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("not an admitted core identity"));
        }
    }
}
