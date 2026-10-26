# Closed generic reference signatures

This is a bounded type-system milestone under the approved IL/CFG + lift design.
It does not complete generic member substitution, value types, or the per-image
<= 1% implementation recovery goal.

## Representation and admitted operations

`signature_types.rs` interns complete, canonical closed type signatures behind
small Copy TypeId handles. Identity includes the parsed input arena and the full
structural key: class/value tags, definition token and every recursive argument.
A TypeRef retains its actual metadata scope. Exact current-module TypeRefs use
the existing local-definition resolver; there is no string-based second binder.
Separate inputs never share TypeId identity, even with equal numeric tokens or
MVID labels. Primitive Ty forms remain unchanged.

The first structural stack type is a reference GenericInst. Arguments can contain
primitive, nominal, nested closed generic and unmodified vector shapes, including
value-type arguments. `List<int?>` is a reference copy; it does not admit a nullable
value on the evaluation stack, its layout, or its operations. Free VAR/MVAR,
modifiers, pointer/function-pointer arguments and multidimensional shapes remain
unsupported. Known local definition arity and class/value kind are checked.

The reference can be copied through arguments, locals, ordinary instance/static
fields, returns and supported direct calls. Null assignment and reference-to-object
upcasts retain identity. Different constructed types are not implicitly assignable;
variance, inheritance specialization and constructed members require their own
facts and are not inferred. Unsupported instructions, EH, receiver, readonly,
initialization and call checks remain in force.

Opaque `__cil_ref_HASH` notation denotes the exact signature-scoped reference
identity, not a wrapper object or an allocation. Hashes only give compact names:
semantic equality compares full canonical keys, and alias collisions are rejected.
Both presentations retain the alias's canonical type-spec bytes and description.
These bytes identify metadata tokens in the original input; they are not an IL
body, an execution proxy or a claim of ordinary C# source binding.

Raw method, field and plain-local signatures are checked before cildec token
construction. Oversized RIDs cannot mask to valid tokens; trailing bytes cannot
silently disappear. Pinned/byref/modified locals remain rejected before the
plain-local checker. These checks do not constitute a full CLR validity verifier.
The structural encoder limits depth/work/bytes, and the image interner bounds the
number of structural identities. Open generic owners/variables are still pending.

## Owned verification

- The public decompilation E2E first failed at List<int> field exchange. All 15
  selected methods now recover in Debug and Release, with equal statuses in
  Details and Readable. Six valid but unsupported operation shapes stay Partial:
  constructed field access, dispatch, value stack transfer, generic methods, open
  owner variables and variance.
- Boundary tests reject differing generic arguments, local arity mismatch,
  oversized definition RIDs, and method/field/local signature suffixes. Static
  malformed images are never runtime-loaded.
- The owned behavior harness runs actual recovered statements, not synthesized
  bodies. Type aliases are independently decoded and resolved in the owned module;
  recovered parameter/return runtime types must exactly match the owned originals.
  Only type/field definitions are borrowed. Forward calls recovered LocalIdentity.
- 85 cases x Debug/Release x Details/Readable = **340 comparisons**. Results,
  reference identity, reachable fields, collection/array contents, static state,
  enumeration observations and exception types match.
- Four helper mutants are rejected through Forward. Four additional mutants
  reverse a list twice: final contents are unchanged, but the observable extra
  mutation invalidates enumeration and is rejected.
- The shared readable adapter has a red/green case for nested generic parameter
  commas; rebinding cannot split one type argument list into extra variables.
- Full offline Rust regression, all-target Clippy with warnings denied, rustfmt,
  seven readable-adapter checks and twelve corpus-tool E2E checks pass. Existing
  object behavior also passes: 444 comparisons and four rejected helper mutants.

The harness executes only newly authored fixtures and recovered carriers on the
existing .NET 10 installation. No original third-party input or archived compiler
was executed. Runtime equivalence for the actual corpus is not established by
these static gains; its authorization gate remains pending.

## Fixed-scope production results

The frozen inventory and reference exclusions are unchanged: 614 implementation
snapshots, 319 with target bodies, 770,748 target CIL bodies. No acquisition,
extraction, inventory regeneration or input hash sweep was repeated.

| Metric | Core-root baseline | Closed-generic references |
|---|---:|---:|
| Recovered | 150,791 | 155,281 |
| Partial | 619,957 | 615,467 |
| Aggregate unrecovered | 80.44% | 79.85% |
| Passing body-bearing images | 33 / 319 | 33 / 319 |
| Incomplete image scans | 0 | 0 |

There are **4,490 new recoveries across 127 inputs**, with **zero previously
recovered statuses regressing**. The cumulative genuine gain against the original
baseline projected to this same implementation scope is 20,247. The first-error
signature bucket shrinks from 226,683 to 187,068; only fully recovered bodies count
as gains, not the much larger number exposing a later blocker.

OpenRA.Game gains 33: 888 recovered / 3,879 Partial out of 4,767 (81.37% unrecovered).
OpenRA.Mods.Common gains 91: 5,324 recovered / 13,083 Partial out of 18,407 (71.08%).
The full per-input/method delta is retained, including archive origins and tokens.
The release incremental build took 4.83 seconds; the four-worker scan took about
15 seconds locally. These counts are static recovery coverage, not per-method
runtime proof or completion of the <= 1% gate.

## Reproduction and evidence

```sh
python3 rust/tests/build-generic-reference-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test generic_references
python3 rust/tests/generic-reference-roundtrip.py --mutation-control
python3 rust/tests/generic-reference-roundtrip.py --readable --mutation-control
```

The driver uses an existing CLI and cached owned compilations. Use `--cli` to
choose an already-built binary. The independent scope inventory remains
`rust/report/core-type-roots/scope-policy/implementation-inventory.json`.

Evidence under `rust/report/generic-reference-roundtrip/` includes the failing
E2E, successful boundaries/full regression, behavior/mutation logs, Clippy,
`corpus-scan/baseline.json` and `corpus-delta.json`. The latter compares exactly
the same target ids, origins, body counts and MethodDef tokens with the saved
core-root statuses. No historical method statuses or reference counts were edited.
