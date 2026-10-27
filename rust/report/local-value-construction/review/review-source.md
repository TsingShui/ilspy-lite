# Local value construction: main-session source/IL self-review

Bounded main-session self-review, not independent or formal verification.
Eight gained methods retain production default readable output, ILSpy source,
original IL, exact reference lists and command statuses in manifest.json.
All runs succeed without explicit ILSpy unknown-result diagnostics. Original IL
is authority. No third-party body was executed.

| Method | Checked obligations | Verdict |
|---|---|---|
| 0003-060000A6 PlainTextOutput.get_Location | line/column snapshots precede the conditional indent read; unchecked addition precedes exact value newobj, then completed value return. | CRUDE-BUT-EQUIVALENT |
| 0003-06000179 DomRegion.get_Begin | Fields are read through the original value receiver; exact TextLocation construction receives the two snapshots without retargeting this. | CRUDE-BUT-EQUIVALENT |
| 1041-06000057 CPos..ctor(int,int) | Original acquired this cell is passed to the three-argument value constructor with byte-zero argument; no allocation or synthesized clearing. | CRUDE-BUT-EQUIVALENT |
| 0002-0600020C ValueTypeCall.InstanceFieldTests | Earlier field writes/Console call remain before newobj; readonly-member value construction completes before destination assignment and later field read. | CRUDE-BUT-EQUIVALENT |
| 0493-060000E1 ManagedTextSection.GetImportAddressTableDirectoryEntry | Original startup-stub call/branch retains the explicit default/initobj path. Other path evaluates size then exact DirectoryEntry constructor, not a reference allocation. | CRUDE-BUT-EQUIVALENT |
| 1042-060005AD GraphEdge.ToConnection | Destination value copy precedes Cost snapshot and exact GraphConnection constructor; foreign CPos identity stays selected/scoped. | CRUDE-BUT-EQUIVALENT |
| 0002-060001C4 InitializerTests.NotAStructInitializer_ExplicitConstructor | .ctor writes the actual local cell; subsequent field/property writes, Y(), value snapshot, fresh box and X call remain ordered. | CRUDE-BUT-EQUIVALENT |
| 0003-06000947 LongSet..ctor | Original this ref crosses the conditional by ref reassignment. Empty static value/field read versus ImmutableArray.Create stay branch-specific; exact final ctor initializes that same cell. | CRUDE-BUT-EQUIVALENT |

No changed-semantic SUSPECT in this sample. The audit's foreign CVec.Minus
constructor is not a gain; its ILSpy unknown-result diagnostics remain
inconclusive. Already recovered constructor bodies are support evidence, not
counted new recoveries. No whole-corpus or cross-runtime equivalence claim.
