# Local reference-class allocation

This admits `newobj` for a locally defined reference class. It does not add
external, struct, abstract or generic allocation.

## Contract

`allocation_constructor` accepts a local `MethodDef` constructor only, and
requires: the name `.ctor`, instance flags with no static/abstract/virtual
combination, a declaring type that is a concrete nongeneric non-interface class
whose ancestry is provable in this image, and a default-convention signature
returning void with no varargs. Parameter types reuse the ordinary scoped
transfer rules.

The recovered form is `__cil_newobj_TOKEN(args...)` with the documented
semantics: evaluate arguments left to right, allocate exactly one fresh object
identity here, then run the exact constructor on that instance. Repeated reads of
the resulting reference observe the same instance; the intrinsic is not a
re-allocating getter. Argument order and any constructor side effects keep their
original position. External, struct, abstract, generic and open constructors keep
their own blocker.

## Verification

The shared count-free contract covers the fixture in both views. Focus tests
assert the emitted form, that arguments are evaluated before the allocation, and
that the four unsupported shapes stay Partial with distinct reasons.

The owned behavior harness compares observable state rather than identity, because
two correct runs legitimately allocate different instances: return value, node
fields, whether a field aliases an argument or the node itself, and exception
type. It additionally requires that two equal-content allocations obtained through
different methods are distinct instances and that a repeated call yields a fresh
one. **29 comparisons** per configuration and presentation, Debug and Release,
Details and Readable. A mutant that caches one instance across calls is rejected
in every configuration.

Full offline Rust regression (53 suites) and all-target Clippy with warnings
denied pass; the frozen expected-partial tables were refreshed where the new
capability moved a first blocker.

## Unchanged-scope static result

Same 614 target managed snapshots, 319 body-bearing targets and 770,748 bodies.

| Metric | Local virtual dispatch | Local allocation |
|---|---:|---:|
| Decompiled | 251,961 | 258,033 |
| Partial | 518,787 | 512,715 |
| Aggregate unrecovered | 67.31% | 66.52% |
| Passing body-bearing targets | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

**6,072 newly recovered methods in 132 inputs**, zero previously recovered status
regressions. Cumulative genuine gain at the corrected common scope is 122,999.
OpenRA.Game +26 (1,490 recovered); OpenRA.Mods.Common +120 (5,875).

## Independent source review

Per the goal constraint, an independent async read-only reviewer sampled the
default readable output against the original IL (10 owned methods plus 5 FSharp.Core
`New*` and 2 OpenRA.Game entries from `gained_methods`), with ILSpy as a secondary
reference only. Result: **15 EQUIVALENT, 2 CRUDE-BUT-EQUIVALENT, 0 SUSPECT**. The
crude verdicts are documented boolean normalisation only, not semantic drift. For
every sample the allocation count matched the number of `newobj` instructions, each
allocation kept its original position relative to surrounding calls/loads/stores,
constructor arguments kept their order and signature-matched types, and no
operation, check or throw point was dropped, merged or moved.

The review executed nothing: no CLI run, no test, no third-party assembly. Dumps
and verdicts live in rust/report/new-object-roundtrip/dumps/ and review-source.md.
With zero SUSPECT findings the counts below stand as final for this milestone.

## Reproduction

```sh
python3 rust/tests/build-new-object-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test new_objects
python3 rust/tests/new-object-roundtrip.py --mutation-control
python3 rust/tests/new-object-roundtrip.py --readable --mutation-control
```
