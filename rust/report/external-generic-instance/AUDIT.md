# Instantiated foreign instance methods: ILSpy-first audit

## Upstream mechanism

- `IL/ILReader.cs::ReadAndDecodeMethodReference` resolves every call operand
  through `MetadataModule.ResolveMethod`, which dispatches a `MethodSpec` to
  `ResolveMethodSpecification` and a `MemberRef` to `ResolveMethodReference`.
- `TypeSystem/MetadataModule.cs::ResolveMethodSpecification` decodes the
  instantiation and, for a MemberRef target, calls `ResolveMethodReference` with
  the method type arguments. There the declaring type is resolved from the
  reference parent, the *definition* is selected, `FindMethod` matches the name
  and declared signature against that definition, and the result is specialized
  with both the class and method arguments.
- `IL/ILReader.cs::DecodeCall` then builds `CallInstruction.Create(opCode, method)`
  from that specialized method; the emitted call keeps the original token as its
  identity. `CSharp/ExpressionBuilder.cs` renders an ordinary member invocation.
- A *virtual* instantiated method still dispatches through runtime lookup. The
  existing owned counterexample (reproduced null-receiver and argument-side-effect
  ordering) keeps that class refused.

## Adopted

- A MethodSpec whose target is a MemberRef on an external TypeRef owner now asks
  the dependency context for a fact about the *selected definition*: generic
  arity, instance/static agreement, interface owner, value-type owner, abstract
  and virtual flags. Static targets keep the existing path.
- A nonvirtual instance definition is admitted with its original MethodSpec
  token and caller-scoped signature. The receiver is the exact external class
  identity, so the existing callvirt leaf keeps the null check, argument order
  and dispatch identity.
- Virtual, abstract, interface, value-type-owner, unavailable and malformed
  definitions stay refused, with the virtual case keeping
  `generic virtual dispatch needs runtime lookup-order modeling`.
- The fact is demand-driven, cached per token and primed before lifting so the
  answer cannot depend on which other method ran first.

## Fixed-corpus candidates before implementation

`audit/` retains production default output, ILSpy output, original IL, commands
and reference selections for eight Partial methods whose first blocker was this
family. ILSpy exited successfully without unknown-result diagnostics. Findings:

| Image/token | Result after implementation |
|---|---|
| 1042-0600007E OpenRA `AppearsFriendlyTo` | Decompiled (nonvirtual `HasTraitInfo<T>`). |
| 1042-06000080 OpenRA `NotifyBlocker` | Decompiled (`TraitsImplementing<T>`). |
| 1042-0600009E OpenRA `InnerValue` | Decompiled (`Trait<T>`). |
| 0004-0600007D Roslyn `CreateAnonymousFunctionConversion` | Decompiled (`DiagnosticBag.AddRange<T>`). |
| 0134-060000DF Newtonsoft `get_File` | Still Partial: `System.Data.Entity` is not available, so no definition fact exists. |
| 0158-060000CD Newtonsoft `get_File` | Same unavailable dependency. |
| 0895-06000A0F Roslyn `GetDeclaringSyntaxReferenceHelper` | Still Partial, now at an unrelated value/receiver gate. |
| 1041-0600006D OpenRA `PublicKeyFingerprint` | Still Partial at its original value-type/member gate. |

These are targeted static runs, not a corpus delta; the batch scan follows this
group. No third-party method was executed.

## Validation

- The owned fixture adds a non-generic class with a generic *instance* method and
  a virtual counterpart. The new test was red before implementation (Partial with
  the missing-fact message) and green after it. Its negative cases keep the same
  token Partial when the selected definition is virtual, and when a mutated
  definition flag makes it virtual; an absent reference keeps the original
  refusal. No answer is inferred from the reference signature alone.
- The reused relation driver now binds the planned call leaf through the harness
  (MethodSpec tokens included) and executes 178 comparisons / 41 closed
  selections per configuration, including the new call under null and non-null
  receivers. It rejects eight mutants, among them a changed generic argument.
  Both the ordinary and version-substituted configurations pass.
- The pre-existing external-call fixture promoted its frozen negative
  `UnsupportedGeneric` (a nonvirtual `Service.EchoOf<T>`) to `RecoveredGeneric`
  and gained a new negative `UnsupportedGenericVirtual` (`Service.VirtualEchoOf<T>`)
  that keeps the generic-virtual refusal, so the boundary stays visible instead
  of shrinking. The external-call driver was re-run after rebuilding its
  fixtures.
- Full Rust suite (468 tests), fmt and clippy pass; see `validation.json`. Leaf
  planning and execution remain owned-fixture only.

## Follow-up group: nested and constructed base calls

The same fixed-corpus investigation showed that the largest remaining F# blocker
(`constructor call needs a reference-class owner`, 1,868 bodies) was a *base*
constructor call whose declared base is a constructed generic type, reached from
a nested class. `MetadataModule.ResolveMethodReference` resolves such a base
through the selected definition and specializes it, so the call is an ordinary
direct call; only the receiver's declared base identity has to be compared.

`Image::immediate_constructor_base` compares the declared `Extends` operand with
the call's owner: nominal owners keep the previous token/definition rule, and a
constructed owner must be a constructed TypeSpec whose normalized bytes and
generic scope match the declared base. `signature_types::encoded_type` exposes
the normalized bytes and scopes for that comparison. Normalizing with the owner's
scope is what makes a nested class inheriting its enclosing parameters
comparable.

Owned red/green: `RelationCases.LocalOuter<,>.NestedLocal..ctor` (identity through
an enclosing generic scope) went from Partial to Decompiled; the negative
`ForeignOuter<,>.NestedForeign` keeps a cross-image constructed base Partial. The
first attempt regressed three existing open-generic/base-delegation tests because
it required exact token equality for nominal owners; the definition-based rule
was restored and 469 tests pass.

Targeted scan effect: FSharp.Core 5,308 -> 7,177 (+1,869), System.Private.CoreLib
13,408 -> 13,425 (+17), OpenRA.Mods.Common 15,261 -> 15,413 (+152). These are not a
batch total; a full scan and self-review remain due.
