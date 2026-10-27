# Nominal exception ancestry: ILSpy-first audit

## Upstream call chain

- `IL/ILReader.cs::PrepareBranchTargetsAndStacksForExceptionHandlers` resolves
  each catch token with `MetadataModule.ResolveType`, then puts that exact type
  into the exception stack slot. `IL/BlockBuilder.cs` attaches the variable to
  the ordered handler; `CSharp/StatementBuilder.cs::VisitTryCatch` uses that
  variable type for the emitted clause. `VisitThrow` passes the original throw
  argument with `KnownTypeCode.Exception` as a translation hint.
- `TypeSystem/MetadataModule.cs::ResolveType` distinguishes definitions,
  references and type specifications and resolves them in the owning module.
  `TypeSystem/Implementation/MetadataTypeDefinition.cs::DirectBaseTypes`
  resolves the actual definition's base and interfaces with that module's
  generic context. `TypeSystem/TypeSystemExtensions.cs::GetAllBaseTypes` walks
  the reflexive, transitive closure of those selected definitions. It does not
  establish inheritance from a name suffix or insert a runtime cast.
- Rust already projects selected-definition relationships through
  `dependencies/binding/reference_relations.rs`. The candidate adaptation is to
  record whether the *class base path* reaches the selected core
  `System.Exception` definition, and reuse its existing demand-driven query and
  identity aliases. Interfaces and unresolved/incorrect definitions are not proof.
  For an image that itself defines the core library, follow its local TypeDef
  base chain to the recognized local core `System.Exception` definition.

## Fixed-corpus candidates, before implementation

`audit/manifest.json` retains production default output, ILSpy output and raw IL
for these eight fixed-scope Partial methods. ILSpy succeeds without an explicit
unknown-result diagnostic. The prior `EH catch type needs an admitted core
exception identity` text is also emitted by the throw proof, so it does not mean
all these bodies actually have exception handlers.

| Image/token | Original operation / reason for study |
|---|---|
| 0620-06000016 | `newobj` local `NotImplementedException` MethodDef, then `throw`; local core exception ancestry. |
| 0620-0600001B | Three local core exception constructors on distinct failure branches; type-token and call order remain separate obligations. |
| 0628-06000002 | Local `OutOfMemoryException` throw after native heap operations; the first blocker is not a gain forecast. |
| 0628-06000094 | Local `ArgumentNullException` and `ArgumentException` throws behind null and bounds checks. |
| 0483-060000C5 | Local core `System.IO.IOException` catch plus finally and `volatile.`; other gates may still apply. |
| 0005-0600033E | Catch and locally constructed `ResourceException` in a method with nested cleanup; other operand gates may still apply. |
| 0993-06000058 | Three ordered catches including core-selected `SecurityException`, each wrapped in `IOException`; caught identity and constructor arguments must be retained. |
| 0520-06000006 | External `PlatformNotSupportedException` constructed after an exact base constructor and resource getter. |

Do not infer gain counts from these first blockers. Unknown references, cycles,
interfaces, unrelated same-named classes, invalid runtime casts, catch ordering,
and original throw/rethrow identity remain distinct obligations. No third-party
body was executed.

## Implementation and focused validation

`Image::exception_reference` recognizes a local core `System.Exception` in the
actual TypeDef class chain. For external classes it asks the existing
`reference_assignment_facts` query to attach a class-only exception fact to the
selected definition. A trusted core exception TypeRef is the same terminal proof
already used by local exceptions. The facts add no runtime operation.

Owned tests were red before implementation for both local core and foreign
selected-definition ancestry; both are green. An untrusted core identity,
unrelated root, absent dependency, a cyclic base and a malformed interface path
stay Partial. The byte-only foreign negative stays unchanged.

The reused exception driver runs seven foreign comparisons per configuration:
derived/base catches, nonmatching/null throws, same caught object and original
thrown identity. Changing the derived catch to an unrelated exception and replacing
the thrown object are both detected. Its existing suite runs 51 comparisons
and rejects six mutants per configuration. The ordinary and selected-version
reference relation drivers each run 143 comparisons / 36 closed selections and
reject four mutants per configuration. All 466 Rust tests and fmt/clippy pass.

Six of the eight preselected third-party candidates now decompile in targeted
static runs; two remain Partial. This is not a full-corpus delta or a substitute
for the batched gained-method self-review. No target body was executed.
