# Enum storage/conversion red record

The following are diagnostic excerpts, not substitutes for the complete retained
local logs. Each public seam first observed Partial, then Decompiled after its
bounded implementation.

| Red log | Observed fallback |
|---|---|
| enum-access-red.log | enum needs one ordinary public special value__ instance field |
| narrow-red.log | unsupported opcode conv.i1 at IL_0001 |
| narrow-wide-red.log | conv.i1 source category needs IL fallback |
| generated-storage-red.log | enum needs one ordinary public or assembly-visible special value__ instance field |

enum-runtime-red.log and generated-runtime-red.log independently show owned
runtime acceptance before the production decompiler rejects the captured field
shape. The runtime probes execute only self-authored patched enums. Corrected
enum behavior runs cover ordinary/public, assembly and generated special fields,
all eight widths, four frozen narrow promotions and twelve I4/I8/native
conversion methods, with thirteen compiled semantic mutants per variant/config.

The initial writer dispatch and C# tuple-name authoring errors were corrected;
they are not semantic reds or mutant rejections. No third-party assembly ran.

An intermediate complete scan remains at corpus-scan/ (557,494/213,254);
the generated field shape was subsequently captured and validated in the same
group. Final scan is separate at corpus-scan-final/ (558,757/211,991).
delta-final.json is against the prior committed typed-vector baseline, not the
intermediate result; its regressions list is empty.
