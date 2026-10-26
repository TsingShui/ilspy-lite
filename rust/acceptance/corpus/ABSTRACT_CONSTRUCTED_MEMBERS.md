# Abstract constructed member owners

## Audit and narrow repair

Four first-blocked methods across FSharp.Core, Decompiler and the two Roslyn
assemblies were compared with ILSpy and original IL before implementation.
ILSpy recovers ordinary calls on abstract generic classes. All four were
ILSPY_YES_OURS_NO; none was BOTH_NO. Some still have unrelated constrained-call,
generic-result or signature blockers, so the bucket size was not a gain estimate.
Evidence: `report/abstract-members/AUDIT.md`.

`constructed_member_owner` is a shared lookup, not an allocation proof. It
rejects interface owners but permits abstract classes, whose members and base
constructors can be called on existing derived instances. The independent
`allocation_constructor` guard still rejects abstract/interface owners for
`newobj`. No type representation, dispatch operation, signature substitution
or receiver-assignment rule changed.

## Tests

- Red before green: calls through `Base<int>` were refused by the concrete-owner
  test; the allocation mutation exposed the same overly early refusal.
- `abstract_members.rs`: virtual/nonvirtual calls recover; the complete fixture
  has frozen interface refusal and explicit bodyless declarations; scan and
  method status agree. Setting Abstract on the allocation target still returns
  `allocation owner must be a concrete class`.
- `tests/abstract-member-roundtrip.py --mutation-control`: default readable
  call-site bodies execute with structural aliases rebound from their evidence
  and the established newobj primitive. 22 comparisons over four methods per
  release/debug configuration check nulls, distinct concrete overrides and
  repeated-call receiver state. Two wrong-dispatch mutants fail.
- Existing generic-declaring behavior: 18 comparisons/11 methods, wrong-field
  mutant rejected. Existing allocation behavior: 29 comparisons/9 methods,
  cached-identity mutant rejected. Both configurations pass.
- 377 Rust tests, clippy warnings denied and fmt pass.
- Eight corpus triples self-reviewed against ILSpy and IL: no differences found
  for the admission change. No third-party code executed, no independent proof
  claimed.

## Same-scope result

Before: `report/external-properties/corpus-scan/baseline.json`.
After: `report/abstract-members/corpus-scan/baseline.json`.

| metric | value |
|---|---:|
| bodies | 770,748 |
| Decompiled / Partial | 343,545 / 427,203 |
| unrecovered | 55.42707% |
| gains / images gaining | 4,751 / 50 |
| recovered-method regressions | 0 |
| incomplete images | 0 |
| images within 1% | 34 / 319 |

Every gain previously had `constructed member parent must be a concrete class`.
OpenRA.Game and OpenRA.Mods.Common each gain four. Interface dispatch, unsafe
allocation, missing definitions, unsupported signatures and constructed-owner
spelling limitations are not loosened by this change.
