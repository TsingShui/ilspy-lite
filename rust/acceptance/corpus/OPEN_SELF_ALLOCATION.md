# Allocating the enclosing generic type from inside it

An F# union/option constructor emits `newobj` with a **MethodDef** constructor whose
declaring type is the generic type itself (`FSharpOption<!0>::.ctor`). The allocation
path refused every generic owner whose call carried no explicit instantiation
(`allocation owner must be a concrete nongeneric class`), even when the call is made
inside that type's own methods, where the caller's type scope names exactly those
parameters (1,374 methods in the last fixed scan).

## Admission

- When the `newobj` target is a MethodDef constructor of a generic type, allocation
  is admitted only if the caller's type scope is that same TypeDef with the same
  arity (`context.type_scope.owner == owner && count == arity`). The allocated type
  is the open self instantiation `Ty::Class(owner)`, which the existing open-self
  equivalence already handles; no argument is guessed and no site outside the type
  is admitted.
- Constructor identity, instance/void signature checks, argument substitution and
  the argument list are unchanged (`member_parameter` with the caller's context).

## Evidence

- Red: `FSharpOption<T>.Some(T)` (`0600017F`) was `Partial` on
  `allocation owner must be a concrete nongeneric class` (recorded log); it now
  decompiles, and the allocated type is the open self `FSharpOption<!0>`.
  ILSpy emits the same `new FSharpOption<T>(...)` shape.
- Targeted scans: `FSharp.Core` 8,631 -> 8,668 (+37, all of them previously this
  reason), `System.Private.CoreLib` 15,113 -> 15,132 (+19),
  `OpenRA.Mods.Common` 16,775 -> 16,788 (+13), `OpenRA.Game` 4,242 -> 4,261 (+19).
- 488 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. The form is F#-specific (C# emits a TypeSpec parent, already handled), so
  the owned construction drivers keep their existing coverage.
