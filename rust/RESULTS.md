# First Rust slice: measured results (historical)

This records the integer-only slice. For the current reference/instance-method
implementation, test counts and source size, see
[OpenRA stage 1 results](acceptance/openra/STAGE1.md).

Environment: macOS arm64, Rust/Cargo 1.98.0, .NET SDK 10.0.401,
Microsoft.NETCore.App 10.0.12. Results concern this bounded implementation and
these inputs, not general .NET compatibility.

## Gates run

- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo test --locked`: 16 tests passed, none ignored.
- `cargo test --release --locked`: 16 tests passed, none ignored.
- `python3 tests/roundtrip.py`: 735 differential cases for each of the Release
  and Debug fixture sets, **1,470 total**, all passed.
- `git diff --check`: passed.

The first inventory, IL/checked-add, CFG/call, CLI, method-flag, division,
raw-failure and nested-state-machine checks were observed failing before their
implementations. Additional negative probes exercise stack underflow, excess
return values, unequal join depths, invalid branch boundaries, invalid argument
indices, maxstack violations, unsupported prefixes, `leave`, native/synchronized
methods, missing body RVAs and undecodable opcodes.

The behavioral test recompiles nine supported `ControlFlowSample` methods and
25 integer methods per configuration, wraps them with explicitly supplied type
and field declarations, then compares execution with the original fixture.
Coverage includes integer boundaries, checked overflow, signed/unsigned division
and remainder, 32/64-bit arithmetic, Boolean joins, signed/unsigned shifts,
nonempty stack merges, mutable snapshots, duplicated values, calls whose results
are popped, and external `Math.Abs` calls. It compares return values and exception
types, not traces, concurrent behaviors, timing or every possible input. The
backedge parallel-copy test inspects output; its infinite-loop input is not run.

## Inventory versus lowering coverage

All methods are visible independently of successful lowering:

| Input | MethodDef count | Decompiled | Declaration | Partial | Failed |
|---|---:|---:|---:|---:|---:|
| LiteExperiment Release | 59 | 9 | 0 | 50 | 0 |
| LiteExperiment Debug | 61 | 9 | 0 | 52 | 0 |
| Integer Release | 25 | 25 | 0 | 0 | 0 |
| Integer Debug | 25 | 25 | 0 | 0 | 0 |
| .NET 10.0.12 CoreLib, System.Math | 133 | 20 | 25 | 88 | 0 |

`System.Math` is a CLI smoke check, **not a behavioral roundtrip**. Its type
command correctly exits 2, not 0. Full CoreLib inventory lists 41,420 MethodDefs;
this Rust implementation has not behaviorally decompiled all of CoreLib.
The earlier decoder-only CoreLib experiment is separate evidence and must not be
reported as this decompiler's success coverage.

Most general-purpose fixture methods still fall back. In particular, async,
iterator and closure implementation methods are retained, not reconstructed.
`--type LiteSamples.AsyncSample` includes its nested `MoveNext` and
`SetStateMachine` methods as explicit fallbacks.

## Actual size, separated by ownership

Physical lines, including headers/blank lines, after rustfmt:

| Category | Files | Lines / size |
|---|---:|---:|
| Self-owned Rust production source, src/*.rs | 6 | 1,173 lines |
| Rust integration tests, tests/*.rs | 3 | 301 lines |
| Python fixture/behavioral tooling | 2 | 101 lines |
| C# fixture/behavioral tooling | 2 | 143 lines |
| Pinned generated binary fixture data | 4 | 36,352 bytes |
| Generated Rust production source | 0 | 0 lines |
| Third-party cildec published Rust source | 17 | 11,470 lines |

The third-party count includes its documentation and inline tests; it is not
self-owned implementation. Cargo's normal dependency closure is exactly one
crate. The optional C# test runtime/compiler and existing ILSpy experiment are
not linked into the Rust executable and are not included in production source
counts. Cargo.lock, documentation and output examples are separate from the
source counts above.

The local stripped release executable is **610,704 bytes** (about 596 KiB).
This is one macOS arm64 build, not a portable binary-size guarantee, memory
measurement or throughput benchmark. Only Rust 1.98.0/macOS arm64 was tested;
the declared Rust 1.85 minimum has not been separately validated.

## Not completed or claimed

- Full object/reference/array/generic/value-type/byref support.
- EH/leave lowering, conversions, floating point and native integers.
- C# binding/accessibility, project export or recompilation of arbitrary types.
- Structured loop recovery or dominator integration.
- Full ECMA-335 verification or hostile-input/resource-hardening guarantees.
- Upstream ILSpy's .NET 11 test suite or the large ILSpy-tests submodule.
- A universal correctness/performance claim derived from these finite cases.

ILSpy's engine sources and root SDK selection are unchanged. The implementation
and reports remain local worktree changes; no commit or push was made.
