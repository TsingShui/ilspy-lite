# Fields of referenced value types

Reading or writing a field of a referenced struct means accessing the managed
location of that struct value; a copied value would only mutate a temporary. The
lift already enforced exactly that rule, but the dependency fact builder refused
every value-type owner with `owner is a value type; its fields need managed-location
semantics`, so such accesses never reached the rule (1,412 methods in the previous
fixed scan).

## Admission

- `DependencyContext::external_field` no longer refuses value-type owners; its fact
  now carries `value_type`, read from the selected definition's exact
  `System.ValueType`/`System.Enum` base.
- `Image::field` builds the receiver type from that fact: a value type becomes
  `Ty::Value` with the referenced struct's own signature identity, a class stays
  `ExternalClass`. The declared field type still comes from the MemberRef/FieldDef
  signature, and the instance/static split from the field flags.
- The lift's existing checks then apply unchanged: an instance read needs the
  managed location (`ValueAddress`) of that exact value type, a write needs the
  writable location, and a copied value is refused. No layout, offset or field
  order is inferred.

## Evidence

- Red: owned `RecoveredField` (referenced `Measure.Units`) and `CopyRefBox`
  (referenced `ref struct` field) were `Partial` on the value-type field reason
  (recorded log); both now decompile to `measure.Units` / `box.Value`.
- Promotions are explicit: `ExternalValueCases.Ops.UnsupportedField` and
  `ExternalAllocationCases.Ops.UnsupportedStructField` were renamed
  (`RecoveredField`, `RecoveredStructField`), removed from their frozen
  expected-partial tables and asserted as `Decompiled`. The external-value table is
  now empty and says so.
- 487 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass.
- Targeted scans: `OpenRA.Mods.Common` 15,980 -> 16,344 (+364),
  `OpenRA.Game` 4,129 -> 4,130 (+1), `System.Private.CoreLib` and `FSharp.Core`
  unchanged.
