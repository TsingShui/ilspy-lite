# Byref-like (ref struct) values

The CLI allows copying a byref-like value; what it forbids is boxing it or storing
it where a byref cannot live. `value_definition_checks` treated the
`IsByRefLikeAttribute` mark as a copy blocker, so `Span<T>` parameters and locals
stopped at `byref-like struct copies need location/lifetime semantics`.

## Admission

- Encountering the attribute now records the definition's identity in the image
  instead of refusing the value. Copies stay ordinary value copies (`Ty::Value`),
  which is exactly what the IL performs; no address escape, pinning or field
  storage is inferred.
- `boxed_value_type` keeps boxing refused for a recorded byref-like identity
  (including its generic instantiations): `box`/`unbox.any` of such a value is not
  a legal operation, so the wrapper refuses with
  `box/unbox of a byref-like value needs IL fallback` rather than emitting a box.
- Storing a byref-like value in a field or array is not newly admitted: those
  operations reach their existing storage checks, and verifiable IL does not
  produce them.

## Evidence

- Red: `RelationCases.Ops::ReadRefBox` (a method call on a referenced `ref struct`)
  was `Partial` on the byref-like copy reason (recorded log); it now decompiles and
  keeps the receiver as the value/address the IL names.
- Promotions are explicit: the fixture contract reported
  `StructCopyCases.UnsupportedOps.UnsupportedRefLike: promoted` and the
  `external_values` contract lost `UnsupportedRefStruct`; both fixtures were
  renamed (`RecoveredRefLike`, `RecoveredRefStruct`) and both tests assert the
  promoted method decompiles. No negative was deleted without a replacement
  assertion.
- `CopyRefBox` (a field read on a referenced `ref struct` copy) remains `Partial`
  on the separate referenced-value-type *field* gap
  (`external member reference needs dependency facts: owner is a value type`),
  showing this group changed only the copy admission.
- No executable driver is added: reflection cannot construct or pass a `ref struct`
  candidate, so the copy form is covered by the owned contract statically, not by
  execution. No third-party body was executed.
- 486 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans: `System.Private.CoreLib` 14,327 -> 14,830 (+503),
  `OpenRA.Mods.Common` 15,951 -> 15,974 (+23), `OpenRA.Game` 4,085 -> 4,092 (+7),
  `FSharp.Core` unchanged.
