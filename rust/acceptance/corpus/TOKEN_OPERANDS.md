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
delegate creation (`ldfn` plus a delegate constructor). With host references the
ten reachable bodies recover; the two delegate bodies stop at the separate
external-constructed-member gate, which is retained as a negative boundary.

Rust seam coverage: token operands are no longer reported as an unsupported
opcode; a UserString operand is refused as "not a type, field or method token";
an out-of-range row is refused during body evidence decoding. No execution
adapter is provided for token leaves, so this group's evidence is static plus
the seam tests; it claims no executed equivalence for the handle bodies.
