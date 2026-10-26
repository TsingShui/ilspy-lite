// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

//! Fixture contract shared by the owned integration tests.
//!
//! There are deliberately no method counts here. Every MethodDef in a fixture
//! must lower without panicking and with identical admission in both views. A
//! method listed as expected-partial must keep its stated blocker, and every
//! other method must be recovered. Because the expectation is keyed by
//! owner/name/reason, a promotion or a regression is an explicit one-line diff
//! instead of an edited magic number, and a stale entry fails rather than
//! silently shrinking the negative surface.

use ilspy_lite::{OutputStyle, Status, list_methods};

/// (declaring type, method name, stable substring of the fallback reason)
pub type ExpectedPartial<'a> = &'a [(&'a str, &'a str, &'a str)];

pub fn check(bytes: &[u8], expected_partial: ExpectedPartial<'_>) {
    check_with_declarations(bytes, expected_partial, &[]);
}

/// Legitimately bodyless methods (abstract/interface members) are their own
/// expected state: they must stay declarations and never turn into a fallback.
pub fn check_with_declarations(
    bytes: &[u8],
    expected_partial: ExpectedPartial<'_>,
    expected_declarations: &[(&str, &str)],
) {
    let methods = list_methods(bytes).unwrap();
    assert!(!methods.is_empty(), "fixture has no MethodDef rows");
    let mut matched = std::collections::BTreeSet::new();
    for method in &methods {
        let details = ilspy_lite::decompile(bytes, method.token).unwrap();
        let readable =
            ilspy_lite::decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
        assert_eq!(
            readable.status, details.status,
            "{}.{}: presentations disagree",
            method.declaring_type, method.simple_name
        );
        let key = (method.declaring_type.as_str(), method.simple_name.as_str());
        if expected_declarations.contains(&key) {
            assert_eq!(
                details.status,
                Status::Declaration,
                "{}.{}: expected a bodyless declaration",
                method.declaring_type,
                method.simple_name
            );
            matched.insert(key);
            continue;
        }
        match expected_partial
            .iter()
            .find(|(owner, name, _)| (*owner, *name) == key)
        {
            None => assert_eq!(
                details.status,
                Status::Decompiled,
                "{}.{}: unexpected fallback; see the expected_partial table",
                method.declaring_type,
                method.simple_name
            ),
            Some((_, _, reason)) => {
                assert_eq!(
                    details.status,
                    Status::Partial,
                    "{}.{}: promoted; drop its expected_partial entry",
                    method.declaring_type,
                    method.simple_name
                );
                assert!(
                    details.text.contains(reason),
                    "{}.{}: reason drifted from {reason:?}",
                    method.declaring_type,
                    method.simple_name
                );
                matched.insert(key);
            }
        }
    }
    for (owner, name, _) in expected_partial {
        assert!(
            matched.contains(&(*owner, *name)),
            "stale expected_partial entry: {owner}.{name}"
        );
    }
    for (owner, name) in expected_declarations {
        assert!(
            matched.contains(&(*owner, *name)),
            "stale expected_declarations entry: {owner}.{name}"
        );
    }
}

/// Scan enumeration and per-method output must share one admission path.
pub fn check_scan_parity(bytes: &[u8]) {
    let mut records = Vec::new();
    ilspy_lite::scan_methods(bytes, |token, status, _| {
        records.push((token, status));
        Ok(())
    })
    .unwrap();
    let methods = list_methods(bytes).unwrap();
    assert_eq!(records.len(), methods.len());
    for (definition, (token, status)) in methods.iter().zip(records) {
        assert_eq!(definition.token, token);
        assert_eq!(ilspy_lite::decompile(bytes, token).unwrap().status, status);
    }
}
