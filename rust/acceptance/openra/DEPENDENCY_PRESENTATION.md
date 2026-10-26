# Static dependencies and constructor/expression presentation

Later naming-only recovery and the unchanged twenty plus three supplemental
selections are reported in [SYMBOL_NAMING.md](SYMBOL_NAMING.md). Historical counts
below retain their original scope.

This extends the frozen [constructor contract](CONSTRUCTOR_PRESENTATION.md), not
the original class-delivery approval. Production remains independent Rust with
pinned cildec 0.1.0, sha1 0.10.7 and serde_json 1.0.154; no dotnet subprocess,
runtime assembly loading, input execution, package restore or SDK installation.
See [dependency policy](../../DEPENDENCIES.md) and [semantics](../../SEMANTICS.md).

## Delivered scope

- Explicit immutable metadata context and bounded automatic local/deps/NuGet/
  shared-framework/reference-pack/Framework/GAC/Mono discovery. Byte APIs retain
  their no-filesystem behavior. Exact identity is default; replacement is explicit,
  metadata-only and diagnosed. Incomplete search cannot authorize replacement.
- Actual scoped type/method/field/property definitions, declared generic positions,
  MethodSpec arguments, base/interface edges, bounded forwarding and reference
  contracts. This is not a full CLR verifier, inherited-member or C# type system.
- Existing original-this local/external base initialization and safe explicit
  constructor initializers remain intact. Earlier effects keep intrinsic calls.
- Property source syntax requires real MethodSemantics and separate exact source
  binding. Owner, reference/definition signatures and the independent PropertySig
  path must all be exact. Changed/other receivers, ambiguous names, unsupported
  dispatch and uncertain metadata keep precise calls.
- Adjacent signed-I4 field/local decrement can use postfix. Live old-value
  snapshots remain before later effects/reads; an immediate sole zero comparison
  can use an unchecked postfix return. Checked/unsigned, effectful receivers,
  intervening operations and exposed-address cases are not forced into that form.
- Unsupported bodies still contain complete original commented IL in their own
  method, including locals/EH/raw evidence. Details remain an additional audit
  view, not a prerequisite for seeing fallback bodies.

## Wait: real Release CLI output, not an illustrative rewrite

The unchanged Common input selects both constructors and Tick: **3 Decompiled,
0 Partial/Failed/Declaration; exit 0**. With the matching Game metadata, actual
output contains explicit base initializers, base-qualified interruptibility/
canceling accesses and the unchecked old-value countdown expression. Some harmless
Boolean temporaries remain; a whole recompilable project is not promised.

Ignored actual output/stdout/stderr files:
`rust/report/openra/wait/final-dependency-demo/{automatic,explicit,isolated,details}.*`.
Automatic and explicit metadata modes recover properties. The isolated no-search
mode retains exact accessor calls; numeric postfix recovery needs no SDK.
Missing unrelated framework metadata is disclosed without suppressing these bodies.

Reproduce from the repository root using the existing input cache:

```sh
rust/target/release/ilspy-lite decompile \
  rust/report/openra/assemblies/OpenRA.Mods.Common.dll \
  --type OpenRA.Mods.Common.Activities.Wait
rust/target/release/ilspy-lite decompile \
  rust/report/openra/assemblies/OpenRA.Mods.Common.dll \
  --type OpenRA.Mods.Common.Activities.Wait --no-auto-references \
  --reference rust/report/openra/assemblies/OpenRA.Game.dll
```

Actual readable class-body gates pass **242** constructor/subsequent-Tick
comparisons and a separate **193** Tick comparison gate. Base initialization,
receiver/argument order, dirty state, null failures, cancellation, old values and
integer wraparound are checked. Property leaves compile the actual C# expression
and verify direct-call opcode/target; they do not borrow detailed statements or
proxy selected original bodies. Own-property leaves call recovered accessors.

## Frozen twenty: all retained, mostly unsupported bodies

