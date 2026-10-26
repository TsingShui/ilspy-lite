# Symbol naming and body admission

## Frozen baseline

Implementation baseline: `519982c88`. The unchanged twenty-class manifest is
[random-classes.json](random-classes.json); the three additional observations are
[naming-samples.json](naming-samples.json), not replacement selections.

Reuse the original MVIDs and provenance from [BASELINE.md](BASELINE.md).
No input was reacquired or changed. The corrected prior static report
`rust/report/openra/random-classes/final-21c469ba6/report.json` applies to the same
production code: later changes up to this baseline were test integration/docs.

The twenty classes contain 77 methods: 2 Decompiled, 75 Partial, no Failed or
Declaration. Of the partials, 58 first report a non-simple identifier. This is the
first gate only, not proof that those 58 bodies have no other unsupported shape.
The other first reasons are 3 constructed/nonlocal fields, 3 EH/data sections,
3 signatures, 2 ancestry, 3 callvirt, 2 allocation and 1 core identity.

The separate three-class observation has 6 Partial bodies: 4 first report naming,
1 signature and 1 array element admission. Actual original default outputs and
the draw record remain under `rust/report/openra/class-demo-gu5q325h/`.
Wait is separate: 3 Decompiled bodies, exit 0.

## Owned reproduction

`NamingCases.Factory/<>c__DisplayClass0_0::.ctor` contains only the existing
Object-base protocol. The public readable and detailed method interfaces must
recover it without changing constructor semantics. Before implementation,
`cargo test --locked --test naming` fails with Partial and
`non-simple identifier needs IL fallback`; the full red is retained at
`rust/report/naming-generated-owner-red.log`.

The same owned source includes a captured-I4 update and a static constructor.
Only ordinary existing shapes may become supported. Static initialization,
unsupported types/instructions, generic/value-type/EH admission and new ordinary
allocation remain outside this naming change.

## Scoped naming corrections

The generated-owner constructor gate is green in Debug and Release. Type paths
are constructed from scoped TypeDef/TypeRef rows, not parsed display labels.
The shared identifier rule reserves tool alias prefixes so a literal user name
cannot impersonate an encoded generated name. Actual public declaration and
field-reference tests reproduced the collision and mismatched path before the
correction; logs use `naming-reserved-alias-*` and `naming-type-reference-*`.

Presentation aliases are not source lookup names. A real external property named
`__cil_X` must retain the exact call, rather than accessing an invented encoded
property. That mutation reproduced the wrong access and is green after the
source-binding fence; logs use `naming-property-alias-*`.

A literal backtick in a nongeneric TypeDef is not generic context. Owned
``X`Y__`` reproduced both truncation to `X` and false generic-body rejection.
TypeDef generic admission now uses actual GenericParam/enclosing metadata;
canonical arity spelling is removed only when appropriate for presentation.
Generic bodies remain fenced. Declaration compilation also covers this case;
logs use `naming-backtick-*`.

The follow-up local-TypeRef case also reproduced a real declaration/reference
mismatch: nongeneric ``Q12`1`` was declared as an encoded alias while a nested,
Module-scoped TypeRef rendered as `Q12`. The saved pre-fix output fails csc with
CS0426 (`naming-local-typeref-before-compile.log`). Rendering now canonicalizes
only exact in-module references to the same TypeDef, using the scoped definition
lookup shared with binding. Ambiguous/cyclic local paths fail rather than choose
a spelling. Foreign scopes do not become local matches. Sixteen actual complete
owned declaration outputs compile; this is not original CLR loadability proof
for adversarial metadata names.

## Verification contract

Use public byte/context/CLI output as the test interface. Preserve scoped tokens
and raw names; aliases are presentation, never member identity or source-binding
proof. Declaration/reference names must agree without rewriting literals.
Newly recovered default class statements require original/recovered execution,
not merely a changed status. Retain independent ownership/inventory and
method-local fallback checks for every frozen selection.

## Naming rules and interface limits

- Assembly/module scope plus original tokens remain identity. Exact metadata
  full names, including `/` nesting and generic arity, remain CLI selectors.
- Simple ASCII names are retained, with C# keyword escaping. Other names use
  `__name_` plus uppercase UTF-8 hex. Literal user names beginning with
  `__name_`, `__cil_` or `__il_` are encoded too, so they cannot impersonate
  tool names. The encoding is deterministic, not a prettier lambda reconstruction.
- Field names requiring aliases use their original `__il_field_<FieldDef token>`
  alias consistently in declarations, instance accesses and static accesses.
  Details retains the original field/token/signature evidence.
- Type paths are built from metadata namespace/enclosing rows, not split from
  human full-name labels. A dot within one type-name record is encoded, not
  mistaken for another namespace or type. Local Module/nested TypeRefs use the
  exact definition's naming policy; foreign references stay scoped and opaque.
