# Slice plan and current position

Authoritative numbers and the ordered queue. Update this file when a milestone lands;
`CHECKPOINT.md` carries the operational state.

## Current position (ordinary local value construction; aggregate 5% target)

- **566,419 Decompiled / 204,329 Partial; fallback 26.51048%**.
- +7,662 in 102 images; zero losses/incomplete; fixed scope/reference policy.
  Game +215 / Common +14. <=5% still unmet.
- Ordinary local MethodDef value newobj and acquired-cell constructor calls
  preserve separate completed values versus partial writes to existing storage.
- 453 tests, fmt/clippy and nine relevant existing owned drivers pass. Default/
  acquired-cell gates are 585 comparisons / 65 closures/config with 16/18 mutants.
  Two frozen new-object failures were retained, executed, then promoted.
- Eight gained source/IL triples self-reviewed crude but equivalent; no
  changed-semantic suspect or third-party execution.
- Next: selected foreign nominal/constructed value definition facts, then
  related foreign value members/construction and assignment proof gaps.
  Reduce test/defensive overhead per latest user direction; do not weaken exact
  identities, semantics, reference policy or scan completeness to lower counts.
  No agents or push. TDD clause removed from AGENTS.md by explicit user request.

## Previous position (enum storage/narrow conversions; aggregate 5% target)

- **558,757 Decompiled / 211,991 Partial; fallback 27.50458%**.
- +4,588 in 101 images, zero losses/incomplete; fixed scope/references.
  Game +13 / Common +12; aggregate <=5% still unmet.
- Exact captured public/assembly value__ and generated special __value shapes;
  fixed-width and nominal core proofs remain strict. Narrow integer operations
  preserve low bits and I4 extension for I4/I8/native inputs.
- 449 tests, fmt/clippy and 38 owned drivers pass. Each of three field variants
  executes 3,480 comparisons/config, thirteen mutants; foreign variants 30/9,
  three mutants. Four frozen narrow rows execute before promotion.
- Intermediate scan +3,325 is retained; final scan adds the separately captured
  legitimate generated shape. No losses hidden by net counts.
- Fourteen source/IL triples self-reviewed crude but equivalent; no changed-
  semantic suspect. No third-party execution or independent/formal claim.
- Next: ordinary local value construction/receivers already audited, then
  nominal/constructed foreign value proofs, exact ldftn/ldtoken foundations.
  Preserve GVM, lifetime/category and reference policy gates. Continue grouped
  work autonomously with no agents or unrequested push.

## Previous position (typed vectors; aggregate 5% target)

- **554,169 Decompiled / 216,579 Partial; fallback 28.09985%**.
- +22,677 in 221 images, zero losses/incomplete; fixed scope/references.
  Decrease 2.94221 points. Game +128 / Common +233.
- Exact SZArray allocation/whole-value operations and acquired element cells,
  reference checks and scalar widths preserve copies, timing and identities.
  Shape/length proof does not infer element layout or scalar R8 support.
- 443 tests, fmt/clippy and 34 owned invocations pass. New gate per config:
  3,040 comparisons / 95 closed selections plus recovered argument helper;
  thirteen semantic mutants. Frozen promotions execute before row removal.
- Fourteen gained source/IL triples self-reviewed crude but equivalent; no
  changed-semantic suspect in bounded sample. No third-party execution or
  independent/formal verification claim.
- Native dimensions/indices, new covariance assignment proofs, further scalar
  alias categories, multidimensional/prefix/lifetime extensions remain separate.
- Next: audit nominal/value/signature proof blockers and exact ldftn/ldtoken
  operations. Prioritize grouped foundations, not forecasts from bucket totals.
  Preserve GVM/ref-policy/unknown-fact gates. Continue autonomously, no agents
  or unrequested push; aggregate <=5% remains unmet.

## Previous position (static storage; aggregate 5% target)

- **531,492 Decompiled / 239,256 Partial; fallback 31.04205%**.
- +9,808 in 112 images, zero losses/incomplete; fixed scope/references.
  Decrease 1.27253 points. Game +3 / Common +234.