The seed remains `a851d53c66e12a7c1282d0c93cfaeca8`; baseline `39bd383ff`, 4,472
eligible classes, twenty ranks without replacement. Wait is separate. The original
[manifest](random-classes.json), population and provenance are unchanged; no
support/visibility/generic/generated/nested filtering or resampling occurred.
Sampling replay instructions remain in CONSTRUCTOR_PRESENTATION.md.

All **20/20 static selections pass** actual automatic and explicit configurations:
**77 methods = 2 Decompiled + 75 Partial; 0 Failed/Declaration**, no metadata
inventory disagreement. This is a low supported-body rate, not a claim of broad
OpenRA decompilation. All partials retain their own full fallback evidence.

The independent PEReader catalog checks detailed metadata and the actual readable
per-owner declaration tree: fields, methods/constructors, signature arity/count,
property/event relationships and nested ownership. Fallbacks are matched to their
own consumed method-body spans, not a whole-class bag of IL. See
[FROZEN20_STATIC.md](FROZEN20_STATIC.md) for same-shape-overload and parser limits.

Final static artifacts are under
`rust/report/openra/random-classes/final-21c469ba6/`: report.json, all twenty
class.json files, original inventories/IL and actual stdout/stderr/argv/exit records
for both configurations/styles. The static reports deliberately retain
`noRuntimeProof`; the execution evidence below is separate.

Reason codes: A = existing non-simple/generated or explicit-interface body-name
restriction; B = nonlocal/constructed fields; C = callvirt; D = constructed/
generic ancestry outside body assignment; E = general allocation; F = EH/data
sections; G = unsupported value/generic/object signature; H = nonadmitted
core/value-construction identity. These are current body-model limits, not missing
class selections or proof that dependency metadata itself was unavailable.

| # | Frozen TypeDef / class suffix (full identity in manifest) | D/P | Remaining limits | Execution evidence |
|---|---|---:|---|---|
| 1 | 02000B65 LatencyTooltipLogic/DisplayClass0_1 | 0/2 | A | None: both bodies outside admission |
| 2 | 02000F97 WithSpriteBodyInfo/<>c | 0/4 | A | None: all bodies outside admission |
| 3 | 020003E1 DamagedByTerrain | 0/2 | A,D | None: explicit-name/constructed ancestry |
| 4 | 02000514 ProximityCapturable | 1/4 | B,C | Ctor: 4 exception-only pairs; other four not executed |
| 5 | 0200047E MustBeDestroyed | 1/0 | None in selected body | Ctor: 10 successful state/identity pairs |
| 6 | 02000CCC BaseBuilderQueueManager/DisplayClass23_1 | 0/2 | A | None: both bodies outside admission |
| 7 | 02000A38 AnonymousProfileTooltipLogic/DisplayClass0_0 | 0/2 | A | None: both bodies outside admission |
| 8 | 020000EB ProductionProperties | 0/4 | A,C,E | None: names/callvirt/allocation |
| 9 | 020003BB ZipFileLoader/ReadOnlyZipFile/get_Contents iterator | 0/9 | A,F | None: generated/EH bodies |
| 10 | 02000858 UnhardcodeBaseBuilderBotModule/BeforeUpdateActors iterator | 0/8 | A,F | None: generated/EH bodies |
| 11 | 02000714 WithHarvestOverlay | 0/11 | A,E,G | None: names/allocation/signature |
| 12 | 020002EA Widget/DisplayClass28_0 | 0/2 | A | None: both bodies outside admission |
| 13 | 02000462 GivesExperienceMultiplier | 0/2 | A,D | None: explicit-name/constructed ancestry |
| 14 | 02000AD7 GameInfoStatsLogic/DisplayClass21_4 | 0/5 | A | None: all bodies outside admission |
| 15 | 020005D4 MarkerLayerOverlayInfo | 0/2 | G,H | None: signature/construction outside admission |
| 16 | 020007A0 SimpleTeleport | 0/2 | C,G | None: callvirt/signature |
| 17 | 02000B5C IntroductionPromptLogic/DisplayClass6_0 | 0/5 | A | None: all bodies outside admission |
| 18 | 02000B36 DownloadPackageLogic/DisplayClass20_0 | 0/4 | A | None: all bodies outside admission |
| 19 | 02000985 DefaultSpriteSequence/<>c | 0/3 | A | None: all bodies outside admission |
| 20 | 020008BA SkirmishLogic/DisplayClass1_1 | 0/2 | A | None: both bodies outside admission |

