# Brief: exception regions and handler control flow

Owner: worker. Dispatch after external allocation, because both touch the lift and the
constructor facts, and because `.cctor` bodies are frequent try/catch carriers.

Design: `report/eh-design/DESIGN.md` (read it fully; its RM-n / section numbers are the
contract for this slice). Why now: 39,234 first-blocked bodies over 251 images, and 39 of
the 49 smallest failing images need it, so it is simultaneously the broadest and the
deepest remaining capability.

## Scope

Implement the region model, the stack discipline and the emission exactly as designed:

- `rust/src/eh.rs` with the clause tree and validated block ranges (RM-1);
- `cfg::build` taking clause boundaries as extra leaders, with byte-identical output when
  no clause is accepted (RM-2) - this is a hard regression gate: the existing 297k
  Decompiled bodies must not change by a single byte;
- handler entry stack discipline, `leave`/`endfinally`/`endfilter`, filter booleans
  (section 2);
- emission of try/catch/finally with the evidence comments the design prescribes, and no
  proxies to original IL (section 3).

## Refusals (section 4, one stable reason each)

Overlapping regions that cannot be expressed, a clause boundary that is not an
instruction start, rethrow outside a handler, filter that writes state the writer cannot
model, and any handler whose stack discipline the design does not admit. These stay as
negative rows with their own reasons, never as a silent fallback.

## Deliverables (section 6)

1. Executable fixtures: try/catch return value, finally side effects on both paths,
   nested try, catch with filter, rethrow, exception thrown inside a static constructor,
   and a negative (unsupported overlap).
2. Frozen contract table with exact reasons for the negatives.
3. `tests/exception-roundtrip.py --mutation-control`, release and debug: returned values,
   finally side effects, filter decisions, exception types. Mutations: swap handler
   selection, drop the finally, invert the filter, run the handler twice.
4. `acceptance/corpus/EXCEPTION_REGIONS.md` + README index.
5. Report with the full scan delta, per-image gate table, reason mix, and an explicit
   statement that emission does not attempt layout/stack-trace parity.

## Gates

- Byte-identical output for every method without an accepted clause (compare the emitted
  text of a sample of 1,000 Decompiled bodies before/after, plus the full scan delta of
  zero changed methods outside EH).
- Full corpus scan against the previous baseline, zero lost methods, gate table.
- Full nextest, clippy `-D warnings`, fmt. No third-party execution. No commits.
