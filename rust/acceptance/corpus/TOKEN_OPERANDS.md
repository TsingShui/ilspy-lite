# Token-operand values

`ldtoken` and `ldftn` no longer fall back on their own. Their results are exact
runtime values, following ILSpy's `LdTypeToken`/`LdMemberToken`/`LdFtn`
instructions:

- `ldtoken` on a type, field or method row yields the corresponding exact core
  handle struct (`RuntimeTypeHandle`, `RuntimeFieldHandle`,
  `RuntimeMethodHandle`). The category comes from the operand row, not from the
  consuming signature, and the value keeps the original token identity.
- `ldftn` yields a native function pointer value carrying the exact method
  token, with no receiver pop and no virtual dispatch.

Consumers keep their original evaluation order and calls: this group does not
fold `Type.GetTypeFromHandle` into `typeof(...)` (ILSpy does that for
presentation only), does not invoke reflection to materialise a member, and does
not allocate anything on the handle's behalf.

Owned fixture `tests/dotnet/TokenCases.cs` covers `typeof(T)` over a generic
parameter, a closed core type, a nested type and a vector, an array initializer
(`ldtoken` field plus `RuntimeHelpers.InitializeArray`), and static/instance
delegate creation (`ldfn` plus a delegate constructor). With host references all twelve bodies recover, including the two delegate
bodies (`ldftn` plus a constructed `Func<,>` constructor); without references they
stop at the allocation-fact gate, which stays a negative boundary.

Rust seam coverage refuses UserString and out-of-range operands, and distinguishes
field and method MemberRefs by their signatures. An owned behavior adapter now
executes recovered generic/closed type-token, array and delegate bodies: 20
comparisons per configuration with three semantic mutants rejected. Its
Reflection.Emit leaves contain the specific `ldtoken`, `ldftn` and delegate
`newobj` operations, not the selected original method bodies. The adapter and
logs are under `report/constructed-foreign-batch/owned/`; this is bounded
evidence, not a claim for all tokens or runtimes.
