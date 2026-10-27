# ilspy-lite (Rust)

A small, independent CIL-to-C#-like decompiler with exact named-type selection
and complete class-level metadata/member results. It is not a complete .NET
decompiler or a port of ILSpy's pass pipeline.
It does not link to ILSpy, invoke a .NET process, runtime-load assemblies or
execute input code. Dependency discovery reads local metadata, including installed
framework layouts when available. The decoder remains pinned `cildec 0.1.0`;
`sha1 0.10.7` supplies ECMA-335 identity tokens (not authentication), and
`serde_json 1.0.154` reads bounded local deps manifests. See [third-party notices](THIRD-PARTY-NOTICES.md).

## Corpus coverage

The fixed implementation corpus comprises the pinned ILSpy-tests scope and the
two pinned OpenRA assemblies: 614 managed images in total, 319 with CIL bodies,
and 770,748 method bodies. The latest complete scan recovers **728,704** of them
and leaves **42,044** as stated IL fallbacks, i.e. **5.4550%** against the 5%
weighted gate (38,537
allowed). Per-assembly counts, batch scans, the remaining first blockers and the
closure audit are in
[the fallback-corpus checkpoint](report/fallback-corpus/CHECKPOINT.md); owned
behavioral gates are in
[the behavior matrix](report/fallback-corpus/BEHAVIOR_GATES.md). Static coverage and
executed behavior are reported separately, and no third-party body is executed.

## Build and use

