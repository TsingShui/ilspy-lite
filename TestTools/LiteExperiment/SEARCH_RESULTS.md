# Bounded transform-combination search

## Decision

Use **mask 27** (`0x1B`) as the experimental **whole-class** recommendation.
Use **mask 59** (`0x3B`) for **warm single-method extraction** when qualified type
names are acceptable. These are corpus-specific recommendations, not a global
optimum or a replacement for upstream defaults.

The constants are `SearchConfiguration.RecommendedClass` and
`SearchConfiguration.RecommendedMethod`. Engine source is still unmodified.
There is **no physical source/binary-size reduction** in this experiment.

Most of the repeatable gain requires replacing heuristic variable naming with
`__il_vN` names, not just removing transforms. Pure pass/settings cuts had only
small gains, close to the noise floor. Keep assignment prettification: removing
more passes did not monotonically improve performance.

## Search space and common contract

All 64 masks start from LitePruned. All additionally disable AutomaticProperties,
GetterOnlyAutomaticProperties, AutomaticEvents and FieldKeyword **before** type
system construction. Otherwise removing PatternStatementTransform can leave
hidden backing fields without reconstructed properties. The reference mask 0 is
therefore **not exactly the first experiment's LitePruned profile**. No comparison
below mixes Full with lowered output or changes reference resolution.

| Bit | Dimension | Operation |
|---:|---|---|
| 1 | SimpleNames | Replace AssignVariableNames with SimpleVariableNames |
| 2 | NoAstPatterns | Remove PatternStatementTransform |
| 4 | NoAssignmentSugar | Remove PrettifyAssignments; IntroduceIncrementAndDecrement=false |
| 8 | NoLateCleanup | Remove ReduceNestingTransform and RemoveRedundantReturn |
| 16 | NoSroa | Remove TransformDisplayClassUsage |
| 32 | QualifiedNames | UsingDeclarations=false; retain the qualification transform |

Mask **27 = 1+2+8+16**. Mask **59 = 27+32**. Both run **32 top-level IL / 12 AST**
passes, versus **35 / 13** for mask 0 (Full: 39 / 16). AssignVariableNames is
**replaced**, not omitted. Nested statement/block passes and remaining order are
unchanged. Stack reading, core control flow, type resolution, checked arithmetic,
declarations, exception handling and accurate evaluation order are retained.

SimpleVariableNames reserves existing variable and declaring-type member names,
keeps parameters and special temporaries, and assigns unique flat local names.
It falls back to upstream naming for nested functions. It does not reproduce
upstream naming heuristics or reuse names for split local slots. This is a
readability tradeoff within a **C#-like**, not universally recompilable, contract.

## Method

Environment: .NET SDK 10.0.401, runtime 10.0.12, macOS arm64; engine baseline
`c08aab375eed468cdcb07f035a24bf6c7ac026ab`. Root SDK selection unchanged.

1. Gate every mask with the same 9-class member-inventory check, lowered-state-
   machine checks, and two compilable execution fixtures.
2. Three independent 64-mask screen processes (7 rounds each). Deterministically
   shuffled mask order, alternating candidate/baseline measurement order, two
   warmups per pair, repeated batches (2 class batches / 5 method batches per
   timing). Rank class and method batches separately by paired timing ratio.
3. Per-target validation: four warmups and eleven samples, rotating profile order,
   both warm and fresh-session paths. First 9 masks, then 6 finalists. Framework
   classes were **not used in the 64-mask screening**, but were used when choosing
   the final recommendation; they are not an untouched final test set after that.
4. Third validation with the default tiered JIT (masks 0/27/59). Final confirmation
   adds pure-cut controls (0/27/48/56/59). Main screens and controlled validations
   set DOTNET_TieredCompilation=0 to reduce tier-transition noise.

Screen targets: 9 fixture classes and 8 methods. Framework targets: complete
MemoryStream, StringBuilder, List<T>, Dictionary<TKey,TValue> types from installed
System.Private.CoreLib. There were 5,376 candidate/baseline batch timings across
three screens, and 10,626 per-target validation timings across four validation
processes. Batches execute decompilation multiple times; these counts are not
numbers of distinct methods.

Assertions, output/IL writes and member inventories are outside timed regions.
Fresh-session includes load, resolve, construction and disposal (plus diagnostics
snapshot), **not cold OS caches or process startup**. Allocation uses process-wide
GC.GetTotalAllocatedBytes: not RSS, retained memory, or peak heap.

## Results

The table sums **per-target median milliseconds**, representing one visit to each
target in the named workload; it is not a median of whole-batch sums. Percentages
are **time reductions**, not throughput increases. Controlled A/B are separate
processes with tiered compilation disabled. Default JIT numbers should only be
compared within their own process.

