# External value allocation batch self-review

Main-session self-review, not an independent review. Three gained F# methods whose
previous blocker was `external allocation needs dependency facts: owner is a value
type` were compared with production output, ILSpy and IL. The review is scoped to
the constructor sites this batch admitted.

| Image / token | Checked obligations | Verdict |
|---|---|---|
| 0000 / 0600110E printf helper | `Decimal::.ctor` is called through `__cil_newobj_0A000605` with five arguments in their original order; the produced value is the local's initial value, not a shared object. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06001114 sibling helper | Same five-argument constructor shape and value identity; no operand reordering relative to the IL. | CRUDE-BUT-EQUIVALENT |
| 0000 / 06000E60 DecimalPattern | The `Decimal` value is constructed and then passed to the option constructor; the allocation stays a value expression and keeps one evaluation per IL site. | CRUDE-BUT-EQUIVALENT |

No row shows an added conversion, a reused instance, or a reordered argument. The
bounded sample found no changed-semantic suspect. Full batch: 679,395 / 91,353,
+5,417 in 120 images, zero regressions.