- Exact static load/store/address operations retain generic scopes, values,
  aliases, initializer context and thread-static storage; selected foreign
  definition facts remain distinct from instance layout and source spelling.
- 430 tests, fmt/clippy and 30 owned invocations pass. New gate per config:
  864/96 closed selections, 20 thread checks, 20 first-touch sequences, eight
  mutants; legacy promotions execute before frozen rows are removed.
- Twelve gained source/IL triples self-reviewed crude but equivalent; no
  changed-semantic suspect in the bounded sample. No third-party execution or
  independent/formal verification claim.
- Next: audit broad typed arrays/signatures and remaining value-constructor/
  receiver proof gaps. Preserve GVM/lifetime/unknown-fact/reference-policy gates.
  Continue autonomously, no agents or unrequested push; <=5% remains unmet.

## Previous position (exception identities/construction; aggregate 5% target)

- **521,684 Decompiled / 249,064 Partial; fallback 32.31458%**.
- +25,305 in 213 images, zero losses/incomplete; fixed scope/references.
  Decrease 3.28317 points. Game +86 / Common +408.
- Checked core exception constructors/parent relationships, local exception
  ancestry and typed catch identities preserve original tokens and objects.
  Catch-object, unknown foreign/generic exception facts and filters/faults remain.
- 423 tests, fmt/clippy and 28 owned driver invocations pass. New gate:
  181 comparisons / 12 public methods plus 2 recovered support helpers/config;
  five mutants. Legacy promoted constructor cases execute before table updates.
- Twelve source/IL triples self-reviewed: one equivalent, eleven crude but
  equivalent, no changed-semantic suspect. No third-party execution or formal/
  independent verification claim.
- Next: audit generic/static/foreign fields and value initialization/constructor
  calls, then broader typed arrays/signatures/foreign value proofs. Do not infer
  promised gains from first-blocker totals; preserve GVM/ref-policy boundaries.
- Continue autonomous grouped work, no agents/push; <=5% still unmet.

## Previous position (constrained/reference CFG; aggregate 5% target)

- **496,379 Decompiled / 274,369 Partial; fallback 35.59776%**.
- +3,662 in 125 images, zero losses/incomplete; fixed scope/references.
  Decrease 0.47512 points. Game +26 / Common +29.
- Exact constrained operations, invariant reference-edge/local transfers and
  narrow Null/reference merge propagation preserve original storage and timing.
- 418 tests, fmt/clippy and 24 owned drivers pass. New gate: 720 comparisons /
  40 closed selections per config; five mutants. Legacy promotions execute.
- Ten source/IL triples self-reviewed, no changed-semantic SUSPECT in sample.
  No third-party execution or independent/formal verification claim.
- Next: audit value initialization/constructor calls, generic/static/foreign
  fields and broader exact exception creation/throw/catch identities. Keep GVM,
  escape and unproved-reference gates; no forecasts from blocker totals.
- Continue autonomous grouped work, no agents/push; <=5% still unmet.

## Previous position (constructed values/boxing; aggregate 5% target)

- **492,717 Decompiled / 278,031 Partial; fallback 36.07288%**.
- +20,561 in 132 images, zero losses/incomplete, unchanged scope/references.
  Actual decrease 2.66767 points. OpenRA.Game +85 / Common +83.
- Ordinary exact value calls, structural open-self receivers/fields, separate
  value newobj and generic/value boxing/unbox.any are validated as one group.
  A field signature forgery was reproduced/fixed before the milestone.
- 411 tests, fmt/clippy and 23 owned drivers pass. New value gate: 522 comparisons
  / 58 closed selections per config; ten mutants. Legacy promotions execute.
- Ten source/IL triples self-reviewed, no changed-semantic SUSPECT in sample.
  Static coverage is not third-party execution or independent verification.
- Next: audit ordinary constrained dispatch and remaining generic/static/foreign
  field/value proof gaps. Preserve GVM refusal and address-CFG/lifetime gates.
- Continue autonomously in groups, no agents or unrequested push; <=5% unmet.

## Previous position (reference relationships; aggregate 5% target)

