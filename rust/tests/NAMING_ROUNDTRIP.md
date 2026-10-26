# Default naming class execution gate

Optional developer gate; not a production dependency or a Rust/static test prerequisite.
Run from the repository root:

```sh
python3 rust/tests/naming-roundtrip.py
python3 rust/tests/naming-roundtrip.py --mutate-store
```

Requires the installed .NET 10 SDK, reference pack, and runtime at `$DOTNET_ROOT`
(default `~/.dotnet10`), plus the offline Cargo cache. No restore, download, fixture
regeneration, original-assembly compiler reference, or existing adapter is used.
The runner builds the current CLI with `cargo build --release --locked --offline`.
It selects raw metadata name `NamingCases.Factory/<>c__DisplayClass0_0` in both
`naming-release.bin` and `naming-debug.bin`, using only default `decompile --type`
output, never detailed statements or the source fixture as an implementation.

## Declaration/reference compilation

The separate `python3 rust/tests/naming-declarations.py --mutate-collision`
gate compiles sixteen actual complete default class outputs: both fixture
configurations with ordinary names, an encoded generated name versus its literal
user spelling, a dot or literal nongeneric backtick inside one metadata type-name
record, UTF-8 names, a keyword
namespace, the combined generated/namespace case, and in-module nested TypeRefs
resolving to a nongeneric TypeDef with a numeric backtick in its literal name. The compiler references
only installed framework assemblies. This is syntax/type-reference evidence,
not runtime behavior. Both intentionally conflated alias mutants must fail
specifically with compiler diagnostic CS0102.

## Recovered source and independence

The complete CLI stdout, including constructor, method bodies, and diagnostic
comments, is inserted verbatim inside `namespace NamingCases` and the original
ordinary `public static class Factory`. No recovered declaration or body is
rewritten for the positive gate. Recovered compilation uses matching optimized
Release / nonoptimized Debug settings and only framework reference-pack DLLs.

The C# host discovers the test ABI by reflection and invokes each side through
separate `DynamicMethod` helpers with exact IL `call` (not `callvirt` or reflection
invocation). It never calls `Factory.Accumulator`. Each side allocates its own
receivers with `GetUninitializedObject` and assigns initial captured fields
independently. Original poststate and original method results never initialize
or implement the recovered side. Both sides must match independently computed
old-return and unchecked I4 state expectations.

Both constructors must have exactly `ldarg.0; call System.Object::.ctor(); ret`,
ignoring nops and allowing only the nonoptimized explicit-return `br.s 0` directly
before `ret`. The resolved target must be the runtime core `System.Object` instance
parameterless constructor. Both captured methods must have no method/function
pointer/signature operands and may reference only their own captured field.
The recovered assembly must contain only the parent and selected nested class,
and may reference only core framework assemblies, never the original fixture.
These checks reject an original-selected proxy rather than relying on output text.

## Exact cases per configuration

Boundary set for both initial values and deltas:
`int.MinValue`, `int.MinValue + 1`, `-1`, `0`, `1`, `int.MaxValue - 1`, `int.MaxValue`.

- Constructors: one fresh zero-initialized receiver pair plus seven independently
  preset receiver pairs, each called three times: **24 paired invocations**.
  Every call must preserve that receiver's initial field, including dirty states.
- Captured method: 7 initial values x 7 deltas x 3 successive calls on the same
  independent receiver pair: **147 paired invocations**. Each call must return the
  OLD field value and store the unchecked sum, including overflow boundaries.
- Null method receivers: one pair per delta, both requiring exactly
  `NullReferenceException`: **7 paired invocations**.

Total: **178 pairs per configuration; 356 positive pairs / 712 individual calls**
across Debug and Release. Constructor null receivers are intentionally not tested.

## Negative control and evidence

`--mutate-store` first requires both unchanged positive gates to pass. It then
changes only the actual recovered `this.value = ...;` statement to
`this.value = 0;` in a separate scratch source for each configuration. Both mutant
assemblies must compile successfully and fail execution with exit **42**, structured
`behavior-mismatch` status, and specifically `field-write` mismatch. Wrong failure
kinds, compile failures, IL/ABI validation failures, malformed reports, unexpected
exceptions, or passing mutants fail the runner. Expected mutation rejection makes
the paired runner succeed; the underlying mutant host executions remain failures.

Artifacts are under ignored `rust/report/naming-roundtrip/`: exact stdout and stderr,
wrapped positive and mutant source, compiler response files listing all references,
compiler/build logs, runtime logs, and `summary.json`. Summary is removed at startup
and written only after all requested checks pass. A pure store-removal mutation was
not used because the private nested field then triggers compiler warning CS0649
under warnings-as-errors; the constant-store mutation tests behavior, not compilation.

This gate covers only the owned Object-base closure constructor and captured I4
method. It does not establish general naming correctness, arbitrary declaration
contexts, static constructors, generics, value types, EH, new type/call support,
constructor null semantics, factories/delegate creation, or a broader corpus gate.
