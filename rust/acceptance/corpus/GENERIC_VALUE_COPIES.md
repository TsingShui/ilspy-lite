# Scoped generic value copies

This extends the owner-scoped variable representation to evaluation values.
It is not general concrete-struct, layout, boxing or managed-address support.

## Contract

Ty::GenericValue keeps its TypeId and has a separate evaluation category. It is
never classified as a reference, integer, floating primitive or managed address
merely because a particular runtime instantiation might have that shape.
Its identity remains input + declaring owner + position, with VAR/MVAR separate.

An ordinary load snapshots the original generic value. Argument/local assignment,
dup, branch-edge copies and return copy that value under the original target
type's transfer semantics. For reference instantiations this preserves identity;
for value instantiations it copies the value, including existing reference fields,
without deep cloning or boxing. Primitive bit/storage interpretation belongs to
that original target type/runtime, not the analyzer's architecture. Initlocals
defaults remain defaults of the original T, not constructor calls.

Opaque __cil_generic_HASH names identify these original generic types, not
wrapper objects, hidden boxed storage, or callbacks to original methods. Both
presentations retain canonical signatures, variable owners and body context.
Only equal scoped generic types are assignable in this slice. Unknown T cannot
be treated as null, an I4 literal, an object reference, a numeric operand, or a
different !0/!!0 without an explicit admitted operation/proof.

Constructed generic fields/calls/receivers, boxing/unboxing, concrete structs,
constraint-derived reference conversions, generic allocation, byrefs/addresses,
EH and unsupported opcodes keep their existing fallback checks. No selected
body delegates its implementation to original IL.

## Verification

The public E2E was red at CopyValue<T>(T). Seven scoped Rust tests now cover 23
positive methods plus seven unsupported operation shapes, owner/arity/position/
flags, scope identity, scan/class/method parity, and malformed null/integer/cross-
variable assignments and numeric operations on generic values. Existing pure
generic-variable identity is explicitly promoted to positive coverage.

The shared owned behavior harness executes actual recovered statements under
230 independently bound closed contexts: 23 methods x ten substitutions, including
int, string, decimal, object, double, an authored aggregate with reference fields,
enum, nullable, float and native integer. Mixed owner/method/nested contexts bind
different arguments. Parameter/return runtime types must match the original
constructed method exactly. The adapter specializes type aliases and carrier
signatures only; it does not reconstruct or borrow body logic.

- 1,350 cases x Debug/Release x Details/Readable = **5,400 comparisons**.
- Parameter reassignment, saved-before-reassignment values, conditional copies,
  defaults, null instance direct-call entry, nullable boxing at the test harness
  invocation seam, scalar bits, aggregate reference fields and reachable state
  are compared. Float/double signed zero and NaN payloads and decimal bit/scale
  patterns are observed, not only numeric Equals.
- **28 mutants are rejected**: four each for a lost recovered body, wrong generic
  owner binding, overwritten snapshot, same-content aggregate string replacement,
  double bits, float bits and decimal representation.
- Existing generic/vector/object profiles still pass 340/412/444 comparisons and
  their mutation controls. The shared harness follows reference fields inside
  owned aggregate values without treating that traversal as a deep-copy operation.
- Full offline Rust regression, all-target Clippy with warnings denied, rustfmt,
  eight readable-adapter checks and twelve corpus-tool E2E checks pass.

Only owned fixtures and carriers execute on the existing .NET 10 installation.
Static third-party coverage is not a claim of universal runtime equivalence.
Original-corpus runtime verification still requires the specific authorization.

## Fixed-scope result

The same 614 target managed snapshots, 319 body-bearing images, and 770,748 target
CIL bodies were scanned. No acquisition, extraction, hash sweep, role change or
inventory regeneration occurred.

| Metric | Scoped-reference baseline | Generic value copies |
|---|---:|---:|
| Decompiled | 159,587 | 159,719 |
| Partial | 611,161 | 611,029 |
| Aggregate unrecovered | 79.29% | 79.28% |
| Passing body-bearing images | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

There are **132 new recoveries in 30 inputs**, with zero previously recovered
statuses regressing. Cumulative genuine gain at the corrected common scope is
24,685. OpenRA is unchanged: Game 914 recovered / 3,853 Partial; Common 5,392 /
13,015. The large remaining generic receiver/member/value/dispatch blockers are
not counted as recovered just because signatures no longer fail first.

Evidence is under rust/report/generic-value-roundtrip/, including red/green,
boundaries, both behavior views/mutants, full regression, Clippy,
corpus-scan/baseline.json and corpus-delta.json. The previous scoped-reference
milestone's evidence remains historical. The independent scan inventory remains
rust/report/core-type-roots/scope-policy/implementation-inventory.json.

```sh
python3 rust/tests/build-scoped-reference-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test scoped_references
python3 rust/tests/scoped-reference-roundtrip.py --mutation-control
python3 rust/tests/scoped-reference-roundtrip.py --readable --mutation-control
```

These commands use the expanded owned fixture and save current behavior evidence
in generic-value-roundtrip. They use an existing CLI and cached owned compilation.
The types task and the per-image <= 1% goal remain incomplete.