- **472,156 Decompiled / 298,592 Partial; fallback 38.74055%**.
- +33,355 in 197 images, zero regressions/incomplete; fixed scope/references.
  Actual decrease 4.32761 points, not a forecast from blocker totals.
- Instantiated relationship proofs, scoped self identity, metadata variance,
  caller-projected foreign ancestors and original overload operands are landed.
  Reference naming does not require proving all foreign ancestor details.
- 405 tests, fmt/clippy and 22 owned behavior drivers pass. New fixture executes
  143 comparisons / 36 closed selections per configuration; four mutants each.
- Initial 18 regressions retained/reproduced/fixed; ten production triples
  self-reviewed, with no changed-semantic SUSPECT in the bounded sample.
- Next: owned red tests and semantic binding for constructed value members,
  value constructors, broader boxing and ordinary constrained dispatch.
  Preserve whole-value copies/locations, exact owner arguments and allocation
  distinctions. GVM lookup-order failure remains an independent gate.
- Continue autonomous grouped work, no agents, third-party execution or
  unrequested pushes. Aggregate <=5% remains unmet.

## Previous position (managed locations; aggregate 5% target)

- **438,801 Decompiled / 331,947 Partial; fallback 43.06816%**.
- +21,002 in 188 images, zero regressions/incomplete; fixed scope/references.
  Actual decrease 2.72489 points. OpenRA.Game +159 / Common +226.
- Typed byref operands, storage operations, exact value receivers and initonly
  field-address leaves retain aliases, acquisition timing and exception writes.
- 399 tests, fmt/clippy and twenty owned behavior drivers pass. New location
  gate executes 5,733 comparisons across ordinary/initonly/raw-Boolean variants;
  36 mutants fail, including load/store normalization of observable byte value 2.
  Promoted legacy cases stay executable, with meaningful mutation controls.
- Twelve source/IL triples self-reviewed. Initial 824 readonly-address regressions
  retained, reproduced and repaired; the final scan has zero recovered losses.
- Next: audit remaining receiver/constructed-value and generic assignment,
  boxing/constrained-call foundations as a related group. Keep GVM refused until
  its independently reproduced runtime ordering problem is modeled.
- Continue autonomously; no agents, third-party execution or unrequested pushes.
  Goal not met; bucket counts are not promised recoveries.

## Previous position (reference-call foundation; aggregate 5% target)

- 417,799 Decompiled / 352,949 Partial; fallback 45.79305%.
- +30,189 in 226 images, zero regressions/incomplete, unchanged scope/references.
- Shared exact-call/scoped-signature binding, qualified core reference identity,
  reference TypeSpec conversions implemented separately from source syntax proofs.
- 393 tests and expanded behavior/mutation gates pass; eleven corpus triples
  self-reviewed. No third-party execution or independent verification claim.
- Actual group gain 3.91684 percentage points, not ten. Next larger infrastructure:
  typed managed locations and byref calls, then broader generic assignment.
  Capture aliases, exception-visible writes and acquisition checks in owned gates.
- Continue autonomously overnight as explicitly requested. Grouped development,
  audit before code, no subagents, local commits only unless push is requested.

## Previous position (MethodSpec calls; aggregate 5% target)

- 387,610 Decompiled / 383,138 Partial, fallback 49.70989%.
- +12,104 in 132 images, zero regressions/incomplete; no scope/reference changes.
- Static/nonvirtual generic calls and simultaneous Var/MVar substitution pass;
  generic virtual callvirt remains gated after a real owned ordering failure.
- 389 tests, owned MethodSpec/expanded legacy behavior and mutations pass;
  ten corpus triples self-reviewed.
- User requests larger batches with foundational leverage rather than narrow
  incremental fixes. Aim for substantial actual recovery via generic assignment,
  external constructed calls and typed managed-location/byref infrastructure.
  Ten percentage points means ~77,075 bodies; bucket totals are not forecasts.
- Keep audit before implementation, local red/green tests, consolidated full
  milestone gates, no subagents, no unrequested pushes.

## Previous position (generic binding group; aggregate 5% target)

