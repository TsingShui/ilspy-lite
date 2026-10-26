# Constructed and external value-type copies

Two capabilities landed together because they share one claim: a value is copied
whole, exactly as the CLI copies it. They differ in where the definition lives and
how it is proven.

## Constructed value types

A `valuetype` generic instantiation (`Pair<int>`, `ImmutableArray<X>`) is now a value
copy. The proof is about the **definition**:

- sealed, not an interface, not abstract,
- exact core `System.ValueType` base and not one of the core scalars,
- no `IsByRefLikeAttribute`,
- no `byref`, `TypedReference`, `void`, modified or unreadable field,
- generic parameters declared == arguments supplied at the site.

A local definition is proven inside the image; an external definition is proven
through the supplied references, so one fact covers every instantiation of it. The
identity is the whole instantiation: `Pair<int>` and `Pair<long>` are different
identities and never merge, and the interned alias is derived from the complete
encoded signature.

## External value types

A `valuetype` whose definition lives in a referenced assembly is admitted as:

- an **enum** with its storage width (the earlier milestone), or
- a **struct** that passes the same proof a local struct copy needs.

Identity is the input-scoped `TypeRef` the signature already spells, so a foreign
`TypeDef` token is never resolved against the image being decompiled.

### Unresolved references must not change anything

The first implementation cached every failed lookup as a negative fact, which made a
context with an unusable reference produce a *different* blocker than the byte-only
run. A test caught it. The lookup is now three-state:

- `Value(..)` - the reference answered and admitted the definition;
- `Refused(reason)` - the reference answered and the definition failed the proof
  (`byref-like struct copies need location/lifetime semantics`);
- `Unavailable` - the reference did not answer, so nothing is cached and the caller
  keeps exactly the blocker it had without a context.

That distinction is now a general rule for cross-image facts, not a local patch.

## Owned fixtures

- `rust/tests/dotnet/GenericValueCases.cs` - `Pair<T>`, `Box<T> where T : struct`,
  `Wrapper<T>`; recovered: `Identity`, `Pass`, `Boxed`, `Wrapped`, `Ignore`.
  Still gated: `Sum`, `Peek`
  (`nonlocal/constructed field needs IL fallback`).
- `rust/tests/dotnet/ExternalValueCases.cs` plus a library defining `Measure`,
  `Token`, `Nested`, `Level : byte` and a `ref struct Cursor`; recovered:
  `Identity`, `Pass`, `Keep`, `Wrap`, `Level`, `Ignore`. Still gated:
  `UnsupportedRefStruct` (`byref-like struct copies need location/lifetime semantics`)
  and `UnsupportedField` (`nonlocal/constructed field needs IL fallback`).
- Without the declared reference the external fixture keeps the old blocker, and the
  scan and single-method decompilation agree on every status.

Frozen tables live in `rust/tests/generic_values.rs` and
`rust/tests/external_values.rs`. The older `external_enums.rs` table lost its
`UnsupportedStruct` row because that external struct is now admitted - a promotion
made explicit in the table rather than by loosening the assertion.

## Behaviour gates

- `rust/tests/generic-value-roundtrip.py`: 5 comparisons over 5 recovered methods in
  release and debug, descriptor compares every field of the returned value.
- `rust/tests/external-value-roundtrip.py`: 6 comparisons over 6 recovered methods in
  release and debug, with the library loaded for both the fixture and the recovered
  carrier.
- Mutation control in both: returning a zeroed value instead of the copied one is
  rejected (`expected=Measure{Scale=224,Units=2} actual=Measure{Scale=0,Units=0}`),
  which is what makes the copy observable rather than assumed.

## Corpus result

Full scan of the fixed scope (`report/external-value/corpus-scan-final/baseline.json`):

| quantity | value |
|---|---:|
| Decompiled | 282,330 |
| Partial | 488,418 |
| unrecovered rate | **63.37%** |
| body images within 1% | 33/319 |
| incomplete scans | 0 |
| gain, constructed value types | 3,165 |
| gain, external value types | 7,889 |
| cumulative over the 140,537 anchor | 141,793 |

Changed statuses versus the previous baseline: 11,054, of which 6,800 previously
reported the enum-storage blocker (external value types) and 4,254 reported the
signature blocker (constructed value types). `report/external-enum/check-parity.py`
re-runs the CLI per token with the same reference set: all 11,054 reproduce on their
own, 0 mismatches.

## Independent review

An exec-capable agent reviewed the default readable output against the original IL for
23 pairs (7 local fixture, 8 external fixture, 8 corpus samples) in
`report/value-types/review/review-source.md`: **17 EQUIVALENT, 6
CRUDE-BUT-EQUIVALENT, 0 SUSPECT, 0 INSUFFICIENT-EVIDENCE**. It found no copy/alias,
identity, ordering, gating or blocker-text defect; the crudeness it did note is the
remaining gated field/member access, which is the next capability.

## Limits

- Field reads and member calls on constructed values remain gated; that is the
  constructed-generic-member capability, not this one.
- Nested definitions inside generic enclosing types are not admitted yet.
- The byref-like refusal is based on the defining assembly's own attribute; a
  definition whose attribute cannot be read is refused rather than assumed safe.
