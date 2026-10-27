# General (non-vector) arrays

C# spells general arrays as `T[,]`, `T[,,]` and so on, and reaches their elements
through indexing syntax rather than method calls. ILSpy decodes the array
intrinsics in `ICSharpCode.Decompiler/IL/ILReader.cs::DecodeCall`: a `newobj` on an
array type becomes `NewArr(elementType, arguments)`, `Set` becomes
`StObj(LdElema(target, indices), value)`, `Get` becomes
`LdObj(LdElema(target, indices))` and `Address` becomes `LdElema(target, indices)`.
The rewrite is chosen by `method.DeclaringType.Kind == TypeKind.Array`, so the
declaring type's element and rank fix the argument split (`Set` takes the value as
its last argument; `Get`/`Address` do not).

## What this repository now admits

- `Ty::from_sig_scoped` represents `ELEMENT_TYPE_ARRAY` as a structural reference
  identity. The signature encoder normalizes the element, the rank and the
  declared lower bounds; only zero lower bounds and no declared sizes qualify,
  because those are the shapes C# can spell. `acceptance` evidence for the alias
  states that it is reference identity only: no element, layout or bounds
  inference.
- `Image::array_allocation` resolves `newobj <array TypeSpec>::.ctor` and requires
  one `int32` length per dimension. The lift pops the lengths in reverse, checks
  each against `int32`, and emits `new T[a, b]`, which preserves argument
  evaluation order and the runtime's zero initialization. A general rank-1 array
  (`int32[1]::.ctor`) is refused because C# has no expression for it: `new int[n]`
  allocates a vector, a different runtime type.
- `Image::array_access` resolves `Get`/`Set` on the same TypeSpec owner and checks
  the signature exactly: one `int32` per dimension, `Get` returning the element,
  `Set` taking the element as its last parameter and returning `void`. The lift
  pops a `Set` value first, then the indices, then the receiver, and emits
  `array[i, j]` (or a store to it). Indexing performs the null check and every
  dimension's bounds check at the same point as the intrinsic and throws the same
  exceptions.
- `ldlen` now accepts any array reference, not just a vector. On a general array
  the IL length is the total element count, which is exactly C# `.Length`.
- `Array.Get`, `Array.Set` and `Array.Address` names outside that shape, `Address`
  (a managed reference result needs byref-return support first), `Array.GetLength`
  and `Array.get_Length` (external members on an array receiver) remain refused.

## Evidence

Owned fixture `tests/dotnet/MultidimCases.cs`, builder
`tests/build-multidim-fixtures.py`, suite `tests/multidim_arrays.rs`.

- Red: before the lift and signature changes, `Build`, `Cell`, `Store`,
  `SumDouble` and `Pick` were all `Partial` with
  `unsupported structural stack type shape`; the recorded log shows
  `left: Partial`.
- Green: all five decompile in release and debug; `Build` emits
  `new int[rows, columns]`, `Cell`/`Pick` emit `grid[i, j]`, `Store` emits
  `grid[i, j] = value;`, `SumDouble` reads `grid[0, 0]` and `grid[1, 1]` with
  binary64 arithmetic. `Sum` and `Total` stay `Partial` on
  `unsupported reference relationship argument shape` because they call
  `System.Array` members, a separate gap; the negative test freezes that reason.
- Behavior driver `tests/multidim-roundtrip.py`: compiles the recovered bodies into
  a carrier with structural aliases and compares them with the original fixtures
  through reflection, on null, empty, 1x1, rectangular and 2x2 grids, integer
  indices including -1 and 5, NaN/-0/+infinity doubles and null/empty strings.
  Every call compares the result, the post-call array shape/lengths/contents and
  the exception type. 23 comparisons per configuration; mutants that drop the
  element store and that swap the two allocation lengths are rejected in both
  release and debug.
- Suite: 477 Rust tests, `cargo fmt --check`, clippy with `-D warnings` and the
  release build pass. The reference-relation driver still reports 538 comparisons
  and 54 closed selections.
- Targeted scans: `System.Private.CoreLib` 13,611 -> 13,630 (+19),
  `FSharp.Core` 7,276 -> 7,350 (+74), `OpenRA.Game` 4,013/754 unchanged,
  `OpenRA.Mods.Common` 15,478 -> 15,479 (+1). The fixed-corpus batch scan lands
  with the next batch, as scheduled.

No third-party method body was executed: the behavior driver only runs the owned
fixtures and the recovered bodies compiled from them.
