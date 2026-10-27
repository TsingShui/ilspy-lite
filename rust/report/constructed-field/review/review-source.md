# Foreign constructed field batch self-review

Main-session self-review of three gained methods from ILSpy's own decompiler
(not an independent review), compared with production output, ILSpy and IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0003 / 06000F75 TransformCollectionAndObjectInitializers | `InstructionCollection<ILInstruction>` element calls and `Dictionary<...>`/`List<AccessPathElement>` reads stay on the same instances in the same order; the structural signatures name the same constructed types as the IL. | CRUDE-BUT-EQUIVALENT |
| 0003 / 06001039 AsyncAwaitDecompiler | The `Dictionary<Block, ValueTuple<ILVariable, IField>>` accesses keep the substituted tuple instantiation in the value identity, and every member name used appears in the IL/ILSpy output. | CRUDE-BUT-EQUIVALENT |
| 0003 / 0600104E StateRangeAnalysis | `IEnumerator<ValueTuple<Block, LongSet>>` enumeration and `get_Current`/`get_Count` calls preserve order and owner identity; no invented member. | CRUDE-BUT-EQUIVALENT |

No row shows a dropped access, a reordered member call or a different generic
instantiation than the IL names. The bounded sample found no changed-semantic
suspect. Batch: +2,529 in 47 images, zero regressions.
