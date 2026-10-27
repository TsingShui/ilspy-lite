# Instantiated foreign instance methods

A `call`/`callvirt` on a `MethodSpec` whose target is a `MemberRef` owned by an
external class reference can now be lowered when the selected definition proves
it is an ordinary nonvirtual instance method. This follows ILSpy's
`ResolveMethodSpecification` -> `ResolveMethodReference` -> `FindMethod` path:
the declaration is found on the selected owner definition and specialized with
the class and method arguments, while the call site keeps its original
`MethodSpec` token.

The exact call leaf preserves the receiver identity, argument order, the
`callvirt` null check and the declared signature. A virtual, abstract, interface
or value-type-owner declaration is refused; virtual generic dispatch keeps
`generic virtual dispatch needs runtime lookup-order modeling` because its
lookup order differs from an ordinary argument-then-null-check call. Missing
references, ambiguous or mismatched declarations and malformed signatures keep
their refusals, and the fact is demand-driven and primed per body so it cannot
depend on which method ran first.

Facts are read only from the selected definition's flags and owner type; the
caller's reference signature still supplies the instantiated parameter and
return types. No body, layout or runtime compatibility is inferred.

## Evidence

- `report/external-generic-instance/AUDIT.md` records the upstream call chain and
  eight pre-change candidates. Four decompile in targeted static runs (three
  OpenRA and one Roslyn); two name an unavailable `System.Data.Entity`
  dependency and two stop at unrelated gates. There is no corpus delta claim yet.
- The new owned test was red before implementation and green after; it keeps
  absent references, a virtual selected definition and a mutated virtual flag
  Partial, so no answer is guessed from the signature.
- The reused relation driver binds the planned call leaf by `MethodSpec` token
  and executes 178 comparisons / 41 closed selections per configuration with
  eight rejected mutants, in both the ordinary and version-substituted modes.
  The external-call fixture promoted its frozen `UnsupportedGeneric` negative to
  a recovered case and gained a virtual counterpart as the new negative.
- 468 Rust tests, fmt, clippy and the release build pass. Only owned fixtures
  were executed; no third-party method ran.
