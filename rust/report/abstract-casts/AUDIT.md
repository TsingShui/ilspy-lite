# Abstract cast target audit

Four first-blocked methods from different assembly families were compared before
repair. Commands, exact references and exit codes are recorded in
`audit/manifest.json`; original IL, ILSpy and default output are retained beside it.
All ILSpy/IL commands succeeded; our default output was Partial (exit 2).

| sample | IL-authoritative operation | classification |
|---|---|---|
| FSharp.Core Joint.Equals 0600031A | isinst Joint, then concrete child tests/field comparisons | ILSPY_YES_OURS_NO |
| Decompiler TextTokenWriter.IsDefinition 06000076 | isinst EntityDeclaration before conditional Parent getter | ILSPY_YES_OURS_NO |
| CodeAnalysis.CSharp Binder.get_ContainingType 06000037 | castclass NamedTypeSymbol on the NamedType branch | ILSPY_YES_OURS_NO |
| CodeAnalysis ConstantValue.Equals 06000B8A | isinst ConstantValue before the virtual Equals call | ILSPY_YES_OURS_NO |

Abstract target classes describe existing derived instances. Neither castclass
nor isinst allocates that target; requiring a concrete target is unnecessary.
The existing local-class representation already admits these signature/receiver
types. Keep exact runtime checks, null/identity behavior and operation ordering.

## Bounded design and validation seams

Remove only the Abstract flag test in `Image::cast_class`; retain TypeDef, local,
class/noninterface, nongeneric, non-value-type and hierarchy validation. No new
IR, metadata representation, assignment or dispatch rule. Allocation keeps its
independent abstract-owner refusal.

Extend the existing public decompile/scan fixture and owned original-vs-generated
runtime gate: casts/tests of an abstract local base against null, two derived
classes and unrelated objects; exact identity, bool/int results and exception
kind; negative local interface/constructed-generic/external targets. Red before
green. Add targeted mutations converting null-yielding abstract `as` to throwing
cast and replacing its bool test with a constant. Inspect the existing result
descriptor: primitive results must compare values, not only their type names.

## Core string/object targets in the same repair group

Three additional successful ILSpy/IL pairs cover core string checks:
FSharp DerivedPatterns.StringPattern (isinst), Decompiler ILReader.DecodeLdstr
(castclass following an Operand getter), and Roslyn MetadataHeapsBuilder
GetConstantBlobIndex (isinst before a branch and stateful calls).
All are ILSPY_YES_OURS_NO; exact manifests include the additional commands.

Use the existing exact core-identity predicate for System.String/System.Object
TypeRefs, returning the already supported String/Object IR types. Do not infer
class facts for arbitrary external names or interfaces. Same-named wrong-key,
wrong-assembly and nested identities must not acquire this admission. Use the
same runtime cast/type-test renderer, without moving or removing checks.

The 8,209-method combined concrete/nongeneric first-blocker bucket is not an
expected gain count and includes independently unsupported shapes. No changes
to scope, references or denominators. Third-party samples are static only.
