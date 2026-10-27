# Three-group gained-method self-review

The main session selected 11 gained methods from the complete delta, covering
all four previous-reason categories, the FSharp join and MethodSpec cases,
several constructor shapes, both OpenRA inputs and all three previous work-limit
failures. Each retained triple is the production default output, original IL and
ILSpy member source. All selected individual commands exit 0; no explicit ILSpy
unknown-result diagnostic appeared. The IL is authoritative. This is a bounded
self-review, not an independent review or a whole-corpus equivalence proof.
Third-party bodies were not executed.

| Sample | Verdict | Checked obligations |
|---|---|---|
| 0000-060010AD | CRUDE-BUT-EQUIVALENT | Sibling closure allocations remain on their original branches; their exact common constructed function type and the selected reference are retained through the stack join. |
| 0000-06001ABC | CRUDE-BUT-EQUIVALENT for verifier-valid references | The owned closure allocation precedes exact `SeqModule.Map` MethodSpec `0x2B000313`; method-scoped key/value positions and reference arguments are retained. Its earlier individual Partial/scan mismatch and repair are documented separately. |
| 0002-06000305 | CRUDE-BUT-EQUIVALENT | Original receiver reaches the exact constructed visitor base constructor, without a new allocation or virtual dispatch. |
| 0003-06000507 | CRUDE-BUT-EQUIVALENT | The stack allocation and field write occur before the exact list-base constructor, as in the IL. |
| 0003-060013EC | CRUDE-BUT-EQUIVALENT | The conditional resolver call and `extensionMethods` store remain inside the try; finally clears `resolver` on both paths, then the null check selects the original field or exact `Enumerable.Empty` MethodSpec. Constructed list-to-enumerable variance keeps the complete arguments. |
| 0055-06002E14 | CRUDE-BUT-EQUIVALENT | The null and TypeCode exits precede the `ldftn`/delegate allocation, interface call, `ToList` MethodSpec and direct mutation call. The exact delegate operation stays represented; no third-party delegate body was executed. |
| 0055-06002EC6 | CRUDE-BUT-EQUIVALENT | The same try/finally and null-coalescing operation order is retained with this assembly's distinct tokens and scoped method/interface identities. |
| 1041-0600075C | CRUDE-BUT-EQUIVALENT | The exact dictionary-base constructor is called on the existing receiver; there is no synthetic allocation. |
| 1042-0600008B | CRUDE-BUT-EQUIVALENT | The `WAngle` value is passed once to the exact `ValueActorInit<WAngle>` base call on `this`. |
| 0134-06000270 | CRUDE-BUT-EQUIVALENT | The open `Dictionary<!0,!1>` base is kept in the owning generic scope and the original constructor token is retained. |
| 0420-06000432 | CRUDE-BUT-EQUIVALENT | Six field stores occur before the exact constructed base constructor, followed by the exact `Initialize` call; Boolean storage remains explicit. |

Where the production CLI selected substituted framework metadata, the report
states that selection. It is not a proof that the selected host library and the
original target runtime behave identically. A sample verdict does not extend to
unexamined methods or imply ordinary C# recompilation of the complete output.
