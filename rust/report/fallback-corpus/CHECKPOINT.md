# Fixed corpus checkpoint

> The current verified section below is authoritative. `HANDOFF.md` retains
> historical transfer notes; its older working-tree state is not the live checkpoint.


Goal: mvouhw7y-679jgb

## Current verified state: ordinary local value construction

- `report/local-value-construction/corpus-scan/baseline.json`: **566,419
  Decompiled / 204,329 Partial of 770,748**, fallback **26.510480727812464%**.
  **+7,662 in 102 images**, zero regressions/incomplete images; fixed scope,
  inputs, hashes, exclusions and exact reference stamps.
  Game **3,113/1,654 (+215)**; Common **12,129/6,278 (+14)**. <=5% unmet.
- metadata/value_calls.rs recognizes ordinary local nongeneric MethodDef
  constructors with declaration-scoped parameters. Existing generic newobj
  remains. Direct MethodDef value constructor calls initialize any exact
  acquired value cell, not only original-this in a reference constructor.
  newobj uses separate completed storage; call .ctor adds no zeroing/copy.
- **453 tests**, fmt/clippy and **nine relevant owned drivers** pass. Existing
  value gate **585 comparisons / 65 closures/config**, default and authored
  acquired-cell variants; **16 / 18 compiled semantic mutants**. Readonly store
  adapter is one exact stfld; constructors/argument support execute recovered
  text. Ref arguments and destination aliases are covered.
- Legacy promotions execute first: struct receiver **43/21** with recovered
  Point ctor support; new-object **54/16** with one exact Holder newobj.
  First full regression retained 2 promotion failures (451/453); final is green.
  No blanket test/behavior matrix expansion. Prior semantic red/negatives kept.
- Zeroing mutant first survived because an overbroad selector picked a byref
  overload; corrected selection compiles and fails acquired-cell observation.
  Do not misreport that authoring issue as a production defect or initial reject.
- Seven audit triples, eight gained self-reviewed source/IL triples. All review
  runs succeed without unknown-result diagnostics; eight CRUDE-BUT-EQUIVALENT,
  no changed-semantic SUSPECT in bounded sample. Foreign CVec.Minus remains
  independent/inconclusive in ILSpy and is not claimed.
- Docs/evidence: `acceptance/corpus/LOCAL_VALUE_CONSTRUCTION.md`;
  `report/local-value-construction/{AUDIT.md,REDS.md,behavior-manifest.json,
  milestone.json,review/}`. No third-party execution, agents or push.
- **Latest user direction:** prioritize fast functional work, reduce defensive/
  duplicate checks and stop expanding test maintenance/matrices. User explicitly
  requested deletion of the TDD clause from AGENTS.md; it is deleted, not marked
  optional. This session still follows already-loaded higher-priority constraints,
  but future loaded repository instructions should reflect the actual file.
  Keep necessary semantic identity/order checks; do not fabricate successful
  validation or erase failures.
- Next high-yield work: exact selected foreign nominal/constructed value proofs,
  then foreign value calls/construction and remaining assignment relationships.
  Existing code has external_value_defs/cache plumbing; investigate why usable
  selected definitions are not providing facts rather than weakening reference
  policy or guessing enum/struct layout. Prior value-metadata candidate triples
  cover StreamingContext/KeyValuePair. Ordinary nominal MemberRef constructor
  forms and generic direct constructor calls remain separately gated.
  Next baseline is the local-value-construction scan. Continue autonomously.

## Previous verified state: enum storage encodings and narrow conversions

- `report/value-metadata/corpus-scan-final/baseline.json`: **558,757
  Decompiled / 211,991 Partial of 770,748**, fallback **27.504579966474125%**.
  **+4,588 in 101 images**, zero losses/incomplete; fixed inputs, exclusions,
  reference stamps and denominator. Game **2,898/1,869 (+13)**; Common
  **12,115/6,292 (+12)**. Aggregate <=5% is unmet.
- Captured real encodings: F# assembly-visible value__ (0x0603) and generated
  public/specialname __value (0x0206); ordinary public value__ (0x0606) retained.
  Matcher checks exact name/flag pairs, one instance fixed-width field, sealed
  nongeneric exact-core enum, no modifiers/marshal/readonly storage. No arbitrary
  field spelling, visibility or metadata relaxation. Foreign binding shares proof.
- conv.i1/u1/i2/u2 preserve low bits then sign/zero extension to I4 for
  I4/I8/native sources. Float/reference and other conversion gates remain.
  Public reds: assembly storage, generated storage, I4 narrowing, wide/native
  narrowing. New negative flag/name/source-category tests preserve boundaries.
- **449 tests**, fmt/clippy and **38 owned driver invocations** pass.
  Per config, each ordinary/assembly/generated enum variant executes **3,480
  comparisons**, eight widths, four promoted enum narrowings and twelve observed
  I4/I8/native conversions; **13 compiled semantic mutants**. Patched-owned runtime
  probes validate exact field encodings before production admission.
  Both foreign variants execute **30/9 methods/config**, three prior mutants.
  Four frozen narrow rows execute before removal. No selected-body proxy.
- First complete group scan retained **557,494/213,254**, +3,325; its remaining
  enum failures exposed the generated __value variant. Extension remains the same
  semantic group. Final scan has a separate path; earlier evidence is retained.
  `delta-final.json` has no regressions. No gate based on first-error forecasts.
- Audit: six candidate triples + four generated-layout triples and exact type IL.
  Additional captures use the intermediate release before generated-shape
  admission. Failed guessed type lookup is inconclusive, not BOTH_NO.
  Fourteen gained production triples self-reviewed CRUDE-BUT-EQUIVALENT, no
  changed-semantic SUSPECT in bounded sample; no ILSpy unknown-result diagnostics.
- Docs/evidence: `acceptance/corpus/ENUM_STORAGE_ACCESS.md`,
  `report/value-metadata/{AUDIT.md,REDS.md,milestone.json,behavior-manifest-final.json,
  review/}`. Raw scans/logs/IL remain ignored local evidence; selected manifests
  and docs travel with the local commit. No agents, push or third-party execution.
- Next: exact ordinary local value construction/receivers (already audited
  PlainTextOutput.get_Location and DomRegion.get_Begin), then nominal/constructed
  foreign value proofs and ldftn/ldtoken foundations. Preserve GVM lookup order,
  exact reference policy, category/lifetime/unknown-fact gates. Other candidates
  in the audit are not claimed as gains. Continue grouped public red/green,
  owned/mutation tests, milestone scan, source/IL self-review and local commit.
  Next baseline is `report/value-metadata/corpus-scan-final/baseline.json`.

## Previous verified state: typed SZArray operations

- Baseline: `report/typed-arrays/corpus-scan/baseline.json`. **554,169
  Decompiled / 216,579 Partial of 770,748**, fallback **28.099845864017812%**.
  **+22,677 in 221 images**, zero losses/incomplete; decrease
  **2.942206791324788 points**. Game **2,885/1,882 (+128)**; Common
  **12,103/6,304 (+233)**. Fixed 614 images / 319 body images, input hashes,
  reference policy and denominator; delta.regressions is empty.
- **443 Rust tests**, fmt/clippy, **34 owned driver invocations** pass.
  New gate **3,040 comparisons / 95 closed selections/config**, plus recovered
  private argument helper; **13 compiled semantic mutants**. Both declaring
  and method variables execute independently in T[], U[] and Mixed<T,U>[].
- arrays.rs separates shape, element evaluation and exact operand/width proof.
  ldlen needs only SZArray shape; allocation retains exact token/context.
  Reference loads/stores preserve identity/covariance checks; exact tokenized
  whole-value/generic ldelem/stelem preserve copies. Exact ldelema preserves
  acquired cell, invariant runtime type checks and timing before argument effects.
  Scalar I1/U1/Bool, I2/U2/Char, I8/U8 and R4 retain widths/bits/rounding.
- Canonical allocation supports checked core primitive identity, local nominal
  kind/arity and original caller-scoped TypeSpecs, without requiring element
  evaluation/layout. Unknown foreign element kind and open generic definition
  allocation remain gated. int[]/uint[] representation stays unchanged.
- Owned default-output execution includes byte 2 Boolean arrays, float -0/payload
  NaN, null/bounds/type precedence, covariance versus invariant addresses,
  value snapshots, aliases, argument effects, zero initialization and repeated
  allocation freshness. Helpers emit individual original array operations;
  no selected body delegates to its original implementation.
- First full regression retained 6 frozen-promotion failures (434/440). All
  promoted cases executed before removal:
  vector references 128/26; enum arrays 32/8; scoped 1,480/270 closures/27 methods;
  receiver 41/20; virtual 64/13 plus prior 9/2 value-receiver cases, per config.
  Prior mutants remain; enum allocation length mutant also fails semantically.
- Audit: nine triples in `report/typed-arrays/{AUDIT.md,audit/manifest.json}`.
  Fourteen gained production triples self-reviewed CRUDE-BUT-EQUIVALENT, no
  changed-semantic SUSPECT in bounded sample; no explicit ILSpy unknown-result
  diagnostics. `review/review-source.md` records obligations. Original IL is
  authority. CecilLoader.HasDynamicAttribute and Util.RandomDistance remain
  Partial for unrelated cast/category gates; not claimed gains.
