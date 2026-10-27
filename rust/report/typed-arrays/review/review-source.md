# Typed vectors: main-session source/IL self-review

Bounded main-session self-review, not independent or formal verification.
manifest.json retains fourteen gained-method triples: production default readable
output, ILSpy source, original IL, exact reference lists, commands and statuses.
Seven were pre-audited; seven extend coverage to width, generated, allocation and
copy paths. All tool runs succeed and none has an explicit ILSpy unknown-result
diagnostic. Original IL is authority; ILSpy is not the correctness oracle.

| Method | Checked obligations | Verdict |
|---|---|---|
| 0000-06000533 Queue<T>.Dequeue | Empty test/throw precede access; original T element is copied before default clearing, head modulus and size write. Repeated array/head reads remain at original sites. | CRUDE-BUT-EQUIVALENT |
| 0000-06000534 Queue<T>.Enqueue | Length checks and long capacity arithmetic precede exact SetCapacity dispatch; T store precedes tail/size updates. Original declaring T and exception ordering are retained. | CRUDE-BUT-EQUIVALENT |
| 0003-060002FA KnownTypeReference.Get | Exact static vector snapshot, signed enum index bits, then one reference load with original null/bounds checks and returned identity. | CRUDE-BUT-EQUIVALENT |
| 0895-0600000F NullableAttribute..ctor | Base Attribute constructor precedes fresh byte[1], byte truncation/store and destination field assignment. No array initializer reordering or cached array. | CRUDE-BUT-EQUIVALENT |
| 1041-0600002A Actor.Tick | Original idle snapshot survives activity mutation. Each interface vector is acquired once per branch; length checks, reference loads and exact callvirt stay in loop order. | CRUDE-BUT-EQUIVALENT |
| 1041-06000D3B HardwarePalette.SetColorShift | GetPaletteIndex runs once; five separate field snapshots/index calculations and R4 stores remain ordered, so an earlier write survives a later failure. R4 storage rounding stays explicit. | CRUDE-BUT-EQUIVALENT |
| 1042-06000586 GridPathGraph.set_Item | infos is acquired before InfoIndex; the original CellInfo token store copies the whole supplied value. No early element check, reconstructed struct or receiver copy. | CRUDE-BUT-EQUIVALENT |
| 0625-0600004E BinXmlSqlDecimal.UIntFromByteArray | Four byte reads remain left-to-right, zero-extended before shifts 0/8/16/24. Index additions and final uint conversion preserve unchecked bits; failures do not move to another access. | CRUDE-BUT-EQUIVALENT |
| 0003-06000900 BitSet.get_Item | Original WordIndex call precedes ulong storage read; ldelem.i8 retains 64 bits through signed carrier, masked shift and final unsigned comparison. | CRUDE-BUT-EQUIVALENT |
| 1042-060007C2 DefaultSpriteSequence.GetAlpha | One alpha snapshot branches on null; null returns original 1.0 bits without access, otherwise original frame-indexed R4 load/return. | CRUDE-BUT-EQUIVALENT |
| 0625-060003ED XmlCharCheckingWriter.ReplaceNewLines | Unsigned UTF-16 loads preserve surrogate/high-bit chars. Null guard, newline/CRLF lookahead bounds, increments, lazy StringBuilder allocation and Append calls retain order. | CRUDE-BUT-EQUIVALENT |
| 0003-06000006 AnonymousType0<T,U>.ToString | Fresh object[2] precedes field snapshots/ToString. Separate declaring variables, copied locals, null/nullable boxing tests, ref joins and constrained calls remain; element 0 store precedes element 1 evaluation and original Format overload. | CRUDE-BUT-EQUIVALENT |
| 0000-06000536 Queue<T>.SetCapacity | Original ZeroCreate MethodSpec precedes size checks. Array.Copy calls retain exact declared Array operands and read/check timing; array/head/tail writes follow successful copies only. | CRUDE-BUT-EQUIVALENT |
| 0000-06000D5C generated Parallel<T>.Invoke | T result store happens before activation/context snapshots and Interlocked.Decrement on the original count location; final MethodSpec retains arguments and original T. | CRUDE-BUT-EQUIVALENT |

No changed-semantic SUSPECT found in this bounded sample. The audit's
CecilLoader.HasDynamicAttribute remains Partial for an unproved foreign cast;
Util.RandomDistance remains Partial for an integer-category mismatch. They are
not gains. No runtime execution of these or other third-party bodies occurred.
Owned execution separately covers acquisition/argument trace order, raw Boolean
byte 2, float bit patterns, copies, aliases, covariance checks and allocation
freshness. This does not establish whole-corpus or cross-runtime equivalence.
