# Milestone B: complete random-class semantic closure

Production revision: `93f849497`; owned mixed-class fallback sentinel updated
at `3ce207aae`. Inputs are the unchanged pinned release-20250330 assemblies.
No game was installed or launched, dependencies downloaded, hashes recomputed,
source size recounted or root SDK/C# engine changed for this milestone.

## Delivered class result

The public `--type OpenRA.Support.MersenneTwister` output contains all seven
original methods and four fields, with the original identity/signature/flags.
Every method is Decompiled: both constructors, all three Next overloads,
NextFloat and Generate. Exit 0; no declarations, partial/failed bodies or metadata
failures. The class is not a whitelist: the same general lowering rules apply
to other exact type selections, with independent method fallback.

`--random-class --from-class` requires all seven frozen MethodDefs, class exit 0
and Decompiled statuses. It extracts all bodies from one actual public class
output and compiles them together. Exact call/constructor adapters only forward
to recovered bodies; core Object, clock, literal and Math operations are
explicitly validated single-operation runtime boundaries. No selected random
logic is supplied by an original-body proxy or algorithm template.

## Semantic closure evidence

```sh
python3 rust/tests/reference-roundtrip.py --random-class --from-class
python3 rust/tests/reference-roundtrip.py --openra --from-class
python3 rust/acceptance/openra/inventory.py --check-cli
```

The combined random gate passed **22,941 return/exception/state comparisons**:

- Independently execute recovered seed allocation/initialization for seeds
  0, 1, -1, int.MinValue, int.MaxValue; compare all 624 words and scalar state.
- 1,250 consecutive Next calls per ordinary seed, comparing every return,
  Last, TotalCount, index, array identity and each state word.
- Repeated Generate, boundary indices/counters and null/short/full controlled
  arrays compare effects and partial state before exceptions.
- Default construction reads one signed TickCount value and delegates once on
  the same receiver. The independent original IL oracle substitutes only the
  clock boundary; receiver/seed/call ordering and null paths are observed.
- Both range overloads compare boundary/negative/degenerate/unchecked-overflow
  behavior and detailed exception type/parameter/message/HResult/ActualValue.
  Invalid/degenerate ranges do not consume state.
- 1,250 NextFloat calls per seed compare binary32 return bits and complete state
  after each call, with the exact Abs(float) operation and call ordering.

The combined gate reuses the separate contracts, not their cached success claims.
Each contract runs against the same recovered seven-body assembly. Original
constructor state is copied only in the additional historical state-only probe;
seed/default/range/float closure initializes the recovered side independently.
Separate mutation runs rejected missing state writes, altered seed arithmetic
and a substituted clock seed. Details:
[RANDOM_STATE.md](RANDOM_STATE.md), [RANDOM_SEED.md](RANDOM_SEED.md),
[RANDOM_DEFAULT.md](RANDOM_DEFAULT.md), [RANDOM_RANGES.md](RANDOM_RANGES.md),
[RANDOM_FLOAT.md](RANDOM_FLOAT.md).

PlayerResources three primary bodies and AmmoPool two getters also passed their
unchanged **837** return/state/exception comparisons. Their remaining unrelated
methods retain explicit IL fallback rather than being counted as semantic closure.

## Inventory and regressions

Independent System.Reflection.Metadata PEReader snapshots matched the complete
public output for all five frozen classes, including original ownership,
fields/methods/properties/events, flags/signatures, parameters, generics,
accessors and status/exit accounting. No metadata failures:

| Class | Decompiled | Partial | Failed | Exit |
|---|---:|---:|---:|---:|
| PlayerResources | 7 | 3 | 0 | 2 |
| AmmoPool | 4 | 7 | 0 | 2 |
| MersenneTwister | 7 | 0 | 0 | 0 |
| Activity with owned generated types | 13 | 49 | 0 | 2 |
| Wait | 0 | 3 | 0 | 2 |

Activity/Wait are complete metadata only here: their frozen Queue/Tick behavioral
contracts have not passed. They remain required and were not replaced.

Full locked Cargo Debug/Release tests, rustfmt and strict all-target clippy
passed. The owned behavior gates passed per Debug/Release configuration:
references 522, arrays 557, native 197, integer conversions 385, element addresses
1,892, strings 124, exceptions 259, floats 2,010; integer differential gate 735.
These are bounded regression domains, not percentage coverage or universal proofs.

A fresh-context read-only reviewer inspected committed production/evidence diffs
from `2ff202525..93f849497`, baseline/current sources, public output and existing
logs. No evidence-backed diff-caused semantic defects were found (OK with notes).
The reviewer did not run tests; fresh integration/gates above are parent evidence.

Logs: ignored `report/openra/semantic-milestone/`, `report/openra/random-class/`.
Review artifact: session-managed `reviews/semantic-milestone-b.md` (run
`3dc146a7-8ec8-427d-865e-6b85ba1f8113`). All GPL-derived inputs/output remain ignored.

## Boundaries and next acceptance

Execution evidence uses local 64-bit .NET 10.0.12 on macOS arm64. Native width and
CLI F precision remain target-runtime properties. The Single float adapter and
compiled literal adapter prove local behavior, not other-runtime F precision,
NaN/subnormal details, 32-bit execution or all literal-interning policies.
Production semantic intrinsics retain these contracts rather than inventing
an analyzer-host runtime. No general resolver, full constructor verifier,
value-type/generic/virtual/EH framework or source project reconstruction is claimed.

Milestone C still requires output-only Agent understanding and the two frozen
independent real classes, including behavioral Queue/Tick recovery. Passing this
random development workload is not whole-game or broad ecosystem acceptance.
