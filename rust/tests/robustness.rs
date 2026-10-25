// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{MethodBody, PeImage};
use ilspy_lite::{Status, decompile, list_methods};
const IMAGE: &[u8] = include_bytes!("fixtures/release.bin");

// Replace only the IL bytes of Sum. Keep the PE/metadata layout unchanged.
fn with_il(code: &[u8], maxstack: u16) -> (Vec<u8>, u32) {
    let token = list_methods(IMAGE)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "Sum")
        .unwrap()
        .token;
    let pe = PeImage::parse(IMAGE).unwrap();
    let row = pe
        .metadata()
        .unwrap()
        .tables()
        .method_def(token & 0xffffff)
        .unwrap();
    let body = MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert!(code.len() <= body.code.len());
    let start = body.code.as_ptr() as usize - IMAGE.as_ptr() as usize;
    let header = start - 12;
    let mut bytes = IMAGE.to_vec();
    bytes[header + 2..header + 4].copy_from_slice(&maxstack.to_le_bytes());
    bytes[header + 4..header + 8].copy_from_slice(&(code.len() as u32).to_le_bytes());
    bytes[start..start + code.len()].copy_from_slice(code);
    (bytes, token)
}

#[test]
fn malformed_control_flow_and_stack_never_report_success() {
    let cases: &[(&[u8], u16)] = &[
        (&[0x26, 0x2a], 8),                                     // pop with empty stack
        (&[0x16, 0x17, 0x2a], 8),                               // return with excess stack
        (&[0x20, 0, 0, 0, 0, 0x2b, 0xfa], 8),                   // branch into ldc operand
        (&[0x2b, 0x7f], 8),                                     // branch outside code
        (&[0x16], 8),                                           // fall off end
        (&[0x17, 0x2a], 0),                                     // maxstack violation
        (&[0x0e, 0xff, 0x2a], 8),                               // invalid argument
        (&[0x17, 0x2a, 0x00], 8),                               // unreachable trailing instruction
        (&[0x02, 0x2d, 0x03, 0x17, 0x2b, 0x00, 0x16, 0x2a], 8), // join with unequal depths
        (&[0xfe, 0x14, 0x17, 0x2a], 8),                         // tail prefix not erased
        (&[0x16, 0xde, 0x00, 0x2a], 8),                         // leave is not an ordinary branch
    ];
    for &(code, maxstack) in cases {
        let (bytes, token) = with_il(code, maxstack);
        let result = decompile(&bytes, token).unwrap();
        assert_eq!(
            result.status,
            Status::Partial,
            "code={code:02X?}: {}",
            result.text
        );
        assert!(result.text.contains("IL FALLBACK"));
    }
}

#[test]
fn undecodable_instructions_keep_the_raw_method_visible() {
    let (bytes, token) = with_il(&[0xff, 0x2a], 8);
    let result = decompile(&bytes, token)
        .expect("a parsed body must retain raw bytes even when decoding fails");
    assert_ne!(result.status, Status::Decompiled);
    assert!(result.text.contains("FAILED"));
    assert!(result.text.contains("FF, 2A"));
}

#[test]
fn stack_entry_backedge_uses_parallel_copies() {
    // Carry [1,2] into an infinite self-loop. No source-level loop recognition needed.
    let (bytes, token) = with_il(&[0x17, 0x18, 0x2b, 0x00, 0x2b, 0xfe], 8);
    let result = decompile(&bytes, token).unwrap();
    assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    assert!(result.text.contains("int e1_1_0 = s1_0;"));
    assert!(
        result.text.find("e1_1_1 = s1_1;").unwrap() < result.text.find("s1_0 = e1_1_0;").unwrap()
    );
}

fn patched_method(rva: Option<u32>, impl_flags: u16, flags: Option<u16>) -> (Vec<u8>, u32) {
    let token = list_methods(IMAGE)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "Sum")
        .unwrap()
        .token;
    let pe = PeImage::parse(IMAGE).unwrap();
    let md = pe.metadata().unwrap();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::MethodDef, token & 0xffffff)
        .unwrap();
    let start = row.as_ptr() as usize - IMAGE.as_ptr() as usize;
    let mut bytes = IMAGE.to_vec();
    if let Some(rva) = rva {
        bytes[start..start + 4].copy_from_slice(&rva.to_le_bytes());
    }
    bytes[start + 4..start + 6].copy_from_slice(&impl_flags.to_le_bytes());
    if let Some(flags) = flags {
        bytes[start + 6..start + 8].copy_from_slice(&flags.to_le_bytes());
    }
    (bytes, token)
}

#[test]
fn synchronized_and_native_methods_are_not_plain_decompilations() {
    let (bytes, token) = patched_method(None, 0x20, None);
    assert_eq!(decompile(&bytes, token).unwrap().status, Status::Partial);
    let (bytes, token) = patched_method(None, 1, None);
    assert!(decompile(&bytes, token).is_err());
}

#[test]
fn only_legitimate_bodyless_methods_are_declarations() {
    let (bytes, token) = patched_method(Some(0), 0, None);
    assert!(decompile(&bytes, token).is_err());
    let (bytes, token) = patched_method(Some(0), 0, Some(0x2016));
    assert_eq!(
        decompile(&bytes, token).unwrap().status,
        Status::Declaration
    );
}

#[test]
fn corrupt_images_and_tokens_fail_without_panics() {
    for bytes in [&[][..], &IMAGE[..32], &IMAGE[..256]] {
        assert!(list_methods(bytes).is_err());
    }
    for token in [0, 0x06000000, 0x06ffffff, 0x02000001] {
        assert!(decompile(IMAGE, token).is_err());
    }
}
