// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/static-constructor-release.bin"),
    include_bytes!("fixtures/static-constructor-debug.bin"),
];

/// Static constructor bodies that are recovered: the type's own initializer runs
/// once at type initialization, keeps store order and never becomes an ordinary
/// method.
const RECOVERED: &[(&str, &str)] = &[
    ("StaticConstructorCases.Simple", ".cctor"),
    ("StaticConstructorCases.Accumulator", ".cctor"),
    ("StaticConstructorCases.First", ".cctor"),
    ("StaticConstructorCases.Second", ".cctor"),
    ("StaticConstructorCases.Sized", ".cctor"),
    ("StaticConstructorCases.Failing", ".cctor"),
    ("StaticConstructorCases.Caches", "TwiceImpl"),
];

fn token(bytes: &[u8], owner: &str, name: &str) -> u32 {
    list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap_or_else(|| panic!("{owner}.{name} is missing from the fixture"))
        .token
}

#[path = "support/fixture_contract.rs"]
mod fixture_contract;

/// Frozen known-unsupported set: (owner, method, stable reason substring). A
/// constructor whose body needs a capability this slice does not have keeps that
/// capability's own reason instead of the former blanket refusal.
const EXPECTED_PARTIAL: fixture_contract::ExpectedPartial = &[
    (
        "StaticConstructorCases.Caches",
        ".cctor",
        "unsupported opcode ldftn at IL_0001",
    ),
    (
        "StaticConstructorCases.Registry`1",
        ".cctor",
        "generic static field needs the constructed owner name",
    ),
    (
        "StaticConstructorCases.ExternalCache",
        ".cctor",
        "external allocation needs dependency facts",
    ),
];

#[test]
fn owned_fixture_contract_holds_in_both_views() {
    for bytes in IMAGES {
        fixture_contract::check(bytes, EXPECTED_PARTIAL);
        fixture_contract::check_scan_parity(bytes);
    }
}

#[test]
fn recovered_static_constructors_keep_their_member_shape() {
    for bytes in IMAGES {
        for (owner, name) in RECOVERED {
            let token = token(bytes, owner, name);
            let details = decompile(bytes, token).unwrap();
            assert_eq!(
                details.status,
                Status::Decompiled,
                "{owner}: {}",
                details.text
            );
            let readable = decompile_with_style(bytes, token, OutputStyle::Readable).unwrap();
            let short = owner.rsplit('.').next().unwrap();
            if *name == ".cctor" {
                // The initializer is rendered as the type's own static constructor:
                // no method name, no return type, nothing to call twice.
                assert!(
                    readable.text.contains(&format!("static {short}()")),
                    "{owner}: {}",
                    readable.text
                );
                assert!(
                    !readable.text.contains(".cctor"),
                    "{owner}: {}",
                    readable.text
                );
                assert!(
                    !readable.text.contains(&format!("static void {short}()")),
                    "{owner}: {}",
                    readable.text
                );
            }
        }
    }
}

#[test]
fn static_initializer_statements_keep_their_order() {
    for bytes in IMAGES {
        let readable = decompile_with_style(
            bytes,
            token(bytes, "StaticConstructorCases.Accumulator", ".cctor"),
            OutputStyle::Readable,
        )
        .unwrap();
        let lines: Vec<&str> = readable
            .text
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("//") && !line.is_empty())
            .collect();
        // Read the accumulated total, write it back, read it again, write it again:
        // neither store may be dropped, merged or moved.
        assert_eq!(lines[0], "static Accumulator()", "{}", readable.text);
        assert_eq!(lines[1], "{", "{}", readable.text);
        assert!(
            lines.iter().any(|line| line.contains("Total = "))
                && lines
                    .iter()
                    .filter(|line| line.contains("StaticConstructorCases.Accumulator.Total = "))
                    .count()
                    == 2,
            "{}",
            readable.text
        );
        let first_read = readable
            .text
            .find("StaticConstructorCases.Accumulator.Total")
            .expect("the initializer reads the field");
        let first_write = readable
            .text
            .find("StaticConstructorCases.Accumulator.Total = ")
            .expect("the initializer writes the field");
        assert!(first_read < first_write, "{}", readable.text);
    }
}

#[test]
fn a_readonly_static_field_is_assigned_by_its_own_initializer() {
    for bytes in IMAGES {
        let readable = decompile_with_style(
            bytes,
            token(bytes, "StaticConstructorCases.First", ".cctor"),
            OutputStyle::Readable,
        )
        .unwrap();
        assert_eq!(readable.status, Status::Decompiled, "{}", readable.text);
        assert!(
            readable
                .text
                .contains("StaticConstructorCases.First.Value = "),
            "{}",
            readable.text
        );
    }
}

#[test]
fn a_static_initializer_reads_across_types() {
    for bytes in IMAGES {
        let readable = decompile_with_style(
            bytes,
            token(bytes, "StaticConstructorCases.First", ".cctor"),
            OutputStyle::Readable,
        )
        .unwrap();
        // The initializer's own read of another type's static field triggers that
        // type's initialization at exactly this point.
        assert!(
            readable
                .text
                .contains("StaticConstructorCases.Second.Value"),
            "{}",
            readable.text
        );
    }
}
