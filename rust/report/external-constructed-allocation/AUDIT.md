# Allocation of a constructed foreign owner

## ILSpy mechanism

- `TypeSystem/MetadataModule.cs ResolveMethodReference`: the member reference's
  parent is resolved with `ResolveDeclaringType`, which returns the
  *parameterized* type (`TypeSpec`); the member is then found on
  `declaringType.GetDefinition()`, and only a missing definition falls back to
  `CreateFakeMethod(declaringType, name, signature)`.
- So the definition (owner flags, arity) and the reference (caller-scoped
  signature) are two separate inputs; the instantiation comes from the parent
  TypeSpec, never from the caller's signature.

## Adopted

`allocation_constructor` no longer routes a TypeSpec parent into the local
member lookup when that parent's definition is not owned by this image. Such an
owner is admitted only through its allocation fact, exactly like a foreign
non-constructed class: the fact supplies the owner kind/sealed/arity and
parameter count, the reference signature supplies the caller-scoped argument
types, and the allocated value keeps the structural signature identity of the
TypeSpec so it stays assignable to the declared type.

`local_type_definition` also canonicalizes a TypeSpec through its definition
instead of rejecting it, which is what the surrounding lookups already do for a
TypeRef with a local definition.

## Boundaries kept

- The fact is required: without dependency facts the body still reports
  `external allocation needs dependency facts`, and the frozen owned gates
  (`StaticConstructorCases.Caches`, `.ExternalCache`, scoped
  `UnsupportedConstruct`) keep that reason.
- Constructed value types, interfaces, abstract/static owners, arity mismatches,
  malformed constructor signatures and non-`.ctor` references are refused by the
  fact as before.
- Constructor *calls* on a foreign constructed base (not allocations) still take
  their existing gated path.

## Evidence

- Owned `TokenCases` now recovers all 12 bodies with host references, including
  `StaticDelegate`/`InstanceDelegate` (`ldftn` plus a constructed `Func<,>`
  constructor); without references they stop at the allocation-fact gate.
- `StaticConstructorCases` keeps both gated types and executes 13 state
  comparisons over 9 recovered types with four mutants rejected.
- Scoped-reference driver: 36 closures and four mutants, unchanged.
- **460 Rust tests**, fmt/clippy green.
- Fixed-corpus numbers are deliberately not claimed for this group: it is the
  first of a new scan batch, so the next full scan covers three groups.
