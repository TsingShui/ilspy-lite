# Independent semantic review: boxing (core scalars + local enums)

Reviewer scope: read-only. **Nothing was built, and nothing was executed as test code.**
The only execution performed was the decompiler itself (`target/release/ilspy-lite list`,
`... decompile --method`, `... il --method`) plus `cat`/`stat`. No `cargo`, no `dotnet`,
no `corpus.py`, no source edit.

- OURS = output of `rust/target/release/ilspy-lite` (bytes 1,632,000, mtime
  2026-10-26 22:44:29), re-run for every pair in this review.
- REFERENCE = `report/boxing/review/ilspy/*.txt` (ILSpy).
- Third evidence = `ilspy-lite il` (original IL).

Freshness note up front: `report/boxing/review/current/*.source.txt` matches the binary
I ran; `report/boxing/review/dumps/*.source.txt` is one revision older and still shows
`fixture-BoxEnum` gated with the pre-local-enum reason, so every quote below is against
the live binary, not against `dumps/`.

Corpus reference set: `--no-auto-references` alone reproduced all six bodies byte for
byte against `current/*.source.txt`, so the reference set was effectively reproduced and
no pair was downgraded for that reason.

## 1. Verdict table

| # | pair (owner::method / token) | box: one fresh instance, declared type, value copied | unbox: null/wrong-type, value copied out | emitted C# type-checks | order / receiver count | gate honoured (and does ILSpy recover it) | verdict |
|---|---|---|---|---|---|---|---|
| 1 | `GenericDeclaringCases.Ops::BoxInt` 0x0600000E | yes: `object v1 = ((object)(value));` - one box of `int32`, copy | n/a | yes | `ret` of the single box - matches | n/a (admitted) | **EQUIVALENT** (ILSpy `return value;`) |
| 2 | `Ops::BoxEnum` 0x0600000F | yes: final box is over `(GenericDeclaringCases.Kind)`, not over the `int`/`byte` - one fresh box of the enum | n/a | yes | single arg, matches | n/a (admitted this slice) | **CRUDE-BUT-EQUIVALENT** (round trip `Kind -> byte -> int -> Kind -> box`; see 4.3, non-defect) |
| 3 | `Ops::BoxStruct` 0x06000010 | n/a (gated) | n/a | n/a | n/a | gated, reason `box/unbox needs an admitted core scalar or local enum` is accurate (`Slot`1<int32>` is a constructed value, not admitted). **ILSpy recovers it** (`return slot;`) -> next-slice evidence, not a defect | **EQUIVALENT** |
| 4 | `Ops::UnboxInt` 0x06000011 | n/a | yes: `int v1 = ((int)(value));` - C# unboxing cast, NRE on null, ICE on wrong type, copies out | yes | matches | n/a (admitted) | **EQUIVALENT** (ILSpy `return (int)value;`) |
| 5 | `Ops::UnsupportedBoxGeneric<T>` 0x06000012 | n/a (gated) | n/a | n/a | n/a | gated, reason accurate (`box !!0`). **ILSpy recovers it** (`return value;`) -> next-slice evidence | **EQUIVALENT** |
| 6 | `RangeInt32@4893::System-Collections-IEnumerator-get_Current` 0x060020AF (FSharp.Core) | yes: `box Int32` of the call result, one instance | n/a | yes | `this.state` then `null`, then one 2-arg call and the box - same order, same receiver count | n/a | **EQUIVALENT** (ILSpy `return current@4718(state, null);`) |
| 7 | `CompoundAssignment::Test` 0x0600015D | yes: two boxes, `a` then `b`, each fresh, both `int32` | n/a | yes | ldstr, box a, box b, call - exact order | n/a | **EQUIVALENT** |
| 8 | `CecilExtensions::OffsetToString` 0x0600007D | yes: `box Int32` of `offset` | n/a | yes | ldstr, box, `String::Format` - exact | n/a | **EQUIVALENT** |
| 9 | `LanguageVersionExtensions::Localize` 0x06001BF4 | yes, and correctly **as `int`**: the IL token is literally `box [System.Runtime]System.Int32` on an enum-typed argument, so boxing the storage integer is what the IL asks for; ILSpy does the same (`return (int)value;`) | n/a | yes (`(int)value` is the legal enum->int explicit conversion; `(object)v0` boxes `int`) | matches | n/a | **EQUIVALENT** |
| 10 | `Boxes::Box` 0x060007C9 | yes on the third path (`((object)(i))`); the two singleton paths copy a reference and box nothing, exactly as `ldsfld; ret` | n/a | yes | `i==0` -> zero, `i==1` -> one, else box: same branch order and same number of `ldarg.0` reads (3) | n/a | **EQUIVALENT** (shape is `goto` soup vs ILSpy's `switch` expression - control-flow style, not box/unbox) |
| 11 | `GenericParameter::get_Name` 0x06000432 | yes: `box Int32` of `this.position` | n/a | yes | `"!"`/`"!!"` then box then `String::Concat`, then `set_Name` on `this`, then return the same concat value (the `dup`/`stloc.0` idiom) - order, both `get_Name` receiver uses and the `set_Name` receiver match | n/a | **EQUIVALENT** |
| 12 | `EnumCases.S8Ops::UnsupportedUnbox` 0x0600000F (enum-release.bin) | n/a (gated) | not judgeable - row is `Partial` | n/a | n/a | **not honoured for the stated reason**: see 4.1. No ILSpy reference dump exists for this token | **SUSPECT** (gate reason) |
| 13 | `EnumCases.S32Ops::UnsupportedUnbox` 0x06000052 (enum-release.bin) | n/a | cast to the enum is right in intent (`(EnumCases.S32)value` = unbox, NRE/ICE preserved) but the row is **not well typed** | **no** - see 4.2 | n/a | n/a (row is `Decompiled`) | **SUSPECT** (ill-typed statement) |

Counts: **13 pairs - 10 EQUIVALENT, 1 CRUDE-BUT-EQUIVALENT, 2 SUSPECT,
0 INSUFFICIENT-EVIDENCE.**

Both SUSPECT rows are the two "extra" pairs, and for both of them the ILSpy reference
dump was never produced (`report/boxing/review/ilspy/` holds only the five fixture dumps
and the six corpus dumps). Their comparison axis is therefore unavailable and they are
judged from the IL plus C# statement-level typing, which is sufficient for both findings:
one is a diagnostic/coverage error, the other is a statement that cannot compile.

## 2. What the admitted rows get right

- Every emitted `box` corresponds to exactly one IL `box`, at the same point in the
  argument/return evaluation, and materialises a **new** local before the consumer, so
  there is no aliasing of the original storage: the value is copied at the box, matching
  ECMA-335 `box` semantics (`src/lift.rs:1276-1303`).
- Every emitted `unbox.any` is a C# unboxing cast on an `object`-typed operand
  (`src/lift.rs:1304-1339`), which carries the required NRE-on-null and
  ICE-on-wrong-type behaviour and copies the value out; the milestone's
  `UnboxInt` row is semantically identical to ILSpy's.
- No pair shows a duplicated or dropped box, an argument-order change, or an extra
  receiver read.
- The two genuine gates (constructed value `Slot`1<int32>`, unconstrained `!!0`) stay
  gated with a reason that describes the actual obstruction, and ILSpy recovers both -
  correct scoping, correctly labelled, next-slice evidence only.

