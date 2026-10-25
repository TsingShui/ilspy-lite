# Readable output and complete evidence

Readable-body simplification validated at `f9ce32653`; inline fallback-body
presentation has separate focused validation described below.

The CLI defaults to C#-like class/method presentation. Append `--details` for the
existing complete metadata/IL analysis. This is a presentation change, not broader
lowering admission, dependency resolution, CLR interpretation or source export.

```sh
rust/target/release/ilspy-lite decompile Assembly.dll --type Namespace.Type
rust/target/release/ilspy-lite decompile Assembly.dll --type Namespace.Type --details
```

The library's existing `decompile` and `decompile_type` retain detailed output.
`decompile_with_style` and `decompile_type_with_style` expose
`OutputStyle::{Readable, Details}`. Both styles preserve status/summary precedence;
unsupported bodies show their reason and complete commented IL inside the
method body in both styles. Failed bodies retain available raw/decodable body
evidence or the original parse error inline. No method is counted successful
because its fallback comments look like a body.

## Delivered presentation

- Normal namespace/class/member declarations, original modifiers, source parameter
  names and `this`; no implicit entry receiver null check is introduced.
- Constants, all fields/methods and recursively owned nested/generated types remain
  visible. Invalid/generated field names keep the lowering token alias.
- Property/event names, types and actual accessor relationships have concise
  summaries. Accessor bodies remain explicit methods; no source sugar is guessed.
- No raw metadata rows, parsed signature structures, per-instruction comments or
  repeated success banners in ordinary supported bodies. Fallback methods show
  complete original IL, locals and EH ranges in place, not a link to `--details`.
- Bounded immutable copy/literal elimination, Boolean encoding simplification,
  adjacent pure unchecked expressions, return-only edges and early returns.
  Other CFG edges/loops remain labeled; field reads, managed addresses, calls,
  allocation and throwing/checked/runtime operations keep their evaluation point.
- Exact-call and target-F/native/literal intrinsics remain when ordinary C# would
  lose identity or semantics. Constructors that cannot be lowered remain fallback;
  readability does not certify complete initialization or source binding.

## Tests and execution

Each presentation slice had a failing public-interface regression before its
implementation. The current readable-output tests cover class/method/detail mode,
constants/nested types, safe copies/old field values, Boolean returns, bounded
branch/indentation, field/argument collisions, unique fallback parameter names,
out parameters, field aliases and generic accessor-summary owner context.

Inline-fallback presentation is covered through readable method/class APIs and
actual default CLI output. Debug/Release unsupported-call and EH fixtures retain
every disassembled body line inside the corresponding method. Corrupted
signature/opcode/RVA fixtures retain raw/decodable evidence or the original error;
peer methods, summaries and failure exits remain unchanged. The fallback change
passed readable/CLI/class-method tests in Debug/Release, fmt, strict all-target
clippy and the actual readable Wait.Tick behavior gate (193 comparisons).
Logs: `report/inline-fallback-{red,green,debug,release,clippy,wait-behavior}.log`.
Its rendering path does not change supported-body compaction or detailed output.

`reference-roundtrip.py --readable --from-class` executes actual readable
statements. The normalized method view must occur in the actual readable class
output. Its test-only adapter rebinds receiver/parameter variables to the existing
shell signature; detailed field comments supply shell identities, not method
logic. Member slots/literals are not rewritten and duplicate parameter names are
rejected. This is a test adapter, not a production runtime or source exporter.

Final locked Rust Debug/Release, fmt, strict all-target clippy and independent
five-class detailed inventories pass. Actual readable-body behavior evidence:

| Gate | Passing comparisons |
|---|---:|
| All seven frozen MersenneTwister methods | 22,941 |
| Required resources/ammo bodies | 837 |
| Frozen Activity.Queue | 20 |
| Frozen Wait.Tick | 193 |
| Owned references, per Debug/Release | 522 |
| Owned arrays, per Debug/Release | 557 |
| Owned native, per Debug/Release | 197 |
| Owned conversions, per Debug/Release | 385 |
| Owned element addresses, per Debug/Release | 1,892 |
| Owned strings, per Debug/Release | 124 |
| Owned exceptions, per Debug/Release | 259 |
| Owned floats, per Debug/Release | 2,010 |
| Owned core constructors/clock/delegation, per configuration | 22 / 5 / 20 |
| Owned exact external references/calls, per configuration | 48 |

These finite-input checks use local 64-bit .NET 10.0.12 and retain the earlier
runtime/dependency limits. They do not prove arbitrary C# compilation, other
CLI F implementations, all-width execution or all-input equivalence.

```sh
(cd rust && cargo test --locked && cargo test --release --locked)
python3 rust/tests/test_readable_adapter.py
python3 rust/acceptance/openra/test_inventory.py
python3 rust/acceptance/openra/inventory.py --check-cli
python3 rust/tests/reference-roundtrip.py --wait --from-class --readable
python3 rust/tests/reference-roundtrip.py --random-class --from-class --readable
```

Logs are ignored under `report/readable-final/`; GPL class output stays under
`report/openra/`, not in tracked implementation/docs. Run the behavior scripts
serially because their optional compiler/harness output directories are shared.

## Independent review and corrections

Read-only review identified five concrete presentation/adapter defects: rewriting
member identifiers, colliding fallback parameter names, out-to-ref header changes,
backing-field alias disagreement and adapter member substitution. All were fixed
in small TDD commits; focused follow-up returned **OK; no issues found**. Artifacts:
`reviews/readable-output.md` and `reviews/readable-output-followup.md` (follow-up
run `e51c3119-e948-42e6-b980-36734d029a3e`).

Final accessor/store polish review confirmed bounded store behavior and found an
event-summary generic-owner omission. Its public regression was red then green;
`98390c9fb` passes the declaring owner to preserve `Action<T>`, not `Action<T0>`.
Artifact: `reviews/readable-output-final-polish.md`, run
`7e27bcd6-28a5-4887-87ca-07c9354e1876`. Parent execution supplies validation;
reviewers inspected sources/output/logs, not executed inputs. A further public
namespace-`a0` regression guards qualified/declaration type slots against value
substitution. Final owner/type follow-up returned **OK; no issues found**;
artifact `reviews/readable-output-owner-type-followup.md`, run
`71481457-c30e-4569-b698-d45a4147ca2a`.

Historical detailed-output delivery is in [openra/DELIVERY.md](openra/DELIVERY.md).
Operation contracts and remaining limitations: [../SEMANTICS.md](../SEMANTICS.md).
