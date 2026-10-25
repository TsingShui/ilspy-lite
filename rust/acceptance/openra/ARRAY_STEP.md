# First array step: integer element reads and writes

## Implemented scope

The production lowering accepts single-dimensional, zero-based int[] and uint[]
references in method signatures, fields, locals and returns. It handles
ldelem.i4 / ldelem.u4 and stelem.i4 with I4 indices. Both arrays have the same
32-bit evaluation-stack element representation; unsigned values retain explicit
bit reinterpretation at load/store boundaries.

Array references are stable snapshots. Reads execute immediately, and stores
execute only after their original value-producing operations. No entry null
check, delayed array lookup or expression duplication is introduced. Null stores
and returns, and same-type stack joins, use the existing reference model.
Array references cannot act as arithmetic operands or class receivers.

This deliberately does not add a general recursive type framework. The two
array kinds cover the next real-world target; unsupported element types and
shapes remain whole-method fallbacks. There is no new Cargo dependency.

## Validation

- Array-read, array-store and null-return tests were observed red before their
  corresponding production changes and then green.
- Six array integration tests passed, including invalid operands, mismatched
  array assignments, unsupported element types and ldelema fallback.
- Owned fixture execution: 11 recovered bodies, 453 return/exception/state
  comparisons per Release/Debug configuration, all passed.
- These cases include null/empty arrays, negative and out-of-range indices, high
  unsigned bits, old values after mutation, field identity, aliasing, array joins
  and call side effects before a failing store. The call target is also recovered;
  adapters do not delegate recovered bodies to the original implementation.
- Existing owned reference regression: 522 comparisons per configuration, passed.
- Existing integer regression: 735 comparisons per configuration, passed.
- cargo fmt --check, strict clippy, and Debug/Release Cargo suites passed.

These are bounded differential tests, not all-input equivalence. Ordinary Cargo
tests consume committed fixtures and require no .NET. The optional behavioral
gate uses the installed .NET 10 compiler/runtime without restore or SDK changes.

    python3 rust/tests/reference-roundtrip.py --arrays

## OpenRA status, not behavioral acceptance

The existing local pinned inputs were reused without downloading, rehashing or
recounting sources. Only the three selected random-method outputs were inspected:

| Method | Current lowering | Behavioral gate |
|---|---|---|
| MersenneTwister..ctor(int32) | Partial | Not run |
| MersenneTwister.Next() | Decompiled body | Not run |
| MersenneTwister.Generate() | Partial | Not run |

Next retains an exact direct call to Generate. A lowerable caller body is not a
recovered call dependency or a working recovered random generator. No OpenRA
random executions or stage-2 success are claimed. The stage-1 behavior results
remain historical evidence; the whole external corpus was not rerun here.

## Remaining small steps

1. Model ldlen and the conv.i4 / conv.i8 / conv.u8 operations used by the loop
   condition. Do not silently equate native int with I4 on a 64-bit host.
2. Model ldelema followed by dup, ldind.u4 and stind.i4 as a stable managed
   element address. Null/bounds checks occur when taking the address, not when
   eventually reading or writing it; this is not ordinary delayed indexing.
3. Handle newarr and the exact constructor/base-call contract before accepting
   the seed constructor. Do not erase the base call.
4. Execute all recovered random bodies against the pinned OpenRA implementation,
   including seed state and sequences crossing at least two state regenerations.

Allocation, lengths, conversions, element addresses, reference-element arrays,
multidimensional arrays, generics and EH are not included in this commit.
