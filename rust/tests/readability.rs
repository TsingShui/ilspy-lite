// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{Status, decompile, list_methods};

const INTEGERS: [&[u8]; 2] = [
    include_bytes!("fixtures/integer-release.bin"),
    include_bytes!("fixtures/integer-debug.bin"),
];

fn output(bytes: &[u8], owner: &str, name: &str) -> String {
    let method = list_methods(bytes)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == owner && m.simple_name == name)
        .unwrap();
    let result = decompile(bytes, method.token).unwrap();
    assert_eq!(result.status, Status::Decompiled, "{}", result.text);
    result.text
}

#[test]
fn unsigned_operations_keep_reinterpretation_and_overflow_checks() {
    for bytes in INTEGERS {
        for (name, signed, unsigned, symbol) in [
            ("AddCheckedUnsigned", "int", "uint", "+"),
            ("SubCheckedUnsigned", "int", "uint", "-"),
            ("MulCheckedUnsigned", "int", "uint", "*"),
            ("AddCheckedU64", "long", "ulong", "+"),
        ] {
            let text = output(bytes, "IntegerCases.Cases", name);
            assert!(
                text.contains(&format!("= unchecked(({signed})(a0));")),
                "{text}"
            );
            assert!(text.contains(&format!(
                "= unchecked(({signed})checked(unchecked(({unsigned})v0) {symbol} unchecked(({unsigned})v1)));"
            )), "{text}");
            assert!(
                text.contains(&format!("return unchecked(({unsigned})(v")),
                "{text}"
            );
        }
        for (name, signed, unsigned, symbol) in [
            ("DivUnsigned", "int", "uint", "/"),
            ("RemUnsigned64", "long", "ulong", "%"),
        ] {
            let text = output(bytes, "IntegerCases.Cases", name);
            assert!(text.contains(&format!(
                "= unchecked(({signed})(unchecked(({unsigned})v0) {symbol} unchecked(({unsigned})v1)));"
            )), "{text}");
        }
    }
}

#[test]
fn boolean_loads_and_returns_keep_stack_conversions() {
    for bytes in INTEGERS {
        let text = output(bytes, "IntegerCases.Cases", "Join");
        assert!(text.contains("__cil_bool_load(a0)"), "{text}");
        let text = output(bytes, "IntegerCases.Cases", "Equal");
        assert!(text.contains("return __cil_bool_store("), "{text}");
    }
}

#[test]
fn narrow_field_stores_keep_truncating_casts() {
    let image = include_bytes!("fixtures/reference-release.bin");
    let pe = cildec::PeImage::parse(image).unwrap();
    let md = pe.metadata().unwrap();
    let field = md.tables().field(1).unwrap();
    assert_eq!(md.strings().str(field.name).unwrap(), "Value");
    let signature = md.blobs().get(field.signature).unwrap();
    assert_eq!(signature, &[0x06, 0x08]); // FIELD, I4
    let start = signature.as_ptr() as usize - image.as_ptr() as usize;
    for (element_type, name) in [
        (0x04, "sbyte"),
        (0x05, "byte"),
        (0x06, "short"),
        (0x07, "ushort"),
        (0x03, "char"),
    ] {
        let mut bytes = image.to_vec();
        bytes[start + 1] = element_type;
        let text = output(&bytes, "ReferenceCases.Cell", "Credit");
        assert!(text.contains("= unchecked((int)(v0.@Value));"), "{text}");
        assert!(
            text.contains(&format!(".@Value = unchecked(({name})(v")),
            "{text}"
        );
    }
}

#[test]
fn base_declared_field_keeps_receiver_cast() {
    let image = include_bytes!("fixtures/reference-release.bin");
    let method = list_methods(image)
        .unwrap()
        .into_iter()
        .find(|m| m.declaring_type == "ReferenceCases.Child" && m.simple_name == "BaseConstant")
        .unwrap();
    let pe = cildec::PeImage::parse(image).unwrap();
    let row = pe
        .metadata()
        .unwrap()
        .tables()
        .method_def(method.token & 0xffffff)
        .unwrap();
    let body = cildec::MethodBody::from_image(&pe, &row).unwrap().unwrap();
    let code = [0x02, 0x7b, 1, 0, 0, 4, 0x2a]; // ldarg.0; ldfld Cell.Value; ret
    assert_eq!(body.code.len(), code.len());
    let start = body.code.as_ptr() as usize - image.as_ptr() as usize;
    let mut bytes = image.to_vec();
    bytes[start..start + code.len()].copy_from_slice(&code);
    let text = output(&bytes, "ReferenceCases.Child", "BaseConstant");
    assert!(
        text.contains("((global::@ReferenceCases.@Cell)v0).@Value"),
        "{text}"
    );
}

#[test]
fn exact_type_field_receivers_need_no_casts() {
    for bytes in [
        include_bytes!("fixtures/reference-release.bin").as_slice(),
        include_bytes!("fixtures/reference-debug.bin").as_slice(),
    ] {
        for name in ["Read", "Credit", "Nested", "StoreAfterRead", "ClearOther"] {
            let text = output(bytes, "ReferenceCases.Cell", name);
            assert!(!text.contains("((global::"), "{text}");
            assert!(
                text.contains(".@Value") || text.contains(".@Other"),
                "{text}"
            );
        }
    }
}

#[test]
fn signed_arithmetic_keeps_overflow_context_without_result_casts() {
    for bytes in INTEGERS {
        for (name, expression) in [
            ("Add", "unchecked(v0 + v1)"),
            ("Add64", "unchecked(v0 + v1)"),
            ("MulChecked64", "checked(v0 * v1)"),
            ("Div", "unchecked(v0 / v1)"),
            ("Rem", "unchecked(v0 % v1)"),
            ("XorOr", "unchecked(v0 ^ v1)"),
        ] {
            let text = output(bytes, "IntegerCases.Cases", name);
            assert!(text.contains(&format!("= {expression};")), "{text}");
            assert!(!text.contains("unchecked((int)"), "{text}");
            assert!(!text.contains("unchecked((long)"), "{text}");
        }
    }
}

#[test]
fn signed_integer_snapshots_and_returns_need_no_identity_casts() {
    for bytes in INTEGERS {
        for (name, ty) in [("Add", "int"), ("Add64", "long")] {
            let text = output(bytes, "IntegerCases.Cases", name);
            assert!(text.contains(&format!("{ty} v0 = a0;")), "{text}");
            assert!(text.contains(&format!("{ty} v1 = a1;")), "{text}");
            assert!(text.contains("return v"), "{text}");
        }
    }
}
