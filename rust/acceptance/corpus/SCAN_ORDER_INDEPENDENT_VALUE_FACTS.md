# Scan-order independent value facts for member owners

A `newobj` or field access on a referenced value type depends on that type's value
fact. The fact prefetch walked the *member signature* (parameters and return type)
but not the member's **declaring type**, so a method that only allocated
`ExternalAllocations.Point` got the fact only if an earlier method in the same scan
happened to prime it. A fresh `decompile` therefore returned `Partial` while the
scan reported `Decompiled` for the same method: admission depended on lowering
order, which the corpus measurement must not do.

## Fix

- `value_type_tokens` now also collects the owner token of every `Method`/`Field`
  operand: the MemberRef parent when it is a `TypeRef`, and the definition of a
  `TypeSpec` parent when it instantiates a value type (`GenericInst` value or a
  single `ValueType`). `MethodSpec` operands follow their inner MemberRef the same
  way. These tokens join the existing operand set, so their `Struct`/`Enum` facts
  are primed for this method alone.
- No admission is widened: the same identity and value proofs run, they are now
  available on the first lowering.

## Evidence

- Owned `ExternalAllocationCases.Ops.RecoveredStruct` (a referenced `Point`
  allocation and field read) decompiled only after another method primed `Point`;
  a fresh `decompile` now returns the same `Decompiled` body as the scan
  (`return v3;`). The fixture contract was updated from an expected-partial entry
  to a positive assertion, and the external-allocation behavior driver now supplies
  populated value-type candidates (38 comparisons, 12 methods, mutants rejected).
- 488 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass.
