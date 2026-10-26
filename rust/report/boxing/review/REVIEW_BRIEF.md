# Review brief: boxing (core scalars and local enums)

Inputs: `report/boxing/review/ilspy/*.txt` (ILSpy output, produced by verification) versus
our output for the same methods (`dumps/*.source.txt`, refresh them against the current
build if the fixture table changed). Original IL (`dumps/*.il.txt`) is third evidence,
used only to settle a disagreement between the two decompilers.

Pairs: fixture-BoxInt, fixture-BoxEnum, fixture-BoxStruct, fixture-UnboxInt,
fixture-UnsupportedBoxGeneric, plus every method in CORPUS_SAMPLES.md.

Judge, per pair:
1. Does the boxed expression box the declared type (an enum must not be boxed as its
   storage integer), and is exactly one fresh instance created at that point?
2. Does the unbox check the runtime type and copy the value out, with null and
   wrong-type behaviour preserved?
3. Are the gated pairs (struct, constructed value, generic parameter) still gated for
   the stated reason, and does ILSpy itself recover them or also fall back? A gated pair
   that ILSpy prints in full is evidence for the next slice, not a defect.
4. Evaluation order, receiver count and value copying match the IL.

Verdicts: EQUIVALENT, CRUDE-BUT-EQUIVALENT, SUSPECT, INSUFFICIENT-EVIDENCE. A SUSPECT
row must quote both outputs. Nothing may be executed or built. Write
`report/boxing/review/review-source.md`; reply with the counts and the SUSPECT list.
