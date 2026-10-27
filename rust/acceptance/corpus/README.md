# Fixed-corpus E2E acceptance

Static acquisition, independent metadata accounting and production CLI gates for
all precompiled managed inputs at ILSpy-tests commit
`9668d49b9f71b40045e5417b55d20e330be95921`, plus the two pinned OpenRA inputs.
Selected assemblies, archived compilers, installation tools and build scripts
are never executed by this tooling. The owned metadata oracle is compiled and
run with an existing .NET 10 SDK; it reads selected images using PEReader only.
Production Rust decompilation remains independent of .NET.

Current enum-storage/conversion milestone: **558,757 / 770,748** implementation
bodies recovered; **27.50458% fallback**, so the aggregate <=5% goal is unmet. See
[enum storage/conversion evidence](ENUM_STORAGE_ACCESS.md) for scope, owned execution,
self-review and complete scan evidence.

## Acquire and independently inventory

From the repository root:

```sh
git submodule update --init --checkout ILSpy-tests
python3 rust/acceptance/corpus/corpus.py collect \
  --checkout ILSpy-tests \
  --output rust/report/fallback-corpus/frozen \
  --supplement rust/report/openra/assemblies/OpenRA.Game.dll \
  --supplement rust/report/openra/assemblies/OpenRA.Mods.Common.dll
python3 rust/acceptance/corpus/corpus.py inventory \
  --manifest rust/report/fallback-corpus/frozen/manifest.json \
  --output rust/report/fallback-corpus/inventory-final
```

`collect` checks the exact commit and tracked-file cleanliness. It records all
tracked files and regular ZIP/NuGet archive members, hashes acquired content,
retains duplicate origins and extracts bounded members below the report path.
Unsafe paths, encryption, unsupported/corrupt archives and expansion limits make
the manifest incomplete, not a smaller passing corpus. The optional
`--reuse-acquisition <path>` reuses the earlier acquisition hashes for unchanged
Git blobs and the two explicitly recorded unchanged OpenRA inputs.

The metadata reader cache lives under `rust/report/fallback-corpus/reader/`.
`DOTNET_ROOT` selects an installed .NET 10 toolchain, defaulting to `~/.dotnet10`.
No restore, downloads or full solution builds are performed. Unchanged source
and toolchain reuse the compiled reader. Raw metadata uses
`MetadataReaderOptions.None` so WinRT projection does not hide original RVAs.
Every MethodDef row is retained, including module, nested and generated methods.
Exact core-scoped reference markers and zero-byte code extents are recorded
separately; they do not silently change the denominator.

## Run the production E2E baseline

Build Rust once when its sources change, then scan all inputs with that binary:

```sh
cargo build --manifest-path rust/Cargo.toml --release --locked --offline
python3 rust/acceptance/corpus/corpus.py baseline \
  --inventory rust/report/fallback-corpus/inventory-final/inventory.json \
  --cli rust/target/release/ilspy-lite \
  --output rust/report/fallback-corpus/baseline \
  --workers 4 --timeout 300
python3 rust/acceptance/corpus/corpus.py gate \
  --baseline rust/report/fallback-corpus/baseline/baseline.json
```

`--resume` on `baseline` reuses per-image checkpoints only when the image,
independent inventory timestamp and CLI timestamp/size match. Full rescans omit
this flag. Equal-content inputs share one analysis but retain every origin.
A method is scoped to its input snapshot, not its token alone.

The production command `ilspy-lite scan <assembly>` emits one NDJSON record per
MethodDef, using the same decompilation path as token output and one parsed image.
Its exit 0 means enumeration completed, not that every body decompiled. Partial
and failed records remain explicit. Exit 1 reports a request/input/output error.

The acceptance baseline combines those records with the independent CIL-body
inventory. Partial, Failed and missing body results count as unrecovered. A
failed process, duplicate/unknown token, omitted CIL body or declaration reported
for a CIL body makes the scan incomplete. There is no implicit auto-reference
search during the production status scan.

