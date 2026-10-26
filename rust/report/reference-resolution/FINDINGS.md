# Why "type binding requires a resolved assembly" still blocks external calls

Read-only investigation. No corpus run, no decompiler CLI invocation, no edits outside this file.
All numbers below were recomputed from the frozen artifacts by re-deriving each image's own
Assembly identity and its AssemblyRef rows with a scratch ECMA-335 reader
(`/tmp/refscan/pe.py`), then replaying `corpus.py`'s selection rules verbatim.

**Reconstruction is faithful.** Replaying `select_references` over the inventory reproduces the
per-image stamps of `report/boxing/corpus-scan/images/*/summary.json` for 611 / 614 managed
images (the three differences are exactly the images where `identities` itself errors: the two
`Mono.Cecil-net45/.../*.netmodule` files and `dotnet/legacy/System.EnterpriseServices.Wrapper.dll`),
and reproduces every total in `acceptance/corpus/EXTERNAL_REFERENCES_PLUMBING.md`: managed 614,
distinct identities 496, identities owned by >1 file 87, declared 3,010, resolved 1,179,
unresolved 1,399, ambiguous 432, images with >=1 selected 313.

**One-line recommendation: new code, not a configuration change.** Flipping
`--reference-version-policy same-major` is safe to try but provably partial (it leaves 155
unresolved occurrences, creates 94 new ambiguities, and 93 of its 533 substitutions are
downgrades the CLI itself rejects), and it does not touch the dominant remaining blocker at all:
references the scanned image never declares (transitive forwarder refs of the supplied
references), which account for 4,244 of the 20,899 failing bodies and are invisible to
`corpus.py`'s selection by construction.

---

## 1. Are the 362 skipped reference-only snapshots in the identity index?

No. The index is built over the **scan targets only**, and the 362 reference-only inputs were
removed from the inventory before `corpus.py` ever sees it.

`rust/acceptance/corpus/corpus.py` (461-478):

```python
def baseline(args):
    started = time.monotonic()
    source = json.loads(args.inventory.read_text())
    ...
    managed = [i for i in source["images"] if i["kind"] == "managed"]
    owned_identities, declared_references = ({}, {})
    if args.references == "declared":
        owned_identities, declared_references = build_identity_index(
            cli, [image["path"] for image in managed], args.timeout)
```

`args.inventory` is `report/core-type-roots/scope-policy/implementation-inventory.json`, whose
header states the restriction explicitly:

```json
"scope": "implementation_only",
"source_inventory": ".../report/core-type-roots/inventory/inventory.json",
"excluded_reference_ids": [165, 166, ..., 779]        // 362 ids
```

and `report/core-type-roots/scope-policy/SCOPE.md`:

> Reference-only inputs are excluded under the user-approved goal revision.
> - Skipped reference snapshots: 362; original bodies: 327,380.
> - Remaining managed snapshots: 614; snapshots with CIL bodies: 319.

`build_identity_index` (291-308) then indexes exactly the paths it is handed, and
`select_references` (322-345) can only ever answer from that index:

```python
def select_references(owned, declared, own_path, policy):
    """Resolve each declared reference to exactly one corpus file, or report why not."""
    for identity in declared:
        candidates = [path for path in owned.get(identity, []) if path != own_path]
        if len(candidates) == 1: selected.append(candidates[0]); continue
        if not candidates and policy == "same-major": ...
        (ambiguous if candidates else unresolved).append(identity)
```

`unresolved` = "no file in the index owns this exact identity string"; `ambiguous` = "two or more
index files own it". Both are stored only as **counts** in the resume stamp built at
`scan_one` (365-372) and reported at (438-448) -- the identity strings themselves are never
persisted outside `stderr.log`, and `stderr.log` only echoes the references actually *passed*
(`src/main.rs:29`, `reference=<token> <identity>` per declared reference), so the unresolved
identities are not recoverable from the stamps alone.

### What that restriction costs, measured

| index | resolved | unresolved | ambiguous | (over the 319 body images, 2,226 declared refs) |
|---|---:|---:|---:|---|
| frozen (614 scan targets) | 1,058 | 782 | 386 | what the stamps record |
| if the 362 were added | 1,063 | 763 | 400 | +5 resolved, -19 unresolved, **+14 ambiguous** |

