# External calls with float operands

The call path refused every external member call whose arguments or return type
involved a float unless it was exactly `System.Math.Abs(float32)` (1,089 methods in
the last fixed scan). That gate also fired when the owner's core identity did not
resolve (`type scope is not an admitted core identity`), so ordinary calls such as
`Math.Sqrt(double)` or `Math.Min(float, float)` were refused although the lift
already has exact packing helpers for float32 arguments and results and needs none
for binary64.

## Admission

- The exact `Math.Abs(float32)` identity/signature check still produces the
  dedicated `__cil_abs_r4_<token>` helper and nothing else does.
- Every other external call with float operands or a float result is emitted as an
  ordinary call. Float32 arguments are packed with `__cil_call_arg_r4(...)` and a
  float32 result is unpacked with `__cil_call_result_r4(...)`, both converting at
  the declared type; binary64 values pass through unchanged. The call target is the
  IL's exact MemberRef token.
- The owned identity mutation control now asserts that the exact-Abs helper never
  follows a mutated identity; a mutated call may be admitted as a plain external
  call (with the same packing) or keep a stated fallback.

## Evidence

- Red: F# `06000F08` was `Partial` on
  `external float call needs exact core Math.Abs(float32) identity/signature`, and
  OpenRA `0600007C` on `type scope is not an admitted core identity` (both logged);
  both decompile now and use the packing helpers.
- Targeted scans: `FSharp.Core` 8,669 -> 8,717 (+48), `OpenRA.Mods.Common`
  16,788 -> 16,903 (+115), `OpenRA.Game` 4,261 -> 4,280 (+19),
  `System.Private.CoreLib` unchanged.
- 488 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. Behavior: float32 packing/unpacking is exercised by the existing float
  drivers' owned methods; no third-party body was executed.
