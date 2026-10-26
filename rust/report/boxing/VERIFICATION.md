# Boxing milestone verification (core scalars + local enums)

Verification run of the current working tree: build, full corpus scan, deltas against
the two saved baselines, behaviour gates with mutation control, and ILSpy reference
dumps for the review pairs. No source file was edited; no commit was made; no
third-party assembly was executed.

- Repo: `/Users/bytedance/zhome/ilspy-lite`, crate `rust/`, shared `rust/target`.
- Tree under test: `HEAD = 262f028508c695ef6dfe3c000860955bade6ec6e` plus the
  uncommitted working-tree changes already present (`rust/src/*`, `rust/tests/*`,
  `rust/acceptance/corpus/*`).
- Scope inventory (unchanged): `report/core-type-roots/scope-policy/implementation-inventory.json`.
- Toolchain: `cargo` release + debug profile; `dotnet` **10.0.401** from
  `~/.dotnet10` (no .NET 11 SDK is installed, see "Build notes" below).

## 1. Build

```
cd rust && cargo build --release      # up to date, 0.12 s
cd rust && cargo build                # up to date (debug profile, needed by the drivers)
```

Both profiles are current with the working tree; `target/release/ilspy-lite` is the
binary the scan was taken with (16 bytes-stamp 1,632,000, recorded in the baseline).

## 2. Full corpus scan

```
cd rust
python3 acceptance/corpus/corpus.py baseline \
  --inventory report/core-type-roots/scope-policy/implementation-inventory.json \
  --cli target/release/ilspy-lite \
  --output report/boxing/corpus-scan-enum \
  --references declared
```

Exit 0. Summary line as printed:

```
{"summary": {"managed_images": 614, "body_images": 319, "bodies": 770748,
 "counts": {"Partial": 473616, "Decompiled": 297132},
 "incomplete_images": 0, "passing_body_images": 33}, "elapsed_ms": 41454}
```

| quantity | value |
|---|---:|
| managed images | 614 |
| body-bearing images | 319 |
| bodies | 770,748 |
| Decompiled | **297,132** |
| Partial | **473,616** |
| unrecovered rate | 61.44888% |
| incomplete images | **0** |
| body images within 1% | 33/319 |
| scan wall time | 41.5 s (`elapsed_ms` 41454; 43.6 s including interpreter start) |

The expected shape (614 / 319 / 770,748 / 0 incomplete) matches exactly, so the scan
is comparable to the saved baselines.

## 3. Deltas

```
python3 report/compare-recovery-scans.py \
  report/boxing/corpus-scan/baseline.json \
  report/boxing/corpus-scan-enum/baseline.json \
  report/boxing/corpus-scan-enum/delta-vs-boxing.json

python3 report/compare-recovery-scans.py \
  report/constructed-members/corpus-scan-demoted/baseline.json \
  report/boxing/corpus-scan-enum/baseline.json \
  report/boxing/corpus-scan-enum/delta-vs-constructed.json
```

Both comparisons assert identical image sets, body counts and origins, so the
differences below are admission changes only, on the same 770,748 bodies.

### 3a. vs `report/boxing/corpus-scan` (boxing core scalars)

| quantity | before | after | delta |
|---|---:|---:|---:|
| Decompiled | 296,978 | 297,132 | +154 |
| Partial | 473,770 | 473,616 | -154 |
| bodies | 770,748 | 770,748 | +0 |
| body images | 319 | 319 | +0 |
| incomplete scans | 0 | 0 | +0 |
| body images within 1% | 33 | 33 | +0 |
| unrecovered rate | 61.46886% | 61.44888% | -0.01998 pp |

- newly recovered: **154**
- images that gained: **41**
- images that lost a method: **0** (regression list empty)
- every one of the 154 gained methods previously carried the first blocker
  `box/unbox needs an admitted core scalar` (the pre-local-enum text), so the whole
  delta is the local-enum boxing admission, not a side effect of something else.

