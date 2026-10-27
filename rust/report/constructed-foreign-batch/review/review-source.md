# Constructed allocation and foreign array elements: self-review

Main-session self-review, not independent review or formal verification. The
sample was selected deliberately to cover two allocations, an indirect delegate,
external value-array initialization, a loop storing external value elements, and
both OpenRA images; it is not a random or exhaustive sample. For each row,
`evidence.zip` retains unmodified production default output, ILSpy output,
original IL, stderr, and a manifest with commands and selected reference inputs.
ILSpy exited successfully without unknown-result diagnostics. No third-party
body was executed; the owned executions are recorded in the batch audit.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 06000158 | Construct the FSharp closure first; use its exact `ldftn` and delegate constructor in order. | CRUDE-BUT-EQUIVALENT |
| 0001 / 0600002B | Allocate the constructed dictionary before the base call; retain null checks, throws, generic calls, and subsequent field writes in IL order. | CRUDE-BUT-EQUIVALENT |
| 0003 / 06000060 | Construct the exact generic stack before the base call and preserve each null check and field write. | CRUDE-BUT-EQUIVALENT |
| 0003 / 0600134D | Allocate the foreign enum vector, retain the exact field token and `InitializeArray` call, and assign the resulting reference once. | CRUDE-BUT-EQUIVALENT |
| 0004 / 06000061 | Allocate the exact `TypedConstant[]`; preserve indexed value stores, loop and conditional order, ref writes, and final value copies. | CRUDE-BUT-EQUIVALENT |
| 1041 / 0600004B | Preserve closure allocation and captured fields before the condition; construct the delegate only on the null branch. | CRUDE-BUT-EQUIVALENT |
| 1042 / 06000085 | Allocate the HashSet, call the base constructor, then form and subscribe both instance delegates before `UnionWith`. | CRUDE-BUT-EQUIVALENT |
| 1042 / 060015CB | Preserve int and foreign `WDist` vector allocations, each value-producing call and indexed store, and the later base constructor call. | CRUDE-BUT-EQUIVALENT |

The verdicts concern these emitted bodies and the exact-operation contract of the
leaves, not original-runtime compatibility of selected substitute references.
The selected dependencies and substitutions are captured by the production output
and the final scan's per-image reports. Full original IL remains authoritative.
The sample found no changed-semantic suspect; the earlier general-catch review's
invalid C# conclusion is separately withdrawn in its corrected review.
