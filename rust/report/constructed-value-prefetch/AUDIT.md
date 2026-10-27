# Constructed value generic prefetch: measured

The prefetch group landed in `30df71596` but its corpus effect was never
measured. This is the first complete fixed-scope scan on the current HEAD.

## Command

```sh
export DOTNET_ROOT="$HOME/.dotnet10"
cd rust
cargo build --release --locked
python3 acceptance/corpus/corpus.py baseline \
  --inventory report/fallback-corpus/implementation-scope/implementation-inventory.json \
  --cli target/release/ilspy-lite \
  --output report/current-head/implementation-scan \
  --references ilspy --workers 4 --scan-threads 2 --timeout 300
```

The admission scan uses the **614-image implementation inventory**. The full
`inventory-final` is only for regenerating that scope with
`implementation_scope.py`. `--references` defaults to `none`; a none-mode scan
resolves no external facts and looks like a ~210k regression. The committed
anchor and every admission scan use `--references ilspy` on the implementation
inventory. Compare only scans whose `references.mode` matches.

## Result

| quantity | anchor | current |
|---|---:|---:|
| Decompiled | 728,704 | **730,866** |
| Partial | 42,044 | **39,882** |
| fallback | 5.454961% | **5.174454%** |
| body images <=1% | 89/319 | **92/319** |
| incomplete | 0 | 0 |

Delta against the committed anchor: **+2,162 Decompiled across 66 images, 0
lost**. Every gained method previously carried
`external constructed value generic method needs dependency facts` (2,162 rows).

The three images that crossed the per-sample gate are `bug-185` (0/8),
`System.Threading.Tasks.Extensions` net45 (0/39) and the net472 copy (0/104).

## Per-sample gate

The active acceptance is `100 * unrecovered <= bodies` for every body image. The
sum of `floor(bodies/100)` over the 319 images is **7,585**, so 32,297 more
bodies are needed for 319/319. The aggregate 1% diagnostic allows 7,707 and
needs 32,175 more; it is not the acceptance.

## Evidence

- `batch-evidence.zip`: the complete baseline and per-image results.
  `rust/report/residual-audit/extract-anchor.py --zip <this file> --output <dir>`
  restores local `results_path` values.
- `delta.json`: the per-image gained/lost comparison against the anchor.
- Focused tests and the owned behavior gate for `30df71596` are recorded in the
  commits and the residual plan (720 behavior comparisons, 6 mutants).
- Main-session self-review: every gain has the prefetch blocker as its previous
  reason and 0 methods were lost. This audit is admission evidence, not an
  independent semantic review.
