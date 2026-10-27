# Enum storage/conversion: main-session source/IL self-review

Bounded main-session self-review, not independent or formal verification.
manifest.json retains fourteen gained methods with production default readable
output, ILSpy source, original IL, exact references, commands and statuses.
All runs succeed; no explicit ILSpy unknown-result diagnostic appears.
Original IL is authority. The sample covers exact assembly/generated enum
storage, all relevant source categories, narrowing observed before return,
state/effect ordering and an unexpected boxing gain.

| Method | Checked obligations | Verdict |
|---|---|---|
| 0129-0600082F Select.get_Item1 | Assembly-visible CanEliminate storage is exactly int32. Field snapshot/return retains nominal enum identity and bits, not its member names or visibility as layout guesses. | CRUDE-BUT-EQUIVALENT |
| 0129-06000826 TransInnerResult.NewSelect | Enum and raw Boolean arguments remain normalized CLI categories for the exact original constructor operation; remaining references and fresh result preserve argument order and identity. | CRUDE-BUT-EQUIVALENT |
| 0160-0600001D ESByte_Conv_I1_SByte | Generated __value/specialname shape has exact int8 storage; argument extension, explicit low-byte truncation and return truncation preserve negative/high-bit values. | CRUDE-BUT-EQUIVALENT |
| 0162-06000019 SByte_ESByte | Existing int8 bits are returned as the exact generated enum, with no allocation or invented conversion between unrelated enum identities. | CRUDE-BUT-EQUIVALENT |
| 0161-06000003 IntPtr_Conv_I1_SByte | Native-width input narrows to low eight bits then sign-extends to I4; final int8 store is original. No fixed 64-bit host assumption is introduced. | CRUDE-BUT-EQUIVALENT |
| 0160-06000013 UIntPtr_Conv_I1_SByte | Native unsigned input is reinterpreted in the signed native carrier, preserving bits before low-byte truncation/sign extension. | CRUDE-BUT-EQUIVALENT |
| 0000-0600140B generated Printf.Invoke | Prefix field read precedes byte truncation, unsigned widening, original Convert.ToString base-8 call and Concat; no sign extension into formatting. | CRUDE-BUT-EQUIVALENT |
| 0483-06000560 Decimal.get_Scale | Original receiver location/flags field, unsigned shift 16 and low-byte zero extension produce the same I4 result. | CRUDE-BUT-EQUIVALENT |
| 0483-060006DE Math.Abs(short) | Negative branch negates in I4 then truncates to int16 before argument-slot write and second sign test. short.MinValue still reaches original overflow throw. | CRUDE-BUT-EQUIVALENT |
| 0625-0600004D BinXmlSqlDecimal.WriteUI4 | Four byte conversions and stream callvirt operations remain ordered. Earlier writes survive later failure; unsigned shifts and masks retain original bits. | CRUDE-BUT-EQUIVALENT |
| 1041-06000054 CPos.get_Y | Arithmetic shift, signed 16-bit truncation/extension and second arithmetic shift occur in original order, preserving negative coordinates. | CRUDE-BUT-EQUIVALENT |
| 1041-0600059D StreamExts.ReadUInt8 | Exact virtual ReadByte is evaluated once; -1 creates/throws original EndOfStreamException before any byte conversion. Null/stream effects are not changed. | CRUDE-BUT-EQUIVALENT |
| 1042-06000090 SubCellInit.get_Value | Int field snapshot is truncated to byte then returned as the exactly selected foreign SubCell enum; no foreign-layout or name-only inference. | CRUDE-BUT-EQUIVALENT |
| 0164-0600002B TestCases.M41 | clt yields I4 0/1; the original generated UInt16 enum is boxed freshly with those bits. It is not a boxed int or Boolean, nor an allocation-free replacement. | CRUDE-BUT-EQUIVALENT |

No changed-semantic SUSPECT found in the bounded sample. The candidate audit's
foreign values/ordinary value constructors are not claimed as part of this
group. QueryModule.CommitTransInnerResult still has a separate tail-prefix
gate; generated 32-bit/native-return and mixed native/checked cases retain
independent category/conversion gates. No third-party runtime execution or
whole-corpus/cross-runtime equivalence claim is made.

Owned execution separately validates the captured field shapes, eight widths,
all three integer input categories, snapshots/state/boxing, original exception
observations and compiled semantic mutants. The field's spelling/flags are
checked against explicit captured variants, not erased generally.
