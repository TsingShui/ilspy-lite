# Exception ancestry from selected definitions

Throw and catch admission now follow the actual class base path to an admitted
core exception identity. For a core library's own TypeDefs, the local class
chain may reach its qualified `System.Exception` definition. For a foreign
TypeRef, the existing demand-driven reference registry follows the selected
metadata definitions and records exception ancestry only on class-base edges.
An already-admitted core exception TypeRef remains a valid endpoint, matching
the byte-only local exception behavior. Interface edges, missing definitions,
unrelated names and cyclic hierarchies do not prove an exception relationship.

The proof supplies type information only: it adds no runtime cast, allocation,
exception wrapping or throw operation. The caught variable keeps the original
metadata token, handler order and object identity. Exact and SameMajor reference
policies retain their existing identity requirements; IlSpy selection can use
its selected substitute and reports that selection, not runtime compatibility.
General catches whose caught object is used, filters, faults and unsupported
control-flow shapes retain their separate fallbacks.

## Evidence

- `report/exception-ancestry/AUDIT.md` records the ILSpy call chain and eight
  fixed-corpus production/ILSpy/IL candidates before implementation. Six of
  those eight are now decompiled in targeted runs; the other two remain Partial
  for later blockers. A full fixed-corpus delta and gained-method self-review
  are deferred to the next three-group batch scan.
- Owned red-to-green tests cover a local core exception, foreign multi-level
  ancestry and context-sensitive selected definitions. Wrong core identity,
  unrelated base, cyclic base, forged interface path and absent reference stay
  Partial. The pre-existing byte-only foreign negative is unchanged.
- The existing exception behavior driver has a foreign-fixture mode: seven
  comparisons per configuration cover ordered derived/base catches, unmatched
  exceptions, null, caught-object identity, and throwing the same input
  object. It rejects both a catch-type mutant and a throw-identity mutant.
  The previous exception driver runs 51 comparisons and rejects six mutants per
  configuration; reference-relationship drivers run 143 comparisons across 36
  closed selections and reject four mutants per configuration.
- These are owned executions. Core-definition fixtures are metadata-only; no
  target-corpus method was executed, and selected-definition facts do not prove
  equivalence across runtimes.