The two applicable constructor fragments execute actual default class statements:

- Must ctor `060017D4`: ten successful comparisons over independently fresh/dirty
  receivers and repeated distinct/null Info overwrites. Exact core Object
  initialization and own argument identity are checked; deleting Info storage fails.
- Proximity ctor `06001A46`: four null-initializer exception paths enter exact
  nonselected immediate-base `06001A4F` once with unchanged receiver/arguments.
  Pinned IL is checked to fault before Actor/world callbacks. Fresh empty base
  list, base.Info, preserved derived.Info and other base state are checked; moving
  derived.Info before the base call fails.

**Proximity success, full-chain/gameplay behavior and its post-base Info assignment
on a successful path remain unproved.** No non-null initializer or Actor/world
initialization is claimed. Null receiver behavior is not claimed for these two
random constructors. Compiler-required dummy constructors and throwing abstract
stubs are never executed. No original initialized poststate is copied.

Run the separately authorized finite execution gate and controls:

```sh
python3 rust/tests/frozen-constructors.py --corpus rust/report/openra --mutate must-info
python3 rust/tests/frozen-constructors.py --corpus rust/report/openra --mutate prox-order
python3 rust/tests/frozen-constructors.py --corpus rust/report/openra
```

## Integrated evidence and limitations

`rust/report/final-dependency-presentation/` retains final Rust Debug/Release,
formatter/clippy, independent-oracle, collector and serial actual-readable-body
logs. The checked production/tooling state is recorded there; subsequent changes
are test-adapter/documentation integration, with affected fixture checks repeated.

- Independent metadata: **406** catalog comparisons, **18** rejected exported
  token/signature/relationship mutations; required-shape preflights reject omitted
  interfaces/generic/nesting/forwarding/accessor shapes. Static evidence only.
- Owned base constructors: **48/config**. Owned base/own properties, hiding,
  virtual/direct dispatch, dirty/null state: **40/config**. Changing a compiled
  base lookup to virtual this lookup is rejected.
- Postfix/local snapshots: **960/config**, repeated under checked compilation;
  prefix-value mutation fails. Effects between read/write remain explicit.
- Prior actual readable gates remain passing: RNG **22,941**, resources/ammo
  **837**, Queue **20**, and the affected reference/array/address/conversion/native/
  string/exception/float/core/delegated/integer corpora.
- Static all20 evidence is distinct from the **14 paired** random-constructor
  cases. One constructor's success domain does not certify another's failures.
- Local environment: macOS arm64, Rust/Cargo 1.98, Python 3.9, optional .NET SDK
  10.0.401/runtime 10.0.12. No native Windows/Mono execution, Rust 1.85, 32-bit,
  original .NET 6 runtime or cross-project compatibility result is claimed.

Relevant review closures include integrated binding f4af6899, signature guard
383121f3, path discovery 77fb9e6b, oracle omission guard 18cceff0, scope-key
e58c2123, property identifiers d2388702, property source/own-source leaves
4bb274f7/005801b0, independent PropertySig certainty cf576c50, postfix local/live
snapshots b9f7e642 and fallback-body ownership 568201b1. Initial BLOCK findings
were reproduced and corrected; these bounded source/log reviews are not a
universal correctness or independently rerun execution certificate.

This work does not transplant ILSpy's full AST/SSA/type system, make every sampled
body supported, provide a C# runtime adapter, prove publisher authenticity or
promise whole-project recompilation. All GPL-derived body/IL output remains in
ignored reports; no push is performed.
