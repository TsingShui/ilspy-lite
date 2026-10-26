// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/constructor-release.bin"),
    include_bytes!("fixtures/constructor-debug.bin"),
];

fn method(bytes: &[u8], name: &str, parameter_count: usize) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| {
            m.declaring_type == "ConstructorCases.Cells"
                && m.simple_name == name
                && m.name
                    .rsplit_once('(')
                    .unwrap()
                    .1
                    .trim_end_matches(')')
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .count()
                    == parameter_count
        })
        .unwrap()
        .token
}

fn patch_body(bytes: &[u8], token: u32, code: &[u8]) -> Vec<u8> {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let row = pe
        .metadata()
        .unwrap()
        .tables()
        .method_def(token & 0xffffff)
        .unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    assert!(code.len() <= body.code.len());
    let start = body.code.as_ptr() as usize - bytes.as_ptr() as usize;
    let mut patched = bytes.to_vec();
    match body.kind {
        cildec::HeaderKind::Tiny => patched[start - 1] = ((code.len() as u8) << 2) | 2,
        cildec::HeaderKind::Fat => {
            let flags = u16::from_le_bytes(bytes[start - 12..start - 10].try_into().unwrap());
            assert_eq!(flags >> 12, 3);
            patched[start - 8..start - 4].copy_from_slice(&(code.len() as u32).to_le_bytes());
        }
    }
    patched[start..start + code.len()].copy_from_slice(code);
    patched
}

fn field(bytes: &[u8], name: &str) -> u32 {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    (1..=md.tables().row_count(cildec::TableId::Field))
        .find(|&rid| {
            md.strings()
                .str(md.tables().field(rid).unwrap().name)
                .unwrap()
                == name
        })
        .map(|rid| 0x04000000 | rid)
        .unwrap()
}

#[test]
fn constructor_body_has_distinct_identity_and_allows_own_readonly_initialization() {
    for bytes in IMAGES {
        let ctor = method(bytes, ".ctor", 2);
        let mut code = vec![0x02, 0x04, 0x7d]; // Original receiver, value, own initonly field.
        code.extend(field(bytes, "Initial").to_le_bytes());
        code.push(0x2a);
        let patched = patch_body(bytes, ctor, &code);
        let result = decompile(&patched, ctor).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(
            result
                .text
                .contains(&format!("public static void __cil_ctor_body_{ctor:08X}(")),
            "{}",
            result.text
        );
        assert!(
            result.text.contains("constructor initialization body")
                && result.text.contains(".@Initial = ")
        );
        let class = decompile_type(&patched, "ConstructorCases.Cells").unwrap();
        assert!(
            class
                .text
                .contains("ConstructorCases.Cells::.ctor(int32, int32)")
        );
        assert!(
            class
                .text
                .contains(&format!(".method-status 0x{ctor:08X} Decompiled"))
        );
        assert!(class.text.contains("initonly"));
    }
}

