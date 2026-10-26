# Exact generic method instantiations

## Implemented semantics

MethodSpec targets are decoded through the raw MethodDefOrRef coded index.
The checked signature must declare the supplied generic argument count.
Local definitions additionally validate GenericParam owner/positions/flags.
Unused method arguments are still validated; void/unsupported shapes, varargs,
modified/byref signatures and unsupported receiver kinds remain refusals.

Type-owner Var and method MVar are substituted simultaneously. Inserted
arguments already belong to the caller and are not substituted again. This
preserves cases such as a caller T with a method U invoking Store<U>.Echo<T>.

`__cil_methodspec_call_TOKEN` denotes the exact original direct/static runtime
call, including instantiated declaring owner, method arguments, overload,
runtime type initialization, constraint/access checks and original exceptions.
It is one leaf call, not an original-method/IL proxy for the recovered body.
Nonvirtual instance callvirt uses the existing exact null-checking callvirt
leaf with the original MethodSpec token. No generic type arguments are discarded
by rendering a bare receiver method name.

Foreign static calls use their declared signature and exact original token,
without pretending to know a body or value layout. Foreign generic instance
dispatch still needs its own dependency facts and remains refused.

## Deliberately excluded generic virtual dispatch

An owned side-effectful argument gate exposed a real mismatch in the available
.NET 10 environment: original GVM callvirt on a null receiver produced
NullReferenceException with trace=0, while snapshot-based recovered text ran
the argument helper, producing trace=1 and DivideByZeroException. Reflection and
minimum-JIT-optimization mode reproduced the original behavior. The original IL
lists the argument helper before callvirt; the runtime-specific generic virtual
lookup behavior is not certified by the current leaf/snapshot model.

The failing fixture remains, with stable refusal
`generic virtual dispatch needs runtime lookup-order modeling`.
Local declarations are inspected exactly before admitting nonvirtual generic
callvirt. The original failing logs and audit are preserved; no GVM gains or
cross-runtime equivalence are claimed. Nongeneric virtual calls are unchanged.

## Validation evidence

- Five pre-change production ILSpy/default-output/IL triples and the bounded
  design: `report/method-instantiations/AUDIT.md`.
- Public decompile/scan red before green; wrong argument count, void argument,
  nongeneric target, byref signature and GVM refusals remain tested.
- Owned default-body reconstruction executes recovered support bodies as well
  as recovered call sites. It does not delegate selected support bodies to their
  original fixture implementations.
- Per release/debug: 122 comparisons / 13 closed selections for reference and
  struct copies, caller/method scope crossing, nonvirtual state, nulls, external
  factory exceptions and argument-before-null side effects.
- Four semantic mutants rejected: wrong forwarded value, wrong stored value,
  wrong external method type argument and dropped argument effect.
- Existing promoted methods enter their behavior gates:
  allocation 40 comparisons / 12 selections (generic new constraint over three
  types and fresh reference identities); ordinary virtual/Enumerable.Count
  51 / 10; scoped references 1,380 / 240 contexts / 24 methods (recovered
  UnsupportedCall uses the recovered Identity closure). Their controls pass.
- Previous generic-binding gate remains green (240 comparisons / configuration,
  four mutants).
- 389 Rust tests, all-target clippy with warnings denied, fmt pass.
- Ten corpus triples self-reviewed against original IL, ten
  CRUDE-BUT-EQUIVALENT, no changed semantic differences found. No third-party
  execution or independent/formal verification claim.

## Same-scope production result

Before: `report/generic-binding/corpus-scan-final/baseline.json`.
After: `report/method-instantiations/corpus-scan/baseline.json`.

| Metric | Result |
|---|---:|
| managed / body-bearing images | 614 / 319 |
| target method bodies | 770,748 |
| Decompiled / Partial | 387,610 / 383,138 |
| overall fallback | 49.70989% |
| gains / images gaining | 12,104 / 132 |
| recovered-method regressions / incomplete | 0 / 0 |
| OpenRA.Game / OpenRA.Mods.Common gains | 86 / 128 |

Every gain previously had the generic/indirect method-reference first blocker.
The aggregate <=5% goal remains unmet. The historical 1%-per-image diagnostic
is not the revised acceptance gate. No reference-selection or scope changes.
