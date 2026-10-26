// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/constrained-release.bin"),
    include_bytes!("fixtures/constrained-debug.bin"),
];

#[test]
fn constrained_prefixes_cannot_erase_shape_type_or_control_flow_boundaries() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let methods = list_methods(bytes).unwrap();
        let method = methods
            .iter()
            .find(|m| m.declaring_type == "ConstrainedCases.Ops" && m.simple_name == "Order")
            .unwrap();
        let row = md.tables().method_def(method.token & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let instructions = body.instructions().map(Result::unwrap).collect::<Vec<_>>();
        let prefix = instructions
            .iter()
            .find(|i| i.opcode.name() == "constrained.")
            .unwrap();
        let call = instructions
            .iter()
            .find(|i| i.offset == prefix.next_offset())
            .unwrap();
        assert_eq!(call.opcode.name(), "callvirt");
        let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut direct = bytes.to_vec();
        direct[start + call.offset as usize] = 0x28;
        let result = decompile_with_style(&direct, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("immediately following callvirt"),
            "{}",
            result.text
        );

        let mut non_type = bytes.to_vec();
        non_type[start + prefix.offset as usize + 2..start + prefix.offset as usize + 6]
            .copy_from_slice(&0x06000001u32.to_le_bytes());
        let result = decompile_with_style(&non_type, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("TypeDefOrRef"), "{}", result.text);

        let branch = instructions
            .iter()
            .find(|i| matches!(i.operand, cildec::Operand::BranchTarget(_)))
            .unwrap();
        assert_eq!(branch.next_offset() - branch.offset, 2);
        let relative = i8::try_from(call.offset as i64 - branch.next_offset() as i64).unwrap();
        let mut inner = bytes.to_vec();
        inner[start + branch.offset as usize + 1] = relative as u8;
        let result = decompile_with_style(&inner, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("cannot enter between constrained prefix"),
            "{}",
            result.text
        );
    }
}

#[test]
fn a_constraint_cannot_rebind_declaring_type_variables_as_method_variables() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ConstrainedCases.Host`1" && m.simple_name == "Text")
            .unwrap();
        let good = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(good.status, Status::Decompiled, "{}", good.text);
        let row = md.tables().method_def(method.token & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let constraint = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "constrained.")
            .unwrap()
            .operand
            .token()
            .unwrap();
        let spec = md.tables().type_spec(constraint.rid()).unwrap();
        let signature = md.blobs().get(spec.signature).unwrap();
        assert_eq!(signature, [0x13, 0]);
        let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut wrong = bytes.to_vec();
        wrong[at] = 0x1e;
        let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[test]
fn a_null_or_reference_result_joins_without_erasing_its_exact_reference_type() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ConstrainedCases.Ops" && m.simple_name == "MaybeText")
            .unwrap();
        let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    }
}

#[test]
fn byref_locals_keep_invariant_elements_and_do_not_admit_unmanaged_pointers() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ConstrainedCases.Ops" && m.simple_name == "Local")
            .unwrap();
        let row = md.tables().method_def(method.token & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let local = md
            .tables()
            .stand_alone_sig(body.local_var_sig.unwrap().rid())
            .unwrap();
        let signature = md.blobs().get(local.signature).unwrap();
        assert_eq!(signature[0], 0x07);
        assert_eq!(signature[2..4], [0x10, 0x11]);
        let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        let plain = (1..=md.tables().row_count(cildec::TableId::TypeDef))
            .find(|&rid| {
                md.strings()
                    .str(md.tables().type_def(rid).unwrap().type_name)
                    .unwrap()
                    == "Plain"
            })
            .unwrap();
        let mut wrong = bytes.to_vec();
        wrong[at + 4] = u8::try_from(plain << 2).unwrap();
        let result = decompile_with_style(&wrong, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        let mut pointer = bytes.to_vec();
        pointer[at + 2] = 0x0f;
        let result = decompile_with_style(&pointer, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[test]
fn a_byref_local_rebinds_the_location_instead_of_overwriting_its_contents() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ConstrainedCases.Ops" && m.simple_name == "Local")
            .unwrap();
        let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    }
}

#[test]
fn chosen_managed_locations_survive_control_flow_without_reading_their_contents() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ConstrainedCases.Ops" && m.simple_name == "Conditional")
            .unwrap();
        let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("= ref "), "{}", result.text);
    }
}

#[test]
fn ordinary_constrained_calls_keep_the_declared_type_and_managed_receiver() {
    for bytes in IMAGES {
        let methods = list_methods(bytes).unwrap();
        for name in ["Text", "Step", "Copy", "Order", "Slot", "Concrete"] {
            let method = methods
                .iter()
                .find(|m| m.declaring_type == "ConstrainedCases.Ops" && m.simple_name == name)
                .unwrap();
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result = decompile_with_style(bytes, method.token, style).unwrap();
                assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
                assert!(
                    result.text.contains("__cil_constrained_"),
                    "{}",
                    result.text
                );
            }
        }
    }
}
