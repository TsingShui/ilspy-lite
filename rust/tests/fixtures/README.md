# Owned binary test fixtures

These are CLI PE assemblies with a ".bin" extension so the repository's general
DLL ignore rules do not exclude them. Tests parse them by content, not extension.
They contain only this repository's MIT-licensed fixture code. No downloaded
third-party test assemblies are checked in here.

| File | Source | Build | Bytes |
|---|---|---|---:|
| release.bin | TestTools/LiteExperiment/Fixtures | Release, .NET 10 | 13,312 |
| debug.bin | TestTools/LiteExperiment/Fixtures | Debug, .NET 10 | 13,824 |
| integer-release.bin | ../dotnet/IntegerCases.cs | optimized, Roslyn .NET 10 | 4,608 |
| integer-debug.bin | ../dotnet/IntegerCases.cs | unoptimized, Roslyn .NET 10 | 4,608 |

The array snapshots (`array-release.bin`, `array-debug.bin`) contain 21 exercised
bodies and one unsupported constructor from `../dotnet/ArrayCases.cs`. They test
signed/unsigned elements, fields, locals, joins, aliasing, native length conversion
chains and typed I4-length allocation. Allocation checks cover negative lengths,
zero initialization, fresh empty/nonempty identity and effects on either side of
failure; memory exhaustion is not forced. Regenerate with
`python3 rust/tests/build-array-fixtures.py`, then execute public class output
with `python3 rust/tests/reference-roundtrip.py --arrays --from-class`.
The Debug/Release gate passed 557 return/exception/state comparisons per
configuration. Git pins snapshots without a separate hash manifest.

The class-selection snapshots (`class-release.bin`, `class-debug.bin`) come from
`../dotnet/ClassCases.cs`. They contain empty/empty-generic types, nested types,
a distinct similarly named sibling, and supported/unsupported methods in one
type. Regenerate with `python3 rust/tests/build-class-fixtures.py`; tests mutate
only owned metadata to check failure isolation and invalid ownership/ranges.
They are parser/selection fixtures, not a semantic decompilation coverage claim.
Snapshot revisions are pinned by Git without a separate hash manifest.

The native-integer snapshots (`native-release.bin`, `native-debug.bin`) come from
`../dotnet/NativeCases.cs`: native storage, fields, calls and conditional selection.
Regenerate with `python3 rust/tests/build-native-fixtures.py`. The explicit
`reference-roundtrip.py --native --from-class` gate compares the actual class
output. Local execution uses a 64-bit .NET runtime; no 32-bit execution result is
claimed. Native arithmetic and implicit width changes remain negative cases.
Snapshot revisions are pinned by Git without a separate hash manifest.

The integer-conversion snapshots (`conversion-release.bin`, `conversion-debug.bin`)
come from `../dotnet/ConversionCases.cs`. They exercise low-bit truncation,
signed/unsigned storage, native widths and snapshots across mutation. Regenerate
with `python3 rust/tests/build-conversion-fixtures.py` and execute actual class
output with `reference-roundtrip.py --conversions --from-class`. Only local
64-bit runtime execution is verified; snapshots are pinned by Git.

The element-address snapshots (`address-release.bin`, `address-debug.bin`) come
from `../dotnet/AddressCases.cs` and the authored IL in
`../build-address-fixtures.py`. Core TypeRef operands are obtained from ref-return
anchors, not guessed from token order. Reserved bodies are replaced without
executing them; the producer validates tiny headers or ordinary fat headers
before changing code size/maxstack. Fifteen recovered bodies exercise immediate
null/bounds checks, address duplication across a call, field rebinding,
`ldind.u4` snapshots before/after alias writes, and `stind.i4` writes to the
original location. Calls can mutate aliased state or throw before a pending
write. The anchors retain unsupported byref signatures. Regenerate with
`python3 rust/tests/build-address-fixtures.py` and execute public class output
with `reference-roundtrip.py --addresses --from-class`. The Debug/Release gate
passed 1,892 return/exception/state comparisons per configuration. Other
indirect operations and cross-block address propagation remain unsupported.
Git pins the snapshots without a separate hash manifest.

The constructor snapshots (`constructor-release.bin`, `constructor-debug.bin`)
come from `../dotnet/ConstructorCases.cs`. Regenerate with
`python3 rust/tests/build-constructor-fixtures.py`. Public method/class tests
patch owned bodies to isolate constructor identity and initonly permission:
exact field ownership, unchanged entry this, ordinary/derived/other-receiver
negatives and visible unsupported constructor calls. A constructor fragment
without a base call tests permission only, not a valid complete initialization
chain. The `reference-roundtrip.py --constructors --from-class` gate executes
Base/Root bodies, including fields before/after the exact core Object call.
The adapter emits a real runtime IL call and checks receiver identity, position
and call count; local direct-core null behavior is tested without adding a check.
Debug/Release passed 22 return/exception/state comparisons per configuration.
Static constructors and generic/value-type constructors remain unsupported.
Git pins the snapshots without a separate hash manifest.

The reference snapshots add 18 methods each from `../dotnet/ReferenceCases.cs`,
4,096 bytes per snapshot. They cover local reference receivers/fields, generated
backing-field names, same-type joins, exact base calls and unsupported struct
receivers. Regenerate with `python3 rust/tests/build-reference-fixtures.py`.

SHA-256:

- release.bin: 9019373477d013d6d5641cb77fd1d1219821722002f00d9ddebae4b7a99065ec
- debug.bin: be8b3f4d7cd1dece34c67e07c8ae266232394ea92246f7b3710a171d3a8a2392
- reference-release.bin: e8ea7e98a8818cfcc7bfb1db705262656ab3e1251d853a1cb4b5f50f2bfa1193
- reference-debug.bin: 7aa8461a77cc9df2bbcbc41dc562f69246a81a044cc2388e12a0c63b99904b28
- integer-release.bin: add566f76a4840feff93195dd4a7bd2fb2a7829481429129ca241dd0453ad2ea
- integer-debug.bin: 9f56ce1f2380e93b4a3cae80dd30956ff454545c63e03877f761a9172dd59080

The two LiteExperiment snapshots were copied from its existing net10.0 outputs.
Rebuild with that experiment's documented root restore/build scripts and nested
SDK selection, then copy the two Fixtures.dll files here. Release has 59 methods;
Debug has 61. This is not a promise of byte-for-byte reproducibility across SDKs
or directory layouts; updating a snapshot requires checking inventory and hashes.

Regenerate the integer snapshots (25 methods each) from the repository root:


    DOTNET_ROOT="$HOME/.dotnet10" python3 rust/tests/build-integer-fixtures.py

The script uses Roslyn from the installed .NET 10 SDK directly, does not restore,
and does not change repository SDK selection. Re-run Cargo tests and the explicit
roundtrip gate after updating any fixture. Cargo tests need no .NET installation;
they consume these pinned snapshots.
