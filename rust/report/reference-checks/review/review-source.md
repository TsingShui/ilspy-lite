# Reference repair group: main-session self-review

Default production readable output was compared with ILSpy and original IL.
The fourteen selected methods and exact references are in `manifest.json`;
all commands succeeded. No third-party input was executed. This is static
self-review, not independent review or a standalone-C# guarantee.

| slug | changed operation checked against IL | verdict |
|---|---|---|
| 0000-06000337 | TaggedText interface field reference returned without copying its object | EQUIVALENT |
| 0003-0600030B | field load, IType interface callvirt, existing byte-enum normalization | CRUDE-BUT-EQUIVALENT |
| 0004-06005280 | original this returned through its declared implemented interface | EQUIVALENT |
| 0005-06000300 | IGenericTypeParameter receiver passed to inherited INamedEntity callvirt | CRUDE-BUT-EQUIVALENT |
| 0006-06000329 | ReturnType getter precedes Scope getter, same interface reference returned | CRUDE-BUT-EQUIVALENT |
| 0000-0600031A | abstract Joint isinst, child tags/branches, checked child casts and field reads | CRUDE-BUT-EQUIVALENT |
| 0003-06000076 | abstract EntityDeclaration test preserves conditional Parent getter and later tests | CRUDE-BUT-EQUIVALENT |
| 0004-06000037 | NamedTypeSymbol cast stays on Kind==11 branch after getter/null handling | CRUDE-BUT-EQUIVALENT |
| 0005-06000B8A | ConstantValue isinst precedes the original virtual Equals target | CRUDE-BUT-EQUIVALENT |
| 0003-06000F10 | external Operand getter, throwing string cast, then one LdStr allocation | CRUDE-BUT-EQUIVALENT |
| 0005-060000ED | string test, conditional blob path, stateful calls and struct result copy retained | CRUDE-BUT-EQUIVALENT |
| 1041-0600001D | exact backing-field interface reference returned | CRUDE-BUT-EQUIVALENT |
| 1042-06000098 | null-yielding string test before allocation, original object identity returned | CRUDE-BUT-EQUIVALENT |
| 0003-06000145 | base field read, two callvirt operations, then exact direct RegisterCecilObject | CRUDE-BUT-EQUIVALENT |

Two EQUIVALENT, twelve CRUDE-BUT-EQUIVALENT; zero SUSPECT for the changed
admission/conversion/assignment semantics. No differences found in those
operations against original IL. Exact call and scoped type aliases remain
low-level notation. The larger Joint method retains explicit labels and stack
joins; the child-test order and field-read paths were checked, not inferred from
the shorter ILSpy expression.

Owned execution is separate: interface calls/copies/casts/fields/assignment and
receiver state, including explicit implementations and boxed receivers, use
125 comparisons over 13 selected call-site methods per configuration. Four
semantic mutants fail. Cast checks use 114 comparisons over 15 methods per
configuration and five mutants, including bool/int result corruption. The
original object gate remains green (111 cases, 19 selected methods/configuration).
These gates do not execute the third-party methods in this table.
