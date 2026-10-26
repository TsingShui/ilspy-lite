# Reference identity comparison

`ceq`/`beq`/`bne` over reference operands, and `cgt.un` against null, now lower
to explicit identity comparison.

## Contract

- Both operands must be reference-like (a reference stack value or `Null`), and
  their static types must be identical or one of them `Null`. Anything else stays
  unsupported rather than being compared by an assumed conversion.
- Equality emits `global::System.Object.ReferenceEquals(a, b)`; `bne` emits its
  negation. This is identity only: it never dereferences, never runs a
  user-defined operator, and never calls a virtual `Equals`.
- `cgt.un` with a null right operand is exactly "is not null" and emits
  `!ReferenceEquals(a, null)`. Roslyn emits this shape for `x != null`.
- Ordered comparison of references (`cgt`/`clt` and the branch forms) has no
  meaningful ordering and stays a fallback.
- `castclass`, `isinst` and external instance dispatch keep their own blockers, so
  a null test cannot silently introduce a cast or a call.

## Verification

The shared count-free contract covers the fixture in both views with three known
negatives (cast, type test, external dispatch). Focus tests assert that the
identity form is emitted and that no `==` operator appears.

The owned behavior harness compares results for a matrix that includes null, two
distinct instances of one type, and two *value-equal but distinct* strings, which
separates identity from value equality; it also compares the chosen reference of
`Choose` by identity and the exception type of every case. **137 comparisons** per
configuration and presentation, Debug and Release, Details and Readable. A mutant
that replaces identity comparison with `Object.Equals` is rejected in every
configuration - it is only detectable because the sample contains value-equal but
distinct strings.

Full offline Rust regression (54 suites) and all-target Clippy with warnings
denied pass.

## Unchanged-scope static result

Same 614 target managed snapshots, 319 body-bearing targets and 770,748 bodies.

| Metric | Local allocation | Reference identity |
|---|---:|---:|
| Decompiled | 258,033 | 260,486 |
| Partial | 512,715 | 510,262 |
| Aggregate unrecovered | 66.52% | 66.20% |
| Passing body-bearing targets | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

**2,453 newly recovered methods in 124 inputs**, zero previously recovered status
regressions. Cumulative genuine gain at the corrected common scope is 125,452.
OpenRA.Game +26 (1,516 recovered); OpenRA.Mods.Common +94 (5,969).

## Independent source review

An exec-capable delegate agent reviewed the default readable output against the
original IL over 16 bodies (owned methods plus corpus gained_methods samples):
**11 EQUIVALENT, 5 CRUDE-BUT-EQUIVALENT, 0 SUSPECT, 0 INSUFFICIENT-EVIDENCE**. It
confirmed that no sample introduces an operator, a dereference or a polarity
change, and that ordered/complex reference comparisons still fail closed. The only
remaining crudeness is layout level. Verdicts are in
rust/report/equality-roundtrip/review-source.md; with zero SUSPECT the counts
above are final for this milestone.

## Reproduction

```sh
python3 rust/tests/build-equality-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test equality
python3 rust/tests/equality-roundtrip.py --mutation-control
python3 rust/tests/equality-roundtrip.py --readable --mutation-control
```
