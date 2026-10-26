# Frozen naming: actual-default Object-only constructors

This original bounded gate retains its own exclusions. The subsequent
[singleton](FROZEN_NAMING_SINGLETONS.md) and
[argument constructor](FROZEN_NAMING_ITERATOR_CONSTRUCTORS.md) gates execute the
additional bodies; those exclusions are not unexecuted bodies in the final
naming acceptance.

Optional offline test, not final naming-goal completion. No corpus is checked in.
From the repository root, with the existing .NET 10 SDK/reference pack/runtime in
`~/.dotnet10` (or `DOTNET_ROOT`) and Cargo's existing offline cache:

```sh
python3 rust/tests/frozen-naming-constructors.py \
  --corpus /absolute/path/to/existing/rust/report/openra
```

The corpus is read-only. Required inputs are `assemblies/`, `runtime-deps/`, and
`random-classes/final-21c469ba6/*/class.json`. Missing inputs fail; this test never
acquires dependencies. Cargo uses `build --release --locked --offline`. C# uses
installed Roslyn directly with reference-pack DLLs only: no project restore,
original assembly compiler reference, original factory, or detailed-output body.

## Boundary and evidence

The unchanged random twenty and separate supplemental three manifests define
owner trees, not a new sample. Assembly SHA-256 values come from the existing
OpenRA manifest; MVIDs, root tokens and names must match the frozen selection.
The C# host's static PEReader catalog independently enumerates descendants,
method tokens/signatures, fields, raw IL, exception regions, and initializers.
The eligible original body is exactly `ldarg.0; call Object::.ctor(); ret`, with
nops only at instruction boundaries. The call must be a MemberRef for the pinned
standard `System.Runtime` 6.0.0.0 core Object identity and parameterless signature.
CLI original IL bytes must agree with PEReader; loaded original IL must agree
again before execution. No module initializer is permitted. Any owner `.cctor`
is inspected statically and excluded rather than executed.

The driver saves the current CLI's exact default `--type` stdout. It binds each
constructor by the independent owner tree and unique parameterless metadata
shape, checks method-local statuses against that inventory and frozen baseline,
and extracts the declaration, initializer and body without modification. A tiny
field-free instance shell keeps the actual alias spelling inside an ordinary
outer class. Aliases are presentation, never runtime identity. The shell is not
a reconstruction of the full original class.

For each eligible constructor, four independently allocated receiver pairs and
two calls per pair execute the original on the reference side only and the
actual compiled recovered constructor on the other side. Both loaded bodies
must have the exact Object direct-call protocol, without proxy/extra calls.
Original primitive/reference fields are snapshotted before execution and checked
for preservation; writable original I4 fields receive seeds -1, 0, 1, MaxValue.
Reference fields remain independently zero-initialized. Unknown value-type fields
are explicitly reported as unobserved, never compared via user `Equals`.
No original poststate is imported into a recovered receiver.

Every eligible constructor also has a separately compiled mutation: its actual
body throws `InvalidOperationException("frozen-ctor-mutation")`. The host verifies
the precise compiled mutation IL, then executes it. Only the specific runtime
mismatch after the original returned, with exit 42, satisfies the control.
Compiler errors, ABI/protocol guards, and unrelated exceptions do not count.

Ignored `rust/report/frozen-naming-constructors/` contains exact default output,
static catalogs, original IL, baseline records, extracted wrapped source, compiler
response files (all references), commands/exit codes, positive/mutation logs, and
`summary.json`. A failed run removes the success summary. An Object-only ctor
regressing to Partial fails instead of silently shrinking execution coverage.

## Checked result at cac683669

13 eligible constructors, **104 positive pairs**, and **13 specific runtime
mutation rejections**. Eleven constructors are from random20 owner trees and two
are supplemental. Tokens below are MethodDefs in `OpenRA.Mods.Common.dll` except
the explicitly marked Game entry; owner paths are abbreviated for readability.
Exact assembly/MVID/owner/method identities are in the saved inventory and report.

| Token | Owner suffix |
|---|---|
| 0x06003442 | LatencyTooltipLogic/<>c__DisplayClass0_1 |
| 0x06003A94 | BaseBuilderQueueManager/<>c__DisplayClass23_1 |
| 0x06002F3C | AnonymousProfileTooltipLogic/<>c__DisplayClass0_0 |
| 0x06002A13 | ProductionProperties/<>c__DisplayClass2_0 |
| 0x060045A5 | WithHarvestOverlay/<>c__DisplayClass3_0 |
| 0x060045A8 | WithHarvestOverlay/<>c__DisplayClass3_1 |
| 0x060011A9 | Widget/<>c__DisplayClass28_0 (OpenRA.Game.dll) |
| 0x060031E2 | GameInfoStatsLogic/<>c__DisplayClass21_4 |
| 0x06003411 | IntroductionPromptLogic/<>c__DisplayClass6_0 |
| 0x0600338C | DownloadPackageLogic/<>c__DisplayClass20_0 |
| 0x060029A2 | SkirmishLogic/<>c__DisplayClass1_1 |
| 0x06003160 | GameSaveBrowserLogic/<>c__DisplayClass31_0 (supplemental) |
| 0x06002F11 | VideoPlayerWidget/<>c__DisplayClass26_1 (supplemental) |

All 83 methods remain classified with individual reasons: 56 Partial, 13 eligible,
2 excluded Object-only ctors with unproven initialization, 2 already-Decompiled
baseline constructors, 2 newly recovered argument constructors, and 8 newly
recovered nonconstructor methods. The random20 totals remain 25 Decompiled /
52 Partial; supplemental3 totals are 2 Decompiled / 4 Partial.

The two Object-only exclusions are deliberate:

- WithSpriteBodyInfo/<>c, ctor `0x0600468C`: `.cctor 0x0600468B` executes
  `newobj 0x0600468C; stsfld 0x04003BD9; ret`.
- DefaultSpriteSequence/<>c, ctor `0x06002C76`: `.cctor 0x06002C75` executes
  `newobj 0x06002C76; stsfld 0x040024C3; ret`.

These initializers allocate selected-original implementations, which are not
permitted factories/leaves for this gate. Neither initializer nor ctor executes.
The two argument ctors (`0x06001477` and `0x0600276F`) and all eight recovered
methods have explicit per-method exclusions in the summary. No Actor/world,
callback or game path is used.

## Limits

This proves only bounded execution of field-free constructor fragments. It does
not prove full class ABI, interfaces, allocation/game behavior, alias runtime
identity, or state equivalence for omitted unused fields. The `target` struct
field of BaseBuilderQueueManager/<>c__DisplayClass23_1 is explicitly unobserved;
all other eligible original instance fields were snapshot-checked. No constructor
null-receiver claim is made. Other recovered methods/constructors still need
their own appropriate gates. No production admission is changed by this test.
