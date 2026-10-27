# Residual audit and continuation plan

Status at commit time: latest complete fixed-scope scan is
`report/foreign-hidden-batch/corpus-scan-fixed/baseline.json`
(**728,704 Decompiled / 42,044 Partial of 770,748**, fallback 5.454961%).
The acceptance gate is overall weighted **1%**, i.e. `100 * unrecovered <= bodies`:
at most 7,707 may remain, so **34,337 more bodies** are required. The former 5%
gate needs 3,507 and is only an intermediate milestone.

This document is the working plan for continuing on another machine. It records
what the residual failures actually are, which shared machinery each group needs,
and the exact commands and pitfalls for reproducing the measurements.

## 1. What the 42,044 residuals consist of

123 distinct first-blocker strings. `reasons.json` holds the full table; the
concentration matters more than the list:

| First blocker | Bodies | Images | Nature |
|---|---:|---:|---|
| `incompatible value/receiver types` | 12,263 | 193 | heterogeneous; see section 2 |
| `invalid user-string encoding: terminal flag differs from code units` | 2,739 | 54 | corpus data, not a modeling gap |
| `external constructed value generic method needs dependency facts` | 2,269 | 66 | prefetch gap; slice implemented, unmeasured |
| `generic virtual dispatch needs runtime lookup-order modeling` | 1,999 | 49 | needs dispatch modeling |
| `pinned/byref/modified local needs IL fallback` | 1,666 | 73 | one `fixed` local rejects the whole body |
| `external allocation needs dependency facts` | 1,325 | 54 | same prefetch gap as above |
| `local may be used before definite assignment` | 813 | 18 | analysis gap (442 in `System.Private.CoreLib`) |
| `EH fault handlers need exceptional-only cleanup` | 640 | 72 | unmodeled region kind |
| `nested/generic/unresolved external class needs IL fallback` | 631 | 34 | external type facts |
| `invalid/modified/open/managed-address struct field needs IL fallback` | 563 | 15 | byref family |
| `byref/modified parameter needs IL fallback` | 549 | 16 | byref family |
| `conv.ovf.*` (23 opcode strings) | ~6,700 | few | conversion matrix |
| `conv.*` source category (11 strings) | ~1,500 | many | conversion matrix |
| EH filters | 444 | 56 | unmodeled region kind |
| unreachable basic block | 443 | 9 | CFG shape |
| cast target is a value type | 402 | 68 | boxed-test cast |
| modified value-call parameter | 363 | 21 | byref family |
| constructor overload selection | 353 | 27 | binding |
| method implementation flags | 317 | 12 | method flags |
| base field access needs constructed owner name | 302 | 32 | naming |
| `unbox` | 297 | 15 | opcode support |

Top 14 strings = 27,007 bodies; the remaining **109 strings = 15,037 bodies**.
Reaching 1% therefore requires clearing most of the long tail, not just the head.

## 2. Sub-cause decomposition of the largest bucket

Instrumented production scans (method-level exact attribution: 12,280 records for
12,263 first blockers; the 17 extra are retry duplicates) split
`incompatible value/receiver types` into families:

| Family | Bodies | Reading |
|---|---:|---|
| `object -> concrete reference` | 3,121 | the value lost its precise reference type at a join; not an invalid program |
| `signature -> signature` | 2,813 | dominated by `T[] -> IEnumerable<T>/IList<T>` and `string[] -> object[]` |
| `integer -> native/pointer` | 2,799 | `I4 -> Native` 1,864, `I4 -> Pointer` 935 |
| `signature -> concrete reference` | 1,404 | constructor/argument shape with a constructed source |
| `integer -> value or reference` | 569 | `I4` where a value type is expected |
| `integer or native -> pointer` | 403 | `UNative -> Pointer` 314, `U4/Native -> Pointer` 89 |
| `pointer -> pointer` | 300 | pointee identity |
| `local class -> external class` | 218 | ancestry through a substituted reference |
| `managed address -> reference` | 192 | address where an object is expected |
| `R4 -> F4` | 134 | float stack-category spelling |
| other | 327 | |

