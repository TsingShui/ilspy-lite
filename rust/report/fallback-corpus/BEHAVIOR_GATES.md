# Behavioral gates

All drivers run owned fixtures only; **no third-party method body is executed**.
Each compares original and recovered bodies for return values, state, identity and
exceptions, and where marked also rejects deliberate mutants.

| Driver | Comparisons | Selections / methods | Mutation control |
|---|---:|---:|---|
| reference relations | 851 | 81 | deep-chain, hidden-argument, sibling-branch, MethodSpec and prior mutants rejected |
| typed vectors | 3,040 | 95 | reference-identity and dropped-store mutants rejected |
| constrained calls | 720 | 40 | location joins, receiver slots, value copies |
| value calls | 585 | 65 | receiver writes, value copies, generics |
| exception regions | 51 | 10 | six mutants rejected |
| external allocation | 41 | 13 | static-storage and null-allocation mutants rejected |
| struct receivers | 43 | 21 | managed-location writes and copies |
| external enums | 30 | 9 | storage widths, loads/stores |
| general arrays (multidim) | 23 | 5 | dropped store and swapped dimensions rejected |
| unmanaged pointers | 9 | 6 | dropped write and double scaling rejected |
| static constructors | pass | - | ordering/state |
| float semantics | 528 (in-suite) | 52 | NaN/-0/±inf inputs |

Scope statements:

- These gates cover the operations each owned fixture exercises. They are not a
  proof of whole-corpus equivalence, and they never substitute for the static
  corpus scan.
- The latest complete static scan (`report/foreign-hidden-batch/corpus-scan-fixed`,
  728,704 / 42,044) is the measurement; behavioral evidence is reported separately
  from static coverage.
- Third-party inputs (ILSpy-tests images, OpenRA assemblies) are analyzed
  statically only. Executing their bodies would need separate authorization.
- Incomplete coverage is stated rather than implied: value-type receiver dispatch
  through MethodSpec (`CallValueGeneric`, `CallLocalValueGeneric`,
  `CallConstructedValueGeneric`) and ref-struct copies are excluded from the
  reflection drivers for the documented reasons and covered statically.

Revision note: the external-allocation driver now accepts a constructed external
owner (`MakeBox`) whose inlined declaration is a valid C# instantiation, so the
matrix records 41 comparisons over 13 methods. All ten drivers were re-run at
`5cce10dca`; subsequent constructed-join and hidden-identity work reran the
reference-relations driver with the expanded 851/81 selection. The hidden-identity
selection also passed with a version-substituted owned library. No third-party
body was executed.

The external constructed-base group additionally executes 48 owned comparisons
across Release/Debug and call/callvirt forms, with four semantic mutants rejected
in each form. Its exact-constructor leaf and bounded coverage are documented in
`report/external-base-ctor/AUDIT.md`; the reference-relations, external-allocation,
exception and value-call drivers were also rerun for this group.