#[test]
fn readonly_writes_outside_the_owning_constructor_are_rejected() {
    for bytes in IMAGES {
        let ordinary = method(bytes, "Reserved", 1);
        let mut code = vec![0x02, 0x03, 0x7d];
        code.extend(field(bytes, "Initial").to_le_bytes());
        code.extend([0x03, 0x2a]);
        let result = decompile(&patch_body(bytes, ordinary, &code), ordinary).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(
            result
                .text
                .contains("readonly field write needs IL fallback")
        );
        let ctor = method(bytes, ".ctor", 2);
        let mut code = vec![0x02, 0x04, 0x7d];
        code.extend(field(bytes, "BaseValue").to_le_bytes()); // Derived constructor is not the field's owner.
        code.push(0x2a);
        let result = decompile(&patch_body(bytes, ctor, &code), ctor).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(
            result
                .text
                .contains("readonly field write needs IL fallback")
        );
        let mut code = vec![0x03, 0x80]; // Ordinary stsfld on static initonly is not initialization.
        code.extend(field(bytes, "StaticValue").to_le_bytes());
        code.extend([0x03, 0x2a]);
        let result = decompile(&patch_body(bytes, ordinary, &code), ordinary).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}

#[test]
fn readonly_permission_does_not_follow_another_receiver_or_a_reassigned_this_slot() {
    for bytes in IMAGES {
        let ctor = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.declaring_type == "ConstructorCases.Cells"
                    && m.simple_name == ".ctor"
                    && m.name.contains("class ConstructorCases.Cells")
            })
            .unwrap()
            .token;
        let initial = field(bytes, "Initial");
        for prefix in [
            &[0x03, 0x04][..],                   // Other receiver argument.
            &[0x03, 0x10, 0x00, 0x02, 0x04][..], // starg.0 then reload.
            &[0x0f, 0x00, 0x26, 0x02, 0x04][..], // A this-slot address needs original-receiver provenance.
        ] {
            let mut code = prefix.to_vec();
            code.push(0x7d);
            code.extend(initial.to_le_bytes());
            code.push(0x2a);
            let result = decompile(&patch_body(bytes, ctor, &code), ctor).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("IL FALLBACK"));
        }
        let code = [0x02, 0x04, 0x7d]
            .into_iter()
            .chain(initial.to_le_bytes())
            .chain([0x2a])
            .collect::<Vec<_>>();
        let mut patched = patch_body(bytes, ctor, &code);
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md
            .tables()
            .row_bytes(cildec::TableId::MethodDef, ctor & 0xffffff)
            .unwrap();
        let start = row.as_ptr() as usize - bytes.as_ptr() as usize;
        for clear in [0x0800u16, 0x1000u16] {
            // A .ctor name alone does not confer constructor permission.
            let flags = u16::from_le_bytes(row[6..8].try_into().unwrap());
            patched[start + 6..start + 8].copy_from_slice(&(flags & !clear).to_le_bytes());
            let result = decompile(&patched, ctor).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("invalid instance constructor context"));
        }
    }
}

#[test]
fn exact_core_object_constructor_call_keeps_initialization_order_and_target() {
    for bytes in IMAGES {
        let root = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.declaring_type == "ConstructorCases.Root"
                    && m.simple_name == ".ctor"
                    && m.name.ends_with("(int32)")
            })
            .unwrap()
            .token;
        let result = decompile(bytes, root).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("__cil_core_object_ctor_0A"));
        assert!(result.text.contains("System.Object::.ctor"));
        assert!(result.text.contains("no added null check"));
        let call = result.text.find("__cil_core_object_ctor_0A").unwrap();
        assert!(result.text.find(".@Before = ").unwrap() < call);
        assert!(result.text.find(" = new uint[").unwrap() < call);
        assert!(result.text.find(".@After = ").unwrap() > call);
    }
}

#[test]
fn core_clock_read_keeps_exact_target_and_original_call_position() {
    for bytes in IMAGES {
        let clock = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ConstructorCases.Root" && m.simple_name == "ClockValue")
            .unwrap()
            .token;
        let result = decompile(bytes, clock).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(
            result.text.contains("__cil_tick_count_0A"),
            "{}",
            result.text
        );
        assert!(result.text.contains("System.Environment::get_TickCount"));
        assert!(
            result
                .text
                .contains("signed I4 clock read at this instruction")
        );
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let body =
            cildec::MethodBody::from_image(&pe, &md.tables().method_def(clock & 0xffffff).unwrap())
                .unwrap()
                .unwrap();
        let call = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "call")
            .unwrap()
            .operand
            .token()
            .unwrap();
        let member = md.tables().member_ref(call.rid()).unwrap();
        let owner = md.tables().type_ref(member.class.rid()).unwrap();
        let assembly = md
            .tables()
            .assembly_ref(owner.resolution_scope.rid())
            .unwrap();
        let sig = md.blobs().get(member.signature).unwrap();
        assert_eq!(sig, &[0x00, 0x00, 0x08]);
        for data in [
            md.strings().str(assembly.name).unwrap().as_bytes(),
            md.blobs().get(assembly.public_key_or_token).unwrap(),
            md.strings().str(owner.type_name).unwrap().as_bytes(),
            sig,
        ] {
            let mut patched = bytes.to_vec();
            let start = data.as_ptr() as usize - bytes.as_ptr() as usize;
            patched[start] ^= 1;
            let result = decompile(&patched, clock).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("IL FALLBACK"));
        }
    }
}

