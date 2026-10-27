# Audit: MethodSpec calls on external constructed value types

Baseline is `5401167c6` and the complete production scan in
`report/foreign-hidden-batch/corpus-scan-fixed/baseline.json`. The first blocker
`external constructed value generic method needs dependency facts` appears on
2,269 bodies. This count is not a forecast: later blockers may remain and the
bucket may contain unrelated shapes.

## ILSpy call chain and selected candidates

`ILReader.ReadAndDecodeMethodReference` calls `MetadataModule.ResolveMethod`
with the caller's `GenericContext`. `ResolveMethodSpecification` decodes method
arguments in that context and `ResolveMethodReference` resolves the MemberRef's
constructed declaring type, searches the definition using the uninstantiated
signature, then specializes class and method arguments separately. ILSpy can
construct a fake member when the definition is missing; this Rust slice does not
use such a fake as proof of the selected definition or dispatch flags.
`ILReader.DecodeCall` obtains expected receiver stack kind from the declaring
value type, pops arguments in reverse stack order, and retains their original
evaluation order in the call node. `ExpressionBuilder.VisitCall` passes that
node to `CallBuilder.Build`. ILSpy's async transformation can produce an
`async` source method; this project may retain the original state-machine calls
as exact low-level operations instead.

Three candidate triples in `audit/` retain production default output, ILSpy
source and original IL. All IL and ILSpy commands exited zero; production
commands exited 2 because they returned Partial. The candidates are:

- 0487 `System.Private.Xml` `0x06000872`, async wrapper: the original IL copies
  the builder out of the state-machine struct into a separate local, calls exact
  `AsyncTaskMethodBuilder<bool>.Start<StateMachine>` on that local's managed
  address, then reads the task from the original state-machine field. No output
  may silently turn this distinct local into the field or run its body.
- 0625 `System.Xml` `0x0600001E`, async wrapper from the Mono corpus. ILSpy
  reconstructs an async method; the original IL and selected dependency facts
  remain authoritative.
- 0005 `Microsoft.CodeAnalysis` `0x06001BEC`, async diagnostics wrapper. Its
  state-machine launch has separate constructed owner and method arguments.

No explicit ILSpy unknown-result diagnostic appeared in these samples. They are
static evidence, not executed third-party behavior. Selected reference assembly
versions are metadata choices, not proof of target-runtime compatibility.

## Rust boundary to test

`method_instantiations::resolve` already substitutes the owner and method
arguments separately and retains the exact MethodSpec token. It accepts a
referenced constructed value receiver only after an external method fact proves
an exact declaration and distinguishes generic virtual dispatch. The lift must
still require the exact managed location, argument types and supported call
kind. `DependencyContext::external_method_instantiation` resolves the selected
method and rejects static, abstract, interface and inconsistent definitions.
The body-level prefetch currently restricts the MemberRef parent to a TypeRef,
so a TypeSpec parent never supplies the requested fact. Validate resolution and
the complete boundary with owned red/green tests before changing this guard;
do not treat a TypeSpec signature alone as proof of a member or layout.

## Validation performed

Owned Debug and Release fixtures cover a closed value owner, a caller-scoped
value owner whose method argument is distinct from the owner argument, a
reference-typed method argument, a copied receiver, a receiver-aliasing field
address, and argument evaluation order. Negatives: a different method argument, a
`callvirt`, a non-location receiver, a renamed declaration and a static
declaration stay Partial; a validated virtual declaration is admitted for the
non-virtual call and refused under `callvirt`. Only the owned fixture and the
recovered carrier are executed.

- Rust: 498 tests, `cargo fmt --check` and clippy `-D warnings` pass.
- Behaviour: 720 comparisons and 80 closed selections per configuration, with
  six added mutants rejected (receiver copy, argument copy, field-address copy,
  evaluation order, exception path, result).

## Not yet measured

Two attempts to run the targeted production scan were interrupted before writing
a delta, so the corpus effect of this slice is **unmeasured**. The last complete
scan remains the 42,044 fallback baseline; nothing here changes it.
