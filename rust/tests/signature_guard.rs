// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

pub use ilspy_lite::Result;
#[path = "../src/signature_guard.rs"]
mod signature_guard;

use cildec::{CallConv, TableId, Token, Type};
use signature_guard::{field, method, method_spec, property, type_spec};

// These are literal blob bodies, not output from a compiler or metadata writer.
const METHOD: &[u8] = &[0x20, 2, 1, 8, 0x0e];
const FIELD: &[u8] = &[6, 0x1f, 5, 0x1d, 0x20, 6, 0x12, 4];
const PROPERTY: &[u8] = &[0x28, 1, 0x1f, 5, 0x10, 8, 0x0e];
const TYPE_SPEC: &[u8] = &[0x15, 0x12, 5, 2, 0x13, 0, 0x1d, 0x12, 6];
const METHOD_SPEC: &[u8] = &[0x0a, 2, 0x1e, 0, 0x12, 5];

type Check = fn(&[u8]) -> bool;
const SEAMS: &[(Check, &[u8])] = &[
    (|b| method(b).is_ok(), METHOD),
    (|b| field(b).is_ok(), FIELD),
    (|b| property(b).is_ok(), PROPERTY),
    (|b| type_spec(b).is_ok(), TYPE_SPEC),
    (|b| method_spec(b).is_ok(), METHOD_SPEC),
];

#[test]
fn known_good_literals_preserve_cildec_results() {
    assert_eq!(
        method(METHOD).unwrap(),
        cildec::MethodSig::parse(METHOD).unwrap()
    );
    assert_eq!(
        field(FIELD).unwrap(),
        cildec::FieldSig::parse(FIELD).unwrap()
    );
    assert_eq!(
        property(PROPERTY).unwrap(),
        cildec::PropertySig::parse(PROPERTY).unwrap()
    );
    assert_eq!(
        type_spec(TYPE_SPEC).unwrap(),
        cildec::TypeSpecSig::parse(TYPE_SPEC).unwrap()
    );
    assert_eq!(
        method_spec(METHOD_SPEC).unwrap(),
        cildec::MethodSpecSig::parse(METHOD_SPEC).unwrap()
    );
}

#[test]
fn every_seam_rejects_trailing_bytes_and_every_truncation() {
    for &(check, good) in SEAMS {
        assert!(check(good), "{good:02x?}");
        for end in 0..good.len() {
            assert!(!check(&good[..end]), "truncated {good:02x?} at {end}");
        }
        for extra in [0, 5, 8, 0x41, 0xff] {
            let mut bad = good.to_vec();
            bad.push(extra);
            assert!(!check(&bad), "trailing {bad:02x?}");
        }
    }
}

#[test]
fn oversized_rid_does_not_alias_typeref_one() {
    let bad = [0x12, 0xc4, 0, 0, 5]; // coded index 0x04000005
    assert_eq!(
        cildec::TypeSpecSig::parse(&bad).unwrap().type_,
        Type::Class(Token::new(TableId::TypeRef, 1))
    );
    assert!(type_spec(&bad).is_err());
}

#[test]
fn every_token_position_is_checked_before_conversion() {
    // prefix, suffix: class/value, modifier, generic definition, nested args,
    // parameter and return modifiers, pointer/vector modifiers, fnptr param.
    let positions: &[(&[u8], &[u8])] = &[
        (&[0x12], &[]),
        (&[0x11], &[]),
        (&[0x1f], &[8]),
        (&[0x20], &[8]),
        (&[0x15, 0x12], &[1, 8]),
        (&[0x15, 0x11], &[1, 8]),
        (&[0x15, 0x12, 5, 1, 0x12], &[]),
        (&[0x0f, 0x20], &[1]),
        (&[0x1d, 0x1f], &[8]),
        (&[0x1b, 0, 1, 1, 0x1f], &[8]),
        (&[0x1b, 0, 0, 0x20], &[1]),
    ];
    for &(prefix, suffix) in positions {
        for coded in [0, 1, 2, 3, 7, 0x0400_0000, 0x0400_0005, 0x1fff_fffe] {
            let mut raw = prefix.to_vec();
            let mut encoded = [0; 4];
            let n = cildec::compressed::write_u32(coded, &mut encoded).unwrap();
            raw.extend_from_slice(&encoded[..n]);
            raw.extend_from_slice(suffix);
            assert!(type_spec(&raw).is_err(), "{raw:02x?}");
        }
        for coded in [4, 5, 6, 0x03ff_fffc, 0x03ff_fffd, 0x03ff_fffe] {
            let mut raw = prefix.to_vec();
            let mut encoded = [0; 4];
            let n = cildec::compressed::write_u32(coded, &mut encoded).unwrap();
            raw.extend_from_slice(&encoded[..n]);
            raw.extend_from_slice(suffix);
            assert!(type_spec(&raw).is_ok(), "{raw:02x?}");
        }
    }
}

