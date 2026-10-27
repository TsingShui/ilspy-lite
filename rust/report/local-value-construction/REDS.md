# Local value constructor red record

Retained public seam diagnostics, as excerpts rather than complete logs:

- newobj-red.log: NewPlain fell back with "value-type receiver needs IL fallback".
- delegation-red.log: Plain(int) fell back with "constructor call needs a reference-class owner".

The green paths are exact value operations, not status overrides.
constructor-boundaries.log retains virtual/nonlocation receiver and malformed
constructor refusals. Ref-argument coverage starts green because typed managed
arguments already exist; it is not labeled a new semantic red.

The first full regression had two frozen new-object promotion failures (451/453).
The legacy new-object and struct-receiver profiles executed their promoted
constructors before row removal. Final Rust regression is 453/453.

The zeroing mutant initially selected a newly added byref overload and survived
because that overload only received fresh storage in those cases. Selecting the
int/object constructor fixes the mutant's observation; the corrected compiled
mutant is rejected by acquired-cell state. This is a test-authoring correction,
not a production defect or a counted initial rejection. Regex authoring and the
single-element-loop lint correction are not semantic reds.

Final commands/statuses: behavior-manifest.json. Fixed scan delta has zero
regressions and incomplete images. No third-party body ran.
