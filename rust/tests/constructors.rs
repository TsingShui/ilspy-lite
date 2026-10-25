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
            &[0x0f, 0x00, 0x26, 0x02, 0x04][..], // ldarga.0 is unsupported, not proof of entry this.
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
fn constructor_calls_are_not_ordinary_calls_and_remain_visible_when_unsupported() {
    for bytes in IMAGES {
        for count in [0, 2] {
            let ctor = method(bytes, ".ctor", count);
            let result = decompile(bytes, ctor).unwrap();
            assert_eq!(result.status, Status::Partial);
            assert!(
                result.text.contains("constructor call needs IL fallback"),
                "{}",
                result.text
            );
            assert!(result.text.contains("IL FALLBACK") && result.text.contains("call"));
        }
    }
}