| # | id | assembly | gains | lost | first origin |
|---:|---:|---|---:|---:|---|
| 1 | 626 | System | +25 | 0 | `mcs/5.23/lib/mono/4.5/System.dll` |
| 2 | 627 | mcs | +15 | 0 | `mcs/5.23/lib/mono/4.5/mcs.exe` |
| 3 | 130 | Microsoft.CodeAnalysis.CSharp | +10 | 0 | `Newtonsoft.Json-net45/Microsoft.CodeAnalysis.CSharp.dll` |
| 4 | 618 | System | +10 | 0 | `mcs/2.6.4/lib/mono/2.0/System.dll` |
| 5 | 798 | Microsoft.CodeAnalysis.CSharp | +10 | 0 | `nuget/Microsoft.Net.Compilers-2.10.0.nupkg!tools/Microsoft.CodeAnalysis.CSharp.dll` |
| 6 | 4 | Microsoft.CodeAnalysis.CSharp | +7 | 0 | `ICSharpCode.Decompiler/Microsoft.CodeAnalysis.CSharp.dll` |
| 7 | 58 | ICSharpCode.NRefactory.Tests | +7 | 0 | `NRefactory/ICSharpCode.NRefactory.Tests.dll` |
| 8 | 895 | Microsoft.CodeAnalysis.CSharp | +7 | 0 | `nuget/...Net.Compilers.Toolset-3.11.0.nupkg!tasks/net472/Microsoft.CodeAnalysis.CSharp.dll` |
| 9 | 994 | Microsoft.CodeAnalysis.CSharp | +7 | 0 | `nuget/...Net.Compilers.Toolset-3.11.0.nupkg!tasks/netcoreapp3.1/bincore/Microsoft.CodeAnalysis.CSharp.dll` |
| 10 | 3 | ICSharpCode.Decompiler | +3 | 0 | `ICSharpCode.Decompiler/ICSharpCode.Decompiler.dll` |
| 11 | 76 | nunit.framework.tests | +3 | 0 | `NUnit/net-2.0/nunit.framework.tests.dll` |
| 12 | 84 | nunit.framework.tests | +3 | 0 | `NUnit/net-3.5/nunit.framework.tests.dll` |
| 13 | 97 | nunit.framework.tests | +3 | 0 | `NUnit/net-4.0/nunit.framework.tests.dll` |
| 14 | 105 | nunit.framework.tests | +3 | 0 | `NUnit/net-4.5/nunit.framework.tests.dll` |
| 15 | 119 | nunit.framework.tests | +3 | 0 | `NUnit/portable/nunit.framework.tests.dll` |

All 41 gained images (id, assembly, gained):
626 System +25, 627 mcs +15, 130 Microsoft.CodeAnalysis.CSharp +10, 618 System +10,
798 Microsoft.CodeAnalysis.CSharp +10, 4 Microsoft.CodeAnalysis.CSharp +7,
58 ICSharpCode.NRefactory.Tests +7, 895 Microsoft.CodeAnalysis.CSharp +7,
994 Microsoft.CodeAnalysis.CSharp +7, 3 ICSharpCode.Decompiler +3,
76 nunit.framework.tests +3, 84 nunit.framework.tests +3, 97 nunit.framework.tests +3,
105 nunit.framework.tests +3, 119 nunit.framework.tests +3, 126 BenchmarkDotNet.Core +3,
55 ICSharpCode.NRefactory.CSharp +2, 447 System.IO.FileSystem.AccessControl +2,
483 System.Private.CoreLib +2, 617 System.Xml +2, 619 gmcs +2, 628 mscorlib +2,
785 Microsoft.CodeAnalysis.VisualBasic +2, 801 Microsoft.CodeAnalysis.VisualBasic +2,
894 Microsoft.CodeAnalysis +2, 993 Microsoft.CodeAnalysis +2, 67 nunit.core.interfaces +1,
71 nunit.framework +1, 83 nunit.framework +1, 113 nunit.engine +1,
114 nunit.engine.tests +1, 131 Microsoft.CodeAnalysis +1, 134 Newtonsoft.Json.Tests +1,
157 nunit.framework +1, 158 Newtonsoft.Json.Tests +1, 487 System.Private.Xml +1,
514 System.Security.AccessControl +1, 620 mscorlib +1, 625 System.Xml +1,
796 Microsoft.CodeAnalysis +1, 898 Microsoft.CodeAnalysis.VisualBasic +1.

### 3b. vs `report/constructed-members/corpus-scan-demoted` (milestone start)

