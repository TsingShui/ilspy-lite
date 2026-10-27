# ILSpy resolver alignment: main-session source/IL self-review

Bounded main-session review, not independent or formal verification.
manifest.json retains commands/statuses, target paths and actual dependency
reports for eight gains and both losses. All ILSpy commands succeed without
explicit unknown-result diagnostics. Original IL is authoritative. No input
assembly was executed.

| Method | Checked obligations | Verdict |
|---|---|---|
| 0000-06000205 FSharpMap.OnSerializing | StreamingContext remains a scoped whole-value parameter. Array/null branch precedes allocation. Tuple item calls remain direct and ordered, completed KeyValuePair construction precedes exact element store, final serializedData store follows the loop. | CRUDE-BUT-EQUIVALENT |
| 0000-060001E1 GeneratedSequenceBase.IEnumerable.GetEnumerator | Original callvirt token/receiver and returned reference remain unchanged; selected IEnumerator ancestry adds no runtime conversion. | CRUDE-BUT-EQUIVALENT |
| 0003-06000051 DecompilerSettings.OnPropertyChanged | Two separate event-field reads, null branch, fresh event-args construction and invocation remain in order; no synthetic event snapshot. | CRUDE-BUT-EQUIVALENT |
| 0003-06000058 DotNetCorePathFinder.FindDotNetExeDirectory | Platform enum width/normalization, path-separator char store, Split, per-item Path.Combine/File.Exists, actual ArgumentException catch and return/leave order match IL. | CRUDE-BUT-EQUIVALENT |
| 0003-0600005A DetectTargetFrameworkId | Null exception precedes enumerator acquisition; value Current/MoveNext use original location; blob calls and uint16 comparison preserve order; actual constrained Dispose remains in finally. | CRUDE-BUT-EQUIVALENT |
| 1041-06000047 Actor.UpdateConditionState | Add/remove and count/cache effects precede created test; notifier iteration uses acquired value enumerator; notifier args and constrained finally disposal remain ordered. | CRUDE-BUT-EQUIVALENT |
| 1042-06000084 ActorIndex.get_Actors | Returns original HashSet reference as IReadOnlyCollection via selected-definition ancestry; no copy, allocation or cast. | CRUDE-BUT-EQUIVALENT |
| 1042-06000102 FireClusterWarhead..ctor | Original-this field writes, CVec.Zero whole-value read and string.Empty read precede exact base initialization. | CRUDE-BUT-EQUIVALENT |

## Two retained coverage losses

0076-060007EA and 0084-06000800 are CompileCode in the NUnit net-2.0/net-3.5
inputs. The private provider field is [System]Microsoft.CSharp.CSharpCodeProvider.
Original IL calls CodeDomProvider.CompileAssemblyFromSource with that receiver.

The strict fixed reference selection reads the older System definition and proves
the subtype. ILSpy-style host discovery chooses the installed System facade,
which forwards CodeDom types to System.CodeDom.dll; that target is absent from
the selected host's shared framework. The new metadata binder cannot prove the
provider-to-base relationship and keeps both bodies Partial. ILSpy's own output
uses an explicit CodeDomProvider receiver cast despite the missing definition.

These are two recorded reference-environment coverage losses, not vanished
methods or successful recoveries. No guessed CodeDom ancestry or additional
runtime cast is inserted to suppress them. The exact-policy output is retained
alongside both losses and the missing-forwarder request. They remain in the
numerator and residual list.

No changed-semantic SUSPECT in the eight gained bodies. This review establishes
neither whole-corpus equivalence nor compatibility of arbitrary substituted
runtime versions.
