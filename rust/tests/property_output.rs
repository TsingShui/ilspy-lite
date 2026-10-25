// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{DependencyContext, OutputStyle, decompile_type_with_style};

const INPUT: &[u8] = include_bytes!("fixtures/binding-release.bin");
const LIBRARY: &[u8] = include_bytes!("fixtures/binding-library-release.bin");

#[test]
fn exact_base_property_relationships_render_reads_and_writes_without_losing_dispatch() {
    let mut context = DependencyContext::new(INPUT.to_vec()).unwrap();
    context
        .add_reference("owned BindingLibrary", LIBRARY.to_vec())
        .unwrap();
    let actual = context
        .decompile_type("BindingCases.Derived", OutputStyle::Readable)
        .unwrap();
    assert!(
        actual.text.contains("return base.Enabled;"),
        "{}",
        actual.text
    );
    assert!(actual.text.contains("base.Enabled = "), "{}", actual.text);
    assert!(
        actual.text.contains("return base.VirtualFlag;"),
        "a direct base call must not dispatch to the override: {}",
        actual.text
    );
    assert!(
        actual.text.contains("return base.ConstantFlag;"),
        "{}",
        actual.text
    );
    assert!(!actual.text.contains("base.Pretender"));
    assert!(
        actual.text.contains("__cil_memberref_call_"),
        "ordinary methods retain exact calls"
    );
    let no_context =
        decompile_type_with_style(INPUT, "BindingCases.Derived", OutputStyle::Readable).unwrap();
    assert!(!no_context.text.contains("base.Enabled"));
    assert_eq!(actual.summary, no_context.summary);
    let detailed = context
        .decompile_type("BindingCases.Derived", OutputStyle::Details)
        .unwrap();
    assert!(detailed.text.contains("__cil_memberref_call_"));
    assert!(!detailed.text.contains("base.Enabled"));
}
