# Aggregate acceptance gate audit

> Historical 5% milestone. The current acceptance gate is overall weighted 1%;
> see `report/one-percent-gate/AUDIT.md`. Historical scan results are unchanged.

The objective at this milestone required one weighted implementation gate:
`100 * unrecovered <= 5 * bodies`, with every target image complete. Per-image
counts remain visible, but no individual image is required to meet 1%.

`acceptance/corpus/corpus.py::gate_image` preserves the historical per-image 1%
result; `gate`, `baseline`, and `implementation_scope.py::apply` currently also
use `all(image.pass)` for their exit codes. That is a stricter, different
criterion and can reject an aggregate-passing corpus. The retained earlier
reports use that historical per-image metric, so its values must not be silently
relabelled or rewritten.

ILSpy's source decompiler does not decide this fixed-corpus acceptance policy;
its metadata and instruction paths are not an authority for the aggregate
arithmetic. The independent inventory, explicit per-image body accounting and
production CLI results remain the inputs. No source decompilation, reference
selection, classification, status or denominator changes belong in this group.

Owned CLI tests must cover a weighted exact-5% pass despite an individual image
above 1%, a just-over-5% failure, incomplete/missing-body refusal, zero-body
non-applicability, and both direct scan and saved implementation-scope exit codes.
The historical per-image decisions remain informational and retain their current
values. No input assembly is executed, reacquired or rehashed.

## Validation

The three new public-command tests failed before the correction and pass after
it. The full corpus-tool suite passes 21/21. A 5.0005% synthetic case displays
5.00% but fails the raw integer test; an empty aggregate is not applicable and
does not pass. Missing bodies remain unrecovered and make the scan incomplete.

Applying the corrected gate to the last completed fixed scan gives 770,748 bodies
and 130,523 unrecovered: complete, applicable, and not passed. This calculation
reads the saved report only; it is not a new scan or recovery gain. No existing
per-image legacy value, scope classification, method status or input was changed.
