// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT
use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/managed-location-release.bin"),
    include_bytes!("fixtures/managed-location-debug.bin"),
];

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

#[test]
fn byref_returns_keep_the_escape_gate_while_local_aliases_and_joins_are_recovered() {
    for bytes in IMAGES {
        let negatives = vec![(
            "ManagedLocationCases.Ops",
            "UnsupportedReturn",
            "byref return needs escape/lifetime analysis",
        )];
        fixture_contract::check(bytes, &negatives);
        fixture_contract::check_scan_parity(bytes);
    }
}

#[test]
fn location_identity_is_invariant_and_initonly_addresses_keep_exact_field_operations() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = |name: &str| {
            list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ManagedLocationCases.Ops" && m.simple_name == name)
                .unwrap()
                .token
        };
        let aliased = md
            .tables()
            .method_def(method("Aliased") & 0xffffff)
            .unwrap();
        let signature = md.blobs().get(aliased.signature).unwrap();
        assert_eq!(signature, [0, 2, 8, 0x10, 8, 0x10, 8]);
        let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut mismatch = bytes.to_vec();
        mismatch[at + 4] = 0x0a;
        let result = decompile(&mismatch, method("AliasedCaller")).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("managed location element identity differs"),
            "{}",
            result.text
        );

        let kind = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Kind")
            .unwrap()
            .0;
        let row = md.tables().method_def(method("Struct") & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let instruction = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "ldobj")
            .unwrap();
        let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut mismatch = bytes.to_vec();
        let operand = start + instruction.offset as usize + 1;
        mismatch[operand..operand + 4].copy_from_slice(&(0x02000000 | kind).to_le_bytes());
        let result = decompile(&mismatch, method("Struct")).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("location operand differs"),
            "{}",
            result.text
        );

        for (field_name, caller) in [("Number", "Field"), ("GlobalNumber", "Static")] {
            let (rid, row) = md
                .tables()
                .iter::<cildec::tables::FieldRow>()
                .find(|(rid, row)| {
                    let owner = md
                        .tables()
                        .type_of_field(cildec::Rid::new(*rid))
                        .unwrap()
                        .unwrap();
                    let owner = md.tables().type_def(owner.get()).unwrap();
                    md.strings().str(row.as_ref().unwrap().name).unwrap() == field_name
                        && md.strings().str(owner.type_name).unwrap()
                            == if caller == "Field" { "Cell" } else { "Ops" }
                })
                .unwrap();
            let raw = md.tables().row_bytes(cildec::TableId::Field, rid).unwrap();
            let at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
            let mut readonly = bytes.to_vec();
            readonly[at..at + 2].copy_from_slice(&(row.unwrap().flags | 0x20).to_le_bytes());
            let result = decompile(&readonly, method(caller)).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            assert!(
                result.text.contains(if caller == "Field" {
                    "__cil_ldflda_"
                } else {
                    "__cil_ldsflda_"
                }),
                "{}",
                result.text
            );
        }
    }
}

#[test]
fn boolean_indirect_storage_does_not_canonicalize_nonzero_bytes() {
    for bytes in IMAGES {
        let token = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ManagedLocationCases.Ops" && m.simple_name == "Flag")
            .unwrap()
            .token;
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md.tables().method_def(token & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let literal = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "ldc.i4.1")
            .unwrap();
        let offset =
            body.code.as_ptr() as usize - bytes.as_ptr() as usize + literal.offset as usize;
        let mut input = bytes.to_vec();
        input[offset] = 0x18;
        let actual =
            ilspy_lite::decompile_with_style(&input, token, ilspy_lite::OutputStyle::Readable)
                .unwrap();
        assert_eq!(actual.status, Status::Decompiled, "{}", actual.text);
        assert!(actual.text.contains("__cil_bool_store("), "{}", actual.text);
        assert!(!actual.text.contains("!= 0"), "{}", actual.text);
    }
}

#[test]
fn ref_parameters_are_real_locations_and_scalar_acquisition_is_not_a_value_load() {
    for bytes in IMAGES {
        for name in [
            "Update",
            "Aliased",
            "Text",
            "ScalarLocal",
            "ScalarReceiver",
            "ReadonlyInstance",
            "ReadonlyNested",
            "ReadonlyStatic",
            "AliasedCaller",
            "Field",
            "Array",
            "Static",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ManagedLocationCases.Ops" && m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn readable_indirect_reads_remain_value_snapshots_before_alias_writes() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ManagedLocationCases.Ops" && m.simple_name == "Text")
            .unwrap();
        let result = ilspy_lite::decompile_with_style(
            bytes,
            method.token,
            ilspy_lite::OutputStyle::Readable,
        )
        .unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(
            result
                .text
                .lines()
                .any(|line| line.trim().starts_with("string ")),
            "An indirect reference load must be a value snapshot before the alias is written: {}",
            result.text
        );
    }
}

#[test]
fn typed_generic_struct_enum_and_boolean_locations_keep_their_storage_identity() {
    for bytes in IMAGES {
        for name in [
            "Byte",
            "Char",
            "Wide",
            "Flag",
            "Enum",
            "Struct",
            "Generic",
            "Default",
            "Captured",
            "ThrowAfterWrite",
            "DefaultPayload",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ManagedLocationCases.Ops" && m.simple_name == name)
                .unwrap();
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}
