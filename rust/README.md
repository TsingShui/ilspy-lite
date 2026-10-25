# ilspy-lite (Rust)

A small, independent CIL-to-C#-like decompiler. This is a working **first vertical
slice**, not a complete .NET decompiler or a port of ILSpy's pass pipeline.
It does not link to ILSpy, invoke a .NET process, load dependencies from the GAC,
or execute input assemblies. The only Cargo dependency is pinned `cildec 0.1.0`.

## Build and use

```sh
cd rust
cargo build --release --locked
./target/release/ilspy-lite list tests/fixtures/release.bin
./target/release/ilspy-lite il tests/fixtures/release.bin --method 0x06000019
./target/release/ilspy-lite decompile tests/fixtures/release.bin --method 0x06000019
./target/release/ilspy-lite decompile tests/fixtures/release.bin --type LiteSamples.ControlFlowSample
```

The last example deliberately exits **2**: nine methods decompile, but `Divide`
uses exception handling and retains an IL fallback. Tokens are assembly-specific;
use `list` to find them. Decimal tokens are also accepted.

`--type` selects the exact qualified type name and all of its nested types.
Nested names use `/`, as shown by `list`. Compiler-generated methods are never
hidden; an async type selection includes its nested state machine, not just the
kickoff method. Type output is a **method collection**, not a compilable class or
project: fields, properties, attributes, access modifiers, and type declarations
are not reconstructed.

Output goes to stdout; decompilation counts/errors go to stderr. Exit codes:

| Code | Meaning |
|---|---|
| 0 | Every selected method rendered, or was a legitimate bodyless declaration |
| 2 | At least one method has an explicitly marked IL fallback; none failed |
| 1 | Invalid request/input, decode failure, or output I/O failure |

Library entry points in `src/lib.rs` accept assembly bytes and method tokens.
`Status::{Decompiled, Declaration, Partial, Failed}` separates successful lowering
from unsupported semantics and decoding failures. Errors that prevent identifying
or parsing the method are returned as `Err`. Parsed bodies with undecodable IL
retain raw code bytes in a `Failed` result.

## Supported lowering

- Static methods with integer/Boolean signatures and locals; I4/I8 evaluation
  stack categories, including unsigned reinterpretation at operation boundaries.
- Local reference-class instance methods with an explicit receiver, `ldfld` /
  `stfld`, and direct instance `call` through MethodDef.
- Local class references in parameters/locals/returns, `ldnull`, same-type
  reference joins, and locally provable base-class assignment.
- Constant/argument/local loads and stores, `dup`, `pop`, static field loads/stores.
- Arithmetic, checked arithmetic (signed and unsigned), division/remainder,
  comparisons, bitwise operations and shifts.
- Branches, conditional branches, switch, loops and nonempty stack joins.
- Direct static `call` through MethodDef/MemberRef with supported signatures.
- `ret`, including void; local initialization only when `initlocals` is present.

All output is explicitly **C#-like reverse-engineering output**. It uses stable
temporaries, original IL offsets, labels and `goto`; it does not recover
`while`, `foreach`, `async`, iterators, lambdas or LINQ. Call and field tokens are
retained. Identity casts on `int`/`long` values and exact-type field receivers
are omitted. Unsigned reinterpretation, narrowing, Boolean conversions, overflow
contexts and base-field receiver casts remain explicit; no expressions are moved
or inlined for this simplification.

The original assembly's metadata remains authoritative: source-level
binding, accessibility, attributes, type initialization context and assembly
resolution are not reconstructed by the writer. A method reported `Decompiled`
is lowered within this restricted model, **not certified equivalent/recompilable
for every CLI input**.

### Explicit instance semantics

Instance methods render as static method bodies with an explicit receiver `a0`.
This is a lowering convention, not a claim that the original method was static.
Receiver loads snapshot a reference; field reads/writes occur at their original
IL position. No entry null check is inserted. Generated/invalid field names use
`__il_field_TOKEN` aliases with the original token/name in a comment.

Direct instance calls render as `__cil_direct_TOKEN(receiver, ...)`, explicitly
meaning an exact IL `call`: no virtual dispatch and no added receiver null check.
These are **semantic intrinsics**, not a shipped C# runtime library. The test
harness maps them only to recovered method bodies; it fails if a call target is
missing. Pasting this output into C# without suitable declarations/adapters is
not sufficient to compile it. `callvirt` is still a fallback.

