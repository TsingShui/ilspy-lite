# Static constructors

A `.cctor` body is lowered like any other static void body and presented as the
type's own static constructor. The former blanket refusal is gone; a body that
still needs an unsupported operation now fails with that operation's own reason.

## Why this exists

`empty/static-initializer/runtime-special method needs IL fallback` was a blanket
refusal of every static initializer: **13,335 bodies in 236 of 319 images**, about
2.1% of every large image (`report/fallback-corpus/PER-ASSEMBLY.md`). The
acceptance gate is per assembly at 1%, so no image could pass while that row
existed.

## What is admitted

The refusal in `lift::emit` was
`name.is_empty() || name == ".cctor" || name != ".ctor" && flags & 0x1000 != 0`.
`.cctor` no longer takes part in it. A static initializer is then an ordinary body:
no receiver, `void` return, its own instructions, and the same admission for every
operation as any other method. The blanket reason survives only for the two cases
it actually describes, as `empty/runtime-special method needs IL fallback`: a
method with an empty name, and a runtime-special method that is neither `.ctor` nor
`.cctor`.

Two supporting rules make real initializers lowerable:

- **`static readonly` assignment inside the declaring type's own initializer.** A
  write to a `static readonly` field is now admitted when the enclosing member is
  the `.cctor` of the field's declaring type (`static_field` reports the declaring
  type token, and `Writer::static_constructor` holds the `.cctor`'s owner). Every
  other readonly static write keeps `readonly static field write needs IL fallback`.
  Without this rule `static readonly X = ...` - the standard delegate-cache and
  two-phase-init shape - could never be written.
- **A static field named through a member reference reports its own cause.** A
  `.cctor` of a generic type names `Registry<!0>.Count` through a `MemberRef` whose
  parent is a `TypeSpec`; that is now `generic static field needs the constructed
  owner name` instead of the misleading `external field needs IL fallback`.

## Presentation

`readable::method_header` already rendered a `.cctor` as `static <TypeName>()`; it
now has a body. The initializer is never renamed to an ordinary method, never
merged into another member, and nothing else calls it, so it keeps exactly one
execution at type initialization: the platform's type-initialization protocol, not
a re-implementation of it. `beforefieldinit` is not inferred and not emitted;
whether a type is marked `beforefieldinit` still belongs to its own metadata row.

## Owned fixture

`rust/tests/dotnet/StaticConstructorCases.cs` (built by
`rust/tests/build-static-constructor-fixtures.py` into
`tests/fixtures/static-constructor-{release,debug}.bin`):

| type | shape |
|---|---|
| `Simple` | field initializers merged into one initializer: integer and string literal stores |
| `Accumulator` | not idempotent: reads the total, stores it twice, so a second execution is observable |
| `First` / `Second` | two-phase initialization: `First`'s initializer reads `Second`'s static field, which initializes `Second` first |
| `Sized` | a struct's initializer writes only static state |
| `Failing` | a store followed by `throw new ArgumentOutOfRangeException("value", "static init failed")` |
| `Caches` | delegate caching (`ldftn` + `newobj`) - still gated |
| `Registry<T>` | a generic type's own static field - still gated |
| `ExternalCache` | a `newobj` of a type from another assembly - still gated |

Contract (`rust/tests/static_constructors.rs`), frozen by owner/name/reason:

- Recovered: `Simple..cctor`, `Accumulator..cctor`, `First..cctor`,
  `Second..cctor`, `Sized..cctor`, `Failing..cctor`.
