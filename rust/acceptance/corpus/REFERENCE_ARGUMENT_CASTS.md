# Reference arguments whose assignability proof is missing

A call argument whose static type is a reference type can be passed to a parameter
of another reference type when the verifier can prove the assignment. The lift asked
`Image::assignable` for that proof and refused the whole method when it was missing
even though the argument path already had a place to emit an explicit cast
(`((ParameterType)(value))`). The remaining `incompatible value/receiver types`
bucket was dominated by such pairs (constructed interface relationships that the
dependency facts cannot map into the caller's tokens, sibling classes, interface
edges of referenced types).

## Policy

- At a **call argument only**, when both the value and the parameter are reference
  types (kind `Ref`) and `assignable` fails, the argument is emitted with the
  explicit cast the path already supports, plus a note in the detailed view. The
  cast is `castclass`: it preserves reference identity, passes null through, and
  performs the runtime check the IL verifier already guarantees for this argument -
  on verifier-valid IL it therefore cannot throw.
- Everything else keeps its refusal: `ref`/`out` addresses, value types, enums,
  integers, floats, function pointers, and identical-type arguments (no cast is
  added when the types are equal).
- No receiver, member or dispatch identity is invented: the callee token is still
  the IL's token, only the argument expression gains a cast. This mirrors ILSpy,
  which emits casts from its own type resolution and does not stop at a proof
  failure.
- **Stated limitation:** on unverifiable IL this cast can throw where the original
  IL might have "worked" by luck; the same assumption underlies ILSpy's output.
  Verifier-valid inputs are unaffected.

## Evidence

- Red: owned `PassRoot(LocalForeign)` (a local class passed to a referenced
  interface parameter) was `Partial` on `incompatible value/receiver types`; it now
  decompiles to `AsRoot(((IRoot)(value)))`.
- The relation driver executes the new reference-argument cases against the owned
  library (`LocalForeign` is a real `IRoot`): 695 comparisons / 73 closed
  selections, all mutants rejected.
- 488 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Targeted scans: `OpenRA.Mods.Common` 16,387 -> 16,775 (+388),
  `OpenRA.Game` 4,151 -> 4,242 (+91), `ICSharpCode.Decompiler` 9,353 -> 9,429
  (+76), `System.Private.CoreLib` 15,101 -> 15,113 (+12), `FSharp.Core`
  8,611 -> 8,617 (+6).
