# Whole local struct value copies

This admits whole-value transfers, not layout reconstruction, arbitrary struct
instructions, managed addresses or value-type instance receivers.

## Definition and copy contract

A concrete value requires an exact local TypeDef (including an exactly bound
current-module TypeRef), a sealed nongeneric definition and an admitted core
System.ValueType base. Enum proof runs first: malformed enums never become opaque
structs, and valid enums retain their checked integer-stack normalization.
Core scalar/special definitions with distinct CLI categories are also excluded
from generic aggregate admission.

Struct attributes and instance-field signatures are checked. Byref-like markers
(including ambiguous same-name lookalikes), unknown constructed attribute owners,
modified fields, and direct void/open/managed-address fields fail closed. This is
bounded admission, not a complete loader or recursive layout verifier. No missing
external value definition is guessed. Raw signature and metadata limits remain
in force; facts and TypeIds are scoped to the parsed input.

Ty::Value has a separate stack category and an interned nominal TypeId. A load is
a stable value snapshot; stores, arguments, locals, direct-call boundaries and
returns copy the original typed value. Embedded object/array references retain
identity rather than being cloned. Source/target value identities must match:
there is no implicit conversion to null, an integer, another struct, or object.

Opaque __cil_value_HASH aliases denote the original value type, not boxes, byte
buffers, wrapper allocations or a hidden invocation of original method IL. Both
views retain exact canonical signatures. The target runtime owns its type layout;
no analyzer pointer width, offsets, sizeof or packing is used to implement copies.
A copy does not call a user-defined constructor. Reference-slot versus value-slot
semantics remain separate, including static-field stores.

## Identity normalization

The core-base reference helper follows ordinary case-insensitive assembly-name
matching while reusing existing full key/culture/content-type normalization.
Structural named class/value validation uses that same base proof. A dedicated
owned metadata case first failed with a lowercased System.Runtime reference and
then passed. This changes type-base proof only: reference-contract classification
and the frozen corpus exclusions were not altered.

## Owned verification and runtime observer diagnosis

Six Rust E2E/boundary checks pass:

- 63 positive methods on seven authored aggregate types in both configurations/
  views: reference-containing, nested, packed, explicit-layout union, readonly,
  user-defined parameterless constructor and empty structs.
- Fifteen independent fallback cases for explicit initialization, construction,
  boxing, managed references, ref structs, concrete generic values and unknown
  external values.
- Distinct nominal value identities, null/integer assignment refusal, malformed
  void/TypedByRef fields, core reference name normalization, and scan parity.

The behavior harness independently decodes type aliases and checks runtime types.
Selected bodies/helpers execute actual recovered text; only their type/field
definitions and separately authored test-data construction are borrowed. All
public mutable static state, including constructor counters, is compared.

**483 cases x Debug/Release x Details/Readable = 1,932 comparisons** pass, including
scalar bit patterns, embedded reference identity, saved-before-write values,
field/static replacement, layout-preserving copies and null-receiver exceptions.
Twenty-four mutants fail: lost helper behavior, overwritten snapshots,
same-content reference replacement, added construction, changed union bits and
omitted static writes.

The initial owned run stopped with SIGBUS on this ARM64 .NET 10 environment.
It was reproduced rather than dismissed as flaky. The debugger isolated
DataMisalignedException in System.Reflection.FieldAccessor.SetValue while setting
Packed.Wide during test-data setup, before running that packed method. Other
state observation runs could stop earlier. Evidence is retained in
native-crash-stack.log and behavior-trace.log.

State read/write helpers use exact field-leaf IL (ldfld/stfld/ldsfld/stsfld),
which respects the original packed type metadata. They do not read or invoke
selected method bodies. Packed/explicit-layout cases remain in the gate; nothing
was skipped. After this adjustment all profiles passed, and three additional
complete struct runs passed consecutively. Readonly fixture initialization remains
separate test-data setup. The result is not a claim about Reflection field access
on every runtime or about unrelated third-party implementations.

Full offline Rust regression, all-target Clippy -D warnings, rustfmt, eight
readable-adapter checks and twelve corpus-tool E2E checks pass. Enum (12,128),
scoped generic (5,400), generic reference (340), array (412) and object (444)
behavioral profiles and their mutation controls also pass with the shared observer.
Only authored fixtures/carriers execute on the existing toolchain.

## Unchanged-scope static result

Same 614 target managed snapshots, 319 body-bearing targets and 770,748 CIL bodies.
No acquisition, extraction, original-input hash sweep, classification adjustment
or independent-inventory regeneration occurred.

| Metric | Enum baseline | Struct copies |
|---|---:|---:|
| Decompiled | 181,893 | 186,454 |
| Partial | 588,855 | 584,294 |
| Aggregate unrecovered | 76.40% | 75.81% |
| Passing body-bearing targets | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

There are **4,561 newly recovered methods in 82 inputs**, with **zero previously
recovered statuses regressing**. Cumulative genuine gain on the corrected common
scope is 51,420. OpenRA.Game gains 222 (1,193 recovered / 3,574 Partial of 4,767);
Common gains three (5,532 / 12,875 of 18,407). External value definitions remain
independently gated; no layout is inferred just to improve that count.

Saved static probes inspect actual newly recovered CoreLib/Game/Common outputs.
No original third-party assembly or archived compiler was runtime-loaded or
executed. Static counts do not establish per-method runtime equivalence, and the
per-image <= 1% goal remains incomplete.

## Reproduction

```sh
python3 rust/tests/build-struct-copy-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test struct_copies
python3 rust/tests/struct-copy-roundtrip.py --mutation-control
python3 rust/tests/struct-copy-roundtrip.py --readable --mutation-control
```

Use --cli for an existing binary; unchanged authored compilation is cached.
Evidence: rust/report/struct-copy-roundtrip/, including red/green tests, native
observer diagnosis, repeated success, behavior/mutants, full regression, Clippy,
corpus-scan/baseline.json, corpus-delta.json and static-probes.json.

The scan inventory remains
rust/report/core-type-roots/scope-policy/implementation-inventory.json.
Managed locations/struct receivers, constructed generic member facts, external
definition binding and remaining operations are still pending.