| Workload / recommendation | Controlled A: base -> candidate ms | Controlled B | Default JIT | Warm allocation reduction (A/B) |
|---|---:|---:|---:|---:|
| 9 classes / 27 | 20.551 -> 16.934 (-17.6%) | 20.694 -> 16.712 (-19.2%) | 42.234 -> 35.115 (-16.9%) | 8.5% |
| 8 methods / 59 | 5.364 -> 4.439 (-17.2%) | 5.331 -> 4.479 (-16.0%) | 4.045 -> 3.378 (-16.5%) | 8.3% |
| 4 framework classes / 27 | 235.604 -> 205.785 (-12.7%) | 235.833 -> 208.206 (-11.7%) | 180.834 -> 161.818 (-10.5%) | 7.0% |

Final confirmation: mask 27 reduced warm class time **17.0%**, framework class time
**12.6%**; mask 59 reduced method time **13.3%**. These differences illustrate
measurement variability. Mask 59 was faster than 27 for methods in all four
per-target validation processes; mask 27 was faster than 59 for framework classes
in all four. The small-mask rankings changed between screens: there is no evidence
for a uniquely best universal combination.

### Pure cuts versus naming replacement

In the final confirmation:

| Mask | Description | Warm class time reduction | Warm method time reduction | Framework class time reduction |
|---:|---|---:|---:|---:|
| 48 | NoSroa + QualifiedNames | 2.5% | 0.9% | 1.7% |
| 56 | 48 + NoLateCleanup | 3.1% | 4.3% | 2.9% |
| 27 | Naming replacement + selected cuts | 17.0% | 10.4% | 12.6% |
| 59 | 27 + QualifiedNames | 7.6% | 13.3% | 11.8% |

Mask 1 (naming only) reduced warm class time about 13% and framework time about
8-9% in the first two validations. Do not attribute the entire 10-19% improvement
to pass removal. Mask 63 (all switches, including removal of assignment sugar)
was not best: warm method time was only about 0-1% lower than baseline in those
validations, with roughly **5.5% more** allocations. The screen even measured it
slower than baseline on method batches.

### New sessions limit the benefit

Framework fresh-session totals improved only **1.0-1.7%** for mask 27 in controlled
runs, and **2.1%** with the default JIT. Construction/resolution dominates those
measurements. Method fresh-session results were variable; default-JIT mask 59 was
**2.1% slower**, despite faster warm decompilation. No general fresh-session speedup
is claimed. Reuse a session when comparing transform throughput.

### Output is not being truncated

Aggregate output characters, excluding banners:

- 9 classes: mask 0 **12,706**, mask 27 **13,197** (+3.9%).
- 8 methods: mask 0 **2,260**, mask 59 **2,964** (+31.2%, includes qualified names).
- Framework classes: mask 0 **140,008**, mask 27 **143,015** (+2.1%).

Generated members/state machines remain visible. Naming is more mechanical and
some loop/nesting cleanup is lost. See examples/Sum.search-27.cs.txt and
examples/ReadAsync.search-59.cs.txt. A kickoff method is still not the complete
async operation; its class output retains MoveNext.

## Verification and limits

- Release and Debug: **90 passed, 3 explicit benchmarks skipped**; builds zero
  warnings/errors after a narrowly documented fixture analyzer suppression.
- Every one of the 64 masks passed in both configurations. Ordinary execution tests
  compile/decompile/recompile ControlFlowSample and StressSample, testing loops,
  integer/string switch, nullable values, exception filters, catch/finally, monitor
  and using lowering, multidimensional foreach, checked overflow, unsigned shift,
  ref/array and argument evaluation order, old local values, name collisions.
- Naming, qualification, explicit-addition and backing-field tests were observed
  failing before their corresponding implementation. Timing has no speed assertion.
- Every validation enforces equal declaration-token inventories and resolved
  reference paths against mask 0, deterministic warm/fresh output, zero missing
  references/errors, and matching IL-offset warning counts. All four validations
  recorded **zero errors, missing references and matched warnings**.
- Token inventory equality does not prove complete IL preservation or semantic
  equivalence; it cannot detect a changed body. High-level and framework classes
  were **not behaviorally round-tripped**. Full upstream .NET 11 tests, malformed
  inputs, obfuscation and a broad third-party corpus are not covered.
- No confidence intervals, CPU affinity, isolated machine or GC-mode sweep. Treat
  close rankings and low-single-digit gains as inconclusive. Default JIT remains
  sensitive to tiering during these short tests.

## Artifacts

Reproduction commands are in README.md. Ignored raw artifacts:

- report/search-screen1, search-screen2, search-screen-final
- report/search-validate1, search-validate2, search-default-jit, search-confirm
- Each contains measurements.json (samples, settings, pass lists, reference paths,
  input/engine/harness hashes), summary.md, outputs/, IL/.
- ../../test-results/lite-search-*/*.trx

The first two screen reports have candidate character counts in paired-baseline
rows and candidate allocation in ratio rows; use the candidate timing rows and
paired time ratios only. The final screen fixes these report-only fields. No
measurement was discarded or adjusted. The final confirmation used the final
source; earlier report hashes identify earlier harness builds.

Recommended next experiment: keep mask 27 as an opt-in baseline, measure larger
real-world assemblies with session reuse, then profile phase costs before touching
resolver/type-system construction. Direct ILAst output remains a separate design.
