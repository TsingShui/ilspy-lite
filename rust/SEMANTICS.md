# Agent-readable lowering semantics

Output is C#-like analysis, not reconstructed source or a compilable project.
The CLI defaults to a readable class/method view; `--details` preserves the full
image-scoped metadata/IL evidence record. Original metadata is authoritative.
All operations below are semantic notation; the CLI does not ship a runtime adapter.

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
base TypeRef; external ancestor chains, interfaces and generic instantiations are
not inferred. Null can be assigned to admitted reference types. Reference
`is null`/`is not null` patterns test identity without dereferencing or invoking
operator equality. Other reference comparisons/joins may still fall back.

Instance field reads/writes retain the original token/owner and position. Base
receiver casts remain when needed to avoid name hiding. Initonly writes require
the exact owning instance constructor and a directly proved unchanged entry-this
load in this subset; this this-only restriction is conservative, not an extra
CLI initonly requirement. Static initonly writes remain unsupported.

For admitted zero-based one-dimensional int[]/uint[] arrays:

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
  general byref parameters/locals/returns and prefixes remain unsupported.

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
