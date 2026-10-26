# Bounded EH milestone

## Changes

Validated lexical catch/finally regions now carry actual runtime semantics in
the default output. Ordinary region-crossing branches and invalid exits are
refused; catch entry binds the runtime exception; rethrow is lexical throw;
finally executes through C# region exits. Filters, faults and unverified
catch-type identities remain Partial. Flat readable compaction is disabled
for region bodies to preserve value lifetimes and effects.

The unfinished working-tree implementation's scalar-address-to-value shortcut,
broad external casts and expanded exception constructors were not included.
Existing exception-object tests/fixtures are retained separately.

## Red, green and behavior

- Pre-repair triples: `AUDIT.md`, `audit/*.txt`.
- Red: `red.log` rejects the default output's missing runtime try/catch braces.
- Focused shape/negative/parity tests: `boundaries.log`.
- Full nextest: 369 passed, zero skipped (`final-tests.log`).
- Clippy warnings denied and cargo fmt checks passed.
- Executed default readable bodies, no EH adapter: 39 comparisons over eight
  recovered owned methods per release/debug configuration. Static traces,
  result values, exception types and original exception identity match.
- Four mutants rejected: finally store, catch result, rethrow, original-object
  replacement. `behavior/behavior-results.json` records each result.
- Existing exception-object readable behavior: 259 comparisons per release/debug
  configuration still pass (allocation, identity, order, exception parameters).
- Region-table-only checks are not substituted for the behavior gate.
- No third-party assembly was executed.

## Frozen full corpus

| metric | before (external fields) | after |
|---|---:|---:|
| bodies | 770,748 | 770,748 |
| Decompiled | 334,899 | 336,090 |
| Partial | 435,849 | 434,658 |
| body images passing <= 1% | 33 | 34 |
| incomplete images | 0 | 0 |

Gain: 1,191 methods in 119 images; regressions: zero. All gains formerly had
`exception/data sections need IL fallback`. Current unrecovered rate:
56.39431%. Inventory 19 (`catch`) recovers all five methods. OpenRA.Game gains
17; OpenRA.Mods.Common gains 58. Corpus exit 2 is expected because the overall
per-image goal is still unmet.

Most EH methods expose another blocker rather than decompile. Remaining
EH-specific first blockers include catch identity 2,881; fault 541; filter 215;
plain boundary-crossing branch 140; unsupported leave origin 63; leave target 6;
clause limit 1.

## Static review

Eleven source/ILSpy/original-IL triples, including both pinned OpenRA images:
11 CRUDE-BUT-EQUIVALENT for the region semantics reviewed; no EH differences
found. This main-session self-review is not independent or formal evidence.
Existing generic-owner and exact-call notation is not certified as standalone
C# by the corpus review. See `review/review-source.md`.

## Commands

```sh
cargo nextest run --no-fail-fast
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
cargo build --release
python3 tests/exception-roundtrip.py --mutation-control --cli target/release/ilspy-lite
python3 acceptance/corpus/corpus.py baseline --inventory report/core-type-roots/scope-policy/implementation-inventory.json --cli target/release/ilspy-lite --output report/exception-regions/corpus-scan --references declared
python3 report/compare-recovery-scans.py report/external-field/corpus-scan/baseline.json report/exception-regions/corpus-scan/baseline.json report/exception-regions/delta.json
```
