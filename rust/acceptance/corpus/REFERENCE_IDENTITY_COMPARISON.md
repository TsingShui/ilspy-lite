# Reference identity comparisons between different static types

`ceq`/`beq`/`bne` on two references compare identity. C# `ReferenceEquals` takes
both operands as `object`, which is an implicit reference conversion and runs no
user code, so no common static type has to be proved: the comparison is on the
values, not on their declared types. The lift already emitted
`Object.ReferenceEquals(a, b)`; it refused only when the two static types differed
and neither was null.

## Admission

- The identical-or-null gate is removed for equality and inequality. Ordered forms
  (`cgt`/`clt` and their `.un` variants) and the "is not null" `cgt.un` special case
  keep their existing handling, so no ordering or null test is re-derived.
- `ReferenceEquals` boxes nothing here: both operands are references (or null).
  Interface, base and unrelated reference static types all compare by identity, and
  null still flows through as a plain null reference.

## Evidence

- Red: owned `RelationCases.Ops::SameReference(Base, object)` returned
  `Partial` on `reference comparison needs identical or null static types`
  (recorded log).
- Green: it decompiles to `System.Object.ReferenceEquals(first, second)`, and
  `DifferentReference` (C# `!=`, compiled as `ceq; ldc.i4.0; ceq`) decompiles as the
  branch-free boolean combination.
- The relation driver executes both methods against the owned library with null,
  equal and distinct instances: 592 comparisons / 56 closed selections, with all
  mutants rejected (DeclaredOverload, ExternalGeneric, FloatLe, DoubleSum).
- 480 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans with the acceptance policy: `OpenRA.Mods.Common`
  15,629 -> 15,656 (+27), `OpenRA.Game` 4,024 -> 4,028 (+4), `FSharp.Core` 7,416.
- The relation driver's selection now excludes the methods owned by the
  value-construction and general-array drivers, whose helper bindings belong to
  those drivers; this narrows one driver's scope, not an assertion.
