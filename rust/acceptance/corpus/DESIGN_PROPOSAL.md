# Recovery expansion proposal

Status: the reference-only skip policy is implemented. The user explicitly
approved the architecture direction and first object-reference slice. Semantic
capabilities are implemented incrementally with their own E2E evidence; approval
of a direction is not evidence that a capability is complete.

## Evidence and the acceptance decision

The fixed corpus contains 976 unique managed snapshots and 1,098,128
CIL-marked method RVAs. All 1,164,708 MethodDef rows were independently enumerated
and matched to exactly one production scan record, including declarations.
The production baseline is 146,752 recovered / 951,376 partial (86.64% partial).

There are 291,764 zero-code RVAs in 124 images. Of these, 270,910 are in images
with an independently checked, core-scoped ReferenceAssemblyAttribute marker.
The other 20,854 are in the pinned dotnet/legacy/mscorlib.dll snapshot.
An absent marker is not evidence that the latter supplies executable code.

For example, the marked System.ServiceModel input has 23,537 CIL-marked RVAs,
all with zero bytes. There is no implementation to recover. A fabricated return,
a generic original-IL execution adapter, or relabeling every stub Decompiled
cannot satisfy the semantic contract.

### Confirmed acceptance policy

Retain every acquired input, origin, method and original result. Reference-only
images are skipped, with no separate reference metadata/API acceptance gate.

1. Every image with executable CIL implementations must independently satisfy
   <= 1% unrecovered. Failed, partial and omitted target bodies count as
   unrecovered. Incomplete implementation inputs/scans cannot pass.
2. Reference contracts retain skip reasons, exact input provenance and raw stub
   evidence. They are not relabeled Decompiled and are not implementation-rate
   denominators. No missing implementation is fabricated.
3. The demonstrated unmarked legacy mscorlib shell is excluded under the user's
   explicit scope revision, using its pinned commit/hash/MVID/extent evidence.
   Unknown unmarked empty/malformed inputs are not excluded by name or difficulty.
4. Preserve the original all-RVA baseline and a reconciliation table. Every
   method belongs to a documented bucket; classification alone recovers zero
   methods. No further reference-scope confirmation is required.

There are 360 marked reference snapshots (299,308 CIL-marked RVAs) and the one
unmarked legacy input (20,869 RVAs). The remaining 615 managed snapshots contain
777,951 CIL bodies, with 642,385 current partial results (82.57%). These are the
approved implementation-only baseline counts, not a semantic improvement.
Nonzero stubs in marked reference assemblies also remain contract evidence;
nonzero code does not make a reference assembly an implementation image.

### Other choices

- Keep the original all-RVA gate unchanged: honest completion is impossible for
  the demonstrated all-empty images without additional original implementations.
- Supply missing implementation images: retain reference contracts and record
  additional inputs under explicit user-approved scope, identity and provenance.
  Do not silently replace the pinned files or claim their bodies were present.

The scope revision has been confirmed through /goal-tweak. Original and scoped
results are reconciled in rust/report/fallback-corpus/implementation-scope/.

## Architecture direction

Keep the Rust typed IL/CFG and lift pipeline. Do not introduce a C# AST, a CLR
emulator, an input executor or a source-project export framework. The decoder
remains pinned to cildec 0.1.0 unless a separately demonstrated decoder defect
requires a dependency decision.

The important internal seam is between scoped metadata/type facts and instruction
lowering. Identity, generic substitution and assignment facts belong in one deep
module, not separate string-based approximations in every opcode or renderer.
Its interface must distinguish known, unresolved and conflicting facts. A missing
fact may block one operation without suppressing independently supported peers.

### D1: Qualified type representation

- Keep primitive storage and CLI stack categories distinct. Object is an ordinary
  reference type, not the Null pseudo-type and not a box for every stack value.
- For structural types, use a session-owned interner and small Copy TypeId handles.
  Nodes represent named types, generic instantiations, owner-scoped type/method
  variables, arrays, managed/unmanaged pointers and modifier-bearing types.
  Initially intern only shapes consumed by an implemented semantic slice.
- A named identity includes its input snapshot/module and metadata token. An
  unresolved TypeRef retains its exact requested scope. Equal display names are
  not equality, and equal numeric tokens in different inputs are not equality.
- Generic parameters include their declaring owner and position. Type parameters
  and method parameters never share a namespace. Substitute only in the correct
  owner context, with checked arity, bounds and recursive work limits.
- Preserve ordinary Ty primitives during migration; replace special int/uint
  arrays and element-address variants only when the general representation has
  equivalent coverage. No all-at-once rewrite is needed for the first slice.

Rejected alternative: adding a Ty enum variant for every real-world constructed
signature. It duplicates semantics and cannot represent nested generic shapes.
An owned recursive Ty tree is viable but would remove Copy from the existing
stack model and spread cloning/identity decisions through lift and rendering.
The interner is recommended for locality and explicit image scoping, not as a
promise that arbitrary type shapes are admitted immediately.

### D2: Reuse exact static binding, not a second resolver

- Extend existing DependencyContext binding to expose typed facts rather than
  parsing its diagnostic signature strings back into semantic types.
- Compare declared member signatures before specialization, then instantiate the
  selected member in the exact owner/method context. Preserve overloads whose
  specialized shapes happen to coincide.
- Use exact scoped core identity to determine enum base/storage and primitive
  identities. Enum arithmetic requires the verified underlying integer type;
  unknown external ValueType must not default to int or reference semantics.
- Reference contracts may prove signatures/layout relationships but never prove
  implementation behavior. Exact missing/conflicting/substituted metadata facts
  remain independently visible. No automatic same-major substitution for proofs.
