# Batch self-review: address writes and Silverlight core identity

Main-session self-review of two gained methods (not an independent review), compared
with production output, ILSpy and IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0014 / 06000001 `PortableClassLibrary.Klass.Perform` | The `ldtoken`/`GetTypeFromHandle` pair keeps the exact tokens and the `RuntimeTypeHandle` value now has the core identity from the Silverlight `mscorlib` (Microsoft key `7cec85d7bea7798e`); no cast or value reinterpretation is added. | EQUIVALENT |
| 0014 / 06000002 `PortableClassLibrary.Klass..ctor` | The constructor call and its empty body preserve order and the same declaring type; the core identity now resolves through the same admitted scope. | EQUIVALENT |

Batch: 720,579 Decompiled / 50,169 Partial of 770,748 (6.5053%), +377 in 19 images,
zero regressions, zero incomplete. Gate unmet: 5,016,900 > 3,853,740 (11,632 bodies
to go). The following correctness-only commit (`87cb6f96f`, open-self structs as
values) measured zero status change.
