// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use cildec::{PeImage, TableId};
use ilspy_lite::decompile_type;

fn push16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend(value.to_le_bytes());
}
fn push32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_le_bytes());
}
fn set32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Authored metadata in the owned PE container. 65536 TypeDefs force four-byte
// simple/coded owner columns without a huge source file or executing any input.
fn wide_image() -> Vec<u8> {
    let mut strings = vec![0];
    let mut string = |name: &str| {
        let index = strings.len() as u16;
        strings.extend(name.bytes());
        strings.push(0);
        index
    };
    let module = string("WideOwners");
    let selected = string("Selected");
    let unrelated = string("Unrelated");
    let other = string("Other");
    let method = string("Read");
    let counts = [
        (0u8, 1u32),
        (2, 65536),
        (6, 1),
        (12, 1),
        (14, 1),
        (15, 1),
        (25, 1),
    ];
    let valid = counts.iter().fold(0u64, |v, (table, _)| v | (1 << table));
    let mut tables = vec![0, 0, 0, 0, 2, 0, 0, 1];
    tables.extend(valid.to_le_bytes());
    tables.extend(0u64.to_le_bytes());
    for (_, count) in counts {
        push32(&mut tables, count);
    }
    push16(&mut tables, 0);
    push16(&mut tables, module);
    for _ in 0..3 {
        push16(&mut tables, 0);
    }
    for rid in 1..=65536 {
        push32(&mut tables, if rid == 10 { 0x91 } else { 1 });
        push16(
            &mut tables,
            if rid == 10 {
                selected
            } else if rid == 11 {
                unrelated
            } else {
                other
            },
        );
        push16(&mut tables, 0);
        push32(&mut tables, 0);
        push16(&mut tables, 1);
        push16(&mut tables, if rid <= 10 { 1 } else { 2 });
    }
    push32(&mut tables, 0);
    push16(&mut tables, 0);
    push16(&mut tables, 0x446);
    push16(&mut tables, method);
    push16(&mut tables, 1);
    push16(&mut tables, 1);
    // One ordinary record in each corrected family, initially owned by TypeDef 10.
    push32(&mut tables, (10 << 5) | 3);
    push16(&mut tables, 10);
    push16(&mut tables, 5);
    push16(&mut tables, 2);
    push32(&mut tables, 10 << 2);
    push16(&mut tables, 5);
    push16(&mut tables, 0);
    push32(&mut tables, 4);
    push32(&mut tables, 10);
    push32(&mut tables, 10);
    push16(&mut tables, 2);
    push16(&mut tables, 2);
    let blobs = vec![0, 3, 0x20, 0, 8, 4, 1, 0, 0, 0];
    let streams = [
        ("#~", tables),
        ("#Strings", strings),
        ("#Blob", blobs),
        ("#GUID", vec![]),
    ];
    let mut metadata = Vec::new();
    metadata.extend(0x424a5342u32.to_le_bytes());
    push16(&mut metadata, 1);
    push16(&mut metadata, 1);
    push32(&mut metadata, 0);
    push32(&mut metadata, 12);
    metadata.extend(b"v4.0.30319\0\0");
    push16(&mut metadata, 0);
    push16(&mut metadata, streams.len() as u16);
    let mut headers = Vec::new();
    for (name, data) in &streams {
        let at = metadata.len();
        push32(&mut metadata, 0);
        push32(&mut metadata, data.len() as u32);
        metadata.extend(name.bytes());
        metadata.push(0);
        while metadata.len() % 4 != 0 {
            metadata.push(0);
        }
        headers.push(at);
    }
    for ((_, data), at) in streams.into_iter().zip(headers) {
        while metadata.len() % 4 != 0 {
            metadata.push(0);
        }
        let start = metadata.len() as u32;
        set32(&mut metadata, at, start);
        metadata.extend(data);
    }
    let original = include_bytes!("fixtures/class-release.bin");
    let pe = PeImage::parse(original).unwrap();
    let section = pe.sections().last().unwrap();
    let cli = pe.rva_to_offset(pe.data_directory(14).rva).unwrap();
    let mut image = original.to_vec();
    while image.len() % 512 != 0 {
        image.push(0);
    }
    let start = image.len();
    image.extend(&metadata);
    while image.len() % 512 != 0 {
        image.push(0);
    }
    let pe_offset = u32::from_le_bytes(image[0x3c..0x40].try_into().unwrap()) as usize;
    let optional = pe_offset + 24;
    let optional_size =
        u16::from_le_bytes(image[pe_offset + 20..pe_offset + 22].try_into().unwrap()) as usize;
    let section_header = optional + optional_size + (pe.sections().len() - 1) * 40;
    let size = image.len() as u32 - section.pointer_to_raw_data;
    set32(&mut image, section_header + 8, size);
    set32(&mut image, section_header + 16, size);
    set32(
        &mut image,
        optional + 56,
        (section.virtual_address + size).next_multiple_of(8192),
    );
    set32(
        &mut image,
        cli + 8,
        section.virtual_address + start as u32 - section.pointer_to_raw_data,
    );
    set32(&mut image, cli + 12, metadata.len() as u32);
    image
}

#[test]
fn oversized_four_byte_owners_cannot_alias_an_existing_token() {
    let bytes = wide_image();
    let md = PeImage::parse(&bytes).unwrap().metadata().unwrap();
    assert_eq!(md.tables().row_count(TableId::TypeDef), 65536);
    assert_eq!(
        decompile_type(&bytes, "Selected")
            .unwrap()
            .summary
            .metadata_failed,
        0
    );
    for (table, column_offset, value, marker) in [
        (TableId::ClassLayout, 6, 0x0100000au32, ".class-layout"),
        (TableId::MethodImpl, 0, 0x0100000a, ".method-impl"),
        (
            TableId::CustomAttribute,
            0,
            (0x0100000a << 5) | 3,
            ".custom-attribute",
        ),
        (TableId::DeclSecurity, 2, 0x0100000a << 2, ".decl-security"),
    ] {
        let row = md.tables().row_bytes(table, 1).unwrap();
        let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut patched = bytes.clone();
        set32(&mut patched, offset + column_offset, value);
        for name in ["Selected", "Unrelated"] {
            let result = decompile_type(&patched, name).unwrap();
            assert_eq!(
                result.summary.metadata_failed, 1,
                "{table:?} {name}: {}",
                result.text
            );
            assert_eq!(result.summary.exit_code(), 1);
            assert!(result.text.contains(&format!(
                "{marker} 0x{:08X} ",
                cildec::Token::new(table, 1).0
            )));
            assert!(
                result.text.contains("ownership=unowned-invalid"),
                "{}",
                result.text
            );
            assert!(result.text.contains(&format!(
                "raw-row-bytes={:02X?}",
                &patched[offset..offset + row.len()]
            )));
        }
    }
}
