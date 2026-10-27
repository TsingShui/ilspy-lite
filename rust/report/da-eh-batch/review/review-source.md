# Batch self-review: local core scalars and exception-region assignment

Main-session self-review of three gained methods (not an independent review), one per
changed path, each compared with production output, ILSpy and IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0483 / 060000BD `Win32Native..cctor` | The local `System.IntPtr` construction keeps the exact constructor token and the `-1` argument; the value is the scalar the IL builds, not an opaque struct copy. | EQUIVALENT |
| 0248 / 0600011E `ModuleUninitializer.SingletonDomainUnload` | The handler body reads only locals the protected region assigns before any possible throw; the emitted statements keep their order and the scalar receiver types. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06000A78 `NullableModule.ToIntPtr<T>` | `box`/`unbox.any` of `System.IntPtr` keeps the scalar box (native value boxed as `IntPtr`), and the generic parameter copy keeps its constraints. | CRUDE-BUT-EQUIVALENT |

No row introduces a cast that the IL does not need, reorders a member, or turns a
value into a reference. The bounded sample found no changed-semantic suspect.

Batch: 720,202 Decompiled / 50,546 Partial of 770,748 (6.5579%), +2,437 in 29
images, zero regressions, zero incomplete. Gate unmet: 5,054,600 > 3,853,740
(12,009 bodies to go).
