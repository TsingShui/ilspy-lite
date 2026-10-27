# Interface/receiver/byref-like batch self-review

Main-session self-review of three gained F# `WebExtensions` `Invoke` methods (not an
independent review), previously blocked at `constructed member parent must be a
class`. Each was compared with production output, ILSpy and IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 06000BD3 AsyncDownloadString | The `Tuple<3>` field reads and the `FSharpFunc` calls keep the constructed instantiations the IL names, in the same field/call order. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06000BEF AsyncDownloadData | Same tuple shape with a `byte[]` element; no element type is widened or specialized to a host type. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06000C0B AsyncDownloadFile | `FSharpFunc<Unit,Unit>` instantiation preserved through the field read and the following calls; no invented member. | CRUDE-BUT-EQUIVALENT |

No row shows a dropped access, a reordered call or a different generic
instantiation. The bounded sample found no changed-semantic suspect.

Batch: 697,630 Decompiled / 73,118 Partial of 770,748 (9.486619%), +7,115 in 108
images, zero regressions, zero incomplete. OpenRA.Game +20, OpenRA.Mods.Common +35.
Gate unmet: 7,311,800 > 3,853,740.