Over all 614 managed images: 96 declared identities (105 of 3,010 declared references) exist
*exactly* but only inside a skipped reference-only input -- e.g.
`Microsoft.CSharp 4.0.0.0` -> `dotnet/legacy/Microsoft.CSharp.dll`,
`System.Windows.Forms 4.0.0.0`, `System.Data.Entity 4.0.0.0`, `System.Data.Linq 4.0.0.0`,
`System.Web.Extensions 4.0.0.0`. These are the only identities the skip actually starves, and the
skipped family is precisely the .NET Framework 4.0 reference set that the corpus keeps
declaring.

**Indexing them is not the fix.** `dotnet/legacy/mscorlib.dll` is `mscorlib 4.0.0.0` and
`dotnet/legacy/System.dll` is `System 4.0.0.0` -- the two largest *ambiguous* identities (97 and
52 body images). Adding the 362 turns each 3-file ambiguity into a 4-file ambiguity and adds no
resolution for `System.Resources.ResourceManager 4.0.0.0` (legacy is 4.0.1.0). It costs
ambiguity (+14 occurrences) to buy 19 unresolved occurrences, and it contradicts the
user-approved implementation-only scope (`report/core-type-roots/scope-policy/SCOPE.md`,
`acceptance/corpus/reference-exceptions.json`).

---

## 2. Top unresolved declared identities across body images

Body-failure ground truth first: across the 319 body images, **20,899 of 473,770 Partial bodies
(4.4%) fail with a reason containing `type binding requires a resolved assembly`**. That total
splits three ways, which matters because only two of them are `corpus.py`'s business:

| class | bodies | meaning |
|---|---:|---|
| unresolved (declared, no file in index) | 11,219 | what `select_references` reports as `unresolved` |
| ambiguous (declared, >=2 files in index) | 5,436 | what `select_references` reports as `ambiguous` (nothing is passed, so the CLI sees the same error) |
| transitive (not declared by the scanned image at all) | 4,244 | the identity comes from inside a *supplied* reference (type forwarders), which `corpus.py` never selects references for |

### By number of body images declaring it (exact policy)

The stamps never store identity strings, so this is aggregated from the reconstructed declaration
+ index replay. `unresolved(200 distinct / 782 occurrences)`, `ambiguous(30 distinct / 386)` over
body images.

| # | identity (declared) | imgs | bodies failing | class | exact identity present anywhere? |
|---|---|---:|---:|---|---|
| 1 | `mscorlib 4.0.0.0` `b77a5c..` | 97 | 1,155 | ambiguous | yes, 3 files (netcore-2.2 ref, mcs/5.23 impl, net5.0 ref) |
| 2 | `System 4.0.0.0` `b77a5c..` | 52 | 39 | ambiguous | yes, 3 files (same three families) |
| 3 | `System.Core 4.0.0.0` `b77a5c..` | 47 | 19 | ambiguous | yes, 3 files |
| 4 | `nunit.framework 3.2.1.0` `2638cd..` | 33 | 1,442 | ambiguous | yes, 5 files (addins/net-3.5/net-4.0/net-4.5/portable) |
| 5 | `System.Resources.ResourceManager 4.0.0.0` `b03f5f..` | **25** | **8,684** | unresolved | **no** (index has 4.1.1.0) |
| 6 | `System.Runtime 4.0.0.0` `b03f5f..` | **24** | 598 | unresolved | **no** (index has 2.5.19.0, 2.6.10.0, 4.2.1.0) |
| 7 | `System.Xml 4.0.0.0` `b77a5c..` | 22 | 37 | ambiguous | yes, 3 files |
| 8 | `System.Linq 4.0.0.0` `b03f5f..` | **20** | 0 | unresolved | **no** (index has 4.2.1.0) |
| 9 | `System.Collections 4.0.0.0` `b03f5f..` | 19 | 2 | unresolved | no (4.1.1.0) |
| 10 | `System.Diagnostics.Debug 4.0.0.0` `b03f5f..` | 17 | 0 | unresolved | no (4.1.1.0) |
| 11 | `System.Security.Principal.Windows 4.1.1.0` `b03f5f..` | 17 | 11 | unresolved | no (4.1.1.**1**) |
| 12 | `System.Runtime.Extensions 4.0.0.0` `b03f5f..` | 16 | 14 | unresolved | no (4.2.1.0) |
| 13 | `System.Threading 4.0.0.0` `b03f5f..` | 16 | 7 | unresolved | no (4.1.1.0) |
| 14 | `System.Reflection.Extensions 4.0.0.0` `b03f5f..` | 15 | 0 | unresolved | no (4.1.1.0) |
| 15 | `System.Buffers 4.0.2.0` `cc7b13..` | 14 | 0 | unresolved | no (4.0.1.0, 4.0.3.0) |

