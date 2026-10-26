# Agent-readable lowering semantics

Output is C#-like analysis, not reconstructed source or a compilable project.
The CLI defaults to a readable class/method view; `--details` preserves the full
image-scoped metadata/IL evidence record. Original metadata is authoritative.
All operations below are semantic notation; the CLI does not ship a runtime adapter.

## Constructed value operations and scoped boxing

`__cil_value_call_TOKEN` performs the original ordinary nongeneric call on an
admitted constructed value owner. Type arguments remain caller-scoped, and an
instance receiver is the exact managed location, not a copied or boxed receiver.
Local generic value bodies use a structural open-self location. Local field
access requires equality of the exact instantiated declaration and operand field
types, not only a matching name or evaluation-stack width.

`__cil_value_newobj_TOKEN` initializes separate value storage, executes the exact
original constructor with evaluated arguments, then copies the completed value.
It does not allocate a fresh reference-class identity. Exceptions keep constructor
effects and prevent subsequent stores of a nonexistent completed result.

Boxing uses the exact scoped type: known non-nullable values copy into fresh
boxes, reference instantiations preserve identity, and nullable/generic types keep
CLR boxing rules. Unbox.any retains exact runtime type/null checks and value
copies, including generic reference and nullable cases. Managed locations are
not silently read as ordinary values. Boolean storage still preserves raw bytes.

The class-only shared owner binder remains class-only. This does not imply
generic value MethodSpec calls, arbitrary static/foreign fields, unbox addresses,
ordinary constrained dispatch or GVM admission. See
[constructed value evidence](acceptance/corpus/CONSTRUCTED_VALUE_CALLS.md).

## Exact scoped reference calls

Ordinary external reference callvirt can remain `__cil_callvirt_TOKEN` without
guessing a dependency implementation. `__cil_reference_call_TOKEN` is the exact
direct/static call for a foreign constructed reference owner; its caller-scoped
signature supplies argument/result types. Metadata-proven property/member source
syntax is optional and separate from the exact runtime call. These are original
IL call leaves, not selected-method/body proxies. Value layout, arbitrary
inheritance, allocation kind, unsupported managed-location shapes and GVM lookup
remain separately gated.

Proved reference TypeSpecs can be cast/tested; reference-type unbox.any uses
castclass semantics, preserving null and successful reference identity. Qualified
core Object/String/Array spellings share their known reference identity, with
vectors assignable to Array only in the valid direction. See
[reference-call foundation](acceptance/corpus/REFERENCE_CALL_FOUNDATION.md).

## Generic method instantiation calls

`__cil_methodspec_call_TOKEN` is an exact original MethodSpec direct/static
call, preserving its instantiated declaring owner, method arguments, runtime
initialization, access/constraint checks and effects. Var/MVar substitution is
simultaneous in the caller's proven scopes, not name-based. Nonvirtual generic
callvirt retains its original MethodSpec token in the existing callvirt leaf.

Generic virtual callvirt remains fallback: owned runtime validation exposed a
null-receiver/argument-effect ordering difference not represented by the current
snapshot model. External generic instance dispatch, unsupported modified
signatures and unsupported arguments are separately refused. See
[MethodSpec acceptance](acceptance/corpus/METHOD_INSTANTIATIONS.md).

## Scoped reference relationships

Assignment proofs retain complete constructed arguments and caller-qualified
variables across local base/interface templates. Open-self structural and named
declaring identities are interchangeable only after the scoped identity proof.
Interface variance needs metadata flags, reference arguments and a directional
proof; value arguments and managed-location elements remain invariant.

Exact dependency origins project ancestor templates into the caller vocabulary.
Unknown/unrepresentable relationships stay unproved. Demand-driven retries and a
context-stamped registry avoid eager graph construction for unrelated operations;
adding definitions and query order are covered by public tests. No foreign token
is reinterpreted as a caller token.

