# Native-width integer operations

`System.IntPtr`/`System.UIntPtr` values live on the CLI's native integer stack
category and take part in the same wrapping arithmetic and comparisons as the
32- and 64-bit integers. The lift's integer helper accepted only the I4 and I8
categories, so a plain `ceq` on two `native int` arguments stopped at
`integer operation on non-integer value` (3,472 methods in the previous fixed scan).

## Admission

- `integer()` accepts the native category, which also admits native operands to
  `expect()` for `ceq`, `beq`, `bne` and ordered comparisons. C# spells the type
  `nint`/`nuint`, and the emitted operators keep the CLI's wrapping behavior.
- The `.un` comparison casts choose `nuint` for native operands instead of the
  narrower `uint`, so the unsigned reading keeps the operand's width.
- No conversion is inserted between native and fixed-width integers; mismatched
  categories still need an explicit `conv` in the IL, as before.

## Evidence

- Red: `FSharp.Core` `LanguagePrimitives/IntPtrIEquality.Equals` and
  `UIntPtrIEquality.Equals` (each a single `ceq`) were `Partial` on
  `integer operation on non-integer value` (recorded log); both decompile to the
  comparison after the change.
- The relation driver executes owned `nint` equality and addition cases:
  624 comparisons / 58 closed selections, all mutants rejected.
- 480 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans with the acceptance policy: `FSharp.Core` 7,416 -> 7,452
  (+36), `System.Private.CoreLib` 13,630 -> 13,699 (+69),
  `OpenRA.Mods.Common` 15,656 -> 15,681 (+25), `OpenRA.Game` 4,028 -> 4,033 (+5).
