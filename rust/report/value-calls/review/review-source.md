# Constructed values and boxing: main-session source/IL self-review

The main session compared the production default readable output, ILSpy source
and original IL retained in `manifest.json`. This is a bounded static self-review,
not independent review, third-party execution or whole-corpus recompilation.
Original IL is the authority. All ten ILSpy invocations succeeded without an
unknown-result diagnostic in these samples. All ten are CRUDE-BUT-EQUIVALENT for
the changed semantics; no changed-semantic SUSPECT was found in this sample.

| triple | checks |
|---|---|
| 0000-0600006E FSharpChoice.GetHashCode | !0/!1 remain separate owner-scoped variables; field values are snapshotted before boxing and hash calls. Null-this branch and case casts remain ordered; both shift/add chains retain unchecked integer widths. |
| 0000-0600018F FSharpResult.NewOk | exact value constructor 0A0000D4 follows T value/zero tag/zero Boolean arguments. Completed value is returned, not a reference-class allocation or generic-argument-erased object. |
| 0000-06000190 FSharpResult.get_IsOk | original-this managed location feeds exact get_Tag; no receiver copy, reference null check or virtual dispatch is introduced. Comparison and Boolean storage remain explicit. |
| 0004-06000012 SyntaxList.IndexOf | ldarga obtains the by-value argument's local copy, not the caller's storage. Foreign SyntaxList<!!0> identity and parameter constraint survive; enum kind normalizes through ushort to int before the original MemberRef. |
| 1041-06000478 Player.RelationshipWith | same-reference/null/spectating/noncombatant branches stay ordered. Each mask address is acquired before the other player's value snapshot and direct Overlaps call; enum constants and short-circuit returns match IL. |
| 1041-0600010C FieldLoader.ParseColor | null input bypasses TryParse; out location is actual zeroed local storage. Success boxes the completed Color snapshot once; failure reads the delegate then arguments and uses original callvirt. |
| 1042-06000117 Warhead.IsValidTarget | initonly addresses retain precise ldflda leaves. Exact external BitSet<TargetableType> owner/arguments persist; second acquisition/call occurs only after the first succeeds, and negation tests zero. |
| 0003-06000925 EmptyList.IEnumerator.get_Current | initobj zeroes T local, then ldloc snapshots and box uses the exact scoped T. Reference/null/nullable/value runtime cases are not replaced by a fixed integer or allocation claim. |
| 0055-060016E0 ReferenceEquality.Equals | both original box operations stay before reference identity comparison. No T equality operator or value-equality helper is substituted; two non-nullable value boxes remain distinct. |
| 0895-0600010B Binder.GetNewCompoundUseSiteInfo | Compilation call precedes Assembly callvirt and exact foreign value constructor; destination/assembly argument order and completed whole-value return persist. No reference-class freshness is claimed. |

## Negative findings repaired before this review

The constructed-value field path initially selected a same-named definition
without checking the operand's field signature. An owned MemberRef-signature-index
mutation retained an int Count declaration but requested a caller-instantiated
string signature; the method incorrectly recovered. `operand-signature-red.log`
retains the failure. Both signatures are now checked/substituted in the same
caller context and their exact stored types must match; the public negative is
green. This is distinct from the earlier shared-blob width mutation.

The shared class-only constructed-owner binder remains class-only. Value calls,
fields and newobj have separate proofs; this group does not incidentally enable
generic value MethodSpec calls, generic static fields or reference allocation.
GVM's existing runtime lookup-order refusal remains intact. Plain value callvirt,
ref-like layouts, unsupported modifiers and byref results remain gated.

## Behavior evidence

`behavior-manifest.json` has 23 passing owned drivers. The new driver executes
522 comparisons / 58 closed selections per configuration and rejects ten semantic
mutants per configuration. Local Read/Write/ReadThrow/Echo/constructor/argument
support bodies invoke recovered code, with typed pointers to shared receiver
cells. Foreign leaves perform a single original operation, never a selected body
proxy. Original and recovered runs reset the same static probe state and compare
constructor counts/traces, normal and exceptional returns, target preservation
when a constructor throws, value contents and embedded reference identity.

Legacy promoted cases execute before removal from frozen negative tables:
generic value Sum/Peek, generic declaring BoxStruct/UnsupportedBoxGeneric, scoped
UnsupportedBox, struct-copy UnsupportedBox and struct-receiver UnsupportedBox.
The NewObject generic null-conditional case stays Partial at its next actual
managed-address-CFG gate; it is not reported as recovered.

CVec.Minus remains outside the gained set; its retained ILSpy unknown-result
diagnostic is inconclusive, not evidence of ILSpy failure or a corpus exclusion.