| quantity | before | after | delta |
|---|---:|---:|---:|
| Decompiled | 296,145 | 297,132 | +987 |
| Partial | 474,603 | 473,616 | -987 |
| bodies | 770,748 | 770,748 | +0 |
| body images | 319 | 319 | +0 |
| incomplete scans | 0 | 0 | +0 |
| body images within 1% | 33 | 33 | +0 |
| unrecovered rate | 61.57694% | 61.44888% | -0.12806 pp |

- newly recovered: **987**
- images that gained: **95**
- images that lost a method: **0**
- previous first blockers of the 987 gained methods: `unsupported opcode box` 785,
  `unsupported opcode unbox.any` 202. No other previous reason appears, so the whole
  milestone delta is the two new opcodes (833 of it was already recorded for core
  scalars; the remaining 154 is the local-enum admission reported in 3a).

| # | id | assembly | gains | lost | first origin |
|---:|---:|---|---:|---:|---|
| 1 | 458 | System.Linq.Expressions | +121 | 0 | `dotnet/netcore-2.2/System.Linq.Expressions.dll` |
| 2 | 626 | System | +64 | 0 | `mcs/5.23/lib/mono/4.5/System.dll` |
| 3 | 618 | System | +48 | 0 | `mcs/2.6.4/lib/mono/2.0/System.dll` |
| 4 | 627 | mcs | +33 | 0 | `mcs/5.23/lib/mono/4.5/mcs.exe` |
| 5 | 2 | ICSharpCode.Decompiler.Tests | +26 | 0 | `ICSharpCode.Decompiler/ICSharpCode.Decompiler.Tests.exe` |
| 6 | 76 | nunit.framework.tests | +26 | 0 | `NUnit/net-2.0/nunit.framework.tests.dll` |
| 7 | 84 | nunit.framework.tests | +26 | 0 | `NUnit/net-3.5/nunit.framework.tests.dll` |
| 8 | 97 | nunit.framework.tests | +26 | 0 | `NUnit/net-4.0/nunit.framework.tests.dll` |
| 9 | 105 | nunit.framework.tests | +26 | 0 | `NUnit/net-4.5/nunit.framework.tests.dll` |
| 10 | 119 | nunit.framework.tests | +25 | 0 | `NUnit/portable/nunit.framework.tests.dll` |
| 11 | 55 | ICSharpCode.NRefactory.CSharp | +23 | 0 | `NRefactory/ICSharpCode.NRefactory.CSharp.dll` |
| 12 | 625 | System.Xml | +20 | 0 | `mcs/5.23/lib/mono/4.5/System.Xml.dll` |
| 13 | 71 | nunit.framework | +19 | 0 | `NUnit/addins/tests/nunit.framework.dll` |
| 14 | 83 | nunit.framework | +19 | 0 | `NUnit/net-3.5/nunit.framework.dll` |
| 15 | 157 | nunit.framework | +19 | 0 | `Newtonsoft.Json-net45/nunit.framework.dll` |

