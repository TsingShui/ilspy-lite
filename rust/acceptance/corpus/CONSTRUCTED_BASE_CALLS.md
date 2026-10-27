# Nested and constructed base constructor calls

## ILSpy mechanism

`IL/ILReader.cs::DecodeCall` creates a `CallInstruction` for a base `.ctor`
from the method resolved by `MetadataModule.ResolveMethod`. `ResolveMethodReference`
selects the declaration on the resolved declaring type and specializes it with
that type's arguments, so a base call is an ordinary direct call on the selected
definition. Nothing in the chain depends on the *spelling* of the base.

## Adopted

A base constructor call is admitted when the receiver's declared base denotes the
same type as the call's owner:

- A nominal owner (`Class`/`ExternalClass`) matches when the declared base is that
  token, or a constructed base whose selected definition is that token (the
  pre-existing non-constructed behavior).
- A constructed owner (`StructuralRef`) matches when the declared base is a
  constructed type whose normalized bytes equal the owner's normalized bytes in
  the same generic scope. This covers nested classes that inherit their enclosing
  type's parameters, including a non-identity substitution such as
  `OpBase<TValue, TKey>`.

Scope for the comparison comes from the call's owner, because both operands are
written in that substitution. Arguments, receiver identity, the "original this"
requirement, evaluation order and the exact constructor token are unchanged; the
proof adds no cast, allocation or dispatch.

Still refused: a base defined in another image (needs a dependency fact), a
different generic instantiation, value-type receivers, same-type delegation
outside the constructor context, and any receiver that is not the original
`this`.

## Local measurement (not a batch scan)

Targeted static scans with the production CLI, ILSpy reference policy:

| Image | Before | After | Gain |
|---|---:|---:|---:|
| FSharp.Core | 5,308 | 7,177 | +1,869 |
| System.Private.CoreLib | 13,408 | 13,425 | +17 |
| OpenRA.Mods.Common | 15,261 | 15,413 | +152 |

These three images account for +2,038. A complete fixed-corpus scan and gained
method self-review are still required before any batch total is reported. The
first F# blocker after this group is a bounded enum/external-value signature
bucket, then `incompatible value/receiver types`.

## Validation

- Owned red/green: `RelationCases.LocalOuter<,>.NestedLocal..ctor` was Partial
  before the change ("constructor call is not the immediate base" path) and is
  Decompiled after it, with the exact `__cil_ctor_call_` token preserved. The
  paired negative, `ForeignOuter<,>.NestedForeign`, keeps a constructed base
  from another image Partial, so no answer is guessed locally.
- 469 Rust tests, fmt, clippy and the release build pass. The relation behavior
  driver still executes 178 comparisons / 41 closed selections per configuration
  with eight rejected mutants, and the exception driver 51 comparisons / 6
  mutants. No third-party body was executed.
