# Low-semantic-compression experiment (.NET 10)

This is an isolated, executable experiment against the **source checkout** of
`ICSharpCode.Decompiler`, not the installed ilspycmd or a decompiler NuGet package.
It retains the existing ILAst, C# AST builders and text writer. No engine source is
deleted or modified. The root SDK selection and upstream test targets are unchanged.

The first experiment is in [RESULTS.md](RESULTS.md). The subsequent 64-combination
search and recommendations are in [SEARCH_RESULTS.md](SEARCH_RESULTS.md).

## Profiles

- **Full**: default syntax recovery, with PDB names/XML documentation disabled and
  accurate evaluation-order handling enabled (the same common settings as Lite).
- **LiteSettings**: disables async/iterator, lambda/closure/local-function,
  expression-tree, query/extension-method and record/primary-constructor recovery
  **before** building the type system. Generated implementations stay visible.
- **LitePruned**: identical settings, plus removal of four top-level IL passes
  (`YieldReturnDecompiler`, `AsyncAwaitDecompiler`, `DelegateConstruction`,
  `LocalFunctionDecompiler`) and three AST passes (`IntroduceQueryExpressions`,
  `CombineQueryExpressions`, `AddXmlDocumentationTransform`). Remaining order and
  nested statement/block pipelines are untouched.

`ExpressionTrees=false` disables its nested transform through its existing setting;
that nested pass is not physically removed. `RecordClasses/RecordStructs=false`
disable record recovery, not the compilation of `RecordDecompiler`. This is an
ablation experiment, **not** a measured binary/source-size reduction.

All profiles use the same instrumented `UniversalAssemblyResolver`, main module
and runtime reference directory. The report records resolved references and checks
that each target resolves identically across profiles. This does **not** benchmark
a replacement resolver. Sessions are single-threaded and own their loaded PE files.

## Build and test

Requires .NET SDK 10.0.401 (or a later .NET 10 feature band), the .NET 10 runtime,
and PowerShell 7 on PATH. No .NET 11 SDK is required.

Run from this directory: SDK selection uses the working directory, including when
building project references outside this directory. Build only this solution.
The root scripts accept an optional `-Solution`; their default remains ILSpy.slnx.

```sh
cd TestTools/LiteExperiment
pwsh -NoProfile -File ../../restore.ps1 -Solution LiteExperiment.slnx
pwsh -NoProfile -File ../../build.ps1 -Solution LiteExperiment.slnx -Configuration Release --no-restore
dotnet test --project LiteExperiment.csproj -c Release --no-build --report-trx \
  --results-directory ../../test-results/lite
```

The three explicit benchmarks are intentionally skipped by the ordinary test run.
The suite now has 90 ordinary tests, including all 64 search masks. Select
it directly (do not run concurrently with correctness tests or other benchmarks):

```sh
dotnet test --project LiteExperiment.csproj -c Release --no-build --report-trx \
  --results-directory ../../test-results/lite-benchmark \
  --filter 'FullyQualifiedName=LiteExperiment.Benchmark.Run'
```

Reports go to `report/lite/`. Set `ILSPY_LITE_REPORT` to select another directory.
Each report contains:

- `Full/`, `LiteSettings/`, `LitePruned/`: 7 class outputs and 6 method outputs;
- `IL/`: matching IL disassembly with instruction offsets and metadata tokens;
- `summary.md`: timing/allocation/output-size table;
- `measurements.json`: individual timing samples, medians/ranges, settings,
  top-level pass lists, reference paths, runtime and input/engine hashes.

Generated output uses `.cs.txt`, not compilable source files. Lite output has an
explicit C#-like banner. Identifiers such as `<ReadAsync>d__0`, `<Clone>$`, and
low-level constructor expressions can remain invalid C#. Do not execute or compile
arbitrary output as trusted code.

## Combination search

