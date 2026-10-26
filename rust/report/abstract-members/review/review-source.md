# Abstract constructed-owner self-review

Main-session static comparison of default output, ILSpy and original IL. No
subagents and no third-party execution. Exact methods/references are in
`manifest.json`; all audit commands succeeded.

| slug | method | verdict | original-IL check |
|---|---|---|---|
| 0000-0600006F | FSharpChoice.GetHashCode | CRUDE-BUT-EQUIVALENT | comparer getter executes first, then the same callvirt token receives this and that comparer |
| 0003-060008AA | DataFlowVisitor.VisitReturn | CRUDE-BUT-EQUIVALENT | Value getter, virtual AcceptVisitor, direct MarkUnreachable remain ordered and use their original targets |
| 0004-06001A2E | AbstractFlowPass constructor | CRUDE-BUT-EQUIVALENT | direct base constructor on existing this with seven original arguments, followed by trackUnassignments store |
| 0005-0600180B | PEModuleBuilder.GetTopLevelTypes wrapper | CRUDE-BUT-EQUIVALENT | EmitContext value copied into exact direct target; returned enumerable reference unchanged |
| 0006-06000207 | SortedTable constructor | CRUDE-BUT-EQUIVALENT | explicit MetadataTable<!0> constructor call retained; ILSpy elides the implicit base call |
| 0011-060004C6 | EqualityComparerAdapter constructor | CRUDE-BUT-EQUIVALENT | base initialization precedes storing the comparer reference |
| 1041-060002C1 | OpenRA CellLayer constructor | CRUDE-BUT-EQUIVALENT | same original this/map passed to CellLayerBase<!0> constructor, no allocation invented |
| 1042-060013A6 | OpenRA ConditionalTrait.Created wrapper | CRUDE-BUT-EQUIVALENT | virtual Created target receives existing this and the original Actor |

Eight CRUDE-BUT-EQUIVALENT, zero SUSPECT for the admission change reviewed.
No dispatch/argument/copy/constructor-order differences found against IL.
The output retains exact-call intrinsics and scoped generic context; this is not
a standalone C# compilation guarantee for every corpus body.

Owned call-site behavior is separate: 22 comparisons per configuration,
virtual/nonvirtual calls through abstract Base<int>, null and two concrete
overrides, repeated-call receiver state, and ordinary allocation. Two wrong-
dispatch mutants are rejected. The abstract-allocation metadata mutation remains
Partial. Existing generic-member and allocation behavior gates also remain green.
This is self-review, not independent or formal verification.
