# Batch scan self-review: exception ancestry, definite assignment, instantiated foreign calls

Main-session self-review, not independent review or formal verification. Eight
gained methods were chosen deliberately to cover the three groups and five large
images; they are not a random or exhaustive sample. For each row, `evidence.zip`
retains production default output, ILSpy output, original IL, stderr and a
manifest with commands and reference selection. ILSpy exited successfully with
no unknown-result diagnostics; no third-party body was executed.

| Image / token | Group | Checked obligations | Verdict |
|---|---|---|---|
| 0000 / 060004F4 FSharp `AsyncResult.Commit` | EH ancestry | Cast-then-null tests stay in order; `Ok` field return; error path calls `item.Throw()` before returning `default`; canceled path rethrows its own item. | CRUDE-BUT-EQUIVALENT |
| 0628 / 06000002 `Interop.MemAlloc` | EH ancestry | Heap handles and size converted once; branch copies retain the same pointer value; `IntPtr.Zero` comparison precedes the local `OutOfMemoryException` allocation and throw; return uses the tested pointer. | CRUDE-BUT-EQUIVALENT |
| 0483 / 06000003 AnonymousType `Equals` | Definite assignment | `isinst` result stored before the null test; comparer default fetched before both field loads; receiver/argument field order preserved; single dispatch. | CRUDE-BUT-EQUIVALENT |
| 0483 / 060000C0 `GetBaseKeyFromKeyName` | Definite assignment | Null check and throw precede the separator search; `null` store precedes the name comparisons; out-parameter write only on the paths that reach it. | CRUDE-BUT-EQUIVALENT |
| 0501 / 06000018 `ContainsNoWhitespaceOrQuotes` | Definite assignment | Loop counter initialised, character load and `IsWhiteSpace`/quote tests keep their order; both early returns unchanged. | CRUDE-BUT-EQUIVALENT |
| 1042 / 0600007E OpenRA `AppearsFriendlyTo` | Instantiated foreign call | Relationship call order, `Ally` comparison, `EffectiveOwner` null test, `Disguised` read and `HasTraitInfo<IgnoresDisguiseInfo>` (exact `MethodSpec` leaf) evaluate in IL order. | CRUDE-BUT-EQUIVALENT |
| 1042 / 0600009E OpenRA `InnerValue` | Instantiated foreign call | Existing-actor fast path first; otherwise `WorldActor.Trait<SpawnMapActors>()` leaf, then `Actors` field, then the name key and dictionary lookup. | CRUDE-BUT-EQUIVALENT |
| 0004 / 0600007D Roslyn `CreateAnonymousFunctionConversion` | Instantiated foreign call | `Bind` before diagnostics read, `AddRange<Diagnostic>` leaf, then allocation, `WasCompilerGenerated` read and store on the new object. | CRUDE-BUT-EQUIVALENT |

All rows keep their leaves explicit; none proxies an original body or reorders
operands. The bounded sample found no changed-semantic suspect. Selected
dependency substitutions are recorded in the production output and per-image
reference report; they are metadata selection, not runtime proof.

Scan facts: 656,786 Decompiled / 113,962 Partial of 770,748 (14.785896%),
+16,561 in 201 images, zero regressions, zero incomplete images, compared with
`report/constructed-foreign-batch/corpus-scan-final/baseline.json`. OpenRA.Game
3,931/836 (+2); OpenRA.Mods.Common 15,261/3,146 (+838). The aggregate gate is
still unmet: 11,396,200 > 3,853,740.
