# MethodSpec group: main-session self-review

Ten default-readable / ILSpy / original-IL triples are recorded in
`manifest.json`, with exact reference sets and successful command status.
No third-party code executed. Static self-review is not independent verification.

| slug | original-IL check | verdict |
|---|---|---|
| 0000-06000003 | null branch bypasses TypeTestGeneric<Unit>, otherwise same MethodSpec determines bool | CRUDE-BUT-EQUIVALENT |
| 0004-06000013 | caller !!0 retained, SyntaxList copied whole, ushort enum normalized, IndexOf result compared signed | CRUDE-BUT-EQUIVALENT |
| 0002-06000159 | two distinct MethodSpecs and overload identities retained, same null arguments and order | CRUDE-BUT-EQUIVALENT |
| 0003-06000119 | first provider field snapshot, second provider/read, allocation, then nonvirtual generic Intern callvirt | CRUDE-BUT-EQUIVALENT |
| 0005-06000277 | repeated getter order, two ImmutableArray snapshots into SequenceEqual, scope loop and identity joins retained | CRUDE-BUT-EQUIVALENT |
| 0006-06000027 | array snapshot and size load precede exact Array.IndexOf<!0> with four original arguments | CRUDE-BUT-EQUIVALENT |
| 0007-06000056 | value and null settings passed to exact DeserializeObject<!!0> overload; result copied/returned | CRUDE-BUT-EQUIVALENT |
| 0008-06000037 | interned string and exact NotNull<Builder<!!0>> before Count getter and signed comparison | CRUDE-BUT-EQUIVALENT |
| 1041-06000037 | World and TraitDict field reads before nonvirtual generic Get<!!0> callvirt; same this argument | CRUDE-BUT-EQUIVALENT |
| 1042-06000077 | four separate Array.Empty<string> calls/stores before exact core Object initialization; no allocations invented | CRUDE-BUT-EQUIVALENT |

Ten CRUDE-BUT-EQUIVALENT, zero SUSPECT for the changed MethodSpec binding and
call semantics. No differences found against original IL for those operations.
Tokens retain full instantiation/overload identity; they are not names chosen
from return types. Existing structural aliases, exact-call notation and metadata
source spelling are not a whole-corpus C# compilation guarantee.

GVM callvirt is excluded after an owned original/recovered ordering discrepancy:
the initial gate and reproduction logs are retained, with details in AUDIT.md.
Neither a short successful GVM sample nor original-IL appearance overrides that
failed behavior gate. No GVM gains are counted.

Owned execution is separate: 122 comparisons / 13 selections per release/debug
for static/nonvirtual calls, cross type/method scope substitution, reference and
struct copies, state, missing-constructor generic argument identity and argument
side effects. Four semantic mutants fail. Expanded legacy gates also exercise
newly promoted allocation, generic reference forwarding and Enumerable.Count
cases rather than dropping them from the behavior selection.
