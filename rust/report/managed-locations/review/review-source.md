# Main-session managed-location source/IL self-review

Production release CLI, default readable output; exact declared references and
commands/statuses are in `manifest.json`. All 36 commands completed with exit 0.
Original IL controls the semantic judgment, not ILSpy's source appearance.
Nine methods are new recoveries; the last three restore initial readonly-address
regressions. No original third-party code was executed.

| slug | changed-operation check | judgment |
|---|---|---|
| 0000-0600021D | FSharpMap TryGetValue retains comparer/key/tree evaluation order, caller !1 pointer and exact MethodSpec; no copied out cell | CRUDE-BUT-EQUIVALENT |
| 0002-06000146 | string out write occurs before integer decrement; original integer read/box and subsequent read/store snapshots stay ordered | CRUDE-BUT-EQUIVALENT |
| 0003-0600008A | local CustomAttribute reference slot is initialized then passed by location to the same helper; ref/out are the same CLR pointer ABI, not a standalone C# source promise | CRUDE-BUT-EQUIVALENT |
| 0003-06001FC4 | ldobj snapshots the complete this value before static equality; other argument remains a value copy | CRUDE-BUT-EQUIVALENT |
| 0004-06001183 | foreign SymbolInfo locals preserve copies and exact direct ref receiver calls, including short-circuiting the second equality | CRUDE-BUT-EQUIVALENT |
| 1041-06000068 | two CPos out locations feed exact generic static calls; failed first conversion skips the second; comparisons read completed value copies | CRUDE-BUT-EQUIVALENT |
| 1041-06000036 | ActorID initonly address is acquired before UInt32.ToString; concatenation, IsInWorld call and conditional append retain their IL order | CRUDE-BUT-EQUIVALENT |
| 0003-060006A5 | static array-reference location is passed unchanged with byte-backed SymbolKind conversion; no array snapshot replaces the pointer | CRUDE-BUT-EQUIVALENT |
| 1042-06003B2A | captured DamageState field location, preceding arguments and the actual enum identity survive forwarding to the same helper | CRUDE-BUT-EQUIVALENT |
| 0403-06000112 | conditional paths and callvirt checks remain; initonly ExpressionBinder address feeds the direct getter, whose BindingContext result is copied before its own address is acquired | CRUDE-BUT-EQUIVALENT |
| 0467-060001F2 | exact initonly _stats pointer then nested outStats pointer, one messages read, unsigned widening to Int64 bits | CRUDE-BUT-EQUIVALENT |
| 0999-06000B44 | handle argument address, row arithmetic, exact readonly Block address, offset evaluation and direct PeekUInt16 remain ordered; returned width is narrowed correctly | CRUDE-BUT-EQUIVALENT |

Twelve crude-but-equivalent, zero changed-semantic SUSPECT in this bounded sample.
Aliases, generic contexts, metadata-named fields, explicit ref operands and exact
runtime leaves are semantic notation. Some headers/source forms (including
virtual struct/interface-slot presentation and ref versus out spelling) are not
standalone C# compilability claims. The owned carrier tests explicitly rebind
headers and types while retaining lowered body operations and true locations.

## Defects exposed and bounded repairs

- A default-view red test reproduced replacing an indirect read snapshot with a
  mutable ref alias after a write. Ref-bound bodies keep explicit snapshots.
- Constructor this-slot exposure could bypass entry-this provenance without starg;
  it remains refused, and existing readonly constructor regressions pass.
- The initial full scan lost 824 methods to a blanket readonly-address gate.
  Six pre-repair triples and owned red/green tests demonstrate legitimate field
  addresses. Exact field-address leaves preserve the actual runtime operation;
  direct readonly field stores remain governed separately.
- A raw Boolean byte-2 variant exposed actual state drift: stind.i1 became a
  C# truth-value store of 1. Byte-preserving Boolean loads/stores, returns and
  boxing now pass ordinary/initonly/raw-bit gates and reject normalization
  mutants. ILSpy also prints a truth-value store, so it is not the oracle.
- External-enum execution no longer skips admitted class/struct copies. Its
  descriptor checks values, input state and reference identity; three meaningful
  mutants fail rather than accepting an unrelated runtime/compile error.

Owned execution and mutations supplement this review, not vice versa.
No formal, independent or full third-party execution claim is made.
