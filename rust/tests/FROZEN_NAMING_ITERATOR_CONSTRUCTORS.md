# Frozen naming: argument constructor audit closure

This optional offline gate covers the two previously unexecuted argument
constructors, not iterator reconstruction or new production semantics:

```sh
python3 rust/tests/frozen-naming-iterator-constructors.py \
  --corpus "$PWD/rust/report/openra"
```

It requires only the existing read-only corpus and installed .NET 10 SDK/runtime
and reference pack. Cargo builds the current CLI with `--locked --offline`.
No original Game/Common assembly is a recovered compiler reference.

## Selected bodies and source correspondence

- OpenRA.Game.dll `06001477`, frozen iterator owner `020003BB`.
- OpenRA.Mods.Common.dll `0600276F`, frozen iterator owner `02000858`.

Independent static PEReader cataloging checks the pinned MVID, exact owner,
instance-void-I4 signature and constructor flags, raw IL, two own I4 fields,
Object ancestry, lack of owner/module initialization and exception regions.
The original protocol is exact core Object initialization, state store, exact
core Environment.CurrentManagedThreadId getter, thread field store, return.
Both original references must use the pinned standard System.Runtime 6.0 identity.
Loaded original IL and the runtime core getter target are checked again.

The current default `--type` output is saved exactly. An independent root path,
unique one-I4 constructor shape and method-local span bind the actual constructor
member. Its declaration, base initializer and statements supply the test shell.
Unused fields/interfaces/outer classes are omitted, and the two observed fields
are public test I4 fields; this is not the full class ABI.

One explicit test-only syntax adapter changes the actual expression
`System.Environment.get_CurrentManagedThreadId()` to the C# property spelling
`global::System.Environment.CurrentManagedThreadId`. C# forbids explicit accessor
method syntax. Exactly one occurrence is replaced and both actual/adapted members
are recorded. The compiled-IL guard proves the adapter invokes that exact runtime
core getter once, with no original selected body or proxy. The verified effect
order on both sides is Object call, state write, thread getter, thread write.
No Details statements supply implementation logic. No production presentation or
static-property support is added by this adapter.

## Executed cases and controls

Each constructor passes **140 original/recovered pairs**, including **14 null
receiver pairs**, on two distinct execution threads. Across both: **280 pairs,
28 null pairs and 4 compiled runtime mutation rejections**.

Per execution thread: three independently preset dirty values x seven boundary
arguments x three repeated calls = 63 nonnull pairs, plus seven null pairs.
The repeated argument is complemented for the middle call. State and captured
thread ID must independently equal the argument and current execution thread,
not the other side's poststate. Other safely observable original fields retain
their own pre-call primitive/reference snapshots. Each side allocates its own
receivers without original factories or copied initialized state. Constructor
calls use exact IL `call`, not new allocation or an added null check. Null this
must fault at the first field write, before the getter's place in the verified
compiled effect order.

For each constructor, separately compiled controls replace the actual state
store with zero, and the thread store with an unchecked self-subtraction yielding
zero while retaining the actual getter evaluation. Both must pass the same
call/field guards and then fail specifically with `state-write` or `thread-write`,
status `behavior-mismatch`, exact token and exit 42. Compiler/guard failures do
not satisfy a control.

Ignored `rust/report/frozen-naming-iterator-constructors/` retains actual output,
independent catalogs/raw IL, exact source spans and adapter record, positive and
mutant sources, framework-only compiler references, all commands/exit codes,
runtime logs and `summary.json`. Failed runs remove the success summary.

Limits: selected constructor fragments and two observed I4 fields, not iterator
protocol, complete field/class ABI, interfaces, allocation, callbacks/gameplay,
other runtime/platform behavior or arbitrary environmental getter failures.
