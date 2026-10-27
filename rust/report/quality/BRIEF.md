# Class-level quality comparison brief

Purpose: find the *quality* gaps between ilspy-lite's readable output and ILSpy's
output for the same classes, so the next phase can improve naming, semantic
compression and general readability. This is not a recovery/admission audit:
both files already contain a body for every method (Partial bodies are visible as
`// IL fallback:` blocks and must be reported separately, not counted as quality).

## Inputs (already harvested, do not regenerate)

`rust/report/quality/<slice>/index.json` holds, per type:

- `ours.path` - `ilspy-lite decompile <image> --type <type>` output
- `ilspy.path` - `ilspycmd -t <type> <image>` output
- `kind`, `type`, and crude metrics (`lines`, `locals`, `helpers`,
  `stray_labels`, `il_comments`, `unchecked`, `defaults`, `bool_returns`,
  `self_increment`, `assign_then_return`, `casts`, `empty_statements`,
  `this_prefix`) plus `ratio` (our lines / ILSpy lines).

Slices: `openra-game`, `openra-common`, `decompiler-lib`, `newtonsoft-tests`,
`system-xml`. Images were read only; no input code was executed.

## What to look for

Group every observation into one of these categories. Each observation needs a
concrete side-by-side quote (our line(s) and ILSpy's line(s)) and the type and
method it comes from.

1. **Identifier naming** - synthetic names where ILSpy has a real one or a
   readable synthetic one: `v0/v1` locals, `__cil_*` helper names, structural
   signature aliases, `arg0`, `this` spelling, generated type/member names,
   parameters whose PDB/ILSpy name is available, local reuse across scopes.
2. **Semantic compression** - IL kept one-to-one with the stack instead of the
   source shape: assign-then-return, `x = x + 1`, `if (c) return true; return
   false;`, redundant temporaries, `while (true)` instead of `for`/`while (c)`,
   `unchecked(...)` noise, `default(T)` where `null`/`0` is exact, empty
   statements / stray `IL_0000: ;` labels, duplicated conditions, missing `??`,
   `?.`, `++`, compound assignment, `for`/`foreach`, `switch`, string
   concatenation, casts that exist only because of our type spelling.
3. **Type and member spelling** - fully qualified names where ILSpy uses the short
   name plus `using`, `global::` noise, array/pointer/byref spelling, generic
   argument spelling, `System.Int32` vs `int`, escaped literals (note: our string
   printer currently emits `\uXXXX` for every character - quantify how often it
   appears), numeric literal form, `@`-escaped identifiers.
4. **Structure and layout** - block nesting, `else` after `return`, declaration
   placement (all locals at the top vs first use), field/property/constructor
   ordering, partial/`record`/`struct` shape, attribute placement, `readonly`,
   modifiers, explicit interface implementations, `unsafe` blocks.
5. **Comments and diagnostics** - `// structural signature ...`, `// Dependency
   unresolved: ...`, `// generic body context ...`, `// IL_...` lines inside a
   body, fallback banners. Say whether each is (a) necessary evidence that should
   stay but move to `--details`, or (b) noise that hides the code.
6. **Missing source-level constructs** - `using`/`lock`/`foreach`/`fixed`/
   `checked`/`nameof`/`?.`/`??`/initializers/deconstruction/pattern matching,
   where ILSpy reconstructs them and we emit the lowered form but semantics are
   preserved. These are the expensive ones: say what the IL shape looks like and
   how much of the class it affects.

## Rules

- Be concrete and quantitative: count occurrences per category per slice, and give
  at least three real examples per category (fewer only if the category has fewer
  instances).
- Separate "semantics preserved, presentation only" from "presentation change
  needs new analysis" (e.g. `foreach` requires proving `GetEnumerator`/
  `MoveNext`/`Dispose` shape). Never propose anything that drops an operation,
  reorders side effects, weakens an exception, or hides a method.
- Do not claim a defect where the two outputs differ only because ILSpy
  reconstructs a compiler feature we deliberately keep lowered (async, iterators,
  closures, `using`). Mark those as "deliberate, low priority" and say why.
- Report Partial/fallback blocks separately with counts; they are admission gaps,
  not quality gaps.
- Do not edit any file outside your own output directory. Do not run the
  decompilers over other assemblies. Do not execute any input code.

## Output

Write exactly two files in your slice directory:

- `FINDINGS.md`: the analysis, ordered by impact, with a summary table of
  category counts and 8-15 concrete examples quoted side by side.
- `findings.json`: machine readable list
  `{slice, totals, categories:[{name, occurrences, examples:[{type, method,
  ours, ilspy, why, fix, risk}]}], partial_blocks, notes}`.

End `FINDINGS.md` with a ranked shortlist of the 5 changes with the best
readability gain per unit of semantic risk.
