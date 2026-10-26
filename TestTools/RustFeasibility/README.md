# Rust infrastructure feasibility probe

This is a bounded dependency experiment, **not a Rust decompiler**. It does not
execute target assemblies. It compares metadata/body inventories and runs the
SSA verifier of a candidate library. See [DECISION.md](DECISION.md) for the proposed
small-core design and observed limitations.

## Pinned versions

- cildec 0.1.0 (MIT), default features: std, no third-party normal dependencies.
- dotscope 0.9.1 (Apache-2.0), default features disabled; analyssa 0.6.0 transitively.
- Cargo.lock pins the complete probe graph. dotscope requires Rust 1.95; tested
  with rustc 1.98.0 / Cargo 1.98.0 on macOS arm64.

Both libraries are included to compare them, **not a proposal to ship both**.
The tool fails if any selected body is absent, sizes/handler counts differ, SSA
construction/validation fails, or dotscope emits diagnostics. Signature, local
signature, IL decode and EH range-validation errors also fail immediately.

## Reproduce

Build the existing .NET 10 fixtures following ../LiteExperiment/README.md, then:

```sh
cd TestTools/RustFeasibility
cargo build --release --locked
cargo fmt --check
cargo clippy --locked -- -D warnings

# All fixture bodies: current dotscope verifier findings make these exit 1.
./target/release/ilspy-rust-probe ../LiteExperiment/Fixtures/bin/Release/net10.0/Fixtures.dll
./target/release/ilspy-rust-probe ../LiteExperiment/Fixtures/bin/Debug/net10.0/Fixtures.dll

# Decoder-only path: exits 0 for the tested CoreLib.
PROBE_DECODE_ONLY=1 ./target/release/ilspy-rust-probe /path/to/System.Private.CoreLib.dll

# Optional second arg: SSA dump directory, or - for none.
# Optional third arg: comma-separated substrings of fully qualified method names.
# Selection happens AFTER decoding every method with cildec.
./target/release/ilspy-rust-probe /path/to/System.Private.CoreLib.dll - \
  'System.IO.MemoryStream::,System.Text.StringBuilder::,System.Collections.Generic.List`1::,System.Collections.Generic.Dictionary`2::'

# Explicit lenient experiment. Diagnostics remain visible AND cause nonzero exit.
PROBE_LENIENT=1 ./target/release/ilspy-rust-probe /path/to/System.Private.CoreLib.dll - \
  'System.IO.MemoryStream::,System.Text.StringBuilder::,System.Collections.Generic.List`1::,System.Collections.Generic.Dictionary`2::'
```

No custom-attribute semantics, cross-assembly resolution, instruction equivalence,
SSA semantics or execution equivalence is proved by this probe. Exception checks
compare counts and body sizes, not all clause fields. Decoder-only mode checks
MethodDef/local signatures, not every referenced MemberRef/MethodSpec signature.
Reported elapsed times are one-shot observations with unequal work and different
validation levels: **not a performance comparison**.

## Observed results

| Input | cildec decoded bodies / instructions | dotscope result | Probe exit |
|---|---:|---|---:|
| Release fixtures | 59 / 916 | 59 SSA built; 2 fail its own verifier | 1 |
| Debug fixtures | 61 / 1,066 | 61 SSA built; 2 fail its own verifier | 1 |
| .NET 10.0.12 CoreLib, decoder only | 39,317 / 616,981 | Not run | 0 |
| CoreLib, strict, 321 selected bodies | all bodies decoded first | Load fails on custom attribute 0x0c000141 | 1 |
| CoreLib, lenient, same selection | all bodies decoded first | 147 diagnostics; 321 SSA built; 11 verifier failures | 1 |

CoreLib contains 41,420 MethodDef rows/signatures, including methods without IL.
Fixture and CoreLib runs had zero cildec decode errors and zero image/metadata
diagnostics. Both libraries saw matching selected body sizes/EH counts and method
tokens when dotscope loading completed. This is useful smoke coverage, not proof
that either library is production-complete.

Fixture verifier findings:

- StressSample.Run: PhiWithoutPredecessors (two handler blocks, both builds).
- StressSample.NullableMath: OrphanVariable (both builds).

These are failures of the library's own validation contract, not yet diagnosed as
semantic SSA defects versus overly strict verification. They need triage before
using that SSA as trusted output. No upstream library was patched.

CoreLib strict load reports unsupported custom-attribute named argument type
0x1D. Lenient mode recorded 10 such messages, 135 other load failures and 2 other
diagnostics. It is **not** an acceptable silent fallback to complete decoding.

A separate smoke run of backtrip 0.1.4 on ControlFlowSample produced `return value
+ 1` for CheckedAdd (no checked semantics) and assigned V_0=1 before returning
Pair(V_0,1) for PreserveOldValue. The required result there is 71, not 11. Output
also contained invalid conditions such as `first == null`. Its source maps
add.ovf to ordinary addition, discards expressions on pop and resets its string
stack after an unsupported instruction. It is not recommended as a correctness
base. No arbitrary decompiled output was executed.

Input SHA-256 values:

- Release Fixtures.dll: 9019373477d013d6d5641cb77fd1d1219821722002f00d9ddebae4b7a99065ec
- Debug Fixtures.dll: be8b3f4d7cd1dece34c67e07c8ae266232394ea92246f7b3710a171d3a8a2392
- CoreLib: 7adbb705a094d8d6d4238cbc80f3dc6be17177384ce836994485bc5cbbb1d281

Raw local observations are under ignored report/*.log. All SSA probe failures
remain failures; only the decoder-only CoreLib run passed its complete probe.
