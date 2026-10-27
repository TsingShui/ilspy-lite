# External reference casts: batch self-review

Main-session self-review, not independent review. Five gained methods (largest
gaining images) were checked against production output, ILSpy and IL; ILSpy
exited successfully with no unknown-result diagnostic. `evidence.zip` keeps the
outputs and commands. Cast operands keep the original token; `isinst` yields null
and `castclass` throws `InvalidCastException` on mismatch, so no assignability
proof is invented.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0054 / 06000021 NRefactory `VisitPreProcessorDirective` | Type test on the external directive type precedes the branch and returns without touching the argument; the alias denotes the exact cast token. | CRUDE-BUT-EQUIVALENT |
| 0626 / 06000154 `GetSSPIException` | Two runtime tests (`as` external type, then `IOException`) in IL order; every path returns the original exception reference, identity preserved. | CRUDE-BUT-EQUIVALENT |
| 0058 / 0600011F `GenericPropertyTest` | The `castclass` to the NRefactory interface appears once at the checked position; both `Assert.AreEqual` boxing arguments keep their order and identity. | CRUDE-BUT-EQUIVALENT |
| 0425 / 060001EB `CloneNode` | Cast to `XmlDataDocument` precedes its first use; enum storage and `OwnerDocument` read order unchanged. | CRUDE-BUT-EQUIVALENT |
| 0625 / 060006DA `FinishInitUriString` | Two casts to the same external type at their original positions inside the URI-initialization flow. | CRUDE-BUT-EQUIVALENT |

No row replaces a cast with a copy, boxing, or a guessed assignability; all five
keep the runtime operation the IL performs. The bounded sample found no
changed-semantic suspect. Aliases are input-scoped identifiers with evidence
comments, not cross-image token equality.

Batch scan: 668,334 Decompiled / 102,414 Partial of 770,748 (13.287612%), +6,368
in 165 images, zero regressions, zero incomplete, against
`report/constructed-base/corpus-scan/baseline.json`. OpenRA.Game 3,955/812 (+21),
OpenRA.Mods.Common 15,441/2,966 (+28). Gate unmet: 10,241,400 > 3,853,740.