A proof inserts no runtime type check, copy or allocation. Formal-type operands
retain the original source-named overload. Qualified declaration spelling does
not require proving every foreign ancestor; a local class can retain reference
identity across a declared foreign constructed base while ancestor assignments
still require their own exact argument proof. See
[reference-relationship acceptance](acceptance/corpus/REFERENCE_RELATIONSHIPS.md).

## Typed managed locations and byref calls

Plain ref/out parameters carry managed pointers, not incoming value copies.
Image-scoped descriptors retain invariant element identity; stable ref bindings
retain storage identity and acquisition timing. Admitted primitive, reference,
enum, struct and scoped generic elements support indirect loads/stores and
ldobj/stobj/cpobj/initobj without inferred host layout. Reads produce value
snapshots at their IL instruction, including Boolean and narrow-width stack
normalization; writes reach the original aliased storage even before exceptions.
__cil_bool_load reads the Boolean storage byte as an unsigned I4 value;
__cil_bool_store truncates I4 bits to that byte without canonicalizing nonzero
values to 1. Comparisons may still produce canonical 0/1, and branches test
zero/nonzero. Byte value 2 is observable through ordinary indirect storage and
boxing; a C# truth-value conversion is not a substitute for these operations.

Known core scalar and proved value owners receive direct calls through the same
location, not a copied/boxed receiver. Initonly field addresses use exact
__cil_ldflda_TOKEN / __cil_ldsflda_TOKEN leaves returning the actual pointer,
with original null/access/initialization behavior. They do not fabricate a
C# writable borrow of a readonly field or replace an address with its contents.
Direct field-store permission checks are separate. Constructor this-slot
exposure remains refused without entry-this provenance.

Flat readable compaction is disabled in ref-bound bodies so reads cannot be
substituted with mutable aliases after a write. Unpinned/unmodified byref locals
and identical-element address CFG edges use scoped reference scratch slots and
parallel ref reassignment, not contents copies. Original argument-slot rebinding,
byref result/return escapes, pinned/modified locals, unmanaged pointers and
unsupported prefixes remain refused. Null/reference CFG joins merge only Null
with the same exact reference identity, with successor dataflow reanalysis.
See [managed-location acceptance](acceptance/corpus/MANAGED_LOCATIONS.md) and
[constrained/control-flow acceptance](acceptance/corpus/CONSTRAINED_DISPATCH.md).

## Static storage operations

Referenced static fields retain the exact original load/store/address operand
and caller generic scopes. Local owner kind/arity and requested/declaration
stored types must agree; foreign flags require an exact selected definition.
One closed instantiation is never erased into another's storage. Static fields
on value containers need no instance receiver/layout proof.

Reads copy values, acquired addresses remain aliases and runtime initialization
stays at the original access. Thread-static identity is the target runtime's.
Readonly direct stores require the same owning initializer and closed storage;
an address grants no source-store permission. Initializers remain actual CLR
initializers, not ordinary methods. See
[static storage evidence](acceptance/corpus/STATIC_STORAGE.md).

## Exact exception identities

A bounded checked core family in System/System.IO has explicit nominal parent
facts for assignment; arbitrary names/suffixes are not exception evidence.
Finite local nongeneric ancestry ending in that family admits the exact local
reference for throw/catch. Catch TypeDefs retain local class identities.
Null throw, incoming/caught exception objects, rethrow and finally use the target
runtime's ordinary exception behavior.

Core constructor lowering validates an ordinary instance void signature and
retains the exact __cil_newobj_TOKEN operation, original arguments, allocation
identity and effects. It does not infer arbitrary foreign allocation facts.
Generic/unknown foreign exception ancestry, catch-object wrapping, filters/faults
and unsupported transfers remain separate gates. See
[exception identity evidence](acceptance/corpus/EXCEPTION_IDENTITIES.md).

## Ordinary constrained dispatch

