# Handoff: continue on another machine

Everything needed to resume the goal exactly where it stopped. Read this file first,
then `report/fallback-corpus/CHECKPOINT.md` and `PLAN.md`.

## 1. What state the repository is in

- Repo: `/Users/bytedance/zhome/ilspy-lite`, crate in `rust/`, `HEAD = 262f02850`.
- **The work is not committed.** `git status` shows 180 entries (modified + untracked),
  including 47 new binary fixtures under `rust/tests/fixtures/`, the decompiler changes in
  `rust/src/`, the new tests and drivers in `rust/tests/`, the docs in
  `rust/acceptance/corpus/`, and the reports under `rust/report/`.
- To move it to another machine, either commit it (recommended: the format hook runs on
  `.cs`, so use a normal commit with the `Assisted-by:` trailer) or archive the whole tree
  excluding build output:
  `tar --exclude='*/target' --exclude='*/target-worker' -czf ilspy-lite-handoff.tgz ilspy-lite`
  from the parent directory. Do not use a `git bundle`: bundles do not carry the
  uncommitted working tree.
- On the new machine: `cd rust && cargo build && cargo build --release`, then
  `cargo nextest run` (expect **362 passed**). If `cargo-nextest` is missing:
  `cargo install cargo-nextest --locked`.

## 2. Verified state at handoff

| quantity | value |
|---|---:|
| tests | **362 passed**, 0 skipped |
| clippy (`--all-targets -- -D warnings`) | 0 errors |
| fmt | clean |
| last full scan (`report/defect-sweep/corpus-scan/baseline.json`) | **333,097 Decompiled / 437,651 Partial** |
| bodies / images / incomplete | 770,748 / 319 body images / **0** |
| unrecovered rate | **56.78263%** |
| per-assembly gate (`report/per-assembly-gate.py`) | **33 / 319** pass |

Delta against the previous milestone baseline
(`report/boxing/corpus-scan-enum/baseline.json`, 297,132 Decompiled): **+35,965, 204
images gained, 0 methods lost**. Attribution was measured, not assumed:

- **+18,288** from the sealed-owner mask fix; every one of them previously had
  `allocation owner must be a concrete class` as its first blocker (measured by rebuilding
  with the old mask and scanning into `report/defect-sweep/scan-no-sealed/`).
- **+17,677** from the other three changes, by previous reason:
  7,547 `enum storage requires an exact local definition`, 7,027 `constructed value type
  needs a local definition`, 1,386 `.cctor` blanket, 1,325 opaque external callvirt,
  390 `box operand type differs`, 2 box/unbox scalar.
- The last five reason buckets need one review pass to explain *why* they moved (the
  `.cctor` gate alone should not unlock an "enum storage" body). Sample dumps show they
  decompile correctly, but the mechanism is unverified and must be documented before the
  slice counts as reviewed.

## 3. What landed in code this session

1. **Static constructors** (`rust/acceptance/corpus/STATIC_CONSTRUCTORS.md`): the blanket
   refusal is gone - `.cctor` is lowered as an ordinary static void body and rendered as
   the type's static constructor. A `static readonly` field may be assigned only inside
   the declaring type's own `.cctor`; every other readonly static write keeps
   `readonly static field write needs IL fallback`. Empty-name and runtime-special methods
   keep `empty/runtime-special method needs IL fallback`.
2. **Sealed allocation owners**: `metadata.rs` now rejects only `interface | abstract`
   (`0x20 | 0x80`) instead of `0x20 | 0x80 | 0x140`; a sealed class is instantiable, a
   static class is still refused through `abstract`. See
   `report/allocation-owner/DEFECT-sealed-mask.md`.
3. **Scalar box/unbox stack category** (`report/boxing/DEFECT-box-scalar-stack-type.md`):
   `box` compares `Ty::stack()` and casts to the declared type, so `char`/`bool`/narrow
   integers no longer fail with `box operand type differs`.
4. **Enum unbox normalization** (`report/boxing/DEFECT-enum-unbox-cast.md`): `unbox.any`
   pushes the normalized stack category and renders an explicit conversion to the storage
   type, so all eight storage widths decompile; previously only `int` storage did.

Contract tables were re-baselined (promotions removed, one reason renamed to
`value-type receiver needs IL fallback`), never loosened.

## 4. Known red / unfinished (do these first)

1. **`tests/enum-roundtrip.py` fails**: `ValueError: Owned enum unbox selection drift`.
   The driver's frozen `Owned enum unbox` selection predates the fix; the 8 `Unbox*`
   methods now decompile, so the selection/expectations must be updated (and a non-null
   boxed value must be fed, per the defect note). `generic-declaring-roundtrip.py` is
   green (18 comparisons, 11 recovered methods, wrong-field mutant rejected).
2. **Docs not yet updated for the new numbers**: `BOXING.md`, `STATIC_CONSTRUCTORS.md`
   (its "why" still says 13,335 rows with no measured result), `PLAN.md`, `CLOSURE.md`
   and `report/fallback-corpus/CHECKPOINT.md` still describe the pre-defect-sweep state.
3. **No `report/defect-sweep/REPORT.md`**: the slice has no written report yet; the
   numbers in section 2 above are the evidence to write it from.
4. **Unreviewed semantics**: the 7,547 + 7,027 gains described in section 2.

## 5. Next steps in order

