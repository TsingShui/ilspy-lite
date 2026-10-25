// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

const RELEASE: &[u8] = include_bytes!("fixtures/release.bin");

fn token(name: &str) -> u32 {
    ilspy_lite::list_methods(RELEASE)
        .unwrap()
        .into_iter()
        .find(|m| m.name.contains(&format!("::{name}(")))
        .unwrap()
        .token
}

#[test]
fn il_dump_preserves_offsets_and_exception_regions() {
    let text = ilspy_lite::disassemble(RELEASE, token("Divide")).unwrap();
    assert!(text.contains("IL_0000:"));
    assert!(text.contains("finally"));
    assert!(text.contains("catch"));
    assert!(text.contains("endfinally"));
    assert!(text.contains(".code-bytes"));
}

#[test]
fn checked_add_is_not_rendered_as_wrapping_addition() {
    let result = ilspy_lite::decompile(RELEASE, token("CheckedAdd")).unwrap();
    assert_eq!(
        result.status,
        ilspy_lite::Status::Decompiled,
        "{}",
        result.text
    );
    assert!(result.text.contains("checked("), "{}", result.text);
}

#[test]
fn control_flow_calls_and_old_values_are_supported() {
    for name in [
        "Sum",
        "Select",
        "Conditional",
        "Shift",
        "Next",
        "Pair",
        "EvaluationOrder",
        "PreserveOldValue",
    ] {
        let result = ilspy_lite::decompile(RELEASE, token(name)).unwrap();
        assert_eq!(
            result.status,
            ilspy_lite::Status::Decompiled,
            "{name}: {}",
            result.text
        );
    }
}

#[test]
fn exceptions_are_explicit_fallback_not_invented_source() {
    let result = ilspy_lite::decompile(RELEASE, token("Divide")).unwrap();
    assert_eq!(result.status, ilspy_lite::Status::Partial);
    assert!(result.text.contains("IL FALLBACK"));
    assert!(result.text.contains(".try"));
    assert!(result.text.contains("endfinally"));
}

#[test]
fn integer_corpus_is_supported_in_debug_and_release() {
    for bytes in [
        include_bytes!("fixtures/integer-release.bin").as_slice(),
        include_bytes!("fixtures/integer-debug.bin").as_slice(),
    ] {
        let methods = ilspy_lite::list_methods(bytes).unwrap();
        assert_eq!(methods.len(), 25);
        for method in methods {
            let result = ilspy_lite::decompile(bytes, method.token).unwrap();
            assert_eq!(
                result.status,
                ilspy_lite::Status::Decompiled,
                "{}: {}",
                method.name,
                result.text
            );
        }
    }
}

#[test]
fn lists_every_method_including_generated_implementations() {
    let methods = ilspy_lite::list_methods(RELEASE).unwrap();
    assert_eq!(methods.len(), 59);
    assert!(methods.iter().any(|m| m.name.contains("MoveNext")));
    assert!(methods.iter().any(|m| m.name.contains("PreserveOldValue")));
    assert!(methods.iter().all(|m| m.token >> 24 == 6));
}
