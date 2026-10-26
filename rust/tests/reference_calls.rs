// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT
use ilspy_lite::{DependencyContext, OutputStyle, Status, decompile, list_methods};

const IMAGES: [(&[u8], &[u8]); 2] = [
    (
        include_bytes!("fixtures/reference-call-release.bin"),
        include_bytes!("fixtures/reference-call-library-release.bin"),
    ),
    (
        include_bytes!("fixtures/reference-call-debug.bin"),
        include_bytes!("fixtures/reference-call-library-debug.bin"),
    ),
];

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

const NEGATIVE: fixture_contract::ExpectedPartial = &[
    (
        "ReferenceCallCases.Ops",
        "UnsupportedGvm",
        "external generic instance dispatch needs dependency facts",
    ),
    (
        "ReferenceCallCases.Ops",
        "UnsupportedByRef",
        "byref/modified parameter needs IL fallback",
    ),
];

#[test]
fn forged_array_identity_and_reference_value_kind_stay_refused() {
    for (bytes, library) in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = |name: &str| {
            list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap()
                .token
        };
        let (_, array) = md
            .tables()
            .iter::<cildec::tables::TypeRefRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Array")
            .unwrap();
        let assembly = md
            .tables()
            .assembly_ref(array.unwrap().resolution_scope.rid())
            .unwrap();
        let key = md.blobs().get(assembly.public_key_or_token).unwrap();
        let at = key.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut forged = bytes.to_vec();
        forged[at] ^= 0xff;
        let result = decompile(&forged, method("ArrayClone")).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("incompatible value/receiver"),
            "{}",
            result.text
        );

        let mut wrong_kind = bytes.to_vec();
        let mut changed = 0;
        for (_, row) in md.tables().iter::<cildec::tables::TypeSpecRow>() {
            let blob = md.blobs().get(row.unwrap().signature).unwrap();
            if blob.starts_with(&[0x15, 0x12]) {
                let at = blob.as_ptr() as usize - bytes.as_ptr() as usize;
                wrong_kind[at + 1] = 0x11;
                changed += 1;
            }
        }
        assert!(changed > 0);
        for references in [false, true] {
            let mut context = DependencyContext::new(wrong_kind.clone()).unwrap();
            if references {
                context
                    .add_reference("ReferenceCallLibrary", library.to_vec())
                    .unwrap();
            }
            let result = context
                .decompile(method("Cast"), OutputStyle::Details)
                .unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
        }
        for name in ["IntGet", "Cast"] {
            let row = md.tables().method_def(method(name) & 0xffffff).unwrap();
            let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
            let at = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
            assert_eq!(body.code[0], 2);
            let mut scalar = bytes.to_vec();
            scalar[at] = 0x16;
            let result = decompile(&scalar, method(name)).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
        }
    }
}

#[test]
fn exact_reference_calls_keep_their_instantiated_signatures_without_source_facts() {
    for (bytes, library) in IMAGES {
        fixture_contract::check(bytes, NEGATIVE);
        fixture_contract::check_scan_parity(bytes);
        let mut context = DependencyContext::new(bytes.to_vec()).unwrap();
        context
            .add_reference("ReferenceCallLibrary", library.to_vec())
            .unwrap();
        for method in list_methods(bytes).unwrap() {
            let bare = decompile(bytes, method.token).unwrap();
            let resolved = context
                .decompile(method.token, OutputStyle::Details)
                .unwrap();
            assert_eq!(bare.status, resolved.status, "{}", method.name);
            if ["IntGet", "Interface", "Indexed"].contains(&method.simple_name.as_str()) {
                assert_eq!(bare.status, Status::Decompiled, "{}", bare.text);
                assert!(bare.text.contains("__cil_callvirt_"), "{}", bare.text);
            }
        }
    }
}
