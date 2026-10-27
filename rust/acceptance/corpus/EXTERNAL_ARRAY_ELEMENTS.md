# Referenced class elements in vector allocation

`newarr T` allocates a vector of its element type; the runtime does not care
whether `T` is a class or a value type. The refusal that remained for referenced
elements was therefore about *which* C# type to name, not about allocation
semantics: a `TypeRef` element had no local definition to classify, so the
allocation stopped at `array allocation needs exact external element kind facts`.

## Admission

- `DependencyContext::external_kind` now returns the raw
  `ExternalTypeKind { value_type }` fact; `external_type_kind` keeps its
  cast-specific refusal for value types on top of it. Both answers come from the
  same selected definition and its class-base chain (interface and class flags),
  never from the type name.
- `prime_external_array_elements` collects `newarr` operands that are plain
  `TypeRef`s, asks the dependency context once and caches the fact on the image
  next to the existing cast-target cache.
- `Image::array_allocation_type` uses that fact for a `TypeRef` element: a
  reference kind becomes `Type::Class(token)` (the exact `TypeRef` stays the
  identity) and the vector types as before. A value type keeps its own refusal
  here even though referenced value types normally take the exact value-type
  branch, and an unresolved reference keeps the original message.

No layout, size or member inference is added: the structural alias for
`RelationLibrary.Child[]` still states exact reference identity only, and the
allocation is emitted as the same `__cil_newarr_<token>(length)` operation used for
other elements, so length conversion, negative-length `OverflowException` and
zero initialization stay at the original instruction.

## Evidence

- Red: with the reference available, `RelationCases.Ops::ForeignArray` was still
  `Partial` on `array allocation needs exact external element kind facts`
  (recorded log), and the test freezes the same refusal for the no-reference case.
- Green: `ForeignArray` decompiles with `__cil_newarr_` and the
  `RelationLibrary.Child[]` signature; `ForeignValueArray` documents that a
  referenced value type already had an exact value identity and takes the
  value-type branch.
- 478 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass.
- Targeted scans after the change: `System.Private.CoreLib` unchanged at 13,630,
  `FSharp.Core` 7,350 -> 7,367 (+17), `OpenRA.Game` 4,013 -> 4,022 (+9),
  `OpenRA.Mods.Common` 15,479 -> 15,483 (+4).
- No executable behavior driver is added for this group: it admits only the
  element identity of an allocation whose length, zero-initialization and failure
  behavior are unchanged and already exercised by the general-array driver on
  owned inputs. The arrays produced here contain referenced types, whose element
  operations keep their own separate proofs. No third-party body was executed.
