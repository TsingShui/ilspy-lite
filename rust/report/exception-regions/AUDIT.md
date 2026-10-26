# EH audit before repair

No subagents. Compared the current default readable output with ILSpy 11.2 for
three owned fixture methods, then checked the original IL. Nothing from the
third-party corpus was executed.

| method | ILSpy | current output | conclusion |
|---|---|---|---|
| Catch | try/catch with -1 handler | flat statements, no catch region | SUSPECT: a thrown exception cannot reach the handler |
| FinallyOrder | finally runs on normal and exceptional exit | goto skips cleanup, followed by __cil_endfinally() | SUSPECT: cleanup has no runtime control-flow meaning |
| Rethrow | nested catch regions with throw; | throw; outside a catch region | SUSPECT: rethrow context and selected exception are lost |

Triples are retained in `audit/<method>.{ours,ilspy,il}.txt`. The original IL,
not ILSpy's formatting, is the semantic authority. ILSpy recovers all three;
none is a both-decompilers-fail example. This small owned sample does not prove
that every EH method in the corpus is recoverable.

## Bounded repair design

Keep typed IL/CFG and offset-ordered statements. Group identical protected ranges,
require contiguous catch/finally bodies in table order, and validate that groups
are disjoint or nested within one whole parent subregion. Emit actual `try`,
`catch (T exception)` and `finally` braces in the lift, before readable cleanup.
Catch entry uses the runtime-bound exception variable, never a fabricated object.
`leave` becomes a goto out of the protected block so runtime cleanup runs;
`endfinally` ends only its own finally block; `rethrow` stays lexical `throw;`.
Reject crossing branches, illegal region transfers, filters, faults and layouts
that cannot be represented by this subset. Do not call a flat region table
recovered semantics. Readable cleanup must not move values across braces.

Validation: compile and execute default readable bodies of owned fixtures,
comparing returns, exception types, original exception identity and static trace
writes. Mutants must detect a dropped finally store, wrong handler result,
dropped rethrow and substituted exception identity. Retain the existing
exception-object fixture suite separately; it is not replaced by region tests.
No full scan or recovery claim until the behavior gate passes.
