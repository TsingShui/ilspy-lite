# Closed vector reference signatures

This extends the structural interner with closed SZARRAY references, without
adding array allocation, element access, covariance or address operations.

## Semantics

Byte, string, object, primitive, nominal value/reference, closed generic and jagged
vector descriptors can occupy reference slots. Loading/storing/returning the
array copies its reference, not its elements, storage, bounds or element layout.
A struct/enum element descriptor does not enable a struct/enum evaluation value.
Array arguments, locals, fields, null and object upcasts follow the same exact
assignment rules as other admitted references.

Existing int[]/uint[] Ty forms and their instructions are unchanged. Additional
vectors use Ty::StructuralRef with image-isolated interned TypeId identities.
Different element types are distinct. Declared covariance is not inferred;
a signature already declared object[] can still receive a runtime string[] or
owned Cell[] under the target runtime's ordinary parameter binding. Both are
tested without changing the reference or runtime type.

Open variables, pointer/function-pointer/modifier-bearing descriptors and
non-vector shapes remain unsupported. Method/field/local raw checking and all
instruction, receiver, readonly, CFG and exception checks remain in force.
Exact canonical type-spec bytes identify opaque aliases in both output views;
aliases are not wrappers, allocations, host-layout guesses or an IL interpreter.

## E2E and owned behavior

- The public array-copy E2E first failed on byte[] field exchange.
- Four array Rust integration/E2E checks pass: 21 positive methods in both owned
  configurations/views, seven unsupported operation shapes, mismatched signed/
  unsigned element identities, and scan/method status parity.
- The old object's pure object[] upcast is explicitly a positive regression, not
  a silently omitted negative. Five other old unsupported operation sentinels
  still reject. No original method status was manually relabeled.
- A shared owned harness binds aliases by independently decoding only their type
  descriptors. Recovered parameter/return runtime types must match the original
  owned signatures. All selected bodies/helpers execute actual recovered text.
- 103 cases x Debug/Release x Details/Readable = **412 behavioral comparisons**.
  Exact reference identity, related reachable state, array/collection contents,
  scalar bit patterns, struct field reference identity, static state and exception
  types match. Null and aliasing fields, runtime-covariant input arrays, enum/
  struct/native/double arrays, nested generic elements and shared jagged children
  are included.
- Four helper mutants, four extra element writes, four positive/negative-zero bit
  mutations and four same-content string-reference replacements inside struct
  elements are rejected. The selected Forward never borrows original Identity.
- The generic profile still passes its 340 comparisons and eight mutation checks;
  existing object behavior passes 444 comparisons and four helper controls.
- Full offline Rust regression, all-target Clippy -D warnings, rustfmt, readable
  adapter checks and twelve corpus-tool E2E checks pass.

No original third-party input was executed. Newly authored fixtures/carriers use
the existing .NET 10 toolchain; unchanged fixture compilations remain cached.

## Fixed-scope static results

Same 614 target managed snapshots, 319 body-bearing snapshots and 770,748 target
CIL bodies. No input acquisition, extraction, hash sweep, role reclassification or
independent inventory regeneration occurred.

| Metric | Closed generic baseline | Vector references |
|---|---:|---:|
| Decompiled | 155,281 | 159,515 |
| Partial | 615,467 | 611,233 |
| Aggregate unrecovered | 79.85% | 79.30% |
| Passing body-bearing snapshots | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

There are **4,234 new recoveries in 171 inputs**, with **zero previously recovered
statuses regressing**. Cumulative genuine gain at the corrected common scope is
24,481. Later signature/member/dispatch/value/EH blockers stay Partial. Changes in
first-error buckets are not counted as method recoveries.

OpenRA.Game gains 26 (914 recovered, 3,853 Partial / 4,767). Common gains 68
(5,392 recovered, 13,015 Partial / 18,407). The goal remains far from complete;
all real implementation images must independently meet <= 1%.

## Reproduction and evidence

```sh
python3 rust/tests/build-array-reference-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test array_references
python3 rust/tests/generic-reference-roundtrip.py --arrays --mutation-control
python3 rust/tests/generic-reference-roundtrip.py --arrays --readable --mutation-control
```

Use --cli for an already-built CLI. Evidence is under ignored
rust/report/array-reference-roundtrip/, including red/green cases, behavior and
mutation logs, full regression, Clippy, corpus-scan/baseline.json and
corpus-delta.json. The latter checks unchanged input ids/origins/body counts and
exact MethodDef token sets against the saved closed-generic results.

Continue scans with
rust/report/core-type-roots/scope-policy/implementation-inventory.json.
Open owner/method variables, exact enum/value/address transfers and generic
member facts remain pending under the approved design; the types task is not
complete.
