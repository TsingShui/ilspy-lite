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
        assert_eq!(class.summary.decompiled, 11);
        assert_eq!(class.summary.partial, 2);
        assert_eq!(class.summary.failed + class.summary.metadata_failed, 0);
        for name in ["UnsupportedUnsigned", "UnsupportedDouble"] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(result.text.contains("IL FALLBACK"));
        }
    }
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
