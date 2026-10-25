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

`--type` selects the exact metadata TypeDef name and all of its owned nested
types, independently of method signatures or lowering support. Empty and generic
types can be selected; generic names include the metadata arity suffix (for
example ``Namespace.Type`1``). Nested names use `/`, as shown by `list`.
Compiler-generated methods are never hidden; an async type selection includes
its nested state machine, not just the kickoff method.

The library entry `decompile_type(bytes, name)` parses the image once and returns
type-token/name sections, raw type flags with named meanings, enclosing type,
base/interface identities, individual method results and a status summary.
TypeRefs retain their assembly/module scope; TypeSpecs retain raw signature bytes
and named-reference identities. No dependency assembly is loaded. One unsupported
or unreadable method does not suppress peers. Generic type/method parameters
retain their owner token, number, variance/special flags and exact constraints;
nested generic types keep their own parameter rows, including inherited type
parameter positions. These are metadata context, not generic-body lowering.
Event records retain their delegate type identity and add/remove/fire/other relationships, even
when the corresponding delegate body cannot be lowered. Property records
retain signatures (including index parameters), flags, defaults and exact
getter/setter/other MethodSemantics relationships. Accessor bodies stay visible as
methods; no automatic-property syntax is inferred.
All owned fields are listed with names/aliases, types, raw signatures and flags.
Constants preserve decoded values and original bytes (strings as UTF-16 units,
floating constants as bits). Field layout and marshalling descriptors are retained;
RVA data is read only for a known primitive or fieldless explicit-layout extent,
otherwise its location and unresolved extent are shown without guessing.
Every method also has an original metadata signature/flags header, implementation
flags, parameter rows/defaults and an explicit lowering status. P/Invoke mappings
retain the exact entry name and module identity without loading the library.
If a method signature is unreadable but its body can be parsed, failed output
keeps raw body bytes and all independently decodable IL evidence.
Output is not a compilable class or project.

Output goes to stdout; decompilation counts/errors go to stderr. Exit codes:

| Code | Meaning |
|---|---|
| 0 | Every selected method rendered, or was a legitimate bodyless declaration |
| 2 | At least one method has an explicitly marked IL fallback; none failed |
| 1 | Invalid request/input, decode failure, or output I/O failure |

Library entry points in `src/lib.rs` accept assembly bytes and method tokens or
an exact metadata type name. `DecompiledType.summary` counts each selected method;
its `exit_code()` uses the same status precedence as the CLI. A valid type with no
methods returns an empty method summary, not a missing-selector error. Metadata
failures use the separate `metadata_failed` count, remain visible beside recovered
methods, and force exit 1 without inflating the failed-method count. The CLI emits
a `metadata_failed=N` diagnostic line only when that count is nonzero.
`Status::{Decompiled, Declaration, Partial, Failed}` separates successful lowering
from unsupported semantics and decoding failures. Errors that prevent identifying
or parsing the method are returned as `Err`. Parsed bodies with undecodable IL
retain raw code bytes in a `Failed` result.

## Supported lowering

- Static methods with integer/Boolean signatures and locals; I4/I8 evaluation
  stack categories, including unsigned reinterpretation at operation boundaries.
- Native integers as a separate stack category: `nint` / `nuint` storage,
  parameters, returns, same-category joins and direct calls. Their width is the
  target runtime pointer width, not the width of the analyzer host. Native
  arithmetic, comparisons and implicit width changes still fall back.
- `conv.i4` from I4/I8/native values: retain low 32 bits without overflow checking.
  `conv.i8` sign-extends I4/native stack values and preserves I8 bit patterns.
  `conv.u8` zero-extends I4/native bits at their source width and preserves I8
  bits; native inputs pass through `nuint` before widening (32-bit and 64-bit
  targets must not be conflated). Local native execution is verified at 64 bits
  only. Conversion rules come from the opcode and source stack category, not parameter
  names or declared unsigned storage. Reference/floating sources and checked
  conversions remain unsupported.
- Local reference-class instance methods with an explicit receiver, `ldfld` /
  `stfld`, and direct instance `call` through MethodDef.
- Instance `.ctor` initialization bodies use `__cil_ctor_body_TOKEN`, distinct
  from ordinary methods and allocation. Initonly instance writes require the
  field's exact owning constructor and a direct, unchanged entry-this load;
  receiver aliases/joins are not inferred. This this-only restriction is a
  conservative subset, not an extra CLI requirement. A direct call to the exact
  admitted core `System.Object::.ctor()` retains its token and position as
  `__cil_core_object_ctor_TOKEN(this)`: initialization on the same receiver,
  without allocation, virtual dispatch or an added null check. Only immediate
  core-base calls with unchanged entry this are admitted. Other constructor
  calls and static initonly writes still fall back; body lowering is not a full
  initialization verifier.
- Exact admitted core `Environment.TickCount` calls use `__cil_tick_count_TOKEN()`:
  a signed I4 clock read at the original instruction, evaluated once. Test-only
  controlled clocks preserve the full bit pattern; differing real times are not
  compared as if they were equal.
- Local class references in parameters/locals/returns, `ldnull`, same-type
  reference joins, and locally provable base-class assignment.
- One-dimensional zero-based `int[]` / `uint[]` references in parameters,
  fields, locals and returns; `ldelem.i4` / `ldelem.u4` and `stelem.i4` with
  I4 indices. Array loads and stores stay at their original IL position.
- `newarr` for these arrays with an I4 length and an admitted core element
  TypeRef allocates a fresh zero-initialized array at the original instruction.
  Negative-length and runtime allocation failures stay there; native/I8 lengths
  and other array shapes remain unsupported. Resource exhaustion is a runtime
  possibility, not a deterministic memory-limit guarantee.
- `ldlen` for these arrays produces a native-sized bit pattern, read immediately
  with its original null check. Explicit `conv.i4` / `conv.i8` / `conv.u8` retain
  the original conversion chain; native length is not silently an I4 value.
- `ldelema` for these arrays with an I4 index and an exact admitted core
  `System.Int32` / `System.UInt32` TypeRef matching the array element. It binds a
  typed, stable `ref` immediately: null/bounds checks happen there, not at a
  later read. `dup` copies the address identity without reading or checking it
  again. `ldind.u4` reads this address at its original instruction, into a
  canonical I4 bit-pattern snapshot; signed I4 storage does not change its bits.
  `stind.i4` writes I4 bits back to that same location at the store instruction,
  including after alias writes, calls and array-field rebinding. Byref
  parameters/locals/returns, prefixes and address values crossing a basic-block
  edge remain unsupported.
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
not sufficient to compile it. The core constructor test adapter performs the
exact runtime constructor call, not a `new object()` substitution or a callback
to an original selected constructor body. `callvirt` is still a fallback.

Reference identities are metadata tokens scoped to the one input image, not
strings compared across assemblies. Local ancestor checks stop at external
bases; no external inheritance compatibility is assumed. Interfaces, value
types and generic reference types remain unsupported. Reference
comparisons/conditional tests and mixed-reference joins are not yet lowered.

Unsupported signatures/instructions/regions cause **whole-method fallback**,
not a half-rewritten function with missing operations. Fallback includes code
bytes, decoded offsets/opcodes/tokens, locals, method/header flags and EH clauses.
`ldstr` values in IL output use exact UTF-16 code units (not lossy replacement).
If a malformed header prevents reading the body, the CLI reports the token and
error instead of claiming to have recovered unavailable bytes.

Currently unsupported: general/external object types, strings, other array
element types, multidimensional/jagged arrays, native/I8 array lengths, other indirect access,
floating point, native integer arithmetic/comparisons, general constructor calls, external instance calls,
callvirt, indirect calls, generic method instantiation, general managed references,
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
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --arrays
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --native --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --conversions --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --addresses --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --constructors --from-class
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
These five bodies lower and pass a bounded behavioral gate. `MersenneTwister.Next`
and its `Generate` dependency also lower and pass the separate full-state gate:
1,250 consecutive calls per fixed seed, with each return and all state checked.
The separate seed gate executes the recovered allocation/initialization body,
compares all 624 words and follows it through continuous draws without copying
original state. Default constructor, range/exception and floating closure remain
pending; see [random state](acceptance/openra/RANDOM_STATE.md) and
[seed initialization](acceptance/openra/RANDOM_SEED.md). The corpus is optional, downloaded
separately, and not part of ordinary Cargo tests. Its probe is static-only; the
separate `tests/reference-roundtrip.py --openra --from-class` gate executes only
the selected bodies extracted from class output. This gate reuses verified local
inputs; `--verify-inputs` explicitly rechecks hashes when needed.
Details: [class milestone](acceptance/openra/CLASS_MILESTONE.md),
[stage 1 results](acceptance/openra/STAGE1.md) and
[first array step](acceptance/openra/ARRAY_STEP.md).
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
