# Generic method instantiations on value-type receivers

`call !!0 S::M<int>(...)` names a generic instantiation whose owner is a value
type (`AsyncVoidMethodBuilder.Start<TStateMachine>(ref TStateMachine)`,
`struct S { T Echo<T>(T) }`). The external instantiation fact refused any value-type
owner with `instantiated method owner is a value type; its receiver needs
managed-location semantics`, and the local definition path required a reference
receiver; the CLI call itself is ordinary: the receiver is the managed location of
the value and the exact MethodSpec token is the call identity.

## Admission

- `DependencyContext::external_method_instantiation` no longer refuses value-type
  owners; the interface refusal and the generic-virtual lookup-order refusal are
  unchanged.
- `method_instantiations::resolve` keeps the receiver type the declaration
  actually has (`Ty::Value` for a value type). The lift's existing value-receiver
  rule then requires the popped operand to be a managed location of exactly that
  value type, so `ldarga`/`ldloca` receivers are used and a copied value is
  refused rather than silently mutated.
- The call remains `__cil_methodspec_call_<token>` with the original argument
  order; no receiver copy, boxing or devirtualization is introduced.

## Evidence

- Red: owned `CallValueGeneric` (referenced struct) and `CallLocalValueGeneric`
  (local struct) were `Partial` on the value-type receiver reason (recorded log);
  both now decompile to `__cil_methodspec_call_<token>(ref v0, v1)`.
- The Rust test checks both bodies and that the receiver is passed as `ref v0`.
- The relation driver does not execute these two bodies: its leaf helpers model
  by-value receivers, and passing `ref` would not compile. Value-receiver call
  semantics are already executed by the owned struct-receiver driver; the new
  admission only adds the MethodSpec token path. The two methods are excluded from
  that driver with this reason recorded, not silently dropped.
- 485 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans: `System.Private.CoreLib` 14,316 -> 14,327 (+11),
  `OpenRA.Game` 4,077 -> 4,085 (+8), `OpenRA.Mods.Common` 15,939 -> 15,951 (+12),
  `FSharp.Core` unchanged. Most of the 3,429 first-blocker methods stop at a later
  gate, so the measured gain is small.
