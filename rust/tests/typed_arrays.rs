// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/typed-array-release.bin"),
    include_bytes!("fixtures/typed-array-debug.bin"),
];

#[test]
fn allocation_operands_keep_owner_and_method_variables_distinct() {
    for bytes in IMAGES {
        let methods = list_methods(bytes).unwrap();
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = methods
            .iter()
            .find(|m| m.simple_name == "NewOwner")
            .unwrap();
        let sibling = methods
            .iter()
            .find(|m| m.simple_name == "NewMethod")
            .unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method.token & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let other = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(sibling.token & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let allocation = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "newarr")
            .unwrap();
        let wrong_token = other
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "newarr")
            .unwrap()
            .operand
            .token()
            .unwrap();
        assert_ne!(allocation.operand.token().unwrap(), wrong_token);
        let at =
            body.code.as_ptr() as usize - bytes.as_ptr() as usize + allocation.offset as usize + 1;
        let mut wrong = bytes.to_vec();
        wrong[at..at + 4].copy_from_slice(&wrong_token.0.to_le_bytes());
        let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("incompatible value/receiver types"),
            "{}",
            result.text
        );
    }
}

#[test]
fn element_tokens_and_scalar_storage_widths_are_not_inferred_from_opcode_alone() {
    for bytes in IMAGES {
        for (name, opcode, replacement) in [
            ("ReadPair", "ldelem", 0x01000001_u32),
            ("StorePair", "stelem", 0x01000001),
            ("AddressPair", "ldelema", 0x01000001),
            ("ReadByte", "ldelem.u1", 0),
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let pe = cildec::PeImage::parse(bytes).unwrap();
            let md = pe.metadata().unwrap();
            let body = cildec::MethodBody::from_image(
                &pe,
                &md.tables().method_def(method.token & 0xffffff).unwrap(),
            )
            .unwrap()
            .unwrap();
            let instruction = body
                .instructions()
                .map(Result::unwrap)
                .find(|i| i.opcode.name() == opcode)
                .unwrap();
            let at =
                body.code.as_ptr() as usize - bytes.as_ptr() as usize + instruction.offset as usize;
            let mut wrong = bytes.to_vec();
            if replacement == 0 {
                wrong[at] = 0x96; // ldelem.i8 is not a byte-vector load.
            } else {
                let allocator = list_methods(bytes)
                    .unwrap()
                    .into_iter()
                    .find(|m| m.simple_name == "NewByte")
                    .unwrap();
                let allocation = cildec::MethodBody::from_image(
                    &pe,
                    &md.tables().method_def(allocator.token & 0xffffff).unwrap(),
                )
                .unwrap()
                .unwrap();
                let wrong_token = allocation
                    .instructions()
                    .map(Result::unwrap)
                    .find(|i| i.opcode.name() == "newarr")
                    .unwrap()
                    .operand
                    .token()
                    .unwrap();
                wrong[at + 1..at + 5].copy_from_slice(&wrong_token.0.to_le_bytes());
            }
            let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(
                result.text.contains("location") || result.text.contains("storage width"),
                "{name}: {}",
                result.text
            );
        }
    }
}

#[test]
fn two_byte_array_elements_preserve_signed_short_and_unsigned_character_storage() {
    for bytes in IMAGES {
        for name in ["ReadShort", "StoreShort", "ReadChar", "StoreChar"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn scalar_element_operations_keep_declared_widths_and_float_storage() {
    for bytes in IMAGES {
        for name in [
            "ReadBool",
            "StoreBool",
            "ReadByte",
            "StoreByte",
            "ReadLong",
            "StoreLong",
            "ReadFloat",
            "StoreFloat",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn typed_element_addresses_keep_actual_cells_and_invariant_runtime_type_checks() {
    for bytes in IMAGES {
        for name in [
            "AddressObject",
            "AddressPair",
            "AddressGeneric",
            "ExchangePair",
            "ExchangeGeneric",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn tokenized_element_operations_copy_exact_value_and_generic_elements() {
    for bytes in IMAGES {
        for name in ["ReadPair", "StorePair", "ReadGeneric", "StoreGeneric"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn primitive_vector_allocation_does_not_trust_a_counterfeit_core_scope() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "NewByte")
            .unwrap();
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method.token & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let token = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "newarr")
            .unwrap()
            .operand
            .token()
            .unwrap();
        let owner = md.tables().type_ref(token.rid()).unwrap();
        let assembly = md
            .tables()
            .assembly_ref(owner.resolution_scope.rid())
            .unwrap();
        let key = md.blobs().get(assembly.public_key_or_token).unwrap();
        let at = key.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut wrong = bytes.to_vec();
        wrong[at] ^= 1;
        let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("exact external element kind facts"),
            "{}",
            result.text
        );
    }
}

#[test]
fn vector_allocations_keep_exact_element_identity_without_scalar_evaluation() {
    for bytes in IMAGES {
        for name in [
            "NewByte",
            "NewObject",
            "NewString",
            "NewDouble",
            "NewPair",
            "NewGeneric",
            "NewOwner",
            "NewMethod",
            "NewMixed",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn reference_element_stores_keep_runtime_covariance_checks_at_the_store() {
    for bytes in IMAGES {
        for name in ["StoreObject", "StoreString"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn reference_element_loads_keep_element_identity_and_runtime_array_checks() {
    for bytes in IMAGES {
        for name in ["ReadObject", "ReadString"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn reference_element_opcodes_reject_scalar_vector_storage() {
    for bytes in IMAGES {
        for name in ["ReadObject", "StoreObject"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let pe = cildec::PeImage::parse(bytes).unwrap();
            let md = pe.metadata().unwrap();
            let signature = md
                .blobs()
                .get(
                    md.tables()
                        .method_def(method.token & 0xffffff)
                        .unwrap()
                        .signature,
                )
                .unwrap();
            assert_eq!(&signature[3..5], &[0x1d, 0x1c]);
            let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
            let mut wrong = bytes.to_vec();
            wrong[at + 4] = 0x05; // Byte vector is not reference storage.
            let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(
                result.text.contains("reference vector element"),
                "{}",
                result.text
            );
        }
    }
}

#[test]
fn vector_operations_do_not_treat_named_references_as_vectors() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.simple_name == "ByteLength")
            .unwrap();
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let signature = md
            .blobs()
            .get(
                md.tables()
                    .method_def(method.token & 0xffffff)
                    .unwrap()
                    .signature,
            )
            .unwrap();
        assert_eq!(signature, &[0, 1, 8, 0x1d, 5]);
        let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut wrong = bytes.to_vec();
        wrong[at + 3] = 0x12; // TypeRef[1], not an array signature.
        let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("operation needs an ordinary vector reference"),
            "{}",
            result.text
        );
    }
}

#[test]
fn vector_lengths_need_no_element_value_or_layout_operation() {
    for bytes in IMAGES {
        for name in [
            "ByteLength",
            "ObjectLength",
            "DoubleLength",
            "PairLength",
            "GenericLength",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}
