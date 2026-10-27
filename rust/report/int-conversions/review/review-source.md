# Integer conversion batch self-review

Main-session self-review, not independent review. Two gained F# printf helpers
whose previous blocker was `unsupported opcode conv.u4` were checked against
production output, ILSpy and IL (ILSpy exited successfully). The review is limited
to the conversion site and its immediate storage, which is what this group
changed; the surrounding printf machinery was already recovered by earlier groups
and is not re-certified here.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 060010C1 F# `f@1027-12` | The native argument is truncated by `conv.u4` at its original position and stored in a `uint` local; the following member call takes the acquired address of that local, unchanged. | CRUDE-BUT-EQUIVALENT |
| 0000 / 060010F7 F# `f@841-30` | `conv.u4` from the `uint` argument remains a 32-bit zero-extension at the same position before the string-concatenation path. | CRUDE-BUT-EQUIVALENT |

No row introduces an extra conversion, reorders operands or changes storage width.
The bounded sample found no changed-semantic suspect.

Batch scan: 672,118 Decompiled / 98,630 Partial of 770,748 (12.796651%),
+1,254 in 76 images, zero regressions, zero incomplete, against
`report/conv-run/corpus-scan/baseline.json`. OpenRA.Game unchanged,
OpenRA.Mods.Common +2. Gate unmet: 9,863,000 > 3,853,740.
