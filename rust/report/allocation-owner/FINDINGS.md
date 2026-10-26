# `allocation owner must be a concrete class` - diagnosis

Read-only source diagnosis. Nothing was built, scanned, edited or executed other
than reading source and the already-written scan artifacts.

## Verdict in one line

The reason is emitted from exactly one site, and its flag mask includes **bit
`0x100` = `TypeAttributes.Sealed`** (written as `0x140`). Every sealed reference
class therefore fails the "concrete class" check, which is why ordinary local
classes such as `<>c__DisplayClass65_0`, `MethodBlockSyntax`, `BuilderOptions`,
`AssemblyBuilder/TypeForwarder` and `CrefSignatureSyntax` are refused. The
deeper message is not wrapped by a helper: the string is a *catch-all for
admission shapes* (sealed / struct / enum / static / abstract) and, because it is
the earliest check in the chain, it also *preempts* the precise diagnostics that
follow it (generic arity, generic declaring context, ancestry, constructor
signature, parameter types).

---

## 1. Where the string comes from, and the full decision chain

Single emit site: `rust/src/metadata.rs:3005-3007`

```rust
3004        let definition = self.md.tables().type_def(owner.rid())?;
3005        if definition.flags & (0x20 | 0x80 | 0x140) != 0 {
3006            return Err("allocation owner must be a concrete class".into());
3007        }
```

`rust/src/metadata.rs:3928` decodes the same flag space and names `0x100` as
`sealed`; `rust/src/metadata.rs:3909-3941` (`type_flags`) names `0x20 interface`,
`0x80 abstract`, `0x100 sealed` and never mentions `0x40`. `0x140` is therefore
`0x100 | 0x40`, where `0x40` has no defined meaning for a `TypeDef` row
(ECMA-335 II.23.1.15); the only bit that does any work is **sealed**.

### Chain from the `newobj` arm

`rust/src/lift.rs:590-619` (the non-exception `newobj` path):

1. `591` `let token = i.operand.token().ok_or("missing constructor token")?`
2. `592` `if self.image.exception_constructor(token).is_err()` - the core
   `ArgumentOutOfRangeException(string,string)` path is tried first; **any** error
   there (it requires a `MemberRef`, `rust/src/metadata.rs:2874-2890`) falls
   through, so all local `MethodDef` constructors land in the allocation path.
3. `593-595` `self.image.allocation_constructor(token, self.type_context)?`
4. `596-601` the returned argument types are popped and `assignable`-checked;
   `602-617` the `__cil_newobj_<token>` intrinsic is emitted.

`rust/src/metadata.rs:2967-3035` (`allocation_constructor`), in execution order:

| # | lines | check | reason if it fails |
|---|---|---|---|
| 1 | 2972-2989 | token table: `MethodDef` (rid != 0) -> `(token, None, None)`; else `MemberRef` (name `.ctor`, guard-parsed signature, `constructed_member_owner`, `find_constructor`); else refuse | `allocation needs a constructor reference` / `external allocation needs dependency facts; only local classes and local instantiations are admitted` / `constructed member parent must be a concrete class` / `constructor overload selection needs IL fallback` / `constructed allocation names no constructor of its definition` / `allocation needs a local constructor definition` |
| 2 | 2990-2997 | method row: name `.ctor`, not static, `flags & 0x1800 == 0x1800` (SpecialName+RunTimeSpecialName), `flags & 0x440 == 0` (no Virtual/Abstract) | `allocation needs an ordinary instance constructor` |
| 3 | 2998-3002 | `type_of_method` -> declaring `TypeDef` rid | `constructor has no declaring type` |
| 4 | 3004-3007 | **`TypeDef.flags & (0x20 \| 0x80 \| 0x140) == 0`** | **`allocation owner must be a concrete class`** |
| 5 | 3008-3013 | `MemberRef` path: `generic_params(owner).len() == arguments.len()` | `allocation instantiation arity differs from its definition` |
| 6 | 3014-3018 | `MethodDef` path: `!generic_declaring_context(owner)` (walks `NestedClass` up to 64 levels, `rust/src/metadata.rs:2654-2678`) | `allocation owner must be a concrete nongeneric class` |
| 7 | 3020 | `class_name(owner.0)` -> `class_chain` (`2702-2740`) + `symbol_names::type_name` | `unresolved/cyclic reference hierarchy needs IL fallback`, `interface declaring type needs IL fallback`, `constructed base needs member binding`, `constructed base needs a local definition; external constructed bases need dependency facts` |
| 8 | 3021-3027 | signature: `has_this`, `CallConv::Default`, no varargs, return `void` | `allocation constructor signature needs IL fallback` |
| 9 | 3029-3033 | per-parameter `member_parameter` (`3279-3296`) | `byref/modified parameter needs IL fallback`, plus `Ty::from_param` / `Ty::from_sig_scoped` errors |
| 10 | 3034 | result type `Ty::Class(owner)` (or the `MemberRef` receiver) | - |

