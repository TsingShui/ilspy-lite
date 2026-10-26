# Defect: scalar `box`/`unbox.any` compares exact `Ty` instead of stack category

Status: open, assigned to the next worker dispatch.

## Evidence

After the boxing milestone the scan still reports, as a *first* blocker:

| reason | bodies | images |
|---|---:|---:|
| `box operand type differs from its declared type` | 896 | 79 |

That reason can only come from the exact-type comparison in the `box`/`unbox.any` arms.
A value loaded from a `bool`, `char`, `i1`, `u1`, `i2` or `u2` location reaches the
writer as a normalized stack value, exactly as an enum loaded from storage does, so
`Ty::Char`/`Ty::Bool`/`Ty::U1` on one side and `Ty::I4` on the other is a *representation*
difference, not a semantic one.

The local-enum path already relaxes this (`value.ty.stack() != declared.stack()`); the
scalar path still uses `value.ty != declared`.

## Fix

Use the same stack-category comparison for every admitted scalar and keep the declared
type for the emitted cast, e.g. `((object)((char)v))` for a boxed `char`. Do not relax
the check for reference types or for a value whose category genuinely differs (an
`int64` value at a `box int32` site must still be refused).

## Coverage

896 bodies across 79 images, all currently first-blocked by this row. Re-scan after the
fix; the frozen contract gains a `char`/`bool` box-unbox pair so the rule is pinned.
