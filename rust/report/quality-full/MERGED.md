# Full-corpus quality review: merged results

Input: 315 of the 319 body-bearing images (the other 4 have no decompiled body),
4 largest classes each, **1,095 classes**, paired outputs from ilspy-lite and
ILSpy 11.2. 24 review slices (DeepSeek-V4-Pro via traex), one per ~46 classes.
Reviews cover 1,093 of the 1,095 classes; slices 06 and 16 each reviewed one class
fewer than their manifest (45/46, 44/45) without saying which, so their per-class
counts are lower bounds. No input code was executed.

Everything below is **quality**, not admission: `// IL fallback:` bodies are
excluded from these counts and are reported separately by the corpus work.

## Category totals (as reported by the slices)

| Category | Occurrences | Slices reporting |
|---|---:|---:|
| naming | 154,166 | 24 |
| control_flow | 35,823 | 24 |
| spelling | 32,160 | 24 |
| compression | 25,957 | 24 |
| comments | 14,544 | 24 |
| structure | 14,974 | 24 |
| other | 1,182 | 20 |

Occurrence counts are per-slice measurements with different rules, so the totals
are indicative, not comparable across categories. The **pattern** evidence is
comparable: 995 labelled examples, of which 632 are printer-only, 174 need a small
proof, 180 need new analysis, 7 are deliberate, 2 unlabelled.

## Patterns ranked by cross-slice repetition

Each row is how many of the 995 examples showed that exact shape.

| Pattern | Examples | What it is |
|---|---:|---|
| `goto`/label dispatch | 127 | every branch and loop is `if (c) { goto IL_X; } else { goto IL_Y; }` plus `IL_X: ;` |
| `get_`/`set_` accessor | 84 | properties printed as method pairs plus a `// property` trailer |
| `__cil_type_<token>` alias | 69 | type references printed as opaque token aliases |
| `global::` qualifier | 68 | fully-qualified names with `global::` instead of `using` |
| `unchecked(...)` wrapper | 64 | same-width arithmetic wrapped in `unchecked` |
| `__il_field_<token>` | 58 | auto-property backing fields printed as raw fields |
| `__cil_ref_<sha>` alias | 56 | structural reference aliases |
| `__cil_value_<sha>` alias | 54 | structural value aliases (mostly in evidence comments) |
| `__cil_newobj_` call | 40 | constructor call sites lose the type name |
| `__cil_ldstr_` literal | 34 | string literals printed as helper calls |
| `__cil_callvirt_` call | 19 | call sites lose the member name |
| `__cil_direct_` call | 16 | call sites lose the member name |

## What the slices agree on

1. **Names, not analysis, are the dominant defect.** Member names, type names and
   literals are already resolved internally; the printer substitutes tokens
   (`__cil_*`, `__name_<hex>`, `__il_field_*`) or synthetic locals. This is
   printer-only in every slice that examined it.
2. **The `vN` flood is downstream of (1).** An unnamed call cannot be inlined into
   its consumer, so each one forces a temporary. Fixing call-site names removes
   most of the temporaries without touching local naming.
3. **`goto` dispatch is universal** (127 examples, present in all 24 slices), but
   it splits: `if`/`else`, early return, `&&`/`||` and `?:` need only a small
   proof; loops, `switch` and `foreach` need new analysis.
4. **Accessors and backing fields** are a single structural fix: pair `get_`/`set_`
   into a property and name the backing field.
5. **Evidence comments** are a printer move, but only after (1) and (2), because
   today they are the only thing carrying the identity that the code loses.

## Recommended order

1. Call-site member names + literals (printer-only, removes the `vN` cascade).
2. Short type names and `using`, resolving `__cil_type_`/`__cil_ref_`/`__cil_value_`
   aliases through the identity already recorded (printer-only).
3. Evidence comments to `--details` (printer-only, must follow 1-2).
4. Accessor pairs to properties, backing-field names (small proof).
5. Delete implied text: fall-through `else { goto }`, unreferenced labels,
   `IL_xxxx: ;`, trailing `return;`, dead `= default` (printer-only).
6. Same-width `unchecked` elision, `x = x + 1`, boolean folding, enum spellings,
   char literals (printer-only, per-item tests).
7. Structured `if`/`else`/early-return from `goto` (small proof).
8. Loops, `switch`, `foreach`, `??`, `?.`, `is T x` (new analysis, one item at a
   time with its own fixtures).

## Caveats

- Two slices under-reported class coverage by one class each; their per-class
  counts are lower bounds, so no slice-level ranking is presented.
- Several slices noted truncated bundles (`ours_truncated`) and ILSpy crashes
  (`ilspy_status=65`) on a few classes; those examples compare against partial
  output only.
- Occurrence totals mix counting rules; use the pattern table, not the category
  totals, to prioritise.
