# External fields: report

## Change

`ldfld`/`stfld` on a `MemberRef` whose owner is a referenced class now reads the field's
identity and flags from the defining image and the field's type from the caller-scoped
signature (`ExternalField` facts, `metadata::external_field` path in `Image::field`).
Reads render `recv.Name`, writes `recv.Name = value`; the receiver is popped once, in
order, as before.

## Evidence

- 365 tests pass; clippy 0; fmt clean.
- `tests/external-allocation-roundtrip.py --mutation-control`: 21 comparisons over 8
  recovered methods, release and debug; null-allocation mutant rejected.
- Full scan: **334,899 Decompiled / 435,849 Partial** (56.5386%), 0 incomplete, 33/319.
- Delta vs `report/external-allocation/corpus-scan`: **+686, 21 images, 0 lost**, all from
  `external member reference needs dependency facts`.

## Limits

Value-type owners (including external structs and enums) and static fields stay gated;
a struct field access needs the managed-location rules of the receiver milestone, and
static fields are a separate increment. Constructed external owners remain gated as in
`EXTERNAL_ALLOCATION.md`.
