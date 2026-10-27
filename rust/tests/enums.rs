// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/enum-release.bin"),
    include_bytes!("fixtures/enum-debug.bin"),
];

fn method(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn generated_special_storage_field_keeps_enum_bits_without_requiring_roslyn_spelling() {
    for bytes in IMAGES {
        let token = method(bytes, "EnumCases.S8Ops", "Identity");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let field = md
            .tables()
            .field_range(cildec::Rid::new(2))
            .unwrap()
            .next()
            .unwrap();
        let row = md.tables().field(field).unwrap();
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::Field, field)
            .unwrap();
        let at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        let name = md.strings().str(row.name).unwrap();
        assert_eq!(name, "value__");
        let name_at = name.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut owned = bytes.to_vec();
        owned[at..at + 2].copy_from_slice(&0x0206_u16.to_le_bytes());
        owned[name_at..name_at + 7].copy_from_slice(b"__value");
        let result = decompile_with_style(&owned, token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("underlying=sbyte"), "{}", result.text);
        for flags in [0x0406_u16, 0x0226, 0x1206] {
            let mut invalid = owned.clone();
            invalid[at..at + 2].copy_from_slice(&flags.to_le_bytes());
            let result = decompile_with_style(&invalid, token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
        }
        owned[name_at..name_at + 7].copy_from_slice(b"__valux");
        let result = decompile_with_style(&owned, token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[test]
fn wide_and_native_integer_sources_preserve_narrow_conversion_bits() {
    for bytes in IMAGES {
        for name in [
            "NarrowSignedByte64",
            "NarrowUnsignedByte64",
            "NarrowSignedShort64",
            "NarrowUnsignedShort64",
            "NarrowSignedByteNative",
            "NarrowUnsignedByteNative",
            "NarrowSignedShortNative",
            "NarrowUnsignedShortNative",
        ] {
            let token = method(bytes, "EnumCases.ConversionOps", name);
            let result = decompile_with_style(bytes, token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn narrow_integer_conversions_do_not_infer_float_or_reference_source_categories() {
    for bytes in IMAGES {
        let token = method(bytes, "EnumCases.S8Ops", "UnsupportedNarrow");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let blob = md
            .blobs()
            .get(md.tables().method_def(token & 0xffffff).unwrap().signature)
            .unwrap();
        assert_eq!(blob.last(), Some(&8));
        let at = blob.as_ptr() as usize - bytes.as_ptr() as usize + blob.len() - 1;
        for kind in [0x0c, 0x1c] {
            let mut owned = bytes.to_vec();
            owned[at] = kind;
            let result = decompile_with_style(&owned, token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(
                result.text.contains("source category needs IL fallback"),
                "{}",
                result.text
            );
        }
    }
}

#[test]
fn integer_narrowing_preserves_low_bits_and_the_result_stack_interpretation() {
    for bytes in IMAGES {
        for owner in ["S8Ops", "U8Ops", "S16Ops", "U16Ops"] {
            let token = method(bytes, &format!("EnumCases.{owner}"), "UnsupportedNarrow");
            let result = decompile_with_style(bytes, token, OutputStyle::Readable).unwrap();
            assert_eq!(
                result.status,
                Status::Decompiled,
                "{owner}: {}",
                result.text
            );
        }
    }
}

#[test]
fn assembly_visible_enum_storage_preserves_its_declared_integer_width() {
    for bytes in IMAGES {
        let token = method(bytes, "EnumCases.S8Ops", "Identity");
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let field = md
            .tables()
            .field_range(cildec::Rid::new(2))
            .unwrap()
            .next()
            .unwrap();
        let row = md.tables().field(field).unwrap();
        assert_eq!(md.strings().str(row.name).unwrap(), "value__");
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::Field, field)
            .unwrap();
        let at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut owned = bytes.to_vec();
        owned[at..at + 2].copy_from_slice(&0x0603_u16.to_le_bytes());
        let result = decompile_with_style(&owned, token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("underlying=sbyte"), "{}", result.text);
    }
}

#[test]
fn enum_underlying_field_shape_is_checked_instead_of_guessed() {
    use cildec::{Rid, TableId};
    let bytes = IMAGES[0];
    let token = method(bytes, "EnumCases.S8Ops", "Identity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let fields: Vec<_> = md.tables().field_range(Rid::new(2)).unwrap().collect();
    assert_eq!(fields.len(), 4);
    let row = md.tables().field(fields[0]).unwrap();
    assert_eq!(md.strings().str(row.name).unwrap(), "value__");
    assert_eq!(row.flags, 0x606);
    let raw = md.tables().row_bytes(TableId::Field, fields[0]).unwrap();
    assert_eq!(raw.len(), 6);
    let field_at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    let sig = md.blobs().get(row.signature).unwrap();
    assert_eq!(sig, &[6, 4]);
    let sig_at = sig.as_ptr() as usize - bytes.as_ptr() as usize;
    for kind in [0x02, 0x03, 0x0c, 0x0e, 0x18, 0x1c] {
        let mut patched = bytes.to_vec();
        patched[sig_at + 1] = kind;
        let result = decompile(&patched, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("fixed-width integer"));
    }
    for flags in [
        0x406u16,
        0x626,
        0x8056,
        0x603 | 0x20,
        0x603 | 0x1000,
        0x600,
        0x607,
    ] {
        let mut patched = bytes.to_vec();
        patched[field_at..field_at + 2].copy_from_slice(&flags.to_le_bytes());
        let result = decompile(&patched, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
    let second = md.tables().row_bytes(TableId::Field, fields[1]).unwrap();
    let at = second.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[at..at + 2].copy_from_slice(&0x606u16.to_le_bytes());
    patched[at + 2..at + 4].copy_from_slice(&u16::try_from(row.name.0).unwrap().to_le_bytes());
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("one ordinary instance storage field"));
}

#[test]
fn a_forged_core_name_key_or_content_type_cannot_prove_enum_storage() {
    use cildec::TableId;
    let bytes = IMAGES[0];
    let token = method(bytes, "EnumCases.S8Ops", "Identity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().type_def(2).unwrap();
    let base = md.tables().type_ref(row.extends.rid()).unwrap();
    assert_eq!(md.strings().str(base.type_name).unwrap(), "Enum");
    let scope = md
        .tables()
        .assembly_ref(base.resolution_scope.rid())
        .unwrap();
    let key = md.blobs().get(scope.public_key_or_token).unwrap();
    let key_at = key.as_ptr() as usize - bytes.as_ptr() as usize;
    let name = md.strings().str(base.type_name).unwrap();
    let name_at = name.as_ptr() as usize - bytes.as_ptr() as usize;
    let raw = md
        .tables()
        .row_bytes(TableId::AssemblyRef, base.resolution_scope.rid())
        .unwrap();
    let flags_at = raw.as_ptr() as usize - bytes.as_ptr() as usize + 8;
    for (at, value) in [(key_at, key[0] ^ 0xff), (name_at, b'X'), (flags_at + 1, 2)] {
        let mut patched = bytes.to_vec();
        patched[at] = value;
        let result = decompile(&patched, token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
    let definition = md.tables().row_bytes(TableId::TypeDef, 2).unwrap();
    let at = definition.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    patched[at + 1] &= !1; // not sealed
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Partial, "{}", result.text);
    assert!(result.text.contains("sealed nongeneric"));
}

#[test]
fn exact_module_typeref_aliases_retain_the_local_enum_definition_and_storage() {
    use cildec::TableId;
    let bytes = IMAGES[0];
    let token = method(bytes, "EnumCases.S8Ops", "Identity");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let definition = md.tables().type_def(2).unwrap();
    let original = md.tables().method_def(token & 0xffffff).unwrap();
    let signature = md.blobs().get(original.signature).unwrap();
    assert_eq!(signature, &[0, 1, 0x11, 8, 0x11, 8]);
    let signature_at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
    let raw = md.tables().row_bytes(TableId::TypeRef, 1).unwrap();
    assert_eq!(raw.len(), 6);
    let at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    for (offset, value) in [
        (0, 4),
        (2, definition.type_name.0),
        (4, definition.type_namespace.0),
    ] {
        patched[at + offset..at + offset + 2]
            .copy_from_slice(&u16::try_from(value).unwrap().to_le_bytes());
    }
    patched[signature_at + 3] = 5;
    patched[signature_at + 5] = 5;
    let result = decompile(&patched, token).unwrap();
    assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    assert!(result.text.contains("enum storage token=0x02000002"));
    assert!(result.text.contains("underlying=sbyte"));
}

#[test]
fn enum_scan_records_every_method_with_the_same_admission() {
    for bytes in IMAGES {
        let mut records = Vec::new();
        ilspy_lite::scan_methods(bytes, |token, status, _| {
            records.push((token, status));
            Ok(())
        })
        .unwrap();
        let methods = list_methods(bytes).unwrap();
        assert_eq!(records.len(), methods.len());
        for (definition, (token, status)) in methods.iter().zip(records) {
            assert_eq!(definition.token, token);
            assert_eq!(decompile(bytes, token).unwrap().status, status);
        }
    }
}

#[test]
fn enum_admission_keeps_unknown_external_storage_gated() {
    for bytes in IMAGES {
        let methods = list_methods(bytes).unwrap();
        // The frozen table is the list; a promotion removes its row instead of
        // editing a count.
        for (owner, name, _) in EXPECTED_PARTIAL {
            let method = methods
                .iter()
                .find(|m| m.declaring_type == *owner && m.simple_name == *name)
                .unwrap_or_else(|| panic!("fixture has no {owner}.{name}"));
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(
                result.text.contains(".code-bytes"),
                "{name}: {}",
                result.text
            );
        }
    }
}

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

// Frozen known-unsupported set: (owner, method, stable reason substring). There
// are no counts; a promotion must drop its row and a regression must add one.
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[(
    "EnumCases.External",
    "UnsupportedExternal",
    "enum storage requires an exact local definition; external/value signatures need IL fallback",
)];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}

#[test]
fn enum_unbox_names_the_enum_and_its_storage_conversion() {
    // A cast to the enum alone has the enum type, which does not implicitly
    // convert to the integer the stack carries, so the storage conversion has to
    // be written out; each owner keeps its own storage type name.
    for bytes in IMAGES {
        for (owner, conversion) in [
            ("EnumCases.S32Ops", "((int)((EnumCases.S32)(value)))"),
            ("EnumCases.S64Ops", "((long)((EnumCases.S64)(value)))"),
        ] {
            let readable = decompile_with_style(
                bytes,
                method(bytes, owner, "UnsupportedUnbox"),
                OutputStyle::Readable,
            )
            .unwrap();
            assert_eq!(
                readable.status,
                Status::Decompiled,
                "{owner}: {}",
                readable.text
            );
            assert!(
                readable.text.contains(conversion),
                "{owner}: expected {conversion} in {}",
                readable.text
            );
        }
    }
}
