# Typed vector operations audit

Baseline local 79eddb28c: 531,492 Decompiled / 239,256 Partial of 770,748
bodies; 31.0420526553426% fallback. Fixed implementation inventory, hashes,
exclusions and exact references. No input replacement or third-party execution.

audit/manifest.json retains nine exact production default/ILSpy/original-IL
triples, all successful ILSpy runs without explicit unknown-result diagnostics.
Original IL, not ILSpy presentation, defines the operation and timing.

| Method | IL obligations |
|---|---|
| 0000-06000533 FSharp Queue<T>.Dequeue | Generic ldelem copies item before default store; repeated field/index reads, initobj and size/head writes retain original order. |
| 0000-06000534 Queue<T>.Enqueue | SZArray length checks precede capacity change; generic store and later head/tail/size reads remain separate. Native length bits and integer conversions matter. |
| 0003-060002FA KnownTypeReference.Get | Read exact reference from field vector, with null/index checks at ldelem.ref; no fresh reference. |
| 0003-06000117 CecilLoader.HasDynamicAttribute | Struct vector length and ldelema precede value receiver calls; acquired element remains actual array storage. Foreign value facts/EH are independent gates. |
| 0895-0600000F NullableAttribute..ctor(byte) | Exact fresh zero-initialized byte vector, dup identity, truncating stelem.i1 and final field write after base ctor. Base constructor remains separate proof. |
| 1041-0600002A Actor.Tick | Interface reference vectors saved before loops; length and element reads preserve check timing and call dispatch. Never execute this production method. |
| 1041-06000D3B HardwarePalette.SetColorShift | Float32 stores at calculated indices preserve bits/rounding and repeated field reads, no bulk rewrite. |
| 1042-06000586 GridPathGraph.set_Item | Element index call occurs before exact whole-struct copy into vector; keep prior effects on store exception. |
| 1042-060000D4 Util.RandomDistance | Length drives empty/single/multiple paths. Single element is copied; multiple path acquires foreign struct element addresses before field reads and random call. Other field/ctor gates remain separate. |

First seam: public decompile/scan for checked SZArray length, then owned actual
default-output execution on null/empty/nonempty vectors, including generic and
otherwise unsupported element evaluation types. Length needs vector shape, not
element layout/value-operation admission. Keep scalar/vector operand validation
separate. Subsequent slices cover exact element opcodes, allocation, invariant
managed addresses, covariance/store checks and copies with owned reds/mutants.

No casts or generic erasure may replace array runtime type. Preserve zero
initialization/freshness, signed/native indices and lengths, bounds/null checks,
ArrayTypeMismatch behavior, acquisition before argument effects, byte-preserving
Boolean storage, generic scope and independent value copies. Multidimensional,
modified, pinned, escaping, readonly-prefix and unknown shapes remain independent
gates until validated. No reference-policy relaxation or recovery forecast.
