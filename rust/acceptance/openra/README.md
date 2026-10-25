# Bounded OpenRA acceptance corpus

This is an external real-world corpus, not an expansion of the product contract
to "decompile all of OpenRA". The first delivery boundary is **stage 1 only**.
Stages 2 and 3 are recorded candidates, not authorization to implement everything
needed by them. Stage 1 is implemented and has passed its bounded behavioral
gate; see [STAGE1.md](STAGE1.md). The original corpus-setup baseline is preserved.

## Pinned input and provenance

- Official release: [release-20250330](https://github.com/OpenRA/OpenRA/releases/tag/release-20250330).
- Source commit: `b4f3d8ae02b6bd6f4a3a5441e6d0daf7c5073787`.
- Artifact: `OpenRA-release-20250330.dmg`, using the files under
  `OpenRA - Red Alert.app/Contents/MacOS/arm64/`.
- Assemblies: `OpenRA.Game.dll` and `OpenRA.Mods.Common.dll`.
- Matching official source archive: `OpenRA-release-20250330-source.tar.bz2`.
- URLs, SHA-256 hashes, method tokens, exact signatures and source locations:
  [manifest.json](manifest.json).

The four selected source files from the release archive were byte-compared with
GitHub raw content at the pinned commit and matched. The release API provides no
asset digest, and its Git tag is unsigned: recorded SHA-256 hashes pin the bytes
retrieved over HTTPS, not an independent publisher signature. The game binaries
were not rebuilt to establish a reproducible-build claim.

The ARM64 designation is the package directory, not the managed PE header. The
DLLs retain portable CIL; `file` reports PE32/i386. This does not mean they came
from the Windows download. No game, mod, installer or downloaded build script
was executed. The DMG was mounted read-only, two DLLs were copied, then it was
unmounted. No game assets are needed for this static probe.

OpenRA is GPL-3.0-or-later. Its source, binaries and generated IL/C# output stay
in the ignored `rust/report/openra/` directory. They are external analysis inputs
and validation references, not copied into our MIT production code. Do not check
these artifacts into this repository or incorporate OpenRA implementations into
the decompiler. If redistributing derived outputs, assess and comply with the
applicable license obligations first.

## Ten selected methods, in order

| Stage | ID / method | Why this method | IL instructions |
|---|---|---|---:|
| 1 | `resources-total`: PlayerResources.GetCashAndResources | Read real instance state | 6 |
| 1 | `resources-credit`: PlayerResources.GiveResources | Update resources/earned totals with a capacity branch | 32 |
| 1 | `resources-debit`: PlayerResources.TakeResources | Early failure or stateful debit | 20 |
| 1 | `ammo-full`: AmmoPool.get_HasFullAmmo | Instance getter call and nested Info field | 7 |
| 2 | `random-seed`: MersenneTwister..ctor(int) | Allocate and initialize a uint array | 50 |
| 2 | `random-next`: MersenneTwister.Next() | Stateful random draw and unsigned operations | 61 |
| 2 | `random-generate`: MersenneTwister.Generate | Array loop with a managed element address | 64 |
| 3 | `ring-read`: RingBuffer<T>.get_Item | Generic array indexing | 13 |
| 3 | `ring-add`: RingBuffer<T>.Add | Generic element stores and wraparound state | 42 |
| 3 | `ring-contains`: RingBuffer<T>.Contains | Generic interface dispatch and early-return loop | 37 |

These are deliberately selected coverage-development cases, not a random sample
or a held-out test set. Passing them would not measure whole-game compatibility.

Three small **support methods** are tracked separately, not hidden in the ten:
`AmmoPool.get_CurrentAmmoCount` for stage 1, and `RingBuffer<T>.get_Count` /
`set_Count` for stage 3. A behavioral harness may provide test setup and external
runtime dependencies, but must not delegate a selected or support method's body
to the original implementation and count it as recovered.

Two **fallback sentinels**, outside the delivery count, retain visibility for
`PlayerResources.GiveCash` (checked overflow with catch regions) and the generated
`RingBuffer<T>/<GetEnumerator>d__23.MoveNext`. These do not add EH or iterator
recovery to stage 1. Their success/fallback status is reported honestly.

## Stage 1: the stopping point

Required capabilities are a reference-type instance receiver, typed instance
field loads/stores, direct instance calls, and null/reference semantics. Existing
integer arithmetic, branches and stable stack snapshots can be reused. Do not
introduce a complete resolver, generic SSA, C# AST or a general optimization
framework merely to pass these methods.

Completion requires all of the following:

1. The four primary methods and their one support getter report `Decompiled`,
   with identifiable field/call targets and no erased operations.
2. Compare return values, **all relevant mutated fields**, and exception outcomes
   against the pinned implementation or an explicitly identified reference built
   from the matching source. Include normal, boundary and negative inputs.
3. The null behavior of the nested Info access remains explicit/correct. Calls,
   reads and writes retain their IL order.
4. Re-run the existing Rust and integer roundtrip tests. Unsupported OpenRA
   methods remain visible with their raw IL; no relabeling of fallback as success.
5. Stop and report the result. Do not automatically continue into stage 2.

The manifest lists per-method behavioral checks. Stage 1 checks are implemented
in the separate reference roundtrip harness; stages 2/3 remain planned. `probe.py` only validates input identity, inventory,
IL evidence and reported lowering status. Neither its ordinary exit 0 nor its
stage status gate proves semantic equivalence or readable/recompilable output.

## Important findings before widening scope

- At corpus setup, all ten primary methods fell back. All are instance methods and hit
  the early instance/implementation gate. Their `.impl-flags` are zero; the
  current combined diagnostic is not evidence of unusual method flags.
- Removing the instance gate would not be enough: fields, array categories,
  conversions, constructors and generic MemberRef/TypeSpec signatures still need
  real implementation.
- `MersenneTwister.Generate` uses `ldelema`, `dup`, `ldind.u4`, `stind.i4` for an
  array update. A safe managed-reference subset is therefore needed for stage 2;
  this cannot be disguised as ordinary scalar array syntax.
- RingBuffer fields are MemberRefs on a constructed generic declaring type, and
  `Contains` calls `IComparer<T>.Compare` through `callvirt`. Merely printing `T`
  does not implement these semantics.
- The selected primary methods have no EH clauses. EH, floating point, vector
  structs, async/iterator reconstruction, world simulation, UI/rendering and
  whole-project export remain outside this first delivery.

If stage 1 requires a substantial redesign, stop and explain the concrete seam
and alternatives. Do not silently replace these methods with easier getters,
hardcode OpenRA tokens, change the input DLL, or broaden the feature list.

## Reproduce locally

Run from the repository root. The existing local corpus is under
`rust/report/openra/`. To populate a fresh copy on macOS:

```sh
mkdir -p rust/report/openra/assemblies
curl --fail --location --retry 2 \
  https://github.com/OpenRA/OpenRA/releases/download/release-20250330/OpenRA-release-20250330.dmg \
  -o rust/report/openra/OpenRA-release-20250330.dmg
shasum -a 256 rust/report/openra/OpenRA-release-20250330.dmg
# Must be: 2a78cd58603fd06ed6006ae5916065455a7ac7c1290dba1d06c0292bad4238ab
```

Mount only after matching the hash. Use a private mount point; do not launch apps:

```sh
mountpoint=$(mktemp -d)
hdiutil attach rust/report/openra/OpenRA-release-20250330.dmg \
  -readonly -nobrowse -noautoopen -mountpoint "$mountpoint"
cp "$mountpoint/OpenRA - Red Alert.app/Contents/MacOS/arm64/OpenRA.Game.dll" \
  rust/report/openra/assemblies/
cp "$mountpoint/OpenRA - Red Alert.app/Contents/MacOS/arm64/OpenRA.Mods.Common.dll" \
  rust/report/openra/assemblies/
hdiutil detach "$mountpoint"
rmdir "$mountpoint"
```

For the four source references, downloading raw files from the pinned commit is
sufficient; fetching/extracting the whole source archive again is unnecessary:

```sh
python3 - <<'PY'
import hashlib, json, pathlib, urllib.request
manifest = json.loads(pathlib.Path('rust/acceptance/openra/manifest.json').read_text())
for path, expected in manifest['sources'].items():
    url = f"https://raw.githubusercontent.com/OpenRA/OpenRA/{manifest['commit']}/{path}"
    with urllib.request.urlopen(url, timeout=60) as response:
        data = response.read()
    if hashlib.sha256(data).hexdigest() != expected:
        raise SystemExit(f'Hash mismatch: {path}')
    dest = pathlib.Path('rust/report/openra/source') / path
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(data)
PY
```

Build our CLI and inspect without executing OpenRA:

```sh
(cd rust && cargo build --release --locked)
python3 rust/acceptance/openra/probe.py
python3 rust/acceptance/openra/probe.py --require-stage 1
```

- Default inspection: 0 means provenance/inventory/output checks passed; the
  report still records each method's actual lowering status.
- `--require-stage 1`: now **exits 0** for lowering status. Run the separate
  behavioral gate in [STAGE1.md](STAGE1.md) before claiming stage completion.
- Missing/mismatched inputs, diagnostics, decode errors and failed methods exit
  1. Nothing downloads or silently skips within this probe.
- Detailed outputs are written to ignored `rust/report/openra/current/`.
- `--require-stage 2/3` also requires earlier stages and relevant support methods;
  these options measure status, not authorization or behavioral correctness.

Baseline: [BASELINE.md](BASELINE.md).
