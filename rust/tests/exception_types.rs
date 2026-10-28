// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use ilspy_lite::{OutputStyle, Status, decompile_with_style, list_methods};

const IMAGES: [&[u8]; 2] = [
    include_bytes!("fixtures/exception-type-release.bin"),
    include_bytes!("fixtures/exception-type-debug.bin"),
];

#[test]
fn core_exception_constructors_keep_their_exact_overload_and_arguments() {
    for bytes in IMAGES {
        for name in ["Construct", "Range", "Order"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ExceptionTypeCases.Ops" && m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            assert!(result.text.contains("new "), "{}", result.text);
        }
    }
}

#[test]
fn exact_core_namespace_exception_identities_keep_catches_and_constructors() {
    for bytes in IMAGES {
        for name in ["Io", "Format", "CatchIo"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ExceptionTypeCases.Ops" && m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn exception_proofs_reject_counterfeit_scopes_ancestry_and_constructor_shapes() {
    for bytes in IMAGES {
        let pe = cildec::PeImage::parse(bytes).unwrap();
        let md = pe.metadata().unwrap();
        let methods = list_methods(bytes).unwrap();
        let token = |name: &str| {
            methods
                .iter()
                .find(|m| m.declaring_type == "ExceptionTypeCases.Ops" && m.simple_name == name)
                .unwrap()
                .token
        };
        let method = token("Construct");
        let body = cildec::MethodBody::from_image(
            &pe,
            &md.tables().method_def(method & 0xffffff).unwrap(),
        )
        .unwrap()
        .unwrap();
        let target = body
            .instructions()
            .map(Result::unwrap)
            .find(|i| i.opcode.name() == "newobj")
            .unwrap()
            .operand
            .token()
            .unwrap();
        let member = md.tables().member_ref(target.rid()).unwrap();
        let signature = md.blobs().get(member.signature).unwrap();
        assert_eq!(signature[0..3], [0x20, 2, 1]);
        let at = signature.as_ptr() as usize - bytes.as_ptr() as usize;
        for (offset, byte) in [(0, 0), (2, 8)] {
            let mut wrong = bytes.to_vec();
            wrong[at + offset] = byte;
            let result = decompile_with_style(&wrong, method, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{}", result.text);
        }
        let owner = md.tables().type_ref(member.class.rid()).unwrap();
        let assembly = md
            .tables()
            .assembly_ref(owner.resolution_scope.rid())
            .unwrap();
        let key = md.blobs().get(assembly.public_key_or_token).unwrap();
        let at = key.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut counterfeit = bytes.to_vec();
        counterfeit[at] ^= 1;
        let result =
            decompile_with_style(&counterfeit, token("ThrowBase"), OutputStyle::Readable).unwrap();
        assert_eq!(result.status, Status::Partial, "{}", result.text);

        let custom = (1..=md.tables().row_count(cildec::TableId::TypeDef))
            .find(|&rid| {
                md.strings()
                    .str(md.tables().type_def(rid).unwrap().type_name)
                    .unwrap()
                    == "Custom"
            })
            .unwrap();
        let object = (1..=md.tables().row_count(cildec::TableId::TypeRef))
            .find(|&rid| {
                let row = md.tables().type_ref(rid).unwrap();
                md.strings().str(row.type_namespace).unwrap() == "System"
                    && md.strings().str(row.type_name).unwrap() == "Object"
            })
            .unwrap();
        let row = md
            .tables()
            .row_bytes(cildec::TableId::TypeDef, custom)
            .unwrap();
        assert_eq!(row.len(), 14);
        let at = row.as_ptr() as usize - bytes.as_ptr() as usize + 8;
        let mut unrelated = bytes.to_vec();
        unrelated[at..at + 2]
            .copy_from_slice(&u16::try_from((object << 2) | 1).unwrap().to_le_bytes());
        for name in ["ThrowLocal", "MatchLocal"] {
            let result =
                decompile_with_style(&unrelated, token(name), OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Partial, "{name}: {}", result.text);
        }
    }
}

#[test]
fn local_exception_ancestry_preserves_typed_catch_matching_and_identity() {
    for bytes in IMAGES {
        for name in ["ThrowLocal", "MatchLocal", "ConstructLocal"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ExceptionTypeCases.Ops" && m.simple_name == name)
                .unwrap();
            let result = decompile_with_style(bytes, method.token, OutputStyle::Readable).unwrap();
            assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
        }
    }
}

#[test]
fn core_exception_throws_and_rethrows_keep_the_original_reference() {
    for bytes in IMAGES {
        for name in ["ThrowBase", "ThrowSystem", "Rethrow"] {
            let method = list_methods(bytes)
                .unwrap()
                .into_iter()
                .find(|m| m.declaring_type == "ExceptionTypeCases.Ops" && m.simple_name == name)
                .unwrap();
            for style in [OutputStyle::Readable, OutputStyle::Details] {
                let result = decompile_with_style(bytes, method.token, style).unwrap();
                assert_eq!(result.status, Status::Decompiled, "{name}: {}", result.text);
            }
        }
    }
}
