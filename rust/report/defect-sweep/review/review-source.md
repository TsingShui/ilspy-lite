I've read all 15 pairs, the manifest, the milestone report/brief, and the source seams the report cites for attribution. No build, no execution, no CLI run — reads only. Here is the artifact.

---

# Independent static review: defect-sweep sample (15 pairs)

**Scope.** Read-only review of `rust/report/defect-sweep/review/{ours,ilspy}/<slug>.txt` against
`MANIFEST.md`, with the milestone's own claims in `rust/report/defect-sweep/REPORT.md` /
`BRIEF.md` and the cited source seams (`rust/src/lift.rs`, `rust/src/metadata.rs`,
`rust/acceptance/corpus/corpus.py`, the three `baseline.json` files) consulted for the
mechanism note only.

**Nothing was executed.** No build, no `cargo`, no `ilspy-lite` run, no ILSpy run, no test,
no script. Every statement below comes from reading the two recorded output texts and, where
noted, from reading source. No original IL was available for any row (not pre-dumped), so
IL-level facts that the two outputs cannot express are named explicitly rather than guessed.

## Row verdicts

| slug | bucket | verdict | decisive observation |
|---|---|---|---|
| box-operand-Core-060020D3 | box-operand | CRUDE-BUT-EQUIVALENT | Declared width/signedness kept: `long v2 = unchecked((long)(...))` (l.5) then `object v3 = ((object)((ulong)(v2)))` (l.6). Positive: one box, to `ulong` (the declared type), fresh and copied, single `this.state` read (l.4), and the signed detour is bit-identical for `ulong`. Rougher than ILSpy, which elides the box into the return's implicit boxing conversion (ilspy l.4). |
| box-operand-Core-060020E5 | box-operand | CRUDE-BUT-EQUIVALENT | Same shape at a different width: `int v2 = unchecked((int)(...))` (l.5), `object v3 = ((object)((uint)(v2)))` (l.6); state is `...RangeState\`1<uint32>` (l.3). Boxed value is `uint`, one instance, not a relabelled `int`. |
| box-operand-Core-06002109 | box-operand | CRUDE-BUT-EQUIVALENT | Third distinct width survives: `int v2 = unchecked((int)(...))` (l.5), `object v3 = ((object)((short)(v2)))` (l.6); state `...RangeState\`1<int16>` (l.3). Widening then re-narrowing is value-exact, box operand is `short`. That three widths (u64/u32/i16) stay distinct is the direct evidence the stack-category comparison did not flatten the declared type. |
| cctor-Core-060002C1 | cctor | CRUDE-BUT-EQUIVALENT | Presented as `static LayoutTag()` (l.1). 32 stores, constructor args 0..32 strictly ascending, one fresh `__cil_newobj_060002C2(n)` per field (ll.3-66), each immediately stored, no shared/hoisted instance, no duplicated argument; sequence equals ILSpy's `new LayoutTag(0..32)` list line for line. |
| cctor-Core-0600030F | cctor | CRUDE-BUT-EQUIVALENT | `v0 = __cil_newobj_0600031D()` (l.3) stored once to `Joint._unique_Unbreakable` (l.4) = ILSpy `new _Unbreakable()`. Owner nested correctly in the declared local type. |
| cctor-Core-06000373 | cctor | CRUDE-BUT-EQUIVALENT | Store first, in order: `...__il_field_040007EF = 0;` (l.3), then the IL's `ldsfld; pop` as an unread local `int v1 = ...` (l.4), vs ILSpy's `_ = $Quotations.init@;`. One store, nothing hoisted or duplicated; the dead read is the IL's own load, not an invented one. (Two IL facts are missing from the texts - see P2-4/P2-5.) |
| constructed-value-CSharp-0600002C | constructed-value | CRUDE-BUT-EQUIVALENT | No `new` in this body: struct-returning callvirt. Receiver `this.Next` read once (l.4), the single argument `scopeDesignator` passed once (l.5), result returned unmodified (l.6) = ILSpy `return Next.GetDeclaredLocalsForScope(scopeDesignator);`. No dropped argument, no duplicated receiver. |
| constructed-value-CSharp-0600002D | constructed-value | CRUDE-BUT-EQUIVALENT | Identical shape (l.4/l.5) for the `LocalFunctionSymbol` instantiation; argument count and order match ILSpy's `Next.GetDeclaredLocalFunctionsForScope(scopeDesignator)`. |
| constructed-value-CSharp-06000088 | constructed-value | CRUDE-BUT-EQUIVALENT | All eight `group.*` reads occur once, in IL argument order - TypeArgumentsOpt `v19` (l.58), Name `v21` (l.59), Methods `v23` (l.60), LookupSymbolOpt `v25` (l.61), LookupError `v27` (l.62), Flags `v29` (l.63), receiverOpt `v30 = l0` (l.64), ResultKind `v32` (l.65) - and are passed in that same order (l.66). The replace result `v16` (l.56) is the argument-7 receiver, not recomputed. `IsStatic && !IsExtensionMethod` is reproduced by the goto graph ll.24-55 (`v13 = (v11 == 0)` at l.40). Enum args cast to their exact types, `int` for Flags and byte width preserved for ResultKind: `unchecked((__cil_enum_0200002C)(unchecked((byte)(v32))))` (l.66). CFG-shaped, but no reorder, drop, merge or hoist. |
| enum-storage-CSharp-06004FD3 | enum-storage | CRUDE-BUT-EQUIVALENT | Only width present is `int`-backed `AssemblyHashAlgorithm` (l.4); the argument round trip `int v2 = unchecked((int)(algorithmId))` (l.6) -> `unchecked((System.Reflection.AssemblyHashAlgorithm)(v2))` (l.7) is exact for an int-backed enum (no signedness or truncation change, negatives included); receiver `this._underlyingModule` read once (l.5) = ILSpy `_underlyingModule.GetHash(algorithmId)`. Signature loses the explicit-interface form (P2-3). |
| enum-storage-Tests-06000077 | enum-storage | CRUDE-BUT-EQUIVALENT | Constant rendered as `unchecked((Mono.Cecil.ModuleKind)(0))` (l.4) vs ILSpy `ModuleKind.Dll`. The value is correct (`Dll == 0`) and the cast goes to the exact enum type, so nothing is truncated or relabelled; only the member name is missing. Argument order `(name, kind)` matches ILSpy. |
| enum-storage-Tests-060000B0 | enum-storage | CRUDE-BUT-EQUIVALENT | Same body as 06000077 (l.4 `(Mono.Cecil.ModuleKind)(0)`, ILSpy `ModuleKind.Dll`); a different source method with identical IL, and both outputs agree it is. |
| opaque-callvirt-Tests-0600018D | opaque-callvirt | CRUDE-BUT-EQUIVALENT | Statement sequence identical to ILSpy: literal -> `module.GetType` (l.5) -> `Assert.IsNotNull` (l.6) -> literal -> `Extensions.GetMethod(v2, v5)` (l.8) -> `Assert.IsNotNull` (l.9) -> literal -> `AssertCode(v8, v6)` (l.11). Receiver `module` is a parameter, dereferenced once, null check at the call point, nothing hoisted. Rougher only in the `__cil_ldstr_70000093("Foo")` wrapper (literal content is present) and the static-call form of an extension method. |
| opaque-callvirt-Tests-0600018E | opaque-callvirt | CRUDE-BUT-EQUIVALENT | Same as above with `"Library"`/`"GetHelloString"`; the IL-dump literal (l.10) matches ILSpy's string item for item (`\r\n\t.locals init (System.String V_0)` ... `IL_000a: ret`). |
| opaque-callvirt-Tests-0600018F | opaque-callvirt | CRUDE-BUT-EQUIVALENT | Same; the 30-item IL-dump literal (l.10) agrees with ILSpy character for character, including `IL_000b: switch (IL_0032, IL_0034, IL_0038, IL_0034)` and `IL_0038: ldc.i4.s 42`. No dropped/merged/reordered operation in the surrounding sequence. |

