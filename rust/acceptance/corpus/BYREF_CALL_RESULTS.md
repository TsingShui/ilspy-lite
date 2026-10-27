# Managed references returned by calls

A call whose return type is `T&` yields a managed pointer (`get_Item` of a Span-like
type, a ref-returning helper, an enumerator's current value slot). The lift refused
every such call with `byref call result needs escape/lifetime analysis`, so a
following `ldobj`/`ldind`/`ldfld` could never run.

## Admission

- The call arm no longer refuses a byref result. The returned reference is bound as
  a ref local (`ref T vN = ref <call>;`) by the existing `push` path for address
  kinds; later reads and writes through that alias keep the callee's referent and
  the same dereference point as the original pointer. A null managed pointer throws
  where the alias is dereferenced, which is the same instruction position as the
  original `ldobj`/`ldind`.
- The call keeps its exact token and argument handling; `Void`, R4 and ordinary
  results use their previous paths. Methods that *return* a managed reference
  (declaring a ref-returning method) keep their refusal: the escape analysis for a
  returned alias is a separate capability.

## Evidence

- Red: owned `RelationCases.Ops.ReadFirst(int[])` was `Partial` on
  `byref call result needs escape/lifetime analysis` (recorded log); it now
  decompiles with `ref int v1 = ref RelationCases.RefHelpers.First(v0);` and reads
  the referent. The ref-returning helper itself stays `Partial` and is not selected
  by any driver.
- Targeted scans: `System.Private.CoreLib` 16,845 -> **17,023** (+178),
  `OpenRA.Game` +1; `FSharp.Core`, `OpenRA.Mods.Common` and the decompiler test
  image unchanged.
- 488 Rust tests, `cargo fmt --check` and clippy `-D warnings` pass; the relation
  driver's carrier cannot model a ref-returning leaf helper, so `ReadFirst` is
  excluded there and covered by the Rust test instead.