**Top three unresolved: `System.Resources.ResourceManager 4.0.0.0` (25 images), `System.Runtime
4.0.0.0` (24), `System.Linq 4.0.0.0` (20).** For all three a file with that *exact* identity exists
nowhere in the inventory -- not in the index, not in the 362 skipped inputs.

### By bodies actually blocked (the ranking that matters)

| identity | bodies | class | why nothing was passed |
|---|---:|---|---|
| `System.Resources.ResourceManager 4.0.0.0` | 8,684 | unresolved | only 4.1.1.0 in index |
| `System.Private.CoreLib 4.0.0.0` `7cec85..` | 2,752 | **transitive** | declared by the *supplied* `System.Runtime`/`System.Collections` facades, not by the scanned image; the index does own `dotnet/netcore-2.2/System.Private.CoreLib.dll` with that exact identity |
| `Microsoft.CodeAnalysis 3.11.0.0` | 1,957 | ambiguous | 2 identity-equal files (net472 + netcoreapp3.1 bincore) |
| `nunit.framework 3.2.1.0` | 1,442 | ambiguous | 5 identity-equal files |
| `mscorlib 4.0.0.0` | 1,155 | ambiguous | 3 identity-equal files |
| `System.Runtime 4.0.20.0` | 893 | unresolved | no 4.0.20.0 (nearest 4.2.1.0) |
| `System.Runtime 0.0.0.0` | 734 | transitive | facade-internal ref, no such identity anywhere |
| `System.Private.Xml 4.0.1.0` `cc7b13..` | 601 | transitive | owner of that identity *is* in the index (`dotnet/netcore-2.2/System.Private.Xml.dll`) |
| `System.Runtime 4.0.0.0` | 598 | unresolved | only 4.2.1.0 |

Two mechanically different problems are visible here: **(a)** 5,436 bodies blocked by ambiguity
(the corpus owns the file, twice or three times), **(b)** 4,244 bodies blocked by a reference the
scan-target image never declares, of which 3,372 name an identity the index already owns.

---

## 3. For the top identities, is a file present under a different version?

Yes for almost every unresolved identity -- the gap is a version, not a name. Declared vs
available (index), plus what the skipped reference-only family would add:

| declared identity | available in index (version -> file) | in skipped 362 | same-major candidate count |
|---|---|---|---|
| `System.Resources.ResourceManager 4.0.0.0` `b03f5f..` | 4.**1.1**.0 -> `dotnet/netcore-2.2/System.Resources.ResourceManager.dll` (ref assembly, 0 method rows) | 4.0.**1**.0 -> `dotnet/legacy/...`; 5.0.0.0 -> `ref/net5.0/...` | 1 |
| `System.Runtime 4.0.0.0` | 4.**2.1**.0 -> netcore-2.2; 2.5.19.0 -> `Mono.Cecil-net45/Resources/assemblies`; 2.6.10.0 -> `NUnit/net-4.0` | 4.1.2.0 -> legacy; 5.0.0.0 -> net5.0 ref | 1 |
| `System.Linq 4.0.0.0` | 4.**2.1**.0 -> netcore-2.2 | 4.1.2.0 -> legacy; 5.0.0.0 -> net5.0 ref | 1 |
| `System.Collections 4.0.0.0` | 4.**1.1**.0 -> netcore-2.2 | 4.0.11.0 -> legacy; 5.0.0.0 -> net5.0 ref | 1 |
| `System.Diagnostics.Debug 4.0.0.0` | 4.**1.1**.0 -> netcore-2.2 | 4.0.11.0 -> legacy | 1 |
| `System.Security.Principal.Windows 4.1.1.0` | 4.1.1.**1** -> netcore-2.2 | -- | 1 |
| `System.Runtime.Extensions 4.0.0.0` | 4.**2.1**.0 -> netcore-2.2 | 4.1.2.0 -> legacy | 1 |
| `System.Threading 4.0.0.0` | 4.**1.1**.0 -> netcore-2.2 | 4.0.11.0 -> legacy | 1 |
| `System.Buffers 4.0.2.0` `cc7b13..` | 4.0.**1**.0, 4.0.**3**.0 | 5.0.0.0 -> net5.0 ref | 2 |
| `mscorlib 2.0.5.0` `7cec85..` (Win8/monotouch contract) | none | none | 0 |
| `System.Private.CoreLib 4.0.0.0` `7cec85..` | **exact** -> `dotnet/netcore-2.2/System.Private.CoreLib.dll` | -- | (never selected: transitive) |

