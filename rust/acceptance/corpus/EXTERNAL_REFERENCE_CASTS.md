# Runtime casts to referenced reference types

`castclass` and `isinst` keep the exact target token the IL already carries; the
only missing fact was whether a referenced `TypeRef` is a class/interface or a
value type, because `castclass` on a value type tests a *boxed* value rather than
producing one.

The dependency context now reads the selected definition's flags and base chain:
a class or interface is admitted, a value type is refused with an explicit
reason, and an unavailable definition keeps `cast target needs dependency facts`.
The cast itself is the original operation (null passes through, `isinst` yields
null, `castclass` throws `InvalidCastException`); identity, argument order and
the owner's null handling are unchanged. No assignability proof, boxing, copy or
allocation is added.

## Evidence

- `report/constructed-base/AUDIT.md` records the mechanism and the measurement.
- Owned red/green: `RelationCases.Ops.AsForeignInterface` and `.CastForeignClass`
  were Partial before the fact existed and are Decompiled after it; without a
  selected library they stay Partial with the dependency-fact reason. An implicit
  upcast carries no IL operation and is not claimed as evidence.
- A crate unit test asserts the fact source itself: `Tagged`/`ITagged`/`Api` are
  reference kinds and the owned `Bits` struct is refused as a value type.
- The relation behavior driver executes 187 comparisons / 44 closed selections
  per configuration and rejects eight mutants, covering null receivers, matching
  and unrelated objects and the thrown `InvalidCastException` type.
- 471 Rust tests, fmt, clippy and the release build pass. No third-party body was
  executed.
- Targeted static scans: NRefactory.CSharp.Refactoring 3,489 -> 3,962 (+473),
  mscorlib 4.5 18,230 -> 18,254, System 13,705 -> 14,038 (+333),
  System.Data.Common 5,243 -> 5,480 (+237), OpenRA.Mods.Common 15,413 -> 15,441.
  These are local measurements; a full batch scan is still due.

## Boxed value-type targets

A value-type target is not a reference cast but a boxed-value test. `BOXED_VALUE_TESTS.md`
records how it is emitted (`x is S ? x : null` for `isinst`, the same test with
`InvalidCastException` for `castclass`); the refusal below applies only to targets
whose value identity cannot be admitted.
