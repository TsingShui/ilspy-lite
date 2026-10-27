// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/token-release.bin"),
    include_bytes!("fixtures/token-debug.bin"),
];

fn token(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap()
        .token
}

fn ldtoken_offset(bytes: &[u8], method: u32) -> usize {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let body =
        cildec::MethodBody::from_image(&pe, &md.tables().method_def(method & 0xffffff).unwrap())
            .unwrap()
            .unwrap();
    let base = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let instruction = body
        .instructions()
        .map(Result::unwrap)
        .find(|i| i.opcode.name() == "ldtoken")
        .expect("fixture ldtoken");
    base + instruction.offset as usize
}

/// The handle is built from the operand row, so a body without dependency facts
/// moves past the opcode and stops at the consumer's own admission instead.
#[test]
fn token_operands_are_no_longer_a_standalone_opcode_blocker() {
    for bytes in IMAGES {
        for (owner, name) in [
            ("TokenCases.Tokens", "TypeName"),
            ("TokenCases.Tokens", "ClosedName"),
            ("TokenCases.Tokens", "NestedName"),
            ("TokenCases.Tokens", "ArrayName"),
        ] {
            let result =
                decompile_with_style(bytes, token(bytes, owner, name), OutputStyle::Readable)
                    .unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(
                !result.text.contains("unsupported opcode ldtoken"),
                "{name}: {}",
                result.text
            );
        }
        let cctor = token(bytes, "TokenCases.Tokens", ".cctor");
        let result = decompile_with_style(bytes, cctor, OutputStyle::Readable).unwrap();
        assert!(
            !result.text.contains("unsupported opcode ldtoken"),
            "{}",
            result.text
        );
    }
}

#[test]
fn function_pointers_are_no_longer_a_standalone_opcode_blocker() {
    for bytes in IMAGES {
        for name in ["StaticDelegate", "InstanceDelegate"] {
            let result = decompile_with_style(
                bytes,
                token(bytes, "TokenCases.Tokens", name),
                OutputStyle::Readable,
            )
            .unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(
                !result.text.contains("unsupported opcode ldftn"),
                "{name}: {}",
                result.text
            );
        }
    }
}

#[test]
fn non_token_operands_and_missing_rows_are_refused() {
    for bytes in IMAGES {
        let method = token(bytes, "TokenCases.Tokens", "ClosedName");
        let at = ldtoken_offset(bytes, method);
        let mut user_string = bytes.to_vec();
        // 0x70xxxxxx is a UserString token, never a type/field/method handle.
        user_string[at + 1..at + 5].copy_from_slice(&0x7000_0001u32.to_le_bytes());
        let result = decompile_with_style(&user_string, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("ldtoken operand is not a type, field or method token"),
            "{}",
            result.text
        );

        let mut missing = bytes.to_vec();
        missing[at + 1..at + 5].copy_from_slice(&0x01ff_ffffu32.to_le_bytes());
        let result = decompile_with_style(&missing, method, OutputStyle::Readable).unwrap();
        // An out-of-range TypeRef is refused while the body evidence is decoded,
        // so a missing row can never reach a recovered handle.
        assert_ne!(result.status, Status::Decompiled, "{}", result.text);
        assert!(
            result.text.contains("missing metadata row")
                || result.text.contains("value out of range"),
            "{}",
            result.text
        );
    }
}
