# Frozen naming: scalar method execution

Optional offline gate, not whole-class/game or naming-goal completion. It adds no
production admission and does not modify the constructor gate.

```sh
# From the repository root, using an existing read-only cache:
python3 rust/tests/frozen-naming-methods.py \
  --corpus /absolute/path/to/existing/rust/report/openra
```

Requires the installed .NET 10 SDK/reference pack/runtime in `~/.dotnet10` (or
`DOTNET_ROOT`), Cargo's existing offline cache, and corpus `assemblies/`,
`runtime-deps/`, `random-classes/final-21c469ba6/*/class.json`. No acquisition,
restore or package installation occurs. Cargo uses `--release --locked --offline`;
Roslyn compiles isolated test artifacts against framework reference packs only.
The original Game/Common assemblies are **not compiler references**.

## Fixed scope and evidence

The unchanged twenty owner trees are enumerated with independent static PEReader
metadata/raw IL. The frozen baseline and current per-method statuses must identify
exactly the eight newly recovered nonconstructor methods below. Missing recovery,
additional methods requiring a scope decision, or changed method/field protocols
fail rather than silently shrinking/changing the gate. The supplemental three
add no newly recovered scalar methods and are not replacements for these trees.

Pinned assembly SHA-256 values, MVID/root identities, MethodDef ownership, raw names,
signatures, attributes, field identities and original IL digests must agree.
Tracked test code pins digests rather than embedding corpus body bytes; raw
original/generated bodies remain in ignored reports. CLI original
IL crosschecks PEReader; loaded original IL, ownership, flags, parameter/result
and field types are checked again before execution. Cataloging never loads the
input. Module initializers are forbidden; allocated original owners/referents
must have Object ancestry, no type initializer, no generics and no finalizer.

The exact current CLI default `--type` output is saved. Method extraction binds
its independent owner path, name, unique signature and token; saved character
spans, body hash and actual/adapted headers describe the correspondence. The
**body substring occurs once, unchanged**, in the positive wrapped source.
Details supplies only status records, never implementation statements. No
`decompile --method` body supplies the implementation.

Test ABI adaptations are explicit: public nonvirtual method headers, public
observed-field declarations, ordinary outer context classes and omitted unused
members/interfaces. For MovementCancelled only, the unused `OpenRA.Actor self`
header parameter becomes `object self`, passed null on both sides in every case.
The original Actor type/assembly remains in the report. No Actor is allocated or
initialized. The closure predicate uses a same-full-name recovered shadow
WithHarvestOverlay containing only `visible`, preserving its actual fully
qualified local declaration and all body statements. Shadows do not proxy any
original method. This is not recompilation of the full original classes.

Selected original/recovered bodies must contain only bounded call-free scalar
IL and the independently mapped instance field operands. Calls, selected-original
proxies, static fields, indirect execution and unrelated field operands fail the
guard. Each side allocates receivers/referents independently without constructors
or factories, and prepares state from independent constants/strings. Original
poststate is never copied into recovered objects or expectations.

## Checked cases

Tokens are in OpenRA.Mods.Common.dll except the marked Game entry. Every case is
called three times on its independently prepared receiver pair.

| MethodDef | Behavior | Positive pairs | Mutation ID / specific mismatch |
|---|---|---:|---|
| 0x0600147B (Game) | Iterator string getter, field 0x04000E40 | 30 | 0x0600147B: return null / return-identity |
| 0x06002770 | Dispose stores -2 in I4 field 0x040020A6 | 30 | 0x06002770: store 0 / store-state |
| 0x06002772 | Iterator string getter, field 0x040020A7 | 30 | 0x06002772: return null / return-identity |
| 0x060021A3 | MovementCancelled returns without touching this | 9 | 0x060021A3: write visible=true / preserved-state |
| 0x060021A5 | Harvest callback clears visible 0x04001B90 | 9 | 0x060021A5: write true / store-state |
| 0x060045A6 | Closure reads referent 0x04003AE3, returns !visible | 12 | 0x060045A6: invert predicate / return-value |
| 0x0600338F | DownloadPackageLogic closure text getter 0x04002AD9 | 30 | 0x0600338F: return null / return-identity |

Total: **7 methods, 150 positive call pairs, 7 compiled runtime mutation
rejections across 5 behavior families**. This includes 21 null-this pairs and
3 null-referent pairs. The remaining 126 pairs have nonnull receivers/referents.

- Each getter tests null, empty, two distinct equal-content `alpha` inputs, NUL,
  surrogate boundary code units, isolated high/low surrogates and a surrogate
  pair. Nonempty input strings are allocated independently per side. Returns
  must preserve each side's own exact reference identity, not just text equality.
- Dispose tests nine I4 states, including MinValue, MaxValue, -3/-2/-1/0/1 and
  adjacent extrema. Its poststate must independently equal -2 on each side.
- Visibility stores test dirty true/false states and preserve false on repeats.
- The closure predicate tests independent true/false referents, null referent,
  and null this. Returned values and receiver/referent state are checked.
- MovementCancelled tests nonnull true/false-state receivers and null this with
  null Actor arguments. Direct calls add no receiver null check; null this
  returns normally. All field getters/stores must instead fault with exactly
  NullReferenceException for null this; the predicate also faults for null referent.

Other original fields are snapshotted only as primitive values/reference
identities, never via user Equals or gameplay methods. Snapshot names and any
unobserved fields are reported. All fields were safely observed in this run;
omitted references remain zero-initialized, not arbitrary gameplay state.

The eighth method, WithSpriteBodyInfo/<>c `0x0600468E`, returns zero but is
**excluded**: `.cctor 0x0600468B` is
`newobj 0x0600468C; stsfld 0x04003BD9; ret`. That selected-original singleton
allocator/constructor is never executed. Its independent IL and actual default
method span remain saved.

## Controls and reports

Each mutation alters a single occurrence in the actual extracted body and must
compile, pass the call/field guards, execute and produce exit **42**, status
`behavior-mismatch`, side `recovered`, the exact method token and the mismatch
kind in the table. Compiler failures, guard rejection, exception mismatches and
unrelated exceptions do not count. The driver executes these behavioral red
controls before the corresponding unchanged positive body. Getter mutations
retain the original field read (compiled pop), so null faults are not replaced
with a misleading return-only stand-in.

Ignored `rust/report/frozen-naming-methods/` retains exact default outputs,
independent metadata/IL, baseline records, extraction evidence, wrapped positive
and mutant sources, framework-only compiler response files, compile/run logs,
commands/exit codes and `summary.json`. A failing run removes the success summary.

Limits: bounded selected-body/observed-field semantics only, not full class ABI,
interface dispatch, constructors, nonnull Actor inputs, world/callback/game paths,
static initialization, arbitrary omitted-field states or alias runtime identity.
The original names/types/flags stay evidence even where the shell ABI is adapted.
