# External property self-review

Main-session static review of eight newly recovered methods. Inputs are default
readable output, ILSpy output and original IL; all command exits were checked.
No third-party method was executed. Manifest records exact tokens/references.

| slug | method | verdict | decisive observation |
|---|---|---|---|
| 0003-06000057 | DotNetCorePathFinder.FallbackToDotNetSharedDirectory | CRUDE-BUT-EQUIVALENT | get_Name remains a separate call on each reached path; no getter merge/hoist across File.Exists or return |
| 0004-06000029 | Binder.get_CheckOverflowAtRuntime | CRUDE-BUT-EQUIVALENT | enum switch branches preserved; Compilation/Options precede CheckOverflow; cast selects declared CompilationOptions property while retaining callvirt |
| 0005-06000DAC | PEModule.get_IsManifestModule | CRUDE-BUT-EQUIVALENT | MetadataReader evaluated once, then IsAssembly getter at the IL callvirt location |
| 0012-06000026 | TestRunner.GetModule | CRUDE-BUT-EQUIVALENT | object created once; argument factories precede their setter calls; each ReadingMode branch stores enum value 1 or 2 before ReadModule/RoundTrip |
| 0054-06000026 | RefactoringExtensions.GetLocalVariableDeclarationSpace | CRUDE-BUT-EQUIVALENT | Parent getter stays inside the loop after each predicate; null test precedes predicate call exactly as IL |
| 0055-060019A8 | AstNode.SetRole | CRUDE-BUT-EQUIVALENT | uint Index getter converts to CLI int stack bits, combines mask -512, then stores uint flags; no width loss |
| 1042-060000E4 | OpenRA WorldExtensions.WithPathTo | CRUDE-BUT-EQUIVALENT | World field read precedes CenterPosition getter; returned WPos is copied before the exact overload call |
| 0413-0600002B | CollectionBase.set_Capacity | CRUDE-BUT-EQUIVALENT | InnerList getter evaluated once, then declared ArrayList.Capacity setter receives the original argument |

Eight CRUDE-BUT-EQUIVALENT, zero SUSPECT in the property transformations reviewed.
No property evaluation/dispatch differences found against original IL. These
samples still contain pre-existing C#-like type aliases and precise leaf-call
intrinsics; no standalone C# compilation claim is made for corpus methods.

Owned default-output behavior is separate: 98 comparisons per fixture
configuration; independent receiver states include base, virtual overrides,
hidden derived properties, null and throwing accessors. Three mutants detect
bypassed getter, bypassed setter and null checking before argument evaluation.
Original direct/base property tests remain green; a direct-dispatch mutant is
rejected by its exact-call check. This is self-review, not independent evidence.
