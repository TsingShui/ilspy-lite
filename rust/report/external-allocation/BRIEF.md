# Brief: external allocation (`newobj` on a type from a referenced image)

Owner: worker. Dispatch after the defect sweep lands (the sealed-owner fix is a
prerequisite for one fixture row and must be reported as its own delta).

Design: `report/external-allocation/DESIGN.md` (read it fully; its section numbers are
referenced below). Why now: 24,488 first-blocked bodies and 27 of the 131 rows in the 38
images that are within 10 recoveries of passing, so it moves more assemblies than any
other remaining capability.

## Scope

Admit `newobj` whose constructor is a `MemberRef` on a type defined in a supplied
reference: read the owner's kind (class, not interface/abstract; sealed allowed), that it
is not a value type, its name, its generic arity, and the constructor signature with
parameter types, then render `new Owner(args)` / `new Owner<int>(args)` with arguments in
IL order and one fresh identity per site (DESIGN sections 1-4).

Use the three-state cross-image fact rule from `EXTERNAL_TYPE_FACTS.md`: `Value`,
`Refused`, `Unavailable`; an `Unavailable` answer must leave the caller byte-identical to
the no-context run, and a failed lookup is never cached as a negative fact. The owner name
always comes from the defining image; a foreign token is never resolved against the
decompiled image.

## Refusals (stable, one reason each, DESIGN section 5)

Owner is interface/abstract/static; owner is a value type; constructor is vararg;
parameter type is byref, modified or pointer; generic definition without a usable name;
fact source unavailable. These are negative contract rows, not silent falls-through.

## Integration

- `allocation_constructor` currently answers only for a local `TypeSpec` parent; a plain
  `TypeRef` parent returns `Ok(None)`. Extend it, do not fork a second path.
- Keep the `one fresh identity per newobj` rule from `LOCAL_ALLOCATION.md`; a constructor
  that throws must not leave a half-initialized alias.
- The local path's sealed mask is fixed by the defect sweep; this slice must not re-fix it
  but must exercise it through `NewSealed`.

## Deliverables (DESIGN section 7)

1. `tests/dotnet/ExternalAllocationLibrary.cs` + `ExternalAllocationCases.cs` + build
   script producing release and debug fixtures: positives Plain/Sized/Mixed/Sealed/
   `Box<T>`/nested/Throwing, negatives struct-owner, vararg, byref parameter.
2. `tests/external_allocations.rs` frozen table with exact reason equality for negatives
   and a row freezing the owner evidence text.
3. `tests/external-allocation-roundtrip.py --mutation-control`, release and debug:
   returned value, observable fields, argument aliasing, thrown exception type, identity
   (two calls distinct, one call single). Mutations: swapped arguments, definition name
   where instantiation is required, cached instance across calls, abstract owner admitted,
   dropped argument.
4. `acceptance/corpus/EXTERNAL_ALLOCATION.md` + README index.
5. Report with two deltas reported separately (sealed-mask shift vs this slice) and the
   gate-distance table after the slice.

## Gates

Full corpus scan with `--references declared` against
`report/boxing/corpus-scan-enum/baseline.json`, zero lost methods/images, per-image gate
table, reason-mix change. Full nextest, clippy `-D warnings`, fmt. No third-party
execution. No commits.