Pattern: the missing half of the corpus is the **.NET Framework 4.0 reference set** the builds
were compiled against, and what the index holds for those names is the **netcoreapp 4.1/4.2
facade family** (and the net5.0 ref pack, which is 5.0.0.0 -- a different *major*, so even
`same-major` cannot use it).

Notable: the netcore-2.2 and net5.0 reference assemblies have `method_rows: 0`
(`implementation-inventory.json` ids 606, 633, 498). They are metadata-only facades -- which is
exactly the fact surface (types, fields, enum storage, member signatures) the decompiler asks
for, and no method body that could be executed.

---

## 4. Would `--reference-version-policy same-major` resolve them?

Partly, and it is not free.

Measured over the 319 body images (declared 2,226):

| policy | selected | unresolved (occ) | ambiguous (occ) |
|---|---:|---:|---:|
| `exact` | 1,058 | 782 (200 identities) | 386 (30 identities) |
| `same-major` | 1,591 | **155** (94 identities) | **480** (56 identities) |

So same-major clears 627 unresolved occurrences and creates 94 new ambiguous ones -- net +533
selected. It removes the top *unresolved* identities (`System.Resources.ResourceManager 4.0.0.0`,
`System.Runtime 4.0.0.0`, `System.Linq 4.0.0.0` are all single-candidate under same-major) but does
**nothing** for the ambiguous half (5,436 bodies) and nothing for the transitive 4,244.

### Two correctness problems in `corpus.py`'s same-major, versus the CLI's

1. **No nonolder filter.** `corpus.py:328-337` matches on `name + token + major` only, so it may
   select an *older* image. The CLI's own rule (`src/dependencies.rs:611-648`,
   `src/dependencies/binding.rs:181-206`) requires `requested.version <= identity.version`:

   ```rust
   .filter(|(_, _, identity)| {
       requested.same_contract_scope(identity)
           && requested.version[0] == identity.version[0]
           && requested.version <= identity.version
   })
   ```

   `matching_modules` therefore filters the downgraded candidate out and the CLI returns
   `Err("type binding requires a resolved assembly: ...")` anyway. Of the 533 same-major
   substitutions, **93 are downgrades** (e.g. declared `System.Runtime 4.2.2.0` -> selected
   4.2.1.0, six images; `System.Resources.ResourceManager 4.1.2.0` -> 4.1.1.0, six images), so a
   fifth of the "gained" resolution is inert. The corpus count `selected` therefore overstates
   resolution whenever corpus.py and the CLI disagree.
2. **Different uniqueness rule.** corpus.py requires exactly one match across all same-major
   versions; the CLI picks the minimum nonolder version and then requires uniqueness *within that
   version*. Those two rules classify a few groups differently (corpus.py calls a group ambiguous
   where the CLI would substitute, and vice versa).

### Semantic risk of substituting a different assembly version as a *fact source*

The substituted image is only parsed, never executed, so the risk is not code execution -- it is
**silently wrong facts**, because the fact surface is exactly what drifts between versions:

- **Enum storage width.** `DependencyContext::external_value_uncached`
  (`src/dependencies.rs:354-382`) reads `value_definition_kind` and `checked_enum_type` from the
  referenced image and returns `ExternalValue::Enum(enum_type, name)`; the width then drives load
  extension and store truncation (`acceptance/corpus/EXTERNAL_ENUM_STORAGE.md`). A version whose
  `value__` is a different integer width produces *decompiled output that looks fine and is
  wrong*, with no error and no marker in the per-method status.
- **Struct whole-value copy admission.** `external_value_definition`
  (`src/dependencies.rs:316-345`) admits a struct copy from a referenced definition; a version
  that added/removed a field changes the admission decision, not just the text.
- **Member signatures / overload selection.** A newer version that adds an overload can change
  which member the call site binds to, i.e. the emitted call target, while the body still reports
  `Decompiled`.
- **Identity exactness is only partly enforced.** The binder does compute `exact_identity`
  (`src/dependencies/binding.rs:254-281, 374`) and three presentation paths refuse facts when it
  is false (`src/dependencies/binding/presentation.rs:23, 112, 216`). But
  `external_value_uncached` and `external_value_definition` never consult `exact_identity`, so
  enum storage / struct admission facts *are* consumed from a substituted image with no
  exactness gate.

