# Typed managed locations and byref calls: pre-implementation audit

Six production default-output/ILSpy/original-IL triples are retained under
`audit/manifest.json`, including exact references and command statuses. All
ILSpy/IL commands succeeded; each case is ILSPY_YES_OURS_NO, zero BOTH_NO.
Independent later blockers are not removed from scope or counted as gains.

| sample | original-IL observation |
|---|---|
| FSharpMap.TryGetValue 0600021D | caller !1 managed pointer forwarded through generic MethodSpec |
| owned legacy Capturing.ReadLine 06000146 | int32 indirect read/write, string reference indirect write, mutation ordering |
| Decompiler GetDefaultMemberName 0600008B | out reference writes and struct enumerator locations; constrained cleanup remains independent |
| Roslyn Binder.ResolveOverloads 0600004D | ref HashSet reference slot, value-type addresses; GVM/constrained/layout gates remain independent |
| OpenRA CPos.Equals 06000068 | two struct local addresses passed to generic TryGetClrValue out parameters |
| OpenRA Target.Recalculate 060000BE | Boolean indirect writes plus external struct receiver locations |

## Architecture within the existing typed IL/CFG + lift model

Use one image-scoped managed-location descriptor arena for the existing ordinary
Ty elements (primitive, reference, enum, struct and owner-scoped generic value).
A location type is not its current contents or its storage identity. Stable
ref-bound Value names preserve the acquired location; dup copies that address,
not an expression that can be reevaluated. Keep existing I4/U4/I8/U8/struct
address variants canonical where already admitted, and bridge them through one
element-type query. Foreign enum/value identities remain caller-bound Ty facts;
never reinterpret a defining-image token against the caller to intern an address.

- Plain ref/out parameters are address slots, not incoming value copies. Loading
  one copies its managed pointer; indirect loads/stores read/write only at their
  original instructions. Unknown modifiers remain refused.
- Call parameters use invariant element identity and explicit ref operands.
  Aliases to the same slot must stay aliases, including writes visible when the
  callee throws. No copy-in/copy-out helper or scalar-address-to-value shortcut.
- ldarga/ldloca and field/static/array address acquisition produce stable ref
  bindings; null/bounds/type-initialization checks happen at acquisition, before
  later arguments/effects. Do not reacquire a field/element after a callback.
- Typed ldobj/stobj/initobj/cpobj work on the same proved storage element. Scalar
  indirect operations preserve width/signedness and stored vs CLI stack types;
  reference operations preserve identity and struct operations copy whole values.
- Byref returns, pointer-to-pointer slots, ref locals/stack joins, readonly or
  other unsupported modifiers, unmanaged pointers and unknown escape shapes
  remain explicit gates until separately implemented and behavior-validated.
  GVM dispatch remains gated; byref support does not bypass that failed gate.

The representation keeps per-image identity, stable aliases and the existing
pipeline. It does not add a C# AST, host-layout inference, unsafe original-IL
executor or whole-method proxy. Unknown target runtime behavior stays in the
same exact leaf call; supplied locations remain live across its effects.

## Related receiver and snapshot findings

The owned scalar-receiver triple (`audit/owned-scalar-receiver.*.txt`) shows
ldarga int32 followed by an exact direct System.Int32.ToString call. Treating
that target as an external reference-class receiver rejects a valid managed
value receiver. Recognize qualified core scalar owners, or exact already-known
struct/enum owner facts, and pass the same acquired location explicitly by ref.
Do not add boxing, a copied receiver, virtual dispatch or a null check. Unknown
foreign value owners remain unproved; constrained/GVM dispatch stays gated.

A focused default-output test also reproduced flat compaction substituting an
indirect read with a later ref alias after a write. Keep explicit snapshot
lifetimes in bodies containing ref bindings; the ref name denotes mutable storage,
not an immutable value. `snapshot-red.log` and `snapshot-green.log` retain the
public red/green evidence; executable checks compare original pre-write returns.
Taking the address of a constructor's this slot is separately refused because
an alias could replace entry-this without a starg instruction.

## Initonly field-address regression and repair

The first full scan retained 20,808 new recoveries but also **824 recovered-method
regressions**, all caused by the new blanket readonly-field-address refusal.
This is not a passing milestone. Its baseline/delta remain in
`corpus-scan/` and `delta.json`; final validation uses separate paths.

`audit/readonly-manifest.json` records three production regressions and three
owned class/value/static readonly-field triples before the repair. All six
ILSpy and IL commands succeeded; ours refused each address. The compiler emits
ldflda/ldsflda for ordinary reads of initonly struct fields. A copied value or
a C# writable source borrow is not the same operation.

Emit exact `__cil_ldflda_TOKEN` / `__cil_ldsflda_TOKEN` location leaves for
initonly fields, preserving the original opcode, field identity, receiver
location and check/initialization point. A leaf returns the actual acquired
managed pointer; it does not read a field value. Direct stfld/stsfld initializer
permission gates and the constructor entry-this provenance gate stay separate.
This does not interpret arbitrary IL or proxy a method implementation.

Owned `readonly-red.log` records the legitimate compiler-produced read failure.
The execution adapter emits just that field-address instruction. The owned
Number-field initonly variants additionally execute the original indirect writes
against the recovered locations, showing that a blanket address refusal or
a copied scratch cell is not faithful to these actual runtime operations.
Five existing mutants run in each configuration/variant; the initonly variants
also reject a separate copied-field-location mutant. No third-party code runs.

## Noncanonical Boolean storage defect

An owned variant changes Flag's ldc.i4.1 to ldc.i4.2 before stind.i1.
The original writes byte 2, while the recovered truth-value conversion wrote 1.
`bool-storage-red.log` records the executed mismatch; the source/ILSpy/IL
triple and commands are retained as `audit/owned-bool-bits.*.txt` and
`audit/bool-manifest.json`. ILSpy also prints `value = true` here; its source
is not a semantic oracle. This is a defect repair, not a fallback count gain.

Represent Boolean loads/stores as explicit primitive byte-preserving operations,
including parameter/field/call loads, returns and boxing. Owned adapters perform
only those primitive conversions, not selected method execution. Inputs include
raw Boolean byte 2; the patched BooleanBits method exposes ldind.u1 directly.
Six ordinary/initonly/raw-bit release/debug runs cover copies, returns and boxed
bits. Dedicated load/store-normalization mutants must fail. Readable cancellation
is allowed only for exact load/store identity or proven 0/1 predicates.

## Validation seams and batching

Public decompile/scan red before green, followed by owned executable default
body tests. Include aliased ref arguments, signed/unsigned widths, bool and enum
storage, reference identity, generic/struct copies, out defaults, field/array
acquisition before effects, receiver/global replacement after address capture,
and writes preserved on exception. Recovered support bodies must call recovered
support bodies, not originals. The runtime test adapter supplies real typed
managed pointers into shared cells, not reflection copy-in/copy-out arrays.

Ref/out source definite-assignment annotations may be rebound to ref in owned
executable carrier headers; CLR parameter element/ByRef identity must still
match exactly. This changes test scaffolding, not recovered body operations.
Negative tests must reject mismatched element types, pointer escape/returns,
readonly/custom modifiers, address joins and value/location confusion. One
larger group milestone consolidates full regression, behavior/mutation, the
same fixed-scope corpus scan and main-session source/IL self-review. No original
third-party assembly execution or denominator/reference-policy changes.
