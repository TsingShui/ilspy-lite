# Scoped reference relationships

## Fixed-corpus result

Unchanged implementation scope: 614 managed inputs, 319 body-bearing images,
770,748 bodies. Final `report/reference-relations/corpus-scan-final/baseline.json`:

- **472,156 Decompiled / 298,592 Partial: 38.74055% aggregate fallback**.
- **33,355 newly recovered methods in 197 images**; zero recovered-method losses
  and zero incomplete images. Actual decrease: 4.32761 percentage points.
- OpenRA.Game: 2,557 / 2,210 (+84 recovered); Common: 11,116 / 7,291 (+1,402).
- Previous-reason attribution: 26,729 incompatible value/receiver cases and
  6,626 constructed-external-base cases. These are observed first blockers,
  not isolated causal experiments or predictions from bucket totals.
- The aggregate <=5% goal is **not met**. No scope, denominator, input version,
  reference selection policy or third-party execution changes.

The initial scan and its 18 regressions are retained in `corpus-scan/` and
`delta.json`. Final counts use `delta-final.json`, never net gains hiding losses.

## Semantics

Reference terms retain image-local nominal identities, every constructed
argument, vectors and caller-qualified generic variables. Local base/interface
templates substitute their own parameters, not unrelated caller/callee variables.
Open-self structural references can access the identical declaring definition
without being mistaken for a different generic base.

Interface covariance/contravariance needs actual metadata flags, reference
arguments and the directional relationship proof. Value arguments do not gain
variance, and managed-address element identity stays invariant. Cycles,
arity/position errors, unknown relationships and bounded work/depth failure
remain refusals.

The initial dependency binding projected foreign ancestors into existing caller
token identities; the later hidden-argument follow-up below also represents
selected foreign definitions without fabricating caller tokens. No foreign token
is interpreted against the main image. Demand-driven static lowering obtains
only graphs needed by failed typed proofs. The context
caches a registry with a candidate-count/version-policy/discovery stamp; tests
verify that new references rebuild it and query order does not change output.
No reference-version relaxation or extra implicit dependency discovery is added.

A proof adds no runtime check, copy, dispatch or allocation. Explicit formal-type
operands preserve a source-named call's original overload selection. Qualified
declared type spelling is kept separate from an ancestry/layout proof. A local
class with a declared external constructed reference base can retain its own
reference identity; this does not erase that base's type arguments or make
unproved ancestor assignments legal.

## Gates and limits

- Pre-audit: eleven production triples, ten usable ILSpy-success candidates and
  one explicitly inconclusive sample, not BOTH_NO or an exclusion.
- Cargo nextest: **405 passed**; fmt and all-target clippy with warnings denied pass.
- **22 owned behavior drivers pass**, including 143 comparisons / 36 closed
  selections per configuration for this fixture and four rejected mutants each.
  Original and recovered operations compare return identity/value, reachable
  fields, null/default/cast behavior, generic positions and overload choice.
- Cell<T>.ReadCast retains a generic declaring type in the execution carrier:
  a static generic carrier changed null-this runtime entry behavior and failed.
- Ten source/IL triples self-reviewed: one EQUIVALENT, nine
  CRUDE-BUT-EQUIVALENT, zero changed-semantic SUSPECT in the bounded sample.

Evidence: `report/reference-relations/{AUDIT.md,DESIGN.md,milestone.json}`,
`audit/manifest.json`, `behavior-manifest.json`, and
`review/{manifest.json,review-source.md}`. Reports distinguish static coverage
from owned execution; neither formal/independent verification nor whole-corpus
source recompilation is claimed. No third-party or metadata-only core toy runs.

Arbitrary unproved foreign relationships, generic value members/constructors,
broader boxing, constrained dispatch and GVM lookup-order modeling remain
separate capabilities. Missing references do not authorize guessing.

## Follow-up: closed constructed bases at stack joins

When two local nominal reference classes meet at an IL stack join, the lift may
use a closed constructed class from their declared local base chains as the
stack-slot type, but only after the existing assignability proof accepts both
incoming references to that exact instantiation. Complete type arguments stay in
the signature arena; no open generic definition, guessed identity, layout, new
cast, allocation or dispatch is introduced. The emitted edge copies select the
same original value, including null. Structural types appearing only in synthetic
stack slots receive their own signature evidence. Unproved, open and foreign-only
ancestor relationships retain their existing fallbacks.

The owned positive first failed, then passed for both local and foreign declared
bases in Debug and Release. Mutating just one child's declared base to a different
instantiation remains Partial. The reference-relation driver now executes 839
comparisons / 77 selections per configuration, including nulls and identity at
both new joins. Reversing each join branch is detected by three semantic mutants.
The original failing compiler-shape experiment and its corrected fixture are
retained in `report/constructed-join/evidence.zip`.

Self-review also exposed a separate scan-order defect: FSharp.Core `ToSeq`
was Decompiled in a whole-image scan but Partial when decompiled alone because
foreign constructed-value facts from MethodSpec arguments had not been collected.
The earlier claim that `32dcafe8b` made these facts generally scan-order
independent is **withdrawn for MethodSpec instantiation arguments**. The prefetch
now walks those arguments, and an owned case tests missing references, individual
output, sequential scan and two-worker scan. The recovered exact MethodSpec call
is executed in the owned reference driver, and a wrong-result mutant is rejected.
The repaired FSharp.Core single-token result and complete one- versus two-thread
scan agree; this does not claim parity across the corpus.

Five self-reviewed FSharp join samples and the MethodSpec case retain their
production default/ILSpy/IL triples. Four targeted scans show 64 gains in
FSharp.Core (63 previously reported as incompatible types, one as the foreign
value-fact blocker) and no status losses in the four images tested. These are not
a complete corpus scan. Rust fmt/clippy and 494 tests pass; see
`report/constructed-join/{AUDIT.md,targeted-final.json,evidence.zip}` for the
red/green tests, behavioral controls, comparison and bounded main-session review.

## Follow-up: selected foreign argument identities

A generic ancestor can mention a type that the caller has no TypeRef for. The
relationship proof now represents such a selected foreign definition by its
normalized assembly identity, full name, arity and exact image/TypeDef location,
with its complete arguments. A bounded demand queue supplies its selected
ancestry and interface variance flags. No foreign token is used as a caller token,
and generic variables retain their original method or type scope. Missing or
ambiguous definitions, invalid kind/arity, unproved relationships and unsupported
shapes remain refusals. The unchanged ILSpy reference policy is metadata selection,
not proof that a substituted assembly matches the target runtime.

Owned closed and open generic interface cases were red and are now green in
Debug and Release. They have no caller TypeRef for the hidden argument; they
exercise covariance through a hidden parent, including `!!0` in a two-parameter
method. Tests refuse missing dependencies, invariant variance, different hidden
ancestry, value/class disagreement, changed generic position and a same-full-name
type in a different supplied assembly. The owned reference driver executes 848
comparisons / 80 selections per configuration, including null/reference identity,
and rejects two added wrong-result mutants. It also passes for a selected-version
owned library. The prior constructor/value/exception gates and 495 Rust tests pass.

Four like-for-like targeted real-input scans show no gains or losses from this
specific group. An initial 22-method Roslyn gain was measured against an older
baseline and is withdrawn as an attribution to this group; the isolated prior
revision scan shows those methods were already recovered. The complete scan for
the three groups remains pending. Audit, negative tests, raw evidence and bounded
main-session self-review are in `report/foreign-hidden-relations/`.
