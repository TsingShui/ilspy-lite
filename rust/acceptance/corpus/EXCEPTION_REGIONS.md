# Bounded exception regions

## Audit and scope

The default output must preserve runtime control flow, not only EH comments.
An audit of the unfinished implementation found that its flat goto bypassed
finally and its catch-entry placeholder did not catch anything. ILSpy emitted
real regions for those owned methods; original IL confirmed the discrepancy.
See `report/exception-regions/AUDIT.md`.

The accepted subset is typed catches for named core exceptions, finally,
nested regions and multiple contiguous catches in metadata order. No C# AST
or original-IL executor is introduced.

## Representation

`src/eh.rs` groups identical protected ranges and validates disjoint/nested
groups. A child group must fit wholly in one parent try or handler, not merely
inside the combined envelope. Boundaries must be instruction starts (or code
end). Table order is retained for catches. Groups with filters, faults, unknown
catch identities, interleaved/gapped handlers or crossing regions stay Partial.

CFG leaders include all accepted region boundaries. A handler is seeded only
after a protected block is reachable: typed exception on catch entry, empty
stack on finally entry. Stack joins and existing managed-location limits remain.

The lift emits lexical `try`, `catch (T exception)` and `finally` constructs
before readable formatting. Catch entry assigns the runtime's exception variable,
not a fabricated instance. `leave` clears the evaluation stack and exits with a
goto, letting runtime finally semantics handle both normal and exceptional
continuations. `endfinally` branches to the end of its own finally body.
`rethrow` remains a lexical `throw;` inside its catch. Return and ordinary
branches may not cross protected-region boundaries; only empty-stack fallthrough
can enter a try.

Readable flat-block compaction is disabled for region bodies so snapshot
lifetimes and effects cannot move across handler boundaries. Existing precise
leaf-call intrinsics are retained; none executes a whole handler or original
method.

## Tests and evidence

- The existing exception-object tests and fixtures are retained unchanged.
  Region tests use separate `ExceptionRegionCases.cs` and
  `exception-region-{release,debug}.bin` fixtures.
- `exception_regions.rs`: runtime braces, exact frozen filter refusal,
  scan/method parity, illegal branch/leave/endfinally rejection and fault refusal.
- `tests/exception-roundtrip.py --mutation-control`: compiles default readable
  method text without replacing EH constructs or supplying no-op helpers.
  Release/debug fixtures each pass 39 comparisons over eight recovered methods.
  Return values, static traces, thrown exception types and original exception
  identity are compared. Four mutants fail for their intended differences:
  finally store, catch result, rethrow and replaced exception identity.
- 369 Rust tests pass; clippy with warnings denied and formatting checks pass.
- Self-review: 11 saved corpus triples (our default output, ILSpy, original IL)
  cover nested iterator cleanup, finalization, a real Roslyn try/finally, a
  multiple-catch fixture and OpenRA checked-overflow handling. No third-party
  assembly was executed. This is not independent or formal verification.

## Same-scope scan

`report/exception-regions/corpus-scan/baseline.json`, compared with
`report/external-field/corpus-scan/baseline.json`:

| quantity | value |
|---|---:|
| implementation bodies | 770,748 |
| Decompiled / Partial | 336,090 / 434,658 |
| unrecovered rate | 56.39431% |
| new recoveries / images gaining | 1,191 / 119 |
| recovered-method regressions | 0 |
| incomplete images | 0 |
| body images passing <= 1% | 34 / 319 (previously 33) |

Every gained method previously had the EH/data-section blocker.
`catch` (inventory id 19) now recovers all five bodies; OpenRA.Game gains 17
and OpenRA.Mods.Common gains 58. The rest of the former EH bucket exposes
other instruction/type limitations; removing one first blocker does not
promise recovery of every body behind it.

## Deliberate limits

Filters, fault handlers, custom catch-type facts, noncontiguous/interleaved
layouts, byrefs across blocks and unsupported operations inside any region
remain fallbacks. Legal but non-lexical EH arrangements may be refused.
Generic owner spelling and exact-call intrinsics are still C#-like notation;
this slice does not promise every corpus output recompiles as standalone C#.
The owned carriers do compile and run. Third-party behavioral execution
requires separate approval.
