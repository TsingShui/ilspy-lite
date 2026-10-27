# Three-group batch: reference-proof regressions

The complete `foreign-hidden-batch/corpus-scan` reports 728,579 Decompiled /
42,169 Partial of 770,748, but comparison with `modopt` found **672 gains and
122 regressions**, so the net +550 is not an acceptable milestone. All 122
regressions report `reference relationship depth/work limit exceeded` across
five NUnit images. The initial scan and complete delta are retained; this is not
being dismissed as flaky or hidden behind the net number.

A single-token comparison reproduces `NUnitLite.Tests.CommandLineTests.NoInputFiles`
(0080-06000003): `596e59416` decompiles it while `7b55bb529` returns Partial.
Original IL, both production outputs and ILSpy source are retained in
`regressions/`. ILSpy renders the original array construction, option validation,
count read and assertion without an unknown-result diagnostic. Substituted
framework reference identities in the production output are metadata selections,
not runtime compatibility proofs; the input body is not executed.

Relevant ILSpy mechanisms are `ILReader.MergeStackTypes`,
`ParameterizedType.DirectBaseTypes`, `TypeSystemExtensions.GetAllBaseTypes`
and `CSharpConversions.IsSubtypeOf` (traced in the preceding group audits).
Rust's `reference_definition` already walks and validates the complete reachable
foreign hierarchy, producing instantiated transitive parent templates, with cycle,
arity, depth and work checks. `Proof::prove` recursively re-walks those flattened
parents. Adding foreign-only identities makes an interface chain expand
repeatedly and can exhaust the existing proof work bound even on a finite graph.
The intended repair must avoid redundant expansion without skipping validation,
dropping any inheritance/variance relationship or loosening resource limits.

Before implementation, an owned deep foreign-class chain reproduces the same
`reference relationship depth/work limit exceeded` refusal. Its normal relation
must recover after the repair; a cyclic hierarchy and missing reference must
remain Partial. A behavior comparison must retain null and object identity, with
a wrong-result mutant. The original 122 losses remain part of the recorded failed
scan until they are individually or in a complete follow-up scan verified recovered.

The first owned attempt used a deep interface chain. Roslyn also emitted its
transitive interfaces on each InterfaceImpl list, exhausting the separate
hierarchy-building bound and yielding a different incompatible-type refusal; that
failed fixture attempt is retained in `interface-fixture-red.log`. The corrected
class chain has one direct base per level and reproduces the actual proof-work
error in `deep-red.log`.

## Repair and targeted checks

`Proof::foreign_ancestry` now compares each already-validated transitive parent
template once instead of recursively expanding its complete closure again. It
still checks every parent, substitutes the complete arguments, accounts for work,
and uses the existing directional variance proof. It does not accept the first
match by bypassing cyclic-hierarchy validation. The owned deep-chain positive is
green; missing references and a mutated cyclic base remain Partial. Existing
foreign cycle and hidden-argument negatives pass as well.

The recovered owned `ForeignDeep` returns the same reference, including null;
the relation driver executes 851 comparisons / 81 selections in each of Debug
and Release and rejects the added wrong-result mutant. The constructor, value
call and exception drivers pass. Rust fmt/clippy and 496 tests pass.

All 122 recorded loss tokens were rechecked with the production CLI in five
complete targeted image scans: 122 Decompiled and 0 still Partial. The single
NUnit example also returns to Decompiled in isolation. `regressions/targeted-fixed.json`
records each token; this targeted result is not a complete regression scan.
A new complete scan and delta against `modopt` remain required before treating
the batch as regression-free or updating the weighted aggregate.
