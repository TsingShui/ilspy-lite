# Locally verified virtual dispatch

This slice admits `callvirt` when the receiver is a class verified in the same
image. It does not add opaque external dispatch, allocation or constrained calls.

## Contract

`callvirt` is a distinct operation, never folded into the direct call form:

- The receiver's declared type must be a locally verified class (`Ty::Class`),
  so the ancestry check used for the receiver is provable from this image.
- The receiver must be assignable to the declared owner by the existing exact
  class-chain rule; value-type receivers, managed locations, interfaces and
  opaque external types stay rejected with their own reasons.
- Constructors and intrinsics are not virtual: `callvirt` on them is invalid IL
  and is rejected instead of being turned into a dispatch.
- The recovered form is `__cil_callvirt_TOKEN(receiver, args...)` with the
  documented semantics: evaluate the receiver and arguments once in order, check
  the receiver for null, then dispatch to the declared method in the target
  runtime. Both views keep the token and the note.
- Arguments and results reuse the existing transfer rules; the evaluation order
  of receiver-first is preserved from the original stack.
- `newobj`, `constrained.`, interface receivers and opaque external dispatch
  (e.g. `List<T>.Add`) remain independent gates.

## Verification

`rust/tests/virtual_calls.rs` uses the shared count-free contract with three
states: recovered, known-unsupported `(owner, method, reason)`, and legitimate
bodyless declarations (the abstract `Shape.Area`). Six fixture tests plus two
focus tests cover:

- `callvirt` produces `__cil_callvirt_`, mentions the null check, and never
  produces the direct-call or memberref-call form.
- Rewriting the original opcode to `call` drops the marker, proving the emitted
  form is driven by the instruction rather than the resolved target.
- A value-type receiver, `constrained.` call, `newobj`, interface call, opaque
  external dispatch and array element type each keep a distinct blocker.
- Scan enumeration and per-method output share one admission path.

The owned behavior harness compiles the recovered bodies and supplies the
target-runtime dispatch the output intentionally leaves to the runtime (the
receiver expression calls the same virtual member). Bodies are not synthesized:
only `__cil_callvirt_TOKEN` is rebound to the equivalent member call.

**44 comparisons** per configuration and presentation (8 recovered methods with
one and two `Shape` arguments) run in Debug and Release, Details and Readable.
Results, dispatch to the concrete override and `NullReferenceException` on a null
receiver all match. A mutant that downgrades dispatch to a direct call to the
declared method is rejected in every configuration.

Full offline Rust regression (52 suites), all-target Clippy with warnings denied
and rustfmt pass. Prior owned profiles (object, generic reference, vector, scoped
variables, generic values, enums, struct copies, struct receivers) still pass.

## Unchanged-scope static result

Same 614 target managed snapshots, 319 body-bearing targets and 770,748 bodies.

| Metric | Struct-receiver baseline | Local virtual dispatch |
|---|---:|---:|
| Decompiled | 204,281 | 251,961 |
| Partial | 566,467 | 518,787 |
| Aggregate unrecovered | 73.50% | 67.31% |
| Passing body-bearing targets | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

**47,680 newly recovered methods in 151 inputs**, zero previously recovered
status regressions. Cumulative genuine gain at the corrected common scope is
116,927. OpenRA.Game gains 92 (1,464 recovered); OpenRA.Mods.Common gains 199
(5,755). The remaining first blockers are opaque external dispatch, interface and
constructed-generic receivers, exception regions and allocation.

## Reproduction

```sh
python3 rust/tests/build-virtual-call-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test virtual_calls
python3 rust/tests/virtual-call-roundtrip.py --mutation-control
python3 rust/tests/virtual-call-roundtrip.py --readable --mutation-control
```

Evidence: rust/report/virtual-call-roundtrip/. The scan inventory remains
rust/report/core-type-roots/scope-policy/implementation-inventory.json.