#[test]
fn token_checks_cover_all_exported_roots() {
    let overflow = [0xc4, 0, 0, 5];
    for (prefix, suffix, check) in [
        (&[0, 1, 1, 0x12][..], &[][..], SEAMS[0].0),
        (&[6, 0x1f][..], &[8][..], SEAMS[1].0),
        (&[8, 0, 0x12][..], &[][..], SEAMS[2].0),
        (&[0x12][..], &[][..], SEAMS[3].0),
        (&[0x0a, 1, 0x12][..], &[][..], SEAMS[4].0),
    ] {
        let mut bad = prefix.to_vec();
        bad.extend_from_slice(&overflow);
        bad.extend_from_slice(suffix);
        assert!(!check(&bad), "{bad:02x?}");
    }
}

#[test]
fn method_conventions_flags_generics_and_varargs_survive() {
    for conv in [0, 1, 2, 3, 4, 5, 9, 0x0b] {
        for flags in [0, 0x20, 0x60] {
            let raw = [conv | flags, 1, 1, 8];
            let sig = method(&raw).unwrap();
            assert_eq!(sig.has_this, flags != 0);
            assert_eq!(sig.explicit_this, flags == 0x60);
            assert_eq!(sig, cildec::MethodSig::parse(&raw).unwrap());
        }
    }
    let generic = method(&[0x30, 1, 1, 0x1e, 0, 0x13, 0]).unwrap();
    assert_eq!(generic.calling_convention, CallConv::Generic(1));
    for conv in [5, 0x0b] {
        let raw = [conv, 3, 1, 8, 0x41, 0x0e, 0x10, 8];
        let sig = method(&raw).unwrap();
        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.vararg_params.len(), 2);
    }
}

#[test]
fn unsupported_flags_and_misplaced_sentinels_fail_closed() {
    for conv in [0x80, 0x40, 0x15, 0x11, 6, 7, 8, 0x0a, 0x0c] {
        assert!(method(&[conv, 0, 0, 1]).is_err(), "{conv:x}");
    }
    for conv in [0, 1, 2, 3, 4, 9] {
        assert!(method(&[conv, 1, 1, 0x41, 8]).is_err());
    }
    assert!(method(&[5, 2, 1, 0x41, 8, 0x41, 8]).is_err());
    assert!(method(&[5, 0, 1, 0x41]).is_err());
    assert!(field(&[0x26, 8]).is_err());
    assert!(property(&[0x48, 0, 8]).is_err());
    assert!(method_spec(&[0x2a, 1, 8]).is_err());
}

#[test]
fn nested_generic_array_and_fnptr_types_survive() {
    let raw = [
        0x15, 0x12, 5, 2, 0x15, 0x11, 4, 1, 0x13, 0, 0x14, 0x1b, 0x61, 1, 1, 0x10, 8, 1, 0, 1, 0x7b,
    ];
    assert_eq!(
        type_spec(&raw).unwrap(),
        cildec::TypeSpecSig::parse(&raw).unwrap()
    );
    assert!(type_spec(&[0x0f, 0x20, 5, 1]).is_ok()); // void* with modopt
    assert!(type_spec(&[0x1b, 5, 2, 1, 8, 0x41, 0x0e]).is_ok());
}

#[test]
fn signed_array_bounds_keep_signed_width_semantics() {
    for (encoded, value) in [
        (&[0x7b][..], -3),
        (&[1][..], -64),
        (&[0x80, 1][..], -8192),
        (&[0xc0, 0, 0, 1][..], -268_435_456),
        (&[0x7e][..], 63),
        (&[0x80, 0x80][..], 64),
        (&[0xbf, 0x7f][..], -65),
        (&[0xbf, 0xfe][..], 8191),
        (&[0xc0, 0, 0x40, 0][..], 8192),
        (&[0xdf, 0xff, 0xbf, 0xff][..], -8193),
        (&[0xdf, 0xff, 0xff, 0xfe][..], 268_435_455),
    ] {
        let mut raw = vec![0x14, 8, 1, 0, 1];
        raw.extend_from_slice(encoded);
        let Type::Array(_, shape) = type_spec(&raw).unwrap().type_ else {
            panic!("not an array")
        };
        assert_eq!(shape.lo_bounds, [value]);
    }
}