All 95 gained images (id, assembly, gained):
458 System.Linq.Expressions +121, 626 System +64, 618 System +48, 627 mcs +33,
2 ICSharpCode.Decompiler.Tests +26, 76 nunit.framework.tests +26, 84 nunit.framework.tests +26,
97 nunit.framework.tests +26, 105 nunit.framework.tests +26, 119 nunit.framework.tests +25,
55 ICSharpCode.NRefactory.CSharp +23, 625 System.Xml +20, 71 nunit.framework +19,
83 nunit.framework +19, 157 nunit.framework +19, 487 System.Private.Xml +19,
96 nunit.framework +18, 104 nunit.framework +18, 3 ICSharpCode.Decompiler +17,
118 nunit.framework +17, 619 gmcs +17, 405 Microsoft.VisualBasic +15,
130 Microsoft.CodeAnalysis.CSharp +13, 131 Microsoft.CodeAnalysis +13,
425 System.Data.Common +13, 796 Microsoft.CodeAnalysis +13,
798 Microsoft.CodeAnalysis.CSharp +13, 894 Microsoft.CodeAnalysis +13,
993 Microsoft.CodeAnalysis +13, 11 nunit.framework +12, 53 nunit.framework +12,
58 ICSharpCode.NRefactory.Tests +12, 73 nunit.framework +12, 473 System.Net.Sockets +11,
484 System.Private.DataContractSerialization +10, 617 System.Xml +10,
4 Microsoft.CodeAnalysis.CSharp +9, 624 System.Core +9, 98 nunit.testdata +7,
106 nunit.testdata +7, 126 BenchmarkDotNet.Core +7, 134 Newtonsoft.Json.Tests +7,
158 Newtonsoft.Json.Tests +7, 895 Microsoft.CodeAnalysis.CSharp +7,
994 Microsoft.CodeAnalysis.CSharp +7, 120 nunit.testdata +6, 5 Microsoft.CodeAnalysis +5,
7 Newtonsoft.Json +5, 77 nunit.testdata +5, 85 nunit.testdata +5, 135 Newtonsoft.Json +5,
159 Newtonsoft.Json +5, 420 System.ComponentModel.TypeConverter +5,
60 ICSharpCode.NRefactory +4, 113 nunit.engine +4, 801 Microsoft.CodeAnalysis.VisualBasic +4,
67 nunit.core.interfaces +3, 785 Microsoft.CodeAnalysis.VisualBasic +3, 0 FSharp.Core +2,
61 IKVM.Reflection +2, 129 FSharp.Core +2, 447 System.IO.FileSystem.AccessControl +2,
464 System.Net.HttpListener +2, 465 System.Net.Mail +2, 483 System.Private.CoreLib +2,
493 System.Reflection.Metadata +2, 531 System.Text.RegularExpressions +2, 628 mscorlib +2,
898 Microsoft.CodeAnalysis.VisualBasic +2, 1041 OpenRA.Game +2, 6 Mono.Cecil +1,
13 Mono.Cecil +1, 28 test +1, 54 ICSharpCode.NRefactory.CSharp.Refactoring +1,
59 ICSharpCode.NRefactory.Xml +1, 62 Mono.Cecil +1, 63 Mono.Cecil +1, 79 nunitlite +1,
86 nunitlite +1, 100 nunitlite +1, 108 nunitlite +1, 114 nunit.engine.tests +1,
116 nunit3-console.tests +1, 145 System.Reflection.Metadata +1,
150 System.Text.Encoding.CodePages +1, 461 System.Linq +1, 463 System.Net.Http +1,
469 System.Net.Primitives +1, 514 System.Security.AccessControl +1, 616 System.Core +1,
620 mscorlib +1, 891 System.Reflection.Metadata +1, 907 System.Reflection.Metadata +1,
912 System.Text.Encoding.CodePages +1, 999 System.Reflection.Metadata +1.

### 3c. Top 5 first-blocker buckets in the new scan (body images)

Grouped by the reason prefix before the first `;` (stable text kept whole when there is
none), summed over the 319 body images; total equals the 473,616 Partial bodies.

| bucket | bodies | share of Partial |
|---|---:|---:|
| constructed value type needs a local definition | 39,281 | 8.29% |
| exception/data sections need IL fallback | 39,234 | 8.28% |
| enum storage requires an exact local definition | 35,218 | 7.44% |
| allocation owner must be a concrete class | 28,685 | 6.06% |
| external allocation needs dependency facts | 24,893 | 5.26% |

Next buckets, in order: interface declaring type needs IL fallback 24,590;
constructed member parent must be a concrete class 20,274; incompatible
value/receiver types 19,990; byref/modified parameter needs IL fallback 19,297;
generic/indirect method reference needs IL fallback 16,567.

Boxing-specific residue still standing as a *first* blocker:

| reason | bodies | images |
|---|---:|---:|
| `box/unbox needs an admitted core scalar or local enum` | 4,358 | 119 |
| `box operand type differs from its declared type` | 896 | 79 |
| `unsupported opcode unbox` | 7 | 2 |

## 4. Behaviour gates

Drivers (`rust/tests/*-roundtrip.py`, owned fixtures only, third-party code never
executed). Note on the flag: `--cli` selects **one** `ilspy-lite` binary, and the
driver then exercises **both** fixture configurations (`release` and `debug`) with it.
To cover both profiles as requested, each driver was run once per binary.

```
cd rust
python3 tests/generic-declaring-roundtrip.py --mutation-control --cli target/release/ilspy-lite
python3 tests/generic-declaring-roundtrip.py --mutation-control --cli target/debug/ilspy-lite
python3 tests/enum-roundtrip.py --mutation-control --cli target/release/ilspy-lite
python3 tests/enum-roundtrip.py --mutation-control --cli target/debug/ilspy-lite
```

Both drivers do have `--mutation-control`; neither was run without it.

