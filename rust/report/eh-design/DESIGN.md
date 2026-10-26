# Exception regions and handler control flow: implementation design

Design only. No build, scan, or source edit was performed for this document.

Baseline (`rust/report/boxing/corpus-scan-enum/baseline.json`): 614 managed / 319 body images /
770,748 bodies, 297,132 Decompiled. `exception/data sections need IL fallback` is the second
largest first blocker: 39,234 bodies over 251 images. Largest holders: ExplicitConversions.exe
7,308 (50% of its Partial), ExplicitConversions_32 2,340, mcs/5.23/4.5/System.dll 1,280,
mscorlib 1,178, System.Xml 922. Smallest dominated images: SOS.NETCore.dll 13 of 33,
ImplicitConversions 263 of 527.

## 1. Region model

Clauses are already decoded by `cildec::MethodBody::exception_handlers`: `kind` (`Catch(Token)`
| `Filter { filter_offset }` | `Finally` | `Fault`),
`try_offset`/`try_end()`, `handler_offset`/`handler_end()`, `raw_flags`
(cildec-0.1.0 `src/body.rs:59-103`); offsets index `body.code`, blocks are
`cfg::Block` (`rust/src/cfg.rs:8-13`).

RM-1. New `rust/src/eh.rs` owns the decoded clauses and a region tree: `Clause { index, kind,
catch: Option<Ty>, try_start, try_end, filter_start, handler_start, handler_end, blocks:
RegionBlocks }`, where `RegionBlocks` holds half-open `try_blocks`, `filter_blocks`,
`handler_blocks`. A clause boundary that is not an instruction start refuses; ranges are never
rounded.

RM-2. `cfg::build` gains extra leaders: `build(instructions, code_size, boundaries: &[u32])`,
boundaries from `try_start, try_end, filter_start, handler_start, handler_end` of every accepted
clause, validated by the same "inside an instruction" check as branch targets (`cfg.rs:32-38`).
With no accepted clause the slice is empty, so leaders, block numbering, labels and emitted text
of all 297,132 Decompiled bodies stay byte-identical. No other `cfg.rs` change is needed:
`leave`/`leave.s` are `FlowControl::Branch`, `endfinally`/`endfilter` are `Return`, `rethrow` is
`Throw` (`src/il/table.rs:2330,2340,2530,2620`).

RM-3. Exceptional successors are not folded into normal edges: a handler is entered by the
runtime with a stack the try block cannot predict, so merging it into normal-edge dataflow would
fabricate or destroy the existing join check. The lift seeds entry stacks instead: `Catch(T)` ->
`[T]` (one reference of the declared catch type); `Filter` -> `[Ty::Object]` (declared object,
no cast); `Finally`/`Fault` -> `[]`. New `Image::catch_type(token) -> Result<Ty>` sits beside
`exception_constructor`/`boxed_value_type` (`metadata.rs:2774-2840`): TypeDef/TypeRef becomes
`cildec::Type::Class`, TypeSpec goes through `Image::type_spec` (`metadata.rs:3336`), so a
generic catch type resolves structurally or refuses.

