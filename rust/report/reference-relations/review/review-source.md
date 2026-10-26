# Main-session reference relationship self-review

Default production release output, ILSpy source and original IL; exact paths,
references, commands and statuses are in `manifest.json`. Ten triples, 30
successful commands. Seven new recoveries and three restored initial regressions.
Original IL controls the result; no third-party code was executed.

| slug | changed-operation check | judgment |
|---|---|---|
| 0000-06000066 | same nested-case newobj and generic payload, then a proved constructed-parent return; no cast/check/allocation added | CRUDE-BUT-EQUIVALENT |
| 0003-06000004 | same open-self isinst; fields read through the matching scoped self; comparer calls and second-comparison short-circuit preserved | CRUDE-BUT-EQUIVALENT |
| 0895-06000028 | inherited Syntax field access stays explicitly owner-qualified; exact calls, Boolean bytes and exact constructor precede the interface return | CRUDE-BUT-EQUIVALENT |
| 1041-0600002C | exact iterator object constructor, two captured field stores and return to its declared interface; no invented yield/state-machine algorithm | CRUDE-BUT-EQUIVALENT |
| 1042-060000CB | iterator allocation, whole CPos copy, Boolean byte copy and interface return remain ordered; later enumeration implementation is not replaced | CRUDE-BUT-EQUIVALENT |
| 0003-060013E2 | declaringType is read once and returned; generic external-base spelling does not change the receiver operation | EQUIVALENT |
| 0004-060001B0 | same binder loop and callvirt sequence, enum bits and original-this argument retained | CRUDE-BUT-EQUIVALENT |
| 0055-06001A6B | same base constructor with typed null TokenRole, then exact setter call with initialization guard intact | CRUDE-BUT-EQUIVALENT |
| 0624-060012B8 | six arguments, enum widths, typed null and the same CreateNew overload; no mapping operation executed during review | CRUDE-BUT-EQUIVALENT |
| 0895-06000619 | branches, typed nulls/formals, value defaults, exact calls, one boxed-state constructor and Interlocked pointer capture preserve original order | CRUDE-BUT-EQUIVALENT |

One equivalent, nine crude-but-equivalent, zero changed-semantic SUSPECT in this
bounded sample. This is self-review, not independent review, formal equivalence,
whole-corpus C# compilation or third-party behavioral validation.

## Retained failures and scope

The initial scan lost 18 recovered methods when a formal reference operand
needed a declared type name whose external constructed base was unproved.
Three failing triples are under `regressions/`; an owned null-formal red test
reproduces the same path. Qualified identity spelling does not require an
ancestry/layout proof. Local reference identity can stop at a declared foreign
constructed reference base; proving assignment to an ancestor still needs exact
instantiation facts. The final scan has zero losses.

The owned static-carrier attempt for Cell<T>.ReadCast also exposed null-this
shared generic entry behavior. The corrected carrier retains a generic declaring
class instead of changing it to a static generic method, and compares null and
nonnull receivers for value/reference instantiations. The failure remains in
`behavior-first.log`; no original selected method is used as its replacement.

Owned default bodies execute 143 comparisons / 36 closed selections per
configuration. Four rejected mutants cover original overload selection (typed
operand removed), swapped-argument return, foreign-reference return and self-cast
result. Additional public negatives reject value variance, erased generic
positions, missing reference constraints and cyclic foreign ancestry. Registry
tests check adding exact definitions and query-order stability.

CVec.Minus remains an inconclusive ILSpy pre-audit sample due to its explicit
unknown-result diagnostics. ActorIndex.Actors remains Partial in the audited
context. Neither becomes an exclusion or a claimed recovery. Constructed value
members, broader boxing/constrained dispatch and GVM remain separate work.
