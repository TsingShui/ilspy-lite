# Dispatch through a constructed interface

A `callvirt` whose parent is an instantiated generic interface
(`ITransform<string>::Apply`) names an interface method, not a class member. The
local-constructed-member path required a class definition, so these calls stopped
at `constructed member parent must be a class; interface dispatch needs IL
fallback` (1,904 methods in the previous fixed scan).

## Admission

- `reference_calls::constructed` already handled referenced constructed members
  with exact-token dispatch. It now also keeps a **local generic interface**
  instantiation on that path: only a local *class* instantiation is deferred to the
  definition-based member path. The test is the TypeDef's interface flag, not a
  name or an `implements_interface` guess.
- The call remains a `callvirt` on the exact MemberRef/LV token with the interface
  instantiation as the static receiver, so the null check and interface dispatch
  are the ones the IL performs. No direct call, no devirtualization and no receiver
  substitution is introduced.
- Constructors, fields and non-interface local class instantiations keep their
  existing paths and refusals.

## Evidence

- Red: owned `ApplyLocalInterface(ITransform<string>, string)` was `Partial` on
  `constructed member parent must be a class` (recorded log); it now decompiles to
  `__cil_callvirt_<token>(transform, value)`.
- Promotions are explicit: the fixture contract reported
  `AbstractMemberCases.Ops.Interface: promoted; drop its expected_partial entry`
  and `InterfaceReferenceCases.Ops.UnsupportedGeneric: promoted`; both tests were
  updated to positive assertions that the body still contains `__cil_callvirt_`,
  and the negative was renamed (`RecoveredGeneric`). No assertion was dropped.
- The relation driver executes the owned interface call with string and null
  arguments and a populated implementation: 662 comparisons / 64 closed
  selections, all mutants rejected.
- 484 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans with the acceptance policy: `System.Private.CoreLib`
  14,122 -> 14,316 (+194), `OpenRA.Game` 4,033 -> 4,077 (+44), `FSharp.Core`
  7,455 -> 7,464 (+9), `OpenRA.Mods.Common` unchanged.