`__cil_constrained_TYPE_METHOD` retains the original adjacent constrained and
callvirt tokens, checked signature and acquired managed receiver. The runtime
selects interface/virtual dispatch, boxing fallback or reference-slot dereference;
no copied receiver or guessed implementation replaces it. Prefix adjacency and
CFG/EH entry boundaries are validated. Argument values and compiler-authored
receiver snapshots remain at their IL instructions. GVM, byref results and
constructor/intrinsic targets keep their separate refusal gates. See
[constrained dispatch evidence](acceptance/corpus/CONSTRAINED_DISPATCH.md).

## Presentation versus semantics

Readable output shows original class/member declarations with parameter names and
`this`, not the detailed static-carrier signature. It removes diagnostic comments
and safely simplifies immutable copies, adjacent unchecked pure expressions,
Boolean 0/1 encodings and a bounded set of return-only edges. Field/address/call/
allocation snapshots, checks and target-runtime numeric operations remain ordered;
not every expression or loop becomes source sugar. Fully qualified type names may
remain to avoid pretending to have general C# binding.

Unsupported bodies show their reason and complete IL as comments inside the
method body, including locals/EH. Failed bodies retain available raw/decodable
body evidence or the original parse error inline. These are not executable
empty-body replacements. Use `--details` for additional raw metadata, exact
scope/token identities and interpretation limits, not to reveal a hidden fallback. A readable declaration or successful body
is not evidence that attribute/security/native policies were interpreted, nor
that dependencies were recovered. Both presentations retain the same statuses and
exit precedence. The detailed library entry points remain compatible; explicit
`OutputStyle::Readable` selects the concise view.

Symbols retain their original assembly/module/token identity independently of
C# spelling. Safe aliases are shared by declarations and references. Local
Module/nested TypeRefs are canonicalized only through exact namespace/name and
enclosing-owner metadata; foreign scopes are not matched by short name. A
literal backtick is not GenericParam ownership. Aliases do not prove property
source lookup or interface implementation, and do not admit static initialization,
new type/operator/call shapes or arbitrary generic/value/EH bodies. Raw identities
remain in Details. See [naming acceptance](acceptance/openra/SYMBOL_NAMING.md).

Constructor initializers are presentation of an existing-receiver exact call,
never allocation. Only a first initialization with unchanged, exactly typed
arguments can move into `: base(...)` / `: this(...)`; pre-base writes/calls stay
in place. Parameterless initialization is explicit, not silently erased.

Property source recovery needs actual Property/MethodSemantics and a separate
source proof: unchanged original `this`, exact direct dispatch, an accessible
unambiguous local/nonvirtual or immediate-base lookup, and exact owner, accessor
reference/definition and PropertySig type paths. `base.Property` suppresses virtual
dispatch exactly when the original opcode was `call`. Getter/setter names alone,
version-substituted metadata, indexer/generic/modified shapes, changed receivers
and unproved lookup keep exact calls. Accessor methods and relationship summaries
remain visible; automatic-property declarations are not inferred.

For owned unchanged-this signed-I4 fields and address-unexposed I4 locals, adjacent
old-value load plus same-location unchecked subtract/store can become postfix.
A still-live generated old-value variable remains at its original point, before
later effects/reads. Only a sole immediate old-value-zero return is further fused
into `return unchecked(location-- == 0);`. Checked/unsigned stores, different or
effectful receivers, intervening operations and unproved address cases remain
explicit. These are presentation rewrites, not new body lowering.

## Identity, status and order

- `--type` selects the exact TypeDef metadata name and recursively owned nested
  types, not referenced dependencies or similarly named siblings. Metadata arity
  uses backticks; nested names use `/`.
- `.method`, `.field`, `.property`, `.event`, `.accessor`, `.generic` and related
  records retain original tokens, signatures, flags, owners and relationships.
  A static-looking lowered body does not change the original method's flags.
