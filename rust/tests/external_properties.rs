// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, Status, list_methods};

const IMAGES: [(&[u8], &[u8]); 2] = [
    (
        include_bytes!("fixtures/external-property-release.bin"),
        include_bytes!("fixtures/external-property-library-release.bin"),
    ),
    (
        include_bytes!("fixtures/external-property-debug.bin"),
        include_bytes!("fixtures/external-property-library-debug.bin"),
    ),
];

fn context(bytes: &[u8], library: &[u8]) -> DependencyContext {
    let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
    context
        .add_reference("ExternalPropertyLibrary", library.to_vec())
        .unwrap();
    context
}

#[test]
fn metadata_associated_external_properties_keep_dispatch() {
    for (bytes, library) in IMAGES {
        let context = context(bytes, library);
        for method in list_methods(bytes).unwrap() {
            if ![
                "Get",
                "Set",
                "GetFlag",
                "SetFlag",
                "ReadTwice",
                "SetAndGet",
                "SetComputed",
            ]
            .contains(&method.simple_name.as_str())
            {
                continue;
            }
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result = context.decompile(method.token, style).unwrap();
                assert_eq!(
                    result.status,
                    Status::Decompiled,
                    "{}: {}",
                    method.name,
                    result.text
                );
                assert!(
                    result.text.contains("ExternalProperties"),
                    "{}",
                    result.text
                );
                assert!(
                    !result.text.contains(".get_") && !result.text.contains(".set_"),
                    "{}",
                    result.text
                );
            }
        }
    }
}

const EXACT_ONLY: &[&str] = &["Orphan", "Indexed", "Generic", "Interface"];

#[test]
fn unproved_source_properties_keep_exact_calls_and_complete_scan_accounting() {
    for (bytes, library) in IMAGES {
        let ctx = context(bytes, library);
        let mut expected = Vec::new();
        let mut exact = Vec::new();
        for method in list_methods(bytes).unwrap() {
            let result = ctx.decompile(method.token, OutputStyle::Details).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            if EXACT_ONLY.contains(&method.simple_name.as_str()) {
                assert!(result.text.contains("__cil_callvirt_"), "{}", result.text);
                let readable = ctx.decompile(method.token, OutputStyle::Readable).unwrap();
                assert!(!readable.text.contains(").Number"), "{}", readable.text);
                exact.push(method.simple_name.clone());
            }
            expected.push((method.token, result.status));
        }
        assert_eq!(
            exact,
            EXACT_ONLY
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        );
        let mut actual = Vec::new();
        ilspy_lite::scan_methods_with_context(bytes, Some(&ctx), |token, status, _| {
            actual.push((token, status));
            Ok(())
        })
        .unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn unrelated_references_do_not_supply_property_facts() {
    for (bytes, _) in IMAGES {
        let empty = DependencyContext::new(bytes.to_vec()).unwrap();
        let unrelated = context(
            bytes,
            include_bytes!("fixtures/external-call-library-release.bin"),
        );
        for method in list_methods(bytes).unwrap() {
            // Context-bearing calls keep their existing detailed resolution refusal.
            // Adding an unrelated image must not turn it into a property fact.
            let before = empty
                .decompile(method.token, OutputStyle::Readable)
                .unwrap();
            let after = unrelated
                .decompile(method.token, OutputStyle::Readable)
                .unwrap();
            assert_eq!(before.status, Status::Decompiled, "{}", before.text);
            assert!(before.text.contains("__cil_callvirt_"), "{}", before.text);
            assert!(!before.text.contains(").Number"), "{}", before.text);
            assert_eq!(after.status, before.status);
            let body = |text: &str| {
                text.split("// Dependency")
                    .next()
                    .unwrap()
                    .split("// Dependencies")
                    .next()
                    .unwrap()
                    .trim()
                    .to_owned()
            };
            assert_eq!(body(&before.text), body(&after.text), "{}", method.name);
            let bare = ilspy_lite::decompile_with_style(bytes, method.token, OutputStyle::Readable)
                .unwrap();
            assert_eq!(bare.status, Status::Decompiled, "{}", bare.text);
            assert!(bare.text.contains("__cil_callvirt_"), "{}", bare.text);
        }
    }
}

#[test]
fn direct_virtual_accessor_calls_keep_exact_call_notation() {
    for (bytes, library) in IMAGES {
        let token = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "Get")
            .unwrap()
            .token;
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body =
            cildec::MethodBody::from_image(&pe, &md.tables().method_def(token & 0xffffff).unwrap())
                .unwrap()
                .unwrap();
        let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let call = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "callvirt")
            .unwrap();
        let mut patched = bytes.to_vec();
        patched[start + call.offset as usize] = 0x28;
        let result = context(&patched, library)
            .decompile(token, OutputStyle::Readable)
            .unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(
            result.text.contains("__cil_memberref_call_"),
            "{}",
            result.text
        );
        assert!(!result.text.contains(").Number"), "{}", result.text);
    }
}

#[test]
fn property_metadata_is_required_and_must_be_consistent() {
    for (bytes, library) in IMAGES {
        let caller = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "Get")
            .unwrap()
            .token;
        let property_method = list_methods(library)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.declaring_type == "ExternalProperties.Cell" && m.simple_name == "get_Number"
            })
            .unwrap()
            .token;
        let pe = cildec::PeImage::parse(library).unwrap();
        let md = pe.metadata().unwrap();
        let (rid, _) = md
            .tables()
            .iter::<cildec::tables::MethodSemanticsRow>()
            .find(|(_, row)| row.as_ref().unwrap().method.get() == property_method & 0xffffff)
            .unwrap();
        let row = md
            .tables()
            .row_bytes(cildec::TableId::MethodSemantics, rid)
            .unwrap();
        let offset = row.as_ptr() as usize - library.as_ptr() as usize;
        let property = md
            .tables()
            .iter::<cildec::tables::PropertyRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == "Number")
            .unwrap()
            .1
            .unwrap();
        let signature = md.blobs().get(property.type_signature).unwrap();
        assert_eq!(signature, [0x28, 0, 8]);
        let signature_offset = signature.as_ptr() as usize - library.as_ptr() as usize;
        for (label, position, replacement) in [
            ("combined-role", offset, 3),
            ("other-role", offset, 4),
            ("different-property-type", signature_offset + 2, 2),
        ] {
            let mut patched = library.to_vec();
            patched[position] = replacement;
            let result = context(bytes, &patched)
                .decompile(caller, OutputStyle::Readable)
                .unwrap();
            assert_eq!(
                result.status,
                Status::Decompiled,
                "{label}: {}",
                result.text
            );
            assert!(
                result.text.contains("__cil_callvirt_"),
                "{label}: {}",
                result.text
            );
            assert!(
                !result.text.contains(").Number"),
                "{label}: {}",
                result.text
            );
        }
    }
}
