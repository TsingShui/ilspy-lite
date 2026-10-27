# Boxing core-named types that are not CLI scalars

`boxed_value_type` resolved the operand's core type name and then required that name
to be one of the CLI scalars (`Boolean`..`Double`). Any other core-named type
(`System.Type`, `System.Decimal`) fell out of the scalar match as a refusal, so a
`box`/`unbox.any` of a reference class or of a stored value type sharing a core name
stopped at `box/unbox needs an admitted core scalar or local enum` even though the
box operation is the ordinary reference-identity or value-box operation.

## Admission

- A resolvable core scalar still maps to its own stack model (including the
  TypeRef spellings of `Int32`/`Double`/`IntPtr`/...).
- Any other core-named type continues to the existing paths: a reference type goes
  through the cast proof and keeps reference identity (`box` allocates nothing), a
  value type goes through the admitted value identity and is copied into a fresh box.
  `TypedReference`, `Void` and unadmitted types keep their stated refusals.
- No new operation is introduced: the same `(object)value` / `((T)(object)value)`
  forms are emitted, now for the types the scalar match previously dropped.

## Evidence

- Red: F# `FSharpType.IsTuple` (`06000299`) was `Partial` on
  `box/unbox needs an admitted core scalar or local enum` (recorded log) although
  its body only boxes the `System.Type` reference; it now decompiles to
  `((object)(typ))`. The same change recovered 41 F# methods, all previously this
  reason.
- 488 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans: `FSharp.Core` 8,717 -> 8,758 (+41), `OpenRA.Game`
  4,280 -> 4,282 (+2), `System.Private.CoreLib` and `OpenRA.Mods.Common` unchanged.