Totals: **15 rows = 0 EQUIVALENT, 15 CRUDE-BUT-EQUIVALENT, 0 SUSPECT, 0 INSUFFICIENT-EVIDENCE.**
No row is EQUIVALENT because no pair reproduces ILSpy's shape exactly: every ours file differs at
least in identifier form (`__name_<hex>`, `__cil_*` placeholders, `v1..vN` locals, `return;`
tails, no `using`/attribute rendering).

## SUSPECT

none

No row shows a semantic divergence from ILSpy's rendering: the three box rows keep three
distinct declared types with one fresh box each; the cctor rows keep store order (0..32) with
one instance per field; the constructed-value rows evaluate every argument once, in IL order,
and pass it in the same order; the enum rows keep the exact enum type and the exact constants;
the opaque-callvirt rows reproduce the statement sequence and all literal contents. Per the
task's rule, the rows ILSpy renders in full while ours gates them (`__cil_newobj_`,
`__cil_callvirt_`, `__cil_ldstr_`, `__cil_value_`) are next-slice evidence, not defects.

## Correct (what the sample positively establishes)

- `box` compares the normalized stack category while carrying the declared type: three
  different widths (`ulong`, `uint`, `short`) each box their own type, one instance each,
  copied - `box-operand-Core-0600{20D3,20E5,2109}.txt` ll.5-6. The named blocker
  "box operand type differs from its declared type" no longer fires on the scalar path
  (`src/lift.rs:1308-1311`), and no `int64`-at-`box int32` style mismatch appears in these rows.
