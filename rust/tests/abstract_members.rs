// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/abstract-member-release.bin"),
    include_bytes!("fixtures/abstract-member-debug.bin"),
];

#[path = "support/fixture_contract.rs"]
#[expect(
    dead_code,
    reason = "This fixture uses check_with_declarations instead of check"
)]
mod fixture_contract;

#[test]
fn abstract_interface_dispatch_uses_the_constructed_interface_token() {
    for bytes in IMAGES {
        fixture_contract::check_with_declarations(
            bytes,
            &[],
            &[
                ("AbstractMemberCases.Base`1", "Compute"),
                ("AbstractMemberCases.IValue`1", "Compute"),
            ],
        );
        fixture_contract::check_scan_parity(bytes);
        // Calling through a constructed interface is an interface dispatch on the
        // exact MemberRef; it must not become a direct call.
        let interface = decompile_with_style(
            bytes,
            list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| {
                    m.declaring_type == "AbstractMemberCases.Ops" && m.simple_name == "Interface"
                })
                .unwrap()
                .token,
            OutputStyle::Readable,
        )
        .unwrap();
        assert_eq!(interface.status, Status::Decompiled, "{}", interface.text);
        assert!(
            interface.text.contains("__cil_callvirt_"),
            "{}",
            interface.text
        );
    }
}

#[test]
fn abstract_constructed_owners_allow_member_calls() {
    for bytes in IMAGES {
        for name in ["Virtual", "Fixed", "Twice"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|method| {
                    method.simple_name == name && method.declaring_type == "AbstractMemberCases.Ops"
                })
                .unwrap();
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result = decompile_with_style(bytes, method.token, style).unwrap();
                assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            }
        }
    }
}

#[test]
fn abstract_owner_allocation_is_still_refused() {
    for bytes in IMAGES {
        let method = list_methods(bytes)
            .unwrap()
            .into_iter()
            .find(|method| method.simple_name == "Allocate")
            .unwrap();
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let (rid, row) = md
            .tables()
            .iter::<cildec::tables::TypeDefRow>()
            .find(|(_, row)| {
                md.strings().str(row.as_ref().unwrap().type_name).unwrap() == "Buildable`1"
            })
            .unwrap();
        let raw = md
            .tables()
            .row_bytes(cildec::TableId::TypeDef, rid)
            .unwrap();
        let offset = raw.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut patched = bytes.to_vec();
        patched[offset..offset + 4].copy_from_slice(&(row.unwrap().flags | 0x80).to_le_bytes());
        let result = decompile_with_style(&patched, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);
        assert!(
            result
                .text
                .contains("allocation owner must be a concrete class"),
            "{}",
            result.text
        );
    }
}