`signature -> signature` details: 972 + 377 + 57 + 38 + 24 + 21 + 17 + 17 + 17 +
17 + 16 records are array-to-interface or array-covariance shapes
(`TreeDumperNode[] -> IEnumerable<TreeDumperNode>`, `string[] -> object[]`,
`string[] -> IEnumerable<string>`, `!0[] -> IEnumerable<!0>`), so a single
array-assignability rule (array covariance plus the generic collection interfaces
an array implements) covers about 1,570 of them. No pair in the sample has two
identical type texts, so this is not a normalization defect.

`pinned/byref/modified local` is exact: 1,666 bodies, every one with at least one
pinned local (2,385 pinned locals total) and only ~120 modified-local records.
One `fixed` statement therefore degrades an entire method; this is a single
capability, not 1,666 problems.

## 3. Grouped plan

Order is by leverage per unit of semantics risk, not by bucket size. Each group
is one commit with red/green owned tests, negatives, and its own evidence.

### Group C - one prefetch pass for every operand shape (started)

Today each external fact class has its own ad-hoc priming guard
(`external_method_instantiation` only for `TypeRef` parents, `external_allocation`
only for some constructors, casts only for some TypeSpecs). The result is whole
families that silently miss facts and report "needs dependency facts".

Replace the per-class guards with one traversal of the body's operand types and
member references that asks for every needed fact class once, under one work
bound, and records why a fact stayed unavailable. Expected to absorb the
value-generic slice (2,269, already implemented for the TypeSpec case), the
allocation bucket (1,325) and several cast buckets.

Code: `rust/src/lib.rs` (`prime_external_*`), `rust/src/dependencies.rs`
(`external_allocation`, `external_method_instantiation`, `external_cast_target`).
Constraint: an unavailable reference must still refuse, never guess.

### Group B - one declarative conversion matrix

`conv`, `conv.ovf`, and the `.un` variants are ~7,785 first blockers, almost all
inside `ExplicitConversions` and `ExplicitConversions_32` (9,472 unrecovered
between them). Implement one table: source category x destination category x
checked/unchecked x signed/unsigned interpretation, with one lowering into the
typed IR and one generated test matrix, instead of per-opcode patches. This also
removes the `I4 -> value` spelling problems that share the category logic.

Code: `rust/src/lift.rs` (conversions), `rust/src/ir.rs` (category helpers).
Constraints: overflow must raise `OverflowException` exactly where IL does;
`conv.r.un` keeps its existing unsigned-read semantics; Boolean stores still
normalize.

### Group A - value precision at joins and pointer/native categories

- Stop degrading a proven reference to `Object` at stack joins (3,121 bodies
  plus 1,404 signature-shaped ones). Reuse `join_reference_type` and the foreign
  identity work; do not invent casts where the program is genuinely unverifiable.
- Keep native/pointer categories on the stack instead of collapsing to `I4`
  (3,503 bodies) and separate pointee identity for pointers (300).

### Group A2 - array assignability

Array covariance (`ref[] -> ref2[]` when element types are related), the generic
collection interfaces an array implements, and `T[] -> Array`/`object`. Value-type
element arrays remain invariant; `object[] -> string[]` stays refused. This is the
cheap part of the largest bucket (~1,570) and needs only array identity facts we
already hold.

### Group D - `fixed`/pinned locals

One scope-level lowering: a pinned local becomes a byref local acquired inside a
pin scope, with escape/lifetime negatives. Covers 1,666 plus part of the 563
struct-field/address bucket.

### Group E - modeled features, each with its own audit

- GVM 1,999: runtime lookup-order modeling. A counterexample already exists, so no
  static assumption is acceptable.
- EH fault 640 and filters 444: independent region/control-flow/exception-search
  audits; fault must not be folded into `finally`.
- Definite assignment 813: handler-state precision (442 concentrated in
  `System.Private.CoreLib`).
- Constructor overload selection 353, method implementation flags 317, constructed
  base field names 302, `unbox` 297: small, independent.

### Group F - corpus-data limits

`invalid user-string encoding` 2,739 is a data property (the terminal flag
disagrees with the code units). Do not invent bytes. Audit what ILSpy prints for
the same blob; if a byte-preserving escaped literal is defensible, implement it as
a literal-only lowering with a negative test, otherwise keep it as a documented
limitation.

