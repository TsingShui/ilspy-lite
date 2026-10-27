# Allocating an instantiated foreign class

A `newobj` whose owner is a constructed type defined in another image used to be
rejected as "constructed member needs a local definition", even though the
dependency binder already proves such an owner for non-constructed classes.

Following ILSpy's `ResolveMethodReference` split, the owner definition and the
caller-scoped reference signature are now separate inputs: the allocation fact
supplies owner kind, sealedness and arity, while the reference signature supplies
the argument types. The allocated value is the structural identity of the parent
TypeSpec, so it remains assignable to the declared type.

Nothing widens: value types, interfaces, abstract or static owners, arity
mismatches, malformed constructors and non-`.ctor` references are still refused,
and a body without dependency facts keeps the allocation-fact reason.

Evidence: `report/external-constructed-allocation/AUDIT.md`. Owned
`TokenCases` recovers 12 of 12 bodies with host references; the frozen
static-constructor and scoped-reference gates keep their reason and their
executed comparisons (13 state comparisons over 9 types, four mutants; 36
closures, four mutants). Corpus counts for this group arrive with the batched
scan that covers three groups.

## Constructor parameters substituted from the instantiation

`newobj` on a referenced constructed owner passes a `MemberRef` whose parameter
signatures name the definition's own type parameters (`Box<!0>::.ctor(!0)`).
`external_allocation_for` decoded those parameters with the *caller's* type context,
which has no scope for the definition's parameters, so the allocation stopped at
`generic variable has no owner or exceeds declared positions` even though the parent
TypeSpec supplies the arguments.

The path now substitutes the parent instantiation's arguments into each parameter
(and its byref wrapper) before decoding; a plain TypeRef owner keeps the
context-based path. Required custom modifiers stay refused, optional ones do not
change the type.

Evidence: `FSharp.Core` 8,764 -> **9,037** (+273), `OpenRA.Mods.Common` 16,916 ->
16,983 (+67), `OpenRA.Game` 4,309 -> 4,321 (+12); the owned `MakeBox` negative was
promoted to a positive assertion (allocation with `__cil_newobj_`), and 489 tests
pass.
