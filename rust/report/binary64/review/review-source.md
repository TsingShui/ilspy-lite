# Binary64 batch self-review

Main-session self-review, not independent review. Three gained methods whose
previous blocker was the missing signature type were checked against production
output, ILSpy and IL (ILSpy exited successfully, no unknown-result diagnostic).

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 06000625 F# `ToDouble<1>` | Literal and `NotSupportedException` allocation precede the throw; generic parameter stays a value snapshot. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06001FD2 F# `Operators.ToDouble` | Same shape; the exact constructor token and argument order are kept. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06002038 F# `RangeDouble` | Three binary64 parameters are passed in order to the iterator allocation and returned; constructed return type keeps its instantiation. | CRUDE-BUT-EQUIVALENT |

No row replaces an operation, reorders arguments or drops a value. The bounded
sample found no changed-semantic suspect.

Batch scan: 670,843 Decompiled / 99,905 Partial of 770,748 (12.995690%),
+2,509 in 109 images, zero regressions, zero incomplete, against
`report/external-casts/corpus-scan/baseline.json`. OpenRA.Game 4,013/754 (+58),
OpenRA.Mods.Common 15,476/2,931 (+35). Gate unmet: 9,990,500 > 3,853,740.