## 3. Corpus pairs, evidence detail

- `0000-060020AF` FSharp.Core: `ldfld state; ldnull; call current@4718; box Int32; ret`
  vs ours `v1 = this.state; v2 = call(v1, null); v3 = (object)v2; return v3;`. Same
  receiver, same arity, same position of the box. (Ours labels the method
  `private virtual` where ILSpy shows no modifier, and renders the name as the hex
  alias `__name_5379...`; both are naming/flags presentation outside box/unbox.)
- `0002-0600015D`, `0003-0600007D`: box positions and order identical; only the
  `__cil_ldstr_*` string indirection differs from ILSpy's literal.
- `0004-06001BF4`: the box token is `System.Int32` while the parameter is an enum, so
  boxing the storage integer is mandated by the IL, not a rendering slip; ILSpy agrees.
- `0006-06000432`: the `dup; stloc.0; call set_Name; ldloc.0; ret` idiom is rendered by
  reusing the `Concat` result for both the argument and the return, which preserves the
  single `Concat` call and the `set_Name` receiver; the boxed `position` is passed as the
  second `Concat` argument, as in the IL.

## 4. SUSPECT rows

### 4.1 `EnumCases.S8Ops::UnsupportedUnbox` (0x0600000F) - gate reason does not describe the IL

OURS (binary under review):

```csharp
public static EnumCases.S8 UnsupportedUnbox(object value)
{
    // IL fallback: incompatible value/receiver types
    // .maxstack 8
    // .initlocals false
    // .code-bytes IL_0000: [02, A5, 02, 00, 00, 02, 2A]
    // IL_0000: ldarg.0 0
    // IL_0001: unbox.any 0x02000002 EnumCases.S8
    // IL_0006: ret 
}
```

ILSpy reference: **none exists** for this token (`report/boxing/review/ilspy/` has no
`EnumCases.*` file). IL (`ilspy-lite il`, third evidence):

```
IL_0000: ldarg.0 0
IL_0001: unbox.any 0x02000002 EnumCases.S8
IL_0006: ret
```

Finding: the IL is the *same shape* as `S32Ops::UnsupportedUnbox`, which this milestone
admits and renders, yet the row is `Partial` and the recorded reason
`incompatible value/receiver types` describes an internal type-model mismatch rather than
anything wrong with the input. Cause, read from the working-tree source (not executed):
the `unbox.any` arm pushes the **raw** storage type (`t.storage.ty()`, `I1` for `sbyte`;
`src/lift.rs:1315-1326`) while the consumer normalises (`Ty::Enum(t).stack()` ->
`storage.ty().stack()`, i.e. `I4`; `src/ir.rs:313` and `292-316`), so `assignable` fails
(`src/metadata.rs:3070-3113`) and reports `incompatible value/receiver types`. `int`/`long`
storage happens to agree, which is why only the narrow/unsigned rows are hit.

