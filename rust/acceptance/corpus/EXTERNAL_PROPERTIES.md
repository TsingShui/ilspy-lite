# Metadata-associated external properties

## Audit before implementation

Six existing first-blocked methods from six assembly families were compared with
ILSpy and original IL. All six were ILSPY_YES_OURS_NO; ILSpy used ordinary property
syntax. No command failures were interpreted as decompilation failures. A blocked
method can have further unsupported operations after the property call, so this
sample does not predict all 13,552 accessor-blocked methods will recover.
See `report/external-properties/AUDIT.md` and its exact-reference manifest.

## Admission and semantics

`ExternalCall` carries an optional metadata-proven property fact. The existing
`resolve_property_accessor` checks MethodSemantics, Property ownership, getter/
setter signatures and ambiguity; prefixes alone do not establish a property.

The bounded source lookup requires:

- exact, nongeneric reference-class owner (not interface/value type);
- public instance ordinary getter/setter and no index parameters, ref returns or
  parameter modifiers;
- canonical accessor name, usable property identifier and exact supported type
  identities in both the definition and reference signatures;
- one same-role accessor and one property with that name, without collisions
  against fields, methods, events or nested types on the declaring owner.

`callvirt` renders access on a cast to the proven declaring class. This selects
that property's slot even when a derived class hides the name, while virtual
overrides still dispatch at runtime. Receiver and argument expressions are the
lift's already-evaluated stack snapshots. The null check remains at the property
access, after argument evaluation; repeated getters are not merged.

A direct `call` keeps its exact-call intrinsic. Turning it into ordinary property
syntax could introduce a null check or invoke an override, so the proof for a
virtual accessor is deliberately not applied to direct calls.

Events, indexers, generic owners, interface receivers, ordinary get_-named methods
without an association, inaccessible/modified accessors and unresolved facts stay
fallbacks. Context-bearing missing references keep their pre-existing detailed
resolution reasons; unrelated references do not supply new property facts.

## Validation

- Failing test before implementation: `external_properties` Get method remained
  Partial with the special-name blocker (`report/external-properties/red.log`).
- Frozen negative rows: Orphan, Indexed, Generic, Interface; malformed association
  flags and inconsistent property signatures also fail closed.
- Scan and separate-method status agreement over the full owned fixture; unrelated
  reference-set output unchanged; patched direct call retains exact-call notation.
- `tests/external-property-roundtrip.py --mutation-control`: compile default
  readable output as-is (no property replacement helper). 98 comparisons over
  seven recovered methods per release/debug fixture configuration. Base, derived,
  hidden and null receivers; getter/setter exceptions; bool storage; repeated
  reads; setter state; throwing argument evaluation before receiver checks.
- Three mutants rejected in each configuration: getter replaced with field read,
  setter replaced with field write, premature null check before argument division.
- Existing external-call behavior: 25 comparisons per configuration and wrong-
  target mutant rejected. Existing direct/base property behavior: 24/16/24/16
  comparisons pass; the direct-dispatch mutant fails its intended target check.
- 374 Rust tests, clippy with warnings denied and cargo fmt checks pass.
- Eight corpus triples self-reviewed against ILSpy and original IL: eight
  CRUDE-BUT-EQUIVALENT for the property transformations, no differences found.
  No third-party assembly executed; no independent or formal proof claimed.

## Same-scope result

Baseline: `report/exception-regions/corpus-scan/baseline.json`.
After: `report/external-properties/corpus-scan/baseline.json`.

| quantity | value |
|---|---:|
| bodies | 770,748 |
| Decompiled / Partial | 338,794 / 431,954 |
| unrecovered rate | 56.04348% |
| gained methods / images | 2,704 / 73 |
| recovered-method regressions | 0 |
| incomplete images | 0 |
| images within 1% | 34 / 319 |

Every gain previously had `external call target is a special-name accessor`
under the opaque-callvirt prefix. OpenRA.Mods.Common gains 240. No input or
reference-selection policy changed.
