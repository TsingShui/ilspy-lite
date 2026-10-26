# Constructed generic members

Members reached through a *constructed* type are written against the definition
with the site's arguments, instead of being refused. This covers the declaring
type's own instance members (`this` inside `C<T>`), fields and calls on `C<int>`,
allocation of `C<int>`, and inheritance from a constructed base `Derived<T> :
Holder<T>`.

## Admission rules

- **Open self instantiation.** `C<!0..!n-1>` is the declaring type itself: it is
  equal to `C` for member access and base calls, and needs no substitution.
- **Constructed receiver.** A MemberRef whose parent is a `TypeSpec` resolves to a
  local, concrete definition; the reference signature is read with `!i` replaced by
  the supplied argument, and the owner identity is the whole instantiation.
  `Pair<int>` and `Pair<long>` therefore never merge.
- **Calls.** A `callvirt` on a constructed receiver is written on the receiver
  (`holder.Score()`), so the receiver null check, target-runtime dispatch and
  evaluation order are the original ones. A non-virtual `call` keeps the documented
  placeholder, because C# cannot emit a non-virtual instance call.
- **Allocation.** `newobj` through `C<int>` selects the definition's constructor by
  the reference's parameter count (an overload set with more than one match is
  refused) and yields one fresh instance of the instantiation.
- **Constructed base.** `class_chain` walks a `TypeSpec` base to its definition; the
  constructor-call check resolves the `extends` column the same way, so a base
  constructor through `Holder<!0>` is accepted as the immediate base.
- **Still gated.** External constructed members (`constructed member needs a local
  definition`), interface receivers, external allocation
  (`external allocation needs dependency facts`), and generic *methods*.

## Owned fixture and contract

`rust/tests/dotnet/GenericDeclaringCases.cs` - `Holder<T>` (field of `T`, `int`
field, constructor, virtual `Score`, `Self`), `Derived<T> : Holder<T>` (constructor
and override), `Slot<T>` struct, and `Ops` wrappers. Frozen table in
`rust/tests/generic_declaring.rs`: **no method is expected partial**; the earlier
negatives (constructed base, call, allocation) are now positives, which the table
records by dropping their rows rather than loosening an assertion.

Two supporting assertions keep the substitution honest:
`UnsupportedContent` must render `holder.Value` (the `!0` field of `Holder<int>` as
`int`) and must not contain an open generic placeholder.

## Behaviour gate

`rust/tests/generic-declaring-roundtrip.py`, release and debug:

- 13 comparisons over 8 recovered static wrappers: identity, field read, substituted
  field read, struct copy, virtual call on a constructed receiver, allocation,
  self-returning call and constructed-base allocation.
- The harness creates `Holder<int>` instances through the fixture's own constructor
  and compares original against recovered results, using reference identity for
  returned instances and field contents for returned values.
- Mutation control: reading `Value` instead of `Count` is rejected
  (`Field expected=3 actual=9`).
- Constructor and instance methods are covered by the contract and by the wrappers
  that call them; they are not compiled into the static carrier.

## Corpus result

Full scan of the fixed scope after the demotion
(`report/constructed-members/corpus-scan-demoted/baseline.json`):

| quantity | value |
|---|---:|
| Decompiled | 296,145 |
| Partial | 474,603 |
| unrecovered rate | **61.5770%** |
| body images within 1% | 33/319 |
| incomplete scans | 0 |
| newly recovered | **6,773** |
| images that gained | 103 |
| images that lost a method | **0** |

Largest single gain is **OpenRA.Mods.Common +502**, the assembly whose remaining
blockers were exactly this family; then `System.Xml` +454, `System.Private.Xml`
+443, `FSharp.Core` +387/+386.

## Scan throughput

The corpus scan now uses one process per image and `--threads` workers inside each
process, each with its own image parse and fact caches; records are emitted in
MethodDef order. One image's work is the critical path, and the largest image
(37k bodies) previously cost 24.4 s on its own, bounding the whole scan. With
6 processes x 2 threads the full scan takes **32.3 s** instead of 53.7 s, and the
per-image sum drops from 247 s to 93 s at 4 threads. Accounting is unchanged: every
independently known body is matched exactly once, unknown or missing tokens are
recorded as errors, and the counts are identical to the serial scan.

## Independent review

An exec-capable reviewer compared the default readable output with the original IL for
21 pairs (13 fixture methods, 8 corpus samples) in
`report/constructed-members/review/review-source.md`: **14 EQUIVALENT, 3
CRUDE-BUT-EQUIVALENT, 1 SUSPECT, 1 INSUFFICIENT-EVIDENCE**.

### SUSPECT: a generic base owner is printed with its type argument erased

`Derived<T>.Score()` reads the inherited `int` field `Count` of `Holder<!0>`. Our
readable output casts the receiver to the **definition name alone**:

```csharp
int v1 = ((GenericDeclaringCases.Holder)this).Count;
```

Two problems, both real: `Holder` is an open generic type and therefore not a legal
C# type name, and the cast erases the very type argument this milestone claims to
preserve, so the `int` local is no longer justified by the text. The reviewer is
correct.

**Resolution: demoted, not hidden.** Naming the constructed base needs a C# renderer
for instantiated owners, which this slice does not have; printing the definition name
instead is not acceptable. The shape is now gated with its own reason,
`base field access needs the constructed owner name`, so those methods fall back
honestly. The cost is 12 methods corpus-wide (296,157 -> 296,145), and the fixture
records the row in `EXPECTED_PARTIAL` rather than claiming the method. The same pairs
go back to review with this expectation.

### Re-review after the demotion

The same 21 pairs were reviewed again with the fallback declared as expected:
**15 EQUIVALENT, 3 CRUDE-BUT-EQUIVALENT, 0 SUSPECT, 3 INSUFFICIENT-EVIDENCE**
(`report/constructed-members/review/review-source-rerun.md`). The previously flagged
`fixture-Score` now falls back for the stated reason, the illegal cast and the
arity-erased name are gone, and no other pair erases a type argument or devirtualizes
a call. With zero SUSPECT the numbers above are final for the demoted shape.

The three INSUFFICIENT-EVIDENCE rows are evidence gaps, not code claims: the IL dump
does not print field signatures (so the substituted `int` for `Holder<int>.Value`
cannot be checked from the pair), and two corpus dumps were absent from the refreshed
input set. Both are tooling fixes for the dump generator.

### Other findings

- `fixture-UnsupportedContent` was INSUFFICIENT-EVIDENCE: the IL dump prints the field
  owner but not the field signature, so the emitted `int` cannot be checked from the
  pair alone. The dump should carry the field signature (tooling gap, not a code
  defect).
- Three CRUDE-BUT-EQUIVALENT rows are the documented placeholder forms
  (`__cil_newobj_*`, `__cil_ctor_call_*`, `__cil_direct_*`); the reviewer confirmed
  the in-image token, order and receiver count are preserved.

## Limits

- Interface dispatch, external constructed members and generic methods keep their
  own blockers.
- Constructor overload selection refuses an ambiguous set rather than guessing.
- A constructed receiver whose definition is external is not admitted; that needs
  external member facts, which is the next slice.