#[test]
fn core_constructor_admission_rejects_spoofs_wrong_calls_and_receiver_contexts() {
    for bytes in IMAGES {
        let methods = list_methods(bytes).unwrap();
        let base = methods
            .iter()
            .find(|m| m.declaring_type == "ConstructorCases.Base" && m.simple_name == ".ctor")
            .unwrap()
            .token;
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md.tables().method_def(base & 0xffffff).unwrap();
        let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
        let call = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "call")
            .unwrap();
        let target = call.operand.token().unwrap();
        let reference = md.tables().member_ref(target.rid()).unwrap();
        let type_ref = md.tables().type_ref(reference.class.rid()).unwrap();
        let assembly = md
            .tables()
            .assembly_ref(type_ref.resolution_scope.rid())
            .unwrap();
        let signature = md.blobs().get(reference.signature).unwrap();
        assert_eq!(signature, &[0x20, 0x00, 0x01]);
        for data in [
            md.strings().str(assembly.name).unwrap().as_bytes(),
            md.blobs().get(assembly.public_key_or_token).unwrap(),
            signature,
        ] {
            let start = data.as_ptr() as usize - bytes.as_ptr() as usize;
            let mut patched = bytes.to_vec();
            patched[start] ^= 1;
            let result = decompile(&patched, base).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains("IL FALLBACK"));
        }
        let mut code = vec![0x02, 0x6f]; // callvirt is not the admitted exact base call.
        code.extend(target.0.to_le_bytes());
        code.push(0x2a);
        assert_eq!(
            decompile(&patch_body(bytes, base, &code), base)
                .unwrap()
                .status,
            Status::Partial
        );
        let ordinary = method(bytes, "Reserved", 1);
        let mut code = vec![0x02, 0x28];
        code.extend(target.0.to_le_bytes());
        code.extend([0x03, 0x2a]);
        let result = decompile(&patch_body(bytes, ordinary, &code), ordinary).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(result.text.contains("outside constructor context"));
        let derived = method(bytes, ".ctor", 2);
        let mut code = vec![0x02, 0x28];
        code.extend(target.0.to_le_bytes());
        code.push(0x2a);
        let result = decompile(&patch_body(bytes, derived, &code), derived).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(result.text.contains("not the immediate base"));
        code[0] = 0x14; // Explicit null is not a proved original receiver.
        let result = decompile(&patch_body(bytes, base, &code), base).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(result.text.contains("unchanged original this"));
    }
}

#[test]
fn delegated_constructor_calls_keep_same_receiver_clock_order_and_exact_target() {
    for bytes in IMAGES {
        let root = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.declaring_type == "ConstructorCases.Root"
                    && m.simple_name == ".ctor"
                    && m.name.ends_with("()")
            })
            .unwrap()
            .token;
        let result = decompile(bytes, root).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        assert!(result.text.contains("__cil_ctor_call_06"));
        assert!(
            result.text.find("__cil_tick_count_0A").unwrap()
                < result.text.find("__cil_ctor_call_06").unwrap()
        );
        assert!(
            result
                .text
                .contains("same original receiver; no allocation or added null check")
        );
        assert!(
            !result.text.contains(" = new ") && !result.text.contains("__cil_core_object_ctor_")
        );
        let caller = method(bytes, ".ctor", 0);
        let result = decompile(bytes, caller).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text); // Caller lowering is not closure of its base dependency.
    }
}

#[test]
fn delegated_constructor_rejects_ordinary_other_receiver_and_invalid_target_contexts() {
    for bytes in IMAGES {
        let target = method(bytes, ".ctor", 2);
        let other = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.declaring_type == "ConstructorCases.Cells"
                    && m.simple_name == ".ctor"
                    && m.name.contains("class ConstructorCases.Cells")
            })
            .unwrap()
            .token;
        let mut code = vec![0x03, 0x16, 0x04, 0x28]; // Other receiver, length, value.
        code.extend(target.to_le_bytes());
        code.push(0x2a);
        let result = decompile(&patch_body(bytes, other, &code), other).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("same owning constructor and original this")
        );
        let ordinary = method(bytes, "Reserved", 1);
        let mut code = vec![0x02, 0x16, 0x03, 0x28];
        code.extend(target.to_le_bytes());
        code.extend([0x03, 0x2a]);
        assert_eq!(
            decompile(&patch_body(bytes, ordinary, &code), ordinary)
                .unwrap()
                .status,
            Status::Partial
        );
        let caller = method(bytes, ".ctor", 0);
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let row = md
            .tables()
            .row_bytes(cildec::TableId::MethodDef, target & 0xffffff)
            .unwrap();
        let start = row.as_ptr() as usize - bytes.as_ptr() as usize;
        let flags = u16::from_le_bytes(row[6..8].try_into().unwrap());
        let mut patched = bytes.to_vec();
        patched[start + 6..start + 8].copy_from_slice(&(flags & !0x1000).to_le_bytes());
        let result = decompile(&patched, caller).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(result.text.contains("invalid constructor call context"));
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(caller & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let mut code = body.code.to_vec();
        let offset = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "call")
            .unwrap()
            .offset as usize;
        code[offset] = 0x6f; // Constructor callvirt remains unsupported.
        assert_eq!(
            decompile(&patch_body(bytes, caller, &code), caller)
                .unwrap()
                .status,
            Status::Partial
        );
    }
}

