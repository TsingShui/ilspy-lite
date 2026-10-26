# Small Rust decompiler: scope after infrastructure experiments

## Recommendation

Use **cildec 0.1.0 + a small method-local IR + a C#-like writer** for the first
independent Rust CLI. Keep ILSpy and its .NET 10 fixtures as reference/test tools,
not runtime dependencies. Do not wait for a second complete C# decompiler to be
extracted before starting the Rust vertical slice.

This revises the earlier mask-27 recommendation: mask 27 is a useful ILSpy output
baseline, not the architecture or pass list for a rewrite. Removing calls to
passes did not shrink its source dependency closure.

## Infrastructure comparison

Versions/licenses were checked against crates.io package metadata, not inferred
from README badges. Details and runnable smoke probes are in README.md.

| Library | Useful existing work | Decision |
|---|---|---|
| [cildec 0.1.0](https://docs.rs/cildec/0.1.0/cildec/) | PE/CLI, tables/heaps, signatures, token display, IL offsets/opcodes/prefixes, EH tables | First choice for a read-only front end; MIT; pin and test |
| [dotscope 0.9.1](https://docs.rs/dotscope/0.9.1/dotscope/) | Above plus CFG, SSA, loops and def/use analysis | Not the default for the small first version: observed load/verifier blockers; Apache-2.0 |
| [dotnetdll 0.3.0](https://docs.rs/dotnetdll/0.3.0/dotnetdll/) | High-level metadata model, signatures and resolved IL | Not a default permissive-license dependency: published license is GPL-3.0+; no local smoke test |
| [dnfile 0.5.1](https://docs.rs/dnfile/0.5.1/dnfile/) | Metadata and IL inspection | Possible alternative decoder, not a demonstrated CFG/stack-lifting replacement; no local smoke test |
| [clrmeta 0.1.0](https://docs.rs/clrmeta/0.1.0/clrmeta/) | Metadata/signature read/write | More assembly/IL integration needed for this goal; no local smoke test |
| [backtrip 0.1.4](https://docs.rs/backtrip/0.1.4/backtrip/) | Existing compact CLI with C# output | Do not use as semantic base; local output loses checked and old-value semantics |

cildec explicitly does not build CFGs, simulate the stack, verify operand types,
or resolve across assemblies. Decoding 39,317 CoreLib bodies is **not** decompiling
or proving them correct. It is a new 0.1 library; the available coverage is a smoke
gate, not enough to establish long-term maintenance or hostile-input robustness.

### Why not take the ready-made SSA immediately?

With default features disabled, dotscope still carries a substantial framework.
The published source archives contain approximately:

- cildec: 11,470 lines under src, 17 Rust files;
- dotscope: 492,245 lines under src, 968 Rust files;
- analyssa 0.6.0: 101,746 lines under src, 114 Rust files;
- backtrip: 12,249 lines under src, 28 Rust files (including its JVM backend).

These are physical source counts including documentation and inline tests, not
compiled code size or maintenance effort we necessarily inherit. The probe's
normal dependency closure has 103 packages excluding the probe itself, with
cildec and dotscope included and dotscope's default features off. A stripped
probe was about 3.7 MB, which likewise is not an estimate for the final product.
A library can be large while the consumer is small; source size alone is not why
it was rejected for the first version.

The practical blockers are the observed two fixture SSA validation failures,
CoreLib strict-load failure, and lenient diagnostics plus further verifier
failures. Adopting it immediately risks spending the first milestone repairing
an external SSA/type-system framework instead of delivering a bounded CLI.
Reconsider after those findings are triaged if broad SSA-based analysis becomes
a requirement. Do not silently enable lenient mode or turn verifier failures
into successful decompilation.

## What not to port from ILSpy

| ILSpy subsystem | First Rust version |
|---|---|
| Metadata/PE reader and IL byte decoder | Reuse cildec; do not translate ILSpy's implementation |
| Full TypeSystem, Semantics and CSharpResolver | Keep signature types, token identity, stack categories, explicit conversions; no full C# binding engine |
| C# AST, builders, visitors, AST transforms | None; use a small expression/statement model and precedence-aware writer |
| General ILAst node/visitor/transform framework | None; method-local IDs and small enums |
| Async/iterator/closure/record/query recovery | None; keep generated methods, fields and state machines |
| Whole-program SSA, alias analysis, optimizer scheduler | Not required for initial labeled-block output |
| Project/solution export, PDB generation, XML docs, GUI | Out of scope |
| Automatic dependency discovery, GAC/runtime scanning | Out of first version; preserve assembly-qualified external references |
| Metadata/assembly writing, deobfuscation, emulation | Out of scope |
| Reliable instruction effects, stack merging, EH control, integer semantics | Must implement or explicitly report unsupported; never silently simplify away |

The source reduction comes from a smaller implementation contract, not from
copying the remaining directories into Rust. Existing ILSpy directories remain
untouched as the reference; the new runtime source set should have no dependency
on ICSharpCode.Decompiler.

## Small pipeline, not 44 passes

1. **Decode and identify.** cildec provides typed rows/signatures, instructions,
   original byte offsets and raw EH clauses. Preserve tokens and code ranges.
2. **Build basic blocks.** Branch/switch targets, fallthrough and EH entry/end
   positions define leaders. Preserve normal edges and EH region descriptors;
   leave is not an ordinary goto across a finally.
3. **Lift the evaluation stack.** Worklist over blocks checks stack depths and
   categories. Each instruction that produces a value gets a stable temporary ID;
   loads of mutable locals snapshot their current value. Block-entry stack slots
   are explicit parameters, supplied by edge-local parallel copies. Account for
   catch/filter entry stacks. Keep original local slots mutable; do not require
   whole-function SSA for all locals just to print useful output.
4. **Conservative expression formation.** Initially emit temporaries. Later inline
   only proven-safe single-use expressions inside one block and EH region. Never
   duplicate calls, lose popped effects, delay mutable reads or reorder exceptions.
5. **Write C#-like text.** Labels/gotos are valid first-version output. Calls retain
   exact tokens/signatures; special operations use documented intrinsics. Print
   every requested method, raw IL on failure, and explicit coverage status.

Optional later stage: recognize simple if/while/switch regions, falling back to
labels. Use an existing graph crate if dominators become necessary; do not build
a generalized graph framework for the first labeled-block release.

This is five coarse stages, **not five trivially implemented ILSpy passes**. The
hardest custom stage remains stack lifting and cross-block/EH semantics.

## Interface and minimum deliverable

Proposed CLI:

```text
ilspy-lite list assembly.dll
ilspy-lite il assembly.dll --method 0x06000001
ilspy-lite decompile assembly.dll --method 0x06000001
ilspy-lite decompile assembly.dll --type Namespace.Type
```

One binary, no .NET runtime required in production, read-only operation. Start
with numeric method tokens so overloads and generated identifiers are unambiguous.
Type selection is a loop over metadata-owned methods, not a second analysis mode.
Add options only as needed; std argument parsing and text output suffice initially.

Each method result must distinguish:

- **decompiled**: supported lowering/rendering with integrity checks passed;
- **partial**: explicit low-level constructs/raw-IL fallback, never counted as
  complete success;
- **failed**: malformed or unsupported input preventing a trustworthy result.

No-body methods (abstract, extern, runtime-supplied) are declarations, not missing
methods. Track inventory independently so a parser/renderer cannot improve its
coverage merely by dropping difficult methods.

### First acceptance gate

- Fixture assemblies: list every method token/name and dump every body with
  offsets, signatures and complete EH metadata.
- Compile/decompile/recompile the supported ControlFlowSample subset; verify
  loops/branches, switch, checked overflow, unsigned shifts, evaluation order,
  old local values and observable side effects. Test joins and dup explicitly.
- Preserve constructors, static initialization, call/callvirt and token identity.
- For unsupported EH/ref/prefix patterns, output clearly marked IL fallback;
  do not claim the entire fixture suite is supported until those semantics pass.
- Process generated MoveNext methods as ordinary IL. High-level async syntax
  reconstruction is not required for accepting those methods.
- No speed or size success criterion can override output/inventory correctness.

### Hard stops against scope growth

- No C# language-version matrix or syntax-sugar work.
- No complete formatter/AST framework.
- No eager cross-assembly type loading merely to display a call signature.
- No source-level method overload inference: the IL target token is authoritative.
- No full optimizer before a method can be emitted using stable temporaries.
- No second full C# implementation as a prerequisite to Rust delivery.

## Compact upstream reading list

These are algorithm references, **not a self-contained transplant**:

| File | Physical lines | Use |
|---|---:|---|
| ICSharpCode.Decompiler/IL/ILReader.cs | 2,248 | Stack import, joins, effects, EH considerations; byte decode is outsourced |
| ICSharpCode.Decompiler/IL/ControlFlow/ControlFlowGraph.cs | 203 | CFG conventions; its container graph deliberately omits exceptional edges |
| ICSharpCode.Decompiler/IL/InstructionFlags.cs | 82 | Effect/throw/order distinctions |
| ICSharpCode.Decompiler/IL/Transforms/ILInlining.cs | 1,008 | Safety rules; not all heuristics belong in the first version |
| ICSharpCode.Decompiler/IL/ControlFlow/LoopDetection.cs | 816 | Later structural readability work |
| ICSharpCode.Decompiler/IL/ControlFlow/ConditionDetection.cs | 681 | Later structural readability work |

Total initial reading list: **5,038 lines**, versus 174,528 engine source lines.
This is not a claim that 5,038 lines implement the full decompiler. The normative
source for semantics is [ECMA-335](https://ecma-international.org/publications-and-standards/standards/ecma-335/),
especially Partitions I/II/III; use ILSpy tests/output as differential evidence,
not as a substitute for the standard.

## Size budget, not a promise

Target roughly **3-6 KLOC of self-owned production code** for a bounded first
version: decoder adapter, method-local IR/stack lifting, labeled output and CLI.
Exclude third-party source, generated opcode tables, fixtures and tests, and
report them separately. This is an engineering budget to constrain features,
not a measured implementation estimate or an assurance of full EH/generic/ref
coverage. Reassess after the first end-to-end method gate rather than inflating
the scope to match ILSpy.

Actual work completed here: infrastructure research, source inspection, pinned
probe and smoke results. No production Rust decompiler or physical ILSpy source
cut is claimed. No commit or push was made.
