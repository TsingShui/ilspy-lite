# External call dispatch

An instance method declared in a referenced assembly is written as a call on the
receiver itself, replacing the opaque placeholder that was emitted before. The
declared name and declaring type come from the referenced definition; which override
a virtual call reaches stays a target-runtime decision, exactly as the original
`callvirt` left it.

## Why this exists

`callvirt on an opaque external receiver needs dependency facts` was a 40k-row first
blocker. The receiver's static type was known (an input-scoped external identity), but
nothing established what the call target was called, so no faithful call could be
written.

## Admission rule

`DependencyContext::external_call` resolves the operand through the supplied
references only and records the fact; `prime_external_calls` asks about every
MemberRef the body names, so the answer does not depend on lowering order. A call is
rendered on the receiver only when the resolved target is:

- an instance method (not static),
- declared on a class rather than an interface,
- not a generic method and not a generic instantiation,
- not a constructor,
- not a special-name accessor (`get_`/`set_`/`add_`/`remove_`, or the special-name
  flag), because writing those as ordinary calls would bypass property binding,
- a plain identifier that can be written after `@`.

Everything else keeps its previous blocker. Interface dispatch, generic
instantiation, accessors and static targets are therefore unchanged surfaces.

## Owned fixture

`rust/tests/dotnet/ExternalCallCases.cs` builds a library (`Service` with virtual,
non-virtual and generic methods, `DerivedService`, `Sealed`, `Abstract`, `Concrete`,
`IWidget`, `Gauge`) and a consumer. Contract (`rust/tests/external_calls.rs`), frozen
by owner/name/reason:

- Recovered with the declared reference: `CallVirtual`, `CallDirect`, `CallEcho`,
  `CallDescribe`, `CallSealed`, `CallTwice`, `CallAbstract`.
- Still partial: `UnsupportedInterface`
  (`callvirt on an opaque external receiver needs dependency facts`) and
  `UnsupportedGeneric` (`generic/indirect method reference needs IL fallback`).
- Without the reference the same calls stay gated with the same reason.
- Scan and single-method decompilation agree for every status.

## Behaviour gate

`rust/tests/external-call-roundtrip.py` compiles the recovered readable bodies, then
compares original and recovered results for each selected method over a base instance,
a derived instance and `null`, plus integer argument edge values:

- release and debug: 25 comparisons over 7 recovered methods, no difference. The
  derived instance makes a devirtualized call visible, and the `null` case keeps the
  receiver null check observable.
- Mutation control: retargeting one dispatch from `Virtual` to `Direct` is rejected
  (`CallVirtual expected=value:1 actual=value:2`), so the target name is not
  cosmetic.

## Corpus result and parity

Full scan of the fixed scope (`report/external-call/corpus-scan/baseline.json`,
delta against the plumbing baseline in `report/external-call/delta.json`):

| quantity | value |
|---|---:|
| Decompiled | 271,276 |
| Partial | 499,472 |
| unrecovered rate | **64.80%** |
| body images within 1% | 33/319 |
| incomplete scans | 0 |
| methods gained vs plumbing baseline | 6,599 (4,304 from enum storage, 2,295 from call dispatch) |
| images that gained | 93 |
| images that lost a method | 0 |

`report/external-enum/check-parity.py` re-runs the production CLI per token with the
same reference set: all 6,599 changed statuses reproduce exactly on their own
(0 mismatches), plus a 400-method sample of statuses that stayed Partial.

## Order independence (defect found by review, fixed here)

The external-enum review flagged one corpus method that a scan reported as recovered
while single-method decompilation still fell back. The cause was real: admission
depended on facts another method had primed earlier in the same scan. Enum candidates
now also come from the signatures of the members the body calls, so each method primes
what it needs by itself. The fix raised the corpus result by itself (thousands of
methods) and is covered by the per-token parity check above.

## Independent review

An exec-capable agent reviewed the default readable output against the original IL
for the nine fixture methods and eight corpus samples whose previous blocker was this
one (`report/external-call/review/review-source.md`): **7 EQUIVALENT, 10
CRUDE-BUT-EQUIVALENT, 0 SUSPECT**. It confirmed that each emitted call names the IL's
member on the IL's receiver with evaluation order and virtual dispatch intact, and
that the non-virtual method is still emitted as a member access (so the null check is
not dropped). Its only actionable remark was that an interface receiver reported the
general blocker rather than naming the refusal; the blocker now keeps that stable
prefix and appends the concrete reason (for example "external call target is declared
on an interface").

## Limits

- Which override runs is not proven statically; it is left to the runtime, which is
  what the original instruction did.
- Accessor, interface, generic and static targets are unchanged surfaces with their
  own reasons.
- The recovered call uses the defining assembly's type names, so a caller that never
  supplies that assembly still sees the placeholder.
