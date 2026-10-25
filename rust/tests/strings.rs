// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/string-release.bin"),
    include_bytes!("fixtures/string-debug.bin"),
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
fn literals_preserve_utf16_units_and_original_module_load_identity() {
    for bytes in IMAGES {
        for (name, literal) in [
            ("Empty", "\"\""),
            ("Ascii", "\"plain-text\""),
            (
                "Escapes",
                "\"\\u0000\\\"\\\\\\u000D\\u000A\\u0009\\u0008\\u000C\\u000B\\u001F\\u007F\"",
            ),
            ("Units", "\"\\uD800X\\uDFFF\\uD83D\\uDE00\\u4E2D\\uFFFF\""),
            ("Shared", "\"same\""),
            ("SharedAgain", "\"same\""),
        ] {
            let result = decompile(bytes, token(bytes, name)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains(literal), "{name}: {}", result.text);
            assert!(
                result.text.contains("__cil_ldstr_70")
                    && result
                        .text
                        .contains("original module string-literal policy")
            );
            assert!(result.text.is_ascii());
        }
        let result = decompile_type(bytes, "StringCases.Cells").unwrap();
        assert_eq!(result.summary.decompiled, 12); // Includes the core-base constructor.
        assert_eq!(
            result.summary.partial + result.summary.failed + result.summary.metadata_failed,
            0
        );
        for name in ["Identity", "Choose", "Null", "Replace", "SetLiteral"] {
            assert_eq!(
                decompile(bytes, token(bytes, name)).unwrap().status,
                Status::Decompiled
            );
        }
    }
}

#[test]
fn malformed_user_string_keeps_raw_evidence_without_dropping_utf16_bytes() {
    let bytes = IMAGES[0];
    let method = token(bytes, "Ascii");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let body =
        cildec::MethodBody::from_image(&pe, &md.tables().method_def(method & 0xffffff).unwrap())
            .unwrap()
            .unwrap();
    let operand = body
        .instructions()
        .map(Result::unwrap)
        .find(|i| i.opcode.name() == "ldstr")
        .unwrap();
    let cildec::Operand::String(t) = operand.operand else {
        panic!("expected user-string operand")
    };
    let entry = md.user_strings().get(t).unwrap();
    let start = entry.bytes.as_ptr() as usize - bytes.as_ptr() as usize;
    assert_eq!(bytes[start - 1], 21); // Ten UTF-16 units plus terminal flag.
    for (offset, value) in [
        (start - 1, 20),
        (start - 1, 0),
        (start + entry.bytes.len(), 2),
    ] {
        let mut patched = bytes.to_vec();
        patched[offset] = value;
        let result = decompile(&patched, method).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("invalid user-string encoding"));
        assert!(result.text.contains(".code-bytes") && result.text.contains("ldstr"));
        assert!(result.text.contains("utf16-bytes=") && result.text.contains("terminal-flag="));
    }
    let mut patched = bytes.to_vec();
    let code = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    patched[code + 1..code + 5].copy_from_slice(&0x70000000u32.to_le_bytes());
    assert_ne!(
        decompile(&patched, method).unwrap().status,
        Status::Decompiled
    );
}