- Evidence: `acceptance/corpus/TYPED_ARRAYS.md`,
  `report/typed-arrays/{behavior-manifest.json,milestone.json,review/}`.
  Raw scans/triples/logs are ignored local evidence; selected manifests/docs
  travel with the local commit. No push or third-party execution.
- Deferred independently: native dimensions/indices/scalar elements, additional
  enum/wider primitive array aliases, covariance assignment proofs, R8 scalar
  math, multidimensional/modified/pinned/readonly-prefix/lifetime extensions.
  Generic opaque double closures are not R8 scalar math admission.
- Next: audit the largest remaining nominal/value/signature proof buckets and
  exact delegate/function-pointer/token operations; do not infer gains from
  first blockers or loosen reference policy. Preserve GVM lookup-order refusal.
  Continue grouped public red/green, owned behavior/mutations, milestone scan,
  main-session source/IL self-review and local commit. No agents/push.
  **Aggregate <=5% remains unmet**; next baseline is the typed-arrays scan.

## Previous verified state: generic and foreign static storage

- `report/static-fields/corpus-scan/baseline.json`: **531,492 Decompiled /
  239,256 Partial** of 770,748; fallback **31.0420526553426%**.
- **+9,808 in 112 images**, zero recovered-method losses/incomplete; decrease
  1.27253006170629 points. Fixed inputs/exclusions/references. Game 2,757/2,010
  (+3), Common 11,870/6,537 (+234). Aggregate <=5% remains unmet.
- New metadata/static_fields.rs owns exact load/store/address operations and
  closed storage/readonly proof. Local class/value kind, arity and exact stored
  operand/declaration signatures are checked. Foreign flags come only from
  selected definitions; no instance layout is needed for static storage.
- Six new public static-field tests. Literal storage admission was caught by an
  owned mutation, generic stores exposed an integer-only tail, and local value
  static owners exposed use of the class-only member binder. Readonly initializer
  mutation changes only the closed owner and must remain Partial. Exact missing/
  wrong/added references and signature-index forgeries are negative cases.
- **430 Rust tests**, fmt/clippy and **30 owned driver invocations** pass.
  New gate per config: **864 steady comparisons / 96 closed selections**,
  **20 thread-static comparisons**, **20 fresh-process first-touch sequences**
  with two calls each, **eight semantic mutants**. Mixed type/method scopes,
  nullable/reference-bearing copies, closed storage isolation, acquisition timing,
  initializer poison/one-time behavior and thread storage execute.
- Three legacy frozen promotions execute before removal: scoped static cache
  1,440/260 closures/26 methods per config; external allocation/static 26/9;
  real CLR initializer mirrors 13 state comparisons/9 closed types, including
  generic readonly Stamp. All retain semantic mutation controls.
- Nine pre-audit triples and twelve gained production source/ILSpy/IL triples
  retained. Self-review: twelve CRUDE-BUT-EQUIVALENT, no changed-semantic SUSPECT
  in bounded sample. Path.DirectorySeparatorChar / DBNull.Value audit methods
  remain Partial for unavailable exact foreign field facts, not claimed gains.
- Evidence: `acceptance/corpus/STATIC_STORAGE.md` and
  `report/static-fields/{AUDIT.md,REDS.md,behavior-manifest.json,milestone.json,review/}`.
  No third-party execution, independent/formal or whole-corpus equivalence claim.
  Local-only commit workflow; no push.
- Next: fresh audit of broader typed arrays/signatures and remaining value/
  constructor/receiver proof gaps. Preserve GVM, lifetime and exact reference
  policy; first-blocker counts are not forecasts.

## Previous verified state: exception identities and construction

- `report/exception-types/corpus-scan/baseline.json`: **521,684 Decompiled /
  249,064 Partial** of 770,748; fallback **32.31458271704889%**.
- **+25,305 in 213 images**, zero recovered-method losses/incomplete; decrease
  3.2831742670756265 points. Fixed inventory, exclusions and references.
  Game 2,754/2,013 (+86), Common 11,636/6,771 (+408). <=5% still unmet.
- Bounded checked System/System.IO exception names and core parent facts,
  signature-driven exact core constructors, local finite exception ancestry and
  typed local catch identities are implemented. Ordinary allocation stays an
  independent proof. Catch-object, generic/unknown foreign ancestry, filters/
  faults and GVM remain gated.
- Five public tests include red/green throw, constructor return subtyping,
  local exception ancestry and core namespace cases; malformed/counterfeit
  scope, constructor signature and unrelated local-base mutations stay Partial.
- **423 Rust tests**, fmt/clippy and **28 owned driver invocations** pass.
  New gate: **181 comparisons / 12 public methods plus 2 recovered private
  helpers per config**, five semantic mutants. Messages/inner/actual identities,
  freshness, argument trace/early throws, local handler ordering, rethrow and
  finally execute. All 21 core parent facts are checked against the owned
  target runtime. No selected method-body proxies or third-party execution.
- Legacy promoted constructor cases execute before negative-table removal:
  6 comparisons / 2 methods per config. Existing object-exception gate 259/config
  and lexical EH 39/8/config with four mutants still pass.
- Retained failures include initial constructor subtype/local proof reds,
  namespace red, wrong static storage in the first local constructor oracle and
  an unsupported command-line flag in an added legacy invocation. Correct
  original declared static storage is shared/reset for original/recovered runs;
  invocation corrected and rerun. No failure is counted as a passing comparison.
- Twelve production default/ILSpy/original-IL triples self-reviewed: one
  EQUIVALENT, eleven CRUDE-BUT-EQUIVALENT; no changed-semantic SUSPECT in sample.
  No independent/formal or whole-corpus equivalence claim.
- Evidence: `acceptance/corpus/EXCEPTION_IDENTITIES.md`,
  `report/exception-types/{AUDIT.md,behavior-manifest.json,milestone.json,review/}`.
  Validated group; local-only commit workflow, no push.
- Next foundation: fresh audit of generic/static/foreign fields and value
  initialization/constructor calls; broader typed arrays/signatures and foreign
  value facts remain substantial independent blockers. Keep scope/GVM gates.

## Previous verified state: constrained dispatch and reference control flow

- `report/value-dispatch/corpus-scan/baseline.json`: **496,379 Decompiled /
  274,369 Partial** of 770,748; aggregate fallback **35.597756984124516%**.
- +3,662 in 125 images, zero recovered losses/incomplete; decrease 0.475122867655834
  points. Fixed scope/reference policy. Game 2,668/2,099 (+26), Common
  11,228/7,179 (+29). First-blocker counts are not promised future gains.
- Exact adjacent constrained/callvirt leaves, strict prefix/CFG entry checks,
  invariant address-edge transfers, scoped byref local rebinding and narrow
  monotone Null/reference joins are implemented. GVM/escape/argument-slot/
  pinned/modified/unmanaged gates remain independent.
- Local red/green: constrained prefixes, managed-address CFG, byref locals and
  Debug null/result joins. Initial C# escape-analysis failure fixed with scoped
  scratch refs; wrong string-identity oracle fixed with contents plus repeat-call
  freshness checks. Raw failures retained, not dismissed.
- 418 tests, fmt/clippy, 24 owned drivers pass. New gate: 720 comparisons /
  40 closed selections per config, five semantic mutants. Type/method scopes,
  nulls, derived dispatch, receiver snapshots, same-cell joins, array checks and
  exception writes execute. Argument/Replace support bodies are recovered.
- Legacy promotions execute before negative-table updates: managed locations
  1,059/48 per config/ordinary-initonly-Boolean variant, new-object 51/15 per
  config, virtual 61/12 plus 9/2 direct receiver/freshness gate per config.
- Ten default-source/ILSpy/original-IL triples self-reviewed, all
  CRUDE-BUT-EQUIVALENT for changed semantics. No changed-semantic SUSPECT in the
  bounded sample, third-party execution, independent or formal claim.
- Local-only commit, no push. Goal <=5% still unmet. Evidence:
  `acceptance/corpus/CONSTRAINED_DISPATCH.md` and `report/value-dispatch/`.
- Next larger high-frequency foundation: value initialization/constructor calls,
  generic/static/foreign fields and exact exception creation/throw/catch types,
  after fresh production audit. Keep GVM refused and fixed input/reference policy.

## Previous verified state: constructed values and boxing

- `report/value-calls/corpus-scan/baseline.json`: **492,717 Decompiled /
  278,031 Partial** of 770,748; aggregate fallback **36.07287985178035%**.
- **+20,561 in 132 images**, zero recovered-method regressions/incomplete;
  decrease **2.667668291062708** points. Fixed scope/reference policy.
  OpenRA.Game 2,642/2,125 (+85), Common 11,199/7,208 (+83).
- Exact ordinary constructed-value calls, generic open-self value receivers,
  local instantiated fields, separate value newobj and scoped value/generic
  boxing/unbox.any are implemented. The class-only shared owner binder stays
  class-only; no incidental MethodSpec/static-field/value allocation promotions.
- Owned field-signature-index forgery exposed a real definition-vs-operand
  identity hole; red/green retained and fixed by comparing exact substituted
  stored identities. Shared-blob width and plain value-callvirt negatives remain.
- **411 tests**, fmt/clippy, **23 owned behavior drivers** pass. New value gate:
  **522 comparisons / 58 closed selections per config**, ten rejected mutants.
  Local value/constructor/argument support bodies are recovered, real receiver
  cells are shared, static ctor effects reset/compared, and failed construction
  does not overwrite the destination.
