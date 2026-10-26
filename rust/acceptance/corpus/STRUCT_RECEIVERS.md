# Struct receivers and bounded managed locations

This slice admits value-type instance methods and the managed locations they
need. It does not add a general pointer model, allocation or exception regions.

## Representation and transfer rules

- Ty::ValueAddress(TypeId) is a managed location of one exact struct value type.
  It is a distinct stack category (Address), copies keep the same location, and
  it can never be stored as an ordinary value (no address escape).
- Ty::Value on the stack is still a copy. A field read through a copy is allowed
  because it only reads that copy; a field write through a copy is rejected
  because it would mutate a temporary.
- A value-type instance method receives ValueAddress, so `this` is the caller's
  storage. Field reads/writes through it mutate that storage, matching CLI
  managed-pointer semantics without inventing a hidden copy.
- `ldarga`/`ldloca` acquire the location of an admitted struct argument/local.
  `initobj` zero-initializes that referenced storage in place (no constructor).
  `ldflda` yields the location of a struct or integer field. Integer locations
  reuse the existing address model (`ldind.i4`/`ldind.i8`, `stind.i4`/`stind.i8`),
  now with I8/U8 address categories.
- A struct instance `call` requires the exact location type and renders the
  receiver argument with an explicit `ref`, so a call site cannot silently pass
  a copied value.
- Addresses still cannot cross a basic-block edge, byref/modified parameters and
  returns stay unsupported, and ref structs, external/generic value types, arrays
  of struct elements and exception regions remain independent gates.

## Verification

`rust/tests/struct_receivers.rs` drives the shared count-free contract: every
MethodDef must lower with identical admission in both views, each listed
`(owner, method, reason)` entry must keep its stated blocker, and stale entries
fail. Six fixture tests cover location mutation, argument-copy semantics, nested
field locations, bounded-address negatives and scan parity.

The owned behavior harness executes actual recovered bodies. A struct instance
method is lowered to a static carrier whose first parameter is `ref T`; the
harness passes a managed pointer into the same boxed storage for both the
original and the recovered body, so writes are compared on identical state.

- 16 recovered methods (Point/Line instance methods plus static location users),
  32 comparisons per configuration, run in Debug and Release and in Details and
  Readable presentation.
- Recovered signatures must match the original exactly, including the byref
  receiver; values, public static counters, all reachable struct fields and
  exceptions are compared.
- Three mutations drop one field store (SetX, Bump, Move). All are rejected, so
  the gate proves the write actually reaches the caller's storage.
- The promoted `UnsupportedDefault`/`UnsupportedMember`/`UnsupportedNew` bodies
  from the previous slice are now covered by the value-copy harness as well.

Full offline Rust regression (51 suites), all-target Clippy with warnings denied
and rustfmt pass. Only authored fixtures and carriers execute.

## Unchanged-scope static result

Same 614 target managed snapshots, 319 body-bearing targets and 770,748 bodies.

| Metric | Struct-copy baseline | Struct receivers |
|---|---:|---:|
| Decompiled | 186,454 | 204,281 |
| Partial | 584,294 | 566,467 |
| Aggregate unrecovered | 75.81% | 73.50% |
| Passing body-bearing targets | 33 / 319 | 33 / 319 |
| Incomplete scans | 0 | 0 |

**17,827 newly recovered methods in 138 inputs**, zero previously recovered
status regressions. Cumulative genuine gain at the corrected common scope is
69,247. OpenRA.Game gains 179 (1,372 recovered); OpenRA.Mods.Common gains 24
(5,556 recovered). Later dispatch/exception/external-type blockers remain Partial.

## Reproduction

```sh
python3 rust/tests/build-struct-receiver-fixtures.py
cargo test --manifest-path rust/Cargo.toml --locked --offline --test struct_receivers
python3 rust/tests/struct-receiver-roundtrip.py --mutation-control
python3 rust/tests/struct-receiver-roundtrip.py --readable --mutation-control
```

Evidence: rust/report/struct-receiver-roundtrip/. The scan inventory remains
rust/report/core-type-roots/scope-policy/implementation-inventory.json.