- Enum identity survives the normalize-and-cast round trip: `enum-storage-CSharp-06004FD3.txt`
  ll.6-7, `constructed-value-CSharp-06000088.txt` l.66 (byte width preserved).
- Static initializers are presented as the type's static constructor (`static LayoutTag()`,
  `static Joint()`, `static FSharpVar()`) with ordered, non-duplicated, non-hoisted stores.
- Constructors/constructed values keep owner, argument count and order: `__cil_newobj_060002C2(n)`
  n = 0..32 (ll.3-66), `Update(group, v19, v21, v23, v25, v27, v29, v30, v32)` (l.66).
- Opaque receivers are evaluated once at the call point (all three `opaque-callvirt` rows).

## Coverage gaps in this sample (not defects)

- **No `unbox.any` anywhere in the 15 pairs.** The brief's item 3 - "unbox pushes the normalized
  integer category and renders the conversion to the storage type explicitly", including the
  `sbyte`-storage fixture - is not exercised by any row. The enum rows exercise enum types in
  *argument* position only (`enum-storage-*`), plus one byte-backed enum argument
  (`constructed-value-CSharp-06000088.txt` ll.6, 66), so the sbyte/short/ushort/long unbox
  widths remain unreviewed here.
- **Box widths exercised: `ulong`, `uint`, `int16` only.** The brief's category-normalization
  cases (`bool`/`char`/`i1`/`u1`/`u2`) do not appear, so "signedness preserved" is evidenced by
  `uint`/`ulong` vs `int16` only.
- **No `new T(args)` in the constructed-value rows.** They are struct-returning `callvirt` bodies;
  constructor rendering appears only as an opaque `__cil_newobj_` token in the three cctor rows.
  The "right owner / argument order" criterion is therefore judged from the token's argument
  count and literal order, not from a resolved `new T(...)`.

## Findings (report-only; none blocks merge of the row set)

