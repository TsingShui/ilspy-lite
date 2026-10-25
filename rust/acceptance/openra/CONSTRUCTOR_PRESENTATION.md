# Constructor and expression presentation acceptance

This is a scoped extension of [READABLE_OUTPUT.md](../READABLE_OUTPUT.md), not an
SDK/dependency resolver or a general C# source exporter. Production baseline:
`39bd383ff`. The support contract below is frozen before evaluating the random
sample's lowering results. A failed in-scope shape is a defect, not permission to
narrow this contract or replace a sampled class.

## Frozen support contract

- Existing ordinary nongeneric reference-class constructors: exact `call .ctor`
  on unchanged original `this`, targeting the same type or its direct base.
  Admit local MethodDef and assembly-scoped external MemberRef direct-base
  identities with ordinary instance-void signatures and already-supported argument
  types. Preserve constructor context, target identity, argument order, exception
  and write positions, and no allocation/extra null check. A nonconstructor caller,
  unrelated owner, changed receiver, invalid signature or `callvirt .ctor` is not
  this shape. This is not a complete CLR initialization verifier.
- Readable initializer syntax only for the existing-receiver delegation whose
  argument evaluation and call can occupy the initializer without crossing any
  earlier observable operation. Literal/immutable argument copies may be removed;
  initialization cannot be erased. Parameterless direct-base initialization may
  be implicit when exactly equivalent. Otherwise keep the exact operation in its
  body position; do not turn a supported body into unsupported solely for syntax.
- Property access must have an actual Property/MethodSemantics relationship and a
  compatible signature, plus a source expression that preserves exact call target,
  dispatch, receiver and null behavior. No inference from `get_`/`set_` naming.
  Local metadata is available; external relationships need evidence that is not
  present in a lone MemberRef. If the Wait requirement needs external metadata,
  stop that slice and ask before adding any explicit metadata input or resolver.
- Post-decrement for the existing supported integer field/local shapes: one stable
  old-value snapshot, unchecked subtraction by one, write to the same location,
  then old-value use. Compact only without moving reads/writes/checks across other
  operations or reevaluating effectful receivers. Keep explicit `unchecked` when
  needed for ambient-context independence. Checked subtraction and patterns with
  extra effects or incompatible uses remain explicit statements.
- Generic/value-type/EH and other existing unsupported body semantics do not
  become supported merely because a sampled class contains them. Preserve the
  complete method IL inline, independent metadata and unchanged summary meanings.

## Random sample before implementation

`random-classes.json` records the frozen 20 distinct root selections, independent
of the fixed Wait regression. Populations come from the static PEReader oracle's
`--list-types` output for the existing pinned OpenRA.Game and OpenRA.Mods.Common
assemblies. The entire TypeDef population is saved under
`report/openra/random-classes/population/`, including excluded type kinds.

Eligibility is `kind=class`, excluding only the separately required Wait class.
Interfaces, value types/enums and the module pseudo-type are not classes in this
sampling rule. Empty, abstract, static, delegate, generic, nested and generated
classes are not filtered. A nested selection remains eligible even when a parent
is also sampled; recursive inventories explicitly identify any overlap rather
than counting it as additional independently sampled evidence.

A single OS-generated 128-bit seed ranks every eligible identity by SHA-256;
the first 20 distinct identities are frozen without replacement. This is
reproducible across Python versions and does not consult decompiler results.
The seed and exact algorithm are in the manifest. No resampling after failures.
Input hashes/provenance are reused from `manifest.json`; this does not acquire or
execute a game. These two assemblies share a project and do not establish
cross-project compatibility.

Reproduce the population (after rebuilding the optional oracle with
`python3 rust/acceptance/openra/inventory.py`):

```sh
mkdir -p rust/report/openra/random-classes/population
for assembly in OpenRA.Game OpenRA.Mods.Common; do
  "${DOTNET_ROOT:-$HOME/.dotnet10}/dotnet" \
    rust/report/openra/class-inventory/ClassInventory.dll \
    "rust/report/openra/assemblies/$assembly.dll" --list-types \
    > "rust/report/openra/random-classes/population/$assembly.json"
done
python3 rust/acceptance/openra/sample_classes.py \
  --input OpenRA.Game.dll=rust/report/openra/random-classes/population/OpenRA.Game.json \
  --input OpenRA.Mods.Common.dll=rust/report/openra/random-classes/population/OpenRA.Mods.Common.json \
  --exclude OpenRA.Mods.Common.Activities.Wait \
  --seed a851d53c66e12a7c1282d0c93cfaeca8 --baseline-ref 39bd383ff \
  --output /tmp/openra-random-classes-reproduced.json
```

The sampler refuses to overwrite a manifest. Compare the reproduced JSON with
`random-classes.json`; do not write a replacement acceptance sample.

## Captured baseline

Before production changes, all 20 selections were run through the Release CLI in
both styles and the independent class inventory oracle. Baseline details match
the independent member inventories; readable/details summaries agree. Original
method disassemblies and both views are retained under
`report/openra/random-classes/baseline/`, with per-class results in `report.json`.
These are static baseline checks, not semantic acceptance or method execution.
Generated and iterator classes are retained in the sample despite fallback bodies.

## Constructor implementation evidence

`d84d07c1c` admits local direct-base initialization; `018a08360` adds scoped external
base calls without a resolver. Public negative cases retain ordinary-call,
unrelated-owner, null/changed-receiver and `callvirt` rejection. Wait no longer
falls back on its base constructor. This is exact initialization notation, not a
claim that the external constructor implementation was recovered.

The actual readable Wait constructor/Tick gate passes 242 comparisons, including
fresh/dirty states, both overloads, base fields, null failures and subsequent Tick.
Deleting its base calls is rejected by the independent base-state comparison.
Owned local/external base fixtures pass 48 comparisons per Debug/Release config,
including pre-base field writes, ordered effectful parameters and base exceptions.
Core/delegated constructor gates remain passing; full Rust Debug/Release and
strict clippy passed after the admission changes.

Readable initializer recovery uses an explicit `: base(...)` or `: this(...)` only
when the exact initialization is the first rendered operation and arguments are
unchanged parameters of the exact target types or admitted typed literals. Even
parameterless `base()` remains explicit; it is not silently dropped. Earlier field
initializers, allocations, calls, labels or other statements prevent the rewrite.
This preserves unsupported source-binding cases as ordered intrinsic statements.
The test adapter expands the initializer's actual arguments at body entry, using
call-token comments only for target identity, never copying detailed statements.
The Wait, owned base, core and delegated gates pass with these actual initializer
outputs. Logs are under `report/constructor-presentation-*.log`.

## Evidence required before completion

- Owned red/green public-interface tests for each support slice, including
  adversarial receiver/dispatch/name/side-effect cases.
- Each frozen class: full actual readable/details CLI output, independent member
  inventory and original IL comparison, explicit in-scope defect versus remaining
  unsupported-body classification. Preserve all 20 even if most bodies fall back.
- Execute applicable in-scope methods from actual readable class output with an
  original/recovered comparison. Label every class's execution status and reason;
  static inspection is not behavioral proof. Owned fixtures cover absent shapes
  but do not replace randomly selected classes.
- Fixed Wait: both constructors and Tick must be supported; compare fresh
  initialization and subsequent state/exception behavior. Exact independent base
  constructor/accessor adapters are allowed, selected-body proxies or copying
  original initialized state are not.
- Relevant Rust Debug/Release, fmt, strict clippy, actual-body gates, truthful
  examples and per-class limitations. This document is a contract, not a claim
  that implementation or final acceptance is already complete.
