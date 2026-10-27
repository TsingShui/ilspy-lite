// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/float-release.bin"),
    include_bytes!("fixtures/float-debug.bin"),
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
fn signed_integer_conversion_rounds_to_r4_then_remains_a_cli_float_value() {
    for bytes in IMAGES {
        for name in [
            "FromI4",
            "FromU4Bits",
            "FromI8",
            "FromU8Bits",
            "FromNative",
            "FromUNativeBits",
            "ConvertedSnapshot",
        ] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("conv.r4") && result.text.contains("__cil_conv_r4("));
            assert!(result.text.contains("__cil_F v") && result.text.contains("__cil_return_r4("));
            assert!(
                result
                    .text
                    .contains("CLI F precision is target-runtime-defined")
            );
        }
        for name in ["Identity", "Choose", "Replace"] {
            assert_eq!(
                decompile(bytes, token(bytes, name)).unwrap().status,
                Status::Decompiled
            );
        }
        let class = decompile_type(bytes, "FloatCases.Cells").unwrap();
        assert_eq!(class.summary.decompiled, 25);
        assert_eq!(class.summary.partial, 1);
        assert_eq!(class.summary.failed + class.summary.metadata_failed, 0);
        let unbox = decompile(bytes, token(bytes, "RecoveredDoubleUnbox")).unwrap();
        assert_eq!(unbox.status, Status::Decompiled, "{}", unbox.text);
        assert!(unbox.text.contains("(double)"), "{}", unbox.text);
        let native = decompile(bytes, token(bytes, "UnsupportedUnsignedNative")).unwrap();
        assert_eq!(native.status, Status::Partial, "{}", native.text);
        assert!(native.text.contains("IL FALLBACK"));
    }
}

#[test]
fn unsigned_integer_sources_convert_through_the_cli_float() {
    for bytes in IMAGES {
        let result = decompile(bytes, token(bytes, "RecoveredUnsigned")).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("conv.r.un"), "{}", result.text);
        assert!(result.text.contains("__cil_conv_r_u4("), "{}", result.text);
        let native = decompile(bytes, token(bytes, "UnsupportedUnsignedNative")).unwrap();
        assert_eq!(native.status, Status::Partial, "{}", native.text);
        assert!(
            native
                .text
                .contains("conv.r.un source category needs IL fallback"),
            "{}",
            native.text
        );
    }
}

#[test]
fn float_constants_and_division_keep_bits_and_target_runtime_precision() {
    for bytes in IMAGES {
        for (name, bits) in [
            ("Unit", "0x3F800000u"),
            ("NegativeZero", "0x80000000u"),
            ("Largest", "0x7F7FFFFFu"),
            ("Infinite", "0x7F800000u"),
            ("NotANumber", "0xFFC00000u"),
            ("Smallest", "0x00000001u"),
        ] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("__cil_ldc_r4_bits("));
            assert!(result.text.contains(bits), "{name}: {}", result.text);
        }
        for name in ["Divide", "Scale", "DivideSnapshot"] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("__cil_div_F("));
            assert!(
                result
                    .text
                    .contains("CLI F precision is target-runtime-defined")
            );
        }
    }
}

#[test]
fn external_float_call_keeps_the_exact_core_abs_r4_overload() {
    for bytes in IMAGES {
        for name in ["Absolute", "AbsoluteSnapshot"] {
            let method = token(bytes, name);
            let result = decompile(bytes, method).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("__cil_abs_r4_0A"), "{}", result.text);
            assert!(result.text.contains("System.Math::Abs"));
            assert!(
                result.text.contains("__cil_call_arg_r4(")
                    && result.text.contains("__cil_call_result_r4(")
            );
        }
        let method = token(bytes, "Absolute");
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
            .find(|i| i.opcode.name() == "call")
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
        assert_eq!(sig, &[0x00, 0x01, 0x0c, 0x0c]);
        for data in [
            md.strings().str(assembly.name).unwrap().as_bytes(),
            md.blobs().get(assembly.public_key_or_token).unwrap(),
            md.strings().str(owner.type_name).unwrap().as_bytes(),
            md.strings().str(reference.name).unwrap().as_bytes(),
            &sig[2..],
        ] {
            let start = data.as_ptr() as usize - bytes.as_ptr() as usize;
            let mut patched = bytes.to_vec();
            patched[start] ^= 1;
            let result = decompile(&patched, method).unwrap();
            // The exact Abs specialization must never follow a mutated identity.
            // A mutated call may still be admitted as an ordinary external call
            // with the argument/result packing helpers; if it is not, it keeps a
            // stated fallback.
            if result.status == Status::Decompiled {
                assert!(!result.text.contains("__cil_abs_r4_"), "{}", result.text);
            } else {
                assert_eq!(result.status, Status::Partial, "{}", result.text);
                assert!(result.text.contains("IL FALLBACK"));
            }
        }
    }
}

#[test]
fn unsigned_or_mixed_float_division_remains_unsupported() {
    let bytes = IMAGES[0];
    let method = token(bytes, "Divide");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    let division = body
        .instructions()
        .map(Result::unwrap)
        .find(|i| i.opcode.name() == "div")
        .unwrap();
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[start + division.offset as usize] = 0x5c; // div.un not a floating operation.
    assert_eq!(decompile(&patched, method).unwrap().status, Status::Partial);
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature, &[0x00, 0x02, 0x0c, 0x0c, 0x0c]);
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 4] = 0x08; // F / I4 is not an implicit conversion.
    assert_eq!(decompile(&patched, method).unwrap().status, Status::Partial);
}

#[test]
fn conv_r4_does_not_reinterpret_references_or_accept_float_to_integer_conversion() {
    let bytes = IMAGES[0];
    let method = token(bytes, "FromI4");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.kind, cildec::HeaderKind::Tiny);
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    for code in [
        &[0x14, 0x6b, 0x2a][..],
        &[0x6b, 0x2a][..],
        &[0x02, 0x76, 0x2a][..],
    ] {
        let mut patched = bytes.to_vec();
        patched[start - 1] = ((code.len() as u8) << 2) | 2;
        patched[start..start + code.len()].copy_from_slice(code);
        let result = decompile(&patched, method).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("IL FALLBACK"));
    }
    let signature = md.blobs().get(row.signature).unwrap();
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    assert_eq!(signature, &[0x00, 0x01, 0x0c, 0x08]);
    let mut patched = bytes.to_vec();
    patched[offset + 2] = 0x08; // int return, float input.
    patched[offset + 3] = 0x0c;
    patched[start + 1] = 0x69;
    assert_eq!(decompile(&patched, method).unwrap().status, Status::Partial);
}
