// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT
use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/method-instantiation-release.bin"),
    include_bytes!("fixtures/method-instantiation-debug.bin"),
];

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

#[test]
fn instantiated_methods_keep_original_tokens_and_substitute_parameters() {
    for bytes in IMAGES {
        fixture_contract::check(
            bytes,
            &[
                (
                    "MethodInstantiationCases.Ops",
                    "UnsupportedByRef",
                    "byref/modified parameter needs IL fallback",
                ),
                (
                    "MethodInstantiationCases.Ops",
                    "Virtual",
                    "generic virtual dispatch needs runtime lookup-order modeling",
                ),
                (
                    "MethodInstantiationCases.Ops",
                    "ArgumentOrder",
                    "generic virtual dispatch needs runtime lookup-order modeling",
                ),
            ],
        );
        fixture_contract::check_scan_parity(bytes);
        for name in ["Int", "Distinct", "Owner", "Forward", "External"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| {
                    m.declaring_type == "MethodInstantiationCases.Ops" && m.simple_name == name
                })
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            assert!(result.text.contains("2B0000"), "{}", result.text);
        }
    }
}

#[test]
fn argument_count_and_non_generic_targets_are_not_guessed() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "MethodInstantiationCases.Ops" && m.simple_name == "Int")
            .unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method.token & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let call = body.code.iter().position(|b| *b == 0x28).unwrap();
        let target = u32::from_le_bytes(body.code[call + 1..call + 5].try_into().unwrap());
        let row = md.tables().method_spec(target & 0xffffff).unwrap();
        let blob = md.blobs().get(row.instantiation).unwrap();
        assert_eq!(blob, [0x0a, 1, 8]);
        let at = blob.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut void_argument = bytes.to_vec();
        void_argument[at + 2] = 1;
        let result = decompile(&void_argument, method.token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("admitted nonvoid"), "{}", result.text);
        let mut patched = bytes.to_vec();
        patched[at + 1] = 0;
        patched[at - 1] = 2;
        let result = decompile(&patched, method.token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("argument count"), "{}", result.text);

        let raw = md
            .tables()
            .row_bytes(cildec::TableId::MethodSpec, target & 0xffffff)
            .unwrap();
        let at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut patched = bytes.to_vec();
        patched[at..at + 2].copy_from_slice(&((method.token & 0xffffff) as u16 * 2).to_le_bytes());
        let result = decompile(&patched, method.token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("target is not generic"),
            "{}",
            result.text
        );
    }
}
