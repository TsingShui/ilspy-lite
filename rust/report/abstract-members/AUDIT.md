# Abstract constructed-owner audit

Four first-blocked corpus methods, each from a different assembly family, were
compared with ILSpy and original IL. Commands succeeded and output is retained
under `audit/`, with exact member/reference provenance in `manifest.json`.

| token / assembly | ILSpy output | IL operation blocked here | decision |
|---|---|---|---|
| 0600006F FSharp.Core | GetHashCode(comparer) | callvirt on FSharpChoice<!0,!1> | candidate: abstract declaring class does not prohibit virtual dispatch |
| 060008A8 Decompiler | state analysis calls | direct GetBlockInputState on DataFlowVisitor<!0> | candidate; constrained receiver operations remain an independent blocker |
| 060009D7 CodeAnalysis.CSharp | visitor switch and Visit* calls | callvirt on BoundTreeVisitor<!0,!1> | candidate; generic default/result handling remains independently gated |
| 060002BD CodeAnalysis | _assemblyRefIndex.Rows | callvirt on HeapOrReferenceIndexBase<AssemblyIdentity> | candidate; member signature/source spelling still requires verification |

Four ILSPY_YES_OURS_NO, zero BOTH_NO in this sample. This does not predict the
number of recoveries behind the 21,182-method first-blocker bucket.

## Bounded repair

`constructed_member_owner` is shared by ordinary member lookup, base-constructor
calls and `newobj`. Its interface/abstract flag test conflates lookup on an abstract
class with allocating an abstract object. Remove only the abstract-owner rejection
from member lookup; keep interface refusal and the independent allocation guard
(`interface | abstract`) unchanged. No new representation or pipeline is needed.

Tests first: closed abstract generic receiver virtual/nonvirtual calls, null and
override dispatch, interface refusal, and a patched `newobj` targeting an abstract
class must remain Partial. Original IL call/callvirt and owner identities remain
the semantic authority. Existing constructor/member-name gates are not relaxed.
