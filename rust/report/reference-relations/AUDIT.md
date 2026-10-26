# Reference relationships and value-owner binding: pre-implementation audit

Baseline: committed managed-location foundation `5b777f9e4`,
`report/managed-locations/corpus-scan-validated/baseline.json`.
Fixed scope 770,748 bodies; 438,801 Decompiled / 331,947 Partial (43.06816%).
No input acquisition, reference selection, exclusions or execution scope changes.

Eleven default-output / ILSpy / original-IL triples and exact reference paths
are retained in `audit/manifest.json`. All commands returned expected statuses
(ours Partial, IL/ILSpy exit 0), but exit 0 alone does not prove source success.

| sample | actual missing relationship/operation |
|---|---|
| FSharpChoice.NewChoice1Of2 0000-06000066 | constructed nested child must retain its generic arguments while upcasting to constructed parent |
| anonymous Equals 0003-06000004 | isinst open-self structural reference is read through a declaring-class field owner; identity must work in both directions |
| Binder.Convert 0403-0600004F | newly allocated local binder derives through external base classes to returned CallSiteBinder |
| CSharpOperationFactory placeholder 0895-06000028 | derived receiver and newly allocated implementation must satisfy local/foreign base/interface identities |
| Actor.Renderables 1041-0600002C | actual IL allocates/writes an iterator object then returns its constructed interface; no yield reconstruction is required |
| ActorIndex.Actors 1042-06000084 | field's collection type must satisfy the returned constructed IReadOnlyCollection interface |
| FSharpResult.NewOk 0000-0600018F | generic value constructor, not reference-class allocation |
| SyntaxList.IndexOf 0004-06000012 | exact external constructed value receiver through the already acquired argument address |
| Player.RelationshipWith 1041-06000478 | generic value field addresses feed direct LongBitSet calls; early branches and enum result must stay intact |
| CVec.Minus 1041-06000097 | original IL boxes a full local struct; ILSpy reports unknown result due to missing references, so this sample is INCONCLUSIVE for ILSpy success, not BOTH_NO |
| BlobBuilder.GetDebuggerDisplay 0005-06000405 | local struct boxing is an initial blocker; later ldftn/delegate/array behavior is independent and not promised recovered |

The first nine and BlobBuilder are readable ILSPY_YES_OURS_NO candidates.
CVec has usable original IL but inconclusive ILSpy source; no bucket is excluded.

## Group design constraints

First deepen reference identity/assignment rather than suppressing the generic
incompatible-receiver diagnostic. Prove open-self equality, instantiated local
base/interface paths and exact dependency-backed relationships. Preserve every
generic argument and its caller scope; never erase C<string> to C<T>, collapse
different parameter positions, reinterpret a foreign token in the caller image,
or guess relationships by namespace/simple name.

A relation module should consume scoped type expressions and metadata facts,
returning proof without inserting a new runtime cast/check. Unknown, ambiguous,
malformed/cyclic or unsupported relationships stay unproved. Variant interfaces
require actual variance flags and reference-type argument proofs; managed-pointer
element identity remains invariant even where ordinary reference variance holds.
Reference facts must retain the existing exact-reference and unavailable-fact
policies. Proven argument upcasts must also retain the original overload target:
passing a Derived-typed snapshot to a source-named Base overload without a typed
operand can accidentally select a different overload. The owned DeclaredOverload
fixture distinguishes return values 11 and 29; constructor/call target identity
must not be lost through source compaction. Owned no-reference and reference-supplied cases need separate coverage.

Constructed value members/constructors and broader boxing are related downstream
foundations, not permission to loosen reference proofs or treat a value
constructor as a fresh reference allocation. Inspect their metadata/binding seams
and capture owned red cases before implementation. GVM lookup-order refusal
remains intact; ordinary constrained calls require their own audit/behavior gates.

Local red/green and boundary tests precede each capability. Consolidate full
regression, owned same-state/mutation execution, fixed-scope scan and main-session
source/IL self-review at the larger milestone. No promised gain from blocker
totals, no whole-method IL executor/proxy, no third-party execution.
