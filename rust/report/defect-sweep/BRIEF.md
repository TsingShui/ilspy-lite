# Brief: false-refusal defect sweep (sealed owner, box stack category, enum unbox)

Owner: worker. Dispatch immediately after the static-constructor slice returns.

Three independent defects share one theme: we refuse bodies we can admit, or emit text
that does not type-check. Each has a written diagnosis; fix all three in one dispatch and
measure them separately.

## 1. Sealed allocation owner (largest)

`report/allocation-owner/DEFECT-sealed-mask.md`: the owner check rejects
`0x20 | 0x80 | 0x140`; `0x140` contains `TypeAttributes.Sealed (0x100)` plus an
undefined `0x40`. A sealed class is instantiable, so **28,685 bodies** are refused
because their owner is sealed. Keep only `interface | abstract` (a static class is
`abstract | sealed`, so it stays refused), and split the catch-all string into precise
reasons for the shapes that fail later in the chain (generic arity, generic declaring
context, ancestry, constructor signature, parameter types).

## 2. Scalar box/unbox stack category

`report/boxing/DEFECT-box-scalar-stack-type.md`: `box`/`unbox.any` compare exact `Ty`,
but `bool/char/i1/u1/i2/u2` reach the writer as normalized stack values. 896 bodies are
first-blocked by `box operand type differs from its declared type`. Compare the stack
category (as the enum path already does) and keep the declared type for the cast; a
genuine category mismatch (`int64` value at a `box int32` site) must still be refused.

## 3. Enum unbox normalization and cast

`report/boxing/DEFECT-enum-unbox-cast.md` plus its addendum: unbox pushes the storage
type instead of the normalized stack type, so six of eight storage widths fail with
`incompatible value/receiver types`; the `int`-storage case emits `int v = (S32)o;`,
which does not type-check. Push the normalized integer category (`sbyte/byte/short/ushort`
-> `I4`, `long` -> `I8`) and render the conversion to the storage type explicitly.

## Gates

- Extend the frozen tables: a `char` and a `bool` box/unbox pair, one sealed-owner
  fixture (a sealed class with a field initializer and a static factory), and an
  `sbyte`-storage enum unbox that must now decompile.
- Roundtrip drivers: feed a non-null boxed value for every unbox case, and add a mutation
  that would catch a wrong storage width (`sbyte` vs `int`) or a dropped construction.
- Re-run the affected drivers in release and debug with `--mutation-control`.
- Full nextest, clippy `-D warnings`, fmt. No corpus scan (verification owns it).
- Report the three deltas separately in the output file so the next scan can attribute
  them: expected ~28,685 eligible owner rows, 896 stack-category rows, and the six
  enum-width unbox rows plus their corpus peers.

## Discipline

`CARGO_TARGET_DIR=rust/target-worker`; only `rust/src`, `rust/tests`,
`rust/acceptance/corpus` and the report file; no commits.