### 4a. `generic-declaring-roundtrip.py` (the slice's gate)

| CLI binary | fixture config | comparisons | recovered methods | skipped | mutation |
|---|---|---:|---:|---|---|
| release | release | 18 | 11 | `[]` | wrong-field rejected |
| release | debug | 18 | 11 | `[]` | wrong-field rejected |
| debug | release | 18 | 11 | `[]` | wrong-field rejected |
| debug | debug | 18 | 11 | `[]` | wrong-field rejected |

- Behavior line: `Generic declaring behavior: 18 comparisons, 11 recovered methods;
  open self receivers, substituting field types and identity results match.`
- Mutant verdict: `Recovered generic-declaring member differs: Field expected=3 actual=9`
  (nonzero exit in every configuration). No mutant survived.
- The counts moved from BOXING.md (17 comparisons / 10 recovered methods) because
  `Ops.BoxEnum` is now recovered. Current `Ops` admission in both fixture configs:
  `CallScore, MakeHolder, MakeDerived, Identity, Field, SelfCall, UnsupportedContent,
  UnsupportedStruct, BoxInt, BoxEnum, UnboxInt` = Decompiled (11);
  `BoxStruct` and `UnsupportedBoxGeneric<1>` = Partial with
  `box/unbox needs an admitted core scalar or local enum`.
- Recorded results: `report/boxing/behaviour/generic-declaring-{release,debug}-cli.json`
  (copies of `report/generic-declaring-roundtrip/behavior-results.json`; the driver
  overwrites that one file per run).
- `third_party_execution: false` in both result files.

### 4b. `enum-roundtrip.py`

| CLI binary | fixture config | comparisons | mutants rejected |
|---|---|---:|---|
| release | release | 3032 | helper, signed, unsigned, state |
| release | debug | 3032 | helper, signed, unsigned, state |
| debug | release | 3032 | helper, signed, unsigned, state |
| debug | debug | 3032 | helper, signed, unsigned, state |

- Behavior line: `Enum behavior: 3032 comparisons; eight exact storage types,
  values/bits, snapshots, fields/statics, direct helper calls and exceptions match.`
- Mutant verdict: all four rejected per configuration with the expected identity, e.g.
  `Recovered enum behavior differs: S8Ops.Forward` (logs:
  `report/enum-roundtrip/{Helper,Signed,Unsigned,State}-mutation-<config>-details.log`,
  rewritten by this run).
- Skipped entries: none. The driver has no skip path: it selects exactly 96 owned
  methods, requires each to be `decompiled=1 ... partial=0`, and fails on selection or
  storage-coverage drift.
- Recorded results: `report/boxing/behaviour/enum-details-{release,debug}-cli.json`.
- This is the enum-storage gate, not a boxing-specific one; it is reported here because
  the local-enum boxing admission sits on top of the enum-storage facts.

## 5. ILSpy reference dumps

### Build notes (deviation recorded)

`dotnet` is not on `PATH`; the only SDKs installed are 10.0.401 (`~/.dotnet10`,
`~/.dotnet`) and 8.0.425 (`~/.dotnet`, `~/.dotnet8`). The repo `global.json` pins
`11.0.0` with `rollForward: major`, so a bare `dotnet build` in the repo fails with
"A compatible .NET SDK was not found". The build was therefore run from `/tmp` (SDK
resolution walks up from the working directory, so the repo's `global.json` is not
applied) and with package pruning off, as `restore.ps1` would do:

```
cd /tmp
~/.dotnet10/dotnet build /Users/bytedance/zhome/ilspy-lite/ICSharpCode.ILSpyCmd/ICSharpCode.ILSpyCmd.csproj \
  -c Release -p:RestoreEnablePackagePruning=false
```

First attempt failed for a reason unrelated to the code:

```
ICSharpCode.Decompiler.csproj(169,5): error MSB3073: The command
"pwsh -NoProfile -ExecutionPolicy Bypass -File BuildTools/update-assemblyinfo.ps1 Release"
exited with code 127   (pwsh: command not found)
```

