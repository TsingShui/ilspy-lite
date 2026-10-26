# Implementation boundary for this group

Keep the existing scoped interner and exact runtime operations. Repair caller
context loss and positional open-self misclassification without introducing a
new AST or a generic interpreter.

An identity argument sequence alone is insufficient to collapse C<!0..> to a
named receiver. It must name the caller's declaring definition or an exact
identity-mapped local base in that caller's proven inheritance chain. Check
arity, definition identity and each inherited argument sequence. Other owners
remain structural, with normal caller-scoped substitution. The signature
interner's open-self registration is restricted to the caller's own definition;
a base member lookup may retain the established named-base representation only
when the identity-mapped path is proved. Nonidentity/unknown inheritance is not
silently erased.

The first group scan exposed 38 constructor regressions: valid local bases
retain an identity prefix of a larger caller scope (for example D<T,U> : B<T>),
not necessarily the same arity. Three regression triples were audited against
ILSpy/IL under `regressions/`. Prove each base's declared arity and identity
prefix; permit shrinking scope along that exact base path without claiming
that an unrelated B<T> is D<T,U>'s own receiver. Capture this shape in an owned
red test before adjusting the proof. Do not restore positional-only collapse.

The original field access renderer still gates generic base-owner spelling;
this group does not invent an open base cast or relax that gate.

Constructed static member calls should use the same owner argument substitution
as instance calls, but no receiver. Render an exact original-token static-call
leaf with constructed owner evidence, rather than spelling an open generic type
or invoking an inferred method. Preserve runtime generic type initialization and
all original call side effects. External constructed owner support remains a
separate audited capability, not a side effect of restoring caller context.
