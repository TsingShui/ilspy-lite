# Random state closure: Generate and Next

This is a bounded semantic gate within milestone B, not completion of the
seven-method random class. It uses the pinned `release-20250330` Game assembly
and the frozen class/method identities in [classes.json](classes.json).
Unchanged inputs reuse [manifest.json](manifest.json) provenance.

## Public output and call closure

At production revision `ac273efc8`, the actual
`decompile <assembly> --type OpenRA.Support.MersenneTwister` output reports:

- `Generate` (`0x060007A3`): Decompiled.
- `Next()` (`0x0600079F`): Decompiled; its Generate call forwards exclusively to
  the recovered Generate body.
- `Next(int32)` also lowers as a caller, but its range overload dependency is
  not recovered and it is not part of this gate.
- Both constructors, the two-argument range overload and NextFloat still have
  explicit IL fallback. Class summary: 3 decompiled, 4 partial, no failures.

The test shell contains only metadata-derived fields. It does not reference the
original random class. Exact direct-call adapters forward only to recovered
bodies, and preparation rejects an absent recovered call target. No random
algorithm is embedded in the harness or in the production decompiler.

## Reproduce

```sh
python3 rust/tests/reference-roundtrip.py --random-state --from-class
```

.NET 10 is used only by the optional gate. The production CLI and ordinary Rust
tests do not depend on it. Generated class output, source, assemblies and logs
stay in ignored `rust/report/openra/random-state/`, not tracked MIT sources.
`--verify-inputs` explicitly opts into provenance rechecks when needed.

The original side invokes the pinned methods through independent Reflection.Emit
exact calls. The recovered side executes bodies extracted from public class
output. For setup only, the original seed constructor supplies initial state,
which is cloned into a separately allocated recovered shell. **This does not
validate or recover seed construction.** Constructor allocation, initialization
order and readonly writes require their own later gate.

## Verified behavior

On the local 64-bit .NET 10.0.12 runtime:

- Seeds 0, 1, -1, int.MinValue and int.MaxValue; 624 words per initial state.
- Three successive Generate calls for each seed, requiring successful returns.
- 1,250 consecutive successful Next calls per seed, requiring integer returns.
  This crosses multiple generation boundaries.
- After every call, compare all words, index, Last and TotalCount. Neither side
  may replace the original array identity.
- Controlled states: zero/all-one/high-bit-alternating/patterned words;
  null and lengths 0, 1, 623, 624, 625; negative/boundary/out-of-range indices;
  TotalCount at both signed extremes and unchanged Last on failing paths.
  Exception types and partial state before failure are compared.
- Null receiver exceptions for both methods.

Result: **6,939 return/exception/state comparisons passed**. This is finite input
evidence, not an all-input proof or a claim of 32-bit execution. In a negative
run, removing the recovered indirect state write caused
`Random state mismatch at word ...`; the state assertions detect that defect.
Logs: `behavior.log`, `mutation.log` under the ignored gate directory.

This historical state-only gate used original seed setup. The subsequent
[RANDOM_SEED.md](RANDOM_SEED.md) gate validates recovered seed allocation and
initialization without copying state. Default-constructor chain, range/exception
and floating-point closure remain required. The frozen Activity/Wait behavioral gates remain
pending. The existing resource/ammo milestone evidence remains unchanged.