- Promoted frozen negatives execute before removal: generic Sum/Peek (8/7),
  generic declaring (30/13), scoped (1,410/250 closed/25 methods), struct copies
  (496), struct receivers (38/19), per config with meaningful mutation gates.
- Ten production triples self-reviewed, all CRUDE-BUT-EQUIVALENT for changed
  semantics; no changed-semantic SUSPECT in this bounded sample. CVec.Minus is
  not a claimed gain. No third-party execution or independent/formal claim.
- Evidence: `acceptance/corpus/CONSTRUCTED_VALUE_CALLS.md` and
  `report/value-calls/{AUDIT.md,behavior-manifest.json,milestone.json,review/}`.
- Local-only commit; no push. Aggregate <=5% still unmet. Next audit/design:
  ordinary constrained dispatch and generic/static/foreign field/value-layout
  gaps, then broader EH/signatures/arrays as justified by remaining blockers.
  Existing GVM runtime lookup-order refusal remains mandatory.

## Previous verified state: scoped reference relationships

- `report/reference-relations/corpus-scan-final/baseline.json`: **472,156
  Decompiled / 298,592 Partial** of 770,748; aggregate fallback **38.74055%**.
- +33,355 in 197 images, zero recovered-method regressions/incomplete; decrease
  4.32761 points. OpenRA.Game +84 (2,557/2,210), Common +1,402 (11,116/7,291).
- Scoped local/foreign base/interface proofs, variance with reference-argument
  checks, symmetric open-self identity, typed original overload operands and
  reference identity across declared foreign constructed bases are implemented.
- Initial 18 regressions retained in `corpus-scan/`, `delta.json` and three
  `regressions/` triples; owned null-formal red/green reproduced the cause.
  Final scan has zero, not hidden by net gains.
- 405 tests, fmt/clippy and 22 owned behavior drivers pass. New fixture:
  143 comparisons / 36 closed selections per configuration; four mutants each.
  Generic declaring-type runtime entry is retained for self-cast execution,
  including null receivers; a wrong static-carrier attempt is retained.
- Ten production source/IL triples self-reviewed: one EQUIVALENT, nine
  CRUDE-BUT-EQUIVALENT, zero changed-semantic SUSPECT. No third-party execution
  or independent/formal verification claim.
- Eleven pre-audit triples retained. CVec.Minus is inconclusive for ILSpy due to
  explicit unknown-result diagnostics. ActorIndex.Actors remains Partial in its
  audited context; neither is excluded or claimed recovered.
- Goal remains aggregate <=5%, not met. Scope/references fixed; no agents/push.
  Evidence: `acceptance/corpus/REFERENCE_RELATIONSHIPS.md` and
  `report/reference-relations/{AUDIT.md,DESIGN.md,review/review-source.md}`.
- Next larger foundation: constructed value members/constructors, broader
  boxing and ordinary constrained dispatch. The first three have production IL
  leads in this group's audit; capture owned red cases before implementation.
  GVM remains refused after its existing runtime lookup-order failure.

## Previous verified state: typed managed-location foundation

- `report/managed-locations/corpus-scan-validated/baseline.json`: **438,801
  Decompiled / 331,947 Partial** of 770,748; aggregate fallback **43.06816%**.
- +21,002 in 188 images, zero regressions/incomplete; decrease 2.72489 points.
  OpenRA.Game +159 (2,473/2,294), Common +226 (9,714/8,693).
- Typed location elements, ref/out parameters and call operands, scalar/reference/
  generic addresses, indirect operations and precise value receivers implemented.
  Initonly addresses remain exact runtime field-address leaves, not value copies.
- Found/fixed readable snapshot substitution and constructor this-slot exposure.
  Initial scan's 824 readonly-address regressions retained in `corpus-scan/`
  and `delta.json`; final scan has zero, not hidden by net gains.
- 399 tests, fmt/clippy pass. Twenty relevant owned behavior drivers pass.
  New location gate: release/debug 960/951 comparisons across ordinary/initonly/
  raw-Boolean variants, 5,733 total; 36 rejected mutants. The raw-byte-2 gate
  exposed and fixed Boolean truth-value normalization (also present in ILSpy).
  Promoted legacy byref/value receiver cases stay executed.
- Twelve production triples self-reviewed, all CRUDE-BUT-EQUIVALENT for changed
  semantics; no changed-semantic SUSPECT in sample. No independent/formal or
  third-party execution claim.
- Goal remains aggregate <=5%, not reached. Fixed scope/references; no agents,
  no unrequested push. Evidence: `acceptance/corpus/MANAGED_LOCATIONS.md` and
  `report/managed-locations/{AUDIT.md,review/review-source.md}`.
- Next larger foundation: characterize remaining receiver/constructed-value and
  generic assignment/boxing/constrained-call blockers before implementation.
  GVM lookup-order refusal stays intact. No gains forecast from bucket totals.

## Previous verified state: scoped reference-call foundation

- `report/reference-call-foundation/corpus-scan/baseline.json`: 417,799
  Decompiled / 352,949 Partial of 770,748; aggregate fallback 45.79305%.
- +30,189 in 226 images, zero regressions/incomplete. Decrease 3.91684 percentage
  points, not the hoped-for ten; no bucket-total recovery promises.
  OpenRA.Game +247 / Common +683.
- Exact external reference calls are separated from optional source proof;
  foreign constructed call signatures, qualified core Object/String/Array
  identities, reference TypeSpec casts and reference unbox.any are implemented.
  GVM, value/layout, allocation, byref and unproved inheritance gates remain.
- 393 tests; clippy/fmt green. Reference fixture: 107 comparisons / 13 methods per
  config/ordinary-or-unbox variant, five rejected mutants each. Expanded legacy
  properties/casts/equality/external/virtual/generic gates and MethodSpec pass.
- Eleven source/IL triples self-reviewed: no changed semantic differences found.
  Metadata-only core fixture never executed; no third-party code execution.
- Larger grouped development remains requested. Next foundational target:
  general typed managed locations/byref arguments and generic assignment.
  Preserve aliases, location acquisition time, null/bounds checks, writes on
  exception and address escape rules; audit before implementation.
- Current goal aggregate <=5%; fixed references/scope. No agents or unrequested
  pushes. Evidence: `acceptance/corpus/REFERENCE_CALL_FOUNDATION.md`,
  `report/reference-call-foundation/AUDIT.md` and `review/review-source.md`.
- User explicitly requests continued autonomous work overnight and an honest
  progress summary, not an unverified promise of reaching the goal by morning.

## Previous verified state: static/nonvirtual MethodSpec calls

- `report/method-instantiations/corpus-scan/baseline.json`: 387,610 Decompiled /
  383,138 Partial of 770,748; aggregate fallback 49.70989%.
- +12,104 in 132 images, zero regressions/incomplete; every gain formerly
  generic/indirect method reference. OpenRA.Game +86 / Common +128.
- Exact static/nonvirtual MethodSpecs and simultaneous owner/method substitution
  implemented. Generic virtual callvirt is refused after an owned null/argument
  ordering discrepancy; failing logs/fixture are preserved, no GVM gains claimed.
- 389 tests, clippy/fmt green. 122 MethodSpec comparisons / 13 selections per
  config, four mutants. Expanded allocation (40), virtual/count (51) and scoped
  reference (1,380) gates include promoted methods; generic binding remains green.
- Ten source/IL triples self-reviewed; no changed semantic differences found.
  No third-party execution, no independent/formal review claim.
- Current goal is aggregate <=5%. User requests larger foundational batches,
  ideally ~10 percentage points where genuine coverage permits; do not promise
  gains by summing blocker buckets. Next prioritize reusable generic assignment,
  foreign constructed calls and managed-location/byref capabilities, not only
  narrow admission tweaks. Preserve fixed scope/references and local commits.
- Evidence: `acceptance/corpus/METHOD_INSTANTIATIONS.md`,
  `report/method-instantiations/AUDIT.md`, `review/review-source.md`.

## Previous verified state: grouped generic member binding

- `report/generic-binding/corpus-scan-final/baseline.json`: 375,506 Decompiled /
  395,242 Partial of 770,748; overall fallback 51.28031%.
- +8,695 in 83 images, zero regressions/incomplete. First scan found 38
  identity-prefix base-constructor regressions; captured owned red case,
  fixed the exact proof and reran the full scan. Retain the initial scan evidence.
- Group: preserve caller scopes for member params/results, distinguish other
  constructed owners from self, qualify open-self registration, constructed
  local static calls with exact target-runtime semantics.
- 387 tests, clippy/fmt green. 240 owned comparisons / 60 closed method selections
  per release/debug; four semantic mutants rejected, including generic type
  initialization. Prior generic/abstract/reference behavior gates remain green.
- Eleven corpus triples self-reviewed against ILSpy and IL; no changed semantic
  differences found, no third-party execution. OpenRA.Game +2 / Common +1.
- Target remains aggregate <=5%, not per-image1%; fixed scope/references retained.
  Group repairs, no subagents, local validated commits only unless push requested.
- Next: audit MethodSpec, external constructed receivers/calls and generic
  assignments; avoid losing caller scope or collapsing different instantiations.