Note `0x140` appears in the working tree only: `git show HEAD:rust/src/metadata.rs`
contains no `allocation_constructor` at all, so this whole function and its mask
are uncommitted work. Line numbers here are for the working tree.

### Empirical size (existing artifact, no new scan)

`rust/report/boxing/corpus-scan-enum/baseline.json`: **28,685** bodies carry this
exact reason across **192 of 617** images (the report's 28,668 is a newer build of
the same defect). Worst images: `Microsoft.CodeAnalysis.VisualBasic` 2055/2005/1885,
`Microsoft.CodeAnalysis.CSharp` 1615/1575/1575/1534/1511, `OpenRA.Mods.Common` 1437,
`mscorlib` 645+429, `FSharp.Core` 437. That shape - Roslyn and F# bodies, where
nearly every internal type is `sealed`/`NotInheritable` - is only compatible with a
sealed-bit mask; an abstract/interface mask could not produce it, because IL cannot
`newobj` an abstract type or an interface.

Decisive in-repo control: `rust/tests/dotnet/VirtualCallCases.cs:32` declares
`public sealed class Square : Shape`, and `rust/tests/virtual_calls.rs:37-46` freezes
`VirtualCallCases.Ops.UnsupportedNew` (whose body is
`new Square { Size = size }.Area()`, `rust/tests/dotnet/VirtualCallCases.cs:62`) as
expected-partial with **this very string**. A sealed concrete class is asserted to be
unallocatable - the mask's behaviour is encoded in the fixture expectations.

---

## 2. Every way the chain can reach this error

| id | shape | which flag fires | correct verdict? |
|---|---|---|---|
| **C1** | **sealed reference class (`0x100`)** - display classes, Roslyn `InternalSyntax`/`Bound*` nodes, F# records/unions, immutable types | `0x100` | **WRONG**. Sealed classes are instantiable and are the normal case. This is the 28k. |
| C2 | static class (`abstract` + `sealed`, `0x80\|0x100`) | `0x80`, `0x100` | refusal right, reason wrong ("concrete class" for a static class as the ctor owner is at best vague; a `newobj` of it is invalid IL) |
| C3 | abstract class (`0x80`) | `0x80` | refusal right (invalid IL), reason could say `abstract` |
| C4 | interface (`0x20`) | `0x20` | refusal right (invalid IL only), reason says "concrete class" instead of "interface" |
| C5 | struct / value type (`0x100` + `ValueType` base, `0x100` is what actually fires) | `0x100` | refusal right, but **by accident**: once C1 is fixed by dropping `0x100`, structs would be wrongly admitted as classes. Needs an explicit value-type test. |
| C6 | enum (`0x100` + `Enum` base) | `0x100` | same as C5 |
| C7 | sealed generic class reached through a `MethodDef` ctor (open generic) | `0x100` today, would be `3015` after the fix | today's reason is wrong; the real one is `allocation owner must be a concrete nongeneric class` |
| C8 | sealed non-generic class nested in a generic type (`generic_declaring_context` true via an outer type) | `0x100` today, `3015` after the fix | same as C7 |
| C9 | sealed class whose ancestry is unprovable in this image (external base, constructed base, cyclic/`extends` chain) | `0x100` today, `3020`/`class_chain` after the fix | wrong today; real reasons are the four `class_chain`/`base_definition` strings |
| C10 | sealed owner with a `byref`/`modreq` constructor parameter, or a parameter type the scoped transfer cannot name | `0x100` today, `3029-3033` after the fix | wrong today; real reason `byref/modified parameter needs IL fallback` |
| C11 | any `TypeDef` with the undefined bit `0x40` set | `0x40` | no explanatory power at all; dead for well-formed images but it widens the mask for no benefit |

