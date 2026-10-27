# Non-initialized local slots: ILSpy-first audit

## Upstream mechanism

- `IL/ILReader.cs::Init` assigns every local `InitialValueIsInitialized` from
  the method body's `.locals init` flag. `IL/ILVariable.cs` distinguishes the
  initial value being initialized from whether it is used at all; an unused
  initial value does not need implicit initialization.
- `IL/Transforms/RemoveDeadVariableInit.cs::ResetUsesInitialValueFlag` invokes
  `FlowAnalysis/DefiniteAssignmentVisitor.cs`. Its state tracks variables for
  which some path has not assigned a value; joins union those possibilities,
  stores clear the corresponding bit, and reads or address-taking check it.
  The broader ILSpy visitor also handles out calls, exception regions and
  finally flows, with exception states propagated conservatively by
  `FlowAnalysis/DataFlowVisitor.cs`.
- This Rust slice adopts the same may-be-unassigned fixed point over the existing
  typed CFG for methods without exception handlers. It requires every `ldloc` and
  `ldloca` to be preceded by a whole-slot `stloc` on every reaching path. It does
  not assume an out call writes an uninitialized slot or apply `Unsafe.SkipInit`
  to a possibly read initial value. No-EH restriction avoids approximating
  exceptional transfers or finally states. The emitted default declaration is
  unobservable only for slots proved unused before assignment/address-taking.

## Fixed-corpus candidates before implementation

Eight candidates retain production default output, ILSpy output, original IL,
commands and reference selections in `audit/`. ILSpy exits successfully without
unknown-result diagnostics; none of these bodies was executed.

| Image/token | IL observation |
|---|---|
| 0483-06000003 | `isinst` then `stloc.0` before either local read; generic comparer dispatch remains separate. |
| 0483-060000C1 | Address of a local passed to an out-like call before an EH/finally region; deliberately remains outside this slice. |
| 0463-06000003 | Same anonymous-type assignment pattern in another assembly. |
| 0473-06000034 | Initial assignment before a catch region; EH-aware proof remains separate. |
| 0501-06000018 | Two locally assigned slots in a loop; backward edges require a fixed point. |
| 0531-06000039 | A branch bypasses the local entirely; all paths to its reads assign first. |
| 0485-06000020 | Several slots and branches, with arrays and out storage as independent gates. |
| 0469-06000012 | Loop-carried local assignment and an early return that never reads the slot. |

The 5,447 first blockers in the preceding scan are not a recovery forecast.
A used uninitialized value, address-taking before assignment, exceptional
handlers, and all unrelated instruction/type gates remain Partial. No whole-body
proxy, target execution, or default-value assumption substitutes for that proof.

## Focused validation

The owned branch, loop and acquired-address fixture was red before implementation
and passes in both compiler configurations afterward. Removing a branch or loop
store, acquiring an out address before assignment, and a finally region remain
Partial. The reused behavior driver executes 163 comparisons / 40 closed
selections per configuration, including 20 comparisons for the four new bodies;
it rejects seven compiled mutants, including branch/loop return changes and
omitting the address-based mutation. Original and version-substituted reference
modes both pass. All 467 Rust tests and fmt/clippy pass.

Six of the eight audited corpus candidates now decompile in targeted static
checks; two remain Partial. No full-corpus gain or regression count is claimed
until the next three-group scan and self-review.
