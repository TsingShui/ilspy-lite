# Typed managed locations and byref operands

## Scope and result

The unchanged fixed implementation corpus contains 614 managed inputs, 319
body-bearing images and **770,748 CIL bodies**. Final production scan:

- **438,801 Decompiled / 331,947 Partial: 43.06816% aggregate fallback**.
- **21,002 newly recovered methods in 188 images**, zero recovered-method
  regressions and zero incomplete images. Decrease: 2.72489 percentage points.
- OpenRA.Game: 2,473 / 2,294 (+159 recovered); OpenRA.Mods.Common:
  9,714 / 8,693 (+226 recovered).
- The aggregate <=5% objective is **not met**. No denominator, reference policy,
  input version, exclusion or third-party execution changes.

Evidence: `report/managed-locations/{AUDIT.md,delta-validated.json}`,
`corpus-scan-validated/baseline.json`, and `review/{manifest.json,review-source.md}`.
The initial scan and its 824 readonly-address regressions remain in
`corpus-scan/` and `delta.json`; they are not a passing milestone.

## Representation and operations

An image-scoped arena interns managed-location **element types**, not storage
identity. Existing integer/struct address variants stay canonical. Acquired ref
bindings preserve the original location and are never substituted with a value
snapshot or a field/array expression that can be reevaluated after effects.

Plain ref/out parameters, scalar/reference/generic argument and local addresses,
and admitted field/static addresses feed invariant ref call operands. Checked
ldobj/stobj/cpobj/initobj and indirect operations preserve element identity,
storage widths, sign/zero extension and CLI stack normalization. Values read
through aliases are snapshotted at the actual read, before any subsequent write.

Exact core scalar owners and already-proved struct/enum owners can receive
direct calls through their actual managed locations. This does not introduce
boxing, virtual dispatch, foreign value-layout guessing or copied receivers.

Initonly field addresses use precise `__cil_ldflda_TOKEN` /
`__cil_ldsflda_TOKEN` operations returning the actual managed pointer.
Source-level writable borrowing of a readonly field is not fabricated.
Field checks/static initialization occur at acquisition, before later arguments.
Direct initonly stfld/stsfld permission checks remain separate. Constructor
this-slot address exposure is refused without original-entry-this provenance.

Readable flat compaction is disabled in bodies with ref bindings: a ref alias is
mutable storage, not an immutable value eligible to replace a pre-write snapshot.

## Owned execution

The default recovered body text is compiled with metadata/type/header rebinding.
Ref/out carrier headers may use ref to satisfy C# definite assignment, but exact
CLR ByRef and element signatures are checked. Dynamic invocation supplies actual
typed pointers to shared cells, not reflection copy-in/copy-out. Recovered support
calls invoke recovered support bodies.

Release/debug ordinary, owned Number-initonly and raw-Boolean variants execute
**960 / 951 comparisons, 47 / 46 closed selections**, respectively: **5,733
comparisons total**.
They cover aliased cells, reference identity, generic/struct copies, signed and
unsigned narrow/native/wide storage, bool/enum storage, Single bit patterns,
field/array/static acquisition, receiver replacement and writes before exceptions.
The initonly/raw-bit variants compare original patched IL, not guessed source
behavior. Five mutants fail in each run, plus two copied-field-location and four
Boolean-normalization controls: 36 rejected mutants total. Boolean storage byte
2 exposed and fixed truth-value normalization; explicit load/store helpers retain
the byte through copies, returns and boxing. ILSpy also normalized this example;
original IL and owned execution, not source appearance, determine the result. Field adapters emit only the original field-address operation;
the primitive receiver adapter performs the original Int32.ToString call.
None executes a selected method body as a proxy.

Promoted legacy cases remain executable:
- MethodSpec: 125 comparisons / 14 selections per configuration.
- Reference calls: 110 / 14 for each configuration and ordinary/unbox variant.
- Struct copies: 495 comparisons per configuration, including former byref refusal;
  seven mutants include corruption of the byref source.
- Struct receivers: 36 / 18 per configuration, including byref/local-call/helper
  promotions; three store mutants.
- Virtual gate: 55 / 11 plus 9 / 2 direct base/value receiver vectors, including
  signed values, null behavior and result freshness; three dispatch/result mutants.
- External enums: all nine selected methods, 30 comparisons per configuration,
  no skipped class/struct copies; numeric, copy and identity mutants.

Twenty relevant owned behavior drivers pass; their commands/statuses are in
`report/managed-locations/behavior-manifest.json`. Cargo nextest: **399 passed**;
fmt and all-target clippy with warnings denied pass. No third-party assembly or
metadata-only core toy is runtime-loaded.

## Boundaries and self-review

Byref results/returns, byref locals, pointer-slot rebinding, managed addresses
crossing CFG edges, unsupported modifiers/prefixes, unmanaged pointers and
unproved value/reference relationships remain gated. GVM keeps its existing
runtime lookup-order refusal. General value layouts, constructed value calls,
allocation and ancestry require their own proofs.

The main session inspected twelve production default/ILSpy/original-IL triples:
nine new recoveries and three restored readonly-address regressions.
All twelve are CRUDE-BUT-EQUIVALENT for the changed operations; no changed-semantic
SUSPECT case remains in that sample. This is self-review and owned execution,
not independent review, whole-corpus C# compilation or formal/cross-runtime
equivalence certification.
