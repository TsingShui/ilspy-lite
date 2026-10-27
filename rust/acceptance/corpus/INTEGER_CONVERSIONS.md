# Integer conversions and stack-category assignment

`conv.u4`, `conv.u` and `conv.i` are the remaining integer conversions the lift
refused. They are now lowered from the width of the source value:

- `conv.u4` keeps the low 32 bits and zero-extends them in the 32-bit category
  (`unchecked((uint)(x))`), `conv.u` zero-extends to the runtime's native unsigned
  width (`nuint`), and `conv.i` sign-extends or truncates to the native signed
  width (`nint`). Non-integer sources keep their refusals.
- `assignable` now recognizes that one CLI integer stack category carries both
  signs and every width up to its size: a value in that category needs no
  conversion, so a `uint`-typed result can be returned as `int` storage and vice
  versa. A Boolean destination stays excluded because its store normalizes the
  byte, and 32-bit, 64-bit and native categories never mix.

Owned red/green: `ToUInt32` (`conv.u4` over `long`) was refused with
`incompatible value/receiver types` and `ToNativeUnsigned` (native signed
conversion) with `unsupported opcode conv.i`; both now decompile, and the relation
driver executes them (538 comparisons / 54 closed selections, ten mutants) with
`long.MinValue`/`MaxValue` inputs. 475 Rust tests, fmt, clippy and the release
build pass. The group's first-blocker counts were 940 (`conv.u4`) and 518
(`conv.u`) in the last full scan; its corpus effect is measured with the next
batch scan.