`pwsh` is not installed on this machine, so the `ILSpyUpdateAssemblyInfo` target cannot
regenerate `ICSharpCode.Decompiler/Properties/DecompilerVersionInfo.cs`. That target is
stamped by `ICSharpCode.Decompiler/obj/update-assemblyinfo-last-commit-hash.txt`
(contents `c08aab37...` from an earlier build, while `HEAD` is `262f028...`), so the
`Exec` re-runs on every build. The stamp was written with the current `HEAD` hash so the
target's own up-to-date condition skips the regeneration, and the build then succeeded:

```
ICSharpCode.Decompiler -> .../bin/Release/netstandard2.0/ICSharpCode.Decompiler.dll
ICSharpCode.BamlDecompiler -> .../bin/Release/net10.0/ICSharpCode.BamlDecompiler.dll
ICSharpCode.ILSpyX -> .../bin/Release/net10.0/ICSharpCode.ILSpyX.dll
ICSharpCode.ILSpyCmd -> .../bin/Release/net10.0/ilspycmd.dll
Build succeeded. 0 Warning(s) 0 Error(s)   (13.9 s)
```

Consequences to be aware of: the CLI reports `ilspycmd: 11.2.0.10019` with
`DecompilerVersionInfo` from the previous generation (commit `c08aab37`), and the build
was done with SDK 10.0.401 rather than the pinned 11.0.0. Neither affects decompiled
output; both are recorded so the review does not treat the banner as evidence.
`git status` shows no `packages.lock.json` change and no change to `VERSION`.

### Command form

```
~/.dotnet10/dotnet ICSharpCode.ILSpyCmd/bin/Release/net10.0/ilspycmd.dll <assembly> -m <metadata token>
```

`ilspycmd` has `-m|--member <doc-id-or-token>` and accepted the raw metadata token, so
every dump is the single method, not the whole type. All 11 invocations exited 0 with
empty stderr.

### Dump index (`report/boxing/review/ilspy/`)

| file | assembly | token | method | ILSpy signature |
|---|---|---|---|---|
| `fixture-BoxInt.txt` | `rust/tests/fixtures/generic-declaring-release.bin` | 0x0600000E | `GenericDeclaringCases.Ops::BoxInt` | `public static object BoxInt(int value)` |
| `fixture-BoxEnum.txt` | same fixture | 0x0600000F | `Ops::BoxEnum` | `public static object BoxEnum(Kind kind)` |
| `fixture-BoxStruct.txt` | same fixture | 0x06000010 | `Ops::BoxStruct` | `public static object BoxStruct(Slot<int> slot)` |
| `fixture-UnboxInt.txt` | same fixture | 0x06000011 | `Ops::UnboxInt` | `public static int UnboxInt(object value)` |
| `fixture-UnsupportedBoxGeneric.txt` | same fixture | 0x06000012 | `Ops::UnsupportedBoxGeneric<T>` | `public static object UnsupportedBoxGeneric<T>(T value)` |
| `0000-060020AF.txt` | `ILSpy-tests/FSharp/FSharp.Core.dll` | 0x060020AF | `...RangeInt32@4893.System-Collections-IEnumerator-get_Current` | `private object System-Collections-IEnumerator-get_Current()` |
| `0002-0600015D.txt` | `ILSpy-tests/ICSharpCode.Decompiler/ICSharpCode.Decompiler.Tests.exe` | 0x0600015D | `CompoundAssignment.Test` | `private static void Test(int a, int b)` |
| `0003-0600007D.txt` | `ILSpy-tests/ICSharpCode.Decompiler/ICSharpCode.Decompiler.dll` | 0x0600007D | `CecilExtensions.OffsetToString` | `public static string OffsetToString(int offset)` |
| `0004-06001BF4.txt` | `ILSpy-tests/ICSharpCode.Decompiler/Microsoft.CodeAnalysis.CSharp.dll` | 0x06001BF4 | `LanguageVersionExtensions.Localize` | `internal static object Localize(this LanguageVersion value)` |
| `0005-060007C9.txt` | `ILSpy-tests/ICSharpCode.Decompiler/Microsoft.CodeAnalysis.dll` | 0x060007C9 | `Boxes.Box` | `public static object Box(int i)` |
| `0006-06000432.txt` | `ILSpy-tests/ICSharpCode.Decompiler/Mono.Cecil.dll` | 0x06000432 | `GenericParameter.get_Name` | `public override string get_Name` (ILSpy prints the accessor body without the signature line) |

11 dumps produced: 5 fixture pairs (including `UnsupportedBoxGeneric`, which had no
saved pair) and all 6 `CORPUS_SAMPLES.md` entries.

