# Array batch self-review

Main-session self-review of three gained F# methods, not an independent review.
Each was compared with production output, ILSpy and IL. All three were previously
blocked at `array allocation needs exact external element kind facts`; the review
is scoped to the allocation and element stores that this batch admitted, not to
the surrounding F# quotation machinery.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 06000673 MakeImplicitExpressionConversion | `newarr TypeRef(0x0100001A)` allocates one `System.Type[]` at the original position; the single element store and callvirt keep their order and exact tokens. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06000683 FuncExprToDelegateExpr | Two-element allocation with `stelem` at constant indices 0 and 1 in source order; both stores target the same fresh array; no extra conversion. | CRUDE-BUT-EQUIVALENT |
| 0000 / 0600088D CleanupLeaf | Allocations of generic F# option/list arrays stay structural identities; element access tokens are unchanged and no element type is specialized to a host type. | CRUDE-BUT-EQUIVALENT |

No row shows a dropped store, a reordered operand, a widened element type or an
added conversion. The bounded sample found no changed-semantic suspect.