Blast radius beyond this pair (same one-line cause, all currently `Partial` with the same
wrong reason): `U8Ops`, `S16Ops`, `U16Ops`, `U32Ops`, `U64Ops` `UnsupportedUnbox`; these
are frozen as expected in `rust/tests/enums.rs:170-260`, i.e. the fixture contract
enshrines the accident. The milestone's own
`report/boxing/DEFECT-enum-unbox-cast.md` addendum already records this; this review
confirms it independently and adds that the cause survives in the current source (the
fix in `src/lift.rs` normalises only the *rendered text*, not the pushed stack type).

Severity: diagnostic + coverage (six rows silently stay unrecovered), not a
wrong-output risk. It is due to the diff: before this slice these rows carried the
accurate `box/unbox needs an admitted core scalar` text.

### 4.2 `EnumCases.S32Ops::UnsupportedUnbox` (0x06000052) - emitted statement is not well typed

OURS (binary under review):

```csharp
public static EnumCases.S32 UnsupportedUnbox(object value)
{
    // enum storage token=0x0200000A alias=__cil_enum_0200000A underlying=int: global::@EnumCases.@S32; ...
    int v1 = ((EnumCases.S32)(value));
    return unchecked((__cil_enum_0200000A)(v1));
}
```

ILSpy reference: **none exists** for this token. IL (third evidence):

```
IL_0000: ldarg.0 0
IL_0001: unbox.any 0x0200000A EnumCases.S32
IL_0006: ret
```

Finding: `(EnumCases.S32)(value)` has type `S32`, which has no implicit conversion to the
`int` local the writer chose, so `int v1 = ...` is CS0266 - the statement does not
type-check (criterion (c)). The stack model is right; only the rendering of the
storage conversion is missing. `__cil_enum_0200000A` is not itself a defect: the
`--details` view declares it as `using __cil_enum_0200000A = global::EnumCases.S32;`.

Important qualification, and the reason this row is reported against the *artifact* and
not against the tree: `src/lift.rs` is newer (2026-10-26 23:01:23) than the reviewed
binary (22:44:29) and already wraps the cast in the storage type
(`src/lift.rs:1315-1326`, rendering `((int)((EnumCases.S32)(value)))`), and
`rust/tests/enums.rs:277-300` already asserts exactly that text. So the built artifact
under review is **behind the source and behind its own tests**: that assertion cannot
pass against the binary I was given, and the release scan in
`report/boxing/VERIFICATION.md` was taken with this binary. A rebuild plus a re-run of the
text gate is required before the slice can be called green; the underlying defect itself
is a one-line rendering bug, now fixed in source but unverified.

### 4.3 A non-defect worth recording: `BoxEnum` is crude, not wrong

OURS: `int v0 = unchecked((int)(unchecked((byte)(kind)))); object v1 = ((object)((GenericDeclaringCases.Kind)(v0)));`
vs ILSpy `return kind;`. The final box is over the enum type, exactly one instance, and
`byte -> int -> Kind` under `unchecked` is value-preserving because the value came from
the byte storage, so this is correct but needlessly indirect. Not a defect; noted so the
next slice can drop the round trip.

## 5. Residual risks

1. **Artifact/source skew (highest).** `target/release/ilspy-lite` (22:44:29) predates
   `src/lift.rs` (23:01:23) and `src/metadata.rs` (22:58:36); the release scan and the
   `current/` dumps in this review therefore describe source that is no longer the head
   of the working tree. Every gate needs a rebuild before it counts.
2. **Readability of the evidence set.** `report/boxing/review/dumps/` is one revision
   stale (BoxEnum still gated there); anyone re-deriving verdicts from `dumps/` instead of
   `current/` will get a different BoxEnum result.
3. **Six narrow/unsigned enum unbox rows** stay `Partial` for the reason in 4.1 and are
   frozen as expected in the fixture contract, so the contract will keep hiding the bug
   after a rebuild unless those rows are re-baselined.
4. **No ILSpy reference for the two extra pairs**, so the "same shape as ILSpy" part of
   their verdict is unverified; only the IL and C# typing axes are evidenced.
5. Enum boxing renders through `display()` while other enum positions render through the
   `__cil_enum_*` alias (`src/ir.rs:251`, `320-326`); both happen to be well formed, but
   the two paths are easy to diverge again.

## 6. Statement of method

Nothing was built and nothing was executed as test code; the only processes run were
`ilspy-lite list`, `ilspy-lite decompile` (with and without `--details`) and
`ilspy-lite il` against the fixtures and the ILSpy-tests assemblies, plus `cat`/`stat`/
`grep` for reading. No corpus scan, no `cargo`, no `dotnet`, no source edit.
