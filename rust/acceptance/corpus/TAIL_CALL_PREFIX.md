# The `tail.` call prefix

`tail.` asks the CLI to replace the current frame with the target call. C# has no
equivalent request. ILSpy decodes the prefix, marks the following call as a tail
call, and emits an ordinary call in C# (the flag is only used by its `jmp`
translation). This repository follows that reference behavior: the prefix is
validated, skipped, and the call is emitted normally with a note in the detailed
view.

## Admission

- `src/tail.rs` requires `tail.` to immediately precede `call`, `callvirt`, `calli`
  or `jmp`, and records the call offset; the lift skips the prefix and handles the
  call through the ordinary arm, which keeps the exact call target, argument
  order, receiver rules, null checks, evaluation order, side effects and
  exceptions.
- `calli`/`jmp` targets keep their existing refusals; the prefix does not admit an
  indirect call.
- **Known divergence, stated:** the CLI's constant-stack guarantee is not
  reproduced in C#. A deeply recursive tail call can overflow where the CLI would
  not. This is the same trade-off ILSpy makes; it is not claimed to be equivalent.

## Evidence

- Red: F# `FSharpChoice<T1,T2>.CompareTo` (`0600006B`) was `Partial` on
  `unsupported opcode tail. at IL_00A2` (recorded log); it now decompiles, and
  ILSpy emits the same comparison body.
- Targeted scans: `FSharp.Core` 7,493 -> **8,608** decompiled (+1,115), partial
  2,129 -> 1,014, and no `tail.` refusal remains in the scanned images
  (`System.Private.CoreLib`, `OpenRA.Game`, `OpenRA.Mods.Common`,
  `ICSharpCode.Decompiler` are unchanged: tail calls are concentrated in F#).
- 487 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. No new runtime operation is introduced, so call semantics stay covered by
  the owned call/return drivers; the stack-depth divergence is not executed.
