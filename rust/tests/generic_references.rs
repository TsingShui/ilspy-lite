// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/generic-reference-release.bin"),
    include_bytes!("fixtures/generic-reference-debug.bin"),
];

#[test]
fn constructed_argument_identity_is_not_just_the_generic_definition_name() {
    let bytes = IMAGES[0];
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == "LocalIdentity")
        .unwrap();
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(method.token & 0xffffff).unwrap();
    let signature = md.blobs().get(row.signature).unwrap();
    assert_eq!(signature.len(), 12);
    assert_eq!(&signature[..4], &[0, 1, 0x15, 0x12]);
    assert_eq!(signature[6], 8);
    let offset = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 6] = 0x0e; // Box<string> LocalIdentity(Box<int>)
    let result = decompile(&patched, method.token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("IL FALLBACK"));
}

// Reuse unrelated attribute storage for static-only malformed metadata. The
// mutated images are passed to the Rust reader, never loaded by the CLR.
fn with_signature(bytes: &[u8], table: cildec::TableId, rid: u32, signature: &[u8]) -> Vec<u8> {
    use cildec::tables::CustomAttributeRow;
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (index, space) = md
        .tables()
        .iter::<CustomAttributeRow>()
        .find_map(|(_, row)| {
            let row = row.unwrap();
            let data = md.blobs().get(row.value).unwrap();
            (data.len() >= signature.len() && data.len() < 128).then_some((row.value.0, data))
        })
        .expect("owned mutation storage missing");
    let blob = space.as_ptr() as usize - bytes.as_ptr() as usize;
    let row = md.tables().row_bytes(table, rid).unwrap();
    let (column, width) = match table {
        cildec::TableId::MethodDef => (10, 14),
        cildec::TableId::Field => (4, 6),
        cildec::TableId::StandAloneSig => (0, 2),
        _ => panic!("unexpected mutation table"),
    };
    assert_eq!(row.len(), width);
    let at = row.as_ptr() as usize - bytes.as_ptr() as usize + column;
    let mut patched = bytes.to_vec();
    patched[blob - 1] = u8::try_from(signature.len()).unwrap();
    patched[blob..blob + signature.len()].copy_from_slice(signature);
    patched[at..at + 2].copy_from_slice(&u16::try_from(index).unwrap().to_le_bytes());
    patched
}

fn selected(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn oversized_generic_definition_rid_cannot_alias_a_valid_constructed_reference() {
    let bytes = IMAGES[0];
    let token = selected(bytes, "LocalIdentity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(token & 0xffffff).unwrap();
    let good = md.blobs().get(row.signature).unwrap();
    assert_eq!(good[4], 8); // Box<T>, TypeDef 2
    let mut bad = good[..4].to_vec();
    bad.extend_from_slice(&[0xc4, 0, 0, 8]);
    bad.extend_from_slice(&good[5..]);
    assert_eq!(
        cildec::MethodSig::parse(&bad).unwrap().return_type.type_,
        cildec::MethodSig::parse(good).unwrap().return_type.type_
    );
    let patched = with_signature(bytes, cildec::TableId::MethodDef, token & 0xffffff, &bad);
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains(".code-bytes"));
}

#[test]
fn trailing_bytes_in_method_field_and_local_signatures_are_not_silently_erased() {
    use cildec::TableId;
    for bytes in IMAGES {
        let token = selected(bytes, "LocalIdentity");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md.tables().method_def(token & 0xffffff).unwrap();
        let mut method = md.blobs().get(row.signature).unwrap().to_vec();
        method.push(8);
        let patched = with_signature(bytes, TableId::MethodDef, token & 0xffffff, &method);
        assert_eq!(decompile(&patched, token).unwrap().status, Status::Partial);

        let read = selected(bytes, "ReadField");
        let row = md.tables().field(2).unwrap();
        let mut field = md.blobs().get(row.signature).unwrap().to_vec();
        field.push(8);
        let patched = with_signature(bytes, TableId::Field, 2, &field);
        assert_eq!(decompile(&patched, read).unwrap().status, Status::Partial);
    }
    let bytes = IMAGES[1];
    let token = selected(bytes, "Snapshot");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(token & 0xffffff).unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    let local = body
        .local_var_sig
        .expect("debug snapshot local signature missing");
    let row = md.tables().stand_alone_sig(local.rid()).unwrap();
    let mut signature = md.blobs().get(row.signature).unwrap().to_vec();
    signature.push(8);
    let patched = with_signature(bytes, TableId::StandAloneSig, local.rid(), &signature);
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains(".code-bytes"));
}

