# Native-integer / reference-comparison batch self-review

Main-session self-review of two gained F# printf helpers (not independent review).
Both previously stopped at `integer operation on non-integer value` and are now
recovered through the native integer category. Compared with ILSpy and IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 060010AC | The `native int` argument keeps its native width; the explicit `conv.i` at IL_001F remains in the body and no fixed-width cast replaces the parameter. | CRUDE-BUT-EQUIVALENT |
| 0000 / 060010B5 | Same native-width parameter and explicit `conv.i`; the following `long` local is the converted value, not a reinterpretation. | CRUDE-BUT-EQUIVALENT |

Neither row introduces a narrowing of the native value or reorders the conversion
relative to the IL. The bounded sample found no changed-semantic suspect.

Batch scan: 684,313 Decompiled / 86,435 Partial of 770,748 (11.214766%), +4,918 in
122 images, zero regressions, zero incomplete images. OpenRA.Game +9,
OpenRA.Mods.Common +52. Gate unmet: 8,643,500 > 3,853,740.
