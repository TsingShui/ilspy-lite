# Acceptance threshold change: 5% to 1%

The user tightened the overall weighted acceptance gate from `<=5%` to `<=1%`.
The metric, scope, denominator, reference policy and evidence requirements are
unchanged; only the constant in the raw integer comparison moved. The former 5%
gate becomes an intermediate milestone.

## Code path and change

`acceptance/corpus/corpus.py::aggregate_gate` computes the single weighted
decision. It now uses `100 * unrecovered <= bodies`, reports
`threshold_percent: 1`, and `baseline` and `implementation_scope.py` copy that
value into their summaries so a saved report states the gate it was judged by.
`gate_image` keeps the per-image `<=1%` diagnostic; per-image results remain
informational and never decide acceptance. `implementation_scope.py` prints the
current threshold in `SCOPE.md`.

## Red then green

`test_gate_uses_the_weighted_one_percent_boundary` failed first with exit 0 for a
3/200 aggregate, which the old 5% rule accepted (`red.log`). After the constant
change it passes: 3 unrecovered of 200 fails, 2 of 200 passes even though one
image stays above the per-image diagnostic, and 2,001 of 200,000 displays
`1.00%` but fails the raw integer test (`green-direct.log`).

The previous 5% test was rewritten rather than deleted: it now asserts that 5%
and `5.00%` are rejected, that incomplete inputs still fail, and that an empty
aggregate stays not applicable. Both the direct `baseline` scan and the saved
implementation-scope exit codes were updated to the new boundary, including a
new passing case that also checks the recorded `aggregate_threshold_percent`.
The full corpus-tool suite passes 22/22 (`e2e-final.log`).

## Effect on the existing evidence

`milestone.json` recalculates the last complete fixed-scope scan
(`report/foreign-hidden-batch/corpus-scan-fixed/baseline.json`) with the new
constant and performs no new decompilation, classification or denominator
change: 770,748 bodies, 42,044 unrecovered, 7,707 allowed, 34,337 additional
bodies required. The same scan under the 5% constant still needs 3,507. All 614
images remain complete.

Because the change is arithmetic and reporting only, no method status, output,
scope decision or skip evidence changes, and no recovery gain is claimed.
Historical reports keep their original scan counts; their threshold wording is
marked historical where it still says 5%.

## Limits

This is a policy and tooling change, not a semantic improvement. The gate still
fails, and the remaining work is real decompiler capability; nothing here
weakens the requirement that recovered bodies be semantically credible.
