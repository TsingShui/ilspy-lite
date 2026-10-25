# Class milestone A: usable named-type analysis

## Delivered interface

    ilspy-lite decompile Assembly.dll --type Namespace.Type

The public decompile_type entry parses one input image and selects TypeDefs by
exact metadata name, including empty, generic, nested and generated types. A
selection includes recursively owned nested types, never similarly named siblings
or referenced dependency classes. No class-name or token whitelist is used.

Each type section contains its identity, raw/named flags, enclosing/base/interface
references, generic parameters/constraints, fields, properties, events, original
method/parameter metadata and a separate status for every method. Unsupported
bodies retain complete readable IL instead of blocking the whole class. Invalid
metadata is visible and counted separately from failed method bodies.

This is an Agent-oriented analysis record, not a compilable class or full source
reconstruction. Runtime attributes, layout beyond the explicitly reported field
evidence and complete assembly resolution are not reconstructed. The existing
IL/metadata and original execution remain authoritative.

## Frozen real-class output check

Production source checked: b4ed0fd48. The inventory oracle is the separate
System.Reflection.Metadata PEReader, not the Rust parser. Five frozen classes
passed complete type/field/method/property/event identity, flags/signature,
parameter/default, generic-context and accessor-relationship comparisons.

| Class | Lowered bodies | IL fallbacks | Failed bodies |
|---|---:|---:|---:|
| PlayerResources | 7 | 3 | 0 |
| AmmoPool | 4 | 7 | 0 |
| MersenneTwister | 2 | 5 | 0 |
| Activity, including owned generated types | 12 | 50 | 0 |
| Wait | 0 | 3 | 0 |

All five outputs exit 2 and have no metadata failures. These body counts are
not behavioral coverage: callers may name unsupported dependencies. Wait is
complete inventory only and has NOT passed its final independent semantic gate.
Activity.Queue and Wait.Tick remain required later targets; no samples were
replaced. The two independent classes are related OpenRA classes, not random
whole-ecosystem coverage.

In particular, MersenneTwister.Next() and Next(int32) can lower as callers, but
Generate, the constructors, the range overload and NextFloat still fall back.
A recovered executable random generator is NOT delivered by this milestone.

## Behavioral and reading evidence

The test harness extracts method bodies from the actual --type output using
their explicit status/token sections, then compiles and executes those bodies.
It does not regenerate them via --method or proxy selected logic to the oracle.
The state shells and exact direct-call adapters remain test-only.

- OpenRA resource/ammo: 5 recovered bodies, 837 return/exception/state comparisons
  passed (same bounded input domains as STAGE1.md).
- Owned reference corpus: 14 recovered bodies, 522 comparisons per configuration.
- Owned array corpus: 11 recovered bodies, 453 comparisons per configuration.
- Existing integer method regression: 735 cases per configuration.
- Debug/Release Cargo suites, rustfmt and strict clippy passed.
- The class-output acceptance checker has 5 positive/negative tests: it rejects
  missing/duplicate members, wrong ownership, summary/exit mismatch and fallback
  presented as successful source.

A fresh-context read-only reviewer inspected only resources/ammo/random class
outputs and the semantic README, not original source, DLLs or implementation. It
correctly explained resource wraparound and old/state values, full-ammo equality,
exact getter dependencies and null-check points, and refused to call the random
generator recovered. Bounded reading acceptance passed with no issues found.
This is a single reading exercise, not formal semantic proof or a model benchmark.

Logs and generated outputs stay under report/openra/class-gate/ and
report/reference-roundtrip/. Runtime: existing .NET 10 on macOS arm64. No game
was launched. No inputs were downloaded or rehashed, and no source-size recount
was performed for this milestone.

## Reproduce the distinct gates

From the repository root:

    (cd rust && cargo build --release --locked)
    python3 rust/acceptance/openra/test_inventory.py
    python3 rust/acceptance/openra/inventory.py --check-cli
    python3 rust/tests/reference-roundtrip.py --openra --from-class
    python3 rust/tests/reference-roundtrip.py --from-class
    python3 rust/tests/reference-roundtrip.py --arrays --from-class
    python3 rust/tests/roundtrip.py

The inventory gate does not execute input bodies and does not claim all required
methods lower. The behavioral OpenRA gate still covers only the five approved
resource/ammo methods. Optional --verify-inputs explicitly rechecks existing
OpenRA hashes; normal iterations reuse previously verified local inputs. Use that
option after replacing inputs or when corruption is suspected, not as a repeated
step for unchanged files. Never point the execution harness at untrusted inputs.

## Next work

Milestone B completes the committed random-class workload in small semantic
steps. Milestone C validates the frozen independent class bodies and reading
contract. Neither is complete just because the class-level inventory is complete.
