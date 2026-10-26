// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT
use ilspy_lite::{Status, decompile, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/interface-reference-release.bin"),
    include_bytes!("fixtures/interface-reference-debug.bin"),
];

#[path = "support/fixture_contract.rs"]
#[allow(dead_code)]
mod fixture_contract;

fn token(bytes: &[u8], name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "InterfaceReferenceCases.Ops" && m.simple_name == name)
        .unwrap()
        .token
}

#[test]
fn local_interface_references_preserve_identity_and_dispatch() {
    for bytes in IMAGES {
        fixture_contract::check_with_declarations(
            bytes,
            &[(
                "InterfaceReferenceCases.Ops",
                "UnsupportedGeneric",
                "constructed member parent must be a class; interface dispatch needs IL fallback",
            )],
            &[
                ("InterfaceReferenceCases.IValue", "Compute"),
                ("InterfaceReferenceCases.IGeneric`1", "Compute"),
            ],
        );
        fixture_contract::check_scan_parity(bytes);
        let result = decompile(bytes, token(bytes, "Invoke")).unwrap();
        assert!(result.text.contains("__cil_callvirt_"), "{}", result.text);
        assert!(!result.text.contains("__cil_direct_"), "{}", result.text);
    }
}

#[test]
fn malformed_interface_shapes_and_cycles_stay_refused() {
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
        for (name, method, shape, reason) in [
            (
                "IValue",
                "Pass",
                0,
                "interface reference needs abstract flags",
            ),
            (
                "IValue",
                "Pass",
                1,
                "interface reference needs abstract flags",
            ),
            ("Holder", "Read", 2, "interface instance fields are invalid"),
            (
                "Counter",
                "Allocate",
                2,
                "allocation owner must be a concrete class",
            ),
        ] {
            let rid = named(name);
            let raw = md
                .tables()
                .row_bytes(cildec::TableId::TypeDef, rid)
                .unwrap();
            let at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
            let row = md.tables().type_def(rid).unwrap();
            let mut patched = bytes.to_vec();
            if shape == 0 {
                patched[at..at + 4].copy_from_slice(&(row.flags & !0x80).to_le_bytes());
            } else if shape == 1 {
                let base = md
                    .tables()
                    .raw_column(cildec::TableId::TypeDef, named("Counter"), 3)
                    .unwrap();
                patched[at + 8..at + 10].copy_from_slice(&(base as u16).to_le_bytes());
            } else {
                patched[at..at + 4].copy_from_slice(&(row.flags | 0xa0).to_le_bytes());
                patched[at + 8..at + 10].fill(0);
            }
            let result = decompile(&patched, token(bytes, method)).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
            assert!(result.text.contains(reason), "{}", result.text);
        }
        let child = named("IChild");
        let (rid, _) = md
            .tables()
            .iter::<cildec::tables::InterfaceImplRow>()
            .find(|(_, r)| r.as_ref().unwrap().class.get() == child)
            .unwrap();
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::InterfaceImpl, rid)
            .unwrap();
        let at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut patched = bytes.to_vec();
        patched[at + 2..at + 4].copy_from_slice(&((child << 2) as u16).to_le_bytes());
        let result = decompile(&patched, token(bytes, "ToBase")).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result.text.contains("cyclic local interface hierarchy"),
            "{}",
            result.text
        );
    }
}

#[test]
fn implemented_interface_is_required_for_static_assignment() {
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
        let counter = named("Counter");
        let unrelated = named("DerivedCounter");
        let mut patched = bytes.to_vec();
        for (rid, row) in md.tables().iter::<cildec::tables::InterfaceImplRow>() {
            if row.unwrap().class.get() != counter {
                continue;
            }
            let raw = md
                .tables()
                .row_bytes(cildec::TableId::InterfaceImpl, rid)
                .unwrap();
            let at = raw.as_ptr() as usize - bytes.as_ptr() as usize;
            patched[at..at + 2].copy_from_slice(&(unrelated as u16).to_le_bytes());
        }
        let result = decompile(&patched, token(bytes, "FromClass")).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
    }
}
