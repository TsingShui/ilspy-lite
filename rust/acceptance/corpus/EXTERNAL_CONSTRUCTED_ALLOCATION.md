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
