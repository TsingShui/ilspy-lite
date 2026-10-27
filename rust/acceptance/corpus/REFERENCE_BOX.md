# Boxing a reference type

`box` on a reference type allocates nothing: the CLI keeps the reference and only
the static type changes. `unbox.any` on a reference type is a `castclass` check
(null passes, a wrong type throws `InvalidCastException`, identity is preserved).

Both forms were already implemented for operands whose declared type was already
admitted as a reference (`String`, `Object`, local classes, constructed types and
SZ arrays). The refusal that remained was admission: `boxed_value_type` accepted a
`TypeRef` only when it was a core scalar or an admitted value type, so
`box [mscorlib]System.Type` (F# emits this for generic arguments) stopped at
`box/unbox needs an admitted core scalar or local enum`.

## Admission

- `Image::boxed_value_type` now also asks `cast_class` for a `TypeRef` and returns
  the proved reference type; the lift's existing reference path then keeps the
  operand's reference and emits `((object)(value))`. The operand must itself be a
  reference; a value operand keeps a stated refusal instead of being boxed as a
  class.
- `Image::reference_unbox_target` accepts any `TypeRef` that `cast_class` proves,
  not just `String`/`Object`, so `unbox.any` on it stays the exact `castclass`
  check with its emitted type name.
- The cast-target priming pass now also collects `box` and `unbox.any` operands,
  because the class-versus-value answer comes from the selected definition and
  must be available to these instructions too.

No new runtime operation is introduced: the same identity cast and `castclass`
were already emitted for other admitted reference types.

## Evidence

- Red: four fixed-corpus methods (`FSharpType.IsTuple`, `FSharpType.IsModule`,
  `FSharpValue.PreComputeRecordFieldReader`, `FSharpExpr.Call`) were `Partial` on
  `box/unbox needs an admitted core scalar or local enum`; with the change they
  decompile (recorded before/after).
- 478 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass.
- Targeted scans: `FSharp.Core` 7,367 -> 7,402 (+35); `System.Private.CoreLib`,
  `OpenRA.Game` and `OpenRA.Mods.Common` unchanged. Many of the 3,400 methods that
  reported this reason stop at a later gate, so the measured gain is smaller than
  the reason count.
- No executable driver is added: reference boxing is identity and reference
  `unbox.any` is `castclass`, both already executed by the owned reference and
  equality drivers; the new admission changes which declared types reach those
  operations. No third-party body was executed.