```sh
cd rust
cargo build --release --locked
./target/release/ilspy-lite list tests/fixtures/release.bin
./target/release/ilspy-lite il tests/fixtures/release.bin --method 0x06000019
./target/release/ilspy-lite decompile tests/fixtures/release.bin --method 0x06000019
./target/release/ilspy-lite decompile tests/fixtures/release.bin --type LiteSamples.ControlFlowSample
./target/release/ilspy-lite decompile tests/fixtures/release.bin --type LiteSamples.ControlFlowSample --details
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

### Readable by default, evidence on request

Default decompile output is a C#-like class/method view: namespaces, original
member modifiers and parameter names, `this`, constants and owned nested types.
Raw rows/signatures, per-instruction comments, repetitive banners and explicit
status records are not mixed into supported bodies. Safe immutable copies,
Boolean encodings, unused labels and bounded return-only branches are simplified;
field reads, addresses, allocations, exact calls and fault-producing operations
are not erased or freely inlined. Some temporaries and labeled loops remain.

Unsupported methods keep their declaration, an `IL fallback` reason and the
complete original IL as comments inside that method body, including locals and
exception regions. Failed bodies retain available raw/decodable evidence or the
original body-read error inline. No `--details` option is needed to inspect a
fallback body. Commented IL is not executable recovered C#. Properties/events have
named relationship summaries and visible accessor methods; automatic-property
declarations are not invented. With an explicit resolved context, proven direct
original-this property calls can use `base.Property` or nonvirtual `this.Property`.
Uncertain scope, dispatch, receiver or lookup keeps exact-call notation.
Exact-call and target-runtime intrinsics remain where ordinary C# would change
semantics. This is similar in presentation to ILSpy, not equivalent syntax
coverage, resolver behavior or a promise of compilable source.

Append `--details` to retain the complete analysis format described below,
including original tokens, raw metadata, interpretation limits and independent
statuses, in addition to the fallback body already visible by default. `list` and `il` keep their diagnostic formats. Both
views have identical status counts/exit precedence. Evidence-consuming scripts
must explicitly request `--details` rather than parse the readable view.

Library `decompile`/`decompile_type` retain their existing detailed contracts.
Choose presentation with `decompile_with_style`/`decompile_type_with_style` and
`OutputStyle::{Readable, Details}`. This avoids changing existing evidence callers
while giving the CLI a readable default. Validation:
[readable output acceptance](acceptance/READABLE_OUTPUT.md).

### Symbol names and generated bodies

Generated metadata names no longer block otherwise admitted bodies. A shared
naming rule renders declarations and references consistently, including nested
paths, keywords, reserved-prefix collisions and exact local TypeRefs. Original
scoped tokens/names remain authoritative and remain the CLI selection input;
aliases are not source-binding or runtime-identity proof.

The unchanged twenty-class corpus improves from 2 to 25 supported bodies out of
77; 52 still retain IL fallback. This does not add lambda/iterator desugaring,
generic/value/EH bodies or new type/call semantics. See
[symbol naming acceptance](acceptance/openra/SYMBOL_NAMING.md) for per-class and
per-method evidence and bounded execution cases for all 27 supported methods
across the twenty plus three separate supplemental owner trees.

### Local comparison website

Use the [local ILSpy / Lite viewer](tools/local-viewer/README.md) to browse test
assemblies and compare real ILSpy output with the current Rust CLI side by side.
It runs on loopback with uv-managed Python, uses existing local tools only, and
never executes selected assemblies. Readable, Details and original-method IL
views retain partial/failure diagnostics.

### Automatic static dependencies

The CLI discovers local dependencies by default for both decompile and scan.
It uses ILSpy-style short-name selection and follows loaded type forwarders:
neighboring images, deps/local NuGet assets, selected shared frameworks and
available Framework/GAC/Mono layouts are searched. No SDK/runtime installation
is required. `--reference <file>` and `--reference-dir <directory>` repeat;
`--no-auto-references` disables automatic lookup.

The default `--reference-version-policy ilspy` can select an identity different
from the requested reference. `exact` requires full identity; `same-major`
permits a limited nonolder version replacement. Requested/selected identities
and provenance are recorded, and a substitute is not runtime compatibility
proof. `--details` exposes dependency selections for decompile and scan.
Source property binding requires exact owner and signature scopes.

Missing/conflicting/rejected metadata never hides independently recovered bodies.
Large missing sets use a concise default notice; `--details` retains all scoped
requests, source/role records, selections and diagnostics. Reference input stubs
are explicitly labeled as API contracts. Reference contracts are not executable
implementations, and absence of a marker is not execution proof.

The existing byte-only APIs perform no filesystem search. `DependencyContext`
provides explicit immutable byte inputs, bounded filesystem discovery and scoped
type/method/field/property/base/interface/generic/forwarder metadata binding.
These are finite metadata capabilities, not a complete CLR or C# type system.
See [DEPENDENCIES.md](DEPENDENCIES.md) and
[dependency/presentation acceptance](acceptance/openra/DEPENDENCY_PRESENTATION.md).

### Detailed metadata contract

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
floating constants as bits). ClassLayout packing/size and raw rows, field offsets
and field/ordinary-parameter/return-parameter marshalling descriptors are retained
with blob indexes, raw bytes/rows and explicit runtime-owned interop limits.
Custom attributes retain their parent,
exact constructor identity/signature and raw payload without interpreting arguments
or executing constructors. Attached member/parameter/generic/relation records and
image-level Module/Assembly attributes are included. Declarative security retains
scope/action/permission-set bytes; enforcement remains runtime-owned, not recovered
body logic. Explicit MethodImpl mappings retain both endpoint identities/signatures
and raw rows without guessing name-based overrides or validating general dispatch.
Each uninterpreted record carries a local limitation, not only a global disclaimer.
RVA data is read only for a known primitive or fieldless explicit-layout extent,
otherwise its location and unresolved extent are shown without guessing.
Every method also has an original metadata signature/flags header, implementation
flags, parameter rows/defaults and an explicit lowering status. P/Invoke mappings
retain the exact entry name and module identity without loading the library.
If a method signature is unreadable but its body can be parsed, failed output
keeps raw body bytes and all independently decodable IL evidence.
Output is not a compilable class or project. Stable values, exact-call intrinsics
and runtime-dependent operations are defined in [SEMANTICS.md](SEMANTICS.md).
Acceptance results and remaining limits: [class delivery](acceptance/openra/DELIVERY.md).

Output goes to stdout; decompilation counts/errors go to stderr. Exit codes:

| Code | Meaning |
|---|---|
| 0 | Every selected method rendered, or was a legitimate bodyless declaration |
| 2 | At least one method has an explicit fallback notice; none failed |
| 1 | Invalid request/input, body/metadata failure, or output I/O failure |

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
- Signed integer `conv.r4` rounds I4/I8/native input to binary32 with nearest
  rounding at the conversion instruction. Nominal R4 evaluation/local values
  use explicit `__cil_F` semantics, whose internal precision belongs to the
  target runtime. Local/argument copies do not silently add storage rounding;
  field stores and call/return boundaries have explicit operations. The optional
  local .NET 10 adapter uses a Single carrier; it is not a cross-runtime CLI F
  implementation. `ldc.r4` preserves raw binary32 bits through
  `__cil_ldc_r4_bits(bits)`; `div` on two nominal R4 values uses `__cil_div_F`,
  retaining target CLI precision and IEEE special values without integer checks
  or an extra storage rounding. Float-to-integer, unsigned `conv.r.un`, R8 and
  other floating arithmetic remain unsupported. External floating calls admit
  only the exact core `Math.Abs(float32)` overload, preserving its token and
  argument/result boundaries through `__cil_abs_r4_TOKEN`.
- `object` reference parameters, locals, returns and fields, preserving identity,
  null and stable snapshots. Admitted string/class/int[]/uint[] references can
  upcast to object without boxing; implicit downcasts and value-to-object stores
  are rejected. Reference static fields use typed reference assignment, not
  integer conversion. This does not add boxing or value stack signatures,
  virtual dispatch or general allocation. See
  [object reference acceptance](acceptance/corpus/OBJECT_REFERENCES.md).
- Closed generic reference signatures such as List<int>, local Box<string> and
  Dictionary<string, List<int>>, interned with complete input-scoped type identity.
  Null/copies/field transfers/object upcasts and already-supported direct calls
  preserve reference identity. Value arguments do not enable value stack/layout
  operations, and generic members/receivers and variance remain unsupported.
  Both views retain canonical signature evidence for opaque aliases.
  See [closed generic reference acceptance](acceptance/corpus/CLOSED_GENERIC_REFERENCES.md).
- Closed vector reference copies, including byte[], string[], object[], struct/
  enum arrays and jagged arrays, without element operations, allocation or inferred
  covariance. Existing int[]/uint[] instruction support remains separate. See
  [array reference acceptance](acceptance/corpus/ARRAY_REFERENCES.md).
- Owner/position-scoped generic variables in reference signatures and as ordinary
  values. Generic value copies have their own evaluation category; no boxing,
  reference/numeric classification or concrete layout is guessed. Parameters,
  locals and returns preserve original value/reference transfer semantics. Generic
  member binding, receivers and addresses still require separate support. See
  [scoped references](acceptance/corpus/SCOPED_REFERENCE_VARIABLES.md) and
  [generic value copies](acceptance/corpus/GENERIC_VALUE_COPIES.md).
- Exact local enums with checked core ancestry and a unique fixed-width value__
  field. Loads preserve sign/zero extension; storage and return/call boundaries
  preserve truncation and original enum identity. Existing integer operations
  apply to normalized stack bits; external widths, boxing and value receivers
  are not inferred. See [enum acceptance](acceptance/corpus/ENUM_STORAGE.md).
- Whole snapshots/copies of verified local nongeneric structs, including class
  field and static-field transfers. Exact value identity and embedded references
  are preserved without boxing, construction or host-layout calculations. Enum
  normalization stays separate; ref structs, external value facts, managed
  addresses and value receivers remain gated. See
  [struct copy acceptance](acceptance/corpus/STRUCT_VALUE_COPIES.md).
- Struct instance methods whose receiver is a managed location of that exact
  value type, plus ldarga/ldloca/ldflda/initobj and integer field locations.
  Writes reach the caller's storage without passing a copy; addresses cannot
  escape as values or cross block edges. See
  [struct receiver acceptance](acceptance/corpus/STRUCT_RECEIVERS.md).
- `callvirt` with a locally verified class receiver, rendered as an explicit
  dispatch primitive that keeps the receiver null check and target-runtime
  dispatch. Opaque external, interface, constrained and value-type receivers stay
  gated. See [dispatch acceptance](acceptance/corpus/VIRTUAL_DISPATCH.md).
- Reference identity comparison: `ceq`/`beq`/`bne` over reference operands and
  `cgt.un` against null lower to an explicit `ReferenceEquals` form, never an
  overloaded operator. Ordered reference comparisons stay gated. See
  [equality acceptance](acceptance/corpus/REFERENCE_EQUALITY.md).
- `newobj` for a locally verified concrete class: one fresh identity, then the
  exact constructor in argument order. External/struct/abstract/generic/open
  constructors stay gated. See [allocation acceptance](acceptance/corpus/LOCAL_ALLOCATION.md).
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
  core-base calls with unchanged entry this are admitted. Same-type MethodDef
  delegation and direct local/external immediate-base initialization use
  `__cil_ctor_call_TOKEN(this, ...)` without allocation or repeated field
  initializers. Readable `: base(...)` / `: this(...)` is used only for a first
  exact call with safely movable, exactly typed arguments; earlier field/argument
  effects keep ordered intrinsic notation. Other receiver/base contexts and static
  initonly writes fall back; this is not a complete initialization verifier.
- Exact admitted core `Environment.TickCount` calls use `__cil_tick_count_TOKEN()`:
  a signed I4 clock read at the original instruction, evaluated once. Test-only
  controlled clocks preserve the full bit pattern; differing real times are not
  compared as if they were equal.
- Readable postfix recovery combines an adjacent signed I4 load and unchecked
  same-location write for owned unchanged-this fields or address-unexposed I4
  locals. Live old-value snapshots stay at their original point; an immediate
  sole old-value zero return can become `return unchecked(location-- == 0);`.
  Checked/unsigned, changed/effectful receivers and intervening operations are
  not forced into this form. Details and lowering statuses are unchanged.
- `ldstr` retains exact UTF-16 code units, including embedded nulls and unpaired
  surrogates, using ASCII C# literal escapes. `__cil_ldstr_TOKEN(literal)` means
  load according to the original module's literal/interning policy, not allocate
  a fresh string or assume the generated assembly has the same policy. String
  parameters, locals, fields, returns and same-type joins retain reference
  identity; string operations and mixed-reference joins remain unsupported.
  Invalid reachable user-string encoding retains raw bytes and cannot count as
  successful lowering.
- Bounded `newobj` for the exact admitted core
  `ArgumentOutOfRangeException::.ctor(string, string)` creates a fresh object at
  the instruction position, keeping `(paramName, message)` order. The output
  `__cil_newobj_TOKEN(...)` is one allocation/constructor operation, not exception
  throwing or an ordinary call. That exception type retains its image-scoped
  TypeRef in signatures/locals/returns; other external types and constructor
  overloads remain unsupported. `throw` terminates on that exception reference
  or null, empties the evaluation stack and retains the existing object and
  prior effects. Throwing null raises NullReferenceException; general object
  throw, rethrow and catch/finally remain unsupported.
- Local class references in parameters/locals/returns, `ldnull`, same-type
  reference joins, and locally provable base-class assignment. Nongeneric
  assembly-scoped external Class TypeRefs are opaque `__cil_type_TOKEN` aliases
  with original names/full scopes. Copies/null assignments preserve identity;
  dependencies are not loaded and external inheritance is not inferred.
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

The original assembly's metadata remains authoritative. Limited explicit-context
binding supports only the source proofs described above; arbitrary accessibility,
attribute/security effects, full dispatch and runtime assembly loading are not
reconstructed by the writer. Their available metadata records are evidence, not
permission to erase unknown effects. A method reported `Decompiled`
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

Single-image body references are scoped metadata tokens, not strings compared
across assemblies. The explicit metadata context separately binds actual scoped
definitions; that does not widen the body checks below. Local ancestor checks stop at external
bases; a local class can be assigned to the exact external TypeRef named in
its local ancestor chain, but no further external inheritance is inferred.
Direct reference-class MemberRef instance calls use `__cil_memberref_call_TOKEN`,
keeping owner scope/name/raw signature, exact dispatch and original runtime
resolution/access/body effects. Test adapters execute independent external leaves
by exact IL call; they never replace selected bodies. Interface dispatch,
concrete value receivers and constructed generic member calls remain unsupported. Reference brtrue/brfalse
tests use identity null patterns without dereferencing or overloaded equality.
Reference comparisons and mixed-reference joins are not yet lowered.

Unsupported signatures/instructions/regions cause **whole-method fallback**,
not a half-rewritten function with missing operations. Fallback includes code
bytes, decoded offsets/opcodes/tokens, locals, method/header flags and EH clauses.
`ldstr` values in IL output use exact UTF-16 code units (not lossy replacement).
If a malformed header prevents reading the body, the CLI reports the token and
error instead of claiming to have recovered unavailable bytes.

Currently unsupported: general external object operations, string operations,
additional array element operations, multidimensional array signatures/operations,
jagged element operations, native/I8 array lengths, other indirect access,
other floating arithmetic/R8/unsigned conversions, native integer arithmetic/comparisons, general constructor calls, constructed/value-type external instance calls,
callvirt, indirect calls, generic method instantiation, general managed references,
general concrete-struct operations, external enum storage without definition facts,
prefixes, exception regions and
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
| `readable.rs`, `readable/` | Bounded presentation without changing lowering admission |
| `dependencies.rs`, `dependencies/` | Immutable scoped metadata binding and explicit search |
| `discovery_paths.rs` | Bounded filesystem-only candidate enumeration |
| `signature_guard.rs` | Checked signature preflight for dependency binding |
| `lib.rs` | Public result/fallback contract |
| `main.rs` | CLI selection, output and exit accounting |

There is no general C# AST, SSA framework or pass manager. The finite static
metadata resolver is separate from body lowering; it is not a full CLR/C# type system.
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

### Fast iteration

The whole suite is one test binary (`[[test]] name = "all"`), plus
`discovery_paths` and `signature_guard`, which stay separate because they compile a
`src/` file into their own crate root. One target means a library change relinks
three binaries instead of one per test file: a warm `cargo test` run is about 2.5 s
instead of about 12 s, and a rebuild after touching `src/` is about 14 s instead of
about 24 s.

With [cargo-nextest](https://nexte.st) installed:

```sh
cargo nextest run                                  # everything, ~2 s warm
cargo nextest run --no-fail-fast                   # keep going after a failure
cargo nextest run -E 'test(casts::)'               # one suite (module name)
cargo nextest run -E 'test(scan_reports_every)'    # one test by name

