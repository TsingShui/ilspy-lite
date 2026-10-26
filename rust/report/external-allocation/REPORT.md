# External allocation: report

## Change

`newobj` on a `MemberRef` whose owner lives in a supplied reference is now admitted when
the defining image proves the owner kind/name/arity and the constructor shape
(`ExternalAllocation` facts, `metadata::external_allocation_for`). Rendering stays
`__cil_newobj_<TOKEN>(args)`; an evidence line names the owner, its kind and the parameter
count. Priming happens per body so admission does not depend on lowering order, and the
three-state rule keeps an unanswered lookup byte-identical.

See `acceptance/corpus/EXTERNAL_ALLOCATION.md` for the fact set, refusals and the two
shapes that stay gated (constructed external owner, nested external owner).

## Evidence

- `cargo nextest run`: 365 passed (new suite `external_allocations`).
- clippy `-D warnings`: 0; fmt clean.
- `tests/external-allocation-roundtrip.py --mutation-control`: 11 comparisons over 4
  recovered methods, release and debug; null-allocation mutant rejected in both.
- Full scan `report/external-allocation/corpus-scan/baseline.json`:
  **334,213 Decompiled / 436,535 Partial** of 770,748 (56.6354%), 0 incomplete,
  33/319 images within 1%.
- Delta vs `report/defect-sweep/corpus-scan`: **+1,116, 89 images gained, 0 lost**, and
  every gained method previously carried `external allocation needs dependency facts`.

## Limits

The slice proves "the exact constructor ran and produced one fresh instance with these
arguments"; it does not claim field-initialiser order, layout, or anything about the
owner's behaviour after construction. Constructed and nested external owners remain gated
with their own reasons and are the next two increments here.