Explicitly *not* causes here (they are unreachable before line 3005, and their own
messages are distinct):
- token table: `newobj` of something that is not `MethodDef`/`MemberRef`
  (`2988`), a `MemberRef` under a non-`TypeSpec` parent (`2983`), a `TypeSpec` that
  is not a class instantiation (`2918`), non-local constructed definitions (`2919`).
- ctor row problems (`2991-2997`), missing declaring type (`3002`), missing/invalid
  `TypeDef` row (`3004` is `?`, so a bad rid surfaces as a `cildec` table error).
- "type declared later in the image": metadata tables are index-addressed, so
  `type_of_method`/`type_def` never depend on declaration order. This is not a cause.
- recursion depth: the only depth limits are `local_type_definition` (32,
  `rust/src/metadata.rs:2004`) and `generic_declaring_context` (64, `2660`); both
  produce their own reasons and are unreachable behind this check.

Count of distinct causes that can produce *this exact string*: **11** (C1-C11).
One adjacent misdiagnosis uses a *different* string and should be fixed in the same
pass: `rust/src/metadata.rs:2981-2983` labels a genuinely local class as
`external allocation needs dependency facts; only local classes and local
instantiations are admitted` whenever the `MemberRef` parent is a `TypeRef` rather
than a `TypeSpec` (24,893 bodies in the same baseline) - the parent is never
canonicalized through `local_type_definition` there, unlike `3020`'s `class_name`.

---

## 3. Is the string a catch-all hiding a deeper helper failure?

**Not in the "wrapped helper error" sense, yes in the "absorbing check" sense.**

- No helper is rewrapped: every deeper refusal propagates with its own text via
  `?` (see the table in section 1). Grepping `src/` finds the literal at exactly one
  site (`rust/src/metadata.rs:3006`); the only other occurrences are the three
  frozen test tables.
- The absorbing effect is real: line 3005 is check #4 of 9 and it collapses five
  different metadata shapes (C1-C6) into one reason, and for genuinely sealed
  owners it *preempts* checks #5-#9. Concretely, today the following can never be
  reported for a sealed owner and are all labelled "concrete class":
  - arity mismatch of a constructed instantiation (`3010-3011`),
  - open generic owner / owner nested in a generic type (`3015-3016`),
  - unprovable/interface/constructed ancestry (`2702-2740` via `3020`),
  - non-void or non-default-convention `.ctor` (`3022-3027`),
  - `byref`/`modreq` constructor parameters (`3290-3294`).
- There is already a precedent for doing this better: `rust/src/lift.rs:983-991`
  keeps a stable prefix and appends the deeper reason
  (`callvirt on an opaque external receiver needs dependency facts: {reason}`).

---

## 4. Does a parameter problem get reported as an owner problem?

Yes - by ordering, not by wrapping. The parameter check is the **last** substantive
step (`rust/src/metadata.rs:3029-3033`); everything below runs before it:

1. operand table dispatch and `constructed_member_owner` / `find_constructor`
   (`2972-2989`, plus `2913-2965`),
2. constructor row flags (`2990-2997`),
3. `type_of_method` (`2998-3002`),
4. `TypeDef` flags mask, i.e. the owner check (`3004-3007`),
5. constructed arity (`3008-3013`),
6. `generic_declaring_context` (`3014-3018`),
7. `class_name` / `class_chain` ancestry proof (`3020`, `2702-2740`),
8. constructor signature shape (`3021-3027`),
9. **then** `member_parameter` per parameter (`3029-3033`), and finally the
   stack/argument `assignable` checks back in `rust/src/lift.rs:596-601`.

So for the 28k sealed owners, a `byref`-parameter body is reported as an owner
problem. This is a genuine diagnosis-quality defect (single-reason, first-failure-wins
output), and it also skews the bucket table: the real post-fix reason distribution is
unknown until line 3005 stops firing.