The search keeps the original three profiles unchanged. Its common baseline is
LitePruned plus disabled automatic property/event/field-keyword recovery, so all
candidates retain the same backing fields. Six independent bits give 64 masks;
see SearchConfiguration.cs and SEARCH_RESULTS.md for exact settings and caveats.

Recommended opt-in usage (not a production API guarantee):

```csharp
using var session = new ExperimentSession(assemblyPath, SearchConfiguration.RecommendedClass);
string text = session.DecompileType(type); // C#-like, may not compile
```

Mask 27 is the class recommendation; mask 59 additionally uses fully qualified
names and performed better on warm method extraction. Both replace heuristic
variable naming with flat names and keep assignment prettification. No core
source has been deleted. Small differences between finalists are not robust
proof of a unique optimum.

Run the search serially, in Release, outside other benchmark/build activity:

```sh
# All 64 masks, correctness gate followed by paired warm batch timings.
DOTNET_TieredCompilation=0 ILSPY_SEARCH_REPORT="$PWD/report/search-screen" \
  dotnet test --project LiteExperiment.csproj -c Release --no-build --report-trx \
  --results-directory ../../test-results/lite-search-screen \
  --filter 'FullyQualifiedName=LiteExperiment.CombinationSearch.Screen'

# Selected masks: individual targets, warm/fresh sessions, plus framework classes.
DOTNET_TieredCompilation=0 ILSPY_SEARCH_MASKS=0,27,48,56,59 \
  ILSPY_SEARCH_REPORT="$PWD/report/search-validate" \
  dotnet test --project LiteExperiment.csproj -c Release --no-build --report-trx \
  --results-directory ../../test-results/lite-search-validate \
  --filter 'FullyQualifiedName=LiteExperiment.CombinationSearch.Validate'
```

Repeat in separate processes. Unset DOTNET_TieredCompilation for a default-JIT
cross-check. ILSPY_SEARCH_MASKS is a comma-separated list from 0 to 63 (baseline 0
is always included); default validation masks are 0,1,3,7,15,31,63. Reports include
raw samples, configurations, assembly hashes, reference evidence, output and IL.
Screen ratio rows use MedianMs as a dimensionless candidate/baseline ratio and
have zero allocation/character fields; other rows use milliseconds and bytes.

## What the checks establish

The tests exercise the public class/method decompilation interfaces:

- lowered state machines, closures, expression-tree calls and record members remain
  visible when their high-level reconstruction is disabled;
- settings-only and pruned class output match exactly on the fixture set;
- basic control-flow method output matches Full;
- a basic control-flow class recompiles with Roslyn and produces known results for
  loops, switch, conditional values, exception/finally handling, unsigned shifts,
  checked overflow, argument evaluation order and preservation of an old local value.

The high-level fixtures are **not** round-tripped. No claim of universal semantic
correctness, malformed-input robustness, or complete preservation of unreachable
IL is made. Other ILSpy optimizations remain enabled.

The first async test and the five lowered-syntax tests were observed failing
against the default profile before their respective settings were implemented.
TRX files are retained under the repository's ignored `test-results/` directory.

## Measurement limitations

The benchmark runs three warmups and nine samples per target/profile, rotating
profile order. Time includes text generation, but excludes report I/O/assertions.
Fresh-session samples additionally include construction, resolution and disposal;
they **do not** represent cold OS caches, process startup or first-time JIT.
Managed allocation is process-wide `GC.GetTotalAllocatedBytes`, not retained heap,
peak RSS or a per-thread allocation trace. There are no speed assertions.

Class workloads are not equivalent: Lite prints state-machine and generated members
that Full compresses/hides. Conversely, a Lite async kickoff method omits the
state-machine body that Full incorporates into its reconstructed async method.
Always inspect output size and content alongside timing.

This small synthetic corpus is a baseline for subsequent cuts, not a production
throughput claim. Keep reference resolution fixed while studying transforms; study
resolver changes and direct ILAst printing as separate experiments.