- Evidence: `acceptance/corpus/GENERIC_MEMBER_SCOPES.md`,
  `report/generic-binding/{AUDIT,DESIGN}.md` and `review/review-source.md`.

## Previous verified state: grouped reference checks and interface semantics

- Current user-approved acceptance: aggregate fallback <= 5% over the same fixed
  implementation scope; per-image rates remain reported, no per-image 1% gate.
- Latest: `report/reference-checks/corpus-scan/baseline.json`.
  366,811 Decompiled / 403,937 Partial of 770,748; fallback 52.40844%.
  Gain +23,266 in 119 images, zero regressions, zero incomplete scans.
- Group: abstract casts, qualified core String/Object checks, local nongeneric
  interface references, runtime casts/dispatch and proven InterfaceImpl assignment.
- 382 tests, fmt/clippy green. Per release/debug: 125 interface comparisons with
  four rejected mutants; 114 cast comparisons with five rejected mutants.
  Existing object behavior remains green. Fourteen static corpus triples
  self-reviewed; no changed semantic differences found, no third-party execution.
- OpenRA.Game +222, OpenRA.Mods.Common +186. Historical per-image 1% diagnostic
  remains 34/319 but is no longer the acceptance criterion.
- User requests grouped repairs, autonomous implementation/design within project
  constraints, no subagents and no unrequested pushes. Keep ordinary tests local;
  consolidate full regression/scans/commits at group completion.
- Next: audit remaining high-frequency receiver, generic call and member-binding
  blockers. Reference closure changes are not implied by this group.

## Previous verified state: abstract constructed member owners

- `report/abstract-members/corpus-scan/baseline.json`: 343,545 Decompiled /
  427,203 Partial of 770,748 (55.42707%), 34/319 passing, zero incomplete.
  Gain +4,751 in 50 images, zero regressions; all gains were first-blocked by
  the concrete-class requirement in constructed member lookup.
- Abstract member lookup is allowed, but abstract allocation remains refused by
  its separate guard. No dispatch, receiver or generic-substitution rule changed.
- 377 Rust tests; 22 owned behavior comparisons per configuration and two
  rejected dispatch mutants. Prior generic and allocation gates still pass.
- Four pre-fix ILSpy/ours/IL audits drove the design; eight post-fix triples
  self-reviewed with no semantic differences found for this admission change.
- No subagents, no third-party execution; commit only, no push.
- Evidence: `acceptance/corpus/ABSTRACT_CONSTRUCTED_MEMBERS.md`,
  `report/abstract-members/AUDIT.md`, `review/review-source.md`.
- Next: audit remaining receiver/generic-member differences before another repair.

## Previous verified state: external properties

- Latest scan: `report/external-properties/corpus-scan/baseline.json`.
  338,794 Decompiled / 431,954 Partial, 770,748 bodies, 56.04348% unrecovered,
  34/319 passing, zero incomplete. +2,704 in 73 images, zero regressions versus EH.
  Every gain formerly hit the special-name accessor blocker; Common gains 240.
- Metadata-proven external getters/setters render with a declared-owner cast;
  virtual dispatch and argument-before-null-check order are preserved. Direct
  calls stay exact intrinsics; events/indexers/interfaces/generic owners remain
  gated. No reference-selection changes.
- 374 tests, clippy warnings denied and fmt checks pass. Default readable owned
  property behavior: 98 comparisons and three rejected semantic mutants per
  release/debug configuration. Prior external calls and direct properties pass.
- Six pre-fix ILSpy/ours/IL samples drove the design; eight post-fix corpus triples
  self-reviewed, no property-semantic differences found. No subagents or third-
  party execution; this is self-review, not independent evidence.
- Evidence: `acceptance/corpus/EXTERNAL_PROPERTIES.md`,
  `report/external-properties/AUDIT.md`, `review/review-source.md`.
- Next repair still requires an ILSpy-first audit. Continue directly in the main
  session; commit validated slices, never push without explicit instruction.

## Previous verified state: bounded EH regions

- Work continues directly in the main session. No subagents; audit ILSpy before
  repairs; original IL is the correctness basis. Commit validated slices, do not
  push without an explicit request.
- `report/exception-regions/corpus-scan/baseline.json`: 336,090 Decompiled /
  434,658 Partial of 770,748 bodies (56.39431% unrecovered), 34/319 passing,
  0 incomplete. Compared with the external-field baseline: +1,191 in 119 images,
  zero regressions; every gain was previously blocked by EH/data sections.
- Actual lexical try/catch/finally replaces flat handler annotations. Filters,
  faults, custom catch types and unsupported layouts stay explicit fallbacks.
- 369 tests pass, clippy warnings denied, fmt clean. Default readable owned
  carriers: 39 comparisons per fixture configuration; four semantic mutations
  rejected for finally effects, handler result, rethrow and exception identity.
- Eleven corpus triples self-reviewed against ILSpy and original IL, no EH
  differences found. This is self-review, not independent verification; no
  third-party code executed.
- The original exception-object tests/fixtures are preserved; region work has
  separate fixtures. Scalar addresses are NOT collapsed into value snapshots.
- Current evidence: `acceptance/corpus/EXCEPTION_REGIONS.md`,
  `report/exception-regions/AUDIT.md`, `REPORT.md`, `review/review-source.md`.
  HANDOFF and sections below are historical and may contain superseded numbers.
- Next: audit the remaining external-call/value-receiver blockers before choosing
  the next small repair. Do not treat a failed audit command as ILSpy failure or
  silently change corpus denominators.

## Historical state (external facts, option C)

- Decision C is recorded in rust/acceptance/corpus/EXTERNAL_TYPE_FACTS.md and
  implemented: every scan now receives exactly the references its image declares, and
  the selection is recorded per image (EXTERNAL_REFERENCES_PLUMBING.md). Reference
  selection today: 610/614 images with a usable identity, 3,010 declared references,
  1,179 resolved to exactly one file, 1,399 unresolved, 432 ambiguous (never guessed).
- External enum storage landed (EXTERNAL_ENUM_STORAGE.md): storage width comes from
  the referenced definition; the defining assembly's own name is kept, so a foreign
  token is never resolved against the image being decompiled.
- External call dispatch landed (EXTERNAL_CALL_DISPATCH.md): instance methods declared
  in a referenced assembly are written on the receiver; interface, generic, static,
  constructor and accessor targets stay gated.
- Order-independence defect found by the enum review and fixed: enum candidates now
  also come from the signatures of the members a body calls, so each method primes
  what it needs. rust/report/external-enum/check-parity.py re-runs the CLI per token;
  6,599 changed statuses plus a 400-method Partial sample all reproduce on their own,
  0 mismatches.
- Current scan: report/external-call/corpus-scan/baseline.json -- 271,276 recovered /
  499,472 Partial (64.80354% unrecovered), 33/319 passing, 0 incomplete. +6,599 over
  the plumbing baseline (4,304 enum storage, 2,295 call dispatch) across 93 images, no
  image lost a method. Cumulative over the 140,537 anchor: 130,739.
- Value copies landed (VALUE_TYPE_COPIES.md): constructed value types (`Pair<int>`,
  `ImmutableArray<X>`) and external value types (referenced structs; enums keep their
  storage) are whole-value copies. Gain +3,165 and +7,889. A test caught that caching
  an unresolved reference as a negative fact changed blocker text; lookups are now
  three-state (Value / Refused / Unavailable) and Unavailable leaves output
  byte-identical. Current scan: report/external-value/corpus-scan-final -- 282,330
  recovered / 488,418 Partial (63.36935% unrecovered), 33/319, 0 incomplete; 11,054
  changed statuses all reproduce per token (0 mismatches). Review closed with 0
  SUSPECT (23 pairs: 17 EQUIVALENT + 6 CRUDE-BUT-EQUIVALENT).
- Constructed generic members landed (CONSTRUCTED_MEMBERS.md): open self
  instantiation, substituted field types, calls on constructed receivers, allocation
  through a constructed class, and inheritance from a constructed base. Current scan:
  report/constructed-members/corpus-scan -- 296,157 recovered / 474,591 Partial
  after demoting the SUSPECT shape: 296,145 recovered / 474,603 Partial
  (61.5770% unrecovered), +6,773 over the previous milestone, 33/319 passing, 0
  incomplete. `Derived<T>.Score` and its corpus peers now fall back with
  "base field access needs the constructed owner name" instead of printing an open
  generic cast; naming a constructed base needs a C# owner renderer.
  Scan final: report/constructed-members/corpus-scan-demoted/baseline.json.
  Re-review of the same 21 pairs after the demotion: 15 EQUIVALENT, 3 CRUDE, 0
  SUSPECT, 3 INSUFFICIENT-EVIDENCE (field-signature evidence gap in the IL dump plus
  two missing corpus dumps). Milestone counts are final at 296,145 / 474,603. Scan cost is 32.3s (6 processes x 2 threads,
  from 53.7s); per-image work is the critical path and each worker now owns its image
  parse and fact caches. Review returned over 21 pairs: 14 EQUIVALENT, 3 CRUDE, 1 SUSPECT, 1 INSUFFICIENT-EVIDENCE. The SUSPECT is real: a generic base field owner renders as `((Holder)this)` - open generic name, type argument erased. DO NOT count this milestone until the cast names the constructed base or is suppressed; fix and re-review the same pairs.
