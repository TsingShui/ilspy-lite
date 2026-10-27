# Definite assignment of non-initialized locals

Methods without `.locals init` can be lowered when they have no exception
handlers and every reachable `ldloc` or `ldloca` follows a whole-slot `stloc`
on every incoming path. The existing CFG propagates a may-be-uninitialized bit
per local, joining by union and iterating to a fixed point. A branch that never
uses a local need not assign it; a loop-carried assignment cannot make a missing
entry assignment safe.

The normal output's default local declarations cannot be observed before the
proved assignment. An address is a use even when it is passed to an `out`
parameter; direct `initobj`, out-parameter guarantees, exception transfers and
finally semantics are not inferred. A genuinely uninitialized read, an address
before assignment, or an exception region remains Partial.

`report/local-definite-assignment/AUDIT.md` traces ILSpy's initial-value and
`DefiniteAssignmentVisitor` mechanism and records eight pre-change corpus
candidates. Six now decompile in targeted static runs; this is not a complete
corpus delta and the batch scan/self-review remains pending.

The owned fixture uses `SkipLocalsInit` and checks the actual IL header. Branch,
loop, early-return and acquired-local-address cases run from recovered output
against the original, with five inputs per method in each configuration. The
reused reference-relation driver performs 163 total comparisons across 40 closed
selections and rejects seven mutants per configuration, including branch, loop
and omitted byref mutation. Its version-substitution mode passes separately.
Patching an assignment away on a branch or loop, acquiring an address before
assignment, and a non-initialized method with a finally region remain negative
coverage. All 467 Rust tests and fmt/clippy pass. No third-party method was
executed.
