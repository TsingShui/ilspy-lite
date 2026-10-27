# Enum storage encodings and narrow integer normalization

## Captured shapes and proofs

The exact fixed FSharp input has an assembly-visible specialname/rtspecialname
int32 value__ field. Generated ExplicitConversions / ImplicitConversions inputs
instead have public specialname __value fields without RTSpecialName.
Both shapes have a single ordinary instance field. Their names/accessibility
do not change integer representation, but no general flag/name relaxation is
inferred.

checked_enum_type admits only these captured pairs:
- value__ with exactly 0x0606 (public) or 0x0603 (assembly).
- __value with exactly 0x0206 (public/specialname).

Sealed nongeneric exact-core Enum ancestry, one instance storage field,
fixed-width I1/U1/I2/U2/I4/U4/I8/U8 storage, no storage modifiers and ordinary
literal static fields remain required. Other names/flags, readonly/marshaled
storage, missing special flags, reserved visibility, counterfeit core identities
and unproved foreign definitions remain gated. Exact foreign binding uses the
same definition proof; reference policy is unchanged.

conv.i1/u1/i2/u2 accept proved I4/I8/native inputs. They explicitly retain low
bits then sign/zero-extend to I4. Narrowing is observed before any later widening,
shift, argument store or call, rather than relying on a final return cast to hide
a wrong stack value. Float/reference sources and other conversions remain gated.
No C# language reconstruction, host enum-layout guess or original-body proxy is
introduced.

## Audit and owned execution

Six candidate triples and four additional generated-layout triples retain
production default/ILSpy/original IL in report/value-metadata/audit/. Type IL
captures both real field encodings. The additional captures used the retained
intermediate release binary before generated-storage admission. Wrong guessed
type lookup is an inconclusive probe, not BOTH_NO or an exclusion.

Public seam reds cover assembly storage, generated special storage, I4
narrowing, then wide/native narrowing. Owned runtime probes load and execute
only patched self-authored enums, independently validating both captured
encodings and underlying bits before production admission. No original fixed
corpus image is executed.

Per configuration and field variant (ordinary/public, assembly-visible and
generated special):
- **3,480 comparisons**: eight exact enum widths, copies, fields/statics,
  identity/boxing/unboxing and exceptions; four promoted narrow enum methods;
  twelve I4/I8/native conversion methods observed as long results.
- **13 compiled semantic mutants**: existing helper, signed/unsigned extension,
  static state and unbox identity controls, plus eight narrowing extension/width
  and wide/native observation controls.
- Integer cases cover boundaries around 8/16 bits, signed extremes, values with
  high 64-bit parts and native extrema. Independent masked arithmetic validates
  expected results; a final enum return cannot conceal a wrong extension.

Assembly and generated foreign-library variants each add **30 comparisons /
9 methods/config** and three existing mutation controls, preserving exact
dependency type identity. Four old narrow negatives execute in ordinary and
patched variants before frozen rows are removed. Wide/native refusal rows are
updated only after their owned positive cases execute. Negative float/reference
categories, field shape/flags and wrong/missing references stay explicit.

Final fmt/clippy and **449 Rust tests** pass; **38 owned driver invocations**
pass. Full commands/statuses: report/value-metadata/behavior-manifest-final.json.
Metadata-writer dispatch and C# tuple-authoring errors are not semantic reds or
mutant rejections. Only corrected compiled mutants count.

## Fixed-corpus milestone

**558,757 Decompiled / 211,991 Partial of 770,748 bodies**, fallback
**27.504579966474125%**, **+4,588 in 101 images** from typed vectors. Zero
regressions/incomplete images; fixed 614 implementation images / 319
body-bearing images, hashes, exclusions, denominator and reference stamps.

OpenRA.Game: **2,898 / 1,869 (+13)**. Common: **12,115 / 6,292 (+12)**.
The retained intermediate scan was 557,494 / 213,254 (+3,325); discovering the
additional legitimate generated shape extended the same group, so final scan
evidence is separate and the earlier failure/result is not overwritten.

First-refusal attribution: conv.u1 1,343; enum shape 1,281; conv.u2 1,184;
conv.i2 464; conv.i1 311; boxing 5. This is not isolated causal proof or a
forecast. Fourteen gained production triples self-reviewed CRUDE-BUT-EQUIVALENT;
no changed-semantic SUSPECT in the bounded sample. See review/review-source.md.

Foreign nominal/constructed value proofs and ordinary local value construction
remain separate candidates. Tail/GVM, unproved references, float/checked/native
conversion and lifetime gates are not relaxed. **Aggregate <=5% is unmet.**
Complete per-image results and residual methods/reasons remain in
report/value-metadata/corpus-scan-final/. No independent/formal verification,
third-party execution or whole-corpus/cross-runtime equivalence is claimed.
