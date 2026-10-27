// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT
use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/generic-binding-release.bin"),
    include_bytes!("fixtures/generic-binding-debug.bin"),
];

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

#[test]
fn member_parameters_and_results_keep_the_callers_generic_scope() {
    for bytes in IMAGES {
        for method in list_methods(bytes).unwrap().into_iter().filter(|m| {
            m.declaring_type == "GenericBindingCases.Bag`1" && m.simple_name.starts_with("Forward")
        }) {
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        }
    }
}

#[test]
fn another_generic_definition_is_not_the_callers_open_self() {
    for bytes in IMAGES {
        for method in list_methods(bytes)
            .unwrap()
            .into_iter()
            .filter(|m| m.declaring_type == "GenericBindingCases.Proxy`1")
        {
            let result = decompile(bytes, method.token).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{}", result.text);
        }
    }
}

#[test]
fn constructed_member_arity_is_not_inferred_from_variable_positions() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let named = |name: &str| {
            md.tables()
                .iter::<cildec::tables::TypeDefRow>()
                .find(|(_, r)| md.strings().str(r.as_ref().unwrap().type_name).unwrap() == name)
                .unwrap()
                .0
        };
        let bag = named("Bag`1");
        let pair = named("Pair`2");
        let mut patched = bytes.to_vec();
        let mut changed = 0;
        for (_, row) in md.tables().iter::<cildec::tables::TypeSpecRow>() {
            let blob = md.blobs().get(row.unwrap().signature).unwrap();
            if blob == [0x15, 0x12, (bag << 2) as u8, 1, 0x13, 0] {
                let at = blob.as_ptr() as usize - bytes.as_ptr() as usize;
                patched[at + 2] = (pair << 2) as u8;
                changed += 1;
            }
        }
        assert!(changed > 0);
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.declaring_type == "GenericBindingCases.Bag`1" && m.simple_name == "ForwardGet"
            })
            .unwrap();
        let result = decompile(&patched, method.token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(result.text.contains("arity"), "{}", result.text);
    }
}

#[test]
fn a_foreign_instantiation_cannot_borrow_an_out_of_range_caller_variable() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let bag = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, r)| md.strings().str(r.as_ref().unwrap().type_name).unwrap() == "Bag`1")
            .unwrap()
            .0;
        let mut patched = bytes.to_vec();
        let mut changed = 0;
        for (_, row) in md.tables().iter::<cildec::tables::TypeSpecRow>() {
            let blob = md.blobs().get(row.unwrap().signature).unwrap();
            if blob == [0x15, 0x12, (bag << 2) as u8, 1, 0x13, 0] {
                let at = blob.as_ptr() as usize - bytes.as_ptr() as usize;
                patched[at + 5] = 1;
                changed += 1;
            }
        }
        assert!(changed > 0);
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| m.declaring_type == "GenericBindingCases.Proxy`1" && m.simple_name == "Read")
            .unwrap();
        let result = decompile(&patched, method.token).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("generic variable has no owner or exceeds declared positions"),
            "{}",
            result.text
        );
    }
}

#[test]
fn cyclic_constructed_type_owner_is_refused_without_overflowing() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let bag = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, row)| md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Bag`1")
            .unwrap()
            .0;
        let (rid, blob) = md
            .tables()
            .iter::<cildec::tables::TypeSpecRow>()
            .filter_map(|(rid, row)| {
                let blob = md.blobs().get(row.unwrap().signature).unwrap();
                (rid < 32 && blob == [0x15, 0x12, (bag << 2) as u8, 1, 0x13, 0])
                    .then_some((rid, blob))
            })
            .next()
            .expect("owned constructed Bag TypeSpec");
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.declaring_type == "GenericBindingCases.Bag`1" && m.simple_name == "ForwardGet"
            })
            .unwrap();
        let mut owned = bytes.to_vec();
        let position = blob.as_ptr() as usize - bytes.as_ptr() as usize;
        owned[position + 2] = ((rid << 2) | 2) as u8;
        let path = std::env::temp_dir().join(format!(
            "ilspy-cyclic-type-{}-{rid}.bin",
            std::process::id()
        ));
        std::fs::write(&path, owned).unwrap();
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_ilspy-lite"))
            .arg("decompile")
            .arg(&path)
            .arg("--method")
            .arg(format!("0x{:08X}", method.token))
            .arg("--no-auto-references")
            .output()
            .unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            result.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(
            text.contains("cyclic") || text.contains("TypeDef or TypeRef"),
            "{text}"
        );
    }
}

#[test]
fn the_whole_owned_fixture_is_recovered_and_scan_agrees() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, &[]);
        fixture_contract::check_scan_parity(bytes);
    }
}