- Boxing core scalars landed (BOXING.md): `box`/`unbox.any` for bool/char/integer
  scalars, enums/structs/constructed/generic gated with one stable reason. Scan
  report/boxing/corpus-scan -- 296,978 / 473,770 (61.46886%), +833 over the demoted
  constructed-members baseline, 87 images gained, 0 lost. Review launched; counts
  provisional until it returns.
- Reviews closed with 0 SUSPECT: cast (two agents), external enum storage (scout, 9
  fixture + 8 corpus pairs; its one contradiction is the parity defect above),
  external call dispatch (scout, 9 fixture + 8 corpus pairs: 7 EQUIVALENT + 10
  CRUDE-BUT-EQUIVALENT). The call review's remark about an unhelpful interface reason
  was fixed by appending the concrete refusal to the stable blocker prefix; a final
  scan (report/external-call/corpus-scan-final) reproduced 271,276 / 499,472 exactly.
- Remaining blockers by first cause (latest scan): unsupported signature type 62.8k,
  interface/generic declaring type 60.4k, external enum storage 55.5k (deeper uses),
  exception/data sections 39.2k, unresolved/cyclic hierarchy 31.0k, allocation
  owner/constructor 48.8k, byref/modified parameter 18.3k.
- Next slices: external/interface cast targets (identity only, no hierarchy), then
  constructed generic member binding, then region-aware CFG + bounded EH. Every slice
  keeps fixture + frozen contract table + behaviour gate with mutation control +
  independent review + full scan.

## Current milestone

- Corpus, baseline, implementation-scope and design tasks are complete (4/9). The types task is current. Object references, qualified core roots, vector/generic reference signatures and scoped generic variable copies are implemented and validated. Exact local enum storage is also implemented. Whole local struct snapshots/copies and struct receivers/managed locations are also implemented. Constructed generic members, external value facts, allocation/dispatch and EH remain pending.
- Final independent inventory is inventory-final/inventory.json; final production status baseline is baseline-final/baseline.json.
- All 1,164,708 MethodDef records across 976 managed snapshots occur exactly once. All 1,098,128 CIL bodies are accounted for.
- Baseline: 146,752 recovered, 951,376 partial, 86.64% unrecovered; no incomplete image scans. Final method results match the first saved baseline exactly.
- Nine acceptance E2E checks pass. The full Rust regression, CLI E2E, clippy and format evidence is saved under this directory.
- No third-party corpus assembly or archived executable was run. Owned object fixtures and recovered carriers were behaviorally executed under the user's existing authorization.
- Preserve all preexisting external working-tree changes. Session production changes include rust/src/lib.rs, main.rs, ir.rs, metadata.rs and lift.rs; new object fixtures/tooling/tests and corpus acceptance files are also session work.

## Latest runtime-type-check milestone

- Baseline: rust/report/cast-roundtrip/corpus-scan/baseline.json; unchanged inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- castclass/isinst on a local concrete nongeneric class render as ((T)(object)value) and ((object)value as T): runtime check preserved, same reference on success, null passes through, castclass throws InvalidCastException, isinst yields null and never throws. External/interface targets and value operands stay gated.
- 55 Rust suites green, Clippy -D warnings clean; frozen tables refreshed where the first blocker moved.
- Behavior: 31 comparisons x Debug/Release x Details/Readable over null/base/derived/unrelated/string, results compared by identity relative to inputs; type-test-turned-throwing-cast mutant rejected in every configuration.
- Same 770,748 bodies: 264,677 recovered / 506,071 Partial (65.65972% unrecovered). 4,191 new recoveries in 109 inputs, zero recovered-status regressions, zero incomplete scans, 33/319 passing. Cumulative same-scope gain 129,643. Game +4 (1,520), Common +5 (5,974).
- Also closed: the second-opinion delegate review of the allocation milestone reported 24 methods, 18 EQUIVALENT + 6 CRUDE-BUT-EQUIVALENT, 0 SUSPECT.
- Report: rust/acceptance/corpus/CAST_SEMANTICS.md. Cast review closed by two independent agents, both 0 SUSPECT (scout over 8 owned dumps; delegate over 8 owned + 5 corpus). Counts are final.
- Behavior-driver infrastructure fix: all shared-harness drivers now derive their compile inputs from GenericReferenceRoundtrip.cs plus globbed *Behavior.cs instead of stale hand-listed files, and struct-receiver was pointing at an undefined lowercase variable. All ten drivers run green again: object 111, generic-reference 340, scoped 1350, enum 3032, struct-copy 492, struct-receiver 32, virtual-call 44, new-object 29, equality 149, cast 31 comparisons.
- Reference-equality review closed: 16 bodies, 11 EQUIVALENT + 5 CRUDE-BUT-EQUIVALENT, 0 SUSPECT. Allocation review closed twice (17-sample report plus the delegate second opinion: 24 bodies, 18 EQUIVALENT + 6 CRUDE-BUT-EQUIVALENT, 0 SUSPECT).
- Context handoff: everything needed to resume is this file plus rust/acceptance/corpus/ (REVIEW_PROTOCOL.md and one report per slice). Open gate: cast review only. The Rust suite is one test binary now: warm `cargo nextest run` is about 2.5s for all 356 tests, a rebuild after touching src/ is about 14s, and a single suite is selectable with `cargo nextest run -E 'test(<module>::)'` (or `cargo test --test all -- <module>::`); discovery_paths and signature_guard stay separate targets because they compile a src/ file into their own crate root; rust/report/compare-recovery-scans.py BEFORE AFTER OUT produces deltas; behavior drivers are rust/tests/NAME-roundtrip.py with --mutation-control and optional --readable; the scan uses acceptance/corpus/corpus.py baseline with the frozen inventory (exit 2 means complete but above threshold).
- Next: external/interface cast targets and constructed generic members, then region-aware CFG and bounded EH.

## Prior reference-equality milestone

- Baseline: rust/report/equality-roundtrip/corpus-scan/baseline.json; unchanged inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- Reference identity comparison: ceq/beq/bne over reference operands (identical static types or Null) lower to global::System.Object.ReferenceEquals, negated for bne; cgt.un against null lowers to a non-null identity test. Ordered reference comparisons, castclass, isinst and external dispatch keep their blockers, so a null test cannot smuggle in a cast or a call.
- 54 Rust suites green, Clippy -D warnings clean. Behavior: 137 comparisons x Debug/Release x Details/Readable over a matrix with null, two instances and two value-equal but distinct strings; value-equality mutant rejected in every configuration (only detectable thanks to those strings).
- Same 770,748 bodies: 260,486 recovered / 510,262 Partial (66.20348% unrecovered). 2,453 new recoveries in 124 inputs, zero recovered-status regressions, zero incomplete scans, 33/319 passing. Cumulative same-scope gain 125,452. Game +26 (1,516), Common +94 (5,969).
- Report: rust/acceptance/corpus/REFERENCE_EQUALITY.md. Review launched on the exec-capable delegate agent (per user instruction to use delegate/scout/worker rather than the shell-less reviewer).
- Next: external/struct/abstract/generic allocation and constructed generic members, then region-aware CFG + EH.

## Resume checklist (context-budget aware)

Closed gate: the independent source review returned **15 EQUIVALENT, 2
CRUDE-BUT-EQUIVALENT, 0 SUSPECT** over 17 samples (10 owned + 5 FSharp.Core + 2
OpenRA.Game), so the `newobj` counts are final. Verdicts:
rust/report/new-object-roundtrip/review-source.md. Process for future milestones:
rust/acceptance/corpus/REVIEW_PROTOCOL.md.

Reviewer tooling lesson: the builtin `reviewer` agent has no shell
(`tools: read, grep, find, ls, watchdog_diff, contact_supervisor`), so it cannot
run the CLI. Either pre-dump sources/IL to files, or use an exec-capable agent
such as `delegate` and instruct it to stay read-only except for its report file.

State at handoff: 258,033 recovered / 512,715 Partial out of 770,748 bodies
(66.52174%), 33/319 body-bearing targets passing, zero incomplete scans, cumulative
same-scope gain 122,999. Rust: 53 suites + Clippy -D warnings green.

Open blockers by size (first error): unsupported signature type 62.7k, generic /
interface declaring type 60.2k, external enum storage 59.2k, EH 39.2k, opaque
external callvirt 39.2k, general newobj 39.1k, unresolved hierarchy 30.3k.

Ordered next slices (each needs its own fixture, contract table, behavior driver,
review, then scan):
1. reference equality (`ceq` on references) -> unlocks `x == null` guards and
   `ReferenceEquals`; emit identity comparison, never an overloaded operator.
2. external/struct/abstract/generic allocation and constructed generic members.
3. region-aware CFG + bounded EH (`control` task); this is the largest structural
   change and needs the approved design's D5 work.

Operational notes that cost time when rediscovered:
- `il <asm> --method <token>` rejects `--no-auto-references`; `decompile` accepts it.
- Fixture contract tables are refreshed by regenerating rows from `list` + `scan`
  of the debug fixture; never hand-edit counts (there are none).
- Reviewers may lack a shell: dump sources/IL to files first.
- `cargo nextest run` full run is ~2.5s warm (356 tests, one merged binary); focused selection is `-E 'test(<module>::)'`.

## Latest local-allocation milestone

