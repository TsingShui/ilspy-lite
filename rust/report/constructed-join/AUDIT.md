# Closed constructed base at a reference stack join: pre-implementation audit

Baseline: `dea319aac`; latest complete corpus scan remains
`report/modopt/corpus-scan/baseline.json` (728,029 / 770,748). The constructor
follow-up has only targeted measurements; this group does not alter inputs,
reference selection or denominator. No third-party method is executed.

## ILSpy implementation and original IL

`IL/ILReader.cs::ImportedBlock.MergeStackTypes` merges stack categories and
reimports a block when its incoming stack changes. ILSpy does not use the Rust
join algorithm. Its `ParameterizedType.DirectBaseTypes` substitutes the complete
owner type arguments, and `CSharp/Resolver/CSharpConversions.cs::IsSubtypeOf`
walks those bases and checks definition identity and variance without erasing
type arguments. The Rust adaptation must retain typed CFG slots and explicit
per-edge copies; this audit does not propose a C# AST conversion.

Two original FSharp.Core methods are retained as default-output/ILSpy/IL triples
under `audit/`: `0x060010AD` and `0x060010AF`. Ours returns Partial, while ILSpy
renders both without an unknown-result diagnostic. Original IL branches between
sibling generated local classes, joins them on the evaluation stack, then stores
the result into a closed constructed generic function local. The sibling
TypeDefs have the same constructed `extends` TypeSpec. A temporary diagnostic
on `0x060010AD` identified the terminal failure as `Object -> StructuralRef`:
`join_reference_type` fell back to `object` after rejecting both sibling-to-
sibling assignments; the subsequent typed store cannot prove that downcast.
Intermediate failed sibling comparisons alone do not establish a foreign-token
identity defect. The diagnostic instrumentation was removed.

The earlier checkpoint's broad explanation of the incompatible-type bucket as
cross-image generic parent projection is not exhaustive: temporary probes of
`ExplicitConversions.exe` show I4-to-Native and I4-to-pointer mismatches in that
same reason bucket. Those require separate width/stack/unsafe analysis; their
counts are not a forecast for this reference-join group.

## Proposed bounded shape

For two nominal local reference classes, inspect the declared constructed bases
of their local class chains in the caller image. A candidate must be a fully
closed, class-kind TypeSpec representable in the caller's signature arena. It
may be selected only if the existing complete reference-relation proof accepts
both incoming types as assignable to that exact candidate. Preserve all generic
arguments, scopes, CFG edge order and values; do not infer a relationship from a
shared generic definition alone. If no candidate is proved, keep the existing
object merge and its later refusals. Open constructed owners, foreign-only
parent identities, variance joins and malformed/cyclic hierarchies are not
implicitly admitted by this slice.

Before implementing, an owned two-sibling branch must fail, a mutation must
change only one declared constructed base to a different instantiation, and
both output and behavior must be tested on Debug and Release fixtures. The
retained behavior must distinguish which original object was selected, nulls,
field state and generic argument identity.

## Scan-order defect encountered during self-review

The targeted whole-image scan reported FSharp.Core `0x06001ABC` as Decompiled,
but an individual production CLI decompilation of that same token with the
same ILSpy reference policy remains Partial (`constructed value type needs a
local definition`). The IL is `newobj` followed by a MethodSpec call whose
instantiation includes `KeyValuePair<!!0,!!1>`. This contradicts the earlier
claim that constructed-value facts are independent of scan order. The scan can
inherit a value-definition fact from an earlier method; `value_type_tokens`
collects member signatures and owner facts but omits MethodSpec instantiation
arguments. No recovery claim for this sample is valid until the discrepancy is
fixed and single-token and scan results agree.

ILSpy's `MetadataModule.ResolveMethodSpecification` decodes each MethodSpec
argument in the caller's generic context and specializes the selected target.
The Rust prefetch must inspect those argument types before lowering, then apply
the existing dependency-fact policy and generic-scope checks. An owned method
whose only use of a foreign constructed value is a MethodSpec argument must be
red on single-token analysis while its later whole-image scan can be green; the
fixed behavior must be green in both orders and remain Partial without the
required selected definition. This is a cache-order correctness repair, not
permission to infer a foreign layout or change reference selection.

## Red, green, behavior and targeted observations

- `red.log` is the original closed-sibling join failure; `green-final.log`
  records both local- and external-base positives and the distinct-base negative.
  A direct TypeDef `extends` mutation changes only one sibling from
  `Base<string>` to `Base<object>`; the requested `Base<string>` result remains
  Partial. Synthetic stack-only structural identity is also tested.
- `evidence-green.log` is a test-authoring failure: the initially chosen
  object-return fixture compiled to two direct returns, so it had no stack join
  and could not test synthetic-slot evidence. The corrected null-test fixture
  produces an actual join; `evidence-corrected.log` passes.
- `methodspec-order-red.log` records the owned individual Partial/scan success;
  `methodspec-order-green.log` records the positive after the prefetch repair.
  A separate absent-reference test remains Partial; the final test checks both
  sequential and two-worker scans against the individual result.
- The first post-repair relation behavior run failed because a private helper
  was accidentally included by the driver's owned Ops selection. The helper was
  moved to a separate owned type, leaving the selection and its assertions
  intact. `reference-relation-corrected.log` and
  `reference-relation-final.log` then pass: 839 comparisons / 77 selections per
  configuration, with three branch-inversion and one MethodSpec-result mutants
  rejected in addition to existing controls. No compiler failure is counted as
  a semantic mutant rejection. The constructor and value-call drivers also pass.
- `targeted-final.json` compares four complete targeted CLI scans with their
  respective prior saved scans: +64 FSharp.Core, zero recovered-status losses
  across the four images. The FSharp.Core one-thread and two-thread results
  agree for all status/reason rows, and the formerly Partial individual ToSeq
  result now matches. This is not an aggregate acceptance scan or a general
  proof of scan-order independence.
- Fmt, clippy with warnings denied, and 494 Rust tests pass. The six reviewed
  production triples and their bounded self-review are recorded in `review/`.
  `evidence.zip` retains the raw audit, test, scan and behavioral records.
