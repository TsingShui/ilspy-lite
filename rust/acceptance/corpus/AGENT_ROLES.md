# Agent roles

The main session commands; it does not write product code, build, scan or run
drivers. Each slice is a short brief from the commander, then the three agents below
run asynchronously. Nobody blocks on another agent: the commander starts the next
slice while verification and review are still running.

## 1. Worker (the only writer)

- Owns `rust/src/**` and `rust/tests/**` for one slice at a time.
- Builds and tests with `CARGO_TARGET_DIR=rust/target-worker` so it never contends
  with the shared `target/` directory.
- Returns: files touched, red-before-green evidence, frozen contract table shape,
  driver comparison count and mutation verdict, and every semantic limitation it
  deliberately gated.
- Never runs the corpus scan (that is verification's job), never executes
  third-party assemblies, never commits.

## 2. The implementation agent also verifies its own slice

One agent per slice owns the whole loop: source change, fixtures, contract table,
driver with mutation control, `cargo build --release` and `cargo build`, full
`corpus.py baseline` into `report/<slice>/corpus-scan/`, deltas via
`report/compare-recovery-scans.py`, the ILSpy dumps that review consumes
(`report/<slice>/review/ilspy/`), and the report.

Why one agent instead of two: a slice has to be measured with the binary it just
built, so splitting the work only adds a serialized round trip and a chance to scan a
stale binary. It is also the only agent that touches `target/`, so there is no lock
contention.

It writes `report/<slice>/REPORT.md`: what changed, red-before-green, scan counts and
rate, gained/lost per baseline, gate distance, the slice's behaviour comparisons and
mutation verdicts, dump index, and what the run does not prove. An image that lost a
method, an incomplete scan, a driver that fails to build, or a mutation that survives
is a failure, not a footnote.

## 3. Review (read-only judgement)

- Compares **our output against ILSpy's output** for the slice's fixture pairs and a
  spread of corpus samples; the original IL is third evidence, used only to settle a
  disagreement between the two decompilers.
- Verdict per pair: EQUIVALENT, CRUDE-BUT-EQUIVALENT, SUSPECT, INSUFFICIENT-EVIDENCE.
- Writes `report/<slice>/review/review-source.md` with the table, a SUSPECT section
  quoting both outputs, and the counts. SUSPECT rows are fixed or demoted before the
  slice counts as a gain.
- Does not build, execute, or edit source.

## Slice loop

1. Commander sends the worker a brief: goal, semantics to preserve, refusals to keep,
   deliverables, gates, and the files it may write.
2. Worker implements and proves red-then-green locally.
3. The implementation agent measures its own slice (build, scan, delta, drivers, dumps)
   and the commander launches review asynchronously at the same time as the next brief.
4. The scan decides whether the slice counts as a gain; review decides whether it is
   semantically sound. Either one can reject the slice.
5. Docs land with the slice: `acceptance/corpus/<SLICE>.md` plus the README index and
   the `CHECKPOINT.md` "Authoritative state" line.
