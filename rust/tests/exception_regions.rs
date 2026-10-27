// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/exception-region-release.bin"),
    include_bytes!("fixtures/exception-region-debug.bin"),
];

const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "ExceptionRegionCases.Ops",
        "Filtered",
        "EH filters need filter-region lowering",
    ),
    // A non-core assembly scope is not an admitted exception identity, so the
    // general-catch admission for System.Object does not widen to other scopes.
    (
        "ExceptionRegionCases.Ops",
        "ForeignException",
        "EH catch type needs an admitted core exception identity",
    ),
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
fn general_catch_discards_only_an_unused_thrown_object() {
    for bytes in IMAGES {
        let method = token(bytes, "GeneralCatch");
        let result = decompile_with_style(bytes, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("catch {"), "{}", result.text);
        assert!(!result.text.contains("catch (object"), "{}", result.text);

        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let handler = &body.exception_handlers[0];
        let instruction = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.offset == handler.handler_offset)
            .unwrap();
        assert_eq!(instruction.opcode.name(), "pop");
        let offset =
            body.code.as_ptr() as usize - bytes.as_ptr() as usize + handler.handler_offset as usize;
        let mut used = bytes.to_vec();
        used[offset] = 0x25; // dup keeps the caught object on the evaluation stack.
        let result = decompile_with_style(&used, method, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("EH general catch needs an unused exception object"),
            "{}",
            result.text
        );
    }
}

#[test]
fn fixture_contract_keeps_unsupported_regions_visible() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}

#[test]
fn recovered_exception_regions_have_runtime_control_flow() {
    for bytes in IMAGES {
        for method in list_methods(bytes).unwrap() {
            if matches!(method.simple_name.as_str(), "Filtered" | "ForeignException") {
                continue;
            }
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(
                result.status,
                Status::Decompiled,
                "{}: {}",
                method.name,
                result.text
            );
            assert!(
                result.text.contains("try {"),
                "{}: {}",
                method.name,
                result.text
            );
            assert!(
                !result.text.contains("__cil_handler_exception"),
                "{}",
                result.text
            );
            assert!(!result.text.contains("__cil_endfinally"), "{}", result.text);
            if method.simple_name == "FinallyOrder" || method.simple_name == "Nested" {
                assert!(result.text.contains("finally {"), "{}", result.text);
            }
            if method.simple_name == "Catch" || method.simple_name == "RethrowIdentity" {
                assert!(result.text.contains("catch ("), "{}", result.text);
            }
        }
    }
}

#[test]
fn invalid_region_transfers_are_not_recovered() {
    for bytes in IMAGES {
        let token = token(bytes, "FinallyOrder");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md.tables().method_def(token & 0x00ff_ffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let leave = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.canonical().name() == "leave")
            .unwrap();
        assert_eq!(leave.opcode.name(), "leave.s");
        let mut patched = bytes.to_vec();
        patched[start + leave.offset as usize] = 0x2b; // br.s cannot skip finally.
        let result = decompile_with_style(&patched, token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("EH plain branch crosses a region boundary"),
            "{}",
            result.text
        );

        let mut patched = bytes.to_vec();
        let target = body.exception_handlers[0].try_offset;
        let delta = i8::try_from(i64::from(target) - i64::from(leave.next_offset())).unwrap();
        patched[start + leave.offset as usize + 1] = delta as u8;
        let result = decompile_with_style(&patched, token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("EH leave must exit to an enclosing region"),
            "{}",
            result.text
        );

        let mut patched = bytes.to_vec();
        let ret = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "ret")
            .unwrap();
        patched[start + ret.offset as usize] = 0xdc; // endfinally outside any handler.
        let result = decompile_with_style(&patched, token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("EH endfinally must terminate its own finally handler"),
            "{}",
            result.text
        );
    }
}

#[test]
fn faults_are_not_silently_rendered_as_finally() {
    for bytes in IMAGES {
        let token = token(bytes, "FinallyOrder");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(token & 0x00ff_ffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let section = (start + body.code.len() + 3) & !3;
        assert_eq!(bytes[section] & 0x3f, 1);
        assert_eq!(bytes[section + 4], 2); // Same flag position in fat/small clauses.
        let mut patched = bytes.to_vec();
        patched[section + 4] = 4; // Fault runs on exceptional exits only.
        let result = decompile_with_style(&patched, token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("EH fault handlers need exceptional-only cleanup"),
            "{}",
            result.text
        );
    }
}
