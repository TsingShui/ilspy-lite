# NextFloat closure and exact external call

The actual public class output for `0x060007A2` retains the exact
`Math.Abs(float32)` MemberRef (`0x0A0004BA`), including its validated core scope
and signature. It uses `__cil_abs_r4_TOKEN`, not name-only overload binding.
Argument/result boundaries remain explicit, as do integer conv.r4 rounding,
raw denominator bits `0x4F000000` and CLI F division precision.

```sh
python3 rust/tests/reference-roundtrip.py --random-float --from-class
python3 rust/tests/reference-roundtrip.py --floats --from-class
```

The OpenRA job selects recovered seed/Next/Generate/NextFloat bodies. Its exact
call adapters forward only to those recovered bodies. The Math adapter performs
one validated external `Math.Abs(float)` operation, never original selected
method logic. Seed initialization is executed independently on both sides;
no original state is copied into the recovered side.

## Evidence

On local 64-bit .NET 10.0.12:

- Each of seeds 0, 1, -1, int.MinValue and int.MaxValue runs 1,250 consecutive
  NextFloat calls, crossing generation boundaries.
- Every return is compared by binary32 bits, not numeric tolerance. After each
  call all 624 words, index, Last, TotalCount and state array identity match.
- An observer requires one Abs call, after the random state update, using the
  expected finite nonnegative interval input.
- Controlled null/empty/short/full arrays, boundary indices, counter extremes
  and null receiver compare detailed exception/state outcomes.

**6,291 return/exception/state comparisons passed.** All generated GPL-derived
output and logs remain in ignored `rust/report/openra/random-float/`. Unchanged
inputs reuse manifest provenance; hash checks require explicit --verify-inputs.

The owned float gate passes **2,010 comparisons per Debug/Release configuration**.
It includes exact Abs input bits after mutation, signed zeros, subnormals,
infinities, NaN payloads, raw constants, conversion boundaries and division
snapshots. Metadata/overload/scope/name spoofs remain unsupported.

CLI F precision is target-runtime-defined. The test-only Single carrier verifies
this local runtime's outputs; it is not an implementation-independent F engine
or a claim of 32-bit/other-runtime execution. NaN payload details and subnormal
behavior have only this local evidence. The production CLI ships no .NET runtime
or semantic adapter library.

All seven random-class methods now lower without IL fallback. Individual seed,
default, state, ranges and floating gates have passed; the fixed multi-class
milestone gate and independent Activity/Wait delivery acceptance remain required
before declaring the overall goal complete.
