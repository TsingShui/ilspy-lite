# Constructed value calls, construction and boxing

## Fixed-scope milestone

Baseline: scoped reference relationships (`a5b6d4eef`), 472,156 Decompiled /
298,592 Partial of 770,748 bodies. This group: **492,717 Decompiled / 278,031
Partial**, aggregate fallback **36.07287985178035%**, still above the aggregate
5% goal. **20,561** methods recovered in 132 images; no recovered-method losses
and no incomplete images. Scope remains 614 implementation inputs / 319
body-bearing images. Reference-selection policy and denominator are unchanged.
The decrease is 2.66766829106270 percentage points, not an estimate based on
first-blocker totals. Per-assembly reporting remains in the full scan.

| image | Decompiled | Partial | gained |
|---|---:|---:|---:|
| OpenRA.Game | 2,642 | 2,125 | 85 |
| OpenRA.Mods.Common | 11,199 | 7,208 | 83 |

Evidence paths (relative to `rust/`):

- `report/value-calls/AUDIT.md`, reused reference-relations pre-audit triples,
  and `report/value-calls/audit/manifest.json`;
- `report/value-calls/corpus-scan/baseline.json`, `delta.json`;
- `report/value-calls/{regression-final.log,clippy-final.log,release-build.log}`;
- `report/value-calls/behavior-manifest.json` and 23 `*-final.log` files;
- `report/value-calls/review/{manifest.json,review-source.md}`;
- `report/value-calls/milestone.json`.

Large raw reports stay local/ignored; selected documentation and manifests are
versioned. Static corpus coverage is not third-party behavioral validation.

## Semantic boundaries

Ordinary nongeneric MemberRefs on admitted constructed value owners use
`__cil_value_call_TOKEN`. Signature substitution preserves each owner argument
and the caller's Var/MVar scopes. Instance operands are the exact managed
receiver location, never a copied or boxed value. Static calls retain the same
original token and type-initialization semantics. Local generic value bodies
receive a structural open-self managed location.

Local constructed-value fields substitute the declaration and requested
MemberRef signatures in the same caller context. Their exact stored identities
must match, not just their stack widths or names. A forged MemberRef that asks
for string where Count declares int fails. Readonly-store and initonly-address
permissions remain separate.

Constructed value `newobj` uses `__cil_value_newobj_TOKEN`: initialize separate
value storage, run the original constructor with already-evaluated arguments,
then copy the completed value. It is not a fresh reference-class identity and
is not ordinary zeroing without a constructor. A throwing constructor preserves
its observable static/byref effects and does not perform a later destination
assignment. Reference allocation and unsupported constructor forms keep their
own proofs/gates.

Box/unbox.any now admit proved local/foreign value identities and checked
caller-scoped TypeSpecs. Non-nullable values box by copy with fresh identity;
reference instantiations retain identity; nullable instantiations retain CLR
nullable boxing and null behavior. Unbox.any checks the exact target type,
retains reference identity where applicable and copies stored values rather
than borrowing boxed storage. A managed address cannot masquerade as its
contents. Existing Boolean byte-preserving storage helpers remain in effect.

This does not admit `unbox` addresses, ref-like layouts, arbitrary foreign
fields, byref results or unknown metadata modifiers. The shared constructed
class-owner binder remains class-only. Generic value MethodSpec calls, generic
static-field gaps, ordinary constrained dispatch and the known GVM runtime
lookup-order issue remain separately gated.

## Owned validation

Six public-seam value-call tests include red/green ordinary calls, constructors,
boxing and a definition-vs-operand signature counterexample. Full regression:
**411 tests**, fmt and clippy green.

`value-call-roundtrip.py` compiles production default bodies, binds local support
calls to recovered Read/Write/Echo/ReadThrow/constructor/argument bodies and uses
actual typed pointers to shared receiver cells. Foreign leaves execute one
original call/constructor operation, not selected method-body proxies.
Per configuration: **522 comparisons / 58 closed selections**, ten rejected
mutants covering receiver copies, lost stores, delayed boxing, wrong unboxed
values, constructor fields, early constructor exceptions and argument traces.
Runs include int/string/object/nullable/reference-bearing value contexts,
constructor writes before failure and unchanged targets after failed newobj.

Legacy promotions stay executable: generic-value Sum/Peek (8 comparisons / 7
methods), generic declaring (30 / 13), scoped generics (1,410 / 250 closed
contexts / 25 methods), struct copies (496), struct receivers (38 / 19), each
per configuration with their retained mutation controls. The 23-driver manifest
includes the other related regression gates. No third-party assembly executes.

Ten production methods were self-reviewed against default source, ILSpy and
original IL: ten CRUDE-BUT-EQUIVALENT, zero changed-semantic SUSPECT in the bounded
sample. This is main-session self-review, not independent or formal verification.
CVec.Minus is not claimed recovered; its ILSpy unknown-result evidence remains
inconclusive and never changes the corpus scope.
