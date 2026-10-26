# Defect: enum `unbox.any` renders an implicit enum-to-integer conversion

Status: open, assigned to the next worker dispatch (fix before the boxing slice counts).

## Evidence

`rust/tests/fixtures/enum-release.bin`, `EnumCases.S32Ops.UnsupportedUnbox` (0x06000052),
default readable view:

```
public static EnumCases.S32 UnsupportedUnbox(object value)
{
    int v1 = ((EnumCases.S32)(value));
    return unchecked((__cil_enum_0200000A)(v1));
}
```

IL:

```
IL_0000: ldarg.0 0
IL_0001: unbox.any 0x0200000A EnumCases.S32
IL_0006: ret
```

The cast `(EnumCases.S32)value` has type `S32`, which does not implicitly convert to the
`int` local the writer chose, so the emitted statement is not well typed. The stack
model is right - the CLI carries the enum as its storage integer - but the expression
must make that conversion explicit.

## Same site, positive case

`GenericDeclaringCases.Ops.BoxEnum` is well typed because the box path writes
`((object)((GenericDeclaringCases.Kind)(v0)))`, where `v0` is already an `int` produced
by an explicit conversion.

## Fix

When an enum `unbox.any` pushes the storage type, wrap the cast to the enum in an
explicit cast to the storage C# type, e.g. `((int)((EnumCases.S32)(value)))`, and use
the storage type's own name (`byte`, `sbyte`, `short`, ... ) rather than a fixed `int`.
Push the storage type exactly as today; only the rendered expression changes.

## Coverage

The defect affects every enum unbox whose result is stored, passed or returned as the
storage integer, i.e. the `Unbox*` methods in `EnumCases.S8Ops`/`U8Ops`/`S16Ops`/
`U16Ops`/`S32Ops`/`U32Ops`/`S64Ops`/`U64Ops`. Statuses do not change, so the corpus
counts stay valid, but the text gate (`*_roundtrip --mutation-control` and the frozen
contract) must be re-run after the fix.

## Why it escaped

The fixture contract only asserted the status and the reason for the blocked rows; once
the row was promoted to `Decompiled`, nothing compared the emitted text. The roundtrip
driver now selects `UnboxInt` but its `object` parameter is only exercised with `null`,
so the recovered body never ran with a real boxed value. Both gaps are fixed by the
worker: add a non-null case to the driver and assert the exact conversion in the
fixture contract.

## Addendum: the pushed type must be the *normalized* stack type

`EnumCases.S8Ops.UnsupportedUnbox` now fails with `incompatible value/receiver types`
instead of rendering, because the unbox arm pushes `t.storage.ty()` (`I1` for an `sbyte`
enum) while the CLI stack holds an `int32` after `unbox.any S8` and the consumer expects
that normalized value. `S32Ops` happens to work because its storage is already `I4`,
which is why the defect first showed up as a text problem rather than a status one.

Correct rule, matching the CLI: the expression is a cast to the enum name wrapped in a
conversion to the **storage** type for readability, and the *stack* type pushed is the
normalized integer category, e.g. for an `sbyte` enum
`((int)((S8)(boxed)))` with stack type `I4`; for a `long` enum, `I8`; for `byte`/`ushort`,
`I4` as well. Never push `I1/U1/I2/U2` from an unbox.
