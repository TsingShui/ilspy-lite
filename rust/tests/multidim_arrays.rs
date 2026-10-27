// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/multidim-release.bin"),
    include_bytes!("fixtures/multidim-debug.bin"),
];

fn token(bytes: &[u8], name: &str) -> u32 {
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.simple_name == name)
        .unwrap_or_else(|| panic!("owned method {name} missing"));
    method.token
}

fn decompile(bytes: &[u8], name: &str) -> ilspy_lite::DecompiledMethod {
    decompile_with_style(bytes, token(bytes, name), OutputStyle::Readable).unwrap()
}

#[test]
fn general_arrays_allocate_and_access_elements_exactly() {
    for bytes in IMAGES {
        let build = decompile(bytes, "Build");
        assert_eq!(build.status, Status::Decompiled, "{}", build.text);
        assert!(
            build.text.contains("new int[rows, columns]"),
            "{}",
            build.text
        );
        let cell = decompile(bytes, "Cell");
        assert_eq!(cell.status, Status::Decompiled, "{}", cell.text);
        assert!(cell.text.contains("grid[i, j]"), "{}", cell.text);
        let store = decompile(bytes, "Store");
        assert_eq!(store.status, Status::Decompiled, "{}", store.text);
        assert!(store.text.contains("grid[i, j] = value;"), "{}", store.text);
        let sum = decompile(bytes, "SumDouble");
        assert_eq!(sum.status, Status::Decompiled, "{}", sum.text);
        assert!(sum.text.contains("grid[0, 0]"), "{}", sum.text);
        assert!(sum.text.contains("grid[1, 1]"), "{}", sum.text);
        let pick = decompile(bytes, "Pick");
        assert_eq!(pick.status, Status::Decompiled, "{}", pick.text);
        assert!(pick.text.contains("grid[i, j]"), "{}", pick.text);
    }
}

#[test]
fn general_array_members_outside_element_access_stay_partial() {
    for bytes in IMAGES {
        for name in ["Sum", "Total"] {
            let result = decompile(bytes, name);
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
            assert!(
                result
                    .text
                    .contains("unsupported reference relationship argument shape"),
                "{name}: {}",
                result.text
            );
        }
    }
}
