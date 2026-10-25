# Class-level delivery acceptance

The confirmed three-milestone class-level goal is delivered within its bounded
method-lowering contract. Final production/test revision checked: `74407e18f`.
The final documentation adds [SEMANTICS.md](../../SEMANTICS.md); historical
baseline/stage reports remain historical rather than being rewritten.

## Public interface and member completeness

```sh
(cd rust && cargo build --release --locked)
rust/target/release/ilspy-lite decompile Assembly.dll --type Namespace.Type
```

Selection is by exact metadata TypeDef name, independent of body support, and
includes recursively owned nested/generated types. Empty and generic types,
private/generated members, overloaded methods, constructors, properties/events
and accessors remain visible. The original identity, flags, base/interfaces,
generic parameters/constraints, fields/constants and all method signatures/statuses
are retained. There is no production acceptance-class/token whitelist.

Unsupported methods keep complete readable IL and reasons independently of
successful peers. Unavailable/invalid evidence is explicitly failed, not invented.
Exit 0 means successful bodies/legal declarations; 2 means fallback without
failure; 1 means input/selection/body/metadata failure. Metadata failures are
separate from failed-method counts. Agent-readable temporaries/labels/intrinsics
are permitted; compilable source/project export is not the contract.

Independent System.Reflection.Metadata PEReader snapshots match all five frozen
public class outputs, including complete ownership/member identities, flags/raw
signatures, generic context, parameters/constants, accessors and status/exit
accounting. Final results have no failed bodies or metadata failures:

| Class | Decompiled | Partial | Exit | Required behavioral closure |
|---|---:|---:|---:|---|
| PlayerResources | 7 | 3 | 2 | Three resource bodies passed |
| AmmoPool | 4 | 7 | 2 | Both required getters passed |
| MersenneTwister | 7 | 0 | 0 | All seven methods/dependencies passed |
| Activity with four owned generated types | 14 | 48 | 2 | Frozen Queue body passed |
| Wait | 1 | 2 | 2 | Frozen Tick body passed |

Counts describe these selections, not semantic coverage percentages. Unrelated
partial members are still limitations, not successful decompilations.

## Semantic and independent-class evidence

Actual bodies are extracted from public --type output and executed, not
regenerated via --method or proxied to the original selected method. State shells
and single-operation/exact-call adapters are owned test tooling only.

- Seven-body random-class assembly: **22,941** return/exception/state comparisons.
  Seed 0/1/-1/int.MinValue/int.MaxValue initialization compares all 624 words;
  at least 1,250 consecutive Next and NextFloat calls per ordinary seed compare
  returns/binary32 bits and full scalar/array state. Default construction uses
  an independent original-IL oracle with only the clock boundary controlled.
  Range checks include unchecked endpoint overflow and detailed exception
  type/parameter/message/HResult/ActualValue. See
  [SEMANTIC_MILESTONE.md](SEMANTIC_MILESTONE.md) and its linked slice contracts.
- Existing resource/ammo state/return/exception gate: **837**, still passing.
- Frozen Activity.Queue: **20** finite-chain link/identity/null comparisons;
  existing and prelinked appended chains are preserved. Cycles are not executed
  as termination proof. See [ACTIVITY_QUEUE.md](ACTIVITY_QUEUE.md).
- Frozen Wait.Tick: **193** cancellation/countdown/old-value/repeated-call/null
  comparisons. The exact unchanged external base getter remains a permitted
  independent leaf, never a replacement for selected Tick logic. See
  [WAIT_TICK.md](WAIT_TICK.md).

Neither independent sample was replaced after lowering evaluation or failure.
They are purpose-selected, nontrivial real classes, but belong to the same OpenRA
release and share an activity abstraction. This proves the frozen acceptance
contract, not broad ecosystem compatibility or whole-game support.

## Output-only Agent reading

A fresh-context read-only Agent read only SEMANTICS.md and the supplied public
class outputs for resources, ammo, random, Activity, Wait, addresses and integer
conversions. It did not read implementation, original source/DLLs, tests, behavior
logs or prior answers, and did not execute methods. Verdict: **PASS; no issues**.

Its explanations correctly traced raw nextActivity traversal rather than property
filtering; null/cyclic behavior; Wait's exact external call and pre-decrement
returns including wraparound; random seed/default/state/range/float ordering;
original-array addresses after rebinding; I4/native signedness; raw denominator
bits and the possibility of 1.0 after rounding. It explicitly refused to invent
external getter behavior, infer dependency closure from inventory or treat
local CLI F precision as universal. This is one bounded reading exercise, not a
formal proof or model benchmark.

Artifact: session-managed `reviews/delivery-output-reading.md`, run
`b30f5f1e-fcd5-486e-8c03-4b167b3dadd6`. Inputs are preserved under ignored
`report/openra/delivery/`. The separate milestone B semantic review found no
scope-local diff-caused issues; parent validation, not reviewer claims, supplies
fresh execution evidence.

## Final gates

Final fmt, strict all-target clippy and locked Cargo Debug/Release suites passed.
The metadata checker and its five positive/negative tests passed. All affected
behavior gates passed from current public output. Owned comparisons per
Debug/Release configuration:

| Gate | Comparisons |
|---|---:|
| References | 522 |
| Arrays | 557 |
| Native storage | 197 |
| Integer conversions | 385 |
| Stable element addresses | 1,892 |
| UTF-16 literals/references | 124 |
| Exception allocation/throw | 259 |
| Float conversions/constants/division/exact calls | 2,010 |
| Core constructor | 22 |
| Controlled clock | 5 |
| Delegated constructor | 20 |
| External identities/exact calls | 48 |
| Integer differential regression | 735 |

Logs: ignored `report/openra/final-gates/`. Reproduce the distinct real-class gates:

```sh
python3 rust/acceptance/openra/test_inventory.py
python3 rust/acceptance/openra/inventory.py --check-cli
python3 rust/tests/reference-roundtrip.py --random-class --from-class
python3 rust/tests/reference-roundtrip.py --openra --from-class
python3 rust/tests/reference-roundtrip.py --activity --from-class
python3 rust/tests/reference-roundtrip.py --wait --from-class
```

## Remaining boundaries

Production Rust uses pinned cildec 0.1.0 and no .NET runtime. Optional execution
uses local 64-bit .NET 10.0.12/macOS arm64; native width and CLI F precision belong
to the target runtime. The Single/literal adapters provide local evidence, not
32-bit execution, all-runtime F/NaN/subnormal/interning equivalence, general
external resolution or a complete constructor/hostile-input verifier. Wait's
.NET 6 framework references intentionally use .NET 10 runtime unification.

General value-type/generic/byref/virtual/EH semantics, full dependency resolution,
source/project export, source-style loop recovery and full .NET method support
remain out of scope. Other precise class selections can legitimately contain
partial methods. The large upstream ILSpy-tests submodule and .NET 11 test suite
were not run. No game installation/execution, input reacquisition/rehashing,
source recount or unrelated root/C# experiment edits were performed during final
acceptance. GPL inputs/generated outputs remain ignored. All intended changes
are local commits; no push was authorized or performed.
