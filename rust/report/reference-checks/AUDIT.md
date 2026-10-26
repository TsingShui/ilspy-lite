# Reference identity, conversion and dispatch repair group

The abstract/core cast audit is in `../abstract-casts/AUDIT.md`. This group also
covers local nongeneric interface references using the existing named reference
IR and exact callvirt operation; no C# AST or new dispatch runtime is introduced.

Five pre-change corpus triples are retained in `audit/manifest.json`. All ILSpy
and IL commands succeeded; default output was Partial:

| sample | original IL | classification |
|---|---|---|
| FSharp Layout.Leaf.get_Item2 | load and return TaggedText interface field | ILSPY_YES_OURS_NO |
| Decompiler ParameterizedType.get_Kind | load genericType, callvirt IType.get_Kind | ILSPY_YES_OURS_NO |
| Roslyn SourceComplexParameterSymbol.get_AttributeOwner | return this as declared interface | ILSPY_YES_OURS_NO |
| Roslyn InheritedTypeParameter.get_Name | load interface field, callvirt inherited INamedEntity.get_Name | ILSPY_YES_OURS_NO |
| Cecil CallSite.get_Scope | ordinary calls returning IMetadataScope reference | ILSPY_YES_OURS_NO |

## Bounded implementation

- CLI class signatures can name reference interfaces. Reuse the existing scoped
  named-reference token and reference stack category for local nongeneric
  interfaces; validate Abstract, Interface and null Extends rather than requiring
  a class inheritance root. Do not weaken class-chain validation.
- Interface references can be copied, stored, returned and compared without
  layout or implementation inference. Keep original token identity.
- Existing exact callvirt leaf preserves interface dispatch, null check and
  argument order. Do not turn it into a direct class call or guess a concrete
  implementation from a name. Signature/name rendering does not resolve a slot.
- Prove class-to-interface and interface-to-base-interface assignment only through
  local nongeneric InterfaceImpl edges and the existing class chain. Do not
  assume arbitrary references are compatible; generic/external edges need their
  own facts. Bound traversal and reject cycles.
- Local interface runtime casts use the existing castclass/isinst renderer, not
  static assignability. Allocation still rejects interfaces/abstract classes;
  interface instance fields and constructors remain invalid.
- Keep constructed/generic interface dispatch, foreign interface binding,
  constrained/value receivers and unsupported modifiers gated.

## Validation seams

Public decompile/scan with owned fixtures: signature/field round trips, explicit
and implicit interface implementations, inherited interface assignment, null,
cast failures, boxed value references, fresh receiver state and argument-before-
null-check ordering. Production default text is compiled with only exact leaf
operation adapters, not whole-method proxies. Mutants must expose dispatch and
result errors. Malformed hierarchy and incompatible assignment remain refused.

The core string negative test also exposed a core-identity guard accepting a
WindowsRuntime content-type bit. A wrong public-key/name was already refused;
content type must also be default. The red case is retained in
`core-identity-red.log`; reject that forged scope rather than treating it as the
ordinary core identity. This is an identity-validation correction, not a new
reference-selection policy.

Combine this with abstract/core runtime checks, then run one full regression,
behavior validation, corpus scan and main-session source/IL self-review. No
third-party execution, reference policy or denominator changes.
