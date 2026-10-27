# Ordinary local value construction and initialization

Local nongeneric MethodDef value constructors share the existing typed value
constructor representation. Their actual declaring value definition, ordinary
instance constructor flags and void signature are checked; parameters use the
declaration's own scope, not caller variables.

newobj emits __cil_value_newobj_TOKEN: separate zeroed value storage, exact
constructor invocation, then completed value copy. A constructor failure leaves
later destinations unchanged while retaining argument/constructor effects.

Direct MethodDef call .ctor emits __cil_value_ctor_TOKEN on the actual managed
cell. It neither allocates nor clears/copies that receiver. Partial writes and
old unassigned fields remain observable on throw. This is distinct from
reference-class base/same-type constructor initialization; virtual constructor
calls and wrong location identities still reject.

Existing generic MemberRef/TypeSpec newobj support remains. Ordinary nominal
MemberRef/foreign construction, generic MethodDef contexts and other previously
unproved cases are not broadly admitted by this slice.

## Evidence

Seven pre-change production default/ILSpy/original-IL triples are retained in
report/local-value-construction/audit/ and AUDIT.md. CVec.Minus has explicit
ILSpy unknown-result diagnostics and remains an independent foreign case.

Public reds cover ordinary local newobj and value-constructor delegation.
Boundary checks cover constructor flags/void signature, callvirt and nonlocation
receivers. Existing acquired-cell and byref-argument behavior is exercised too.

Owned actual recovered output passes **585 comparisons / 65 closed selections
per compiler configuration**, including prior generic/value cases and ordinary
construction/delegation, readonly values, argument trace/failure, byref arguments
and destination aliases. A separate authored call-.ctor fixture executes the
same matrix on nonzero incoming storage. Selected constructor and argument
bodies execute recovered text; the readonly field adapter emits one original
stfld operation, not a constructor-body proxy or a layout mirror.

Default/newobj variant rejects **16 compiled semantic mutants/config**; the
acquired-cell variant rejects **18**, including unwanted zeroing and delayed
partial writes. A byref-overload addition initially made the zeroing mutant
select the wrong constructor, so it survived; after fixing selection to the
by-value constructor, the compiled mutant fails the intended acquired-cell
state observation. The survival is not counted as a rejection.

Legacy promotions execute before row removal:
- Struct receiver: **43 comparisons / 21 methods/config**, recovered Point
  constructor support and the prior three store mutants.
- New-object profile: **54 comparisons / 16 methods/config**, including exact
  Holder value construction and the existing identity mutant.

The first full regression retained two new-object frozen-promotion failures
(451/453). Final fmt/clippy and **453 Rust tests** pass. To keep validation
proportional, this milestone reuses the existing suite and runs **nine relevant
owned drivers**, not an expanded all-driver matrix. Commands/statuses are in
report/local-value-construction/behavior-manifest.json.

## Fixed-corpus milestone

**566,419 Decompiled / 204,329 Partial of 770,748 bodies**, fallback
**26.510480727812464%**, **+7,662 in 102 images**. Zero regressions/incomplete
images; fixed scope, exclusions, input hashes and exact reference stamps.

OpenRA.Game: **3,113 / 1,654 (+215)**. Common: **12,129 / 6,278 (+14)**.
First-blocker attribution: value receiver 6,877; constructor outside constructor
context 489; reference-class constructor restriction 266; original-this
restriction 30. Attribution is not causal isolation or a forecast.

Eight gained production triples self-reviewed CRUDE-BUT-EQUIVALENT, no
changed-semantic SUSPECT in the bounded sample. No third-party execution,
independent/formal verification or whole-corpus/cross-runtime claim.
**Aggregate <=5% remains unmet.** Complete per-image results and residual methods
remain at report/local-value-construction/corpus-scan/.
