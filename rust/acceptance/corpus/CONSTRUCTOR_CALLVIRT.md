# `callvirt` on a reference constructor

Some compilers (F# among them) emit `callvirt` on a base or core-object constructor
(`callvirt System.Object::.ctor`, `callvirt System.Attribute::.ctor`). Constructors
are never virtual, so the runtime performs the same non-virtual receiver call; the
previous blanket refusal (`constructor/intrinsic callvirt needs IL fallback`) blocked
329 methods in `FSharp.Core` alone.

## Admission

- A `callvirt` whose target is a constructor is handled by the constructor path when
  its receiver is a **reference** type (or the core `Object` constructor, whose
  receiver is the original `this`): the existing constructor checks (original `this`,
  same owner or immediate base) still run, the target stays
  `__cil_ctor_call_<token>`, and no `__cil_callvirt_` dispatch or extra null check is
  introduced.
- A **value-type** constructor stays refused: its receiver is a managed cell, and
  `callvirt` would treat that cell as an object reference. A static intrinsic
  (`get_TickCount`) also stays refused.
- A null or otherwise non-`this` receiver keeps its existing refusal
  (`constructor call needs same owning constructor and original this` or the core
  constructor message).

## Evidence

- Owned mutation tests: turning a base-constructor `call` into `callvirt` decompiles
  with no `__cil_callvirt_`; using `ldnull` as the receiver stays `Partial` with the
  original-this reason; the value-type constructor case
  (`value_calls::constructor_calls...`) still refuses.
- Red: the two owned negative mutations were `Partial` on
  `constructor/intrinsic callvirt needs IL fallback` before the change.
- Targeted scans: `FSharp.Core` 9,037 -> **9,366** (+329); `System.Private.CoreLib`,
  `OpenRA.Game`, `OpenRA.Mods.Common` unchanged. 490 Rust tests, fmt and clippy pass.
