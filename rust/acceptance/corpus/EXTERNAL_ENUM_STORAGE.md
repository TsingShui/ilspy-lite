# External enum storage

A value type defined in another assembly is admitted as an enum only when that
assembly was supplied as an explicit reference for the image being decompiled, and
only when its definition passes the same proof a local enum needs: sealed
non-generic value definition, exact core `System.Enum` base, one ordinary public
special `value__` instance field, literal (non-`initonly`) statics, and a
fixed-width integer storage. Nothing is inferred from a name, a directory, or a
failed guess.

## Why this exists

`enum storage requires an exact local definition; external/value signatures need IL
fallback` was the third largest first blocker before this slice (59,837 methods).
The definition of those types lives in an assembly the decompiler was not allowed to
read, so the storage width - and therefore the load extension and store truncation -
was unknown.

Option C (`EXTERNAL_TYPE_FACTS.md`) gives each image exactly the references it
declares. This slice is the first capability that consumes that context, so the
plumbing from `EXTERNAL_REFERENCES_PLUMBING.md` is now load-bearing rather than inert.

## What changed

- `Image::value_type_tokens` collects the value-type identities named by a method's
  own signature and local slots; the scan only asks the context about those.
- `prime_external_enums` asks the context once per candidate, and a resolved fact is
  cached on the image under the same TypeRef token the lowering will ask about. A
  type that does not resolve keeps its existing blocker unchanged.
- `DependencyContext::external_enum_type` resolves the TypeRef through the supplied
  references only, parses the defining image, and runs the same `checked_enum_type`
  proof there.
- `EnumType` now distinguishes a same-image token from a cross-image fact. The
  defining assembly's own name is stored with the fact, so a foreign TypeDef token is
  never resolved against the image being decompiled - the failure mode that first
  made this slice produce `global::@ExternalEnumCases.@Ops` for a `Level` value.
- Loads keep the existing rule (normalize to CLI integer stack bits according to the
  declared width) and stores keep truncating to it; both are now applied with the
  referenced definition's width.

## Owned fixture

`rust/tests/dotnet/ExternalEnumCases.cs` builds twice: `external-enum-library-*.bin`
defines `Level : byte`, `Signed : sbyte`, `Plain` (int), `Wide : long`, `Point`
(struct) and `Widget` (class); `external-enum-*.bin` consumes them through locals,
arguments, comparisons and returns.

Contract (`rust/tests/external_enums.rs`), frozen by owner/name/reason:

- Recovered with the declared reference: `ReadLevel`, `Next`, `IsHigh`, `ReadWide`,
  `ReadPlain`, `Widen`, `Copy`, `UnsupportedClass`.
- Still partial: `UnsupportedStruct` with
  `enum storage requires an exact local definition; external/value signatures need IL fallback`.
- Without any reference the same methods stay Partial with the same reason, which is
  the negative coverage proving the fact comes from the declared input rather than
  from a default.
- `scan_methods_with_context` and single-method decompilation agree on every status,
  and the CLI-level parity test repeats that through `scan` and `decompile`.

## Behaviour gate

`rust/tests/external-enum-roundtrip.py` compiles the recovered bodies into
`ExternalEnumCases.Recovered`, runs them next to the original fixture against the
same library, and compares results for enum values `0, 1, 2` and an all-ones value:

- release and debug: 25 comparisons over 7 recovered methods, no difference.
- `UnsupportedClass` is excluded from execution because its body legitimately keeps
  an input-scoped opaque identity that cannot be compiled; the driver requires the
  skip set to be exactly that one method, so the exclusion cannot grow silently.
- Mutation control: changing the load extension from the declared unsigned width to
  a signed one is rejected (`expected=value:255 actual=value:4294967295`), which is
  what makes the width claim observable rather than assumed.

## Corpus result

Full scan of the fixed scope (614 managed inputs, 319 with bodies, 770,748 target
bodies), each image scanned with exactly the references it declares
(`report/external-enum/corpus-scan-final/baseline.json`, delta against the plumbing
baseline in `report/external-enum/delta.json`):

| quantity | value |
|---|---:|
| Decompiled | 271,276 |
| Partial | 499,472 |
| unrecovered rate | **64.80%** |
| body images within 1% | 33/319 |
| incomplete scans | 0 |
| methods whose previous blocker was external enum storage | 4,304 |
| images that gained | 93 |
| images that lost a method | 0 |

That figure is measured on the final scan that also contains the external call
dispatch slice, and the two are attributed by each method's previous blocker reason:
4,304 enum-storage methods and 2,295 call-dispatch methods, 6,599 in total against the
plumbing baseline (`report/external-call/delta.json`). Before the order-independence
fix described in `EXTERNAL_CALL_DISPATCH.md` the enum slice measured 4,004 methods;
the fix admits the same facts without depending on which method was lowered first,
which is why the number is larger and why every changed status now reproduces on its
own. Largest gains are the Roslyn assemblies, then `ICSharpCode.NRefactory.Tests`,
`OpenRA.Mods.Common` and `gmcs`. Cumulative capability gain over the fixed-corpus
baseline (140,537 recovered methods) is 130,739.

## Limits

- Only storage is admitted. An external enum used as a *generic argument*, a field
  type of a locally defined struct, an array element or a `switch` operand still
  meets its own blockers; this slice does not touch them.
- Conversions between an external enum and its underlying type (`conv.u1` and
  friends) remain a separate gap; the fixture deliberately avoids `(Level)(x + 1)`
  so this slice cannot claim that capability.
- `Point` (external struct value) and opaque external classes are unchanged.
