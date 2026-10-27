# Unmanaged pointers

A pointer's pointee fixes both its C# spelling and its dereference width, so the
type is kept as an interned pointee descriptor (`Ty::Pointer(PointerId)`) rather
than as an opaque native integer. `ELEMENT_TYPE_PTR` with custom modifiers stays
refused; no other signature shape changes.

## Admitted operations

- Declarations: pointer parameters, returns, locals and fields, plus `ldarg`,
  `ldloc`, `stloc`, `ldfld`, `stfld`, `ret` and argument passing through the
  existing value rules. `Ty::stack()` keeps the pointer category, so a pointer is
  never rewritten into `int32` when it is pushed.
- Dereference: `ldind.*`/`stind.*` on a pointer read/write `*pointer` with the same
  width and kind checks the managed-location path uses (`ldind.i4` still needs a
  4-byte pointee, `ldind.ref` a reference pointee).
- Arithmetic: IL adds a *byte* offset and subtracts byte addresses. C# `T* + n`
  scales by the pointee and `T* - T*` divides by it, so the lift emits the
  operation on `byte*` and casts the result back (or keeps the byte difference for
  pointer subtraction). This preserves pointer scaling exactly; a naive
  `pointer + offset` would have read the wrong element, which the mutation control
  exercises.
- Comparisons: pointer identity between identical pointee types, pointer against
  the null constant, and pointer against an integer null-pointer constant (the
  bit-preserving cast is written out).
- Conversions: pointer to `nint`/`nuint`/`long`/`int` through the native width,
  with the truncation of `conv.i4` explicit.
- Integer arithmetic now widens a narrower operand to the wider one explicitly,
  because Roslyn's pointer lowering produces mixed `native int`/`int32` additions
  and multiplications. Sign rules follow the operand's own storage; mixing int64
  and native spellings has no single C# result type and stays refused.

## Refused

`localloc`, `initblk`, `cpblk`, `calli`, function-pointer signatures,
`TypedReference`, pinned/modified locals and pointer `.ovf` forms keep their
refusals: they need lifetime, block or calling-convention semantics this group
does not model.

## Evidence

- Red: the owned pointer cases were `Partial` on
  `unsupported signature type needs IL fallback` (recorded log).
- Green: `ReadPointer`, `ReadWrite`, `SamePointer`, `NullPointer`, `OffsetValue`
  and `Difference` decompile with `int*` declarations, `*pointer`, `(byte*)`
  scaling and `(nint)` differences. Rust test:
  `reference_relations::unmanaged_pointers_keep_their_pointee_and_operation_forms`.
- Behavior driver `tests/pointer-roundtrip.py` compiles the recovered bodies with
  `-unsafe` into a separate carrier and executes them against owned stack memory,
  comparing return values and mutated memory with the fixture: 9 comparisons per
  configuration. Mutants that drop the in-place write and that scale the byte
  offset a second time are rejected in release and debug.
- 481 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass; the existing native tests were updated because native arithmetic is now
  recovered (an implicit width change into a native return slot stays Partial).
- Targeted scan: `System.Private.CoreLib` 13,699 -> 14,122 (+423); `FSharp.Core`,
  `OpenRA.Game` and `OpenRA.Mods.Common` unchanged. Batch scan follows.