RM-4. Reachability stays honest about dead code. Phase 1 propagates from block 0 over normal
edges (today's loop, `lift.rs:115-146`). Phase 2 repeats to a fixpoint: for each clause with a
reached block in `try_blocks`, seed its filter entry then its handler entry and propagate
normally. Every block must still be covered; leftovers report `EH protected region is
unreachable` when they belong to a clause, else keep `unreachable basic block requires IL
fallback`. A clause is never seeded from an unreachable try.

RM-5. Accept strict containment (try in try, try in handler, try in filter) and several clauses
sharing one `try_start`/`try_end` with contiguous non-overlapping handler ranges (multi-catch,
and the filter-then-handler pair where `filter_end == handler_start`). Refuse partial overlap,
and a shared try with gapped or duplicated handler ranges. Limits: 64 clauses, depth 8, the
existing 4,096-block cap. Clause order is never reordered: EH table order is selection order.

RM-6. Regions do not restructure the walk; they are facts about blocks. The one-pass emitter
still iterates blocks in ascending IL offset order (`lift.rs:169-234`), names entry values
`s{block}_{slot}`, values `vN`, and snapshots the outgoing stack into `e{n}_{s}` temps before a
`goto` (`lift.rs:194-207`). Handler entry stacks are seeds of that same scheme, so a handler's
first value is `s{n}_0` with its declared type. Nothing is hoisted, duplicated, inlined or sunk
across a region boundary, so the preserved evaluation order is untouched. Membership only:
faultability stays attached to the operation that faults; we never claim "this may throw here".

## 2. Stack discipline inside handlers

SD-1. `leave`/`leave.s` from a try or a catch handler: the evaluation stack is discarded, every
enclosing finally (and fault-on-exceptional-exit) region between source and target runs
innermost-first, then control continues at the target. Implementation: pop nothing,
`self.stack.clear()` exactly as `throw` does (`lift.rs:1258`), emit `leave IL_xxxx;` plus one
evidence line naming the finally clauses run and their order. The stack is cleared before the
edge copy, so the leave edge carries an empty stack and the target's join check enforces it;
`edge()` needs no special case.

SD-2. The `leave` target must lie outside the clause whose try/filter/handler region contains
the transfer, i.e. not inside any region of the current region stack at or above the innermost
try. A target inside its own protected region is a refusal.

SD-3. `endfinally` ends a `finally` or `fault` handler; entry and exit stacks empty; no normal
successor. Emit `endfinally;` and terminate; a finally/fault block that falls out of its region
without it refuses.

SD-4. `endfilter` ends a filter; pops exactly one value that must be `Ty::Bool` or `Ty::I4`/`U4`
(nonzero selects this handler, zero continues the search). Emit `endfilter(<expr>);` with
evidence that the filter's effects already happened and that zero continues rather than
rethrows. Two values, or a non-integer/bool value, refuses.

SD-5. The catch entry value is one reference of the declared catch type, provided by the runtime
after it matched the type, and it is the same object identity that was thrown - which is why
`throw e` from the handler rethrows that instance and `rethrow` differs only in its recorded
trace. No null check, no cast, no copy is added: a castclass adds an impossible
`InvalidCastException`, a null test an impossible `NullReferenceException`. A filter entry is
declared `object`; a filter wanting the concrete type contains its own `isinst`/`castclass`.

SD-6. `rethrow` is legal only while the innermost region-stack entry is a catch handler (filter
and finally illegal), stack empty. It re-raises the original object preserving identity and
recorded trace and does not re-run catch selection for the same group.

SD-7. `throw` inside a region is the already-admitted operation (`lift.rs:1248-1259`); it clears
the stack and has no successor, so no new code is needed, only an extended admitted operand set:
`Ty::Class(t)` with provable `System.Exception` ancestry in this image, beside the existing core
exception and null.

SD-8. Refused: a cross-finally transfer by plain `goto`; a `leave` in a filter or finally;
`endfinally` with a non-empty stack; a catch handler entered with anything but the single
object; an address (`StackKind::Address`) live at a region boundary; a handler region that falls
through.

## 3. Emission

No C# AST, no new node type: output stays the flat, offset-ordered, labelled statement list with
evidence comments.

- Replace the blanket guard (`lift.rs:13`) with `if !body.unknown_sections.is_empty()`,
keeping the exact string `exception/data sections need IL fallback`, so a body with a
non-decodable data section keeps its bucket and that string covers only that case.
- Emit the region table in the format `disassemble` already uses
(`metadata.rs:1845-1866`) with resolved catch names: `// .try IL_0000..IL_0012 catch
global::System.Exception IL_0012..IL_0020`, `// .try IL_0000..IL_0012 finally IL_0020..IL_0028`,
ordered by clause index, then `// .eh-order: selection follows the clause order above, not type
specificity.` and, for a shared try, `// .eh-order: clauses 0,1 share this protected region.`
- At each handler-entry block, before its `IL_xxxx: ;` line: `// EH entry clause=0
catch: runtime-matched object; identity preserved; no cast and no null check are added.` then
`<type> s12_0 = __cil_handler_exception(0);`; a filter entry uses `__cil_filter_exception(0)`
declared `object`. These follow the established intrinsic naming (`__cil_newobj_TOKEN`) and
denote one runtime-supplied value.
- `leave IL_xxxx;`, `endfinally;`, `endfilter(<expr>);`, `rethrow;` are the four
documented transfers, each with one evidence line (finally order for `leave`, runtime
propagation for `endfinally`, nonzero-selects-this-handler for `endfilter`,
same-object-same-trace for `rethrow`).
- Handler instructions go through the existing `instruction()` matcher unchanged: one
unadmitted operation anywhere inside any region makes the whole method Partial with complete IL,
as today.

Phase B (presentation only): wrap region block ranges in balanced `try {`, `} catch (<type> e0)
{`, `} finally {`, `} fault {` lines in `readable.rs`, gated on every handler region starting
after its try region (otherwise offset-ordered emission cannot nest the braces). The
brace-counting indenter already accepts a `} catch (...) {` line (`readable.rs:626-645`), so no
indenting change is needed, but region braces must be alone on their line. `leave` keeps naming
its target even when that label is textually outside the braces, with evidence that the exit
runs the finally regions.

No proxies to original IL: each construct is the operation whose CLI semantics the evidence
states, never "execute the original handler"; catch types are resolved identities, never guessed
names; selection order equals table order; no instruction is moved, merged or dropped; `.eh`
rows and `__cil_handler_exception` evidence a runtime-supplied value, not delegation; no helper
re-runs EH.

## 4. Refusals (single-line, ASCII; consts in `rust/src/eh.rs`)

- `EH regions overlap without nesting` - partial overlap or crossing try-handler ranges
- `EH try region needs contiguous ordered handlers` - shared try, gapped or duplicated handlers
- `EH clause boundary is not an instruction start` - boundary inside an instruction, or zero length
- `EH clause or nesting limit exceeded` - >64 clauses or depth >8
- `EH protected region is unreachable` - no reached block in the try range
- `EH handler entry also has a normal predecessor` - a normal edge targets a handler entry
- `EH leave target is inside a protected region` - SD-2
- `EH leave in a filter or finally region` - SD-8
- `EH leave crosses a finally region by plain branch` - `br`/`b*` over a finally range
- `EH finally region must end with endfinally` - SD-3
- `EH endfinally needs an empty evaluation stack` - SD-3
- `EH filter region must end with endfilter` - SD-4
- `EH filter must leave one boolean value` - SD-4
- `EH filter leaves a managed location live across endfilter` - address at a filter boundary
- `EH rethrow outside a catch handler` - SD-6
- `EH catch type needs an exactly resolved type identity` - unresolved TypeRef/TypeSpec/open generic
- `EH handler region precedes its protected region` - Phase B only

## 5. Ordering and risk

Land in order: (1) `throw` over local exception classes with proved `System.Exception` ancestry
- without it most handler bodies that end in `throw` stay refused and the milestone measures
nothing; (2) `eh.rs` tree and validation; (3) the `cfg::build` boundary argument; (4) dataflow
seeding and reachability; (5) the four transfers and entry bindings; (6) Phase B braces. Nothing
else in the pipeline moves.

Can wait: `fault` beyond flat admission (a decision point - `fault` has no C# spelling; a flat
`.try ... fault ...` region noted "runs only on exceptional exit" is defensible, refusing it
also is), deep nesting in Phase B, filters needing unadmitted capabilities.

Where a wrong design breaks verified semantics: hoisting a try body into an expression or
inlining a finally at each `leave` reorders or duplicates effects (one copy of every
instruction, always); a copying leave edge smuggles a value across a region boundary (SD-1
prevents it, the join check is the backstop); `goto` for a cross-finally transfer drops cleanup,
a finally run at both the leave and region end duplicates it, and `endfinally` as `return`
swallows the propagating exception; sorting catches by specificity changes which body runs. A
`.cctor` already lowers (`lift.rs:64-84`), and an exception escaping it is wrapped by the target
runtime in `TypeInitializationException`, faulting the type permanently, so evidence must
attribute the wrapping to the target runtime and never model the wrapper.

Predicted outcome: far fewer than 39,234 recovered bodies, because handler bodies commonly hold
still-unadmitted operations (external ctor calls, interface dispatch). The progress signal is
the reason-mix shift - bodies leaving the EH bucket for a named inner blocker - plus the newly
Decompiled ones. Report both; do not judge the milestone by `-39,234`.

## 6. Acceptance plan

Owned executable fixtures shaped like the existing ones: `tests/dotnet/EhCases.cs`,
`tests/build-eh-fixtures.py` (local .NET 10 Roslyn, release and debug), `tests/eh.rs`,
`tests/dotnet/EhBehavior.cs`, `tests/eh-roundtrip.py --mutation-control` in the style of
`tests/new-object-roundtrip.py`. Nothing in the frozen corpus is executed.

1. `TryCatchReturn(int x)`: `try { if (x > 0) return 1; throw ... } catch (Exception) { return 2; }` -> 1 and 2.
2. `FinallySideEffect()`: counter incremented in `finally`; once on the normal path and once on the throwing path.
3. `NestedTry()`: nested try/finally where one `leave` crosses both, appending to a `StringBuilder`; order "inner,outer", each once.
4. `Rethrow()`: `catch { throw; }`; caller asserts the same instance (`ReferenceEquals`) and a trace still naming the inner throwing method.
5. `FilterSelects(int x)`: `catch (Exception e) when (Filter(e, x))` with a counter in the filter plus a later `catch (Exception)`; filter runs once, selection flips, later catch does not run.
6. `MultiCatch(int i)`: `ArgumentException` / `InvalidOperationException` / `Exception`; selection order holds and the general catch is last.
7. `StaticCtorCatch()` plus a `.cctor` that lets the exception escape; asserts `TypeInitializationException` wrapping in the owned fixture only.
8. `LeaveWithLiveStack()`: legal `leave` with a value still on the stack; the value is discarded and does not reach the target.
9. `CatchInLoop()`: try in a loop plus an inner try in the catch handler; exercises block reuse and back edges with regions.

Negative rows (Partial with the exact new reason and complete IL): overlapping regions and a
gapped shared-try region (hand-written IL), `rethrow` outside a catch, `leave` in a finally,
filter with two stack values, finally without `endfinally`, catch of an unresolvable generic
external type, address live across `endfilter`.

Mutation controls, each of which must be detected (a survivor means the fixture is vacuous): M1
reorder selection (`Exception` first) -> rows 5, 6; M2 drop a finally (`leave` as `goto`) ->
rows 2, 3 (zero markers); M3 duplicate a finally -> rows 2, 3 (count 2); M4 null-check or cast
the catch object -> row 1; M5 leave edge copies the stack -> row 8; M6 `rethrow` as `throw e` ->
row 4; M7 invert or re-evaluate the filter -> row 5; M8 `endfinally` as `return` -> row 2.

Corpus side, no execution: one same-scope rescan with one release build; report
Decompiled/Partial, zero status regressions, the EH-bucket delta and the full reason-mix delta.
Independent review per `REVIEW_PROTOCOL.md`, sampling 10-15 gained readable dumps against the
original IL with ILSpy as secondary reference, from ExplicitConversions.exe, mcs/4.5/System.dll,
mscorlib, System.Xml, OpenRA.Mods.Common, System.Private.Xml, Microsoft.CodeAnalysis net472 and
the small SOS.NETCore.dll / ImplicitConversions images; prefer dumps with a multi-catch, a
nested try, a filter and a rethrow.

## 7. Not read, and what stays uncertain

Not read in full: `metadata.rs` (3,976 lines; only ~1,830-1,900, 2,774-2,900, 2,967-3,100 and
3,336), `readable.rs` beyond the method/type view and the fallback path (~470-560, 626-645),
`signature_types.rs`, `dependencies*.rs`, `main.rs` beyond usage, `signature_guard.rs`, the
corpus tooling, `REVIEW_PROTOCOL.md`, `AGENT_ROLES.md`, and every per-milestone document except
`LOCAL_ALLOCATION.md`. `cfg.rs`, `ir.rs`, `lift.rs`, `lib.rs` and the corpus `README.md` were
read as instructed; `SEMANTICS.md` and `DESIGN_PROPOSAL.md` in full. No build, test, scan or
corpus access was performed.

Uncertain: (a) how many of the 39,234 bodies also carry `unknown_sections`, which would keep the
old string and shrink the bucket before any code is written; (b) whether any corpus body places
a handler at a lower offset than its try, deciding how much Phase B can nest; (c) the `fault`
decision; (d) whether the gate or the reports diff individual reason strings (the table is
proposed as the contract, but nothing here proves the tooling pins them); (e) the real recovered
count; (f) whether an external generic catch type is common enough to justify `StructuralRef`
instead of refusal.
