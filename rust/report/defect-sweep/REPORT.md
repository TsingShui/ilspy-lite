# Defect sweep: sealed owners, scalar box category, enum unbox, static constructors

## What changed

| change | file | previous row |
|---|---|---|
| allocation owner accepts sealed classes (reject only `interface\|\|abstract`) | `src/metadata.rs` | `allocation owner must be a concrete class` |
| `box`/`unbox.any` compare the normalized stack category and cast to the declared/storage type | `src/lift.rs` | `box operand type differs from its declared type`, `incompatible value/receiver types` |
| `.cctor` lowered as an ordinary static void body; `static readonly` writable only in the declaring type's own initializer | `src/lift.rs` | `empty/static-initializer/runtime-special method needs IL fallback` |

Contract tables were re-baselined: the eight `Unbox*` enum rows promoted, one stale
`UnsupportedNew` row removed, `UnsupportedStruct`/`UnsupportedConstruct` re-reasoned to
`value-type receiver needs IL fallback`. No assertion was loosened.

## Measurement

- `cargo nextest run`: **362 passed**.
- `cargo clippy --locked --all-targets -- -D warnings`: 0 errors; `cargo fmt`: clean.
- Full scan: `report/defect-sweep/corpus-scan/baseline.json`
  **333,097 Decompiled / 437,651 Partial** of 770,748 bodies (56.78263%), 319 body images,
  **0 incomplete**, 33 images within 1%.
- Versus `report/boxing/corpus-scan-enum/baseline.json` (297,132): **+35,965**, 204 images
  gained, **0 methods lost**.

## Attribution (measured, not assumed)

Two scans isolate the sealed change: with the mask reverted the same tree scores 314,809
(`report/defect-sweep/scan-no-sealed/`), so the sealed mask alone is **+18,288** and every
one of those methods previously carried `allocation owner must be a concrete class`.

The remaining **+17,677** by previous first blocker:

| gained | previous reason |
|---:|---|
| 7,547 | `enum storage requires an exact local definition` |
| 7,027 | `constructed value type needs a local definition` |
| 1,386 | `.cctor` blanket |
| 1,325 | `callvirt on an opaque external receiver needs dependency facts` |
| 390 | `box operand type differs from its declared type` |
| 2 | `box/unbox needs an admitted core scalar or local enum` |

The last five buckets are larger than the code change that plausibly causes them, so the
mechanism is not yet isolated: removing the `.cctor` blanket lets a static initializer
proceed to the next blocker, but a body whose *first* blocker was enum storage should not
become decompilable from that alone. Sample outputs (e.g. `Mono.Cecil.Tests`
`ImportCecilTests::CreateModule`) decompile to the expected shape, but the slice counts as
reviewed only after the independent sample review confirms the semantics and the mechanism
is documented.

## Behaviour gates

| driver | result |
|---|---|
| `enum-roundtrip.py` | 3,120 comparisons (was 3,032); mutants helper/signed/unsigned/state/unboxtarget rejected, release and debug |
| `generic-declaring-roundtrip.py` | 18 comparisons, 11 methods; wrong-field rejected |
| `virtual-call-roundtrip.py` | 47 comparisons (was 44), 9 methods; direct-dispatch rejected |
| `new-object-roundtrip.py` | 29 comparisons, 9 methods; cached-identity and dropped-store mutants rejected |
| `struct-receiver-roundtrip.py`, `object-roundtrip.py` | green |

Driver fixes made while promoting rows: the enum driver now expects all eight storage
widths; the virtual-call harness derives candidates from the declared parameter types
instead of passing `Shape` to `int` parameters (that comparison only contrasted delegate
unboxing with reflection binding), and it inlines `__cil_newobj_` as `new Type(args)`.

## Repeat

```
cargo build --release
python3 acceptance/corpus/corpus.py baseline \
  --inventory report/core-type-roots/scope-policy/implementation-inventory.json \
  --cli target/release/ilspy-lite --output report/defect-sweep/corpus-scan \
  --references declared
python3 report/compare-recovery-scans.py report/boxing/corpus-scan-enum/baseline.json \
  report/defect-sweep/corpus-scan/baseline.json report/defect-sweep/delta.json
python3 report/per-assembly-gate.py report/defect-sweep/corpus-scan/baseline.json
```
