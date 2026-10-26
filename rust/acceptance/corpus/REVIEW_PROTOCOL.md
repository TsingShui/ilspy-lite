# Per-milestone source review

Every capability milestone runs this before its recovery counts are treated as
final. The user has replaced subagent review with direct audit and review by the
main session. Record it as self-review, never as independent verification.

## Trigger and shape

- Starts after the capability's owned E2E and behavior gate pass, and before the
  milestone numbers are written into a report as final.
- The main session compares saved output triples itself. No subagents are launched.
  No third-party assembly is executed. Reports explicitly separate self-review
  from owned-fixture behavioral validation.
- Output: `rust/report/<slice>/review-source.md` plus whatever dumps it needs.

## Inputs

- Our own output in the **default readable source view**
  (`decompile <asm> --method <token> --no-auto-references`). `--details` is not
  the review input; details are consulted only when triaging a SUSPECT finding.
- The original method IL (`il <asm> --method <token>`) as the only correctness
  basis. ILSpy output is a secondary reference; "ILSpy writes it this way" is not
  evidence of correctness.
- Sample 8-15 newly recovered methods, stratified across small/large bodies,
  reference/value kinds, generics, and branchy shapes; include corpus inputs from
  the milestone delta, not only owned fixtures.

## Checklist (semantic, not cosmetic)

Evaluation order and side-effect position; null/bounds check presence and
placement; reference copy vs value copy vs deep copy and alias write-back;
sign/zero extension and truncation widths; dispatch vs direct call; allocation
count and identity reuse; constructor order; exception type and throw point;
operations that were dropped, merged, moved across a branch/loop, or duplicated.

## Verdicts and consequences

- EQUIVALENT / CRUDE-BUT-EQUIVALENT / SUSPECT / INSUFFICIENT-EVIDENCE,
  per sampled method. Missing evidence never counts as semantic equivalence.
- Crude is acceptable. Semantically different is not.
- Any SUSPECT finding means the affected shape does not count toward recovery
  gains: either fix it, or demote that shape back to fallback and restore the
  matching negative coverage, then rerun the gates and the scan.
- Triaging a SUSPECT requires re-reading `--details` evidence; never decide from
  source appearance alone.
- The sampling list and per-method verdicts are retained so the review can be
  re-checked; "no differences found" is a valid result and must list the sample.

## Boundary

This source-level review is a heuristic complement. It never replaces the owned
behavior gate (execute the original and the recovered body on the same state and
compare, with mutation controls that must fail). Reports list static review and
executed behavior as separate evidence.

## Tooling note

The `il` subcommand accepts exactly `il <asm> --method <token>`; it rejects
`--no-auto-references`. Retain source, ILSpy output and original IL together so
another session can reproduce each verdict.
