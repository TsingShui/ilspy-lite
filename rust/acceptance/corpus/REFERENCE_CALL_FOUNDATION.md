# Scoped reference-call foundation

This group separates exact IL call semantics from optional source-language
presentation. It extends shared binding and reference identity, not just a
single blocker predicate. No C# AST, whole-method proxy or original-IL executor.

## Exact calls versus source lookup

A checked caller signature and original MemberRef token define the stack shape
and exact runtime call operation. Resolving an external definition is necessary
for inferred layout/ancestry/allocation shape or proven source property/member
syntax. It is not necessary to preserve an ordinary reference callvirt as the
existing exact runtime leaf. Unknown/missing/ambiguous references are not guessed
or substituted; they retain their requested scope and diagnostics.

- Opaque ordinary external reference calls retain `__cil_callvirt_TOKEN` and
  its null check, original class/interface dispatch, ordered supplied arguments,
  runtime effects and original resolution/access behavior.
- Foreign constructed reference-class signatures substitute the owner's
  supplied arguments in the caller's proven scope. Direct/static calls retain
  `__cil_reference_call_TOKEN`; callvirt retains its exact callvirt leaf.
  No bare member-name source lookup is invented for these targets.
- Metadata-backed source properties remain a separate proof. Unsupported
  indexer/orphan/interface/generic associations and inconsistent property
  metadata keep exact calls rather than becoming fake source properties.
- MethodSpec/GVM, byref/modifier and value/constrained receiver limitations remain
  independently checked. This group does not classify unknown allocations or
  foreign value layouts as reference objects.

## Reference identity and conversions

Qualified core Object/String/Array identities are recognized across primitive,
local TypeDef and external TypeRef spellings. Core name/key/culture/content
validation remains mandatory; unrelated same-name types are not aliases.
Vectors are assignable to core Array, not implicitly interchangeable with an
arbitrary Array reference. No arbitrary external hierarchy/variance proof.

Known reference TypeSpecs (constructed classes/interfaces and vectors) use the
existing castclass/isinst runtime check. Reference-type unbox.any has castclass
semantics: null passes through, incompatibility throws InvalidCastException,
success keeps the same reference. Unknown Var/MVar and value-type operands do not
become reference checks by guesswork.

An exposed bool-box presentation defect is also repaired: a normalized integer
bool from a call result is converted to Boolean before being boxed. The owned
equality gate checks boxed type, value and fresh identity; it does not compare
separately allocated boxes by cross-run object identity. No new noncanonical
Boolean bit-pattern guarantee is claimed.

## Audits, public tests and behavior

Pre-change corpus comparisons and bounded design:
`report/reference-call-foundation/AUDIT.md`. Six call/conversion examples,
three qualified core String self-returns, plus the exposed owned bool-box case.
Failed tooling is not a BOTH_NO result; independent later blockers remain in
scope. Public tests were red before green for opaque calls, constructed owners,
reference conversions, core identity and bool-box presentation.

- 393 Rust tests; all-target clippy warnings denied and fmt pass.
- Missing/present reference scan parity, counterfeit core key, reference/value
  kind mismatch and scalar receiver/cast negatives remain explicit.
- Invalid/missing source-property association now positively requires exact
  call notation and rejects fabricated property syntax. Promoted cases are not
  merely removed from frozen tables.
- Reference fixture: 107 comparisons / 13 methods for each release/debug and
  ordinary/reference-unbox variant (428 comparisons overall). Five semantic
  mutants rejected per variant/configuration: wrong result/state, dropped
  argument effect, lost clone identity and wrong type-test result.
- Exact test leaf adapters resolve only the original owned caller's MemberRefs
  and emit a single call/callvirt with typed arguments. They are lazy at the
  operation, do not decode selected bodies or proxy recovered support methods.
- Expanded gates per release/debug: properties 135/11; casts 139/17; equality
  155/11; external calls 27/8; virtual calls 55/11; generic references 88/16.
  Their mutation controls pass. MethodSpec gate remains 122/13 with GVM refusal.
- Eleven corpus triples self-reviewed against original IL: three EQUIVALENT,
  eight CRUDE-BUT-EQUIVALENT, no changed semantic differences found. No third-party
  execution or independent/formal proof claim. Metadata-only core toy images are
  never runtime-loaded; owned helper generators are separate executables.

## Unchanged-scope result

Before: `report/method-instantiations/corpus-scan/baseline.json`.
After: `report/reference-call-foundation/corpus-scan/baseline.json`.

| Metric | Result |
|---|---:|
| managed / body-bearing images | 614 / 319 |
| target method bodies | 770,748 |
| Decompiled / Partial | 417,799 / 352,949 |
| overall fallback | 45.79305% |
| percentage-point decrease | 3.91684 |
| gains / images gaining | 30,189 / 226 |
| recovered-method regressions / incomplete | 0 / 0 |
| OpenRA.Game / OpenRA.Mods.Common gains | 247 / 683 |

Prior first-reason attribution: opaque callvirt 20,637; foreign constructed
member 6,908; incompatible receiver 2,208; nonlocal cast 390; box/unbox type 35;
interface declaring type 11. These sum to 30,189, not isolated causal attribution.

The larger group did not produce a ten-percentage-point gain; do not predict
that by summing blocker counts. Reference selection, versions, corpus scope and
denominator are unchanged. The overall <=5% target remains unmet; historical
per-image 1% diagnostics are not the current acceptance gate.
