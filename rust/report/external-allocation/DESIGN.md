# External allocation: implementation-ready design

Read-only design; nothing built, scanned or edited. Line numbers are the current tree. Scope:
`newobj` whose ctor is a `MemberRef` naming `.ctor` on a type in a referenced image. Current first
blocker (`external allocation needs dependency facts; only local classes and local instantiations
are admitted`): **24,488** bodies (`report/constructed-members/corpus-scan-demoted/baseline.json`,
`reasons`).

## 0. Where it stops today

`lift.rs:590-643` (`newobj`) -> `metadata.rs:2967-3035` (`allocation_constructor`), which for a
`MemberRef` calls `constructed_member_owner` (`metadata.rs:2901-2930`). That answers only for a
`TypeSpec` parent with a *local* definition; a plain `TypeRef` parent returns `Ok(None)`, and the
`ok_or` at `metadata.rs:2983` produces the row above. A `TypeSpec` parent with an external
definition dies earlier at `metadata.rs:2918` (`constructed member needs a local definition`, 12,104
bodies, a *different* row: section 9). Today the owner is therefore always local and no
defining-image fact is read.

## 1. Fact set needed from the defining image

| fact | why | current |
|---|---|---|
| `TypeDef`, neither `interface 0x20` nor `abstract 0x80`; `sealed 0x100` allowed | only concrete classes can be `newobj`'d; a static class is `abstract|sealed`, still refused by `0x80` | new |
| not a value type (base `ValueType`/`Enum`) | `newobj` on a struct ctor is value initialisation, not an identity | new; twin `value_definition_kind` (`metadata.rs:2320`) |
| owner identity: source, token, forwarders | the name must come from the defining image; its token must never meet an input-scoped one | new (`resolve_type`) |
| owner name, nested path joined/escaped | only that image can spell it | new; precedent `dependencies.rs:400` |
| declared generic arity, and whether the site supplies it | an open definition has no legal name; a mismatched instantiation must not borrow the fact | new (`bind_type_in`, `binding.rs:294-313`) |
| ctor exists, instance, not static/virtual/abstract, exact scoped signature | else the allocation runs a different member than the IL names | new (`resolve_method`, `binding/members.rs:223-320`) |
| signature shape: default conv, no vararg, void, `has_this` | same as the local path (`metadata.rs:3021-3027`) | new (`definition_signature`) |
| argument types in the **caller's** vocabulary | `assignable` at `lift.rs:596-601` needs them | there: the reference signature, what `metadata.rs:3020` reads |

Argument `Ty`s need no new `Image` surface either: they come from the caller-scoped `MemberRef`
signature through `member_parameter` (`metadata.rs:3279-3295`) - names from the definition, types
from the site.

## 2. Where the facts live; three-state rule

**One new lookup, not a widened `ExternalCall`**, which deliberately refuses ctors
(`dependencies.rs:288-290`, frozen table) and cannot carry two questions in one answer.

```rust
pub(crate) struct ExternalAllocation { owner_token: u32, owner_name: String, owner_sealed: bool,
    constructed: bool, arity: usize, parameter_count: usize, owner_source: String }
pub(crate) enum ExternalAllocationLookup { Fact(ExternalAllocation), Refused(String), Unavailable }
```

Cache: `Image::external_allocations: RefCell<BTreeMap<u32, Result<ExternalAllocation, String>>>`
beside `external_calls` (`metadata.rs:48-54`), accessors mirroring `metadata.rs:2301-2314`. Priming:
`prime_external_allocations` in `lib.rs`, called from `decompile_method` (`lib.rs:349-355`) beside
`prime_external_calls`; walk the body operands and ask once per `MemberRef` named `.ctor`. Priming,
not lazy resolution, is mandatory - the external-enum review proved order-dependent admission makes
scan and single-method decompilation disagree.

Exact three-state behaviour. `Fact(f)` caches `Ok(f)`; the arm renders. `Refused(r)` caches
`Err(r)`; the arm returns `external allocation needs dependency facts: {r}`, the
stable-prefix-plus-detail shape the external-call milestone adopted. `Unavailable` inserts
**nothing**: the lookup returns `None` and the arm returns today's exact string verbatim, so the
method stays `Partial` with **byte-identical text** to a context-free run (triggers: no context, or
a reference set without the owner's assembly; discipline as `lib.rs:248-252`,
`metadata.rs:2604-2620`). Errors before the defining image is reached are `Unavailable`; errors
after it are `Refused` - the split `external_value_definition` (`dependencies.rs:316-345`) already
makes.

## 3. Rendering

