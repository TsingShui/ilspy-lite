// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/conversion-release.bin"),
    include_bytes!("fixtures/conversion-debug.bin"),
];

#[test]
fn conv_i8_sign_extends_stack_values_without_using_declared_unsignedness() {
    for bytes in IMAGES {
        for name in [
            "ExtendI4",
            "ExtendU4Bits",
            "ExtendNative",
            "ExtendUNativeBits",
            "ExtendSnapshot",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("conv.i8"), "{name}: {}", result.text);
            assert!(
                result.text.contains("unchecked((long)(v"),
                "{name}: {}",
                result.text
            );
        }
    }
}

#[test]
fn conv_i8_identity_and_invalid_source_categories_keep_distinct_results() {
    let bytes = IMAGES[0];
    let token = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "FromI8")
        .unwrap()
        .token;
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(token & 0xffffff).unwrap();
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature, &[0x00, 0x01, 0x08, 0x0a]);
    let signature_start = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.kind, cildec::HeaderKind::Tiny);
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    for (code, status) in [
        (&[0x02, 0x6a, 0x2a][..], Status::Decompiled), // I8 identity
        (&[0x14, 0x6a, 0x2a][..], Status::Partial),    // reference source
        (&[0x6a, 0x2a][..], Status::Partial),          // underflow
        (&[0x02, 0xb9, 0x2a][..], Status::Partial),    // checked conversion separate
    ] {
        assert!(code.len() <= body.code.len());
        let mut patched = bytes.to_vec();
        patched[signature_start + 2] = 0x0a; // Declared long return.
        patched[start - 1] = ((code.len() as u8) << 2) | 2;
        patched[start..start + code.len()].copy_from_slice(code);
        let result = decompile(&patched, token).unwrap();
        assert_eq!(result.status, status, "{}", result.text);
        if status == Status::Decompiled {
            assert!(result.text.contains("long v1 = unchecked((long)(v0));"));
        } else {
            assert!(result.text.contains("IL FALLBACK"));
        }
    }
}

#[test]
fn conv_i4_identity_is_supported_but_reference_and_empty_stack_sources_are_not() {
    let bytes = IMAGES[0];
    let token = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "FromI8")
        .unwrap()
        .token;
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let row = pe
        .metadata()
        .unwrap()
        .tables()
        .method_def(token & 0xffffff)
        .unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.kind, cildec::HeaderKind::Tiny);
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    for (code, status) in [
        (&[0x17, 0x69, 0x2a][..], Status::Decompiled), // I4 identity conversion
        (&[0x14, 0x69, 0x2a][..], Status::Partial),    // reference source
        (&[0x69, 0x2a][..], Status::Partial),          // stack underflow
        (&[0x02, 0xb7, 0x2a][..], Status::Partial),    // checked conversion is separate
    ] {
        assert!(code.len() <= body.code.len());
        let mut patched = bytes.to_vec();
        patched[start - 1] = ((code.len() as u8) << 2) | 2;
        patched[start..start + code.len()].copy_from_slice(code);
        let result = decompile(&patched, token).unwrap();
        assert_eq!(result.status, status, "{}", result.text);
        if status == Status::Partial {
            assert!(result.text.contains("IL FALLBACK"));
        } else {
            assert!(result.text.contains("unchecked((int)(v0))"));
        }
    }
}

#[test]
fn conv_i4_preserves_low_bits_and_evaluation_order() {
    for bytes in IMAGES {
        let methods = list_methods(bytes).unwrap();
        for name in [
            "FromI8",
            "FromU8",
            "FromNative",
            "FromUNative",
            "Snapshot",
            "ReadThenWrite",
        ] {
            let token = methods
                .iter()
                .find(|m| m.simple_name == name)
                .unwrap()
                .token;
            let result = decompile(bytes, token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("// IL_"));
            if name != "Snapshot" {
                assert!(result.text.contains("conv.i4"), "{name}: {}", result.text);
            }
            assert!(
                result.text.contains("unchecked((int)"),
                "{name}: {}",
                result.text
            );
        }
        let result = decompile_type(bytes, "ConversionCases.Cells").unwrap();
        assert_eq!(result.summary.decompiled, 11);
        assert_eq!(result.summary.partial, 1); // Constructor not lowered.
        assert_eq!(result.summary.failed, 0);
        assert_eq!(result.summary.metadata_failed, 0);
    }
}
