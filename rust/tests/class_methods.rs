// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

fn method(bytes: &[u8], name: &str) -> (u32, usize) {
    let token = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ClassCases.Methods" && m.simple_name == name)
        .unwrap()
        .token;
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::MethodDef, token & 0xffffff)
        .unwrap();
    assert_eq!(row.len(), 14);
    (token, row.as_ptr() as usize - bytes.as_ptr() as usize)
}

#[test]
fn unreadable_method_name_keeps_flags_signature_parameters_and_body() {
    for bytes in IMAGES {
        let (token, offset) = method(bytes, "Optional");
        let mut patched = bytes.to_vec();
        patched[offset + 8..offset + 10].copy_from_slice(&0xffffu16.to_le_bytes());
        let result = decompile_type(&patched, "ClassCases.Methods").unwrap();
        assert_eq!(result.summary.metadata_failed, 1, "{}", result.text);
        assert_eq!(result.summary.exit_code(), 1);
        let line = result
            .text
            .lines()
            .find(|l| l.starts_with(&format!(".method 0x{token:08X} ")))
            .unwrap();
        assert!(
            line.contains("flags=0x0096")
                && line.contains("impl-flags=0x0000")
                && line.contains("rva="),
            "{line}"
        );
        assert!(
            line.contains("name-index=0x0000FFFF")
                && line.contains("signature-bytes=[00, 01, 08, 08]"),
            "{line}"
        );
        assert!(result.text.contains(".param ") && result.text.contains("name=amount"));
        assert!(
            result
                .text
                .contains("type=0x0008 value=int32(-7) bytes=[F9, FF, FF, FF]")
        );
        // A readable body can remain supported independently of an invalid name.
        assert!(
            result
                .text
                .contains(&format!(".method-status 0x{token:08X} "))
        );
        assert!(
            result.text.contains("return v0;") || result.text.contains("Raw IL bytes (undecoded):"),
            "{}",
            result.text
        );
        assert!(result.text.contains("ClassCases.Methods::Pick(int64)"));
    }
}

#[test]
fn unreadable_method_signature_keeps_header_parameters_and_original_body() {
    let bytes = IMAGES[0];
    let (token, offset) = method(bytes, "Hidden");
    let original = ilspy_lite::disassemble(bytes, token).unwrap();
    assert!(original.contains("[02, 2A]"));
    let mut patched = bytes.to_vec();
    patched[offset + 10..offset + 12].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Methods").unwrap();
    let header = result
        .text
        .lines()
        .find(|l| l.starts_with(&format!(".method 0x{token:08X} ")))
        .unwrap();
    assert!(header.contains("ClassCases.Methods::Hidden"), "{header}");
    assert!(header.contains("flags=0x0091"));
    assert!(header.contains("signature-blob-index=0x0000FFFF"));
    assert!(
        result
            .text
            .contains(&format!(".method-status 0x{token:08X} Failed"))
    );
    assert!(
        result.text.contains("Raw IL bytes (undecoded): [02, 2A]"),
        "{}",
        result.text
    );
    assert!(result.text.contains(&format!(
        "method=0x{token:08X} sequence=1 flags=0x0000 name=value"
    )));
    assert!(result.text.contains("ClassCases.Methods::Pick(int64)"));
    assert_eq!(result.summary.failed, 1);
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
}

#[test]
fn invalid_parameter_name_keeps_header_constant_and_later_parameters() {
    let bytes = IMAGES[0];
    let (token, _) = method(bytes, "Optional");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md
        .tables()
        .method_def(token & 0xffffff)
        .unwrap()
        .param_list
        .get();
    let row = md.tables().row_bytes(cildec::TableId::Param, rid).unwrap();
    assert_eq!(row.len(), 6);
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 4..offset + 6].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Methods").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert!(
        result
            .text
            .contains(&format!(".method-status 0x{token:08X} Decompiled"))
    );
    assert!(result.text.contains("<unreadable name-index=0x0000FFFF>"));
    assert!(
        result
            .text
            .contains("type=0x0008 value=int32(-7) bytes=[F9, FF, FF, FF]")
    );
    assert!(result.text.contains("name=output"));
}