- Still partial, each with its own reason:
  - `Caches..cctor` - `unsupported opcode ldftn at IL_0001` (delegate creation is a
    capability the decompiler does not have anywhere, not a `.cctor` rule),
  - `Registry`1..cctor` - `generic static field needs the constructed owner name`,
  - `ExternalCache..cctor` - `external allocation needs dependency facts`.
- The readable output of every recovered initializer contains `static <Type>()`
  and neither `.cctor` nor `static void <Type>()`; `First`'s initializer contains
  the admitted readonly store and the cross-type read; `Accumulator` keeps exactly
  two stores with the read before the first one.
- Scan and single-method decompilation agree on every status.

## Behaviour gate

`rust/tests/static-constructor-roundtrip.py` decompiles each recovered type with
`decompile --type`, renames the type to `Recovered<Name>` (and its own qualified
stores with it), compiles it against the fixture, and runs the harness
`tests/dotnet/StaticConstructorBehavior.cs`:

- Every original initializer runs first, then every recovered one, in the same
  order, through `RuntimeHelpers.RunClassConstructor`, so each runs exactly once per
  process and the two-phase read observes the same value on both sides.
- Compared: the failure descriptor (exception chain plus `ParamName`/`Message` of
  the thrown object) and the value of every public static field.
- release and debug: **7 state comparisons over 6 recovered types**, no difference.
  A failed initialization poisons its type, so for `Failing` only the failure
  descriptor is comparable; that is stated in the harness rather than worked around.
- The placeholder shells the emitted text calls (`__cil_ldstr_TOKEN`,
  `__cil_newobj_TOKEN`) are validated by the harness against the fixture's own
  tokens: the string tokens must resolve as strings, the constructor token must be
  `ArgumentOutOfRangeException::.ctor(string, string)`.
- Mutation control, rejected in both configurations:
  - `DroppedStore` - one of the two accumulated stores removed:
    `Accumulator.Total expected=12 actual=5`.
  - `DuplicatedBody` - the statements moved into a method that the initializer calls
    twice: `Accumulator.Total expected=12 actual=24`.
  - `WrongLiteral` - the string literal changed:
    `Simple.Label expected="seed" actual="mutant"`.
  - `SwappedArguments` - the exception's arguments swapped:
    `Failing ... expected=...[value|static init failed (Parameter 'value')]`
    versus `actual=...[static init failed|value ...]`.

## What still fails inside `.cctor` bodies

A bounded probe (`rust/tests/static-constructor-probe.py`, output
`report/static-constructor-roundtrip/cctor-probe.json`) scans three named images
with the CLI's default reference discovery. This is not the acceptance scan: it
answers "what does a static constructor still need".

| image | bodies | `.cctor`s | recovered | remaining |
|---|---:|---:|---:|---:|
| `OpenRA.Mods.Common` | 18,767 | 427 | 6 | 421 |
| `Microsoft.CodeAnalysis.CSharp` (tools) | 27,577 | 397 | 19 | 378 |
| `Microsoft.CodeAnalysis.VisualBasic` (tools) | 32,648 | 718 | 20 | 698 |

Before this change all 1,542 of those bodies were refused with the blanket row.

Remaining first blockers, combined over the three images:

| count | reason | what it actually is |
|---:|---|---|
| 1,062 | `allocation owner must be a concrete class` | `newobj` of a local **sealed** class (lambda singleton `<>c`) or of a local **struct** (`newobj` of a value type); the allocation admission refuses `flags & 0x140`, which includes `Sealed` |
| 293 | `unsupported opcode ldftn at IL_XXXX` | delegate / method-group caching |
| 34 | `constructed member needs a local definition; ...` | `newobj`/call through a constructed generic type |
| 13 | `operation needs a core TypeRef` | array/boxing operations on a non-core type |
| 11 each | `array element type needs IL fallback`, `external allocation needs dependency facts`, `external field needs fallback`, `generic static field needs the constructed owner name`, `constructed member parent is not a class instantiation`, `generic/indirect method reference needs IL fallback` | unchanged surfaces |
| 5 each | external enum storage, external constructed value | unchanged surfaces |
| rest | `ldtoken`, `exception/data sections`, `box/unbox`, `unresolved core type identity`, ... | one to three bodies each |

So the blanket row is now a diagnosis. It is **not** yet a gate-sized gain: the
probe images keep 97% of their initializers blocked, because the shape "allocate
one object or array and store it in a static field" needs allocation shapes this
decompiler still refuses. Measured for the next dispatch: relaxing the sealed half
of the allocation-owner mask alone (leaving structs gated) takes
`OpenRA.Mods.Common` from 6 to **375/427** and
`Microsoft.CodeAnalysis.CSharp` from 19 to **320/397** recovered initializers. That
is a separate capability with its own fixture, contract and review; it was measured
here and reverted, not shipped.

## Limits

- Delegate creation stays unsupported everywhere (`ldftn`, `ldtoken`), so a
  cached-delegate initializer keeps its own reason.
- A generic type's own static field cannot be named without the constructed owner
  name, exactly like a constructed base field.
- A failing initializer's pre-throw state is not observable after the platform
  poisons the type; only the failure descriptor is compared.
- `beforefieldinit` ordering is metadata, not text: the emitted member is the static
  constructor either way.
