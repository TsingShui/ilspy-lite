# Brief: static constructors (highest priority)

Owner: worker. This replaces the earlier queue position: do it immediately after the
external-field slice, before external constructed values.

## Why this is first

`empty/static-initializer/runtime-special method needs IL fallback` is a blanket refusal
of `.cctor` bodies: **13,335 bodies in 236 of 319 images**, about 2.1% of every large
image (CodeAnalysis.CSharp 2.10%, VisualBasic 2.20%, OpenRA.Mods.Common 426/18,407 =
2.31%). The acceptance gate is per assembly at 1%, so no image can pass while a 2.1%
blanket row exists. Evidence: `report/fallback-corpus/PER-ASSEMBLY.md`.

## Scope

1. Remove the blanket `.cctor` refusal and lower a `.cctor` body like any other void
   body, including static field stores, static method calls, delegate caching
   (`ldftn` + `newobj`), nested type statics and generic-type statics.
2. Emit the body as the type's static constructor; do not rename it to a normal method
   and do not merge it into another member.
3. Anything genuinely unsupported inside a `.cctor` must fail with **its own** reason
   (e.g. external field facts, external allocation, generic delegate construction), not
   with the blanket row. The blanket row may survive only for the cases it actually
   describes: runtime-implemented and empty-declaration methods with no body.

## Semantics that must survive

- Exactly one execution at type initialization; no duplication into instance paths.
- Static field stores keep their IL order and are never eliminated as dead stores.
- An exception inside the static constructor propagates as the platform does
  (`TypeInitializationException` wrapping), not swallowed.
- No inference of `beforefieldinit`; if the flag matters for observable ordering, say so
  in the evidence comment instead of changing the emitted member.
- Delegate caching must keep the same instance across calls (one cached field), not
  allocate per call.
- No status changes, no hidden methods, no proxies to original IL, no magic counts.

## Prerequisites in the same dispatch

Fix `report/boxing/DEFECT-enum-unbox-cast.md`: enum `unbox.any` must render an explicit
conversion to the storage type (`((int)((EnumCases.S32)(v)))`, storage's own name), and
the roundtrip driver must feed a non-null boxed value so the recovered body actually
runs.

## Deliverables

1. `rust/tests/dotnet/StaticConstructorCases.cs` + build script: simple field init,
   delegate cache, struct static, generic type static, two-phase init across types,
   throwing static constructor, and a negative (a `.cctor` containing an operation that
   must still be refused, with its own reason).
2. `rust/tests/static_constructors.rs` frozen table.
3. `rust/tests/static-constructor-roundtrip.py` with a mutation control that drops a
   store or duplicates the body; run release and debug.
4. `rust/acceptance/corpus/STATIC_CONSTRUCTORS.md` + README index entry.
5. Report: files, red-before-green, test counts, driver comparisons and mutation
   verdicts, and the per-reason breakdown of what still fails inside `.cctor` bodies.

## Discipline

`CARGO_TARGET_DIR=rust/target-worker`; focused nextest, then full nextest, clippy
`-D warnings`, fmt; no corpus scan; no commits; only `rust/src`, `rust/tests`,
`rust/acceptance/corpus` and the report file.