- GenericParam ownership, not a punctuation test, fences local generic bodies.
  Static initialization and runtime-special methods remain explicit negatives.
  Canonical generic arity presentation does not admit generic body semantics.
- Names in generated declarations and references agree; strings/characters and
  unrelated user identifiers are not lexically replaced. A presentation alias
  does not prove an external property's source lookup: exact calls stay when
  that proof is unavailable.

Public byte/style/context interfaces and their defaults are unchanged. No implicit
filesystem search was added to byte interfaces; dependency discovery and version
policy are unchanged. No new dependency, type/operator/call/EH admission or
AST/SSA framework was introduced.

## Integrated frozen results

Production naming fixes through `015afc66e`; execution-gate integration at
`3de7df578`. Subsequent report/document changes do not change body admission.
The actual automatic and explicit outputs pass **20/20** frozen static selections
and **3/3** separate supplemental selections, including independent declarations,
ownership, nesting and method-owned complete fallback checks.

- Frozen twenty: **77 methods: 25 Decompiled, 52 Partial, 0 Failed/Declaration**,
  versus the original 2/75. Of the 58 first-name blockers, 23 recover and 35 expose
  other existing limits. No class was replaced or filtered; the seed is unchanged.
- Supplemental three: **6 methods: 2 Decompiled, 4 Partial**. Their two generated
  constructors recover. LegacyBridgeLayerInfo still has signature/array limits.
- Across all 23 owner trees: **83 methods, 27 Decompiled, 56 Partial**. All methods
  have individual before/after status, original owner identity and execution or
  nonexecution reason in `rust/report/openra/naming-final/acceptance.json`.
- Remaining first blockers include unsupported signatures (20 in frozen20),
  nonlocal/constructed fields (6), ancestry (5), callvirt (4), EH/data sections
  (3), plus explicit initialization, allocation and other unsupported operations.
  Moving past a name check is not a claim that the rest of a method is supported.

D/P counts below include selected descendants, not only the root's own methods.
`Executed` counts methods with bounded cases, not fully proven classes. F4 is
exception-only; all other limits below still apply. Full names/MVIDs are in the
unchanged manifests and per-method report.

| Selection | TypeDef / owner suffix | D/P | Executed methods |
|---|---|---:|---:|
| F1 | 02000B65 LatencyTooltipLogic/DisplayClass0_1 | 1/1 | 1 |
| F2 | 02000F97 WithSpriteBodyInfo/<>c | 2/2 | 0 |
| F3 | 020003E1 DamagedByTerrain | 0/2 | 0 |
| F4 | 02000514 ProximityCapturable | 1/4 | 1 |
| F5 | 0200047E MustBeDestroyed | 1/0 | 1 |
| F6 | 02000CCC BaseBuilderQueueManager/DisplayClass23_1 | 1/1 | 1 |
| F7 | 02000A38 AnonymousProfileTooltipLogic/DisplayClass0_0 | 1/1 | 1 |
| F8 | 020000EB ProductionProperties | 1/3 | 1 |
| F9 | 020003BB ZipFileLoader/ReadOnlyZipFile/get_Contents iterator | 2/7 | 1 |
| F10 | 02000858 UnhardcodeBaseBuilderBotModule/BeforeUpdateActors iterator | 3/5 | 2 |
| F11 | 02000714 WithHarvestOverlay | 5/6 | 5 |
| F12 | 020002EA Widget/DisplayClass28_0 | 1/1 | 1 |
| F13 | 02000462 GivesExperienceMultiplier | 0/2 | 0 |
| F14 | 02000AD7 GameInfoStatsLogic/DisplayClass21_4 | 1/4 | 1 |
| F15 | 020005D4 MarkerLayerOverlayInfo | 0/2 | 0 |
| F16 | 020007A0 SimpleTeleport | 0/2 | 0 |
| F17 | 02000B5C IntroductionPromptLogic/DisplayClass6_0 | 1/4 | 1 |
| F18 | 02000B36 DownloadPackageLogic/DisplayClass20_0 | 2/2 | 2 |
| F19 | 02000985 DefaultSpriteSequence/<>c | 1/2 | 0 |
| F20 | 020008BA SkirmishLogic/DisplayClass1_1 | 1/1 | 1 |
| S1 | 02000ABB GameSaveBrowserLogic/DisplayClass31_0 | 1/1 | 1 |
| S2 | 02000A29 VideoPlayerWidget/DisplayClass26_1 | 1/1 | 1 |
| S3 | 020005C1 LegacyBridgeLayerInfo | 0/2 | 0 |

Static artifacts retain `noRuntimeProof`; execution is separate:

- `rust/report/openra/random-classes/naming-final-3de7df578/`: all twenty actual
  outputs, independent inventories, original IL, statuses and process evidence.
