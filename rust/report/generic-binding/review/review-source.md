# Generic binding group: main-session self-review

Eleven original-IL / ILSpy / default-output triples, with exact reference sets,
are retained in `manifest.json`. All commands succeeded. No third-party
execution. This is heuristic self-review, not independent or formal proof.

| slug | semantic check | verdict |
|---|---|---|
| 0000-0600017A | original this and caller !0 copied to exact setter token; no virtual dispatch added | CRUDE-BUT-EQUIVALENT |
| 0000-06000078 | proxy field identifies a different constructed owner; !0 item copied out | CRUDE-BUT-EQUIVALENT |
| 0000-0600007A | second proxy preserves distinct !1 identity and item type | CRUDE-BUT-EQUIVALENT |
| 0000-0600018B | caller !0 forwarded once to original static Some token and result returned | CRUDE-BUT-EQUIVALENT |
| 0004-06001B2D | static constant tests, both State/UnreachableState evaluation orders, repeated Type getter and final Split stay ordered | CRUDE-BUT-EQUIVALENT |
| 0004-06006047 | one constructed result allocation with caller argument, then same reference returned | CRUDE-BUT-EQUIVALENT |
| 0000-06001D3C | seven stores precede exact existing-receiver base initialization; base uses caller's first variable | CRUDE-BUT-EQUIVALENT |
| 0000-06002000 | exact base initialization precedes two generic function-reference stores, distinct !2 bindings retained | CRUDE-BUT-EQUIVALENT |
| 0005-06002851 | 21-variable base prefix initialized before storing caller !21 | CRUDE-BUT-EQUIVALENT |
| 1041-06000B07 | exact constructed static allocator Reset token; no allocation/receiver/dispatch invented | CRUDE-BUT-EQUIVALENT |
| 1042-0600142B | condition token store, direct base init with caller !0, then byte ValidLayerType store | CRUDE-BUT-EQUIVALENT |

Eleven CRUDE-BUT-EQUIVALENT, zero SUSPECT for changed binding/operation semantics;
no differences found in those semantics against original IL. Static and
constructor intrinsics are precise target-runtime leaf operations, not recovered
C# method calls guessed from names. Scoped aliases and metadata names are not
certified standalone-compilable C# across this corpus sample.

The first full scan exposed 38 constructor regressions due to requiring equal
caller/base arity. They were not hidden or counted as zero. Three representative
regression triples above were inspected; an owned D<T,U>:B<T> case was captured
red before the bounded identity-prefix proof was fixed. Final delta has zero
regressions against the prior milestone.

Owned behavior compiles the default bodies with scoped alias bindings and
exact-call leaf adapters, including recovered support bodies in reconstructed
classes. It does not call the original selected/support bodies to simulate the
recovered result. Per release/debug: 240 comparisons, 60 closed method selections
(int/string/object/struct and swapped distinct arguments), four rejected
mutations including dropped generic static initialization. Prefix constructors
have public decompile/scan regression evidence, not a claim of new direct
execution of every constructor in this table.