#[test]
fn local_definition_arity_and_class_value_kind_are_checked_before_reference_admission() {
    let bytes = IMAGES[0];
    let token = selected(bytes, "LocalIdentity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().method_def(token & 0xffffff).unwrap();
    let good = md.blobs().get(row.signature).unwrap();
    // A complete, parseable Box<int, string> return with a one-parameter owner.
    let mut bad = good[..5].to_vec();
    bad.extend_from_slice(&[2, 8, 0x0e]);
    bad.extend_from_slice(&good[7..]);
    let patched = with_signature(bytes, cildec::TableId::MethodDef, token & 0xffffff, &bad);
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("arity"));

    let mut bad_kind = good[..6].to_vec();
    bad_kind.extend_from_slice(&[0x15, 0x11, 8, 1, 8]);
    bad_kind.extend_from_slice(&good[7..]);
    let patched = with_signature(
        bytes,
        cildec::TableId::MethodDef,
        token & 0xffffff,
        &bad_kind,
    );
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("class/value kind"));
}

#[test]
fn identical_display_names_do_not_merge_distinct_generic_definitions() {
    use cildec::{TableId, tables::TypeDefRow};
    let bytes = IMAGES[0];
    let token = selected(bytes, "LocalIdentity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let original = md.tables().type_def(2).unwrap();
    let (other_rid, _) = md
        .tables()
        .iter::<TypeDefRow>()
        .find(|(_, row)| {
            md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "OpenOwner`1"
        })
        .expect("owned second generic definition missing");
    let other_row = md.tables().row_bytes(TableId::TypeDef, other_rid).unwrap();
    assert_eq!(other_row.len(), 14);
    let at = other_row.as_ptr() as usize - bytes.as_ptr() as usize;
    let method = md.tables().method_def(token & 0xffffff).unwrap();
    let signature = md.blobs().get(method.signature).unwrap();
    let signature_at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[at + 4..at + 6]
        .copy_from_slice(&u16::try_from(original.type_name.0).unwrap().to_le_bytes());
    patched[at + 6..at + 8].copy_from_slice(
        &u16::try_from(original.type_namespace.0)
            .unwrap()
            .to_le_bytes(),
    );
    patched[signature_at + 4] = u8::try_from(other_rid << 2).unwrap();
    let pe = cildec::PeImage::parse(&patched).unwrap();
    let md = pe.metadata().unwrap();
    let names = cildec::Names::new(&md);
    assert_eq!(
        names.type_def(cildec::Rid::new(2)).unwrap(),
        names.type_def(cildec::Rid::new(other_rid)).unwrap()
    );
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("incompatible value/receiver types"));
}

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring). There
// are no counts; a promotion must drop its row and a regression must add one.
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "GenericReferenceCases.Ops",
        "UnsupportedDispatch",
        "constructed member needs a local definition; external constructed members need dependency facts",
    ),
    (
        "GenericReferenceCases.Ops",
        "UnsupportedValue",
        "constructed value type needs a local definition; external instantiations need dependency facts",
    ),
    (
        "GenericReferenceCases.Ops",
        "UnsupportedVariance",
        "incompatible value/receiver types",
    ),
];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}
