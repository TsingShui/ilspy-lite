# Foreign generic argument identities: pre-implementation audit

Baseline: `596e59416`; the last complete corpus scan is
`report/modopt/corpus-scan/baseline.json`, while the two following groups have
only targeted scans. Inputs, reference policies and denominator are unchanged.
The 12,370 incompatible-type first blockers are not all foreign-identity cases.
A temporary diagnostic over twelve Microsoft.CodeAnalysis.CSharp candidates did
not observe this particular missing-argument projection; the instrumentation was
removed and no gain is predicted from those samples.

## ILSpy call chain and retained triples

`MetadataModule.ResolveType` resolves the selected defining module and types;
`TypeSystem/ParameterizedType.cs::DirectBaseTypes` substitutes the full argument
list into each base template. `TypeSystemExtensions.GetAllBaseTypes` traverses
that graph. `CSharp/Resolver/CSharpConversions.cs::IsSubtypeOf` compares the
resolved definition and full instantiation before applying actual interface
variance flags and directional reference conversions. These operations do not
require an intermediate type to have a TypeRef in the original caller image.
ILSpy's resolver may select substituted reference versions; that is metadata
selection, not evidence of target-runtime compatibility.

The `audit/` directory retains production default/ILSpy/original-IL triples.
Owned `ForeignHiddenToBase` and `ForeignHiddenOpen<T>` are each ILSPY_YES /
OURS_NO: their original IL simply returns the original input, but the library's
actual covariant interface argument is a hidden class or a constructed hidden
class. The closed case derives from `VisibleBase`; the strengthened open case
in `owned-open-scoped.*` derives from `VisibleGeneric<A>` and keeps `A` rather
than the distinct method position `B`. The caller does not use either hidden
intermediate type directly.
The captured compiler-produced reference library is supplied explicitly to both
analyzers. The real Roslyn `RecordDefinition<T>` triple is a broader incompatible
type case; the twelve-sample diagnostic did not establish that it exercises this
projection gap, so it is context only and is not attributed to this group.
No explicit unknown-result diagnostic appeared in the retained ILSpy outputs.

## Rust boundary

`DependencyContext::build_reference_registry` maps selected `(module, TypeDef)`
definitions to caller tokens. `reference_definition` walks foreign ancestors and
substitutes their positions, but `reference_argument` discards an edge when a
named generic argument lacks a caller TypeRef. `Proof::same_or_variant` then
never sees the relevant argument, and only knows variance facts keyed by caller
tokens. Keeping an exact foreign definition identity inside the metadata proof
is different from manufacturing a caller token or treating an assembly name as a
runtime-compatibility proof. Generic arguments keep their full caller scope.

The intended owned tests verify absence of caller TypeRefs for hidden arguments,
red/green for closed and open cases, missing-library refusal, a changed
covariance flag, and a changed hidden class base. Behavior must compare original
and recovered returned identities, including null, in Debug and Release and
reject mutants. Unknown, ambiguous, unsupported or cyclic definitions remain
unproved; no arbitrary type, variance, box or runtime cast is introduced.

## Implementation and validation

The proof keeps a `ForeignNamed` term for a resolved definition that has no
caller TypeRef. Its key contains the selected image index and TypeDef token as
well as the normalized assembly identity, full type name and generic arity.
Image index and token prevent colliding duplicate snapshots; the existing
reference policy still decides which snapshot is selected. `reference_named`
checks the definition's arity and class/value kind before creating a foreign
term. A bounded demand queue adds selected foreign ancestor and interface
variance facts, while caller tokens remain caller-scoped. `same_or_variant`
compares canonical identities and recursive arguments; generic variables keep
their original owner and position. No foreign token is interpreted as a caller
metadata token.

The initial owned positive failed in `red.log`; the scoped open generic
pre-change production/ILSpy/IL triple is `owned-open-scoped.*`. The final owned
negative tests retain Partial for a missing library, an invariant interface, an
unrelated hidden base, a class encoded for a value definition, a changed `!!0`
to `!!1`, and a same-full-name `VisibleBase` from a different supplied assembly.
Both original relations pass, and modifying the closed hidden base does not break
the unrelated open positive. The same-name negative uses a second owned library,
not a missing-assembly shortcut.

The reference-relations driver executes 848 comparisons / 80 selections per
configuration and rejects both hidden-result mutants, alongside the existing
controls. It also passes with the explicitly version-substituted owned library;
that confirms behavior only for this owned runtime, not the selected third-party
inputs. Constructor, value-call and exception drivers pass. Fmt, clippy and 495
Rust tests pass. `review/` contains the two owned recovered default/ILSpy/IL
triples and bounded main-session review; no third-party body was executed.

Four targeted production scans, compared against `596e59416`, have zero gains
and zero recovered-status losses. An initial comparison of Roslyn against the
older `modopt` baseline appeared to gain 22 methods; an isolated build and scan
of `596e59416` showed those 22 were already recovered by the earlier groups.
That attribution is withdrawn; `targeted-final.json` records the like-for-like
comparison. The subsequent complete three-group scan and its initially failed
then repaired regression delta are in `report/foreign-hidden-batch/`.