Net: same-major is an explicit, recorded substitution (`AssemblyResolution::Substituted`,
`src/dependencies.rs:110, 645`), and it is honest about the version it used -- but it is not
evidence-equivalent, and the output has no per-body marker distinguishing "fact from 4.0.0.0" from
"fact from 4.1.1.0". Enabling it in the frozen acceptance baseline would weaken the evidence
claim for the 533 (best case 440 effective) selections it changes, while leaving 5,436 + 4,244
bodies untouched.

---

## 5. Is any ambiguity avoidable by exact version?

No. Under `exact` policy, ambiguity arises only when two or more index files own the **identical**
identity string, so by construction every candidate has the same name, version, culture,
public-key token and ContentType -- there is no version to prefer. Excerpts:

| identity | candidates (all identity-equal) | avoidable by version? |
|---|---|---|
| `mscorlib 4.0.0.0` `b77a5c..` | `dotnet/netcore-2.2/mscorlib.dll` (ref), `mcs/5.23/lib/mono/4.5/mscorlib.dll` (impl, 23,889 CIL), `ref/net5.0/mscorlib.dll` (ref) | no |
| `System 4.0.0.0` / `System.Core 4.0.0.0` / `System.Xml 4.0.0.0` | same three families | no |
| `nunit.framework 3.2.1.0` | 5 files: `addins/tests`, `net-3.5`, `net-4.0`, `net-4.5`, `portable` | no |
| `Microsoft.CodeAnalysis 3.11.0.0` | `tasks/net472/...`, `tasks/netcoreapp3.1/bincore/...` | no |
| `System.Collections.Immutable 5.0.0.0` | `tasks/net472/...`, `tasks/netcoreapp3.1/bincore/...` | no |

The ambiguity is *content* ambiguity (different builds of one identity), which only a tie-break
outside the identity key can settle -- e.g. prefer the candidate supplied in the same directory
family / same TFM as the scanned image, or prefer the reference assembly over an implementation
when the reference must not be executed. Note the CLI cannot do this itself: two identity-equal
candidates produce `matching_modules(...) == [a, b]` ->
`Err("type binding has conflicting assembly definitions")` (`src/dependencies/binding.rs:220-225`),
so `corpus.py` refusing to pass anything is currently the only safe behaviour, and it is what
turns 5,436 bodies into `requires a resolved assembly` failures.

Under `same-major`, ambiguity *does* become partly version-caused (480 occurrences, 56 identities:
multiple same-major versions match and corpus.py requires a single match), so it is the policy --
not the version -- that creates those.

---

## Recommendation

**New code.** Configuration (`--reference-version-policy same-major`) is a reasonable experiment
but cannot be the answer: it leaves 155 unresolved occurrences, adds 94 ambiguities, and its
`corpus.py` implementation disagrees with the CLI on the nonolder and uniqueness rules so ~93 of
its 533 substitutions are no-ops. Ranked by bodies unblocked per unit of risk:

1. **One-hop reference closure (new selection code).** After resolving an image's declared
   references, also pass the references *those* files declare (one hop, non-recursive, still
   identity-selected by the same rules). This is the only thing that can reach the 4,244
   transitive bodies -- 2,752 of them from `System.Private.CoreLib 4.0.0.0`, whose exact identity
   the index already owns as `dotnet/netcore-2.2/System.Private.CoreLib.dll`. Nothing about the
   evidence changes: still exactly one file per identity, still no guessing, and the selection is
   already recorded in the resume stamp.
2. **Ambiguity tie-break with recorded provenance (new selection code).** 5,436 bodies wait on
   `mscorlib`/`nunit.framework`/`Microsoft.CodeAnalysis` having >1 identity-equal file. A declared,
   deterministic tie-break (prefer the candidate in the scanned image's own directory family, else
   a reference assembly over an implementation) unblocks them without touching the identity rule;
   it must be stamped per image so the choice is auditable.
3. **Same-major as configuration only after (a) aligning `corpus.py` with the CLI's nonolder +
   per-version-uniqueness rule and (b) gating fact consumption on `exact_identity`**, so a
   substituted image can never silently supply an enum width or struct layout. Until then
   same-major weakens the evidence claim for a 155-occurrence gain that item 1 largely subsumes.
