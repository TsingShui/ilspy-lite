# Static storage red/green record

All tests use public decompile/scan seams and owned inputs. No third-party
method was executed.

| Slice | Observed red | Green/evidence |
|---|---|---|
| Local generic fields | local-red.log: ReadInt, generic static field needs the constructed owner name | local-green.log |
| Generic store | First local-green.log invocation advanced to Write<T> and reported integer operation on non-integer value | Writer now uses typed assignment; generic value closures execute |
| Stored definition | storage-boundary-red.log: literal-marked static Value was incorrectly Decompiled | storage-boundary-green.log rejects literal storage |
| Foreign selected facts | foreign-red.log: ReadForeign remains external field fallback even with exact reference | foreign-green.log |
| Local value static container | value-owner-red.log: constructed member parent is not a class instantiation | value-owner-green.log; class-only callable binder remains separate |
| Promoted historical cases | regression-first.log: three frozen negatives became Decompiled | legacy-public-green.log; all three execute before table removal |

The generic-store failure above survives in the session tool transcript; its
initial output file was replaced by the later successful local-green.log run.
The diagnostic excerpt was:

    assertion left == right failed: Write: public static T Write<T>(T value)
    // IL fallback: integer operation on non-integer value
    left: Partial
    right: Decompiled

This is a recorded excerpt, not a reconstructed complete test log. The other
named red files retain their original full executions. Final regression-final.log
reports 430 passing tests; behavior-manifest.json records 30 passing invocations.

A first Debug late-address mutant moved only one ref declaration and did not
compile because another ref binding still used it. That is not semantic rejection.
The corrected mutant moves the already evaluated argument/binding before the
acquisition and compiles in both configurations, then fails the first-touch
trace/exception comparison. Fixture authoring parse/type errors likewise do not
count as red semantic proof.
