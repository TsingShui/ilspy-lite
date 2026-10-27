# Constructed foreign allocation batch: correction audit

## ILSpy source

- `TypeSystem/MetadataModule.cs::ResolveMethodReference` resolves a constructed
  owner to its definition and substitutes the reference's type arguments. The
  existing exact allocation leaf retains the original constructor token.
- `IL/ILReader.cs::LdToken` resolves an entity before distinguishing type and
  member; `IL/Instructions.cs::LdMemberToken` distinguishes field from method.
  A MemberRef table alone does not say which handle kind it represents.
- `CSharp/StatementBuilder.cs` emits a bare catch for an unused `System.Object`
  catch variable. An object-typed exception variable is not valid C# catch syntax.

## Findings and corrections

The first batch scan against `report/catch-general/corpus-scan/baseline.json`
reported 13,982 gains and 34 losses. A refusal cached merely because a type
appeared in an inline operand could poison unrelated uses of that token. Positive
operand facts still populate the cache, but an operand-only refusal no longer
changes other uses. The original scan and its losses remain under `corpus-scan/`
and `delta.json`; the complete `corpus-scan-fix/` and `delta-fix.json` reported
13,983 gains and zero losses before the subsequent correctness fixes.

A cyclic owned TypeSpec caused a CLI stack overflow while canonicalizing its
constructed owner. `tests/generic_binding.rs` reproduces the crash in a child
process and now requires a Partial result. The red and green logs are
`cyclic-owner-first.log` and `cyclic-owner-fixed.log`.

A field MemberRef was classified as a method handle by `ldtoken`. The owned
static-field fixture produced Method instead of Field; the red/green logs are
`memberref-field-red.log` and `memberref-field-green.log`. The signature now
selects Field or Method and the `ldftn` path refuses field MemberRefs.

The earlier general-catch review accepted `catch (object ...)`; the actual owned
roundtrip failed to compile with CS0155 (`exception-behavior-first.log`). Its
previous equivalence claim is withdrawn. Bare `catch { }` is emitted only when
the handler first discards the object; using the object remains Partial. The
owned red/green checks are `catch-unused-red.log` and `catch-unused-green.log`.

## Owned execution

- The token adapter runs 20 comparisons per release/debug configuration, and
  rejects `GenericScope`, `CachedIdentity` and `ArrayLength` mutants. It compiles
  selected recovered bodies and adapts each token leaf to a Reflection.Emit method
  containing only the corresponding `ldtoken`, `ldftn` or `newobj` operation.
  It does not proxy an original method body. The harness source and manifest are
  in `owned/`.
- The existing exception driver runs 51 comparisons / 10 methods per
  configuration and rejects six semantic mutants, including `GeneralCatch` and
  `OutOfMemory`. The latter allocates and throws an owned exception; its exact
  constructor shell is checked against the original IL before being inlined.
- Existing typed-array, external-enum, scoped-reference and static-constructor
  drivers pass. No selected third-party method was executed.

The final fixed-corpus scan must include all corrections and count any bodies
that now fall back when their general catch uses the raw object.
