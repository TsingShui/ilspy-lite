# Open-self allocation batch self-review

Main-session self-review of two gained F# union constructors (not an independent
review), compared with production output, ILSpy and IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 0600017F `FSharpOption<T>.Some(T)` | One allocation of `FSharpOption<!0>` at the IL site, constructed with the argument in order; the result is the open self instantiation the caller's scope names. Matches ILSpy's `new FSharpOption<T>(value)`. | EQUIVALENT |
| 0000 / 060001B1 `FSharpList<T>..cctor` | The static initializer allocates the open generic list through its MethodDef constructor; the allocation stays a single `new` with the original arguments and is assigned to the static field. | CRUDE-BUT-EQUIVALENT |

Batch: 714,714 Decompiled / 56,034 Partial of 770,748 (7.269927%), +1,524 in 89
images, zero regressions, zero incomplete. OpenRA.Game +19, OpenRA.Mods.Common +13.
Gate unmet: 5,603,400 > 3,853,740 (17,497 bodies to go).
