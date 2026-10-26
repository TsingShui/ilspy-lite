# External property accessor audit

Main-session audit before implementation. Six methods were selected from six
assembly families in the latest fixed corpus scan. Exact member tokens and the
supplied references are in `audit/manifest.json`; each has default-output,
ILSpy-output and original-IL files. All commands completed successfully; a
failed command would be inconclusive rather than BOTH_NO.

| input/token | ILSpy construct | original IL | conclusion |
|---|---|---|---|
| DiffLib 06000045 | Thread.CurrentThread.ManagedThreadId | callvirt get_ManagedThreadId | candidate; later iterator typing may remain unsupported |
| Decompiler.Tests 06000120 | MainModule and IndentSwitchBody property assignment | get_MainModule / set_IndentSwitchBody | candidate; other calls remain independent blockers |
| Decompiler 06000057 | name.Name, read separately on each path | repeated callvirt get_Name | candidate; reads must not be merged or hoisted |
| CodeAnalysis.CSharp 06000029 | Compilation.Options.CheckOverflow | callvirt get_CheckOverflow | candidate; preserve declared owner and dispatch |
| CodeAnalysis 06000D7B | reader.IsAssembly | callvirt get_IsAssembly | candidate; later struct accessor calls are outside this slice |
| nunit.framework 06000185 | delegate.Method | callvirt get_Method | candidate; no added early receiver null check |

Six ILSPY_YES_OURS_NO, zero BOTH_NO. This is not a claim that every special-name
method is a property or that all 13,552 first-blocked methods will recover.

## Bounded design

Reuse the existing MethodSemantics/Property resolver. Admit only a uniquely
associated public instance getter/setter on an exact, nongeneric reference-class
owner, with a non-indexed property and ordinary by-value signatures. Check the
property name against colliding fields/methods/events/nested types and duplicate
accessor roles. Require canonical metadata association, not a get_/set_ prefix
heuristic. Interface owners, events, indexers, ref returns, generic owners and
unresolved facts retain fallbacks.

For callvirt, render property access on a receiver cast to its proven declared
owner, so hidden derived members cannot change binding; runtime virtual dispatch
still applies. Receiver/arguments are existing evaluated stack snapshots. Direct
call accessors keep exact-call intrinsics: changing a nonvirtual call to ordinary
property syntax could add a null check or dispatch to an override.

Tests will compare base/derived/hidden receivers, null receivers, setter/getter
side effects, throwing argument evaluation and observable state. A malformed
association and ordinary get_-named method must stay refused. No third-party
assemblies will be executed.
