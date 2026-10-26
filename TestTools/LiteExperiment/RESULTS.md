# First experiment: syntax-recovery ablation

Historical first-run results. See [SEARCH_RESULTS.md](SEARCH_RESULTS.md) for the
subsequent 64-combination search and current experimental recommendations.

Date: 2026-10-25 (local). Upstream baseline: `c08aab375eed468cdcb07f035a24bf6c7ac026ab`.

## Environment and scope

- SDK **10.0.401**, runtime **10.0.12**, macOS 26.5.1 / arm64.
- Built `ICSharpCode.Decompiler` directly from this checkout, targeting netstandard2.0.
- Isolated harness and fixtures target net10.0; no root SDK downgrade.
- Same resolver on all profiles; the eight fixture references resolved identically
  into the .NET 10.0.12 runtime directory. No unresolved references were observed.
- Three profiles: Full (39 IL / 16 AST top-level passes), LiteSettings (39 / 16),
  LitePruned (35 / 13). See `ExperimentProfile.cs` for the exact switches and cuts.
- **No physical source removal or engine algorithm changes. No binary-size claim.**

## Verification

- Release and Debug builds succeeded with zero warnings/errors.
- 22 ordinary tests passed in each configuration. The explicit benchmark is skipped
  in ordinary runs and was run separately, successfully, in multiple processes.
- All seven class outputs matched between LiteSettings and LitePruned.
- Basic method outputs matched Full on six cases.
- The basic control-flow class was recompiled and executed for all three profiles:
  loops, switch, conditional values, catch/finally, checked overflow, unsigned shifts,
  left-to-right call evaluation, and retaining an old local value passed known-value checks.
- Async/iterator/closure/query/expression-tree/record outputs were inspected and
  checked for retained lowered implementation. They were **not** recompiled or
  behaviorally round-tripped.
- The final benchmark reports contained zero decompiler errors and zero matched
  `// IL_xxxx:` warning comments. This is not a proof of semantic equivalence.
- The upstream net11.0 test suite and large ILSpy-tests corpus were not run.

## Timing results

Two consecutive final runs, each with 3 warmups and 9 samples per target/profile.
Values below are the medians from the first run / repeated run, in milliseconds.
These are **warm session** results, including text generation. They are not cold-start
latencies or a controlled production benchmark; the spread between processes is material.

| Target | Full ms | LitePruned ms | Lite allocation change | Full/Lite output chars |
|---|---:|---:|---:|---:|
| Method: Sum | 1.028 / 0.805 | 0.985 / 0.775 | -3.0% | 153 / 153 |
| Method: ReadAsync | 1.009 / 0.966 | 0.774 / 0.717 | -35.7% | 126 / 424 |
| Method: Count | 0.927 / 0.994 | 0.420 / 0.519 | -63.6% | 148 / 226 |
| Method: Positive | 0.986 / 1.080 | 0.729 / 0.752 | about -41.9% | 179 / 283 |
| Class: ControlFlowSample | 7.582 / 5.660 | 7.285 / 7.788 | -3.2% | 1131 / 1131 |
| Class: AsyncSample | 2.159 / 1.919 | 5.216 / 5.274 | +136.9% | 191 / 2061 |
| Class: IteratorSample | 2.655 / 1.883 | 7.442 / 6.891 | about +204% | 218 / 2141 |
| Class: RecordSample | 2.728 / 2.207 | 9.629 / 7.833 | about +210% | 73 / 3079 |

Output character counts exclude the C#-like banner. Allocation means process-wide
managed bytes allocated during the call, not retained memory or peak RSS.

### Interpretation

1. **Disabling recovery does not imply universally faster decompilation.**
   Full compresses an async/iterator implementation into one source method and hides
   generated members. Lite prints the complete state machine; processing and output
   grow. Record output grows from 73 to 3079 characters. These are unequal workloads.
2. **Single-method extraction can become cheaper.** Async kickoff and iterator
   factory methods no longer pull in and reconstruct their state-machine bodies.
   This is useful for IL-level inspection, but it is also less work, not an
   algorithmic speedup over equivalent output.
3. **Removing already-disabled top-level passes adds no demonstrated robust gain.**
   LiteSettings and LitePruned produce identical fixture output and nearly identical
   warm allocation. Individual timings fluctuate in either direction. Most removed
   passes already exit immediately when their feature switch is false.
4. **Reusing sessions matters.** Fresh-session measurements are reported separately
   and include loading/resolution/type-system construction/disposal. Their OS caches
   and JIT are still warm, so they must not be labeled cold-start results.
5. **The C#-like contract is necessary.** Output retains names such as
   `<ReadAsync>d__0`, `<Clone>$`, and `<Id>k__BackingField`. The lowered record
   constructor also contains `base..ctor()`. The current experiment does not repair
   these into compilable C#, and a zero error count does not make them valid C#.

## Artifacts

Reproducible commands are in [README.md](README.md).

Generated, gitignored files from the two final runs:

- `report/final/summary.md` and `report/repeat/summary.md`;
- `report/{final,repeat}/measurements.json` (individual samples and environment);
- `report/{final,repeat}/{Full,LiteSettings,LitePruned}/*.cs.txt`;
- `report/{final,repeat}/IL/*.il.txt`;
- `../../test-results/lite-*/*.trx`.

Representative outputs are retained in `examples/`, with explicit C#-like labels.

Input fixture SHA-256:
`65A2A32F90605F588936381DD11D0036E953B250598DFDD7FDA12D023FEC6B45`

Engine SHA-256:
`EA52DCB60216A1F3C2ABCFA3BC244907BAB6858F70BB803D2651F97C51D44CED`

## Recommended next cut

Keep this harness as the comparison point. Do not start with formatter or Stepper
removal. Next isolate a **source-level removal of project/PDB/XML export support**, or
add an **ILAst-direct writer** as a fourth profile. Measure those changes separately
from resolver replacement. A larger real-assembly corpus is needed before declaring
any general throughput gain or committing to a minimal transform list.
