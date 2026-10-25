// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

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
fn allocation_is_a_typed_zero_initialized_array_at_the_original_instruction() {
    for bytes in IMAGES {
        for (name, element) in [
            ("AllocateSigned", "int"),
            ("AllocateUnsigned", "uint"),
            ("AllocateAfterEffect", "uint"),
            ("AllocateThenEffect", "uint"),
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(
                result.text.contains(&format!(" = new {element}[v")),
                "{}",
                result.text
            );
            assert!(result.text.contains("// element type token 0x01"));
            assert!(result.text.contains("newarr"));
        }
        let result = decompile_type(bytes, "ArrayCases.Cells").unwrap();
        assert_eq!(result.summary.decompiled, 22);
        assert_eq!(result.summary.partial, 0);
        assert_eq!(result.summary.failed + result.summary.metadata_failed, 0);
    }
}

#[test]
fn newarr_checks_element_identity_and_requires_an_i4_length() {
    let bytes = IMAGES[0];
    let methods = list_methods(bytes).unwrap();
    let method = methods
        .iter()
        .find(|m| m.simple_name == "AllocateSigned")
        .unwrap();
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method.token & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    let element = body
        .instructions()
        .map(Result::unwrap)
        .find(|i| i.opcode.name() == "newarr")
        .unwrap()
        .operand
        .token()
        .unwrap()
        .0;
    for (prefix, token) in [
        (&[0x14][..], element),
        (&[][..], element),
        (&[0x02][..], 0),
        (&[0x02][..], method.token),
    ] {
        let mut code = prefix.to_vec();
        code.push(0x8d);
        code.extend(token.to_le_bytes());
        code.push(0x2a);
        let (patched, token) = with_code("AllocateSigned", &code);
        let result = decompile(&patched, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("IL FALLBACK"));
    }
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature, &[0x00, 0x01, 0x1d, 0x08, 0x08]);
    let start = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    for parameter in [0x0a, 0x18] {
        // I8 and native lengths are outside this model.
        let mut patched = bytes.to_vec();
        patched[start + 4] = parameter;
        assert_eq!(
            decompile(&patched, method.token).unwrap().status,
            Status::Partial
        );
    }
    let other = methods
        .iter()
        .find(|m| m.simple_name == "AllocateUnsigned")
        .unwrap();
    let row = md.tables().method_def(other.token & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    let other_element = body
        .instructions()
        .map(Result::unwrap)
        .find(|i| i.opcode.name() == "newarr")
        .unwrap()
        .operand
        .token()
        .unwrap()
        .0;
    let mut code = vec![0x02, 0x8d];
    code.extend(other_element.to_le_bytes());
    code.push(0x2a);
    let (patched, token) = with_code("AllocateSigned", &code);
    assert_eq!(decompile(&patched, token).unwrap().status, Status::Partial); // No uint[] -> int[] return coercion.
}

#[test]
fn array_lengths_use_native_stack_values_and_keep_the_conversion_chain() {
    for bytes in IMAGES {
        for name in [
            "LengthSigned",
            "LengthUnsigned",
            "WideLength",
            "LengthAfterEffect",
            "LengthSnapshot",
            "CountLoop",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("ldlen"), "{name}: {}", result.text);
            assert!(
                result
                    .text
                    .lines()
                    .any(|line| line.trim_start().starts_with("nint v")
                        && line.contains(".LongLength")),
                "{name}: {}",
                result.text
            );
            if name == "CountLoop" {
                for op in ["conv.u8", "conv.i4", "conv.i8"] {
                    assert!(result.text.contains(op), "missing {op}: {}", result.text);
                }
            }
        }
    }
}

#[test]
fn ldlen_requires_an_array_and_explicit_fixed_width_conversion() {
    for code in [
        &[0x16, 0x8e, 0x69, 0x2a][..], // Numeric value is not an array.
        &[0x8e, 0x69, 0x2a][..],       // No array on the stack.
        &[0x02, 0x8e, 0x2a][..],       // Native result cannot silently become I4.
    ] {
        let (bytes, token) = with_code("LengthSigned", code);
        let result = decompile(&bytes, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("IL FALLBACK"));
    }
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
