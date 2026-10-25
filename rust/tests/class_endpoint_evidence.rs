// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{TableId, Token};
use ilspy_lite::decompile_type;

#[test]
fn malformed_constructor_or_mapping_signature_keeps_endpoint_identity_and_bytes() {
    let bytes = include_bytes!("fixtures/class-release.bin");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let owner = md
        .tables()
        .iter::<cildec::tables::TypeDefRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Attached`1")
        .unwrap()
        .0;
    let attribute = md
        .tables()
        .rows_for_parent(TableId::CustomAttribute, Token(0x02000000 | owner))
        .unwrap()[0];
    let constructor = md.tables().custom_attribute(attribute).unwrap().constructor;
    let mapping = md
        .tables()
        .iter::<cildec::tables::MethodImplRow>()
        .find(|(_, row)| {
            row.as_ref().unwrap().method_declaration.table() == Some(TableId::MemberRef)
        })
        .unwrap()
        .1
        .unwrap();
    for (name, endpoint, marker) in [
        (
            "ClassCases.Attached`1",
            constructor,
            ".attribute-constructor",
        ),
        (
            "ClassCases.ExplicitMethods",
            mapping.method_declaration,
            ".method-impl-declaration",
        ),
    ] {
        let (row, signature, endpoint_name) = if endpoint.table() == Some(TableId::MethodDef) {
            let endpoint_row = md.tables().method_def(endpoint.rid()).unwrap();
            (
                md.tables()
                    .row_bytes(TableId::MethodDef, endpoint.rid())
                    .unwrap(),
                endpoint_row.signature,
                md.strings().str(endpoint_row.name).unwrap(),
            )
        } else {
            let endpoint_row = md.tables().member_ref(endpoint.rid()).unwrap();
            (
                md.tables()
                    .row_bytes(TableId::MemberRef, endpoint.rid())
                    .unwrap(),
                endpoint_row.signature,
                md.strings().str(endpoint_row.name).unwrap(),
            )
        };
        let original = md.blobs().get(signature).unwrap();
        let offset = original.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut patched = bytes.to_vec();
        patched[offset] = 0xff; // Readable blob, invalid method calling convention.
        let result = decompile_type(&patched, name).unwrap();
        // Several attachments can reference the same malformed constructor.
        assert!(result.summary.metadata_failed > 0, "{}", result.text);
        let line = result
            .text
            .lines()
            .find(|l| l.starts_with(&format!("{marker} 0x{:08X} ", endpoint.0)))
            .unwrap();
        assert!(
            line.contains(endpoint_name) && line.contains("owner="),
            "{line}"
        );
        assert!(
            line.contains(&format!("raw-row-bytes={row:02X?}")),
            "{line}"
        );
        assert!(
            line.contains(&format!("signature-blob-index=0x{:08X}", signature.0)),
            "{line}"
        );
        let mut expected = original.to_vec();
        expected[0] = 0xff;
        assert!(
            line.contains(&format!("signature-bytes={expected:02X?}")),
            "{line}"
        );
        assert!(result.text.contains("FAILED metadata"));
        assert!(result.text.contains(&format!("{name}::Read")));
    }
}

#[test]
fn invalid_named_signature_reference_keeps_signature_and_other_identity_parts() {
    let bytes = include_bytes!("fixtures/class-release.bin");
    let md = cildec::PeImage::parse(bytes).unwrap().metadata().unwrap();
    let endpoint = md
        .tables()
        .iter::<cildec::tables::MemberRefRow>()
        .find(|(_, row)| {
            let row = row.as_ref().unwrap();
            md.strings().str(row.name).unwrap() == ".ctor"
                && md.blobs().get(row.signature).unwrap() == [0x20, 1, 1, 0x11, 0x11]
        })
        .unwrap();
    let signature = endpoint.1.unwrap().signature;
    let blob = md.blobs().get(signature).unwrap();
    let offset = blob.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    // TypeDefOrRef TypeRef RID 31 is absent; no signature length changes.
    assert!(md.tables().row_count(TableId::TypeRef) < 31);
    patched[offset + 4] = 0x7d;
    let result = decompile_type(&patched, "ClassCases.ExplicitMethods").unwrap();
    assert_eq!(result.summary.metadata_failed, 1, "{}", result.text);
    let line = result
        .text
        .lines()
        .find(|l| {
            l.starts_with(&format!(
                ".attribute-constructor 0x{:08X} ",
                0x0a000000 | endpoint.0
            ))
        })
        .unwrap();
    assert!(line.contains(".ctor") && line.contains("owner="), "{line}");
    assert!(
        line.contains("signature-bytes=[20, 01, 01, 11, 7D]"),
        "{line}"
    );
    assert!(line.contains("raw-row-bytes="), "{line}");
    assert!(result.text.contains("ClassCases.ExplicitMethods::Read"));
}
