# Original `this` provenance across control-flow joins

A constructor may only run on its own `this` (delegation) or on the immediate
base's constructor (`: base(...)`). The lift tracked that provenance by the
*emitted value name* inside one basic block: `ldarg.0` registers the name it was
pushed under. At a join the stack is re-seeded with synthetic slot names
(`s{block}_{slot}`), so a base-constructor call reached through a branch lost the
provenance and was refused with `constructor call needs same owning constructor and
original this` - even when the receiver was still the same entry `ldarg.0`, as in
compiler-optimized constructors that branch while evaluating a base argument
(1,807 methods in the last fixed scan).

## Admission

- `this_slots` records, per basic block, the stack index that carries the original
  `this`. It is propagated to successors only when the value is still on the stack
  and every predecessor agrees on the same index; otherwise provenance is dropped
  (a join of different values never claims to be `this`).
- Both the analysis pass and the emission pass seed `original_this` with the
  synthetic slot name of that index, so the existing check applies unchanged: the
  receiver must still match the constructor's owner type exactly, must be that
  original receiver, and a base call must still target the immediate base and a
  delegation the same type.
- No new call shape is admitted: the same `__cil_ctor_call_`/base rules are used,
  only the provenance information now survives a join.

## Evidence

- Red: ILSpy's own `MemberResolveResult..ctor` was `Partial` on
  `constructor call needs same owning constructor and original this` (recorded
  log); it now decompiles, keeping the `: base(...)` call on `this` and the field
  writes in their original order.
- 487 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass.
- Targeted scans: `ICSharpCode.Decompiler` 9,253 -> 9,353 (+100 against the last
  measurement of that image after the external-field group),
  `OpenRA.Mods.Common` 16,344 -> 16,387 (+43), `OpenRA.Game` 4,130 -> 4,151 (+21),
  `System.Private.CoreLib` 15,074 -> 15,088 (+14), `FSharp.Core` unchanged.
- Constructor delegation/base semantics keep their existing owned drivers
  (`constructors`, value-construction and struct-receiver suites); this group only
  preserves which value is `this`, it does not add a construction operation.