#[test]
fn base_calls_preserve_preinitialization_and_ordered_effectful_arguments() {
    for bytes in [
        include_bytes!("fixtures/base-constructor-release.bin").as_slice(),
        include_bytes!("fixtures/base-constructor-debug.bin").as_slice(),
    ] {
        for owner in [
            "BaseConstructorCases.Local",
            "BaseConstructorCases.External",
        ] {
            let token = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == owner && m.simple_name == ".ctor")
                .unwrap()
                .token;
            let result = decompile(bytes, token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
            let before = result.text.find(".@Before = ").unwrap();
            let call = result.text.find("__cil_ctor_call_").unwrap();
            let after = result.text.find(".@After = ").unwrap();
            assert!(before < call && call < after, "{}", result.text);
            assert_eq!(result.text[..call].matches(".@Mark(").count(), 3);
            assert_eq!(result.text[call..].matches(".@Mark(").count(), 1);
        }
    }
}

#[test]
fn local_base_initialization_rejects_other_or_reassigned_receivers_and_ordinary_callers() {
    for bytes in IMAGES {
        let methods = list_methods(bytes).unwrap();
        let target = methods
            .iter()
            .find(|m| m.declaring_type == "ConstructorCases.Base" && m.simple_name == ".ctor")
            .unwrap()
            .token;
        let ctor = methods
            .iter()
            .find(|m| {
                m.declaring_type == "ConstructorCases.Cells"
                    && m.simple_name == ".ctor"
                    && m.name.contains("class ConstructorCases.Cells")
            })
            .unwrap()
            .token;
        for prefix in [
            vec![0x03, 0x04],
            vec![0x14, 0x04],
            vec![0x03, 0x10, 0x00, 0x02, 0x04],
        ] {
            let mut code = prefix;
            code.push(0x28);
            code.extend(target.to_le_bytes());
            code.push(0x2a);
            let result = decompile(&patch_body(bytes, ctor, &code), ctor).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(
                result.text.contains("original this")
                    || result.text.contains("constructor receiver reassignment"),
                "{}",
                result.text
            );
        }
        let ordinary = method(bytes, "Reserved", 1);
        let mut code = vec![0x02, 0x03, 0x28];
        code.extend(target.to_le_bytes());
        code.extend([0x03, 0x2a]);
        let result = decompile(&patch_body(bytes, ordinary, &code), ordinary).unwrap();
        assert_eq!(result.status, Status::Partial);
        assert!(
            result.text.contains("outside constructor context"),
            "{}",
            result.text
        );
    }
}

#[test]
fn local_direct_base_constructor_keeps_call_before_derived_initialization() {
    for bytes in IMAGES {
        let target = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "ConstructorCases.Base" && m.simple_name == ".ctor")
            .unwrap()
            .token;
        let ctor = method(bytes, ".ctor", 2);
        let result = decompile(bytes, ctor).unwrap();
        assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        let call = result
            .text
            .find(&format!("__cil_ctor_call_{target:08X}("))
            .unwrap();
        assert!(call < result.text.find(".@Effects = ").unwrap());
        assert!(call < result.text.find(" = new uint[").unwrap());
        assert!(
            result.text.contains("exact direct-base constructor")
                && result.text.contains("no allocation")
        );
        assert!(!result.text.contains("__cil_core_object_ctor_"));
    }
}
