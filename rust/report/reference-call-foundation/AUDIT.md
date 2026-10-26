# Reference-call foundation: pre-implementation audit and design

Six successful ILSpy/IL comparisons against production default output are
recorded in `audit/manifest.json`, with exact selected references and statuses.
All six are ILSPY_YES_OURS_NO. Some have independently gated allocation, array,
boxing or delegate operations; this does not predict their final recovery.

| sample | original-IL semantic observation |
|---|---|
| Roslyn CSharpResources 06001362 | ordinary ResourceManager.GetString reference callvirt with an absent definition |
| FSharp counts getter 06000040 | Array.Clone reference callvirt followed by unbox.any of a reference-type IEnumerable<int> |
| DiffLib finally helper 0600004D | null-guarded IDisposable.Dispose interface callvirt |
| FSharpMap.Partition 06000219 | exact constructed Tuple getters, generic static call and separately gated allocations |
| DiffLib iterator GetEnumerator 06000045 | ordinary Thread getter callvirt before state/identity paths |
| Decompiler package iterator 0600218B | indexed external accessors, plus independently gated delegate/foreign-value operations |

## Foundational separation

A semantic exact-call operation is not source-name presentation or selected-body
inlining. The existing `__cil_callvirt_TOKEN` operation already denotes receiver
null checking and original target-runtime class/interface dispatch, with exact
input-scoped member identity and supplied argument snapshots. Resolving a
foreign definition is necessary to render a proven property/name access or
infer layout/ancestry/allocation shape, but not to retain an exact reference-call
opcode and its caller-declared stack signature. Missing/ambiguous references
must never be guessed or substituted to obtain source sugar.

Extend the binding module to describe a call's scoped owner/signature and exact
runtime operation independently from its optional metadata-to-source proof.
Keep all original runtime resolution/access/dispatch errors, side effects and
copy behavior as part of that leaf, just as for admitted exact static calls.
Do not inline external bodies, create empty helpers, execute original selected
methods, or proxy an entire recovered method. A call that was already in the IL
remains a call; every surrounding operation must still lower independently.

## Related group work

1. Admit ordinary opaque external reference callvirt as an exact leaf when its
   receiver/signature/arguments fit the reference stack model. Dependency facts
   may improve presentation but cannot rename an unknown target. Preserve direct
   call vs callvirt and argument snapshots. External GVM/MethodSpec dispatch
   remains gated after the previous owned failure; do not infer nonvirtual flags
   for an unknown generic method.
2. Extend the same scoped signature binding to externally constructed reference
   owners. Substitute their declaring arguments in the caller's scope, preserving
   the full instantiation and exact call token. Do not infer external value
   layouts, allocation kind, arbitrary ancestry, interface variance or byrefs.
3. Normalize proved intrinsic reference relationships (core Object, String,
   Array) without guessing arbitrary external base chains. Audit/freeze negative
   identity cases. This is type/stack reasoning, not dependency selection.
4. Reuse runtime checked-reference conversion for proved reference TypeSpec
   operands, including unbox.any of a known reference type (castclass semantics,
   not scalar unboxing). Do not treat unconstrained Var/MVar as known references
   or infer the kind of an arbitrary foreign TypeRef.

## Boolean-box presentation defect exposed by the group

Owned EqualityCases.Ops.UnsupportedExternal was audited against ILSpy and IL
in `audit/owned-bool-box.*.txt`. IL calls Object.Equals, then boxes Boolean;
our output correctly normalized the returned bool to int32 but then emitted an
illegal C# bool cast from that integer. The new equality behavior gate is red
at carrier compilation (`equality-behavior.log`), not a runtime equivalence pass.
For normalized bool stack values, render the bool storage conversion before
boxing; retain stack-category validation and bool identity. Add a focused public
API red test and rerun the executable gate. This does not admit arbitrary new
scalar box kinds or make a claim about noncanonical bool bit patterns.

## Core primitive/named reference identity

Three additional successful ILSpy/IL/default-output triples for String.ToString
in qualified implementation CoreLib/mscorlib inputs (483/620/628) show identical
`ldarg.0; ret` bodies refused because the local named String receiver and the
primitive string return use different IR spellings. These are
ILSPY_YES_OURS_NO. Normalize only qualified core Object/String/Array reference
identities across primitive, local TypeDef and external TypeRef spellings. A
vector is assignable to core Array, not identical to an arbitrary Array value;
do not allow the reverse implicit downcast. Add an owned metadata-only core
String self-return case and wrong-name/key/culture negatives before repair.

## Evidence requirements

Use public API/scan red-before-green and owned original-vs-generated behavior
across missing/present references, class/interface/indexed accessors, generic
owners, null and argument effects, hidden/overridden names, direct dispatch and
runtime failures. Execute recovered call-site bodies with exact leaf adapters;
selected support bodies must be recovered rather than proxied. GVM, value-kind,
byref/modifier, counterfeit core identity and incompatible receiver negatives
stay explicit. Existing source-property proofs still require metadata evidence;
new exact-call admission must never fabricate property syntax.

One larger group milestone will consolidate full regression, behavior/mutation,
same-fixed-scope scan and main-session source/IL self-review. No third-party
execution, identity guessing, scope or reference-selection-policy changes.
Bucket totals are not promises of a ten-percentage-point gain.
