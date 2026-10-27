# Reference types merged at control-flow joins

At a control-flow join the CLI merges stack types; two reference types merge to
their common supertype (and a null with a reference becomes that reference). The
lift required byte-identical types (plus the null case), so a branch that produced
two different reference types stopped at
`incompatible evaluation stacks at join` (1,757 methods in the last fixed scan).

## Admission

- `join_reference_type` merges only reference-like values: identical types, null
  with a reference, either side assignable to the other (the existing proof,
  including local class chains and admitted external facts), a shared local base
  class found by walking the stored class chain, and otherwise `object`, which is
  the CLI's reference merge target.
- The merge changes only the synthetic slot's declared type. Incoming expressions
  are unchanged references: no conversion, cast, box or copy is inserted, and a
  later instruction that needs a specific type still goes through its own check, so
  an imprecise merge cannot emit a wrong member access - it keeps the refusal.
- Mixed integer widths, addresses, value types and differing stack depths keep the
  existing refusal.

## Evidence

- Red/green on the fixed corpus: the same F# image scanned before and after the
  change shows **22 methods** whose previous reason was
  `incompatible evaluation stacks at join` now `Decompiled` (both NDJSON scans kept
  in the evidence). Two were reviewed against production output, ILSpy and IL:
  `060001DF MoveNextImpl` (a generic state-machine body whose merged reference slot
  is later used through an explicit cast) and `060021AD` (a static initializer).
- The merge introduces no new operation; the reviewed bodies keep the same calls,
  casts and member order as ILSpy.
- 487 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans: `FSharp.Core` 7,471 -> 7,493 (+22),
  `System.Private.CoreLib` 15,067 -> 15,074 (+7), `OpenRA.Game` and
  `OpenRA.Mods.Common` unchanged.