- Baseline: rust/report/new-object-roundtrip/corpus-scan/baseline.json; unchanged inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- newobj is admitted only for a local concrete nongeneric non-interface class constructor (MethodDef, .ctor, instance, non-abstract/virtual, void default-convention, provable ancestry). Emitted as __cil_newobj_TOKEN(args): arguments first, exactly one fresh identity, then the exact constructor in order. External/struct/abstract/generic/open constructors keep distinct blockers.
- 53 Rust suites green, Clippy -D warnings clean. Frozen expected-partial tables were refreshed where the first blocker legitimately moved (objects/scoped/generic/vector/struct/enum/virtual-call fixtures).
- Owned behavior: 29 comparisons x Debug/Release x Details/Readable comparing observable state (fields, argument/self aliasing, exception type) rather than identity, plus explicit checks that equal-content allocations are distinct instances and a repeated call is fresh. Cached-identity mutant rejected in every configuration.
- Same 770,748 bodies: 258,033 recovered / 512,715 Partial (66.52174% unrecovered). 6,072 new recoveries in 132 inputs, zero recovered-status regressions, zero incomplete scans, 33/319 body-bearing targets pass. Cumulative same-scope gain 122,999. Game +26 (1,490), Common +120 (5,875).
- New goal constraint in effect: an independent async read-only reviewer samples the default readable source against the original IL (ILSpy secondary only) before gains are treated as final. Dumps and artifacts: rust/report/new-object-roundtrip/dumps/; verdicts in review-source.md. SUSPECT items must be fixed or demoted.
- Report: rust/acceptance/corpus/LOCAL_ALLOCATION.md. Next: external/struct/abstract allocation and generic construction, then EH.

## Prior local virtual-dispatch milestone

- Baseline: rust/report/virtual-call-roundtrip/corpus-scan/baseline.json; unchanged inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- callvirt is admitted only when the declared owner is a locally verified class and the receiver is assignable by the existing exact class-chain rule. It renders __cil_callvirt_TOKEN with documented semantics (receiver/args evaluated once in order, receiver null check, then target-runtime dispatch), never the direct-call form. Value-type/interface/constrained/opaque-external receivers, constructor/intrinsic callvirt and newobj stay independently gated.
- Contract gained a third expected state: legitimately bodyless declarations (abstract members). The shared count-free contract now distinguishes recovered, known-unsupported with a stable reason, and declarations, with stale entries failing.
- Two existing conservative expectations were updated because the capability is now real: callvirt on a local class no longer falls back, and a patched call->callvirt must show dispatch rather than partial output. Both keep explicit assertions on the marker.
- 52 Rust suites green, Clippy -D warnings clean, rustfmt clean. Behavior: 44 comparisons x Debug/Release x Details/Readable (8 methods, 1-2 Shape args, null receivers), direct-dispatch mutant rejected in every configuration.
- Same 770,748 bodies: 251,961 recovered / 518,787 Partial (67.30955% unrecovered). 47,680 new recoveries in 151 inputs, zero recovered-status regressions, zero incomplete scans, 33/319 body-bearing targets pass. Cumulative same-scope gain 116,927. Game +92 (1,464), Common +199 (5,755).
- Report: rust/acceptance/corpus/VIRTUAL_DISPATCH.md. Next: opaque/external dispatch and interface receivers with dependency facts, then constructed generic members, allocation (newobj) and EH.

## Prior struct-receiver / managed-location milestone

- Current baseline: rust/report/struct-receiver-roundtrip/corpus-scan/baseline.json. Unchanged scan inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- Test approach changed on user request: all magic method counts and negative-sentinel counts are gone from the seven owned fixture suites. rust/tests/support/fixture_contract.rs requires every MethodDef to lower with identical admission in both views, every non-listed method to be Decompiled, every listed (owner, method, reason) to stay Partial with an un-drifted reason, and every table row to be non-stale. Promotions/regressions are explicit one-line table edits; the table immediately caught a real name/arity mismatch during generation.
- Every Decompiled fixture method is behaviorally verified (no separate allow-list), and newly promoted bodies (UnsupportedDefault/Member/New) were added to the value-copy harness rather than only losing a sentinel.
- New capability: Ty::ValueAddress plus I8/U8 address categories, ldarga/ldloca, initobj, ldflda (struct and integer fields), ldind.i4/i8, stind.i8, struct instance receivers as managed locations (`this` is caller storage), and explicit `ref` at struct instance call sites. Reading a field through a copied value stays allowed; writing through a copy, address escape/as-value storage, addresses crossing block edges, byref params/returns, ref structs and external/generic value types stay rejected.
- Evidence: 51 Rust suites green, all-target Clippy -D warnings clean, rustfmt clean, twelve corpus-tool E2E checks pass. Struct receiver behavior: 32 comparisons per configuration/view (16 methods), Debug+Release, Details+Readable, three dropped-store mutants rejected (SetX/Bump/Move). Struct-copy harness re-run with promoted methods in both views plus all prior mutants.
- Same 770,748 bodies: 204,281 recovered / 566,467 Partial (73.49575% unrecovered). 17,827 new recoveries in 138 inputs, zero previously recovered regressions, zero incomplete scans, still 33/319 body-bearing targets meet <=1%. Cumulative same-scope genuine gain 69,247. Game +179 (1,372 recovered); Common +24 (5,556).
- Report: rust/acceptance/corpus/STRUCT_RECEIVERS.md. Types task still incomplete; next are constructed generic member binding, external value facts, allocation/callvirt and EH.

## Prior whole-struct-copy milestone

- Current baseline: rust/report/struct-copy-roundtrip/corpus-scan/baseline.json. Same fixed inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- Ty::Value has a distinct stack category and image-isolated interned nominal TypeId. Whole loads/stores/arguments/locals/fields/direct-call boundaries/returns copy original typed values and preserve embedded references. No boxing, construction, host size/offset/packing or managed-address inference. Enum proof stays first, and exact value identities must match.
- Local admission checks exact core ValueType base, sealed/nongeneric shape, attributes and field signatures. Ref-like markers/lookalikes, unknown constructed attribute owners, modified/direct void/open/managed-address fields and special core scalars stay gated. External values and constructed generic values remain unsupported.
- Core base-reference proof now applies existing assembly-name case folding plus normalized key/culture/content-type facts; structural class/value checking uses that same proof. Reference-contract classification and all frozen exclusions are unchanged. Red/green lowercased System.Runtime and invalid void field regressions are recorded.
- Six Rust struct E2E/boundary checks pass: 63 positives across seven struct shapes, 15 independent unsupported cases, typed assignment and scan parity. 1,932 owned comparisons (483 x configurations/views) pass, with 24 helper/snapshot/reference/constructor/bit/static-state mutants rejected.
- Initial ARM64 .NET 10 observer failures were reproduced and debugged: native-crash-stack.log identifies DataMisalignedException in Reflection.FieldAccessor.SetValue when initializing Packed.Wide. Exact field-leaf IL observers/setup retain packed tests without invoking selected bodies; all behavior passed afterward, plus three consecutive complete repeat runs. No test was skipped. Native trace/failed logs remain evidence.
- Full Rust regression, Clippy -D warnings/rustfmt, eight readable-adapter checks and twelve corpus E2E pass. Enum, scoped generic, generic reference, vector and object behavior/mutants also pass with the shared observer. No third-party input code was executed.
- Fixed 770,748 bodies: 186,454 recovered / 584,294 Partial (75.80869% unrecovered). 4,561 new recoveries in 82 inputs, zero prior recovered-status regressions, zero incomplete scans. 33/319 body-bearing targets meet <=1%. Cumulative same-scope genuine gain 51,420.
- OpenRA.Game +222 (1,193 recovered / 3,574 Partial); Common +3 (5,532 / 12,875). Static probes retain CoreLib CancellationToken getter, Game MapSize getter and Common EditorBlit.Commit output without running them.
- Release incremental build 4.97s; full implementation scan about 15.6s. Evidence under rust/report/struct-copy-roundtrip/; report rust/acceptance/corpus/STRUCT_VALUE_COPIES.md.
- Next: bounded ldarga/ldloca managed locations and value receivers/field operations, keeping stable location identity, no address escape/block-join guesses, and explicit copy-versus-location behavior. Then constructed generic member binding/external facts and call/dispatch remain. The types task is not complete.

## Prior local-enum milestone

