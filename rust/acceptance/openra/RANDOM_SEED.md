# Recovered seed initialization and state closure

This gate adds the frozen `MersenneTwister::.ctor(int32)` (`0x0600079E`) to the
Generate/Next closure in [RANDOM_STATE.md](RANDOM_STATE.md). The production
implementation at `bf0ad273a` lowers the constructor initialization body from
public class output; no OpenRA-specific implementation or initialization template
is added to the MIT decompiler or test harness.

## Reproduce

```sh
python3 rust/tests/reference-roundtrip.py --random-seed --from-class
```

Inputs reuse the pinned [manifest.json](manifest.json) provenance. All generated
GPL-derived class output, C# bodies, binaries and logs stay in ignored
`rust/report/openra/random-seed/`. The optional gate uses local 64-bit .NET 10.0.12;
the production CLI needs no .NET runtime. Hash rechecks require `--verify-inputs`.

## Independent execution

Both sides start from separately allocated object shells. The original side
invokes the pinned constructor through an exact Reflection.Emit call. The
recovered side executes the initialization body extracted from `--type` output,
including fresh uint-array allocation, readonly field store, exact core Object
constructor operation and every original array/loop/arithmetic operation.
**No original array or scalar state is copied into the recovered side.** The
core operation adapter emits the exact runtime Object constructor call, not a
call to the original selected seed constructor.

For ordinary zeroed receivers, final state is also compared with a normally
allocated original constructor instance. A dirty-shell case compares exact body
behavior: the array is replaced, while scalar fields not written in IL retain
their starting values. These cases validate body semantics, not permission to
construct arbitrary invalid CLI objects or a general initialization verifier.

At the recovered core call, an observer verifies unchanged receiver identity,
exactly one call, all 624 newly allocated words still zero and the prior scalar
state intact. This protects pre/post-base initialization order independently of
the final-state comparison.

## Results

- Seeds 0, 1, -1, int.MinValue and int.MaxValue.
- Every initialized word and index/Last/TotalCount compared after construction.
- Fresh array identity, zero state before the core call, and dirty-state
  replacement/retention checked.
- For each normal seed, 1,250 consecutive recovered Next calls compared with
  the independently initialized original, including every return, all 624 words
  and scalars after each draw. An additional Generate call is also compared.
- Null receiver fails at the field store before the core call; exception type
  and zero core-call count agree.

**6,266 return/exception/state comparisons passed.** Removing the recovered seed
multiplication in a negative run caused `Seed state mismatch at word ...`.
Logs: `behavior.log`, `mutation.log` in the ignored gate directory. Finite-input,
local-runtime evidence is not a universal equivalence or 32-bit execution proof.

Class output has four decompiled methods and three partial methods at this
production revision. The default constructor, two-argument range overload and
NextFloat remain incomplete. The one-argument range method is lowerable as a
caller but is not closed until its dependency is recovered. All seven original
members remain visible. Default-clock/delegated-constructor, range/exception and
floating closure are still required before milestone B is complete.
