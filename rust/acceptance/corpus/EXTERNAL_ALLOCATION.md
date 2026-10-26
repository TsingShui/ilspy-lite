# External allocation

`newobj` whose constructor is declared in a referenced image is admitted when the
defining image proves the owner and the constructor shape. The rendered form stays the
existing intrinsic `__cil_newobj_<TOKEN>(args)`, so the input-scoped constructor identity
is preserved; only the facts and an evidence line are new.

## Facts read from the defining image

- owner is a `TypeDef`, neither `interface` nor `abstract` (a static class is
  `abstract | sealed`, so it stays refused); sealed is allowed;
- owner is not a value type (`ValueType`/`Enum` base): `newobj` on a struct is value
  initialisation, not an object identity;
- the constructor is an instance `.ctor`, not static/virtual/abstract, with a default
  calling convention, no vararg, `void` return, and no byref/modified/typedref parameter;
- the owner's name comes from the defining image (`symbol_names::type_name`), the
  declared generic arity must match an instantiation if one is supplied, and an open
  generic definition is refused.

Parameter types are read from the **caller-scoped** `MemberRef` signature, so the same
`assignable` checks run as for a local constructor and no defining-image token enters `Ty`.

## Three-state rule

`Fact` caches the admission, `Refused(reason)` caches the reason and renders
`external allocation needs dependency facts: <reason>`, and `Unavailable` caches
nothing, leaving the method byte-identical to a run without dependency facts. Facts are
primed from the body operands (`prime_external_allocations`) so admission never depends
on which method was lowered first.

## Refusals

Owner is an interface / abstract / a static class / a value type; constructor is static /
abstract or virtual / vararg / not an ordinary instance `.ctor`; parameter is byref,
modified or typedref; open generic definition without an instantiation; arity mismatch;
owner name not usable.

Known gaps: a **constructed** external owner (`Box<int>`) is still gated before the facts
are consulted (`constructed member needs a local definition; external constructed members
need dependency facts`), and a **nested** external owner stays
`nested/generic/unresolved external class needs IL fallback`. Both are separate slices.

## Fixtures and gates

`tests/dotnet/ExternalAllocationCases.cs` builds a library and a consumer
(`build-external-allocation-fixtures.py`); positives return the allocated object
(`MakeNode`, `MakeDefault`, `MakeSealed`, `MakeMixed`), negatives cover a struct owner,
and the constructed/nested shapes named above.

- `cargo nextest run`: the frozen table asserts the four positives decompile in both
  styles with an owner evidence line and exactly one `__cil_newobj_`, that the negatives
  keep their own reasons, and that a run without references never admits any of them.
- `tests/external-allocation-roundtrip.py --mutation-control` compares the runtime type
  and every public field of the returned object for 11 argument cases in release and
  debug; the null-allocation mutant is rejected in both.

## Corpus result

`report/external-allocation/corpus-scan/baseline.json`: **334,213 / 436,535** of 770,748
bodies (56.6354%), 0 incomplete, 33 images within 1%. Versus the defect-sweep baseline
(333,097): **+1,116 across 89 images, 0 lost**, and every gained method previously carried
`external allocation needs dependency facts` as its first blocker.
