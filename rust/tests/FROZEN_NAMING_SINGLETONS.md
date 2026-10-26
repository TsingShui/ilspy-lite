# Frozen naming: singleton-owner fragments after preinitialization

Optional offline corrective test for exactly three restored bodies. This does not
approve the final naming goal or replace the separate iterator-constructor gate.
From the repository root, using the existing .NET 10 SDK/reference pack/runtime
in `~/.dotnet10` (or `DOTNET_ROOT`) and existing Cargo offline cache:

```sh
python3 rust/tests/frozen-naming-singletons.py \
  --corpus /Users/bytedance/zhome/ilspy-lite/rust/report/openra
```

The absolute corpus directory is read-only. It must already contain `assemblies/`,
`runtime-deps/`, and `random-classes/final-21c469ba6/*/class.json`. Nothing is
acquired, installed, restored, or copied into tracked fixtures. Cargo runs only
`build --release --locked --offline`; installed Roslyn compiles directly against
.NET 10 framework reference-pack DLLs, never an original assembly reference.

## Fixed identities and initialization boundary

The unchanged random20 selection, original assembly SHA-256 pin, MVID
`daac77a9-0bf7-4155-8229-f659cf68d982`, owner identities, MethodDef signatures,
and IL digests bind this gate to `OpenRA.Mods.Common.dll`:

| Selected body | Owner suffix | Oracle-only initializer | Own singleton |
|---|---|---|---|
| ctor `0x0600468C` | WithSpriteBodyInfo/<>c (`0x02000F97`) | `0x0600468B` | `0x04003BD9` |
| int method `0x0600468E` | WithSpriteBodyInfo/<>c (`0x02000F97`) | `0x0600468B` | `0x04003BD9` |
| ctor `0x06002C76` | DefaultSpriteSequence/<>c (`0x02000985`) | `0x06002C75` | `0x040024C3` |

An independent PEReader catalog, before any corpus assembly load, requires:

- No module initializer, original instance fields, generic owner or finalizer;
  the owner directly derives from the pinned core `System.Object`.
- The selected parameterless constructor is exactly an Object constructor direct
  call followed by return, without fields, extra calls or exception regions.
  Its MemberRef must resolve to the pinned `System.Runtime` 6.0.0.0 Object identity.
- The owner's sole initializer is **only** `newobj` its own verified Object-only
  constructor, `stsfld` its own readonly singleton of the owner type, then `ret`.
  No Actor, world, callback or other instruction is admitted. Any change fails.

Initializer SHA-256 pins are
`83ba9efb6fc607bc6e5e3b2de66c9e78021d2a33efd57cd23c43db0d3e68af8c`
and `4ab5764b580d3a5521d6e604d502481ddca1998d4d4a1d265d195de69d714b95`,
respectively. Raw IL remains only in ignored reports. CLI original IL bytes must
match PEReader; execution repeats the static inventory before load and compares
loaded original methods with it again.

`RuntimeHelpers.RunClassConstructor` runs the verified **original** initializer
as an oracle-only precondition. Its allocation of the selected original ctor is
allowed only on that reference side. It is not a permitted recovered leaf, and
no recovered initializer implementation is generated or claimed.

## Actual source and independent setup

Actual CLI default `--type` output supplies the method-local source spans. Details
is consumed only for statuses, checked against the independent metadata inventory
and frozen per-method baseline. All three must be newly Decompiled; none can be
silently excluded. Constructor declaration, `: base()` initializer and body are
retained byte-for-byte. The constant method's internal declaration and body are
also retained unchanged, including its encoded name.

The minimal instance shadow adapts only the private sealed owner to a public
sealed class inside an ordinary test context. Unused fields/methods and the
fallback static initializer are omitted. The singleton field's own-type spelling
is shortened to the shadow type, and readonly is removed so the host can install
its fixture. These adaptations and original spans are explicit in each job JSON.
The recovered assembly must contain only these shadow/context types, no type
initializers, and only framework references. Its compiled constructor must make
only the exact core Object call; its constant body has no fields or calls.

After original oracle setup, the host independently allocates a recovered
singleton and invokes the actual recovered constructor on it, then installs it
in the recovered fixture field. No original receiver, singleton, field value or
poststate is transferred. Positive comparisons start only after both setups
succeed. Each side snapshots and preserves its **own** singleton identity and
all its retained static fields across selected calls; unused original delegate
caches are also identity-snapshotted.

For each body, the postinit matrix has two fresh receiver pairs and two
constructor-warmed receiver pairs, each invoked three times, plus three calls on
the independently prepared own-singleton pair. These owners have no instance
fields, so warmed receivers are the only dirty-receiver variant; this is not a
claim about dirty instance-field equivalence. The constant adds three null-this
pairs. Bridges use exact CIL `call`, not `callvirt` or a wrapper null-entry check;
the original and recovered constant both actually return zero on null this.
Constructor null-this behavior is deliberately not claimed.

## Behavioral red controls and checked result

Every selected body has a separately compiled mutation, executed **before** the
unchanged positive shell. Compiler errors and ABI/protocol guard failures cannot
satisfy a control; exit 42 and the specific runtime mismatch are required.

- Each ctor mutation throws the exact
  `InvalidOperationException("frozen-singleton-ctor-mutation")` from the actual
  selected compiled ctor. Original oracle setup succeeds, then recovered
  singleton preparation invokes that mutated body and throws. This is labeled
  `phase=recovered-preinitialization`, `completedPairs=0`, with exact selected
  ctor identity and compiled IL. These negatives do **not** reach the postinit
  matrix; no alternate provider, fabricated singleton or copied prestate is used.
- The constant mutation changes the actual return to one. Its unchanged actual
  recovered ctor prepares the singleton successfully; direct selected comparison
  then detects `return-value` mismatch in `phase=fresh-receiver`.

Checked with the command above on base `987ab39bd`:

| Token | Positive paired calls | Runtime mutation exit | Phase |
|---|---:|---:|---|
| `0x0600468C` | 15 | 42 | recovered-preinitialization |
| `0x0600468E` | 18 (3 null-this) | 42 | fresh-receiver |
| `0x06002C76` | 15 | 42 | recovered-preinitialization |

Total: **three bodies, 48 positive paired calls, three specific runtime mutation
rejections**. Preinitialization and receiver-warming calls are not counted as
selected positive pairs.

Ignored `rust/report/frozen-naming-singletons/` contains commands and exit codes
(`commands.jsonl`), compiler response files, exact default/Details stdout,
PEReader inventories, frozen baselines, extracted wrapped source, and per-token
positive/mutation logs. Raw initializer evidence is in
`02-0x02000F97/0x0600468B-il.stdout.txt` and
`19-0x02000985/0x06002C75-il.stdout.txt`, as well as the corresponding inventory
and runtime result records. `summary.json` aggregates counts and phases; it is
removed at the beginning of a run so failures cannot leave stale success.

## Limits

This is bounded field-free fragment behavior **after independently verified
preinitialization**, not full startup or type-initializer equivalence. The
recovered singleton installation is test fixture setup, not recovered static
initializer code. Omitted unused fields, full class ABI/interfaces, reflection
alias identity, gameplay and other selected bodies remain outside this seam.
No production admission or frozen selection changes, and no final goal approval.