1. Fix the enum driver selection and re-run it release + debug with
   `--mutation-control`; also run `new-object`, `struct-receiver`, `virtual-call` and
   `objects` drivers (their fixtures changed).
2. Write `report/defect-sweep/REPORT.md` with the section 2 numbers and the per-reason
   attribution.
3. Explain and document the enum/constructed-value gains (find the code path that admits
   them; if it is an unintended relaxation, either justify it with a fixture or revert it
   and re-measure).
4. Update the docs named in section 4 and refresh the `PLAN.md` queue.
5. Continue the queue: **external allocation** (design and brief are ready:
   `report/external-allocation/DESIGN.md`, `BRIEF.md`), then **exception handling**
   (`report/eh-design/DESIGN.md`, `BRIEF.md`), then reference-selection work
   (`report/reference-resolution/FINDINGS.md`).

## 6. Environment and commands

- Rust toolchain as pinned by `rust-toolchain.toml`; `python3`; `cargo-nextest`.
- `.NET`: the drivers compile C# fixtures with `dotnet`; only SDK 10.0.401 was available
  on the old machine (`~/.dotnet10`), while `global.json` pins 11 with `rollForward: major`.
  For the ILSpy side, `pwsh` was missing, which makes the `ILSpyUpdateAssemblyInfo` target
  fail; verification worked around it by stamping
  `ICSharpCode.Decompiler/obj/update-assemblyinfo-last-commit-hash.txt`.
- Never run a bare `dotnet build` in the repo (it prunes `packages.lock.json`); use
  `restore.ps1` / `build.ps1` if the .NET side must be built.
- Core loop:
  - `cd rust && cargo nextest run`
  - `cargo clippy --locked --all-targets -- -D warnings && cargo fmt`
  - `cargo build --release`
  - `python3 acceptance/corpus/corpus.py baseline --inventory report/core-type-roots/scope-policy/implementation-inventory.json --cli target/release/ilspy-lite --output report/<slice>/corpus-scan --references declared`
    (about 45 s)
  - `python3 report/compare-recovery-scans.py <before> <after> <delta.json>`
  - `python3 report/per-assembly-gate.py <baseline.json>` (integer gate; exits 1 if any
    image is above 1%)
  - Fixtures: `python3 tests/build-<name>-fixtures.py`; behaviour:
    `python3 tests/<name>-roundtrip.py --mutation-control --cli target/release/ilspy-lite`.

## 7. Method decisions from this session

- **Subagent-based development was abandoned.** Implementation agents ran 60-90 minutes
  per slice, ignored time-box steers (single long step), and two produced nothing usable;
  direct work by the main session was several times faster. The only agent use that paid
  off was read-only diagnosis/design/review.
- Keep the frozen contract tables and the "no status change without decompilation" rule;
  never loosen an assertion to make a slice pass - promote the row and record why.

## 8. Recreating the goal on the new machine

A new session does not inherit this machine's goal object. Recreate it with the same
objective and task tree:

1. Start pi in the repository and call `create_goal` with the objective text below
   (`mode: regular`; no token budget was set).
2. Call `set_goal_tasks` with the task list in section 8.2, copying the statuses; then
   call `update_goal` to keep `calls` as the current task.
3. Read this file and `CHECKPOINT.md` before the first action. The verified state in
   section 2 is the starting point; do not re-derive it.

### 8.1 Objective (verbatim, Chinese)

```
扩展 Rust ilspy-lite 的真实反编译能力，在固定 ILSpy-tests 全量预编译托管语料及当前两个 OpenRA 程序集中，跳过仅提供声明/参考契约、没有真实实现的参考程序集，使有真实实现的程序集分别达到未恢复方法体比例 <= 1%，同时保持语义可信、证据完整。
```

Success criteria, boundaries, constraints and the verification contract are reproduced in
`CHECKPOINT.md` (search for "Success criteria"). The essential invariants:

- fixed ILSpy-tests commit `9668d49b9f71b40045e5417b55d20e330be95921` plus `OpenRA.Game.dll`
  and `OpenRA.Mods.Common.dll`; never re-acquire unchanged inputs;
- reference-only assemblies are skipped with per-input static evidence, never by name or
  by decompilation failure, and never counted in the denominator;
- per-assembly gate is integer arithmetic: `100 * unrecovered <= bodies`; the reported
  rate must not be a rounded display value;
- keep typed IL/CFG + lift, no C# AST, no execution of third-party assemblies, no status
  changes, no hidden methods, no proxies to original IL;
- each capability needs red-before-green E2E plus behaviour/mutation evidence, and an
  independent read-only source review before its gains are counted.

### 8.2 Task tree

| id | title | status |
|---|---|---|
| corpus | Acquire the fixed corpus and build the full assembly/method inventory | complete |
| baseline | Per-assembly baseline, first-failure classification, 1% gate | complete |
| implementation-scope | Skip reference-only inputs with evidence, reconcile denominators | complete |
| design | Confirm architecture and implementation order | complete |
| types | Signatures, value types, generics | complete |
| calls | Calls, construction and related opcodes | **current** |
| control | Exceptions, control flow, remaining semantics | pending |
| behavior | Representative real-corpus behaviour gates (needs user approval) | pending |
| closure | Per-assembly convergence and final acceptance | pending |

`get_goal(section="tasks")` on the old machine holds the completion evidence for the five
completed tasks; copy it into the evidence fields when recreating them, so the audit trail
is not lost.