- 375,506 Decompiled / 395,242 Partial, overall fallback 51.28031%.
- +8,695 in 83 images; final zero regressions/incomplete after resolving
  38 prefix-base constructor regressions found in the first full scan.
- 387 tests, 240 behavior comparisons / 60 closed selections per configuration,
  four rejected mutants; eleven static corpus triples self-reviewed.
- Preserve caller scope through member binding; foreign owner instantiations
  stay structural, exact constructed static calls preserve initialization.
- Next audit: MethodSpec calls, external constructed receivers and assignments.
  Keep grouped milestones, fixed input/reference policy, no agents or push.

## Previous position (grouped reference repairs; aggregate 5% target)

- 366,811 Decompiled / 403,937 Partial of 770,748; aggregate fallback 52.40844%.
- +23,266 in 119 images, zero regressions/incomplete; fixed references/scope.
- 382 tests, interface and cast behavior/mutations pass, fourteen corpus triples
  self-reviewed. No third-party execution.
- Complete related semantic repairs as groups: local red/green tests inside the
  group, one full regression/behavior/scan/self-review milestone and local commit.
- User revised acceptance to aggregate <= 5%; per-image results remain diagnostic.
  User authorizes implementation/design work; retain repository constraints.
- Continue on high-frequency receiver/generic/member gaps after ILSpy/IL audit.
  No subagents, no automatic push.

## Previous position (abstract constructed members)

- 343,545 Decompiled / 427,203 Partial (55.42707%), 34/319 passing.
- +4,751 in 50 images, zero regressions/incomplete. Abstract lookup is allowed;
  abstract allocation remains rejected. No reference policy change.
- 377 tests, 22 owned comparisons and two dispatch mutations per configuration;
  eight corpus triples self-reviewed. OpenRA.Game/Common each gain four.
- Audit first, then bounded repair. No subagents and no automatic push.

## Previous position (external properties)

- 338,794 Decompiled / 431,954 Partial (56.04348%); 34/319 images passing.
- +2,704 in 73 images, zero regressions/incomplete; every gain from the special-name
  accessor blocker. OpenRA.Mods.Common +240.
- 374 tests and 98 property behavior comparisons per configuration; three semantic
  mutants rejected. Main-session self-review completed over eight corpus triples.
- Next: audit the remaining value-receiver and generic-member differences against
  ILSpy, then select a bounded repair. No subagents, no automatic push.

## Previous position (bounded EH)

- 336,090 Decompiled / 434,658 Partial; 770,748 bodies, 56.39431% unrecovered.
- 34/319 body images pass; zero incomplete; +1,191 in 119 images, zero regressions.
- Evidence: `report/exception-regions/REPORT.md`; 369 tests, owned behavior
  and four mutation controls pass in both fixture configurations.
- Main-session audit, implementation and review only; no subagents.
- Audit remaining blockers against ILSpy before selecting a repair. Unsupported
  inputs remain recorded and denominators are not silently changed.
- Commit validated slices; push only on explicit request.

## Historical position (before bounded EH)

| quantity | value |
|---|---:|
| managed images / body images | 614 / 319 |
| bodies | 770,748 |
| Decompiled / Partial | **334,899 / 435,849** |
| unrecovered | **56.5386%** |
| body images within 1% | **33 / 319** |
| incomplete scans | 0 |
| recoveries still needed for every image to pass | see `CLOSURE.md` (recomputed per scan) |
| images needing <= 10 / 50 / 200 more | 71 / 111 / 176 |

Sources: `report/boxing/corpus-scan-enum/baseline.json`,
`report/boxing/VERIFICATION.md`, `PER-ASSEMBLY.md`, `CLOSURE.md`.

## Two measures, always together

1. **Absolute recovery**: Decompiled count and rate; how far the corpus moves.
2. **Gate distance**: per image, `unrecovered - bodies/100`; how close each assembly is
   to passing. Use `python3 report/per-assembly-gate.py <baseline.json>`.
   Pass counts are a lagging indicator and move late, in waves.

## Queue

