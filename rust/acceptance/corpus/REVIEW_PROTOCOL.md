# Per-milestone independent source review

Every capability milestone runs this before its recovery counts are treated as
final. It is a goal constraint, not an optional extra.

## Trigger and shape

- Starts after the capability's owned E2E and behavior gate pass, and before the
  milestone numbers are written into a report as final.
- Runs as an independent, read-only, async subagent: it must not block the next
  slice, must not edit `rust/src`, and must not execute any third-party assembly.
- Use an exec-capable agent (`delegate`, `scout` or `worker`) and require read-only
  behaviour except for its own report file. The builtin `reviewer` has no shell
  (`tools: read, grep, find, ls, watchdog_diff, contact_supervisor`), so it can only
  judge pre-dumped files; prefer the exec-capable agents and keep `reviewer` as a
  fallback when dumps are already on disk.
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

- EQUIVALENT / CRUDE-BUT-EQUIVALENT / SUSPECT, per sampled method.
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
`--no-auto-references`. Dump source and IL to files before delegating to a
reviewer that may have no shell tool.
