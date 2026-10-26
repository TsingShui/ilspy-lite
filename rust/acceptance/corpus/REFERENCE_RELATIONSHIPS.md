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

Exact dependency binding projects foreign ancestors into existing caller token
identities; no foreign token is interpreted against the main image. Demand-driven
static lowering obtains only graphs needed by failed typed proofs. The context
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
