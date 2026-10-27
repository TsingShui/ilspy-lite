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

## Constructors called on existing value storage

`call <constructed value>::.ctor` (the address form, without `newobj`) was routed
through the class-only constructed-member path and refused with
`constructed member parent is not a class instantiation`. The `.ctor` branch now
asks `value_calls::constructor` first: a constructed value type yields the value
identity and the substituted parameters, and the call keeps the exact MemberRef
token as `__cil_ctor_call_<token>`, run on the managed location by the lift's
existing value-constructor rule. `newobj` keeps its own allocation path.
Targeted: the decompiler test image 1,175 -> 1,176 decompiled,
`System.Private.CoreLib` +4, `OpenRA.Mods.Common` +13, `OpenRA.Game` +3; 488 tests.

## Referenced constructed value owners

When the constructed value type is defined in another assembly, its arity fact is
now primed from the call operand (`value_type_tokens` records the `GenericInst`
definition and argument count for a member owner), and the instantiation resolver
falls back to `external_method_instantiation` for the declaration and virtuality
instead of requiring a local definition. The receiver is the instantiated value
identity. The fixed-corpus scan moved by four bodies (zero regressions), so this is
a narrow completeness fix; the remaining cases still lack the dependency facts for
the value definition.