- Every method has its own status: `Decompiled`, legitimate bodyless
  `Declaration`, unsupported `Partial` with complete readable IL, or `Failed`
  with the available raw/decodable evidence. Metadata failures are counted
  separately. Exit 0 means complete lowering/declarations, 2 means fallback
  without failure, 1 means input/selection/body/metadata failure.
- `aN` is an argument; instance `a0` is the explicit receiver. There is no implicit
  receiver null check at entry. Each `vN` is a stable value evaluated at its
  original IL position. A field read is a snapshot, not a deferred expression.
  Calls, stores, allocation and checks remain ordered even when a result is
  popped. `dup` copies the value/location identity, not the generating expression.
- Labels/goto describe original normal control flow. Edge copies first snapshot
  all outgoing stack values, then assign destination stack slots in parallel.
  A back edge does not imply termination. EH is not modeled by these edges;
  methods with exception regions fall back.

## Attached metadata and interpretation limits

Class results retain attached records even when their higher-level meaning is
not interpreted. These records are not executable recovered method bodies:

- `.class-layout TOKEN parent=... packing=... size=... raw-row-bytes=[...]`
  preserves every owned ClassLayout row. Zero size means runtime-computed; zero
  packing means default. Full target layout and ancestry validation is not
  performed. Field offsets remain separate FieldLayout evidence.
- `.field-marshal TOKEN parent=... native-type-blob-index=... bytes=[...]
  raw-row-bytes=[...] payload=uninterpreted` preserves field, ordinary parameter
  and return-parameter (sequence 0) native descriptors. Marshaling flags and
  association counts are checked; payload interpretation is not performed.
  Native conversions, allocation, pinning and cleanup remain runtime-owned,
  not additional operations recovered from an ordinary managed body.
- `.pinvoke TOKEN method=... flags=... scope=... name=... module=...`
  retains ImplMap and ModuleRef row/index evidence. Entry and module names are
  independent: an unreadable entry does not hide the module, or vice versa.
  Invalid scope retains its full raw RID rather than a truncated token.
  Native binding and interop execution remain runtime-owned; no library is loaded.
- `.custom-attribute TOKEN parent=... constructor=... value-blob-index=...
  value-bytes=[...] raw-row-bytes=[...] payload=uninterpreted` retains exact
  attachment, constructor identity/signature and complete payload. Parents
  include owned members, parameters, generic parameters/constraints, interface
  relations and security declarations, plus image-level Module/Assembly context.
  Payload arguments and attribute effects are not decoded or executed. Do not
  infer that an unknown attribute has no effects from a successful body status.
- `.decl-security TOKEN parent=... action=... permission-set-blob-index=...
  permission-set-bytes=[...] raw-row-bytes=[...] payload=uninterpreted` retains
  type/method/assembly declarations. Permission-set formats and action policy are
  not interpreted; security enforcement belongs to the original target runtime.
  A recovered body is not a security-policy reconstruction or bypass contract.
- `.method-impl TOKEN class=... body=... declaration=... raw-row-bytes=[...]`
  retains the explicit MethodImpl endpoints, with separate exact owner/signature
  records for body and declaration. This mapping is not inferred from names,
  overload spelling or source `override` syntax. Full dispatch, access, ancestry
  and signature-compatibility validation is not performed; external and generic
  endpoint records remain raw metadata context; the separate explicit resolver
  does not turn them into general dispatch or implementation validation.

Broken readable records retain raw evidence, explicit metadata failure and peer
members; their failure count is independent of body statuses. Null/out-of-range
owners are retained as `ownership=unowned-invalid` image context, not assigned to
the selected type or silently treated as valid unrelated records. Endpoint
name/owner indexes, original row and signature blob index/bytes precede fallible
signature/reference interpretation, so malformed signatures do not erase them.
A complete inventory means the records are present, not that every payload or runtime policy is modeled.

## Static dependency context