Reference identities are metadata tokens scoped to the one input image, not
strings compared across assemblies. Local ancestor checks stop at external
bases; no external inheritance compatibility is assumed. Interfaces, value
types, generic reference types and constructors remain unsupported. Reference
comparisons/conditional tests and mixed-reference joins are not yet lowered.

Unsupported signatures/instructions/regions cause **whole-method fallback**,
not a half-rewritten function with missing operations. Fallback includes code
bytes, decoded offsets/opcodes/tokens, locals, method/header flags and EH clauses.
`ldstr` values in IL output use exact UTF-16 code units (not lossy replacement).
If a malformed header prevents reading the body, the CLI reports the token and
error instead of claiming to have recovered unavailable bytes.

Currently unsupported: general/external object types, strings, arrays, floating
point, native integers, conversions, constructors, external instance calls,
callvirt, indirect calls, generic method instantiation, managed references,
value types, prefixes, exception regions and
`leave`. Synchronized/unmanaged method implementation semantics, uninitialized
locals and unreachable blocks also fall back. Metadata pointer tables are
rejected. These are real coverage limits, not successful decompilations.

## Small architecture

| File | Responsibility |
|---|---|
| `metadata.rs` | Strict PE/CLI adapter, inventory, signatures, references, IL dump |
| `cfg.rs` | Instruction-boundary validation and normal-flow basic blocks |
| `ir.rs` | Small declared-type / evaluation-stack-category vocabulary |
| `lift.rs` | Worklist stack propagation, stable values and conservative writer |
| `lib.rs` | Public result/fallback contract |
| `main.rs` | CLI selection, output and exit accounting |

There is no general C# AST, SSA framework, pass manager or dependency resolver.
Every mutable load becomes a snapshot before later stores/calls. `dup` duplicates
a stable value, not an expression. A popped call result is still evaluated.
Every edge first snapshots all stack arguments, then assigns destination stack
slots: self-edges and joins cannot turn parallel copies into sequential swaps.

The CFG models normal edges **only because methods with EH are rejected before
CFG construction**. Do not extend this graph to EH by simply ignoring exceptional
edges. Dominators are not required for this labeled representation. Structural
recovery can be added later without replacing stack lifting.

## Verification

These commands do not need .NET:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --release --locked
```

Optional behavioral gate (Python 3 and .NET **10** SDK/runtime):

```sh
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/roundtrip.py
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py
```

It invokes the SDK's Roslyn compiler directly, without restore, modifying root
`global.json`, or requiring .NET 11. It compiles CLI output from the **owned test
fixtures only**, then compares returns and exception types with the original
methods. It does not execute arbitrary user assemblies. This test is explicit,
not silently skipped by `cargo test`.

Fixture origins and regeneration: [tests/fixtures/README.md](tests/fixtures/README.md).
Measured results and boundaries: [RESULTS.md](RESULTS.md).

A pinned, external real-world corpus is described in
[acceptance/openra/README.md](acceptance/openra/README.md). Its first milestone
is four resource/ammo methods plus one supporting getter, not the whole game.
These five bodies now lower and pass a bounded behavioral gate; the other six
primary candidates remain IL fallbacks. The corpus is optional, downloaded
separately, and not part of ordinary Cargo tests. Its probe is static-only; the
separate `tests/reference-roundtrip.py --openra` gate executes only the selected
bodies. Details and limitations: [stage 1 results](acceptance/openra/STAGE1.md).
License/dependencies: [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Deliberate next steps

1. Add conversions and reference/array operations in tested vertical slices.
2. Add explicit managed-reference/value-type and callvirt semantics before
   widening their accepted signatures.
3. Design EH entry states and `leave`/finally handling; do not merely remove
   the fallback gate.
4. Add simple control-flow structuring using a verified dominator implementation.
   Irreducible graphs must remain valid labeled output.

Keep acceptance tests ahead of each coverage expansion. Do not replace an
unsupported operation with a comment and continue the method.

## Input safety

This is not a hardened hostile-input analyzer or a full CLI verifier. Strict
parsing, a 512 MiB image limit, 100,000-instruction lowering limit and 4,096-block
limit catch some bad inputs; they are not a sandbox or a complete resource bound.
Use OS memory/time limits when processing untrusted assemblies. Broad obfuscation,
malformed metadata and cross-platform compatibility have not been validated.
