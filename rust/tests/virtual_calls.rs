// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/virtual-call-release.bin"),
    include_bytes!("fixtures/virtual-call-debug.bin"),
];

fn token(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap()
        .token
}

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring).
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "VirtualCallCases.Shape",
        "Describe",
        "managed location of a non-struct slot needs IL fallback",
    ),
    (
        "VirtualCallCases.Ops",
        "UnsupportedConstrained",
        "unsupported opcode constrained.",
    ),
    (
        "VirtualCallCases.Value",
        "ToString",
        "incompatible value/receiver types",
    ),
    (
        "VirtualCallCases.Compute",
        "Sum",
        "expected int[] or uint[] evaluation-stack value",
    ),
];

// Shape.Area is abstract: a legitimate bodyless declaration, not a fallback.
const EXPECTED_DECLARATIONS: &[(&str, &str)] = &[("VirtualCallCases.Shape", "Area")];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check_with_declarations(bytes, EXPECTED_PARTIAL, EXPECTED_DECLARATIONS);
        fixture_contract::check_scan_parity(bytes);
    }
}

#[test]
fn callvirt_retains_null_check_and_target_runtime_dispatch() {
    for bytes in IMAGES {
        let details = decompile(bytes, token(bytes, "VirtualCallCases.Ops", "Area")).unwrap();
        assert_eq!(details.status, Status::Decompiled, "{}", details.text);
        assert!(details.text.contains("__cil_callvirt_"), "{}", details.text);
        assert!(details.text.contains("null check"), "{}", details.text);
        let readable = decompile_with_style(
            bytes,
            token(bytes, "VirtualCallCases.Ops", "Area"),
            OutputStyle::Readable,
        )
        .unwrap();
        assert_eq!(readable.status, details.status);
        assert!(
            readable.text.contains("__cil_callvirt_"),
            "{}",
            readable.text
        );
    }
}

#[test]
fn callvirt_is_not_downgraded_to_a_direct_call_or_a_value_receiver() {
    for bytes in IMAGES {
        let details = decompile(bytes, token(bytes, "VirtualCallCases.Ops", "Area")).unwrap();
        assert!(!details.text.contains("__cil_direct_"), "{}", details.text);
        assert!(
            !details.text.contains("__cil_memberref_call_"),
            "{}",
            details.text
        );
    }
    // Rewriting the opcode to a direct call must drop dispatch, proving the
    // marker is driven by the original instruction, not the resolved target.
    let bytes = IMAGES[0];
    let method = token(bytes, "VirtualCallCases.Ops", "Area");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let callvirt = body
        .code
        .iter()
        .position(|byte| *byte == 0x6f)
        .expect("callvirt opcode");
    let mut patched = bytes.to_vec();
    patched[start + callvirt] = 0x28;
    let result = decompile(&patched, method).unwrap();
    assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    assert!(result.text.contains("__cil_direct_"), "{}", result.text);
    assert!(!result.text.contains("__cil_callvirt_"), "{}", result.text);
}