# The corpus scan runs one process per image; --scan-threads adds workers inside
# each process, each with its own image parse and fact caches.
python3 acceptance/corpus/corpus.py baseline --inventory <inventory.json> \
    --cli target/release/ilspy-lite --output <dir> --references declared
```

Without it, the same selection works through the harness filter:

```sh
cargo test --test all -- casts::
```

The two internals targets keep their own names: `cargo test --test discovery_paths`.

Optional behavioral gate (Python 3 and .NET **10** SDK/runtime):

```sh
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/roundtrip.py
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --arrays
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --native --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --conversions --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --addresses --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --constructors --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --strings --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --exceptions --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --floats --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/dependency-oracle.py
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --properties --readable --from-class
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/reference-roundtrip.py --post-decrement --readable --from-class --checked-carrier
DOTNET_ROOT="$HOME/.dotnet10" python3 tests/frozen-constructors.py --corpus report/openra
```

It invokes the SDK's Roslyn compiler directly, without restore, modifying root
`global.json`, or requiring .NET 11. It compiles CLI output from the **owned test
fixtures** for ordinary gates, then compares returns/exceptions/state. Explicit
external gates additionally require the already acquired pinned OpenRA cache and
execute only the selected bounded bodies/leaves, never the game or arbitrary user
assemblies. This test is explicit,
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
original state. Default construction also passes its controlled-clock/delegation
gate. Both range overloads also pass their return/detailed-exception/full-state
gate, including original unchecked boundary behavior. NextFloat also passes
1,250 bitwise results per seed with full state and exact Abs(float) call checks.
The combined seven-method semantic milestone, frozen independent Queue/Tick
behavior gates, output-only reading and final regressions passed. These are
bounded contracts, not full-game or all-method support. See
[class delivery](acceptance/openra/DELIVERY.md) and
[semantic milestone](acceptance/openra/SEMANTIC_MILESTONE.md),
[Activity.Queue](acceptance/openra/ACTIVITY_QUEUE.md),
[Wait.Tick](acceptance/openra/WAIT_TICK.md) and
[random state](acceptance/openra/RANDOM_STATE.md),
[seed initialization](acceptance/openra/RANDOM_SEED.md) and
[default constructor](acceptance/openra/RANDOM_DEFAULT.md) and
[ranges](acceptance/openra/RANDOM_RANGES.md) and
[floating closure](acceptance/openra/RANDOM_FLOAT.md). The corpus is optional, downloaded
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
