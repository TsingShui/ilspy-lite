# Pointer batch self-review

Main-session self-review of three gained methods from ILSpy's own `UnsafeCode`
correctness cases (not an independent review). Each was compared with production
output, ILSpy and IL.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0002 / 060001F9 PointerArithmetic | IL is `ldc.i4.2; conv.i; ldc.i4.4; mul; add`; the emitted body keeps the explicit scale, then adds through `byte*` and casts back to `int*`. Matches ILSpy's `(int*)((byte*)p + 2 * 4)`. | EQUIVALENT |
| 0002 / 060001FB PointerArithmetic3 | IL adds the raw constant `3` to the pointer; both our output and ILSpy emit `(long*)((byte*)p + 3)`, so no element scaling is invented. | EQUIVALENT |
| 0002 / 060001FE PointerSubtraction | IL subtracts byte addresses and divides by 8; the emitted `(byte*)p - (byte*)q` keeps the byte count and the following `/ 8` is preserved. Matches ILSpy. | EQUIVALENT |

The three cases are the decompiler's own pointer-arithmetic fixtures, so this also
cross-checks the scaling rule against ILSpy's semantics directly.
Batch: 687,105 Decompiled / 83,643 Partial of 770,748 (10.852113%), +2,792 in 44
images, zero regressions, zero incomplete. Gate unmet: 8,364,300 > 3,853,740.
