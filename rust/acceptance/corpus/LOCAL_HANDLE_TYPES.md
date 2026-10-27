# Runtime handle types defined by the image itself

`ldtoken` yields a runtime handle struct (`RuntimeTypeHandle`,
`RuntimeMethodHandle`, `RuntimeFieldHandle`). `Image::handle_type` looked for the
handle type only among the image's **TypeRefs**, so an image that *defines* the core
types (`System.Private.CoreLib`) - where the handle is a local TypeDef - refused
every `ldtoken` consumer with
`RuntimeTypeHandle needs an exact core value definition in this image` (1,325
methods in the last fixed scan; 427 in CoreLib alone).

## Admission

- `handle_type` first looks for the local TypeDef whose admitted core name matches
  the handle kind, using `local_core_type_name`; it returns the admitted value
  identity (`struct_type`) or interns the `ValueType` signature when the struct proof
  is not cached. Only then does it fall back to the existing TypeRef search.
- The identity is the same value type the consumer's parameter interns (the local
  TypeDef vs its TypeRef alias are the same runtime type), so the handle value and
  `Type.GetTypeFromHandle`/`MethodBase.GetMethodFromHandle` parameters compare
  equal. No layout, field or constructor inference is added.

## Evidence

- `System.Private.CoreLib` 16,335 -> **16,762** (+427); the sampled method
  (`0x06000089`) now decompiles with the `System.RuntimeTypeHandle` value identity.
  `FSharp.Core`, `OpenRA.Game` and `OpenRA.Mods.Common` are unchanged.
- 488 Rust tests, `cargo fmt --check` and clippy `-D warnings` pass.
