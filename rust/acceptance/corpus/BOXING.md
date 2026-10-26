# Boxing core scalars

`box` and `unbox.any` for core scalars (`bool`, `char`, the integer family) are
admitted. Everything else stays gated: a boxed enum, struct, constructed value or
generic parameter must keep its own value identity, which needs an admission this
slice does not have.

## Semantics written down

- `box`: the value is copied into **one fresh boxed instance at that point**; later
  changes to the original value are not visible through the box. The output says so
  on its own evidence line and renders `((object)(value))`.
- `unbox.any`: null throws `NullReferenceException`, a different runtime type throws
  `InvalidCastException`, and the value is copied out of the box. Rendered as a cast
  to the declared scalar type.

## Admission

The operand type must resolve to a core scalar through the core reference identity;
anything else is refused with `box/unbox needs an admitted core scalar`. The operand
value must already have the declared stack type, so a box can never relabel a value.

## Fixture and contract

`rust/tests/dotnet/GenericDeclaringCases.cs` gained `BoxInt`, `BoxEnum`, `BoxStruct`,
`UnboxInt` and `UnsupportedBoxGeneric`. The frozen table in
`rust/tests/generic_declaring.rs` records `BoxInt` and `UnboxInt` as recovered and the
other three as falling back with the stated reason. The older fixtures whose box rows
were `unsupported opcode box` (`objects`, `enums`, `scoped_references`,
`struct_copies`, `struct_receivers`) were updated to the new stable reason, and the
object fixture's magic `unsupported.len() == 5` count assertion was replaced by its
own frozen table, so a promotion is an explicit row removal.

## Behaviour gate

`rust/tests/generic-declaring-roundtrip.py --mutation-control`: 17 comparisons over 10
recovered methods in release and debug, now including the boxed scalar return and the
unbox round trip; the wrong-field mutant is still rejected. The gate compares returned
values; box *identity* is not compared, because two correct runs allocate different
instances (the same rule as `newobj`).

## Corpus result

| quantity | value |
|---|---:|
| Decompiled | 297,132 |
| Partial | 473,616 |
| unrecovered rate | **61.44888%** |
| newly recovered | **987** (833 core scalars + 154 local enums) |
| images that gained | 95 |
| images that lost a method | **0** |
| body images within 1% | 33/319 |
| incomplete scans | 0 |

Largest gains: `System.Linq.Expressions` +121, `System` +64/+48, `mcs` +33,
`ICSharpCode.Decompiler.Tests` +26. Every gained method previously carried exactly one of
`unsupported opcode box` (785) or `unsupported opcode unbox.any` (202), so the delta is
the two new opcodes and nothing else. Source: `report/boxing/VERIFICATION.md`.

Two defects were found after the scan and are queued for the next worker dispatch:
`DEFECT-enum-unbox-cast.md` (enum `unbox.any` emits an implicit enum-to-integer
conversion) and `DEFECT-box-scalar-stack-type.md` (896 bodies still first-blocked by
`box operand type differs from its declared type`, because scalars compare exact `Ty`
instead of stack category). Both are text-correctness fixes; statuses do not change.

## Independent review

Review verdict (`report/boxing/review/review-source.md`): **13 pairs - 10 EQUIVALENT,
1 CRUDE-BUT-EQUIVALENT, 2 SUSPECT, 0 INSUFFICIENT-EVIDENCE.** Both SUSPECT rows are the
two enum-unbox methods, i.e. exactly the defects recorded in
`DEFECT-enum-unbox-cast.md` (ill-typed statement for `int` storage, wrong gate reason for
the other six widths); the core-scalar box/unbox pairs and all six corpus samples are
clean. The milestone therefore counts only after the defect sweep lands and the fixture
contract re-baselines those six rows - the review explicitly warns that freezing them as
expected-`Partial` would keep hiding the bug.

## Limits

- Boxed enum/struct/constructed values and generic parameters are gated; that needs
  value identity plus constructed-value admission at the box site.
- Box identity is not compared by the behaviour gate.
