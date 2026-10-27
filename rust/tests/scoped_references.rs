// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/scoped-reference-release.bin"),
    include_bytes!("fixtures/scoped-reference-debug.bin"),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == name)
        .unwrap()
        .token
}

fn signature_offset(bytes: &[u8], method: u32) -> (usize, Vec<u8>) {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method & 0xffffff).unwrap();
    let signature = md.blobs().get(row.signature).unwrap();
    (
        signature.as_ptr() as usize - bytes.as_ptr() as usize,
        signature.to_vec(),
    )
}

#[test]
fn method_and_type_variable_zero_have_distinct_identities() {
    for bytes in IMAGES {
        let method = token(bytes, "Mixed");
        let (offset, signature) = signature_offset(bytes, method);
        assert_eq!(&signature[..5], &[0x10, 1, 1, 0x15, 0x12]);
        assert_eq!(&signature[7..11], &[0x13, 0, 0x1e, 0]);
        let mut patched = bytes.to_vec();
        patched[offset + 9] = 0x13; // Pair<!0,!0> return, Pair<!0,!!0> parameter
        let result = decompile(&patched, method).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("incompatible value/receiver types"));
    }
}

#[test]
fn generic_positions_must_exist_in_their_own_declaring_context() {
    for bytes in IMAGES {
        let method = token(bytes, "Identity");
        let (offset, signature) = signature_offset(bytes, method);
        assert_eq!(&signature[7..9], &[0x1e, 0]);
        for (at, value, reason) in [
            (8, 1, "exceeds declared positions"),
            (7, 0x13, "no owner"),
            (1, 0, "calling convention"),
            (1, 2, "parameter count"),
        ] {
            let mut patched = bytes.to_vec();
            patched[offset + at] = value;
            let result = decompile(&patched, method).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains(reason), "{}", result.text);
        }
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let rid = md.tables().generic_params(cildec::Token(method)).unwrap()[0];
        let row = md
            .tables()
            .row_bytes(cildec::TableId::GenericParam, rid)
            .unwrap();
        let at = row.as_ptr() as usize - bytes.as_ptr() as usize;
        for field in [0, 2] {
            let mut patched = bytes.to_vec();
            patched[at + field] = 1; // Missing position zero or illegal method covariance
            let result = decompile(&patched, method).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("owner/position/flags"));
        }
    }
}

#[test]
fn generic_values_cannot_be_null_integer_operands_or_another_variable_without_proof() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let method = token(bytes, "CopyValue");
    let row = md.tables().method_def(method & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert_eq!(body.code, &[2, 0x2a]);
    let offset = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    for opcode in [0x14, 0x17] {
        // null and I4 one are not unknown T values
        let mut patched = bytes.to_vec();
        patched[offset] = opcode;
        let result = decompile(&patched, method).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("incompatible value/receiver types"));
    }
    let method = token(bytes, "MixedValue");
    let row = md.tables().method_def(method & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    let offset = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset] = 3; // return !!0 as !0
    let result = decompile(&patched, method).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("incompatible value/receiver types"));

    let method = token(bytes, "ChooseValue");
    let row = md.tables().method_def(method & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert!(body.code.len() >= 4);
    let offset = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset..offset + 4].copy_from_slice(&[3, 4, 0x58, 0x2a]); // add two T values
    let result = decompile(&patched, method).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(
        result
            .text
            .contains("integer operation on non-integer value")
    );
}

#[test]
fn equal_generic_spelling_in_different_methods_never_shares_an_alias() {
    for bytes in IMAGES {
        let mut aliases = std::collections::BTreeSet::new();
        for name in ["Identity", "Snapshot", "Null"] {
            let method = token(bytes, name);
            let result = decompile(bytes, method).unwrap();
            assert_eq!(result.status, Status::Decompiled);
            let line = result
                .text
                .lines()
                .find(|l| l.starts_with("// structural signature "))
                .unwrap();
            assert!(line.contains(&format!("method-owner=0x{method:08X}")));
            let alias = line.split_whitespace().nth(3).unwrap();
            assert!(aliases.insert(alias.to_owned()));
        }
    }
}

#[test]
fn scan_class_and_individual_output_share_scoped_admission_and_stable_aliases() {
    for bytes in IMAGES {
        let mut records = Vec::new();
        ilspy_lite::scan_methods(bytes, |token, status, _| {
            records.push((token, status));
            Ok(())
        })
        .unwrap();
        let methods = list_methods(bytes).unwrap();
        assert_eq!(records.len(), methods.len());
        let class = ilspy_lite::decompile_type(bytes, "ScopedReferenceCases.MethodOps").unwrap();
        for (method, (token, status)) in methods.iter().zip(records) {
            assert_eq!(method.token, token);
            let output = decompile(bytes, token).unwrap();
            assert_eq!(output.status, status);
            if method.declaring_type == "ScopedReferenceCases.MethodOps" {
                for line in output
                    .text
                    .lines()
                    .filter(|l| l.starts_with("// structural signature "))
                {
                    assert!(class.text.contains(line));
                }
            }
        }
    }
}

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring). There
// are no counts; a promotion must drop its row and a regression must add one.
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[(
    "ScopedReferenceCases.MethodOps",
    "UnsupportedConstruct",
    "constructed member needs a local definition; external constructed members need dependency facts",
)];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}