- Latest baseline: rust/report/enum-roundtrip/corpus-scan/baseline.json; unchanged scope inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- Ty::Enum carries exact local TypeDef plus one of eight verified signed/unsigned storage widths. Definition admission checks sealed/nongeneric shape, exact core Enum base via normalized identity, every owned field, unique ordinary public special value__, unmodified integer signature and literal static fields. Facts cache only per Image; local TypeRef aliases canonicalize through the existing resolver.
- Loads normalize enum storage to I4/I8 bits with proper sign/zero extension. Stores explicitly truncate and restore enum identity. Both views retain enum aliases and storage evidence; arguments/locals/fields/direct calls/returns use the same transfers. Existing integer/branch operations remain; narrow opcode conversions, external enum layouts, boxing, array allocation and value/address receivers stay gated.
- Six Rust enum E2E/boundary checks pass: 96 positives, 29 unsupported cases, raw shape/flags/storage/core-identity negatives, local alias and scan parity. 3,032 owned cases covering all eight widths x Debug/Release x Details/Readable = 12,128 comparisons total. Sixteen helper/sign/unsigned/static-state mutants are rejected.
- Full Rust regression, Clippy/rustfmt, eight adapter checks and twelve corpus E2E checks pass. Prior scoped/generic/vector/object behavior and controls pass. All execution remains owned only.
- Same 770,748 target bodies: 181,893 recovered / 588,855 Partial; 76.40046% aggregate unrecovered. 22,174 new recoveries in 160 inputs, zero recovered-status regressions, zero incomplete scans. Still 33/319 body-bearing images meet <=1%. Cumulative same-scope genuine gain 46,859.
- Game +57 (971 recovered/3,796 Partial); Common +137 (5,529/12,878). Static probes for CSharp Binder flag getter and both OpenRA enum getters are retained in enum-roundtrip/static-probes.json and .txt, without runtime-loading inputs.
- Release incremental build 4.73s; four-worker full scope scan about 14s. Report: rust/acceptance/corpus/ENUM_STORAGE.md. Current types task is not complete.
- Next bounded type slice: concrete value snapshots/copies without host-layout, boxing or managed-address inference; then constructed receiver/member and address facts. Keep the exact enum specialization before any broad value-type admission so primitive stack normalization is not lost.

## Prior generic-value-copy milestone

- Current baseline: rust/report/generic-value-roundtrip/corpus-scan/baseline.json. Continue scanning only rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- Added Ty::GenericValue with a separate StackKind. Bare VAR/MVAR uses its validated owner/position TypeId. Loads/locals/arguments/returns copy the original generic value; no boxing, reference/numeric classification or layout inference. Unknown T cannot accept null, I4 or a different variable. Both views retain __cil_generic aliases, canonical bytes, scope and body context.
- Scoped owned fixture expanded to 23 positives/seven unsupported cases. Seven Rust E2E/boundary checks pass. The prior pure generic identity is now an explicit positive; constructed member, generic receiver, boxing and allocation gates remain closed.
- 5,400 owned comparisons: 1,350 x Debug/Release x Details/Readable, 230 contexts per combination from ten substitutions (int/string/decimal/object/double/owned aggregate/enum/nullable/float/nint). Exact runtime types, copies/snapshots, reference fields, reachable state, float/double signed-zero/NaN bits and decimal representation match. Twenty-eight body/binding/snapshot/reference/float/single/decimal mutants fail.
- Prior generic/vector/object behavior profiles pass, as do full Rust regression, Clippy -D warnings, rustfmt, eight adapter checks and twelve corpus E2E. No third-party code was executed.
- Same 770,748 bodies: 159,719 recovered / 611,029 Partial (79.27740%). New 132 recoveries in 30 inputs, zero previously recovered regressions; 33/319 body-bearing targets pass; zero incomplete scans. Cumulative genuine gain 24,685. OpenRA unchanged (Game 914/3,853; Common 5,392/13,015).
- Release incremental build 4.65s; full target scan about 13s. Evidence/report: rust/report/generic-value-roundtrip/ and rust/acceptance/corpus/GENERIC_VALUE_COPIES.md. Scoped fixture/behavior drivers now run the expanded value matrix and save behavior to generic-value-roundtrip; old scoped-reference scan/evidence remains historical.
- Reusable ignored evidence comparator: rust/report/compare-recovery-scans.py BEFORE AFTER OUTPUT, requiring identical target ids/origins/body counts and MethodDef token sets. It does not regenerate inventory or rewrite prior statuses.
- Types task still incomplete. Next slices: exact local enum underlying storage and transfers, then concrete value/address and generic receiver/member facts, reusing approved architecture. Static gains from type signatures alone are modest because later operations remain blocked.

## Prior scoped-reference-variable milestone

- Baseline: rust/report/scoped-reference-roundtrip/corpus-scan/baseline.json; same inventory at rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- TypeContext validates generic owner/position/arity/flags. VAR/MVAR descriptors inside reference shapes carry their owner and position through canonical interning; closed keys do not pick up unused context. Generic methods and static generic owners can copy reference descriptors; bare T, constructed members and generic receivers remain gated.
- Six Rust E2E/boundary checks pass; 16 positive methods and eight negative shapes. Old open-reference/vector pure copies are explicit positive regressions. Both views keep canonical signatures and generic body context; scan/class/individual status and alias parity are checked.
- 1,392 owned comparisons: 348 cases x Debug/Release x Details/Readable, 64 type/method contexts per combination. Four body and four binding-swap mutants fail. Prior object/generic/vector gates pass. Eight readable-adapter checks (including a comment-punctuation fix), twelve corpus E2E, full Rust regression, Clippy and rustfmt pass. No original third-party execution.
- 72 new recovered methods in 16 inputs, zero recovered-status regressions. 159,587 recovered / 611,161 Partial out of unchanged 770,748 (79.29453% unrecovered). Passing body targets remain 33/319; zero incomplete scans. OpenRA unchanged. Cumulative same-scope gain 24,553. Later blockers, not bucket disappearances, govern gains.
- Report: rust/acceptance/corpus/SCOPED_REFERENCE_VARIABLES.md. Current types task still incomplete; next bounded capability is generic variable value copies with a distinct evaluation/storage category, no boxing or layout inference.

## Prior vector-reference milestone

- Current production baseline: rust/report/array-reference-roundtrip/corpus-scan/baseline.json. Same scope inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json.
- StructuralRef (renamed from GenericRef) now represents closed SZARRAY copies as well as closed generic references. int[]/uint[] forms and operations are unchanged. Byte/string/object/value/enum/generic/jagged elements remain descriptors, not element/layout/allocation/covariance semantics.
- Four array integration/E2E checks pass (21 positives, seven unsupported shapes, element identity and scan parity). The old pure object[] upcast is explicitly a positive test; its five other unsupported object-operation sentinels remain negative. All selected bodies keep ordinary admission checks.
- Shared generic-reference-roundtrip.py --arrays executes 103 cases x two configurations x two views = 412 comparisons; 16 mutants rejected (helper, element write, signed zero, struct-field reference). Prior 340 generic/eight mutants and 444 object/four mutants also pass. Full Rust regression, Clippy/rustfmt and twelve corpus E2E checks pass.
- 4,234 newly recovered methods in 171 images, zero previous recovery regressions. 159,515 recovered / 611,233 Partial out of 770,748 (79.30387% unrecovered). 33/319 body-bearing targets meet <=1%; no incomplete scans. Cumulative same-scope gain 24,481. Game +26 (914/3,853), Common +68 (5,392/13,015). No classification change or third-party execution.
- Release incremental build 5.16s; full target scan about 15s. Evidence and exact delta: rust/report/array-reference-roundtrip/. Report: rust/acceptance/corpus/ARRAY_REFERENCES.md.
- Next: owner/position-scoped VAR/MVAR descriptors inside references (List<T>, T[]), then value/address/member semantics; generic receivers and T stack values remain separately gated. The types task is not complete.

## Prior closed-generic reference milestone

- Current statuses: rust/report/generic-reference-roundtrip/corpus-scan/baseline.json. Continue using rust/report/core-type-roots/scope-policy/implementation-inventory.json; scope/exclusions and all original inputs are unchanged.
- Added rust/src/signature_types.rs: arena-isolated Copy TypeId handles backed by full canonical closed signature keys, preserving nominal token scope and every recursive argument. Alias hashes are naming only; key equality and collision checks cannot merge distinct types. Exact local TypeRefs use the existing module resolver. No new dependency resolver or CLR executor.
- New Ty::GenericRef accepts GenericInst class copies, null, object upcasts, locals, ordinary fields/returns and already-supported direct calls. Closed primitive/named/nested generic/vector arguments, including value arguments, are descriptors, not value-stack/layout support. Free VAR/MVAR, constructed fields/member dispatch, generic owner receivers, variance and EH remain unsupported.
- Raw semantic method/field/plain-local signatures now use checked pre-token walking, preserving suffix and full-RID rejection. Known local generic arity and class/value flags are checked. Copying references does not allocate wrappers or convert layouts.
- Public E2E red evidence is generic-reference-red.log. Seven Rust generic tests pass across Debug/Release; 15 positive owned methods, six unsupported sentinels, argument/nominal identity, arity/kind, RID and suffix boundaries. Identical display names with different TypeDefs remain distinct. Shared readable adapter has a red/green nested-comma case; seven adapter checks pass.
- Owned behavioral gate: 85 cases x Debug/Release x Details/Readable = 340 comparisons. Exact runtime parameter/return types, reference identity, reachable fields/collection/array contents, static state, enumeration observations and exception types match. Four helper and four same-contents/double-Reverse mutants are rejected. Only signature tokens/type and field definitions are borrowed; all selected bodies and helpers execute recovered statements.
- Full Rust regression, all-target Clippy -D warnings, rustfmt, twelve corpus E2E checks and 444 prior object behavior comparisons/four helper mutants pass. No original third-party input executed. Own compilation is cached; no full solution build.
- Same 614 managed/319 body-bearing targets and 770,748 bodies: recovered 150,791 -> 155,281; Partial 619,957 -> 615,467; aggregate 80.43576% -> 79.85321%. 4,490 newly recovered methods in 127 inputs; zero previously recovered status regressions; no incomplete scans. Passing body-bearing inputs remain 33/319. Cumulative same-scope gain is 20,247.
- OpenRA.Game +33 (888 recovered/3,879 Partial); Common +91 (5,324 recovered/13,083 Partial). The signature first-error bucket shrinks 226,683 -> 187,068, but only whole-method gains count. Full per-method delta retained at generic-reference-roundtrip/corpus-delta.json.
- Incremental Release build 4.83s, full implementation scan about 15s. Capability report: rust/acceptance/corpus/CLOSED_GENERIC_REFERENCES.md. The goal and types task remain incomplete.
- Next bounded type slice can reuse the interner for closed SZARRAY reference copies (all element descriptors, but no new element/value operations), preserving existing int[]/uint[] handling and unsupported pointer/modifier/open-variable gates. Then owner-scoped free variables, exact enum/value/address and member facts remain necessary. Do not ask again for approved D1-D6 architecture.