[DEPENDENCIES.md](DEPENDENCIES.md) defines discovery order, full scoped identity,
exact/default versus explicit metadata replacement, forwarders/generic positions,
reference-contract classification and layered limits. Byte-only APIs never search
files. Metadata definitions and signature matches are not implementation, publisher
or runtime-compatibility proof. Reference input stubs are explicitly marked in
context output. Missing/rejected dependencies do not suppress independent bodies;
large missing sets have a default notice and complete scoped details.

## Numeric categories

The canonical evaluation values for unsigned storage are bit patterns in `int`,
`long` or `nint`. Declared uint/ulong/nuint does not change an opcode's input
interpretation. Explicit unchecked casts preserve those bits at storage/operation
boundaries. `checked` means the original overflow check is retained, not a general
source compilation setting. Shift masks and unsigned reinterpretations follow
CIL rules within the admitted I4/I8 subset.

- `conv.i4`: keep low 32 bits from I4/I8/native input, without overflow checking.
- `conv.i8`: sign-extend signed I4/native stack interpretation; keep I8 bits.
- `conv.u8`: zero-extend I4/native at the source width; keep I8 bits. Native
  reinterpretation through `nuint` must happen before widening.
- `nint`/`nuint` use the **target runtime pointer width**, not the analyzer host.
  Storage/copy/join/call support does not imply native arithmetic support.
- Integer division/remainder, checked operations and field accesses can throw
  where the corresponding instruction would. No operation is moved merely to
  simplify expression spelling.

## References, fields and addresses

Local class tokens and opaque `__cil_type_TOKEN` aliases are scoped to the input
image. Alias comments retain external name and full AssemblyRef identity. Tokens
from different images cannot be compared as if they were globally unique.
No dependency is runtime-loaded or executed by production. Single-image lowering
retains declared external Class identity. The explicit context can independently
bind supplied definitions, without expanding that body admission.

Local assignment may follow locally known ancestors to an exact named external
base TypeRef. A null-base local root is admitted only as top-level System.Object
in an exact admitted core definition scope; raw noncanonical null tags and
unqualified roots are rejected. Local core ValueType/Enum descendants are not
reference receivers. External ancestor chains, interfaces and generic
instantiations are not inferred. Null can be assigned to admitted reference types. Reference
`is null`/`is not null` patterns test identity without dereferencing or invoking
operator equality. Other reference comparisons/joins may still fall back.

`object` is an ordinary reference type, distinct from the Null pseudo-type and
from integer/value/address stack categories. Parameters, locals, returns and
field slots copy the reference without boxing or allocation. Copying does not
dereference the copied object; field receiver checks remain at the access.
An already admitted string, class or int[]/uint[] reference can be assigned to
object, including null; the original reference identity is unchanged. This does
not authorize an implicit object-to-string/class downcast or a value-to-object
conversion. Reference static-field stores use the same assignment check as
instance/local/call transfers. Closed generic references also use these copy and
object-upcast rules. Boxing/unboxing, virtual calls, arbitrary allocation and
mixed-type joins remain separate unsupported
operations. Bounded owned behavior evidence is in
[object reference acceptance](acceptance/corpus/OBJECT_REFERENCES.md).

GenericInst class signatures and vectors have input-scoped TypeId identities. Their
complete canonical keys retain class/value tags, metadata-scoped definitions and
all recursive arguments; equal names are not equality. Opaque __cil_ref_HASH
aliases denote those exact reference types, not wrapper allocations. Both views
retain canonical type-spec bytes and the input-scoped interpretation. Hashes name
aliases only; collisions cannot merge distinct semantic identities.

