# Constructing referenced value types

`newobj` on a referenced struct has exact C# semantics: `new S(args)` zero
initializes a fresh value, runs the constructor on it, and yields the completed
value. The relocation, argument order and failure behavior (a throwing constructor
produces no value) are the same in IL, so no layout knowledge is needed; only the
struct's own value identity and the constructor's exact signature are.

Before this group, `DependencyContext::external_allocation` refused that owner with
`owner is a value type; value initialisation is not an object identity`, which was
the first blocker of 5,816 methods in the previous fixed scan.

## Admission

- `ExternalAllocation` now carries `value_type`, read from the selected
  definition's flags and its exact `System.ValueType`/`System.Enum` base, exactly
  like the class-versus-value answer used by casts. The value-type refusal is gone.
- `Image::external_allocation_for` builds the allocated type from that fact: a
  referenced struct becomes `Ty::Value` with the referenced struct's own structural
  identity (the same identity its signature uses), while classes keep
  `ExternalClass` (or the constructed structural identity). A constructed
  referenced struct still stops at the value-definition proof, which needs the
  definition's own generic fields.
- The lift keeps the single `__cil_newobj_<token>(args)` operation and states
  whether it produces an object identity or a value: for a value it is
  "zero-initialize separate value storage, run the exact constructor in original
  argument order, then copy the completed value".

Nothing is inferred about layout, field offsets or the constructor's body: the
defining image's constructor runs exactly as written, and only its signature is
read here.

## Evidence

- Red: `RelationCases.Ops::SeededValue` (owned fixture calling a referenced
  struct's constructor) was `Partial` on
  `external allocation needs dependency facts: owner is a value type; value
  initialisation is not an object identity` (recorded log).
- Green: the same method decompiles as `__cil_newobj_0A000011(seed)` with the
  referenced struct's value identity; the Rust test reproduces the acceptance CLI's
  reference setup (one explicit reference, ILSpy version policy, no automatic
  discovery).
- The frozen external-allocation negative for `UnsupportedStruct` was updated, not
  weakened: value owners are admitted now, and under exact-policy references that
  body still stops at the referenced struct's signature identity
  (`enum storage requires an exact local definition`). The expectation records the
  new first blocker instead of the removed gate.
- 479 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass.
- Fixed-corpus batch scan under the acceptance reference policy (`--references
  ilspy`): **679,395 Decompiled / 91,353 Partial of 770,748**, fallback
  **11.8521%**, **+5,417 in 120 images, zero regressions, zero incomplete**.
  OpenRA.Game +2, OpenRA.Mods.Common +146.
- No third-party body was executed. The owned fixture confirms the constructor
  call shape statically; `new S(args)` needs no additional runtime driver beyond
  the owned fixture, whose behavior is identical in both compilations by
  construction.
