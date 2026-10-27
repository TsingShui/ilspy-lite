# Float-call / scalar-TypeRef batch self-review

Main-session self-review of one F# and one OpenRA method gained by the two groups in
this batch (not an independent review), compared with production output, ILSpy and
IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 06000F08 (F#) | The float32 call keeps the exact MemberRef target and packs its argument with `__cil_call_arg_r4`, unpacking the result with `__cil_call_result_r4`; no Math.Abs specialization is applied. | EQUIVALENT |
| 1042 / 0600007C (OpenRA.Mods.Common) | An ordinary external call whose owner's core identity does not resolve keeps its exact token and the same argument/result packing; argument order is unchanged. | CRUDE-BUT-EQUIVALENT |

Batch: 716,283 Decompiled / 54,465 Partial of 770,748 (7.066477%), +1,569 in 133
images, zero regressions, zero incomplete. OpenRA.Game +19, OpenRA.Mods.Common +115.
Gate unmet: 5,446,500 > 3,853,740 (15,928 bodies to go).
