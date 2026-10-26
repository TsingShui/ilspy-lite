# Ordinary constrained dispatch and reference control flow

## Fixed-scope result

Baseline local commit `7f0a225f4`: 492,717 Decompiled / 278,031 Partial of
770,748 bodies. Final: **496,379 Decompiled / 274,369 Partial**, aggregate
fallback **35.597756984124516%**, still above the aggregate 5% goal.
**3,662** methods recovered in 125 images, zero recovered-method losses and zero
incomplete images. Scope remains 614 implementation inputs / 319 body images;
no reference-selection or denominator changes. Decrease: 0.475122867655834 points.

| image | Decompiled | Partial | gained |
|---|---:|---:|---:|
| OpenRA.Game | 2,668 | 2,099 | 26 |
| OpenRA.Mods.Common | 11,228 | 7,179 | 29 |

The previous first blockers of gained methods were constrained prefixes (1,954),
stack joins (1,054), address CFG edges (461), and local signatures (193). These
are attribution labels, not isolated causal estimates or forecasts of remaining
bucket recovery. Complete per-image results stay in the fixed-inventory scan.

Evidence under `report/value-dispatch/`: `AUDIT.md`, `audit/manifest.json`, red and
green logs, `corpus-scan/baseline.json`, `delta.json`, `behavior-manifest.json`,
`milestone.json`, and `review/{manifest.json,review-source.md}`. Large raw scans
and source/IL triples are retained locally; selected manifests/docs are versioned.

## Semantics and admission

`__cil_constrained_TYPE_METHOD` is the original adjacent constrained/callvirt
operation, with both tokens, checked signature and exact managed receiver
location. Runtime behavior includes interface/virtual dispatch, value boxing
fallback and reference-slot dereference. No copied/boxed receiver or guessed
implementation replaces it. Argument snapshots precede the operation exactly as
in IL; compiler-authored receiver reference snapshots remain separate from the
acquired location. GVM keeps its independently reproduced runtime lookup-order
refusal. Byref call results and constructor/intrinsic targets stay gated.

A bounded structural pass validates prefix adjacency and type-token kind. CFG
and EH boundaries cannot enter between prefix and call. Unsupported companion
prefixes are not silently discarded. Owned negatives mutate opcode, type token,
branch entry, generic scope, local element identity and unmanaged-pointer kind.

Managed-address stack joins require identical element identities. Parallel edge
transfers use ref-bound temporaries and rebind only synthetic scratch refs.
Scoped scratch refs start at the pure zero managed-pointer constant
`Unsafe.NullRef<T>()`; reachable predecessor/stack checks require incoming writes
before reads. Location acquisition still performs original field/array checks.
No contents read or allocation substitutes for the acquired address.

Unpinned/unmodified byref IL locals use scoped reference slots. `stloc` rebinds
the same typed location; it does not write its contents. Original argument-slot
rebinding, byref return/result escapes, pinned/modified/unmanaged locals and
address-of-address operations remain gated. Caller/IL-local storage lifetimes
remain within the method; no arbitrary escape admission is implied.

Null/reference joins merge only Null with the exact reference identity already
present at the join. Successor dataflow is reprocessed after this monotone change.
Destination-typed edge values preserve null rather than inserting object casts,
boxing or a guessed common base. Other incompatible joins remain fallback.

## Validation

- **418 tests**, fmt/clippy green; seven public constrained/control-flow tests.
- **24 owned behavior drivers** pass, with their meaningful mutation controls.
- New default-output gate: **720 comparisons / 40 closed selections per config**,
  five rejected mutants (receiver copy, argument trace, slot snapshot, joined
  location, array-storage copy). It covers declaring-type and method generic
  scopes, primitive/nullable/reference/struct receivers, derived dispatch,
  null/throw paths, ref-local/edge aliases and array acquisition checks.
- Local Ledger argument/replacement support uses recovered bodies. Exact runtime
  leaves emit only constrained + callvirt, never a selected method-body proxy.
- Legacy managed-location promotions execute across ordinary/initonly/raw-Boolean
  variants: 1,059 comparisons / 48 selections per config/variant. The generic
  null-conditional allocation-fixture case runs in both configs (51 / 15 total);
  virtual fixture runs 61 / 12 plus the existing 9 / 2 receiver/freshness gate.
  Frozen negative entries were updated only after those execution gates passed.
- Ten production default/ILSpy/IL triples were self-reviewed: ten
  CRUDE-BUT-EQUIVALENT, no changed-semantic SUSPECT in the bounded sample.
  This is main-session self-review, not independent/formal or whole-corpus
  behavioral verification. No third-party assembly is executed.

CVec.Minus remains Partial at the foreign constructor. Its ILSpy unknown-result
diagnostic remains inconclusive and does not support scope exclusion or recovery.
