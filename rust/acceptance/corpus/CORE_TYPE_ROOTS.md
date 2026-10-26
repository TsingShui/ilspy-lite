# Qualified core type roots

This is a bounded metadata/type-admission milestone, not completion of the
per-implementation-image <= 1% goal or a generic/value-type implementation.

## Type-root admission

A reference hierarchy may terminate at a null base only for a top-level
System.Object definition in an admitted core assembly identity. The definition
identity is checked using the existing full assembly identity normalization and
core contract rules, including public-key token, culture and content type.
Public-key identity labels are not publisher authentication or signature proof.
The result is cached within the one parsed image, not across images or MVIDs.

- The raw TypeDef base coded index is checked before token construction. A
  nonzero tag with RID zero is not an alternative spelling of a null base.
- Same-name roots in other namespaces, wrong-key/name/culture images, cycles and
  generic owners remain unsupported.
- Local core ValueType/Enum ancestry is recognized and still rejected by the
  reference-receiver subset. Fixing a null base does not admit struct or enum
  receivers as references.
- Existing instruction, stack, address, call/constructor, readonly and exception
  checks remain in force. No new opcode or whole-method primitive was added.

Seven owned metadata regression tests passed. They include positive root/field/
constructor paths, local value/enum rejection, counterfeit identities and roots,
noncanonical null tags, valid core-local reference markers, malformed marker
payloads and refusal of a generic marker constructor. The hand-authored metadata fixtures are deterministic and are never
runtime-loaded. Existing owned reference operation behavior is tested separately.

## Reference-marker classification correction

ReferenceAssemblyAttribute can name a constructor MethodDef in the same core
image rather than a MemberRef to an external core type. Both production metadata
classification and the independent PEReader oracle recognize this exact form.
They require a top-level nongeneric marker type in an admitted core scope,
ordinary nongeneric instance constructor flags, exact void parameterless
signature and valid attribute payload. A local lookalike in an ordinary assembly is not proof.
No constructor is executed to classify an image.

Static reinspection found precisely two previously missed markers:

| Input | Original CIL-marked bodies | Effect on target scope |
|---|---:|---|
| legacy mscorlib | 20,869 | Already excluded through the approved pinned exception; no additional body removal |
| .NET 5 System.Runtime reference input | 7,203 | Excluded as a proven reference contract |

The same fixed 1,098 PE origins and 1,098,128 original CIL-marked RVAs remain in
the inventory. No input acquisition, extraction or hash sweep was repeated.
There are now 362 recognized reference snapshots and 614 target managed
snapshots, 319 of which contain target bodies. The target denominator is 770,748.
All original counts and scope reconciliation records remain available.

The first object milestone's 5,540 status gains were measured at its recorded
scope. The additional reference input accounts for 37 of those gains. On the
corrected common implementation scope the object gain is 5,503, not 5,540.
This correction is a classification adjustment, not a semantic regression or a
claimed improvement. The prior report is retained as historical evidence.

## Same-scope production results

Compare the saved post-object results projected to the corrected target ids with
the new production scan. Per-method statuses in the projected baseline were not
recomputed or rewritten.

| Metric | Before root fix | After root fix |
|---|---:|---:|
| Target bodies | 770,748 | 770,748 |
| Decompiled | 140,537 | 150,791 |
| Partial | 630,211 | 619,957 |
| Aggregate unrecovered rate | 81.77% | 80.44% |
| Body-bearing images meeting <= 1% | 33 / 319 | 33 / 319 |
| Incomplete image scans | 0 | 0 |

There are 10,254 newly recovered bodies and zero regressions among previously
recovered bodies. Gains are confined to the three implementation CoreLib inputs:

| Input | Recovered before | Recovered after | Gain |
|---|---:|---:|---:|
| .NET Core 2.2 System.Private.CoreLib | 1,603 | 4,064 | 2,461 |
| Mono 2.6.4 mscorlib | 344 | 3,641 | 3,297 |
| Mono 5.23 mscorlib | 575 | 5,071 | 4,496 |

The primitive CompilationRelaxationsAttribute getter was independently probed in
all three files: it changed from the TypeDef out-of-range fallback to supported
output. This was static analysis, not execution of those original files.
Newly exposed signature, dispatch, value or EH blockers remain Partial. Neither
eliminated error messages nor changed scope counts are counted as recovered code.
The cumulative gain versus the original baseline projected to this same corrected
scope is 15,757 (5,503 object + 10,254 root repair).

## Validation and limits

- Full Rust regression passed, with focused core/reference/object/binding gates.
- Clippy with warnings as errors and rustfmt checks passed.
- Twelve corpus acceptance E2E checks passed, including independent recognition
  of a core-local marker and refusal of a same-name ordinary-assembly marker.
- Existing owned object behavior was rerun against the new release CLI in both
  configurations and presentations: 444 comparisons and four rejected helper
  mutation controls. Unchanged owned compiler artifacts were cached.
- No original third-party assembly, archived compiler or fixture core image was
  runtime-loaded or executed. Static CoreLib recovery counts do not prove every
  recovered CoreLib method's runtime equivalence. The original-corpus behavior
  authorization gate is still pending.
- The release incremental build took 5.47 seconds and the four-process target scan
  took 15.57 seconds locally. No full ILSpy solution build was used.

## Artifacts and continuation

Evidence is under ignored rust/report/core-type-roots/: core-types-red.log,
core-types-green.log, focused/full regression logs, metadata-marker red/green
logs, the independent inventory, role-delta.json, static role/getter probes,
before-core-fix.json, corpus-scan/baseline.json and corpus-delta.json.

The current frozen implementation inventory for capability scans is
`rust/report/core-type-roots/scope-policy/implementation-inventory.json`.
`scope-policy/implementation-baseline.json` is the original pre-object baseline
projected to this policy; `before-core-fix.json` is the post-object baseline at the
same scope. This distinction prevents classification corrections from being
counted as capability gains.

Structural TypeId/generic signatures, exact value/enum transfers and managed
addresses remain in progress under the approved design. The goal is not complete.
