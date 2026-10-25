# Controlled default constructor closure

The public class output now recovers the default constructor's exact clock call
and same-type delegated constructor call, on the original receiver. The seed,
Generate and Next bodies are also recovered; see [RANDOM_SEED.md](RANDOM_SEED.md).
No field initializer is duplicated in the delegating body and no new object is
substituted for an existing receiver.

```sh
python3 rust/tests/reference-roundtrip.py --random-default --from-class
python3 rust/tests/reference-roundtrip.py --delegated --from-class
```

Both commands use actual public class bodies. The OpenRA job uses the frozen
`0x0600079D`, `0x0600079E`, `0x0600079F`, `0x060007A3` targets. Core method scopes
and signatures are validated; constructor adapters only forward to recovered
constructor bodies. Runtime Object initialization uses an exact IL call.

## Controlled clock oracle

Comparing two real TickCount reads would not establish constructor equivalence.
Instead, the independent original oracle reads the pinned original default IL
and accepts only its straight-line ldarg.0 / exact clock call / exact delegated
constructor call / ret sequence, with optional nops. It re-emits that sequence
and substitutes only the external clock boundary with a controlled Func<int>.
The input assembly is not patched and no recovered output drives this oracle.
Unsupported original instructions, signatures, targets, locals or EH fail the
gate rather than being omitted. The original seed constructor remains the
independent side's implementation; the recovered side executes only recovered
bodies, never the original selected constructor.

Clock values are int.MinValue, -1, 0, 1, int.MaxValue. The gate verifies:

- One read of the exact core Environment.TickCount operation.
- Exact signed seed bits, same receiver identity and one delegated constructor
  call, before the core Object call.
- No field/array initialization before the clock/delegated boundary.
- One newly allocated zero-filled array before the core call; all final words
  and scalars match. Dirty initial state and null receivers are included.
- Null receiver still reads the clock and delegates before the seed constructor
  fails at its field store; the core call does not occur.
- Three subsequent Next calls for every ordinary controlled seed, checking the
  resulting full state. The separate seed gate provides the longer sequence
  acceptance.

Results on local 64-bit .NET 10.0.12: **35 OpenRA return/exception/state comparisons
passed**, and **20 per Debug/Release owned delegated fixture**. Substituting zero
for the recovered clock seed is rejected by the target/argument observer and
outcome gate. These are finite controlled-clock and local-runtime contracts,
not a claim that different real times are equal or that 32-bit execution ran.

Logs and all generated GPL-derived output stay in ignored
`rust/report/openra/random-default/`; the summary log is
`rust/report/openra/random-default.log`. Unchanged inputs reuse manifest
provenance; `--verify-inputs` is optional and explicit.

The random class has five decompiled methods and two partial methods. Both
constructors and Generate/Next are behaviorally closed. The one-argument range
caller is not closed because the two-argument range overload remains partial;
NextFloat remains partial. Range/string/exception and floating work are still
required before milestone B completion. General local-base constructor calls,
static constructors, generic/value-type constructors and newobj remain outside
this bounded delegation step.
