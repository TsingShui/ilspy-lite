# Slice plan and current position

Authoritative numbers and the ordered queue. Update this file when a milestone lands;
`CHECKPOINT.md` carries the operational state.

## Position (after the boxing milestone)

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
