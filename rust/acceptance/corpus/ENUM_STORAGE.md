# Exact local enum storage

This slice lowers local enum storage and integer-stack transfers. It does not
infer external enum layouts, add boxing or permit enum/value receivers.

## Definition and transfer contract

An enum requires an exact current-module TypeDef (or an exactly resolved local
TypeRef), a sealed nongeneric definition, and an admitted core System.Enum base.
The core-reference check reuses full assembly identity normalization, including
key/culture/content type; locally defined core bases use the existing qualified
core-definition policy. These identity labels are not publisher authentication.

All owned fields are checked. The admitted shape has exactly one ordinary public
special/RTSpecialName instance field named value__, with an unmodified fixed-width
integer FieldSig. Static fields must be literal and not initonly. The supported
storage types are sbyte, byte, short, ushort, int, uint, long and ulong. There is
no default-to-int rule. Bool/char/native/float/reference storage and modified or
conflicting fields fail closed. Definition facts are cached only in the parsed
image. Other ordinary value types do not become enums.

Declared slots retain the enum's exact local token and storage width. Loads
normalize to I4/I8 CLI stack bits with the correct sign/zero extension; stores
explicitly truncate to the declared underlying storage and restore enum identity.
Unsigned 32/64-bit storage does not change an integer opcode's interpretation.
Arguments, locals, fields, direct call boundaries and returns all use these rules.
Opaque __cil_enum_TOKEN aliases denote original input types, not wrappers or boxes.
Both views retain exact token/underlying-type evidence.

Already-admitted integer instructions operate on these normalized stack values.
This includes the actual compiler-generated narrow enum bitwise-OR shape, which
does not contain an extra conversion opcode. Explicit conv.i1/u1/i2/u2, enum array
allocation, unknown external enum storage, boxing/unboxing, value receivers,
addresses and EH still require their own capabilities. No opcode is silently
erased and no method is proxied to original IL.

## Owned validation

The public E2E first failed at the sbyte-backed Identity method. Six Rust
integration/E2E checks now cover:

- 96 positive methods across eight underlying types, Debug/Release and both views.
- 29 unsupported external-storage, boxing, allocation and explicit narrow-opcode
  shapes, with complete fallback evidence.
- Missing/duplicate/wrongly flagged value__ fields and invalid underlying types.
- Forged core names/keys/content type, an unsealed definition, and exact local
  TypeRef-to-TypeDef canonicalization.
- Scan enumeration/status parity with individual method output.

The behavior harness uses actual recovered statements and independently checks
each enum token and underlying type through the owned runtime metadata. Only type
and field definitions are borrowed; selected Forward targets recovered Identity.
Per-type carriers avoid overload/name collisions, and method rebinding never
redirects the original state fields to an empty carrier.

**3,032 cases x Debug/Release x Details/Readable = 12,128 comparisons** pass:
enum bit patterns, zero/high/sign boundaries and unnamed values, signed/unsigned
widening, conditional/argument/local copies, OR operations, instance/static state,
null-receiver exceptions and recovered helper calls. Sixteen mutation controls
reject borrowed helpers, wrong sign extension, wrong unsigned widening and a
lost static-field write.

Full offline Rust regression, all-target Clippy -D warnings, rustfmt, eight
readable-adapter checks and twelve corpus-tool E2E checks pass. Previous generic
value/reference, array and object behavioral profiles and mutation controls also
pass. Only authored fixtures/carriers execute on the existing .NET 10 toolchain;
no original third-party image or archived compiler was executed.

## Fixed-scope production result

Same 614 implementation snapshots, 319 with target bodies and 770,748 target CIL
bodies. No input acquisition, extraction, hash sweep, inventory regeneration or
reference-scope change occurred.

| Metric | Generic-copy baseline | Local enum storage |
|---|---:|---:|
| Decompiled | 159,719 | 181,893 |
| Partial | 611,029 | 588,855 |
| Aggregate unrecovered | 79.28% | 76.40% |
| Passing body-bearing images | 33 / 319 | 33 / 319 |
| Incomplete image scans | 0 | 0 |

**22,174 newly recovered methods in 160 inputs**, with **zero previously recovered
statuses regressing**. Cumulative genuine gain at the corrected common scope is
46,859. First-failure bucket changes do not count as recovery: external layouts,
other structs and later member/dispatch/region blockers remain Partial.

OpenRA.Game gains 57 (971 recovered / 3,796 Partial out of 4,767; 79.63% unrecovered).
Common gains 137 (5,529 / 12,878 out of 18,407; 69.96%). Large gains include compiler
assemblies using local flag enums. Static probes retain actual output for
CSharp.Binder.get_IsSemanticModelBinder, OpenRA.MouseButtonPreference.get_Action
and Common.WebServices.get_ModVersionStatus. These probes did not execute inputs.

The release incremental build took 4.73 seconds; the four-worker implementation
scan took about 14 seconds. The per-image <= 1% goal remains incomplete.

## Reproduction and evidence

```sh
python3 rust/tests/build-enum-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test enums
python3 rust/tests/enum-roundtrip.py --mutation-control
python3 rust/tests/enum-roundtrip.py --readable --mutation-control
```

Use --cli for an already-built CLI; unchanged owned compilation remains cached.
Evidence is under rust/report/enum-roundtrip/: red/green/boundary tests, behavior/
mutation logs, full regression, corpus-scan/baseline.json, corpus-delta.json and
static-probes.json plus the retained probe output.

Continue scanning with
rust/report/core-type-roots/scope-policy/implementation-inventory.json.
Concrete struct copies/addresses, constructed receivers/members and external
type facts remain pending under the approved architecture.
