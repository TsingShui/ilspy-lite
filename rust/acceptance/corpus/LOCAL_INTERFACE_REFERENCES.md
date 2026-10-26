# Named reference checks, assignments and interface dispatch

This repair group extends existing typed IL/CFG/lift operations. No new AST,
original-IL executor, whole-method proxy or interface implementation guessing.

## Admitted shapes

- Local nongeneric interface identities reuse the input-scoped named-reference
  token and reference stack kind. Interfaces require Interface/Abstract flags
  and null Extends; class hierarchy validation is unchanged.
- Reference loads, stores, returns and copies preserve the same object, including
  boxed values that already arrive through interface references.
- Class-to-interface and interface-to-base-interface assignments require a path
  through local nongeneric InterfaceImpl edges. Class ancestors are considered;
  generic/external interface edges are not erased to their definitions.
  The per-image edge index is cached, traversal is bounded and cycles are refused.
- Existing exact callvirt leaves perform original runtime interface dispatch,
  including explicit implementations, null checking and argument order.
  They do not select a concrete implementation from a method name.
- castclass/isinst admit local nongeneric abstract classes and interfaces, and
  exact core String/Object TypeRefs. They retain the throwing vs null-yielding
  check, reference identity and null behavior. Core name/key/culture/content-type
  validation remains mandatory.
- Abstract/interface allocation remains rejected. Interface instance fields and
  constructors are invalid. Generic interfaces, external interface dispatch,
  constrained receivers and unsupported modifiers remain separately gated.

## Audit and tests

Before each repair, ILSpy/default-output/IL triples were inspected:
`report/abstract-casts/AUDIT.md` (abstract and core casts) and
`report/reference-checks/AUDIT.md` (five interface cases).
The first-blocker bucket size was not treated as a predicted recovery count.

Public Rust API/CLI fixture tests were red before green. Frozen tables keep
generic/foreign cases explicit and fail on stale promotions. Negative metadata
patches cover invalid interface flags/base, instance fields, abstract/interface
allocation, cyclic interface inheritance, missing implementation edges and
counterfeit core keys/names/content types. Core Object cast/test opcodes are also
checked with a patched owned body.

Owned execution, release and debug:
- Interface references: 125 comparisons / 13 selected call-site methods.
  Null, implicit/explicit/boxed receivers, inherited assignments, copies, casts,
  field exchange, stateful repeated calls and argument exception order.
  Four mutants rejected (dropped call, dropped repeated call, dropped argument
  effect and throwing type test).
- Casts: 114 comparisons / 15 methods, five mutants rejected.
  Primitive result values are compared, not merely their type names; bool/int
  corruption controls protect that oracle. Abstract and interface success cases
  include actual implementing/derived objects.
- Existing objects: 111 cases / 19 selected methods, helper mutation rejected.
- 382 Rust tests; fmt and all-target clippy with warnings denied pass.

Fourteen corpus triples self-reviewed against original IL: two EQUIVALENT,
twelve CRUDE-BUT-EQUIVALENT, zero SUSPECT for the changed semantics. No
third-party execution, independent review claim or general C# recompilation
claim. See `report/reference-checks/review/review-source.md`.

## Unchanged implementation scope

Before: `report/abstract-members/corpus-scan/baseline.json`.
After: `report/reference-checks/corpus-scan/baseline.json`.

| Metric | Result |
|---|---:|
| managed / body-bearing images | 614 / 319 |
| method bodies | 770,748 |
| Decompiled / Partial | 366,811 / 403,937 |
| overall fallback | 52.40844% |
| newly recovered / images gaining | 23,266 / 119 |
| recovered-method regressions / incomplete scans | 0 / 0 |
| OpenRA.Game / OpenRA.Mods.Common gains | 222 / 186 |

Gain attribution by prior first reason (not isolated per-code-change causation):
interface declaring type 15,401; concrete cast target 5,032; incompatible
value/receiver 2,644; nonlocal cast target 189. These sum to 23,266.

The current acceptance criterion is overall fallback <= 5%, with per-image
reporting retained. The scanner's historical 1%-per-image diagnostic is 34/319;
it is not the revised acceptance gate. This group does not meet the overall goal.
