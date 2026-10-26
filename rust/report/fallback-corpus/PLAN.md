# Slice plan and current position

Authoritative numbers and the ordered queue. Update this file when a milestone lands;
`CHECKPOINT.md` carries the operational state.

## Current position (grouped reference repairs; aggregate 5% target)

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
