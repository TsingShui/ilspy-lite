# Volatile fields and volatile-prefixed accesses

The CLI marks a volatile field with `modreq(System.Runtime.CompilerServices.IsVolatile)`
and prefixes each access with `volatile.`. The field decoder refused every modified
signature, so volatile fields stopped at `modified field needs IL fallback`, and the
prefix itself was an unsupported opcode.

## Admission

- `Image::volatile_only_modifiers` accepts a field signature whose only modifiers are
  required `IsVolatile` markers (from `TypeRef` or `TypeDef`); any other custom
  modifier keeps the existing modified-signature refusal. The same allowance is used
  by the local field path, the referenced-field path and the static-field path.
- `volatile.` is now a recognized prefix: `src/volatile.rs` records the offset of the
  immediately following field access and refuses indirect/block forms
  (`ldind`/`stind`/`ldobj`/`stobj`/`cpblk`/`initblk`), which need an address view this
  module does not model.
- A volatile field load becomes
  `global::System.Threading.Volatile.Read(ref <lvalue>)` and a store becomes
  `Volatile.Write(ref <lvalue>, value)`, for both instance and static fields. Those
  APIs have exactly the CLI's acquire/release semantics; a plain access would drop
  the ordering. Enum, char and struct fields have no `Volatile` overload and keep a
  stated refusal (`volatile access needs a scalar or reference field ...`).
- `readonly` static writes still go through the existing static-constructor rule
  before any volatile handling.

## Evidence

- Red: owned `ReadVolatile`, `WriteVolatile`, `VolatileReferenceNull`,
  `ReadVolatileStatic`, `WriteVolatileStatic` were `Partial` on
  `unsupported opcode volatile.` (recorded log); all five decompile to
  `System.Threading.Volatile.Read/Write` with a `ref` to the field.
- The relation driver executes the read forms against the owned fixture
  (669 comparisons / 67 closed selections). Taking a `ref` to a volatile field warns
  CS0420, which is expected here because the emitted `Volatile` calls are what
  provide the ordering; the driver compiles that carrier with `-nowarn:0420`.
  The write forms are excluded from that driver because it compares the candidate
  instance state across two calls, so the mutation itself would be the difference;
  their value form is covered by the static-field and managed-location drivers.
- 487 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans: `System.Private.CoreLib` 14,830 -> 15,067 (+237),
  `OpenRA.Game` 4,092 -> 4,129 (+37), `FSharp.Core` 7,464 -> 7,471 (+7),
  `OpenRA.Mods.Common` 15,974 -> 15,980 (+6).