---

## 5. Fix per cause

| cause | admission rule change | message change |
|---|---|---|
| C1 sealed class | **yes - required.** Mask must be interface/abstract only (`0x20 \| 0x80`); the `0x100`/`0x40` bits must not appear in a rejection mask | no |
| C2 static class | no (still refuse) | **yes** - name the shape (`static class cannot own an allocation`) |
| C3 abstract class | no | **yes** - name `abstract` |
| C4 interface | no | **yes** - name `interface` |
| C5 struct / C6 enum | **yes - required as a follow-on.** Dropping `0x100` removes the accidental refusal; add a positive reference-class proof (`struct_type(owner)?.is_none()` or `has_core_base(owner, "ValueType")/("Enum")`, cf. `rust/src/metadata.rs:2507-2510`, `2419-2426`) | **yes** - value types need their own reason ("value type allocation needs a storage copy"), not "concrete class" |
| C7 sealed generic class | no (correct at `3015`) | no - reason already distinct; it simply becomes reachable |
| C8 nested-in-generic | no | no |
| C9 unprovable ancestry | no | no |
| C10 parameter problems | no | no - becomes reachable |
| C11 undefined `0x40` | **yes** - drop the dead bit | no |
| adjacent (TypeRef parent) | **yes** - canonicalize the `MemberRef` parent through `local_type_definition` before concluding "external" | no |

Both are needed in practice, in this order: (a) admission rule for C1/C5/C11, then
(b) messages so the residual buckets are diagnosable, then the three frozen tables.

Frozen tables that move (all three encode the bug and are *struct* or *sealed class*
fixtures, so their rows must be re-derived, not loosened):

- `rust/tests/new_objects.rs:30-35` `NewObjectCases.Ops.UnsupportedStruct` ->
  `new Holder(value)` where `Holder` is `public struct Holder`
  (`rust/tests/dotnet/NewObjectCases.cs:42`): stays partial, needs the value-type
  reason.
- `rust/tests/struct_receivers.rs:55-59` `StructReceiverCases.Ops.UnsupportedConstruct`
  -> `new Point(1,2,3,null)`, a struct (`rust/tests/dotnet/StructReceiverCases.cs:78`).
- `rust/tests/virtual_calls.rs:41-46` `VirtualCallCases.Ops.UnsupportedNew` ->
  `new Square { ... }` where `Square` is `public sealed class`
  (`rust/tests/dotnet/VirtualCallCases.cs:32`): this row **must be deleted** once
  C1 is fixed; it is a sealed class and should decompile.

---

## Recommendation for the next worker brief (one paragraph)

Fix the admission rule at `rust/src/metadata.rs:3005` first and treat it as the whole
defect: replace `flags & (0x20 | 0x80 | 0x140) != 0` with `flags & (0x20 | 0x80) != 0`
interface/abstract-only rejection plus an explicit positive check that the owner has a
reference-class identity (`struct_type(owner)?.is_none()` / no `System.ValueType` or
`System.Enum` core base), because the current `0x100` bit is `TypeAttributes.Sealed`
and is the reason 28,668 ordinary sealed classes are refused - proven in-repo by
`VirtualCallCases`' `public sealed class Square` being frozen as unallocatable. Do the
minimum in one commit: mask fix + explicit value-type refusal + a reason suffix that
names the failing bit/shape (mirror the `callvirt ... : {reason}` precedent at
`rust/src/lift.rs:983-991`), refresh the three frozen tables (`new_objects.rs:30-35`
stays partial with a value-type reason, `struct_receivers.rs:55-59` stays partial,
`virtual_calls.rs:41-46` is deleted), then re-run the corpus scan and read the *new*
bucket distribution rather than trusting the current one, since checks 5-9 at
`metadata.rs:3008-3033` have been unreachable behind the mask for the entire 28k and
their real frequency is still unmeasured. Do not bundle the adjacent "TypeRef parent
reported as external allocation" misdiagnosis (`metadata.rs:2981-2983`, 24,893 bodies)
into the same change; record it as a separate brief with its own fixture.