#[test]
fn noncanonical_compressed_values_are_rejected() {
    for bad in [
        &[0, 0x80, 0, 1][..],       // method count
        &[0x10, 0x80, 1, 0, 1][..], // generic arity
    ] {
        assert!(method(bad).is_err());
    }
    for bad in [
        &[0x12, 0x80, 5][..],
        &[0x12, 0xc0, 0, 0, 5][..],
        &[0x13, 0x80, 0][..],
        &[0x15, 0x12, 5, 0x80, 1, 8][..],
        &[0x14, 8, 0x80, 1, 0, 0][..],
        &[0x14, 8, 1, 1, 0x80, 5, 0][..],
        &[0x14, 8, 1, 0, 1, 0x80, 0][..],
        &[0x14, 8, 1, 0, 1, 0xbf, 0xff][..], // -1 overlong
        &[0x14, 8, 1, 0, 1, 0xc0, 0, 0, 0][..],
        &[0x14, 8, 1, 0, 1, 0xdf, 0xff, 0xff, 0xff][..],
    ] {
        assert!(type_spec(bad).is_err(), "{bad:02x?}");
    }
    assert!(property(&[8, 0x80, 0, 8]).is_err());
    assert!(method_spec(&[0x0a, 0x80, 1, 8]).is_err());
}

#[test]
fn non_token_integers_are_not_mistaken_for_tokens() {
    assert_eq!(
        type_spec(&[0x13, 0xc4, 0, 0, 5]).unwrap().type_,
        Type::Var(0x0400_0005)
    );
    assert_eq!(type_spec(&[0x1e, 3]).unwrap().type_, Type::MVar(3));
    assert!(type_spec(&[0x14, 8, 1, 1, 3, 1, 1]).is_ok());
    assert!(method(&[0x10, 3, 0, 1]).is_ok());
}

#[test]
fn malformed_element_types_counts_and_compressed_prefixes_fail() {
    for byte in [0, 0x17, 0x1a, 0x21, 0x40, 0x41, 0x45, 0xff] {
        assert!(type_spec(&[byte, 8]).is_err(), "{byte:x}");
    }
    for raw in [
        &[0x12, 0xe0][..],
        &[0x13, 0xff][..],
        &[0x15, 8, 5, 1, 8][..],
        &[0x15, 0x12, 5, 0x7f, 8][..],
    ] {
        assert!(type_spec(raw).is_err());
    }
    assert!(method(&[0, 0x7f, 1, 8]).is_err());
    assert!(property(&[8, 0x7f, 8]).is_err());
    assert!(method_spec(&[0x0a, 0x7f, 8]).is_err());
}

#[test]
fn depth_modifier_size_and_work_limits_are_bounded() {
    let mut shallow = vec![0x0f; 31];
    shallow.push(1);
    assert!(type_spec(&shallow).is_ok());
    shallow.insert(0, 0x0f);
    assert!(type_spec(&shallow).is_err());
    let mut nested = vec![];
    for _ in 0..16 {
        nested.extend_from_slice(&[0x1b, 0, 0]);
    }
    nested.push(1);
    assert!(type_spec(&nested).is_err());
    let mut mods = vec![];
    for _ in 0..32 {
        mods.extend_from_slice(&[0x1f, 5]);
    }
    mods.push(8);
    assert!(type_spec(&mods).is_ok());
    mods.splice(0..0, [0x1f, 5]);
    assert!(type_spec(&mods).is_err());
    assert!(
        type_spec(&vec![8; 1_048_577])
            .unwrap_err()
            .to_string()
            .contains("byte limit")
    );
    let mut wide = vec![0x0a, 0xc0, 0x08, 0, 0]; // 524288 arguments
    wide.resize(524_293, 8);
    assert!(
        method_spec(&wide)
            .unwrap_err()
            .to_string()
            .contains("work limit")
    );
    let mut ordinary = vec![0x0a, 0x80, 0x80]; // 128 arguments
    ordinary.resize(131, 8);
    assert_eq!(method_spec(&ordinary).unwrap().args.len(), 128);
}

#[test]
fn deterministic_byte_mutations_never_panic() {
    for &(check, good) in SEAMS {
        for index in 0..good.len() {
            for byte in 0..=255 {
                let mut mutated = good.to_vec();
                mutated[index] = byte;
                let _accepted = check(&mutated);
            }
        }
    }
}