Arguments can be primitive, named, nested generic and vector shapes, with
VAR/MVAR bound to their actual TypeDef/MethodDef owner and position. Body entry
checks arity, contiguous positions, raw ownership and supported parameter flags.
An unbound or out-of-range variable is not a wildcard. Closed shapes do not
acquire unrelated caller context. Member references never implicitly capture the
caller's variables. See [scoped variables](acceptance/corpus/SCOPED_REFERENCE_VARIABLES.md).
Concrete Nullable<int> value operations, constructed members, variance and
generic receiver specialization remain pending.

A bare generic variable uses the separate GenericValue evaluation category.
__cil_generic_HASH denotes the original generic type, not an object wrapper.
Loads/assignments/returns snapshot/copy that value: references retain identity;
value instantiations copy values while preserving their embedded references.
No boxing, deep copy, layout inference, numeric reinterpretation or generic
reference conversion is implied. Null/I4/different generic variables cannot be
assigned to unknown T without an admitted proof/operation. Initlocals uses
default(T), not a user constructor. See
[generic value copies](acceptance/corpus/GENERIC_VALUE_COPIES.md).
Method, field and plain-local raw signatures are checked before token creation;
oversized RIDs and ignored suffixes cannot admit a body. See
[closed generic references](acceptance/corpus/CLOSED_GENERIC_REFERENCES.md).

Known local nongeneric structs use a separate Value category with exact interned
nominal identity. Their whole-value loads, assignments and returns snapshot/copy
the original target type, preserving embedded references without deep cloning,
boxing, construction or host-layout inference. __cil_value_HASH denotes that
original type, not an execution wrapper. Both views keep canonical signature
evidence. Enum proof remains prior to aggregate admission so normalized integer
semantics cannot disappear into opaque copies.

Base proof, sealed/nongeneric shape, attached byref-like markers and instance-field
signatures are checked. Unknown external values, ref-like types, direct invalid/
open/managed-address fields and special core scalar types remain gated; this is
not full loader/layout verification. Struct field/address/receiver operations
require their own support. See
[whole struct copies](acceptance/corpus/STRUCT_VALUE_COPIES.md).

A value-type instance method receives a managed location of its exact struct
type, not a copy: field reads and writes through it act on the caller's storage.
__cil_value_HASH still names the value type; the address is a separate stack
category. ldarga/ldloca acquire the location of an admitted struct argument or
local, initobj zero-initializes that storage in place (no constructor runs), and
ldflda yields the location of an admitted field. Integer locations reuse
the existing address model with I4/I8 signed and unsigned categories.

Reading a field from a copied value is allowed; writing through a copy is not.
Struct instance calls require the exact location type and render the receiver
with an explicit ref, so a call site cannot silently pass a value. Addresses
cannot cross a basic-block edge or be stored as ordinary values. Plain byref
parameters are admitted; byref returns, ref structs, unproved value types and
struct-element address operations remain independent gates. Bounded EH has its
own region/transfer checks. See
[struct receiver acceptance](acceptance/corpus/STRUCT_RECEIVERS.md).

Instance field reads/writes retain the original token/owner and position. Base
receiver casts remain when needed to avoid name hiding. Initonly writes require
the exact owning instance constructor and a directly proved unchanged entry-this
load in this subset; this this-only restriction is conservative, not an extra
CLI initonly requirement. Static initonly writes require the exact owning static
initializer; field-address operations remain distinct from direct field stores.

Closed SZARRAY descriptors also occupy structural reference slots, including
object[], byte[], string[], struct/enum arrays and jagged vectors. Whole-array
copies never inspect or convert elements, allocate storage or infer covariance.
Pointers/modifiers/unbound variables and non-vector shapes still fall back. Runtime
subtypes already legal for the declared parameter retain their actual identity.
See [vector reference acceptance](acceptance/corpus/ARRAY_REFERENCES.md).

For admitted zero-based one-dimensional int[]/uint[] array operations:

- `new int[length]` / `new uint[length]` represents `newarr` at that instruction:
  fresh array identity, zero initialization, runtime length/allocation failure.
  Only I4 lengths and exact admitted core element identities are lowered.
