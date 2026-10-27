# Tail/constructor/external-field batch self-review

Main-session self-review of three gained methods, one per group in this batch (not
an independent review), each compared with production output, ILSpy and IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 0600006B `FSharpChoice<T1,T2>.CompareTo` | The `tail.`-prefixed comparison call becomes one ordinary call with the same target and arguments; the surrounding comparisons and returns are unchanged. Matches ILSpy's body. | CRUDE-BUT-EQUIVALENT (stack-depth divergence stated in TAIL_CALL_PREFIX.md) |
| 0003 / 0600080E `MemberResolveResult..ctor` | The base constructor call still runs on the original `this` and targets the immediate base; field writes and the branch that picks the base argument keep their order. | CRUDE-BUT-EQUIVALENT |
| 0004 / 0600029E `Binder.TryGetSpecialTypeMember` | The referenced value type's fields are read through the managed location of that value, and no copied-value write is emitted. | CRUDE-BUT-EQUIVALENT |

No row shows a dropped or reordered call, a different receiver, or a field access
on a copied value. The bounded sample found no changed-semantic suspect.

Batch: 706,520 Decompiled / 64,228 Partial of 770,748 (8.332818%), +5,902 in 120
images, zero regressions, zero incomplete. OpenRA.Game +22, OpenRA.Mods.Common +407.
Gate unmet: 6,422,800 > 3,853,740 (25,691 bodies to go).
