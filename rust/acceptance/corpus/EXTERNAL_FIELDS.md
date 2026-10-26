# External instance fields

A field read or write whose declaring type lives in a referenced image is admitted when
the defining image identifies the field and the caller's `MemberRef` signature names its
type. The access renders on the receiver (`node.Value`, `node.Value = value`), so
evaluation order and the single receiver evaluation are the ones the lift already keeps.

## Facts and rendering

- from the defining image: the field's name and flags (must be an instance field, not a
  literal, not marshalled), and the owner's kind and name (a class; a value-type owner
  stays gated because its fields need managed-location semantics);
- from the caller: the `MemberRef` field signature, so the field's `Ty` is in the caller's
  vocabulary and no foreign token enters the type;
- priming covers only operands of `ldfld`/`ldflda`/`stfld`/`ldsfld`/`ldsflda`/`stsfld`,
  so a method reference never reaches the field resolver.

Three-state rule as everywhere: `Fact` admits, `Refused(reason)` renders
`external member reference needs dependency facts: <reason>`, `Unavailable` caches nothing
and leaves the method byte-identical to a run without references.

## Refusals

Static fields (`external field needs IL fallback` at the site), literals with no storage,
marshalled layout, interfaces, value-type owners, unusable owner names.

## Fixtures and gates

`tests/dotnet/ExternalAllocationCases.cs` now also covers reads and writes:
`ReadValue(Node)`, `ReadAfterAllocation(int)`, `WriteValue(Node, int)`,
`WriteAndRead(int)` are positives; `UnsupportedStatic` and `UnsupportedStructField` are
negatives with their own reasons.

- `external_allocations` nextest suite: four object-returning allocations plus the four
  field methods decompile with one `__cil_newobj_` each and a field access; negatives keep
  their reasons; without references nothing is admitted.
- `tests/external-allocation-roundtrip.py --mutation-control`: 21 comparisons over 8
  recovered methods in release and debug (owner type, field contents, argument order,
  identity, exceptions); the null-allocation mutant is rejected.

## Corpus result

`report/external-field/corpus-scan/baseline.json`: **334,899 / 435,849** of 770,748
(56.5386%), 0 incomplete, 33 images within 1%. Versus the external-allocation baseline
(334,213): **+686 across 21 images, 0 lost**, every gained method previously carrying
`external member reference needs dependency facts`.