| # | slice | expected bodies | notes |
|---|---|---:|---|
| 1 | static constructors (remove the `.cctor` blanket gate) | 13,335 rows, 236 images | landed: +1,386 direct, rest surfaces its real blocker |
| 2 | boxing scalar stack-category fix + enum unbox cast | 390 + all enum widths | landed, contract re-baselined |
| 3 | sealed-owner mask fix | 28,685 | landed: **+18,288** measured with the mask reverted as control |
| 4 | external allocation (concrete external owner) | 24,893 | landed: **+1,116**, 0 lost; constructed/nested owners still gated |
| 5 | external member facts (field read/write) | 24,893-ish | landed: **+686**, 0 lost; value-type/static owners still gated |
| 6 | property/event accessor binding | 1,500 first-blockers in 60 images, more overall | brief ready |
| 7 | external callvirt receiver type facts | 45 images, 11.4% median | design in `EXTERNAL_CALL_DISPATCH.md` |
| 8 | byref/modified parameters; uninitialized locals | ~19,300; 9 images at 15% | fillers between big slices |
| 9 | exception and data sections | 39,234; 50 images; 39 of the 49 small failing images | largest design, do after 3-6 |
| 10 | interface declaring types, then the long tail | 24,590 | interacts with 7 |

## Gates per slice

- Fixtures red before green, frozen `(owner, method, reason)` table with no count
  assertions, mutation control in the roundtrip driver, release and debug.
- Full corpus scan with `--references declared`, delta against the previous baseline:
  zero lost images/methods, every gained method's previous reason accounted for.
- Independent review against ILSpy output; SUSPECT is fixed or demoted before counting.
- Docs: `acceptance/corpus/<SLICE>.md`, README index, `CHECKPOINT.md` line, this table.

## Known gaps that are not slices yet

- Enum unbox cast and scalar stack-category defects (text correctness, no status change).
- Reference resolution for facade identities (`System.Runtime`, `mscorlib` 4.0.0.0):
  scout is checking whether reference-only inputs are usable as fact sources.
- `ILSpyUpdateAssemblyInfo` cannot run without `pwsh`; verification works around it by
  stamping `ICSharpCode.Decompiler/obj/update-assemblyinfo-last-commit-hash.txt`.

## New evidence (scout findings)

- `report/allocation-owner/FINDINGS.md` + `DEFECT-sealed-mask.md`: the allocation-owner
  check includes `TypeAttributes.Sealed`, so **every sealed class** is refused - 28,685
  bodies. Reject only `interface | abstract`; static classes stay refused through
  `abstract`. Split the catch-all string into precise reasons.
- `report/reference-resolution/FINDINGS.md`: 20,899 bodies fail with "type binding
  requires a resolved assembly", split as unresolved 11,219 / ambiguous 5,436 /
  transitive 4,244. Do **not** index the 362 skipped inputs (+14 ambiguity, -19
  unresolved, contradicts the approved scope). Same-major is partial and needs corpus/CLI
  rule alignment. Two real fixes: (a) deterministic, stamped ambiguity tie-break among
  identity-equal files; (b) one-hop reference closure (pass what the supplied references
  declare, one hop, same identity rules).
- `report/boxing/DEFECT-*.md`: enum unbox must push the normalized stack type and cast to
  the storage type; scalar box/unbox must compare stack category, not exact `Ty` (896
  bodies).

## Review status

- Boxing review: 13 pairs, 10 EQUIVALENT, 1 CRUDE-BUT-EQUIVALENT, 2 SUSPECT (both enum
  unbox, matching `DEFECT-enum-unbox-cast.md`). The milestone counts only after the
  defect sweep; the six narrow-width unbox rows must be re-baselined in the contract,
  not frozen as expected-`Partial`.
- Review artifacts are one revision behind the working tree (`dumps/` predates the
  enum admission); after any rebuild, regenerate `review/current/` before judging.

## Slice entry gate (mandatory)

Every slice now begins with a read-only audit that samples the target bucket and compares
our output with ILSpy's member by member. Only `ILSPY_YES_OURS_NO` classes are candidates;
`BOTH_NO` is explicitly out of scope and must not be implemented. The audit lands in
`report/<slice>/AUDIT.md` and the slice report cites it. Agents commit accepted slices and
do not push unless asked.
