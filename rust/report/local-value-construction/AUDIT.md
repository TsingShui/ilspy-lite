# Ordinary local value construction and constructor-call audit

Baseline local 602e2fbc2: 558,757 Decompiled / 211,991 Partial of 770,748,
fallback 27.504579966474125%. Fixed inputs, exact references and denominator.
Seven retained production default/ILSpy/original-IL triples are in audit/manifest.json.
Original IL is authority; no third-party body was executed.

| Method | Required semantics |
|---|---|
| 0003-060000A6 PlainTextOutput.get_Location | Conditional second argument, then exact ordinary local TextLocation newobj. Separate value storage, completed copy, no reference-class allocation. |
| 0003-06000179 DomRegion.get_Begin | Original struct receiver fields supply two arguments, then ordinary local value construction. No receiver retargeting. |
| 0003-06001FBD TextLocation..ctor | Already recovered: two writes to original-this fields. Construction must invoke exact target and preserve readonly/current receiver proof. |
| 1041-06000056 CPos..ctor | Already recovered local value constructor; bit arithmetic/field writes retain exact integer categories. |
| 1041-06000057 CPos..ctor(int,int) | call to same value type's three-argument constructor on existing this; no allocation or synthetic zeroing. Reference-class delegation rules must not reject it. |
| 0002-0600020C ValueTypeCall.InstanceFieldTests | Earlier field writes and Console call precede a readonly-member value newobj, then destination assignment. Constructor failure must not assign the destination. |
| 1041-06000097 CVec.Minus | Foreign constructor remains a distinct proof. ILSpy emits unknown-result diagnostics, so its result is inconclusive, not BOTH_NO or a claimed gain. |

The value constructor module currently admits only MemberRef/TypeSpec generic
value construction. Add ordinary local MethodDef and exact same-image nominal
MemberRef forms behind the same typed-value seam; keep class allocation and
foreign ordinary proof separate. Reject malformed/static/nonvoid constructors,
wrong nominal kind/arity and byref escape. Preserve all generic constructor cases.

Distinguish newobj (fresh zeroed value storage, constructor, completed copy) from
call .ctor (the actual acquired managed cell, including old unassigned contents
and writes before throw). A direct value constructor call can target an existing
local/ref/field cell; it is not reference-class base initialization and must not
require original-this solely because its name is .ctor. callvirt remains invalid.

Use approved public decompile/scan seams and actual owned recovered-output
execution. First capture ordinary newobj red, then direct value-call/delegation
red; cover constructor trace/argument failures, separate storage versus acquired
cell, ref aliases, embedded identity, repeated construction and throws before
completion. Only individual operation leaves may bind original metadata; selected
owned constructor/argument bodies must also execute recovered code. Execute the
legacy StructReceiverCases.UnsupportedConstruct before removing its frozen row.

Delivered scope is ordinary local nongeneric MethodDef newobj and direct
MethodDef constructor calls. Same-image nominal MemberRef and other foreign/
generic direct-constructor forms remain deferred rather than being guessed.
No signature erasure, unknown foreign facts, reference-policy relaxation,
original-body proxy or third-party runtime execution is part of this group.