Gate decisions use `100 * unrecovered <= bodies`, with exact integer counts.
Legitimate bodyless declarations and native resources are reported separately;
images with no CIL bodies have no recovery rate. Incomplete scans cannot pass.
Acceptance commands exit 0 for complete passing gates, 2 for complete but
unsatisfied gates, and 1 for incomplete/error results.

## Validation and iteration

```sh
python3 -m unittest discover -s rust/acceptance/corpus -p test_e2e.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test cli
```

These E2E tests exercise the real CLI/filesystem, static reader and an owned
small assembly. The raw WinMD check uses the fixed submodule as static input.
No test executes that input. Red/green runs and outputs stay under the ignored
report directory. Ordinary iteration uses relevant E2E cases; capability
milestones run relevant full regressions and corpus scans. Do not rebuild the
full ILSpy solution or recompile unchanged input assemblies for Rust changes.

## Approved implementation-only scope

Reference-only inputs are skipped under the confirmed goal revision, with no
separate metadata/API acceptance gate. All input origins, original statuses and
classification evidence remain available. Unknown empty or hard-to-decompile
implementation images are not excluded by filename or failure reason.

Apply this policy using saved reports only:

```sh
python3 rust/acceptance/corpus/implementation_scope.py \
  --inventory rust/report/fallback-corpus/inventory-final/inventory.json \
  --baseline rust/report/fallback-corpus/baseline-final/baseline.json \
  --output rust/report/fallback-corpus/implementation-scope
python3 rust/acceptance/corpus/corpus.py gate \
  --baseline rust/report/fallback-corpus/implementation-scope/implementation-baseline.json
```

This does not reacquire, extract, hash, compile or rescan any selected input.
Checked reference markers support ordinary skips. The specific unmarked legacy
mscorlib shell is identified by pinned commit, recorded SHA-256, MVID, origin and
method/code-extent facts in `reference-exceptions.json`, not by its short name.
Malformed or mismatched exception evidence cannot authorize a skip.

The first scoped report retained 615 managed snapshots and 777,951 target bodies.
Exact local-core marker recognition subsequently proved another input reference-only.
The corrected scope retains 614 managed snapshots, 319 with implementation bodies,
and 770,748 target bodies; it skips 362 reference snapshots and reconciles all
1,098,128 original CIL-marked RVAs without rewriting statuses. The legacy mscorlib
input is now independently recognized by its local marker; the pinned exception
remains available for historical evidence but is not needed by the current reader. Classification
alone recovers zero methods. Empty functions, synthetic returns, status relabeling
and executing the original method are not acceptable ways to improve recovery.
For future capability milestones, apply the same frozen policy to each new scan;
compare implementation counts only at the same scope. The emitted
`implementation-inventory.json` is the saved independent inventory with the
approved reference ids removed, retaining native/not-applicable entries and all
original ids. Use it with `corpus.py baseline` for capability scans so skipped
reference inputs are not decompiled again. See
[the object reference milestone](OBJECT_REFERENCES.md) for the first measured
semantic improvement at its recorded scope. The latest corrected scope and
current implementation inventory are documented in
[the core type-root milestone](CORE_TYPE_ROOTS.md).

The baseline and per-method evidence are in
`rust/report/fallback-corpus/BASELINE.md`, `inventory-final/`, `baseline/`,
`baseline-final/` and `scope-contradiction.json`. The final baseline has identical
per-method results to the initial scan; `baseline-comparison.json` retains that
cross-check. `baseline-integrity.json` covers all MethodDef rows, including
bodyless declarations. The acceptance and architecture decisions awaiting user
approval are documented in [DESIGN_PROPOSAL.md](DESIGN_PROPOSAL.md). `report.py` summarizes these saved snapshots without
recompiling, rescanning or rehashing unchanged inputs. External binaries and
generated output remain ignored and retain their original license boundaries.

## Per-milestone documents

- `OBJECT_REFERENCES.md`, `CLOSED_GENERIC_REFERENCES.md`, `ARRAY_REFERENCES.md`,
  `SCOPED_REFERENCE_VARIABLES.md`, `GENERIC_VALUE_COPIES.md`, `ENUM_STORAGE.md`,
  `STRUCT_VALUE_COPIES.md`, `STRUCT_RECEIVERS.md`, `VIRTUAL_DISPATCH.md`,
  `LOCAL_ALLOCATION.md`, `REFERENCE_EQUALITY.md`, `CAST_SEMANTICS.md`: the implemented semantic slices, each with its owned
  evidence, mutation controls and same-scope recovery delta.
