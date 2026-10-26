# Development workflow

The main session performs implementation, builds, scans, audits and documentation
itself. Do not launch subagents unless the user explicitly changes this instruction.

## Before a repair

Compare the production CLI's default readable output with ILSpy for the same
methods, and retain the original IL to settle semantic questions. Record assembly,
token, reference selection and the relevant output differences in
`report/<slice>/AUDIT.md` before choosing an implementation.

- `ILSPY_YES_OURS_NO`: candidate for a bounded repair.
- `BOTH_NO`: not a current implementation target. Preserve the input and failure;
  a sample failure does not justify excluding a whole bucket from the denominator.
- `BOTH_YES_DIFFERS`: inspect original IL and distinguish cosmetic from semantic
  differences before changing code.
- `OURS_YES_ILSPY_NO`: do not change a correct result just to resemble ILSpy.

A failed ILSpy command, missing SDK or unresolved audit input is inconclusive,
not evidence that ILSpy cannot decompile the method. ILSpy output guides priority;
original IL remains the semantic authority.

## Implementation and verification

1. Record the audited difference and a narrow design within the approved typed
   IL/CFG + lift architecture. Ask before a new major architectural decision.
2. Show a relevant failing test, implement, then run the focused regression and
   owned behavior gate with mutation controls. Keep unsupported shapes explicit.
3. At a capability boundary, run full relevant regressions and one full corpus
   scan using the same production build and frozen reference selections.
4. Perform the source review directly, recording methods, verdicts and limitations.
   This is self-review, not independent review. Do not claim otherwise.
5. Fix or demote SUSPECT shapes before counting gains. Report static evidence and
   executed owned-fixture behavior separately; never execute third-party assemblies
   without the user's approval.
6. Commit each validated slice. Do not push unless the user explicitly asks.

Do not poll stopped/background agents, launch model probes, or repeatedly rerun
unchanged validation. Goal state alone is not a reason to ask the user to repeat
an explicit instruction to continue.