- `rust/report/openra/naming-samples/final-3de7df578/`: the same checks for the
  separate three, using the existing collector rather than relaxed checks.
- `rust/report/openra/naming-final/acceptance.json`: all 83 method classifications,
  references to those static records and the bounded execution evidence.

## Actual-default execution and remaining gaps

The optional gates use the already installed .NET 10 tools only. Production stays
Rust/static. All GPL-derived generated bodies remain in ignored reports.

- [Owned closure gate](../../tests/NAMING_ROUNDTRIP.md): **356 pairs** across both
  fixture configurations; original-this Object initialization, dirty/repeated
  state, captured-I4 old return/write, wraparound and null faults. Two compiled
  wrong-store mutants fail specifically at runtime. Recovered class stdout is
  compiled verbatim inside its ordinary parent, with framework references only.
- The companion declaration gate compiles **16 actual complete owned outputs**:
  reserved-name collisions, nested/dotted/UTF-8 names, namespace keywords, literal
  backticks and mixed local TypeDef/TypeRef paths. Two alias-conflation mutants
  must fail specifically with CS0102. This is syntax/reference evidence, not
  runtime proof or CLR-loadability proof for adversarial metadata.
- [Real field-free constructors](../../tests/FROZEN_NAMING_CONSTRUCTORS.md):
  **13 constructors / 104 pairs / 13 compiled runtime mutation rejections**,
  eleven in frozen owner trees plus two supplemental. Unused recovered fields
  are omitted; one original struct field is explicitly unobserved. No full class
  ABI, arbitrary field state or constructor-null claim follows.
- [Real scalar methods](../../tests/FROZEN_NAMING_METHODS.md): **7 methods / 150
  pairs / 7 compiled runtime mutation rejections**. Actual default body substrings
  are unchanged in explicit test shadows. Field/return identity, dirty stores,
  repeated calls, 21 null-this and 3 null-referent pairs are checked. Public
  nonvirtual headers and a null-only unused Actor-to-object test parameter are
  disclosed; no nonnull Actor, interface dispatch or game behavior is claimed.
- The previous frozen constructors still pass **Must 10 successful pairs** and
  **Proximity 4 exception-only pairs**, without selected-original proxies or
  copied poststate. Proximity success, Actor/world initialization and its
  successful post-base write remain unproved.

Thus **22 of 27 supported methods across all 23 trees have bounded execution
cases** (20 of 25 in frozen20). Supported but NOT executed:

1. WithSpriteBodyInfo/<>c ctor `0600468C` and constant method `0600468E`, and
   DefaultSpriteSequence/<>c ctor `06002C76`: their owner initializers allocate
   selected-original singleton implementations. Initializers are inspected and
   excluded, never used as a recovered factory/leaf.
2. Iterator argument constructors Game `06001477` and Common `0600276F`:
   state/thread-identity setup is outside the field-free constructor gate. Their
   actual output and complete original IL are retained, but no runtime
   equivalence is claimed. The iterator as a whole is not reconstructed.

The 56 Partial methods are not executed as recovered C#. They retain complete
method-local evidence and their concrete first blocker in the per-method report.
These limits are not hidden by the improved status count.

## Reproduction

From the repository root, with the unchanged ignored input cache and installed
optional developer SDK/runtime:

```sh
cargo test --manifest-path rust/Cargo.toml --locked --test naming
python3 rust/tests/naming-declarations.py --mutate-collision
python3 rust/tests/naming-roundtrip.py --mutate-store
python3 rust/tests/frozen-naming-constructors.py --corpus "$PWD/rust/report/openra"
python3 rust/tests/frozen-naming-methods.py --corpus "$PWD/rust/report/openra"
python3 rust/acceptance/openra/validate_naming_samples.py \
  --cli rust/target/release/ilspy-lite --assemblies rust/report/openra/assemblies \
  --output rust/report/openra/naming-samples/new-report \
  --oracle rust/report/openra/class-inventory/ClassInventory.dll
```

Run the unchanged frozen20 collector as documented in
[FROZEN20_STATIC.md](FROZEN20_STATIC.md). Report directories must be new. Sampling
reproduction remains in [CONSTRUCTOR_PRESENTATION.md](CONSTRUCTOR_PRESENTATION.md).
Existing provenance is reused; no acquisition, resampling or input changes occur.

Relevant bounded reviews: `4ff92e69` (initial naming/owned gate), `1e2d69bb`
(backtick/real ctor gate, with a latent local-TypeRef finding), `4cb85278`
(reproduced local-TypeRef fix closure), and `e45c98e8` (real scalar gate).
These are source/log reviews, not independently rerun executions or whole-goal
approval. Local execution uses macOS arm64, Rust/Cargo 1.98, Python 3.9 and
SDK 10.0.401/runtime 10.0.12; no other-platform/runtime or Rust 1.85 claim.
