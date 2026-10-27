# Constructed-join and MethodSpec self-review

The main session selected gained methods from the FSharp.Core targeted scan,
with both join-heavy and MethodSpec forms. Each has production readable output,
original IL and ILSpy source under this directory. This is a bounded self-review,
not an independent review, and it does not execute third-party code. Original IL
is authoritative. All IL and ILSpy commands exited 0 and the selected ILSpy
outputs contain no unknown-result diagnostics.

| Token | Verdict | Selected obligations checked |
|---|---|---|
| 0x060010AD | CRUDE-BUT-EQUIVALENT | Each sibling allocation remains on its original branch; its reference is copied into the exact common `OptimizedClosures.FSharpFunc<int,nint,string>` slot and then into the declared function local. The later choice and reference `box` preserve the chosen identity. |
| 0x060010AF | CRUDE-BUT-EQUIVALENT | The second numeric-format closure pair uses its own exact constructed common base and original `newobj` tokens; the two branch results and later local stores are not interchanged. |
| 0x06001110 | CRUDE-BUT-EQUIVALENT for checked joins | Separate float, double and decimal closure joins retain their different complete type arguments and allocation sites. Argument loads, constrained calls and exceptional branches precede or follow those joins as in IL; this is not a line-by-line whole-method proof. |
| 0x06001116 | CRUDE-BUT-EQUIVALENT for checked joins | The decimal closure branch selects the corresponding concrete allocation and copies it to a constructed generic stack slot. The earlier format-selection branches and throw path remain visible. |
| 0x060019DE | CRUDE-BUT-EQUIVALENT | The value-type test selects field or property reflection, then allocates the corresponding reader and merges both through the common `FSharpFunc<object,object[]>` identity. The later length test and recursive reader allocation remain ordered. |
| 0x06001ABC | CRUDE-BUT-EQUIVALENT for verifier-valid reference conversions | The fixed individual output (`.fixed.txt`) retains the closure `newobj` followed by exact `MethodSpec` `0x2B000313` with the method-scoped `KeyValuePair<!!0,!!1>` argument. Explicit formal-reference casts preserve the represented original operands. The earlier `.ours.txt` is the retained Partial result, not a successful review. |

The MethodSpec sample first exposed a scan-versus-single-token mismatch; it is
retained in the audit rather than counted twice. The repaired one-thread and
2-thread FSharp.Core scans agree on every status/reason, and the individual
sample is now Decompiled. This does not establish parity for every input.
