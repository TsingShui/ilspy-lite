# Range overload and throw closure

The range gate executes public class output for the frozen two-argument and
one-argument Next overloads, with recovered seed/Next/Generate dependencies.
No selected body forwards to its original implementation. The original side
uses independent exact Reflection.Emit calls against the pinned assembly.

```sh
python3 rust/tests/reference-roundtrip.py --random-ranges --from-class
python3 rust/tests/reference-roundtrip.py --exceptions --from-class
```

The OpenRA gate selects `0x0600079E`, `0x0600079F`, `0x060007A0`, `0x060007A1`,
`0x060007A3`. Generated GPL-derived bodies and logs remain in ignored
`rust/report/openra/random-ranges/`. Inputs reuse manifest provenance; explicit
`--verify-inputs` remains optional. The optional gate uses local 64-bit .NET
10.0.12, not a production runtime dependency.

## Verified behavior

- Five seeds: 0, 1, -1, int.MinValue, int.MaxValue. Initialization on each side
  is independent; no original state is copied into the recovered shell.
- Two-argument bounds from int.MinValue, int.MinValue + 1, -17, -1, 0, 1, 2,
  17, int.MaxValue - 1, int.MaxValue; all pairs. One-argument overload uses the
  same high bounds. Negative and equal/adjacent bounds remain original behavior.
- Ordered extreme bounds can overflow the original unchecked high-minus-low
  subtraction. The gate checks the resulting original branch, not a corrected
  mathematical interval or an assumed positive range.
- Index 0, 1, 623 and TotalCount 0/int.MaxValue exercise generation, index wrap
  and counter overflow. Each call compares index/Last/TotalCount, array identity,
  extent and every state word, as well as the return value.
- Invalid/degenerate ranges must not consume state. Invalid ranges throw the
  exact core ArgumentOutOfRangeException with parameter `high`; message,
  HResult and ActualValue also match. Null receiver outcomes are compared for
  every bound pair and one-argument bound, including branches that never touch
  the receiver.

Result: **3,410 return/exception/state comparisons passed**. This is finite input
and local-runtime evidence, not a universal equivalence or cross-width proof.

The owned exception gate separately passes **259 comparisons per Debug/Release
configuration**. It covers allocation/argument identity, existing exception-object
throw identity, throw-null behavior and effects before throwing. Public Rust
negative cases reject integer/string throw operands and underflow; throw empties
lower evaluation-stack values as specified by ECMA-335 III.4.31. Catch/finally,
rethrow and general non-Exception object throwing remain unsupported.

Six of the seven random-class methods now lower with dependency closure for
constructors, state and range operations. NextFloat remains a partial IL result;
floating conversion/operation/exact-call acceptance is still required before
milestone B completion. All original methods remain visible.
