# Nominal value metadata candidate audit

Baseline local 3961eefa1: 554,169 Decompiled / 216,579 Partial of 770,748
bodies; 28.099845864017812% fallback. Fixed implementation scope, exact selected
references, hashes and denominator. No third-party body was executed.

audit/manifest.json retains six production default/ILSpy/original-IL triples.
All ILSpy/IL runs succeed and none has an explicit unknown-result diagnostic.
Original IL is authority. This is candidate triage, not a forecast or admission
of all value operations.

| Method | Observed obligation / remaining proof |
|---|---|
| 0129-060007AD FSharp QueryModule.CommitTransInnerResult | Local CanEliminate enum storage is assembly-visible, not public. Its exact type IL has one assembly specialname rtspecialname int32 value__ and assembly literal constants. Tail dispatch is a separate later gate. |
| 0000-06000145 MatchFailureException..ctor | StreamingContext is a foreign nominal value in the signature; exact base constructor remains separate. No enum/struct guess is justified by a name. |
| 0000-06000205 FSharpMap<T,U>.OnSerializing | Foreign StreamingContext plus generic KeyValuePair construction/copy; vector shape alone proves neither foreign value definition nor constructor signature. |
| 0000-06000222 FSharpMap<T,U>.ComputeHashCode | Foreign constructed KeyValuePair value snapshots, acquired receiver calls and finally/dispose. Caller scopes and value category need independent facts. |
| 0003-060000A6 PlainTextOutput.get_Location | Ordinary local value newobj, with conditional second argument; exact value construction must not allocate a reference or reorder argument checks. |
| 0003-06000179 DomRegion.get_Begin | Local struct receiver fields, followed by ordinary local TextLocation constructor. Receiver identity/layout and construction are independent. |

## First semantic group: enum storage visibility and narrow integer storage

The retained type IL at audit/can-eliminate-type.il.txt comes from ILSpy's
--ilcode type dump of the exact fixed FSharp input. The original instruction
stream separately names CanEliminate. Field accessibility is not evidence of a
different integer representation; requiring public visibility rejects this
legitimate assembly-visible storage shape. Capture the same shape in an owned
metadata mutation before changing the matcher, prove runtime acceptance using
owned inputs, and retain all other shape/flag/width/core-identity checks.

Use the existing public decompile/scan seam and actual enum storage execution
seam. Exercise all eight exact underlying widths in both configurations, local
and exact foreign binding, signed/unsigned bits, copies, boxing and mutations.
The existing four frozen narrow-conversion cases also require explicit owned
execution before promotion. Narrowing must retain low bits and signed extension,
not infer enum layout or normalize Boolean-like storage. Any additional source
categories/conversions remain gated until their own red and behavior evidence.

## Additional captured enum shape

The milestone scan's remaining enum-field refusals are concentrated in the fixed
ExplicitConversions / ImplicitConversions generated inputs, not evidence for
excluding them. The exact type dump audit/0160-ESByte.type.il.txt shows
Enums.ESByte extends exact System.Enum and has one
`public specialname int8 __value` field (no RTSpecialName). The original
representation must be validated on an owned equivalent before admission; do not
drop the shape merely because it is not Roslyn's field spelling. Preserve the
single instance field, exact width/kind, sealed nongeneric enum and core identity.

This extension stays in the enum storage/conversion group. The first group scan
(557,494 / 213,254, +3,325, zero losses/incomplete) is retained as an intermediate
result, not the committed final milestone. Use a new final scan path if this
additional shape changes recovery; never overwrite evidence or infer gains from
the 10,507 remaining first-refusal count.

The other value candidates are not silently included in this first group.
Missing foreign facts, exact references, value receiver/layout, constructor
operations, tail/GVM dispatch and lifetime gates remain independent. No scope
change, selected-body proxy or third-party runtime execution is authorized here.