- `ldlen` reads length immediately, including its null check, producing native
  bits. Any original explicit conversion chain remains visible.
- `ref uint v = ref array[index]` (or ref int) represents immediate `ldelema`:
  bind the original array/index location and perform null/bounds checks **here**,
  without reading the element. Later field rebinding or calls cannot retarget it.
- `ldind.u4` reads that already acquired location at the read instruction into
  an I4 snapshot. `stind.i4` writes I4 bits back at the store instruction, with
  signed/unsigned storage encoding. No second array lookup/check is synthesized.
- Address dup retains the same location. Addresses crossing a basic-block edge,
  byref locals/returns and unsupported prefixes remain gated; admitted byref
  parameters retain their true locations.

## Local enum storage

An exact local enum definition must have an admitted core Enum base and one
unmodified public special value__ instance field of a fixed-width integer type.
Sealed/nongeneric shape, raw base identity, field count/flags and underlying
signature are checked. External enum widths are not guessed. __cil_enum_TOKEN
denotes the original input-scoped value type; both views retain its storage fact.

Loads sign/zero extend from the declared 8/16/32/64-bit storage into I4/I8 stack
bits. Stores truncate to that storage and explicitly restore the enum type.
Arguments/locals/fields/returns and admitted direct calls preserve this boundary.
Integer operations use normalized CLI stack bits, not overloaded enum operators
or inferred boxed objects. Narrow OR shapes emitted without conversion opcodes
are supported; explicit narrow conversion opcodes remain independently gated.
No value receiver, layout-derived address, boxing or enum array allocation is
implied. See [enum storage acceptance](acceptance/corpus/ENUM_STORAGE.md).

## Local virtual dispatch

callvirt on a locally verified class is a distinct operation:
__cil_callvirt_TOKEN(receiver, ...). The receiver and arguments are evaluated
once, in order; the receiver is null-checked first and dispatch to the declared
method belongs to the target runtime. It is never rewritten as a direct call.
Value-type, interface, constrained and opaque external receivers, constructor or
intrinsic callvirt, and newobj remain separately gated. See
[dispatch acceptance](acceptance/corpus/VIRTUAL_DISPATCH.md).

## Reference identity comparison

Equality over reference operands is identity: ceq/beq/bne with identical static
types (or a null operand) render as Object.ReferenceEquals, and cgt.un against
null renders as a non-null identity test. No dereference, overloaded operator or
virtual Equals is introduced. Ordered reference comparisons stay unsupported, and
castclass/isinst remain separate gates so a null test cannot hide a cast or call.
See [equality acceptance](acceptance/corpus/REFERENCE_EQUALITY.md).

## Local allocation

__cil_newobj_TOKEN(args...) on a locally verified concrete class means: evaluate
arguments in order, allocate exactly one fresh object identity at this
instruction, then run the exact constructor on that instance. Repeated reads of
the resulting reference observe the same instance; it is never a re-allocating
getter. External, struct, abstract, generic and open constructors keep their own
blockers. See [allocation acceptance](acceptance/corpus/LOCAL_ALLOCATION.md).

## Exact calls, initialization and exceptions

- `__cil_direct_TOKEN(receiver, ...)`: exact local MethodDef IL `call` target,
  not virtual dispatch and not an added receiver null check. The target's own
  body can still fault. No automatic dependency decompilation is implied.
- `__cil_memberref_call_TOKEN(receiver, ...)`: exact instance MemberRef IL
  `call` with adjacent owner scope/name/raw signature. The original target runtime
  owns resolution, access checks, dispatch rules and body effects. It is not a
  name-only C# call or a claim that the external body was recovered. Test adapters
  may perform that one exact external operation, not proxy a selected body.
- `__cil_ctor_body_TOKEN(receiver, ...)`: instance constructor initialization
  body on an existing receiver. It does not allocate an object or certify a
  complete valid constructor chain. Pre-base field writes stay where they were.
