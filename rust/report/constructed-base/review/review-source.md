# Constructed base calls: batch self-review

Main-session self-review, not independent review. Eight gained constructors whose
previous blocker was `constructor call needs a reference-class owner` were
reviewed against production output, ILSpy and IL. ILSpy exited successfully with
no unknown-result diagnostic. `evidence.zip` keeps the outputs and commands.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 06000651 F# `...GetEnumerator@113-1` | Declared base `FSharpFunc`2<KVP<!0,!2>, IEnumerable<KVP<!1,!2>>>` is the call owner; base call precedes the captured-field store. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06000653 F# `...GetEnumerator@113` | Owner is the permuted instantiation `DictImpl`3<!2,!0,!1>`; the exact constructor token is kept and the captured store follows. | CRUDE-BUT-EQUIVALENT |
| 0129 / 06000658 F# `dictValueType@98-2` | Parameterless base call on the constructed base; no added allocation or argument reordering. | CRUDE-BUT-EQUIVALENT |
| 0129 / 0600065A F# `dictValueType@98-3` | Same shape in the sibling type; token identity preserved. | CRUDE-BUT-EQUIVALENT |
| 0054 / 06000037 NRefactory `AddArgumentNameAction` | Base call kept explicitly; ILSpy prints an empty body because `base()` is implicit there. | CRUDE-BUT-EQUIVALENT |
| 0054 / 06000056 NRefactory `CheckIfParameterIsNullAction` | Same shape, distinct base token. | CRUDE-BUT-EQUIVALENT |
| 1042 / 060005F0 OpenRA `PowerDownOrderGenerator` | `"PowerDown"` literal evaluated once before the base constructor call; matches ILSpy's `: base("PowerDown")`. | CRUDE-BUT-EQUIVALENT |
| 1042 / 060005F3 OpenRA `SellOrderGenerator` | Same with `"Sell"` and its own token. | CRUDE-BUT-EQUIVALENT |

No row reorders the receiver or arguments, and none replaces the base call with a
placeholder. The bounded sample found no changed-semantic suspect. Selected
dependency substitutions remain recorded per image and are metadata selection,
not runtime proof.

Batch scan facts: 661,966 Decompiled / 108,782 Partial of 770,748 (14.113822%),
+5,180 in 69 images, zero regressions, zero incomplete images, against
`report/exception-ancestry/batch-scan/baseline.json`. OpenRA.Game +3,
OpenRA.Mods.Common +152. The aggregate gate is unmet: 10,878,200 > 3,853,740.