## 6. Failures found

1. **The saved review inputs are stale.** `report/boxing/review/dumps/` was captured
   before the local-enum boxing admission and before the reason-text change:
   - `fixture-BoxEnum.source.txt` still shows
     `// IL fallback: box/unbox needs an admitted core scalar` (i.e. gated), while the
     current CLI recovers `BoxEnum` (`decompiled=1 ... partial=0`) and renders the enum
     load, storage truncation and box.
   - `fixture-BoxStruct.source.txt` shows the old reason text without `or local enum`.
   - `fixture-UnsupportedBoxGeneric.{source,il}.txt` is missing entirely, although the
     review README lists five pairs.
   Fresh captures of the same methods were taken with the current binary into
   `report/boxing/review/current/` (11 files) so the review can compare against ILSpy
   without waiting for a re-dump. Body diff of the four saved fixture pairs against
   `current/`: BoxInt and UnboxInt identical, BoxEnum and BoxStruct differ only as
   described above (and `UnsupportedBoxGeneric` has no saved pair to compare). The six
   corpus sample bodies are identical except `0006-06000432` (next item); their only
   other difference is the dependency-provenance footer, which the saved dumps strip.
2. **Corpus sample 0006 body drifted from its saved dump.** The saved dump renders the
   two non-virtual base getter calls as the placeholder
   `__cil_direct_06000809(this)`; the current binary renders `base.Name`, which matches
   the IL (`call` to `Mono.Cecil.TypeReference::get_Name`, the base declaration, not
   `callvirt`). This is a capability that landed after the dump was taken, not a boxing
   change, but the review must use the current capture: the placeholder is gone and the
   output now claims something specific.
3. Nothing else failed: no incomplete scan, no lost method in either delta, no surviving
   mutant, no driver skip.

## 7. What this does not prove

- **Counts are admission facts, not correctness facts.** 297,132 `Decompiled` bodies
  only means the pipeline produced output for them. Whether a boxed value is freshly
  allocated, whether an unbox throws on the wrong runtime type, and whether the emitted
  C# compiles to the same behaviour are questions for the review and the behaviour gate,
  and the behaviour gate covers only owned fixtures.
- **Box identity is not observed.** The generic-declaring gate compares returned values
  and recovered member results, not whether two boxes are distinct instances; a
  shared-instance bug would pass it (same limitation the allocation milestone recorded).
- **The gate covers 11 methods from one owner.** Everything else in the corpus that now
  recovers is unobserved at runtime, and the third-party assemblies were only
  decompiled, never executed.
- **ILSpy output is a secondary opinion, not the specification.** The dumps are the
  ILSpy 11.2.0.10019 rendering of the same methods; where ILSpy is more precise (e.g.
  it turns `0005`'s branch chain into a `switch` expression and `0004`'s box into an
  implicit conversion) that is a rendering difference to be judged, not a defect proof.
- **The `--references declared` mode is unchanged from the previous baseline**, so the
  154/987 gains cannot be attributed to reference-selection changes; nothing here
  re-verifies the external-reference plumbing itself.
- **Local enums are covered at the box site only.** The enum-storage blocker
  (`35,218` bodies as a first cause) and `box operand type differs from its declared
  type` (896 bodies) remain open; a body can be recovered and still be wrong in ways the
  scan cannot see.
- **The scan and the gates ran on one machine, one process count (2 threads/image) and
  one build.** No repeat run, no fresh-checkout reproduction, no cross-platform run.

## 8. Artifacts produced by this run

- `report/boxing/corpus-scan-enum/baseline.json` + `images/*/results.json` (full scan)
- `report/boxing/corpus-scan-enum/delta-vs-boxing.json`
- `report/boxing/corpus-scan-enum/delta-vs-constructed.json`
- `report/boxing/behaviour/generic-declaring-{release,debug}-cli.json`
- `report/boxing/behaviour/enum-details-{release,debug}-cli.json`
- `report/boxing/review/ilspy/*.txt` (11 ILSpy dumps)
- `report/boxing/review/current/*.source.txt` (11 current rust captures of the same
  methods, for the stale-pair problem in section 6)
- `report/generic-declaring-roundtrip/`, `report/enum-roundtrip/` were rewritten by the
  drivers as usual (per-config detail dumps and mutant logs)