The emitted form stays the intrinsic `__cil_newobj_<TOKEN>(args...)`, `TOKEN` the caller's
`MemberRef` token. Only that form preserves the input-scoped ctor identity, which the local
milestone, its harness and its review protocol are built on
(`report/new-object-roundtrip/review-source.md:99`); and since `readable.rs` has no pass rewriting
`__cil_*` intrinsics, a bespoke `new Owner(...)` for external owners only would present one
instruction two ways and re-open the SUSPECT finding class of `CONSTRUCTED_MEMBERS.md` (a definition
name where an instantiation is needed).

The owner facts go in the evidence line above the intrinsic, `lift.rs:1119-1124` style:

```
// external allocation owner 0x0A000003: global::Lib.Sealed; class, sealed; one fresh identity
//   here, then .ctor(int32, string) runs in original argument order; field initialisers are
//   the defining image's, not visible here.
AllocCases.Node v3 = __cil_newobj_0A00001B(v0, v1);
```

Names come from `symbol_names::type_name(&definition_image, definition_token, true)`
(`symbol_names.rs:233-299`): segments joined by `.`, backtick stem stripped, keywords `@`-escaped,
`global::` prefix, so a nested owner reads `global::Lib.Outer.Inner`; an instantiation also prints
the caller-scoped argument identities from `ResolvedType.generic_arguments`, while the value
identity stays the interned `__cil_ref_...`. Fresh identity is unchanged: one `push` per `newobj`,
hence exactly one `T vN = __cil_newobj_TOKEN(...)`, no re-evaluation on later reads, and the
contract test asserts `occurrences("__cil_newobj_") == count(newobj)`.

## 4. Ordering, receiver, side effects

Receiver-free: only ctor arguments are popped, so no null check is added or dropped, and they are
popped in reverse then reversed (`lift.rs:596-605`), keeping IL order for the type checks and the
printed list. The reference signature is read with the caller image, so `assignable` compares
caller-scoped `Ty`s only and no defining-image token enters `Ty`. Result type: a `TypeRef` class
token gives `Ty::ExternalClass(token)` (`ir.rs:253`, `metadata.rs:2858`), a `TypeSpec` class token
gives `Ty::StructuralRef(intern_signature_type(&spec.type_, context))` as a constructed local owner
already does (`metadata.rs:2882-2889`); no new `Ty` variant. A throwing ctor: the intrinsic
initialises the local it is pushed into, so nothing is aliased before it returns, the throw surfaces
at the same instruction and no half-initialised reference exists (asserted by the behaviour driver).
Field initialisers live in the owner's `.ctor` in the defining image and are invisible; the claim is
only "the exact ctor ran".

## 5. Refusals with stable reasons

All `Refused`, rendered under `external allocation needs dependency facts: `: `owner is an
interface`; `owner is abstract`; `owner is a static class`; `owner is a value type; value
initialisation is not an object identity`; `constructor is static`; `constructor is abstract or
virtual`; `allocation reference is not a constructor`; `constructor signature is vararg`;
`constructor signature is not an ordinary instance .ctor` (no `has_this`, non-default conv,
non-void); `constructor parameter is byref or modified`; `owner is an open generic definition
without an instantiation`; `owner instantiation supplies N arguments but the definition declares M`;
`owner name is not usable` (empty segment, stem mismatch); plus the resolver's message verbatim for
ambiguity or signature mismatch. Every one is its own string - the second half of the sealed-mask
fix, and what makes the next reason mix readable.

## 6. Sealed-owner defect: fix first

`metadata.rs:3004-3007` tests `flags & (0x20 | 0x80 | 0x140)`; `0x100` is `Sealed`, so every sealed
local class is refused (28,668 bodies). Land that first: reduce the set to interface-or-abstract
only, add an explicit value-type test (`value_definition_kind`), split the catch-all per
`FINDINGS.md` C2-C11. External allocation then builds on a correct mask through one shared
predicate, `fn owner_kind_refusal(flags: u32, is_value: bool) -> Option<&'static str>`, so the paths
cannot drift: the local path passes the input image's `TypeDef.flags` and its
`value_definition_kind`, the external path the *defining* image's.

**Test that distinguishes sealed from abstract.** C# cannot emit `newobj` on an abstract type, an
interface or a static class, so no compiled fixture covers the negatives. (1) Predicate table test,
no binary: `(0x100, false) -> None` (sealed admitted), `(0x80, false) -> abstract`, `(0x120, false)
-> static class`, `(0x20, false) -> interface`, asserting the sealed bit alone never yields the
sealed refusal while `0x80` does. (2) End-to-end positive: `ExternalAllocationLibrary.Sealed` with a
field initialiser, allocated from the consumer, is `Partial` with the sealed refusal before the fix
(red) and `Decompiled` in both styles after (green). Existing control:
`tests/virtual_calls.rs:37-46` freezes `UnsupportedNew` (`new Square { ... }`,
`tests/dotnet/VirtualCallCases.cs:62`, `public sealed class Square`) as expected-partial with this
string; that row leaves `EXPECTED_PARTIAL` in the same commit.

