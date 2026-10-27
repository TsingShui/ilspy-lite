# Token operands: ldtoken and ldftn

## ILSpy mechanism

- `ICSharpCode.Decompiler/IL/ILReader.cs` reads `ldtoken` into
  `LdTypeToken(type)` when the operand kind is a type, otherwise
  `LdMemberToken(member)`; anything else is a `BadImageFormatException`.
- `IL/Instructions.cs`: `LdTypeToken.ResultType` is `VT` and `InferType` is
  `KnownTypeCode.RuntimeTypeHandle`; `LdMemberToken.InferType` is
  `RuntimeFieldHandle` or `RuntimeMethodHandle` depending on the member kind.
- `ILReader.cs` reads `ldftn` into `LdFtn(method)`; `CSharp/Transforms/
  DelegateConstruction.cs` recognises the `newobj delegate(object, ldftn)` shape.
- `CSharp/Transforms/ReplaceMethodCallsWithOperators.cs` folds
  `System.Type.GetTypeFromHandle(ldtoken T)` into `typeof(T)`; that fold is a
  presentation choice, the handle itself is a first-class value.

So the handle category follows the operand row, never the consumer signature.

## Adopted in Rust

- `ldtoken` maps TypeDef/TypeRef/TypeSpec to `HandleKind::Type`, Field to
  `Field`, and MethodDef/MemberRef/MethodSpec to `Method`; the operand row is
  checked for existence (and a TypeSpec signature is parsed) before a handle is
  produced. The value is the exact core handle struct
  (`System.RuntimeTypeHandle`/`RuntimeFieldHandle`/`RuntimeMethodHandle`),
  taken from the cached value-type fact or, when no fact was requested, from the
  signature identity of the same TypeRef so a consumer parameter stays identical.
- `ldftn` produces a `nint` value carrying the exact method token. No receiver
  is popped; delegate construction still consumes it as the `IntPtr` argument.
- No reflection call, dispatch, allocation or reordering is introduced.

## Audit triples

Five pre-change ldftn bodies (FSharp.Core `op_Implicit`, `ToConverter`,
`QueryBuilder.Where`, `QueryBuilder.Exists`) and six pre-change ldtoken bodies
(`DefaultUnresolvedAssembly..cctor`, `Unsafe.SizeOf<T>`, `Exts.HasAttribute<T>`,
`Enum<T>.GetValues`, `CryptoUtil..cctor`, `ActorInitLoader.CanConvertFrom`)
retain production default, ILSpy and original IL in `audit/` plus
`report/runtime-handles/audit/`. Every ILSpy run succeeds without unknown-result
diagnostics; original IL is authoritative. No selected input was executed.

## Boundaries retained

- A delegate constructor is an external constructed member and stays gated with
  its own reason; `ldftn` itself is no longer the blocker.
- `ldvirtftn`, function-pointer type signatures, varargs, and consumers that
  expect a handle struct this image never names remain gated.
- Bad operand tables and missing rows are refused; they cannot produce a handle.