**P1 - the milestone's residual per-bucket attribution mixes a reference-selection change with
the code change.** Location: `rust/report/defect-sweep/REPORT.md`, section "Attribution
(measured, not assumed)", table of the remaining +17,677. Evidence: the "before" side
(`rust/report/boxing/corpus-scan-enum/baseline.json`) and the "after" side
(`rust/report/defect-sweep/corpus-scan/baseline.json`) passed **different reference sets** to the
CLI for the same images. Id 0 FSharp.Core, `image_sha256 130411728df6...` in both, same
inventory mtime: before `"declared": 4, "selected": 0, "ambiguous": 4, "resolution": null`; after
`"ambiguity": "tie-break", "declared": 4, "selected": 4, "tie_broken": 4, "resolution":
{"resolved": 4}`. Id 12 Mono.Cecil.Tests, `image_sha256 f4778238e595...` in both: before
`"declared": 6, "selected": 2, "ambiguous": 4`; after `"passed": 6`. The harness now defaults to
tie-break (`rust/acceptance/corpus/corpus.py:689`; the older records carry no `ambiguity` field and
selected nothing for ambiguous identities), and `REPORT.md`'s Repeat command passes only
`--references declared`. The gates that the five inflated buckets are attributed to are exactly
the ones that depend on resolvable definitions/references - `src/metadata.rs:2173` ("enum storage
requires an exact local definition"), `:2645` ("constructed value type needs a local definition"),
`src/lift.rs:998` ("callvirt on an opaque external receiver needs dependency facts") - and none of
them is touched by this diff. So the 7,547 / 7,027 / 1,325 (and possibly the 390) rows the report
itself calls "larger than the code change that plausibly causes them" cannot be credited to the
three fixes from this comparison. The sealed number is *not* affected: the A/B used
`rust/report/defect-sweep/scan-no-sealed/baseline.json`, which carries the same tie-break
selection (`"passed": 6` for id 12), so the +18,288 is a clean like-for-like delta. Smallest fix:
re-run the pre-milestone tree with the current harness flags (or re-run both sides with
`--reference-ambiguity skip`) and recompute the per-reason delta; alternatively record the
reference-mode difference next to the table and stop attributing those buckets to the diff. This
does not affect any row verdict above - the sample shows the outputs are semantically faithful,
not why they unblocked.

**P2-1 - box rows route the value through a signed local and wrap only half of the conversions.**
`ours/box-operand-Core-060020D3.txt` ll.5-6 and `ours/box-operand-Core-060020E5.txt` ll.5-6: the
widening to `long`/`int` is `unchecked(...)`, while the back-conversion used as the box operand
(`(ulong)v2`, `(uint)v2`) is emitted bare (`src/lift.rs:1311` formats the cast with no
`unchecked`). Bit-exact in default C# (default context is unchecked), so the rows are correct as
recorded, but under a `checked` compilation the `negative long/int -> ulong/uint` conversion would
throw where the IL cannot - i.e. the emitted text is only faithful in one compilation setting and
the file gives no hint of that. Smallest fix: box the call result directly
(`object v3 = (object)call(...)`) so no signed detour exists, or wrap the back-cast in
`unchecked(...)` for symmetry. The `int16` row needs no change (`(short)` of an in-range value
cannot throw).

**P2-2 - enum constants are never rendered by member name.** `ours/enum-storage-Tests-06000077.txt:4`
and `ours/enum-storage-Tests-060000B0.txt:4` emit `unchecked((Mono.Cecil.ModuleKind)(0))` where
ILSpy prints `ModuleKind.Dll`. The value is right; the shape makes every enum constant read as a
cast of a literal, which is the readable-difference the sample review is meant to surface. Smallest
fix: when a constant matches a field of the local enum, print the field name.

**P2-3 - explicit interface implementations lose their interface designation.**
`ours/enum-storage-CSharp-06004FD3.txt:1` renders `private virtual ... __name_4D6963726F736F...`
where ILSpy renders `ImmutableArray<byte> IFileReference.GetHashValue(...)`: the interface member
fact exists only as hex inside the name. Same class of loss for closure methods
(`ours/opaque-callvirt-Tests-0600018D.txt:1-2`, `PrintStringEmptyLambda2`) - mitigated there by the
`// Metadata name:` line. Body semantics are unaffected in both.

**P2-4 - `FSharpVar`'s initializer store crossing types cannot be checked against the new
readonly rule.** `ours/cctor-Core-06000373.txt:3` stores into
`<StartupCode$FSharp-Core>.$Quotations.__il_field_040007EF` from `FSharpVar`'s static constructor -
a field of a different type, as ILSpy also renders (`_ = $Quotations.init@;` after the store). The
milestone's rule is "`static readonly` writable only in the declaring type's own initializer"; from
the two outputs alone it is not knowable whether `040007EF` is `static readonly`, so this row can
be neither confirmed as a correct admission nor flagged as a rule hole. Missing IL fact: the
`FieldAttributes` of token `0x040007EF`. Worth one contract check rather than a guess.

**P2-5 - `cctor-Core-06000373.txt:4` materializes the IL's `ldsfld; pop` as a typed unread local**
(`int v1 = <field>;`) where ILSpy writes `_ = $Quotations.init@;`. Harmless (an unread local, no
side effect, no duplicated store), but a reader cannot verify that `int` is the field's static
type. Missing IL fact: the field signature of token `0x040007EF`. Cosmetic; no action required for
this slice.

## Merge verdict

**OK with notes.** No row is SUSPECT; every sampled output is semantically faithful to ILSpy's
rendering (CRUDE-BUT-EQUIVALENT, 15/15). The row set does not, however, exercise the enum
`unbox.any`/storage-width fix or the `bool`/`char`/small-integer box categories, and the residual
+17,677 per-bucket attribution in `REPORT.md` is confounded by a changed reference-selection policy
(P1) - the mechanism the report says still needs documenting. Suggested next slice: one pair per
`sbyte`/`byte`/`short`/`ushort`/`long` enum unbox, one `bool` and one `char` box, one resolved
`new T(args)` pair, and a like-for-like pre-milestone scan with the current harness flags.