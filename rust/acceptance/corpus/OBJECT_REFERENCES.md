# Object reference milestone

This is one bounded semantic slice of the fixed implementation-corpus goal,
not completion of the <= 1% per-assembly target. The numbers below retain this
milestone's original recorded scope. A later exact local-marker correction
identified an additional reference input: at the corrected common scope the
object gain is 5,503, not 5,540. The 37 other gains belong to that reference
contract, not implementations. See [core type roots](CORE_TYPE_ROOTS.md) for
complete same-scope reconciliation and the subsequent milestone.

## Implemented scope

- ELEMENT_TYPE_OBJECT is an ordinary reference type, separate from Null,
  integer/value categories and managed addresses.
- Object parameters, locals, returns, copies and instance/static fields preserve
  the existing reference, including null. Copies do not allocate or box.
- Already admitted string/class/int[]/uint[] references can upcast to object.
  Object-to-string/class assignment is not an implicit downcast.
- Reference static stores use the same typed assignment checks as other reference
  transfers. This also admits bounded existing string/class/array static stores.
- Stable snapshots, field receiver null checks and original operation order stay
  explicit. Readable and Details output and the production scan share admission.

No boxing/unboxing, object[] signatures, generic/value-type lowering, general
allocation, virtual dispatch, mixed-type joins or EH was added. Unsupported
methods retain their original IL and failure reasons.

## E2E and behavioral evidence

The owned ObjectCases fixture is compiled in Debug and Release. Its 19 positive
methods cover identity, null, object arguments/locals/returns, same-type branches,
string/class/array upcasts, field snapshots, argument/field writes, static object
and typed-reference writes, and a caller whose helper is independently recovered.

- Seven Rust integration/E2E tests pass. Both presentations and scan/token results
  are checked for identical admission.
- Six unsupported-operation sentinels remain Partial: boxing, unboxing, downcast,
  virtual dispatch, general allocation and object[] signatures.
- Malformed IL/signature cases reject object arithmetic, fake typed field
  receivers, integer returns/static stores without boxing and object-to-string
  assignment without a cast.
- Actual recovered Details and Readable statements are compiled and executed in
  both configurations: 111 cases per configuration/view, 444 comparisons total.
- Behavior checks compare return reference identity, every relevant instance and
  static field, array contents and exception type. Null receivers are invoked
  using an exact IL call for the original owned fixture, not a reflection-added
  null check that could mask method behavior.
- Selected/support helpers run their recovered bodies. Mutating recovered
  Identity to return null must make a Forward-only behavior check fail. All four
  configuration/view mutation controls reject the mutant.
- Full Rust regression, clippy with warnings as errors and rustfmt checks passed.
  Focused object tests and all four behavior matrices were rerun after adding
  the typed-static reference cases.

The test adapter rebinds names/types/selected helper targets, not method logic.
Original fixture types provide owned state declarations only. No third-party
corpus body or entry point was executed. Local .NET 10.0.12 behavior is finite
owned-runtime evidence, not universal or cross-runtime equivalence proof.

## Fixed implementation-corpus scan

The same 361 reference-only exclusions and immutable input snapshots are used
before and after. No new acquisition, extraction or input-hash sweep was needed.
Every one of the 777,951 independently inventoried implementation bodies remains
accounted for. All 615 target managed snapshots were scanned (320 have bodies).

| Metric | Before | After |
|---|---:|---:|
| Decompiled bodies | 135,566 | 141,106 |
| Unrecovered bodies | 642,385 | 636,845 |
| Aggregate unrecovered rate | 82.57% | 81.86% |
| Body-bearing images satisfying <= 1% | 32 / 320 | 33 / 320 |
| Incomplete image scans | 0 | 0 |

There are 5,540 newly recovered bodies across 173 images and zero status
regressions among previously recovered bodies. Of the gains, 5,354 formerly hit
the signature error and 186 formerly hit the integer-operation check on
reference static stores. These are actual status improvements at unchanged scope,
not the separate reference-input classification adjustment.

The signature first-failure bucket shrinks from 252,137 to 225,093, a decrease of
27,044. Only 5,354 of these become recovered bodies; the others reveal further
blockers. A smaller first-error bucket is not counted as recovery by itself.

| Supplemental assembly | Recovered before | Recovered after | Gain |
|---|---:|---:|---:|
| OpenRA.Mods.Common | 4,928 | 5,233 | 305 |
| OpenRA.Game | 824 | 855 | 31 |

This release incremental build took 4.75 seconds; the four-process implementation
scan took 15.54 seconds on the local macOS ARM64 host. These are recorded local
runs, not general performance guarantees. No full ILSpy solution build was used.

## Reproduce the owned gate

```sh
python3 rust/tests/build-object-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test objects
python3 rust/tests/object-roundtrip.py --mutation-control
python3 rust/tests/object-roundtrip.py --readable --mutation-control
```

The behavior driver uses the existing Debug Rust CLI by default; pass
`--cli rust/target/release/ilspy-lite` to select the existing release binary.
Unchanged owned fixtures, adapter and carriers reuse their compiler cache.
The driver never builds Rust or restores packages.

Evidence stays under ignored `rust/report/object-roundtrip/`: red/green logs,
final behavior matrices and negative controls, full regression/clippy/build logs,
`corpus-scan/baseline.json`, original per-method result files, and
`corpus-delta.json` listing every newly recovered method and changed image.

The active types milestone remains in progress. Structural signatures, scoped
generic instantiation, enum/value semantics and addresses still need their
separate implementations and E2E evidence under the approved architecture.