## Prior core-root milestone

- Current implementation inventory: rust/report/core-type-roots/scope-policy/implementation-inventory.json. Current production statuses: rust/report/core-type-roots/corpus-scan/baseline.json.
- Qualified local System.Object may terminate a null-base hierarchy only in an admitted full core definition scope. Core identity uses the existing normalization and is cached per parsed Image, never across inputs/MVIDs. Raw base coded indices are checked; noncanonical null tags, wrong identity/name/culture, cycles and generic roots remain rejected. Local ValueType/Enum descendants still cannot be reference receivers.
- Both Rust classification and the independent PEReader now recognize ReferenceAssemblyAttribute constructors defined as MethodDef in the same core image, with exact nongeneric owner/constructor, flags, signature and payload checks. A local lookalike in an ordinary image remains nonproof.
- Static full metadata reinspection changed only ids 400 (legacy mscorlib, already skipped by exception) and 725 (.NET 5 System.Runtime reference input). All input hashes, MVIDs and CIL denominators stayed unchanged. Current role evidence: core-type-roots/inventory/, role-delta.json and role-probe-400/725.txt. All 362 reference-only inputs now have direct markers; no pinned exception is needed by the current oracle.
- Corrected implementation scope: 614 managed snapshots, 319 with CIL bodies, 770,748 bodies. The 7,203 System.Runtime contract RVAs are excluded under the already approved policy; no new user scope question is needed.
- At this same corrected scope, the object step gained 5,503 bodies, not the initially measured 5,540: 37 earlier gains belonged to the newly recognized contract. Old evidence remains historical and is annotated. Classification gains are never counted as semantic fixes.
- before-core-fix.json projects the saved post-object baseline to current target ids without rescanning or rewriting method statuses. Core-root repair adds 10,254 recovered bodies in three real CoreLib inputs, with zero previously recovered status regressions. Final totals: 150,791 Decompiled, 619,957 Partial, 80.43576% aggregate; 33/319 body-bearing inputs meet <=1%; zero incomplete scans. Cumulative genuine gain versus the original baseline at this same corrected scope: 15,757.
- CoreLib gains: .NET Core 2.2 System.Private.CoreLib +2,461; Mono 2.6.4 mscorlib +3,297; Mono 5.23 mscorlib +4,496. Getter probes in all three changed from TypeDef out-of-range to supported output. No original third-party input was executed.
- Seven owned core metadata regression tests pass; twelve corpus E2E checks pass. Full Rust regression, clippy -D warnings and rustfmt passed. Existing object behavior was rerun against the new Release CLI: 444 comparisons and four rejected helper mutants, with unchanged owned compilation cached. Hand-authored fake core fixtures are static-only and never loaded into CLR.
- A final red/green case also rejects a marker constructor with GenericParam rows despite a nongeneric signature, in both Rust and the independent reader. Final Release role checks confirm genuine local marker / ordinary lookalike / generic-ctor results. This guard does not affect the scan body-admission path or change the frozen valid input roles.
- Release incremental build 5.47s; current implementation scan 15.57s. Evidence/notes: rust/acceptance/corpus/CORE_TYPE_ROOTS.md. The types task is not complete; structural/generic/value/address capabilities still need implementation.

## Confirmed scope revision (initial recorded basis)

- The user approved skipping reference-only inputs, with no separate metadata/API gate, and confirmed the revised goal through /goal-tweak.
- implementation-scope/implementation-baseline.json retains 615 managed snapshots, including 320 with implementation bodies, totaling 777,951 CIL bodies; 135,566 recovered and 642,385 partial (82.57%).
- 361 reference-only snapshots (360 verified markers and one pinned legacy mscorlib exception) are explicitly listed with evidence. Every one of the original 1,098 PE origins is retained.
- Original 1,098,128 CIL-marked RVAs reconcile exactly with 777,951 implementation bodies and 320,177 skipped reference RVAs. Classification alone recovers zero methods.
- The policy application used saved reports only and took 18 ms, without input acquisition, extraction, hashing, compilation or rescanning. Eleven acceptance E2E checks pass.
- The original raw-metric reports are historical evidence, not the revised acceptance denominator. Do not reopen reference-scope confirmation.

## Confirmed architecture and object milestone (historical basis)

The user explicitly replied "同意" to the concrete Rust IL/CFG + lift architecture direction (input-scoped TypeId, reused static binding, separated values/addresses, explicit calls/EH) and the first object-reference slice, then resumed. These decisions are recorded in rust/acceptance/corpus/DESIGN_PROPOSAL.md. Do not ask for the same approval again. Major deviations still need a decision; original third-party corpus execution still needs separate test/environment/risk approval.

- Added Ty::Object as an ordinary reference type; admitted null/copy/argument/local/return/field transfers and reference-to-object upcasts. Reference static stores use typed assignment checks. No boxing, general newobj/callvirt/generic/value/EH capability was admitted.
- Seven Rust object E2E/regression tests pass, including six unsupported sentinels and invalid integer/downcast/field-receiver cases.
- Nineteen owned positive methods run in Debug/Release and Details/Readable, 111 behavioral cases per combination (444 total). Identity, all relevant instance/static fields, arrays and exception types match. All four recovered-helper mutation controls reject the mutant.
- Full Rust regression and clippy/rustfmt passed; final focused object and behavioral gates were rerun after adding typed-static reference cases.
- Fixed-scope production scan: rust/report/object-roundtrip/corpus-scan/baseline.json. All 777,951 implementation bodies accounted for, zero incomplete inputs. 141,106 recovered / 636,845 partial (81.86184%); 33/320 body-bearing inputs meet <=1%.
- corpus-delta.json records 5,540 newly recovered methods across 173 images, zero regressions from previously recovered status. The first-error signature bucket fell by 27,044, but only 5,354 of those were recovered; later blockers are not counted as successes.
- OpenRA.Mods.Common: 5,233 recovered / 13,174 partial, 305 gained; signature reason 6,016 -> 5,230. OpenRA.Game: 855 recovered / 3,912 partial, 31 gained.
- Release incremental build: 4.75s. Implementation-corpus scan: 15.54s. Same frozen reference exclusions and saved independent inventory; no reacquisition or input rehash.
- Implementation inventory for future scans: implementation-scope/implementation-inventory.json. The scope tool now emits it so reference-only inputs are not rescanned.
- Acceptance summary: rust/acceptance/corpus/OBJECT_REFERENCES.md. The goal remains far from complete; do not complete the types task on object-only evidence.

## Evidence

- BASELINE.md and scope-contradiction.json: scope contradiction and per-image evidence.
- scope-options-comparison.json: diagnostic alternate group counts; no revised gate is applied.
- baseline-integrity.json and baseline-comparison.json: every MethodDef token exactly once; final production results equal the initial baseline.
- e2e-baseline-integrity-red.log / green.log: rejection of omitted declarations and extraction-directory symlink escapes.
- baseline-final.log: final all-image scan, 20.32 seconds. No per-method process launches or full solution build.

## Acquisition history

### Completed acquisition

- Checked out ILSpy-tests at 9668d49b9f71b40045e5417b55d20e330be95921.
- Submodule status is clean; the host gitlink is unchanged.
- Recorded all 865 tracked files, Git blob identities and acquired-file SHA-256 hashes in acquisition-manifest.json.
- Found 648 files beginning with MZ: 599 DLLs, 46 EXEs, two netmodules and one winmd.
- Listed all members of five NuGet archives in archive-listings.json. Their listings contain another 448 DLL/EXE entries; managed/native classification remains pending.
- No selected input, compiler, build script or installer from the corpus was executed.
- Reused recorded provenance and hashes for the two unchanged OpenRA inputs from rust/acceptance/openra/manifest.json.

## Remaining work

- Continue the approved types work from the implemented closed-generic interner: bounded array reference signatures, owner-scoped open generic types, then exact enum/value semantics and addresses, using relevant E2E and behavior gates. The null-base core-root defect has been repaired; do not repeat that investigation.
- Generic signature integration must keep Ty handles image-scoped, owner-scoped type/method variables distinct, reference/value categories separate, and all existing unsupported callvirt/newobj/EH gates honest. No need to request the already approved D1-D6 direction again; only real major deviations need another decision.
- Do not implement a whole-method original-IL proxy to reduce fallback counts or conflate reference-only classification gains with semantic gains.
- Separately authorized original-corpus behavior checks and final per-image closure.

Ordinary test-interface approval is no longer a blocker. The user requested agile E2E iteration; relevant red/green E2E cases satisfy repository test discipline without repeated tool-detail approval.
