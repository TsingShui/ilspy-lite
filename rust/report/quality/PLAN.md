# Readability work plan

Five class-level slices were compared against ILSpy 11.2 (`report/quality/BRIEF.md`,
`harvest.py`). Each slice paired 3-5 classes, all of them already decompiled: the
totals below are quality gaps, not admission gaps (fallback bodies are listed
separately in each `FINDINGS.md`).

| Slice | Classes | Our lines | ILSpy lines | Ratio |
|---|---|---:|---:|---:|
| openra-game | Map, World, Exts, Actor, Game | 20,380 | 4,260 | 4.78x |
| openra-common | Aircraft, Mobile, ProductionQueue, LobbyCommands, ColorGlobal | - | - | 4.62x |
| decompiler-lib | CSharpOutputVisitor, DepthFirstAstVisitor, ILInstruction, CSharpFormattingOptions | 13,682 | 4,788 | 2.86x |
| newtonsoft-tests | JsonSerializerTest, JObjectTests, JPathParseTests, ExceptionHandlingTests | 26,356 | 8,250 | 3.19x |
| system-xml | XmlReader, DtdParser, XmlSqlBinaryReader | 52,663 | 11,462 | 4.60x |

The line ratio understates the gap where our braces are compact and overstates it
where async state machines dominate (`system-xml`: async is 44% of our output and
2.55x remains after excluding it).

## What actually differs

Ranked by cross-slice evidence. Counts are per slice and come from the findings.

1. **Member names at call sites are replaced by tokens.** `__cil_direct_TOKEN`,
   `__cil_callvirt_TOKEN`, `__cil_newobj_TOKEN`, `__cil_ldftn_`, `__cil_ldsfld_`,
   `__cil_ldstr_`, `__cil_memberref_call_`, `__cil_methodspec_call_`: 952
   (openra-common), 1,390 (openra-game), 2,978 (newtonsoft), 3,007 (system-xml),
   3,193 (decompiler-lib). This is the dominant naming defect and it *cascades*:
   an unnamed call cannot nest into its consumer, so it also creates the
   `vN` temporaries (8,203 in newtonsoft alone). The member name and signature are
   already resolved; only the printer substitutes the token.
2. **No `using` directives and fully-qualified names everywhere.** 2,141 / 4,045 /
   3,970 / 6,618 fully-qualified tokens plus 1,026-2,011 `global::`, and opaque
   aliases `__cil_type_`, `__cil_ref_<sha>`, `__cil_value_<sha>` (403 in
   openra-game, 6,321 in system-xml). Every alias already carries a decodable
   metadata spelling in its own comment.
3. **IR evidence printed inside the readable view.** `// structural signature`,
   `// generic body context`, `// enum storage`, `// IL_xxxx`, `.code-bytes`,
   `// Dependency unresolved`: 7,046 lines / 26.7% of newtonsoft output, 16.5% of
   system-xml bytes with only 177 distinct facts and 85-93% verbatim duplicates.
4. **Lowered control flow.** No `while`/`for`/`foreach`/`switch` at all; 94.8% of
   `if`s in system-xml have `goto` in both arms, 994 `goto`/952 labels in
   decompiler-lib. Zero-risk subset: fall-through `else { goto L; }` pairs
   (~1,276 in openra-common), unreferenced `IL_xxxx: ;` labels (870+), trailing
   `return;`, dead `= default` declarations.
5. **Accessors and auto-properties.** 122 accessors (openra-game), 498
   (decompiler-lib) printed as `get_X`/`set_X` methods plus a `// property ...`
   trailer; auto-property backing fields printed as `__il_field_<token>`.
6. **Enums and constants.** Enum use sites print as `(E)(6)` with `case 6:`
   labels (974 + 857 in system-xml); char literals are lost (273); `unchecked`
   wraps same-width arithmetic (4,949); `op_Equality` instead of `==`;
   `newobj` + `throw` instead of `throw new`.
