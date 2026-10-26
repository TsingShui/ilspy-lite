# Owner-scoped variables inside reference signatures

The structural reference slice admits VAR/MVAR descriptors in GenericInst class
and SZARRAY shapes. It does not admit a generic variable itself as an evaluation
value, constructed member calls, or generic receivers.

## Identity and body context

A variable's identity is its parsed input, declaring TypeDef or MethodDef owner,
and zero-based position. Names are presentation, not identity. VAR and MVAR are
separate, including !0 and !!0 in one signature. Nested types use their own
metadata GenericParam positions, including redeclared outer parameters.

Body entry constructs a checked TypeContext from the method definition and its
actual declaring type. Method generic arity must match GenericParam rows; positions
must be contiguous and unique; raw owners and supported flags are checked. Zero
generic arity, missing owners, out-of-range positions, invalid variance/flag
combinations and malformed context rows fail closed. Generic reference arguments,
returns and local signatures use the same context.

Interned canonical keys retain only scopes actually used by the signature.
Closed shapes remain identical across methods in the same image; List<!!0>
from different methods never shares identity. Evidence in both views includes
canonical signature bytes, actual variable owners and the body context/arity.
Runtime entry binding and the original constraints still apply; this is not a
second resolver or permission to erase constraints.

Nongeneric receivers and static generic owners can host these bodies. A constructed
generic reference can be copied, but cannot be used as an inferred generic field/
method receiver. Ordinary member-ref/field interpretation has no ambient caller
variable context; it cannot accidentally capture the caller's !0 or !!0.
Value-stack T, boxing, MethodSpec calls, allocation, covariance, generic receivers
and EH remain separate unsupported operations.

## Verification

- Public decompilation E2E was red at Identity<T>(List<T>); 16 positive methods
  recover in Debug/Release and Details/Readable. Eight negative instruction/value/
  member/receiver shapes remain Partial.
- Six Rust integration/E2E checks cover owner/position validation, distinct VAR/
  MVAR zero, independent method identities, arity mismatch/invalid flags, and
  scan/class/individual admission and stable aliases.
- Old pure open-array and open-generic-reference tests are explicitly promoted
  to positive tests. Their unrelated negative operation cases remain negative.
- The owned harness independently resolves 64 closed contexts (four combinations
  per method), using int/string/decimal/object substitutions and distinct type/
  method arguments. It specializes only exact type aliases and unchanged carrier
  signatures, not method statements. Recovered runtime parameter and return types
  must match the original constructed method exactly.
- 348 cases x two input configurations x two presentations = **1,392 behavioral
  comparisons**. Reference identity, reachable fields/collections/arrays, static
  state where applicable, enumeration observations and exceptions match. Null
  instance receivers are invoked through direct IL call, without adding callvirt.
- Four recovered-body mutants and four type/method-binding swap mutants fail.
- The readable behavior adapter has red/green coverage for comments ending in a
  period before a parameter assignment. Comments no longer masquerade as member
  access or have their identifiers rebound. Eight adapter checks pass.
- Full offline Rust regression, all-target Clippy, rustfmt and twelve corpus-tool
  E2E tests pass. Prior object/generic/array behavior also passes: 444 + 340 + 412
  comparisons and their mutation controls.

Only authored fixtures/carriers execute on the existing .NET 10 installation.
No original third-party method or archived compiler was executed.

## Fixed-scope result

Same 770,748 target bodies across 614 managed snapshots, including 319 with bodies.

| Metric | Vector baseline | Scoped reference variables |
|---|---:|---:|
| Decompiled | 159,515 | 159,587 |
| Partial | 611,233 | 611,161 |
| Aggregate unrecovered | 79.30% | 79.29% |
| Passing body-bearing images | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

**72 methods** newly recover in **16 inputs**, with zero previously recovered
statuses regressing. Cumulative same-scope gain is 24,553. The small gain is not
inflated by the disappearance of generic-signature first errors: most newly
admitted signatures expose value, receiver or member blockers. OpenRA counts are
unchanged (Game 914 recovered; Common 5,392). The per-image <= 1% goal is not met.

Evidence: rust/report/scoped-reference-roundtrip/, including red/green, boundaries,
behavior/adapter/mutation logs, corpus-scan/baseline.json and corpus-delta.json.
The fixed scan inventory remains
rust/report/core-type-roots/scope-policy/implementation-inventory.json.
No input acquisition, extraction, classification or input hash sweep was repeated.

```sh
python3 rust/tests/build-scoped-reference-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test scoped_references
python3 rust/tests/scoped-reference-roundtrip.py --mutation-control
python3 rust/tests/scoped-reference-roundtrip.py --readable --mutation-control
```

The next value-copy slice must keep generic evaluation values distinct from
references and integers, without inferring boxing, layout or managed addresses.