- `CORE_TYPE_ROOTS.md`: qualified core roots and reference-marker classification.
- `AGENT_ROLES.md`: direct main-session workflow, ILSpy-first audit, owned behavior
  gates, self-review, and commit-only version control.
- `REVIEW_PROTOCOL.md`: source-review evidence before a milestone's counts are
  treated as final; IL is the semantic authority, not ILSpy's appearance.
- `EXCEPTION_REGIONS.md`: bounded catch/finally regions, validated transfers and
  executable default output, with filters/faults kept as explicit fallbacks.
- `EXTERNAL_TYPE_FACTS.md`: the confirmed scope decision (option C) for capabilities
  that need facts about types defined outside the scanned image, plus the
  implementation plan.
- `EXTERNAL_REFERENCES_PLUMBING.md`: the landed plumbing that gives every scan
  exactly the references its image declares and records the selection.
- `EXTERNAL_ENUM_STORAGE.md`: the first capability that consumes those references -
  storage width of enums defined in a referenced assembly.
- `EXTERNAL_PROPERTIES.md`: metadata-proven external instance getters/setters,
  retaining declared-owner lookup, virtual dispatch and argument/null-check order.
- `EXTERNAL_CALL_DISPATCH.md`: instance calls on receivers whose method is declared
  in a referenced assembly, with the order-independence defect that review found and
  its per-token parity gate.
- `VALUE_TYPE_COPIES.md`: constructed value types and external value types as
  whole-value copies, and the three-state rule that makes an unresolved reference
  leave output byte-identical.
- `CONSTRAINED_DISPATCH.md`: exact ordinary constrained calls, invariant reference
  CFG/local slots and narrow null/reference merges, with same-state execution.
- `CONSTRUCTED_VALUE_CALLS.md`: exact constructed value calls and newobj, scoped
  field signatures and value/generic boxing, with receiver/constructor/copy gates.
- `REFERENCE_RELATIONSHIPS.md`: scoped base/interface assignments, metadata-backed
  variance, exact foreign ancestor projection and original overload identity.
- `MANAGED_LOCATIONS.md`: typed managed pointers, invariant byref operands,
  indirect storage operations, exact initonly field addresses and alias/snapshot gates.
- `REFERENCE_CALL_FOUNDATION.md`: exact scoped external/reference calls,
  constructed owners, qualified core identity and reference TypeSpec conversions.
- `METHOD_INSTANTIATIONS.md`: exact static/nonvirtual MethodSpec calls and
  simultaneous Var/MVar substitution; generic virtual lookup remains gated.
- `GENERIC_MEMBER_SCOPES.md`: caller-qualified generic member binding,
  distinct constructed owners and exact constructed static calls.
- `LOCAL_INTERFACE_REFERENCES.md`: grouped abstract/core/interface runtime checks,
  local interface reference identity, exact dispatch and proven assignment.
- `ABSTRACT_CONSTRUCTED_MEMBERS.md`: member lookup on abstract generic classes,
  kept separate from the concrete-owner requirement for allocation.
- `CONSTRUCTED_MEMBERS.md`: members reached through a constructed type - open self
  instantiation, substituted fields, calls, allocation and constructed base classes.
- `EXTERNAL_FIELDS.md`: instance reads and writes on a field declared in a referenced
  image, with the caller-scoped field type and the owning-class requirement.
- `EXTERNAL_ALLOCATION.md`: `newobj` on a constructor from a referenced image - the
  owner/constructor facts, the three-state rule, the refusals, and the constructed and
  nested owners that remain gated.
- `BOXING.md`: historical core-scalar boxing foundation. Broader enum/value/generic
  admission is recorded in the enum and constructed-value milestone documents.
- `STATIC_CONSTRUCTORS.md`: static constructor bodies lowered as the type's own
  initializer instead of the former blanket refusal, with the bounded probe that
  decomposes the old row into the shapes that still block.
