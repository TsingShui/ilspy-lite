# Core scalars named by their local definition

An image that *defines* the core types (`System.Private.CoreLib`, mscorlib and the
other core assemblies in the corpus) names its own scalars through a **local
TypeDef** (`valuetype System.IntPtr` as a MethodDef/TypeDef token), while consuming
images name them through a TypeRef. `Ty::from_sig_scoped` mapped only the TypeRef
spelling, so every local-`TypeDef` scalar fell into the struct path and was refused
by `value_definition_checks` as a "core scalar/special value signature". That also
made the receiver of a scalar instance method, a local scalar constructor and
`box`/`unbox.any` of a local scalar fail.

## Admission

- `ir::core_scalar(image, token)` resolves an admitted core scalar from either table:
  `Image::core_type_name` for a TypeRef and `Image::local_core_type_name` for a
  TypeDef. It returns the scalar's stack model (`Int32`..`Double`, `IntPtr`/
  `UIntPtr` as the native categories) and `None` for every other name, so
  `System.Decimal` and `System.Type` keep their existing value/reference paths and
  `TypedReference`/`Void` keep their refusals.
- `Ty::from_sig_scoped` uses it for both spellings, so signatures, locals and
  parameters of a locally defined scalar keep the scalar stack category.
- `Image::receiver` returns the managed address of that scalar for a scalar's
  instance method (`this` of `IntPtr::ToInt64`), matching the byref the IL passes.
- `value_calls::constructor` returns the scalar type for a local scalar's
  `newobj` (`new IntPtr(5)`), with the constructor's exact parameters.
- `boxed_value_type` accepts the scalar identity for either spelling, so
  `box`/`unbox.any` of a local scalar is the ordinary scalar box.

Field access to a scalar's private storage field (`IntPtr.m_value`) keeps its
refusal: C# cannot name those fields, and reinterpreting the address would be layout
knowledge this repository does not assume.

## Evidence

- Red/green on the fixed corpus (targeted): `System.Private.CoreLib` 15,132 ->
  15,468 with the signature/receiver/constructor mapping, then 15,739 with the box
  mapping. F# 8,758 -> 8,764, OpenRA images unchanged.
- The fixed-corpus batch that also contains the exception-region analysis (next
  entry) measures the combined effect: 720,202 / 50,546, +2,437 in 29 images, zero
  regressions.
- 488 Rust tests, `cargo fmt --check` and clippy `-D warnings` pass. Behavior is
  covered by the owned scalar/box drivers; no third-party body was executed.
