// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/native-release.bin"),
    include_bytes!("fixtures/native-debug.bin"),
];

#[test]
fn native_values_keep_target_width_at_storage_and_call_boundaries() {
    for bytes in IMAGES {
        for name in [
            "Identity",
            "UnsignedIdentity",
            "Choose",
            "Replace",
            "ReplaceUnsigned",
            "ReplaceStatic",
            "Forward",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("nint"), "{}", result.text);
            assert!(!result.text.contains("long v"), "{}", result.text);
            assert!(
                !result.text.lines().any(|line| {
                    let line = line.trim_start();
                    line.starts_with("long v") || (name != "Choose" && line.starts_with("int v"))
                }),
                "{}",
                result.text
            );
        }
        let result = decompile_type(bytes, "NativeCases.Cells").unwrap();
        assert_eq!(result.summary.decompiled, 7);
        assert_eq!(result.summary.partial, 2); // Constructor and native arithmetic.
        assert_eq!(result.summary.failed, 0);
        assert_eq!(result.summary.metadata_failed, 0);
        assert!(result.text.contains("native int @Signed"));
        assert!(result.text.contains("native uint @Unsigned"));
    }
}

#[test]
fn native_arithmetic_and_implicit_width_changes_remain_fallback() {
    let bytes = IMAGES[0];
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "Forward")
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
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    for code in [
        &[0x02, 0x25, 0x58, 0x2a][..],       // native add
        &[0x02, 0x65, 0x2a][..],             // native neg
        &[0x02, 0x25, 0xfe, 0x01, 0x2a][..], // native ceq
        &[0x16, 0x2a][..],                   // I4 returned as native
        &[0x14, 0x2a][..],                   // object returned as native
    ] {
        assert!(code.len() <= body.code.len());
        let mut patched = bytes.to_vec();
        patched[start - 1] = ((code.len() as u8) << 2) | 2;
        patched[start..start + code.len()].copy_from_slice(code);
        let result = decompile(&patched, method.token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("IL FALLBACK"));
    }
}
