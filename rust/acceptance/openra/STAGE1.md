# OpenRA stage 1: implemented and behaviorally checked

## Scope and result

The four frozen primary methods and the one supporting getter now report
Decompiled, using generic IL handling rather than OpenRA name/token cases:

| Method | Baseline | Stage 1 |
|---|---|---|
| PlayerResources.GetCashAndResources | Partial | Decompiled |
| PlayerResources.GiveResources | Partial | Decompiled |
| PlayerResources.TakeResources | Partial | Decompiled |
| AmmoPool.get_HasFullAmmo | Partial | Decompiled |
| AmmoPool.get_CurrentAmmoCount (support) | Partial | Decompiled |

The six later-stage primary methods, two generic support accessors and both
EH/iterator sentinels still report Partial. Stage 2 was not started. The CLI
continues to show their IL rather than pretending these methods are recovered.

## Implementation

Only three production files changed:

- src/ir.rs: image-scoped reference type identity and separate stack categories;
  integer operations cannot consume object references. ldnull has an explicit
  null-reference value. Reference joins require identical types.
- src/metadata.rs: local class/field identities, structured FieldRef, bounded
  local ancestor checks and MethodDef direct-instance-call descriptors. Field
  aliases are deterministic by token; original names stay in comments. External
  bases do not trigger dependency loading or establish external assignability.
- src/lift.rs: explicit receiver slot, ordered ldfld/stfld, typed reference
  stores/returns and exact-call intrinsics. Stable values and parallel stack
  copies are retained. Constructors, callvirt and managed addresses are not
  accepted merely because instance methods became possible.

cfg.rs, lib.rs and main.rs are byte-identical to their pre-stage-1 versions.
No new Cargo dependency, AST, SSA framework, resolver or pass manager was added.
The runtime executable still has no .NET dependency.

### Output contract

Instance methods render as static bodies with receiver a0. The explicit receiver
permits exact IL call semantics, including calls with null receivers whose bodies
never dereference them. Direct calls use __cil_direct_TOKEN(receiver, args), not
ordinary C# virtual calls. This is documented C#-like syntax, not a complete
self-contained compilable class. Intrinsics preserve an operation rather than
reconstructing source-level dispatch or property syntax.

Generated field names use __il_field_TOKEN aliases, retaining exact token and
original metadata spelling. Output is intentionally verbose, with temporaries,
IL offsets and goto. Access modifiers, constructors, type initializers and class
layout are not reconstructed. The original metadata remains authoritative.

## Gates run

Environment: macOS arm64, Rust/Cargo 1.98.0, .NET SDK 10.0.401/runtime 10.0.12.

- cargo fmt --check: passed.
- cargo clippy --locked --all-targets -- -D warnings: passed.
- cargo test --locked and cargo test --release --locked: 24 tests each, passed.
- Existing integer roundtrip: 735 cases per configuration, 1,470 total, passed.
- New owned reference roundtrip: 522 cases per configuration, 1,044 total, passed.
- Pinned OpenRA stage-1 roundtrip: 837 cases, passed.
- OpenRA static probe --require-stage 1: exits 0.
- No-body/unsupported/failed statuses remain distinct; same corpus hashes retained.

Field access, direct instance call, external-base handling and null-return tests
were observed failing before their implementations. Additional negative IL probes
ensure references do not become arithmetic operands, numeric field receivers are
rejected, callvirt does not become a direct call, and struct receivers fall back.

### How the behavioral comparison works

The optional C# harness uses reflection on the pinned input to identify fields
and methods. It creates separate state shells containing only accessed fields,
compiles the actual Rust output into static bodies, and maps each direct-call
intrinsic to another body in the recovered set. An absent recovered call target
is a hard failure. It never maps a generated method back to the original body.

The original side invokes exact MethodDefs using Reflection.Emit call, independently
of the generated source. Objects are allocated without constructors and populated
with matching test state. This isolates the selected operations; it does not test
construction, native dependencies, class initialization or the running game.

OpenRA cases compare Cash, Resources, ResourceCapacity, Earned and Spent after
normal, negative, capacity and integer-boundary inputs. Ammo cases compare return
values, backing-field state, Info identity, Info.Ammo and null-reference outcomes.
The inputs are Cartesian products, so 837 is not 837 independent behaviors.

Owned fixtures add old-field-value preservation, aliased objects, null source /
target / receiver, reference identity through branches, and a base method called
on a derived instance with an override. These specifically test the no-virtual-
dispatch and no-added-null-check semantics of direct calls.

A local mutation probe removed three resource stores from recovered source while
leaving return values in place. The same harness failed with Field mismatch:
Resources. This demonstrates that field-state assertions are active; it is not a
comprehensive mutation score. Mutant/source/output artifacts remain ignored.

## Reproduce

From the repository root:

    (cd rust && cargo build --release --locked)
    python3 rust/acceptance/openra/probe.py --require-stage 1
    python3 rust/tests/reference-roundtrip.py
    python3 rust/tests/reference-roundtrip.py --openra
    python3 rust/tests/roundtrip.py

The first command after building is static only. --openra explicitly executes the
five selected method bodies in a separate .NET 10 test process, not the game.
Do not use the harness to execute arbitrary untrusted assemblies.

The behavioral harness needs dependencies from the same official macOS package.
With the verified DMG mounted read-only as described in README.md, additionally:

    mkdir -p rust/report/openra/runtime-deps
    cp "$mountpoint/OpenRA - Red Alert.app/Contents/MacOS/arm64/"*.dll rust/report/openra/runtime-deps/

Then unmount the DMG. The local setup copied 189 DLLs without running anything.
They are ignored analysis dependencies, not shipped runtime dependencies of our
Rust tool. The two OpenRA assemblies are hash-checked against manifest.json before
the gate runs. Other dependencies must come from that verified package; the gate
does not independently hash every dependency. Generated sources and logs are in
rust/report/reference-roundtrip/ and rust/report/openra/.

## Size accounting

Physical lines after rustfmt, including headers and blank lines:

| Category | Size |
|---|---:|
| Production Rust, 6 files | 1,401 lines (was 1,173; +228) |
| New Rust integration tests | 158 lines |
| New Python fixture/behavioral tooling | 129 lines |
| New owned C# fixture/behavioral tooling | 320 lines |
| New generated reference fixture binaries | 2 x 4,096 bytes |
| New third-party Cargo dependencies | 0 |
| cildec published source (separate ownership) | 11,470 lines, unchanged |

The optional 133-line OpenRA static probe predates this implementation step.
Documentation, downloaded OpenRA source/binaries and generated test shells are
separate from production size. Local release executable: 627,280 bytes; this is
one macOS arm64 build, not a portable size/performance guarantee.

## Limits

These are bounded method-level differential tests, not whole-game equivalence or
CLI verification. Token identity is scoped to a single image. Only local reference
class signatures and locally provable class assignments are accepted; mixed-type
joins, reference comparisons, external/interface/generic calls, constructors,
arrays, value types, EH and prefixes still have explicit limits. The implementation
is not hardened against every malformed or hostile metadata shape.

The first delivery stops here. Stage 2 is not part of this milestone.