- `__cil_ctor_call_TOKEN(receiver, ...)`: exact same-type delegation or local/
  external immediate-base initialization at the call position, on the same entry
  receiver. Do not repeat allocation or field initializers. Other receiver/base
  contexts remain unsupported.
- `__cil_core_object_ctor_TOKEN(receiver)`: exact admitted immediate core Object
  constructor initialization call, not `new object()` or an erased statement.
  No callvirt-style null check is introduced. Runtime resolution/body semantics
  remain part of the core contract.
- `__cil_tick_count_TOKEN()`: one exact core Environment.TickCount read at that
  position, returning signed I4 bits. It is not analyzer-host time or a constant.
- `__cil_ldstr_TOKEN("...")`: literal load using the original module's
  literal/interning policy. The argument encodes exact UTF-16 code units, including
  embedded nulls and unpaired surrogates; do not convert it lossily or infer fresh
  allocation. Module relaxations can affect interning; generated literal policy
  is not assumed to match every input module.
- `__cil_newobj_TOKEN(paramName, message)`: one fresh exact core
  ArgumentOutOfRangeException .ctor(string,string) allocation/initialization at
  the instruction. It is separate from throwing and ordinary constructor calls.
- `throw value`: throw the existing admitted exception reference and clear the
  evaluation stack. Throwing null raises NullReferenceException. Prior effects
  remain; later instructions are not executed. General object throw, rethrow and
  catch/finally are not implied by this support.

## Nominal R4 and CLI F

`float` in original signatures/storage is nominal float32. `__cil_F` denotes an
R4-nominal CLI evaluation/local value whose internal precision is target-runtime
specified. It is **not defined as analyzer-host float or double**. Copies between
stable values, locals and edges do not introduce storage rounding.

- `__cil_conv_r4(integer)`: interpret I4/I8/native input as signed, round to
  binary32 using CLI nearest rounding at the explicit conv.r4 instruction, then
  retain that exactly representable value in F.
- `__cil_ldc_r4_bits(bits)`: original binary32 constant bits, widened to F without
  changing the represented value. NaN payload conversion details are runtime
  specific; the raw source constant is not discarded.
- `__cil_arg_r4` / `__cil_set_arg_r4`: read/write the nominal R4 argument under
  the target runtime's internal representation contract, not a field store.
- `__cil_load_r4` / `__cil_store_r4`: load/store a fixed R4 storage location.
  Widening preserves the value; storage conversion obeys CLI coercion rules.
- `__cil_div_F(left,right)`: ordered floating division in target F precision,
  with CLI/IEEE special values, not integer zero/overflow checks or an additional
  forced R4 store. The result remains a nominal R4 F value.
- `__cil_call_arg_r4`, `__cil_call_result_r4`, `__cil_return_r4`: explicit nominal
  R4 method-boundary transfer according to the target runtime contract. They do
  not silently redefine every boundary as an implementation-independent fixed
  R4 storage store.
- `__cil_abs_r4_TOKEN`: exact admitted core Math.Abs(float32) overload. Do not
  infer double/int overload binding or replace the whole caller with original
  logic. Its target and argument/result boundaries stay visible.

The owned test adapter uses a Single carrier on local 64-bit .NET 10.0.12.
Bitwise passing results are finite-input **local-runtime** evidence, not an
all-runtime F engine, cross-width proof, or universal NaN/subnormal guarantee.

## Scope of successful bodies

A Decompiled result means the method's operations were expressed within the
admitted model, not source binding/recompilation or formal equivalence for all
malformed/unverifiable inputs. Unsupported semantics retain whole-method IL,
not a partly rewritten function with erased effects. Broad value-type/generic/
virtual/EH support, a complete resolver/verification framework and source-project
export are outside this model. Complete member inventory alone is not behavioral
closure, and a lowered caller can still reference an unrecovered dependency.
