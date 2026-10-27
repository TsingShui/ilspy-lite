# Generic member caller scopes and constructed static calls

This grouped repair uses existing input-scoped structural type identities and
exact runtime call/allocation operations. It does not add a C# AST, erase
instantiation arguments or execute original IL.

## Repairs

- Member parameters/results in an open-self instantiation retain the supplied
  caller TypeContext. The empty-context conversion was losing owner-qualified
  type variables even when the call site had already proved them.
- C<!0..> is collapsed to a named receiver only for the caller's own definition
  or a proved local identity-prefix base path. Arity, owner and base arguments
  are checked. A second generic definition with the caller's variables stays
  structural and substitutes in that caller's scope.
- Signature interning registers open self only for the actual caller definition,
  not every positional instantiation of a different owner.
- Local constructed static calls substitute arguments/results through the same
  path as instance calls but carry no receiver. `__cil_static_call_TOKEN`
  denotes the exact original static target, instantiated declaring owner, runtime
  type-initialization semantics and effects. It is not an open-generic source
  method-name guess.
- Interfaces, MethodSpec calls, unsupported external constructed definitions,
  nonidentity generic inheritance and generic-base field spelling are not
  blanket-admitted by this change.

## Audit and test evidence

Before repair: five production triples in `report/generic-binding/AUDIT.md`;
one additional owned static-forwarder ILSpy/IL/default-output triple. Design
and the regression refinement are recorded in `DESIGN.md`.

Public decompile/scan tests were red before green for member forwarding and
foreign constructed owners, then for constructed static calls. Negative patches
retain arity and caller-variable bounds. A complete fixture check fails on
unexpected fallback; scan/method admission agrees.

An initial full scan found 38 constructor regressions involving a smaller
identity-prefix base scope, not invalid methods. Their artifacts are preserved.
An owned prefix-base constructor was captured red and the exact path proof
adjusted; final scan has zero regressions.

Owned behavior reconstructs Bag/Proxy/Pair/Swapped declarations with production
default-readable method bodies, field scaffolding and scoped alias/exact-leaf
bindings. Recovered support bodies call other recovered support bodies; they
are not replaced by original fixture method execution. Per release/debug:
240 comparisons over 60 closed method selections, covering int/string/object/
struct, distinct swapped generic arguments, object identity, reference-bearing
struct copies, receiver mutation, null nested receivers and generic static
initialization. Four semantic mutants rejected: wrong forwarded argument,
wrong returned value, wrong exchanged value and dropped initializer effect.

Existing declaring-member behavior (18/11), abstract-member behavior (22/4),
and generic-reference behavior (85/15) and their mutation controls remain green
per configuration. The historical 340 generic-reference count included multiple
configurations/presentations; do not confuse it with the per-configuration 85.

387 Rust tests, clippy warnings denied and fmt pass. Eleven corpus triples
self-reviewed, eleven CRUDE-BUT-EQUIVALENT, no changed semantic differences found.
No third-party execution or independent/formal proof claim.

## Final unchanged-scope result

Before: `report/reference-checks/corpus-scan/baseline.json`.
After: `report/generic-binding/corpus-scan-final/baseline.json`.

| Metric | Result |
|---|---:|
| managed / body-bearing images | 614 / 319 |
| target method bodies | 770,748 |
| Decompiled / Partial | 375,506 / 395,242 |
| overall fallback | 51.28031% |
| gains / images gaining | 8,695 / 83 |
| recovered-method regressions / incomplete | 0 / 0 |
| OpenRA.Game / OpenRA.Mods.Common gains | 2 / 1 |

Prior first blockers: caller-variable scope 8,082; incompatible receiver 455;
concrete nongeneric allocation owner 112; generic static owner 46. These sum
to 8,695 but are attribution by old reason, not isolated causal measurements.
The first scan's net +8,644 and 38 regressions are superseded, not discarded as
evidence. The current overall <=5% goal is not yet met.

## External constructor and allocation signatures

`external_allocation_for` and the referenced-constructor path decoded their
parameter and return types with `Ty::from_param`, which uses an empty type context.
A constructor whose signature mentions its declaring type's own parameters
(`C<!0>::.ctor(!0)`, common for local generic classes inside `FSharpMap<K,V>`) then
failed with `generic variable has no owner or exceeds declared positions` even
though the call site's context names those parameters. Both paths now use
`Ty::from_param_scoped` with the caller's context, so the parameters keep their
exact variable identity. The check that a constructor returns `void` uses the same
scoped decoding.

Evidence: `FSharp.Core` 8,617 -> 8,631 decompiled (+14) with no other image change;
488 Rust tests, fmt and clippy pass.
