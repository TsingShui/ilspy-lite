# Full-corpus quality review brief

You are reviewing decompiler **quality**, not admission. Every class you receive
already has a body for its methods in both outputs; `// IL fallback:` blocks are
the only exception and must be reported separately, never counted as quality.

You get one JSON file per class:

```json
{"image": "...", "image_id": 1234, "type": "Ns.Type", "bodies": 120,
 "ours_status": 2, "ilspy_status": 0,
 "ours_truncated": false, "ilspy_truncated": false,
 "ours": "<ilspy-lite readable output>", "ilspy": "<ILSpy 11.2 output>"}
```

`ours_status` is the decompiler exit code (2 means at least one method fell back
to IL). `*_truncated` means the file was cut at a byte budget: judge only what you
see, and say so if a conclusion depends on the cut.

## Task

Read `ours` and `ilspy` for the same class and record the *differences that hurt
readability*. For each difference give the exact side-by-side text (one or two
lines each), the member it comes from, why it is worse, and what a fix would
involve. Do not report things where our output is merely different but equally
readable.

Classify every observation into exactly one category:

- `naming` - synthetic locals (`v0`, `v1`), `__cil_*` helpers, `__name_<hex>`,
  `__il_field_<token>`, `__cil_type_`/`__cil_ref_`/`__cil_value_` aliases, lost
  member names at call sites, `@`-escaped identifiers, parameters/locals whose
  real name is recoverable.
- `compression` - the IL is kept one instruction per statement: assign-then-return,
  `x = x + 1`, `if (c) return true; return false;`, redundant temporaries,
  `unchecked(...)` noise, `(E)(6)` constants, missing `++`/`+=`/`??`/`?.`/`is T x`.
- `spelling` - fully qualified names where ILSpy uses short names, `global::`,
  `System.Int32` vs `int`, array/pointer/byref spelling, literal form, char
  literals, `\uXXXX` where a short escape or the character itself would do.
- `structure` - `get_`/`set_` methods instead of properties, `__il_field_` backing
  fields, field initializers left in the constructor, declaration order, block
  nesting, `else` after `return`, members printed in the wrong order, enum
  declarations printed as constants.
- `control_flow` - `goto`/label dispatch where ILSpy has `while`/`for`/`foreach`/
  `switch`, unreferenced `IL_xxxx: ;` labels, fall-through `else { goto L; }`.
- `comments` - IR evidence printed inside the readable view
  (`// structural signature`, `// IL_`, `.code-bytes`, `// enum storage`,
  `// property`, `// Dependency unresolved`).
- `other` - anything real that does not fit above (say what).

## Rules

- Be specific: quote, count within the class, and name the pattern. "Too verbose"
  without a quote is useless.
- Say whether the fix is **printer-only** (identity already available), needs a
  **small proof** (e.g. single-use temporary before folding), or needs **new
  analysis** (e.g. `foreach` requires proving `GetEnumerator`/`MoveNext`/
  `Dispose` shape). Never propose anything that drops an operation, reorders side
  effects, weakens an exception, or hides a method.
- If ILSpy reconstructs a compiler feature we deliberately keep lowered (async,
  iterators, closures, lambdas, `using`), mark it `deliberate` with one line of
  reasoning and move on.
- Only judge what is in the file. Do not run either decompiler, do not open other
  files, do not execute anything.

## Output

Write exactly one file: `<bundle>/<your-slice>/FINDINGS.json`

```json
{"slice": "...", "classes_reviewed": 12, "partial_classes": 1,
 "categories": [{"name": "naming", "occurrences": 340,
   "examples": [{"image_id": 1234, "type": "Ns.Type", "member": "M",
                 "ours": "...", "ilspy": "...", "why": "...",
                 "fix": "printer-only"}]}],
 "notes": "..."}
```

Also write `<bundle>/<your-slice>/SUMMARY.md`: at most 40 lines, the ranked
patterns that recur across your classes, each with the number of classes it
appears in, one representative quote, and the fix class (printer-only / small
proof / new analysis).