4. **Do not index the 362 skipped inputs.** It costs +14 ambiguous occurrences to buy 19
   unresolved ones, adds a 4th file to `mscorlib 4.0.0.0`/`System 4.0.0.0`, and contradicts the
   approved implementation-only scope.

## Code references

| file | lines | content |
|---|---|---|
| `rust/acceptance/corpus/corpus.py` | 291-308 | `build_identity_index` -- index built from the paths handed in, via `ilspy-lite identities` |
| `rust/acceptance/corpus/corpus.py` | 310-319 | `parse_identity` -- name / version / token only; culture and ContentType ignored for same-major matching |
| `rust/acceptance/corpus/corpus.py` | 322-345 | `select_references` -- exact then same-major; `unresolved` / `ambiguous` definitions |
| `rust/acceptance/corpus/corpus.py` | 355-372 | `scan_one` -- selection from `_REFERENCE_INDEX`, resume stamp |
| `rust/acceptance/corpus/corpus.py` | 386-391 | the `--no-auto-references` + `--reference` command line actually run |
| `rust/acceptance/corpus/corpus.py` | 438-448 | per-image record: `declared`/`selected`/`unresolved`/`ambiguous` counts + CLI `resolution` |
| `rust/acceptance/corpus/corpus.py` | 461-478 | `baseline` -- `managed` from the implementation-only inventory; index build |
| `rust/acceptance/corpus/corpus.py` | 530-534 | `--references` / `--reference-version-policy` defaults (`none` / `exact`) |
| `rust/src/dependencies/binding.rs` | 181-206 | `matching_modules` -- same-major filter `requested.version <= identity.version` |
| `rust/src/dependencies/binding.rs` | 215-226 | `binding_assembly` -- the `requires a resolved assembly` / `conflicting assembly definitions` errors |
| `rust/src/dependencies/binding.rs` | 254-281, 374 | where `exact_identity` is computed and propagated |
| `rust/src/dependencies/binding/presentation.rs` | 23, 112, 216 | the only three sites that refuse a fact when `exact_identity` is false |
| `rust/src/dependencies.rs` | 316-345 | `external_value_definition` -- struct whole-value copy from a referenced image (no exactness gate) |
| `rust/src/dependencies.rs` | 354-382 | `external_value_uncached` -- external enum storage / struct kind (no exactness gate) |
| `rust/src/dependencies.rs` | 611-648 | `resolve` -- CLI's same-major substitution, nonolder + unique-per-version |
| `rust/src/dependencies.rs` | 729-748 | `image_identity` -- the `identities` subcommand's contract (errors for netmodules) |
| `rust/src/main.rs` | 26-46, 88-111 | `--reference-version-policy` parsing; `identities` output |
| `rust/report/core-type-roots/scope-policy/SCOPE.md` | 1-20 | the implementation-only scope decision and skip counts |
| `rust/acceptance/corpus/reference-exceptions.json` | 1-25 | the one approved exception (`dotnet/legacy/mscorlib.dll`, 20,854 zero-byte CIL stubs) |
| `rust/acceptance/corpus/EXTERNAL_REFERENCES_PLUMBING.md` | 18-45 | selection semantics + the 3,010 / 1,179 / 1,399 / 432 table (independently reproduced) |

## Residual risks / open questions

- The 782 / 155 / 480 figures are **selection-level** counts, recomputed statically. No run in
  the repo pairs the *same* CLI binary with `exact` and `same-major`, so the actual status delta
  for same-major is unmeasured (`report/reference-policy/corpus-scan-same-major{,-2}` are two
  different pre-boxing binaries, `Decompiled` 274,441 and 282,330, versus 296,978 for the current
  `exact` run -- not comparable).
- The 20,899 body count is the *upper bound* on what any reference fix can recover: resolution
  only removes the first blocker; the next reason (facts, constructed members, `callvirt` tails)
  may still stop those bodies.
- My scratch reader cannot parse `Mono.Cecil-net45/Resources/assemblies/noblob.dll` (no `#Blob`
  heap) and reads no Assembly row for the two netmodules and
  `dotnet/legacy/System.EnterpriseServices.Wrapper.dll`; the CLI's `identities` fails on the same
  four, and every documented total still reproduces exactly.
- `System.Private.CoreLib` is in the CLI's core-contract name list
  (`src/dependencies.rs:700-708`) yet resolves only if explicitly supplied; the closure in
  recommendation 1 must not special-case it, but its 2,752 bodies are the single largest
  transitive win.