## 7. Acceptance plan

New `rust/tests/dotnet/ExternalAllocationLibrary.cs` + `ExternalAllocationCases.cs`, built by
`rust/tests/build-external-allocation-fixtures.py` into release and debug `.bin` (sibling of
`build-external-call-fixtures.py`). Positives, `Decompiled` in both styles: `Plain` implicit ctor
(`NewPlain`); `Sized(int)`; `Mixed(string, int)` (`NewMixed`, argument order asserted in text);
`Sealed` with a field initialiser (`NewSealed`, also the sealed-mask gate); `Box<T>` (`NewBox(1)`,
owner line names the definition and `<int32>`); `Outer.Inner` (`NewNested`); `Throws` whose ctor
throws (`NewThrows`, driver observes the exception). Negatives, `Partial` with their own reason:
`Point` struct `(int,int)` ctor (`owner is a value type...`); `__arglist` ctor (`constructor
signature is vararg`); `ByRef(ref int)` (`constructor parameter is byref or modified`). Every
positive also stays `Partial`, byte-identical to today, with no reference supplied.

Frozen table `rust/tests/external_allocations.rs`: `EXPECTED_PARTIAL` as `(declaring_type, method,
reason)` for both configurations and styles, asserting exact reason equality (not `contains`), plus
rows freezing the owner comment text (name, kind, sealed) so a wrong owner fails loudly. Behaviour
gate `rust/tests/external-allocation-roundtrip.py`, release and debug, `--mutation-control`: return
value, the allocated object's observable field values, argument aliasing (a field aliases an
argument vs the node itself), exception type for `NewThrows`, identity (two calls give distinct
instances, one call gives one). Mutations that must be rejected: swap two argument texts in `Mixed`;
print the definition name where the instantiation is required (`Box<int>` -> `Box`); cache one
instance across calls; flip the predicate to admit abstract owners; drop an argument (caught by the
`parameter_count` cross-check and the behaviour diff).

Corpus: full fixed scope, then `report/per-assembly-gate.py` on the new baseline; sample
`Microsoft.CodeAnalysis.CSharp` / `VisualBasic`, `Newtonsoft.Json`, `FSharp.Core`,
`OpenRA.Mods.Common`, `System.Private.Xml`. Expected reason-mix shift: the sealed fix moves 28,668
bodies out of `allocation owner must be a concrete class` into precise reasons, then external
allocation moves at most 24,488 out of the external row into `Decompiled` plus the section 5
details. Report both deltas separately (`defect-sweep/BRIEF.md`); the 27-of-131 gate rows in the 38
near-passing images can only be read after both commits.

## 8. What must NOT be claimed

Allocation proves neither field-initialisation order, nor layout, nor that the object's internals
behave - only one fresh identity, then the exact `.ctor` with the IL's arguments in the IL's order
at the IL's position. The ctor body is not inspected: an in-ctor throw, a lock, a failing static
initialiser or `TypeInitializationException` are target-runtime effects at that instruction. Other
members of the defining image are not admitted, so a field read on the fresh instance keeps its own
blocker. Which assembly version loads, and whether the resolved definition is the one the process
would load, is a policy question (`exact` / `same-major`), not a proof. Without the owner's assembly
in the reference set the row does not move at all (`Unavailable`), so 24,488 is an upper bound under
the frozen manifest's declared references.

## 9. Deliberately out of scope

`TypeSpec`-parented external allocation (already lost its row to `constructed member needs a local
definition`, 12,104) needs the same fact plus a constructed-owner presentation decision. `new
Owner(args)` / `new Owner<int>(args)` presentation for local *and* external allocation together, one
rule: name from the defining image, instantiation arguments from the site, ambiguous or unnameable
owners refused rather than shortened. External base-ctor calls, external field access, external
generic methods.

## Top risk

The gain is bounded by the reference set: `Unavailable` is silent by design, so an image whose
manifest does not declare the owner's assembly keeps its exact current text and 24,488 is an upper
bound to measure, not assume. The risk that would produce *wrong* output rather than none is
name-based type selection inside the defining image: the fact must come from a unique
exact-signature `resolve_method` match, and every ambiguity must be `Refused`, never the first
candidate.
