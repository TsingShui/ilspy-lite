# Runtime type checks: castclass and isinst

Both lower to an explicit runtime check for locally defined classes. Neither is
ever erased or replaced by a static conversion.

## Contract

- The target must be a local, concrete, nongeneric class definition. External and
  interface cast targets keep their own blocker, because they need type facts this
  image does not carry.
- The operand must be an admitted reference (or `null`); value operands fall back.
- `castclass T` renders as `((T)(object)value)`: a runtime check that yields the
  same reference on success, passes `null` through unchanged, and throws
  `InvalidCastException` for an incompatible instance. The `(object)` hop is an
  identity conversion on references; it keeps the text legal when the static types
  are unrelated, which IL permits and C# does not.
- `isinst T` renders as `((object)value as T)`: a runtime type test that yields
  `null` on failure, never throws, and returns the same reference on success. No
  user code runs in either case.
- Result type is the exact cast target, so a later field access or call still
  checks against the right owner.

## Verification

The shared count-free contract covers the fixture in both views; the frozen
expected-partial set keeps the string cast, the interface call and the interface
operand gated. Focus tests assert that the cast target survives in the readable
text and that the type test uses the null-yielding form.

The owned behavior harness describes every result by identity relative to the
inputs (null / one of the arguments / a type name) instead of by content, so a
cast that returned a copy or a fresh instance would be caught. It runs a matrix of
`null`, base instance, derived instance, unrelated instance and a string over all
eight recovered methods: **31 comparisons** per configuration and presentation,
Debug and Release, Details and Readable, including the `InvalidCastException`
cases. A mutant that turns the null-yielding type test into a throwing cast is
rejected in every configuration.

Full offline Rust regression (55 suites) and all-target Clippy with warnings denied
pass; frozen expected-partial tables were refreshed where the new capability moved
a first blocker (objects, generic/vector/scoped/struct/enum/virtual-call/newobj
fixtures).

## Unchanged-scope static result

Same 614 target managed snapshots, 319 body-bearing targets and 770,748 bodies.

| Metric | Reference identity | Runtime type checks |
|---|---:|---:|
| Decompiled | 260,486 | 264,677 |
| Partial | 510,262 | 506,071 |
| Aggregate unrecovered | 66.20% | 65.66% |
| Passing body-bearing targets | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

**4,191 newly recovered methods in 109 inputs**, zero previously recovered status
regressions. Cumulative genuine gain at the corrected common scope is 129,643.
OpenRA.Game +4 (1,520 recovered); OpenRA.Mods.Common +5 (5,974).

## Independent source review

Two independent agents reviewed this slice against the original IL, both with
**0 SUSPECT**:

- `scout` over the eight owned methods from pre-dumped source/IL pairs: 7
  EQUIVALENT, 1 CRUDE-BUT-EQUIVALENT (the int-returning `is` guard, boolean
  normalisation only).
- `delegate` over the eight owned methods plus five corpus `gained_methods`
  samples: runtime check retained, same reference on success, null flows both ways,
  `InvalidCastException` thrown at the original cast site, `isinst` yields null
  without throwing, and external/interface/generic/value-type targets still fall
  back wholesale.

Verdicts are in rust/report/cast-roundtrip/review-source.md; with zero SUSPECT the
counts above are final for this milestone.

## Reproduction

```sh
python3 rust/tests/build-cast-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test casts
python3 rust/tests/cast-roundtrip.py --mutation-control
python3 rust/tests/cast-roundtrip.py --readable --mutation-control
```
