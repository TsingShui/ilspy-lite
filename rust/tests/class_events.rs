// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{decompile_type, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/class-release.bin"),
    include_bytes!("fixtures/class-debug.bin"),
];

fn event(bytes: &[u8], name: &str) -> (u32, usize) {
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let (rid, _) = md
        .tables()
        .iter::<cildec::tables::EventRow>()
        .find(|(_, row)| md.strings().str(row.as_ref().unwrap().name).unwrap() == name)
        .unwrap();
    let raw = md.tables().row_bytes(cildec::TableId::Event, rid).unwrap();
    assert_eq!(raw.len(), 6);
    (
        0x14000000 | rid,
        raw.as_ptr() as usize - bytes.as_ptr() as usize,
    )
}

#[test]
fn unreadable_event_name_keeps_type_identity_on_its_declaration() {
    let bytes = IMAGES[0];
    let (token, offset) = event(bytes, "External");
    let mut patched = bytes.to_vec();
    patched[offset + 2..offset + 4].copy_from_slice(&0xffffu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Events").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    let line = result
        .text
        .lines()
        .find(|l| l.starts_with(&format!(".event 0x{token:08X} ")))
        .unwrap();
    assert!(
        line.contains("name-index=0x0000FFFF") && line.contains("System.EventHandler"),
        "{line}"
    );
    assert!(result.text.contains("FAILED metadata"));
    assert!(result.text.contains("ClassCases.Changed"));
    assert!(result.text.contains("::add_External("));
}

#[test]
fn invalid_event_type_keeps_accessors_and_peer_events() {
    let bytes = IMAGES[0];
    let (token, offset) = event(bytes, "External");
    let mut patched = bytes.to_vec();
    patched[offset + 4..offset + 6].copy_from_slice(&0xfffdu16.to_le_bytes());
    let result = decompile_type(&patched, "ClassCases.Events").unwrap();
    assert_eq!(result.summary.metadata_failed, 1);
    assert_eq!(result.summary.exit_code(), 1);
    assert!(
        result
            .text
            .lines()
            .any(|l| l.starts_with(&format!(".event 0x{token:08X} "))
                && l.contains("type=0x01003FFF"))
    );
    assert!(result.text.contains("FAILED metadata"));
    assert!(result.text.contains("ClassCases.Changed"));
    assert_eq!(
        result
            .text
            .lines()
            .filter(|l| l.starts_with(".accessor "))
            .count(),
        4
    );
    assert!(result.text.contains("::add_External("));
}

#[test]
fn fire_other_and_unknown_event_relations_are_not_omitted() {
    let bytes = IMAGES[0];
    let (token, _) = event(bytes, "External");
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let rid = md.tables().semantics_for(cildec::Token(token)).unwrap()[0];
    let row = md
        .tables()
        .row_bytes(cildec::TableId::MethodSemantics, rid)
        .unwrap();
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    for (bits, kind, failures) in [(32u16, "fire", 0), (4, "other", 0), (64, "unknown", 1)] {
        let mut patched = bytes.to_vec();
        patched[offset..offset + 2].copy_from_slice(&bits.to_le_bytes());
        let result = decompile_type(&patched, "ClassCases.Events").unwrap();
        assert_eq!(result.summary.metadata_failed, failures);
        assert!(result.text.contains(&format!(
            ".accessor 0x{:08X} association=0x{token:08X} semantics=0x{bits:04X} {kind} method=",
            0x18000000 | rid
        )));
        assert!(result.text.contains("::add_External("));
    }
}

#[test]
fn invalid_event_map_range_cannot_silently_hide_members() {
    let bytes = IMAGES[0];
    let pe = cildec::PeImage::parse(bytes).unwrap();
    let md = pe.metadata().unwrap();
    let row = md.tables().row_bytes(cildec::TableId::EventMap, 1).unwrap();
    assert_eq!(row.len(), 4);
    let offset = row.as_ptr() as usize - bytes.as_ptr() as usize;
    for start in [0u16, u16::MAX] {
        let mut patched = bytes.to_vec();
        patched[offset + 2..offset + 4].copy_from_slice(&start.to_le_bytes());
        assert!(decompile_type(&patched, "ClassCases.Events").is_err());
    }
}

#[test]
fn events_keep_types_and_accessor_methods_without_delegate_lowering() {
    for bytes in IMAGES {
        let result = decompile_type(bytes, "ClassCases.Events").unwrap();
        assert_eq!(result.summary.metadata_failed, 0, "{}", result.text);
        let events: Vec<_> = result
            .text
            .lines()
            .filter(|l| l.starts_with(".event "))
            .collect();
        assert_eq!(events.len(), 2, "{}", result.text);
        assert!(events.iter().any(|l| l.contains("External")
            && l.contains("System.EventHandler")
            && l.contains("type=0x01")));
        assert!(events.iter().any(|l| l.contains("Local")
            && l.contains("ClassCases.Changed")
            && l.contains("type=0x02")));
        assert_eq!(
            result
                .text
                .lines()
                .filter(|l| l.starts_with(".accessor "))
                .count(),
            4
        );
        for name in [
            "add_External",
            "remove_External",
            "add_Local",
            "remove_Local",
        ] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ClassCases.Events" && m.simple_name == name)
                .unwrap();
            let kind = if name.starts_with("add_") {
                "adder"
            } else {
                "remover"
            };
            assert!(
                result.text.lines().any(|l| l.starts_with(".accessor ")
                    && l.contains(&format!(" {kind} method=0x{:08X}", method.token))
                    && l.contains(name)),
                "{}",
                result.text
            );
            assert!(
                result
                    .text
                    .contains(&format!(".method-status 0x{:08X} ", method.token))
            );
        }
        let generic = decompile_type(bytes, "ClassCases.GenericEvents`1").unwrap();
        assert_eq!(generic.summary.metadata_failed, 0, "{}", generic.text);
        assert!(
            generic.text.contains("System.Action`1<!0>"),
            "{}",
            generic.text
        );
        assert!(generic.text.contains("Updated") && generic.text.contains("type=0x1B"));
        assert!(generic.text.contains("add_Updated") && generic.text.contains("remove_Updated"));
        assert!(generic.summary.partial > 0);
    }
}