7. **Boolean and arithmetic lowering.** `if (c) return true; return false;`,
   `x = x + 1`, `__cil_bool_load/store` wrappers (818-1,336), masked-compare
   casts, `is T x` patterns never recovered.
8. **Field initializers.** A `readonly string Label = "label"` is printed as an
   empty declaration; the store lives in the constructor.
9. **Compiler-generated machinery printed literally.** `<>c` becomes
   `__name_3C3E63`, 208 lambdas in newtonsoft are emitted as lowered bodies,
   `<>c__DisplayClass` allocations cause 74 fallbacks there.

## Phased order

Phase 1 is printer-only: no new analysis, no semantic risk, and each item is
independently revertible.

1. **Member names at call sites.** Print the resolved name and signature; keep the
   exact token as a trailing comment when the call has a semantic caveat
   (`call` on a virtual method, interface target, no null check) and move the rest
   to `--details`. Update the tests that pin `__cil_*_TOKEN` targets to assert the
   name plus the retained token evidence.
2. **Short type names and `using` directives.** Gate on shadowing instead of
   deleting `global::` blindly; resolve `__cil_type_/__cil_ref_/__cil_value_`
   aliases through the identity already recorded.
3. **Evidence comments to `--details`**, deduplicated per type in the evidence
   view.
4. **Delete implied text**: fall-through `else { goto L; }`, unreferenced labels,
   `IL_xxxx: ;`, trailing `return;`, dead `= default`.
5. **Properties from accessor pairs**, with backing-field names; no member is
   merged, only reshaped.
6. **Enum spellings**: `enum` declarations, named use sites, named `case` labels.
7. **Expression compression**: increments/compound assignment, boolean folding,
   same-width `unchecked` elision, `op_Equality` -> `==`, `throw new`,
   character literals, `\r\n` short escapes.

Phase 2 needs proofs and gets its own tests per item: locals declared at first use
and single-use temporaries folded (depends on item 1), field initializers inlined
only when the constructor store is provably the only one, `is T x`, `??`, `?.`,
`using`/`lock`, `foreach`, `switch` recovery, and structured `goto` removal.

Phase 3 is deliberately out of scope for now: async/iterator/lambda/closure
reconstruction. Our lowered form is faithful; ILSpy's source form needs a
transform pipeline, and the brief marks these low priority.

## Guardrails

- Every item keeps the exact identity available in `--details`; nothing is dropped
  from the evidence channel. No operation is removed, reordered or hidden.
- Compression must preserve evaluation order, exception behavior, copies and
  aliases; `x++` only for a single-use increment, boolean folding only for a
  literal `true`/`false` return pair.
- Each item ships with its own red/green Rust test and, where behavior could
  change, the existing owned behaviour drivers stay green. Readable output is
  regression-tested by re-harvesting a slice and comparing before/after counts.
- The corpus admission baseline (42,044 fallback and the 89/319 per-sample gate)
  is unrelated to this work and must not move because of a printer change; a
  status or per-sample change is a defect here. A change that admits a body
  belongs to the admission track, not this plan.

## Review and validation

- Review is main-session self-review per `acceptance/corpus/REVIEW_PROTOCOL.md`;
  no subagents are launched. The report separates self-review from owned-fixture
  behavior validation.
- Tests follow the current AGENTS: test changed behavior, keep validation
  proportional, and use red/green where behavior changes. Re-harvest the five
  slices before/after each Phase 1 item and at each Phase 2 proof gate.
- The per-sample gate (`100 * unrecovered <= bodies` for every body image) is the
  active acceptance; the aggregate 1% is diagnostic.

## Correction to an earlier claim

The first inspection of `OpenRA.Server` suggested the string printer escapes every
character as `\uXXXX`. That was wrong: `ldstr` literals were already correct. The
defect only affected **metadata string constants** (`const string` fields), which
`readable.rs` rendered as raw UTF-16 escapes. Fixed in this phase; `ldstr` and
control-character escaping are unchanged and covered by `tests/strings.rs` and the
new `tests/class_method_impls.rs` case.