## 4. Reproduction on another machine

Prerequisites: Rust toolchain matching `rust-toolchain`/CI, a .NET 10 SDK at
`~/.dotnet10` (or `DOTNET_ROOT` pointing at it), and the `ILSpy-tests` submodule.

```sh
git submodule update --init ILSpy-tests      # pinned to 9668d49b..., never move it
export DOTNET_ROOT="$HOME/.dotnet10"         # required, see pitfall 1
cd rust
cargo build --release --locked
```

The two OpenRA inputs are committed at `rust/report/openra/assemblies/`
(4.1 MB total, GPL-3.0 input-only; see `acceptance/openra/manifest.json` for
provenance and hashes). They are never executed by the decompiler.

Regenerate the corpus and the anchor (do not re-download anything):

```sh
cd ..
python3 rust/acceptance/corpus/corpus.py collect \
  --checkout ILSpy-tests --output rust/report/fallback-corpus/frozen \
  --supplement rust/report/openra/assemblies/OpenRA.Game.dll \
  --supplement rust/report/openra/assemblies/OpenRA.Mods.Common.dll
python3 rust/acceptance/corpus/corpus.py inventory \
  --manifest rust/report/fallback-corpus/frozen/manifest.json \
  --output rust/report/fallback-corpus/inventory-final
python3 rust/acceptance/corpus/corpus.py baseline \
  --inventory rust/report/fallback-corpus/inventory-final/inventory.json \
  --cli rust/target/release/ilspy-lite \
  --output rust/report/fallback-corpus/baseline --workers 4 --timeout 300
python3 rust/acceptance/corpus/corpus.py gate \
  --baseline rust/report/fallback-corpus/baseline/baseline.json
```

Compare a new scan against the committed anchor (per-token statuses):

```sh
# unzip the anchor next to a working directory, then
python3 rust/report/compare-recovery-scans.py \
  <anchor-baseline.json> <new-baseline.json> <delta.json>
```

Targeted scans of selected images (fast, reference policy identical to a full
scan) use `rust/report/targeted-scan.py --output <dir> <ids...>`.

### Pitfalls that have already cost time

1. **`DOTNET_ROOT` must be set** for any scan whose comparison depends on
   framework substitution. Without it the CLI reports
   `status=missing ... no matching supplied identity`, loses the substituted
   reference facts, and a scan of the Roslyn images loses ~12,000 bodies that look
   like regressions. They are not; discard and rerun with the variable set.
2. `rust/report/` is ignored. Evidence must be force-added (`git add -f`) and log
   files packed or stripped of trailing whitespace before committing, otherwise
   the pre-commit hook rejects them.
3. `ILSpy-tests` is 258 MB and must stay at the pinned commit; never follow
   `master`.
4. The last complete scan is the only source of aggregate numbers. Targeted scans
   prove local gains and regressions only; do not sum them into a rate.
5. `--threads` does not change results; a status difference after changing it
   means an environment difference, not flakiness.

## 5. Audit tooling that is not committed

The sub-cause tables above came from a temporary instrumented build: an
environment-gated `audit::record` call at `metadata::assignable`'s failure site and
at the local-signature gate, plus a report at the final `Partial` emission in
`lib.rs`. It is deliberately not part of the tree because it changes the
production binary. To reproduce: add a `thread_local` recorder, call it at those
two sites, print the last record when the emitted error starts with the same
message, gate everything on `ILSPY_LITE_AUDIT`, and run
`rust/report/residual-audit/run-audit.py --reasons <reason>`. Aggregates are in
`reasons.json` and `assignable-subcauses.json`; raw per-image lines are in
`residual-audit-logs.zip`.

## 6. Honest state of each group

| Group | Code | Tests | Measured corpus effect |
|---|---|---|---|
| Constructed value generic prefetch | committed | red/green + 720 behaviour comparisons + 6 mutants | **unmeasured** (two interrupted targeted scans) |
| C | not started | - | - |
| B | not started | - | - |
| A / A2 | not started | - | - |
| D | not started | - | - |
| E | not started | - | - |
| F | audit only | - | - |

No group claims a corpus gain without a completed scan; the aggregate remains
42,044 until the next complete scan says otherwise.
