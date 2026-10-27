# Generic method instantiations on constructed value types

`call !!0 LocalBox<int>::Echo<int>(...)` names a generic method whose owner is an
*instantiated value type*. The instantiation resolver required a reference-class
owner (`is_value_type: false`), so these calls stopped at
`generic method owner needs a reference-class instantiation` (2,289 methods in the
last fixed scan, dominated by F#'s `AsyncActivation<'T>::WithContinuations<...>`).

## Admission

- A `MethodSpec` whose target MemberRef has a `GenericInst { is_value_type: true }`
  parent now resolves its local declaration on the definition (the same
  `local_instance_method` identity check the class path uses), keeps the site's
  type arguments for substitution, and returns `Ty::Value` of that instantiation as
  the receiver.
- The lift's existing value-receiver rule then requires the managed location of
  that exact value type; a copied value is refused. Static generic methods on a
  value instantiation still have no receiver.
- The call keeps the exact `__cil_methodspec_call_<token>` identity and argument
  order; the generic-virtual refusal is unchanged.

## Evidence

- Red: owned `CallConstructedValueGeneric(LocalBox2<int>, int)` was `Partial` on
  `generic method owner needs a reference-class instantiation` (recorded log); it
  now decompiles to `__cil_methodspec_call_<token>(ref v0, v1)`.
- The Rust test asserts the token call and the `ref v0` receiver; the relation
  driver's leaf helpers cannot model `ref` receivers, so the case is excluded from
  that driver with the reason recorded (value-receiver semantics stay covered by
  the struct-receiver driver).
- 488 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans: `System.Private.CoreLib` 15,088 -> 15,101 (+13),
  `FSharp.Core` 8,608 -> 8,611 (+3), `OpenRA.Game` and `OpenRA.Mods.Common`
  unchanged. Most methods in the bucket stop at a later gate, so the measured gain
  is small.
