# Boxed-value `isinst`/`castclass`

`isinst`/`castclass` with a **value-type** target is a boxed-value test: the runtime
checks the box's exact type and the result is still an object reference (the same
box on success, null for a failed `isinst`). The cast path refused every value-type
target as a "reference cast", which also blocked the common `isinst S; unbox.any S`
pattern (for example `obj is AssemblyQualifiedTypeName` inside a struct's `Equals`).

## Admission

- The cast arm first asks `boxed_value_type` for the target; when it yields a
  `Ty::Value`/`Ty::Enum` the operation is emitted with the exact C# type test:
  - `isinst S` -> `((object)(x is S ? x : null))` (same box on success, null on
    failure, no exception);
  - `castclass S` -> `((object)(x is S ? x : throw new InvalidCastException()))`
    (same box on success, the runtime exception on a failed test).
  The pushed type is `object`, matching the CLI's object reference result, so a
  following `unbox.any S` keeps its exact unbox semantics.
- Reference targets keep the previous `as`/castclass emission. Value targets that
  cannot be admitted keep the stated refusal, so the earlier external-reference
  boundary is unchanged for under-proved structs.

## Evidence

- The decompiler image 9,429 -> 9,466 (+37), `System.Private.CoreLib` 16,762 ->
  16,841 (+79), `OpenRA.Game` 4,282 -> 4,305 (+23); `FSharp.Core` and
  `OpenRA.Mods.Common` unchanged.
- 488 Rust tests, `cargo fmt --check` and clippy `-D warnings` pass. The owned cast
  drivers still cover reference casts; the value-type test is emitted only where the
  target's value identity is admitted.