- Keep byte-only interfaces filesystem-free. Dependency-aware decompilation and
  scan must receive the same explicit immutable context to use new binding facts.
  Scan must never be a more permissive recovery path than method output.

### D3: Separate value snapshots from locations

- Struct values are typed copies; references preserve object identity. Use explicit
  copy operations when the renderer cannot safely express value semantics.
- Managed addresses retain the original acquired location and element type,
  readonly state and provenance. Taking an address is not reading a value.
- Carry enough facts at CFG joins to preserve location identity or reject the
  join. Do not approximate an address with its current scalar value or reacquire
  array/field locations after an intervening effect.
- Struct receivers, ldarga/ldloca/ldflda/ldelema, ldobj/stobj/initobj/cpobj and
  constrained calls enter through bounded, independently verified slices.
  A general GC, object layout engine or host-pointer simulation is unnecessary.

### D4: Explicit call and construction operations

- Keep direct call, virtual/interface dispatch and allocation/initialization as
  different operations. Retain exact original target token, owner and instantiated
  signature even when ordinary C# invocation is not sound.
- callvirt must retain its null check and target-runtime dispatch, including
  nonvirtual targets. constrained dispatch must preserve value/reference receiver
  and boxing rules, not unconditionally box or mutate a copied receiver.
- newobj must allocate exactly once and execute the selected constructor in the
  original order. Constructor-body execution on an existing receiver is separate.
- Generic/static initialization and volatile/unaligned/tail semantics need their
  own admission checks; no prefix is erased to admit its following instruction.
- Intrinsics denote one defined primitive or exact leaf operation, not an entire
  selected method or an interpreter for its original IL. Selected/support method
  bodies in behavior gates cannot be delegated to original implementations.

### D5: Effect-aware CFG and exception regions

- Extend the existing CFG with region membership, normal/exceptional successors
  and explicit transfer operations before implementing EH lowering.
- Preserve catch/filter selection order, exception stack entry, leave stack
  clearing, rethrow identity, and finally/fault execution under normal and
  exceptional exits. Filter effects and nested region order must remain visible.
- A structured low-level region representation may render C#-like regions or
  documented primitive operations. A generic execute-original-EH operation is not
  recovered semantics. A flattened goto must not bypass region cleanup.
- Keep stable value evaluation and fault-producing operation order. Reference
  joins, unreachable code, initialization state and numeric widening require
  separate verified rules, not relaxed stack equality checks everywhere.

### D6: Transformation order

1. Strict raw metadata/signature validation and image/contract role evidence.
2. Scope-aware type/member binding and method generic context.
3. Decode IL/prefixes/regions and build normal plus required exceptional CFG.
4. Typed stack/location dataflow with explicit values, effects and region transfers.
5. Render C#-like output retaining operation and identity evidence.
6. Apply only proven readable simplifications; they cannot change recovery status
   or hide failed operations. Details, readable output and scan share admission.

A full new optimizer or C# syntax recovery pass is not part of this sequence.
Each major addition to these decisions must be approved before implementation.

## Incremental implementation order

First-failure counts are prioritization evidence, not projected recovered counts.
Later blockers remain present after the first blocker is implemented.

1. Acceptance scope is complete: retain original metrics and the implemented
   reference skip reconciliation. Do not reacquire or rehash unchanged inputs.
2. First semantic slice: object reference signatures, identity-preserving copies,
   null/reference assignment and reference-to-object upcasts. No implicit boxing,
   newobj, generic dispatch or EH admission. This can extend the existing Ty model
   without forcing a complete interner migration.
3. Introduce structural TypeId representation for a bounded reference-only generic
   and array subset. Then exact enum storage/stack transfers and enum operations.
4. Implement generic owner/MethodSpec member facts and controlled value/location
   transfer, then struct copies/receivers and address instructions.
5. Implement callvirt/interface dispatch, general construction and constrained
   operations with the prerequisite facts and explicit side effects.
6. Add region-aware CFG and bounded EH, followed by remaining numeric, prefix,
   initialization, pointer and calling-convention cases actually required by the
   fixed corpus. Do not automatically reconstruct new C# language syntax.
7. At each completed capability, rescan the full fixed corpus with one production
   release build, report newly recovered methods and any regressions, and rerank
   residual blockers without deleting or replacing inputs.

## E2E verification without excessive compilation

- Tests cross the real production CLI and byte-library interface. Use owned
  minimal assemblies and the existing behavior adapters, not mocked lowering.
- Each semantic slice gets a failing E2E/regression case before implementation,
  then a green run. Do not require a large preliminary unit-test framework or
  repeated approval of ordinary test/tool details.
- For the first object slice, include null, reference identity, pass/return through
  fields/locals, state mutations, and rejected boxing/value-to-reference paths.
  Verify unchanged operation order and malformed-input refusal.
- Compile owned fixtures/adapters only when their content changes; build Rust
  incrementally for focused E2E. One release build is reused for a capability's
  complete corpus scan. No full ILSpy solution build for a Rust change.
- Major capabilities require behavior comparison of returns, relevant state,
  effects and exceptions plus negative/mutation checks against vacuous success.
- The original corpus remains static-only until the user approves specific real
  methods/tests, environment and risks. Unsupported platforms and skipped behavior
  cases are reported, not called verified.

## Confirmed decisions

The user approved D1-D6 as the architecture direction and the first bounded
object-reference slice with owned E2E behavior checks. New major deviations still
require a decision. Original third-party assembly execution is not authorized.

Evidence: rust/report/fallback-corpus/BASELINE.md, baseline-final/baseline.json,
baseline-comparison.json, baseline-integrity.json, scope-contradiction.json and
scope-options-comparison.json. No estimated final fallback rate is promised.