#[test]
fn duplicate_parameter_sequences_are_visible_not_normalized() {
    let bytes = IMAGES[0];
    let (token, _) = method(bytes, "ByRef");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md
        .tables()
        .method_def(token & 0xffffff)
        .unwrap()
        .param_list
        .get();
    let row = md
        .tables()
        .row_bytes(cildec::TableId::Param, rid + 1)
        .unwrap();
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[offset + 2..offset + 4].copy_from_slice(&1u16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Methods").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert!(
        result
            .text
            .contains("invalid or duplicate parameter sequence")
    );
    assert!(result.text.contains("sequence=1 flags=0x0002 name=output"));
}

#[test]
fn overlapping_parameter_ownership_is_rejected_before_class_output() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (token, _) = method(bytes, "Hidden");
    let start = md
        .tables()
        .method_def(token & 0xffffff)
        .unwrap()
        .param_list
        .get();
    let mut patched = bytes.to_vec();
    for (step, boundary) in [(0, start), (1, start + 2), (2, start + 1)] {
        let row = md
            .tables()
            .row_bytes(cildec::TableId::MethodDef, (token & 0xffffff) + step)
            .unwrap();
        assert_eq!(row.len(), 14);
        let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
        patched[offset + 12..offset + 14].copy_from_slice(&(boundary as u16).to_le_bytes());
    }
    assert!(decompile_type(&patched, "ClassCases.Methods").is_err());
}

#[test]
fn optional_parameters_and_native_import_targets_keep_metadata() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Methods").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        assert!(
            result.text.contains("sequence=1 flags=0x1010 name=amount"),
            "{}",
            result.text
        );
        assert!(
            result
                .text
                .contains("type=0x0008 value=int32(-7) bytes=[F9, FF, FF, FF]"),
            "{}",
            result.text
        );
        assert!(result.text.contains(".pinvoke "), "{}", result.text);
        assert!(result.text.contains("name=native_count"), "{}", result.text);
        assert!(
            result.text.contains("module=fixture-native"),
            "{}",
            result.text
        );
        assert!(result.text.contains("preservesig"), "{}", result.text);
    }
}

#[test]
fn class_methods_keep_complete_headers_parameters_and_statuses() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Methods").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        let methods: Vec<_> = list_methods(bytes)
            .unwrap()
            .into_iter()
            .filter(|m| m.declaring_type == "ClassCases.Methods")
            .collect();
        assert_eq!(methods.len(), 8);
        for method in &methods {
            assert!(
                result
                    .text
                    .lines()
                    .any(|l| l.starts_with(&format!(".method 0x{:08X} ", method.token))),
                "missing header: {}",
                result.text
            );
            assert!(
                result
                    .text
                    .contains(&format!(".method-status 0x{:08X} ", method.token)),
                "{}",
                result.text
            );
        }
        assert!(
            result
                .text
                .contains("private static hidebysig int32 ClassCases.Methods::Hidden(int32)"),
            "{}",
            result.text
        );
        assert!(result.text.contains("ClassCases.Methods::Pick(int64)"));
        assert!(
            result
                .text
                .contains("ClassCases.Methods::ByRef(int32&, int32&)")
        );
        assert!(result.text.contains("sequence=1 flags=0x0000 name=input"));
        assert!(result.text.contains("sequence=2 flags=0x0002 name=output"));
        assert_eq!(result.summary.declarations, 2);
        assert!(result.text.contains(".ctor"));
        assert!(!result.text.contains("IL FALLBACK"));
        assert_eq!(result.summary.partial, 0);
        assert_eq!(result.summary.exit_code(), 0);
    }
}
