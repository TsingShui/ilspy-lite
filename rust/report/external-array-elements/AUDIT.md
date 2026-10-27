# Arrays of foreign value elements

## ILSpy mechanism

- `IL/ILReader.cs` reads `newarr` with an inline type token; the element type is
  resolved through the same `ResolveType` path as any other inline type, so a
  foreign element is simply its resolved definition plus the instantiation from
  the token (TypeSpec).
- Nothing in that path needs the element's layout: the array is a vector whose
  element identity is the resolved type.

## Adopted

- `value_type_tokens` now also collects inline `Operand::Type` tokens
  (`newarr`, `castclass`, `box`, `initobj`, `constrained.`), so a foreign
  element gets its value-type fact primed from the same demand-driven lookup the
  signature vocabulary already uses. Before, only method/field signatures were
  collected and a `newarr` element could stay fact-less.
- `array_allocation_type` keeps the exact local definition path and, for a foreign
  token whose admitted value fact exists, uses `ValueType(token)` as the element.
  A recorded refusal is reported with its own reason instead of the generic one;
  primitives, TypeSpec elements and local definitions are unchanged.

No layout is inferred: the element is only named. Vector shape, zero
initialization, bounds and covariance checks remain the existing ones.

## Red then green

Owned `TokenCases` gains `Dates` (`new System.DateTime[2]`), `Days`
(`new System.DayOfWeek[2]`) and `ExternalArrays`, which calls both and reads their
lengths. With host references and the change stashed, 13 of 15 bodies recover and
both array methods report "array allocation needs exact external element kind
facts"; with the change, 15 of 15 recover.

## Regression and hermeticity

`typed-array` behavior driver passes with its mutants. While running the full
suite with `DOTNET_ROOT` exported, four CLI tests failed because they invoked
`scan`/`decompile` without `--no-auto-references` and therefore depended on the
host's framework availability for status parity. They now pass the flag, so the
parity assertions are deterministic; no assertion was relaxed.

Fixed-corpus numbers are not claimed here: this is the second group of a new scan
batch, and the next full scan covers three groups.
