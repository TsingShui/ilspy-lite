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

At the time of this group, a base defined in another image was still refused;
the follow-up below admits the exact-call case. A different generic instantiation,
value-type receivers, same-type delegation outside the constructor context, and
any receiver that is not the original `this` remain subject to their own checks.

## Local measurement (not a batch scan)

Targeted static scans with the production CLI, ILSpy reference policy:

| Image | Before | After | Gain |
|---|---:|---:|---:|
| FSharp.Core | 5,308 | 7,177 | +1,869 |
| System.Private.CoreLib | 13,408 | 13,425 | +17 |
| OpenRA.Mods.Common | 15,261 | 15,413 | +152 |

These three images accounted for +2,038 in the original local measurement; the
subsequent full scans and gained-method reviews are recorded in the checkpoint.
The first F# blocker after this group was a bounded enum/external-value signature
bucket, then `incompatible value/receiver types`.

## Validation

- Owned red/green: `RelationCases.LocalOuter<,>.NestedLocal..ctor` was Partial
  before the change ("constructor call is not the immediate base" path) and is
  Decompiled after it, with the exact `__cil_ctor_call_` token preserved. The
  paired negative, `ForeignOuter<,>.NestedForeign`, keeps a constructed base
  from another image Partial; that case was subsequently promoted by the
  follow-up below after it received its own negative and behavior coverage.
- 469 Rust tests, fmt, clippy and the release build pass. The relation behavior
  driver still executes 178 comparisons / 41 closed selections per configuration
  with eight rejected mutants, and the exception driver 51 comparisons / 6
  mutants. No third-party body was executed.

## Follow-up: exact external constructed base calls

The earlier broad claim from `416cbc4fe` that a reference-constructor `callvirt`
retains the exact non-virtual operation is **withdrawn for constructed local
owners**. A strengthened test showed `this..ctor(seed)` instead of the
`__cil_ctor_call_TOKEN` operation. The virtual-call rendering branch now excludes
constructors, and both ordinary and constructed reference owners keep their
exact constructor operation. The original IL remains authoritative.

An external constructed class `.ctor` MemberRef now uses the TypeSpec owner and
substitutes its type arguments into the declared parameters. The operation carries
the original token rather than selecting a source-language method or inventing an
external implementation. The existing typed receiver, unchanged original `this`,
constructor context and immediate-base checks remain required. This admission
is not for `newobj`, value-type constructors or other constructed members.

The prior swap probe edited the shared TypeSpec used by both `extends` and the
MemberRef parent. The current owned fixture emits two distinct TypeSpec rows for
`OpBase<!1,!0>` and `OpBase<!0,!1>`. Patching only the call operand to the other
MemberRef stays Partial. A separate focused test calls the immediate-base
comparison directly and confirms the correct instantiation matches while the
swapped instantiation does not. The old shared-row probe does not demonstrate a
comparison defect and is not counted as a negative.

Owned default-output behavior runs on both Debug and Release fixtures and on
both `call` and a patched `callvirt`: 12 original/recovered comparisons each,
48 total. The exact base-constructor leaf is an individual operation; recovered
constructor statements execute in a separate carrier, not as an original-method
proxy. Tests cover two generic positions, reference identity, receiver fields,
constructor ordering, an exception and unchecked overflow. Four compiled mutants
(dropped base call, wrong seed, dropped derived write and swapped arguments) are
rejected in all four configurations. Only owned code is executed.

`cargo fmt --check`, clippy with warnings denied, and 492 Rust tests pass. Five
production targeted scans retain all baseline body tokens and show +88, zero
recovered-status losses: ILSpy.Tests +1, ILSpy core +4, Newtonsoft.Json +4,
OpenRA.Game +4 and OpenRA.Mods.Common +75. They are not a complete corpus scan.
Five gained production output/ILSpy/IL triples and the bounded main-session
self-review are in `report/external-base-ctor/`; the later complete batch scan
and its regression repair are in `report/foreign-hidden-batch/`. Reference
substitutions are metadata selections, not runtime proofs.
