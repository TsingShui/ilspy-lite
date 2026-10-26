# Symbol naming and body admission

## Frozen baseline

Implementation baseline: `519982c88`. The unchanged twenty-class manifest is
[random-classes.json](random-classes.json); the three additional observations are
[naming-samples.json](naming-samples.json), not replacement selections.

Reuse the original MVIDs and provenance from [BASELINE.md](BASELINE.md).
No input was reacquired or changed. The corrected prior static report
`rust/report/openra/random-classes/final-21c469ba6/report.json` applies to the same
production code: later changes up to this baseline were test integration/docs.

The twenty classes contain 77 methods: 2 Decompiled, 75 Partial, no Failed or
Declaration. Of the partials, 58 first report a non-simple identifier. This is the
first gate only, not proof that those 58 bodies have no other unsupported shape.
The other first reasons are 3 constructed/nonlocal fields, 3 EH/data sections,
3 signatures, 2 ancestry, 3 callvirt, 2 allocation and 1 core identity.

The separate three-class observation has 6 Partial bodies: 4 first report naming,
1 signature and 1 array element admission. Actual original default outputs and
the draw record remain under `rust/report/openra/class-demo-gu5q325h/`.
Wait is separate: 3 Decompiled bodies, exit 0.

## Owned reproduction

`NamingCases.Factory/<>c__DisplayClass0_0::.ctor` contains only the existing
Object-base protocol. The public readable and detailed method interfaces must
recover it without changing constructor semantics. Before implementation,
`cargo test --locked --test naming` fails with Partial and
`non-simple identifier needs IL fallback`; the full red is retained at
`rust/report/naming-generated-owner-red.log`.

The same owned source includes a captured-I4 update and a static constructor.
Only ordinary existing shapes may become supported. Static initialization,
unsupported types/instructions, generic/value-type/EH admission and new ordinary
allocation remain outside this naming change.

## Verification contract

Use public byte/context/CLI output as the test interface. Preserve scoped tokens
and raw names; aliases are presentation, never member identity or source-binding
proof. Declaration/reference names must agree without rewriting literals.
Newly recovered default class statements require original/recovered execution,
not merely a changed status. Retain independent ownership/inventory and
method-local fallback checks for every frozen selection.
