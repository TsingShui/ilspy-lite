# Reference-call foundation: main-session self-review

Eleven production default-output / ILSpy / original-IL triples were checked.
Exact methods, original first blockers, selected references and successful
command statuses are retained in `manifest.json`. No third-party execution.
This is static self-review, not independent or formal verification.

| slug | semantic comparison against original IL | verdict |
|---|---|---|
| 0004-06001362 | ResourceManager getter, literal, culture snapshot, exact GetString callvirt in original order | CRUDE-BUT-EQUIVALENT |
| 0000-06000040 | same counts array snapshot; exact Array.Clone; reference-type unbox.any remains checked cast to IEnumerable<int> | CRUDE-BUT-EQUIVALENT |
| 0483-060002C6 | qualified local core String receiver returned as primitive string, no operation or identity change | EQUIVALENT |
| 0620-06000242 | same String self-return in Mono core identity | EQUIVALENT |
| 0628-06000E51 | same String self-return in another Mono core identity | EQUIVALENT |
| 0000-0600053A | queue removal, function invocation, option branch, exact List<!0>.Add, original receiver/argument slot updates and loop retained | CRUDE-BUT-EQUIVALENT |
| 0000-06000067 | exact constructed child isinst with caller-owned !0/!1 followed by nonnull test | CRUDE-BUT-EQUIVALENT |
| 0000-060001BC | reference unbox.any checks FSharpList<!0> before original CompareTo callvirt | CRUDE-BUT-EQUIVALENT |
| 0000-06000149 | checked self casts, original getter/direct String.GetHashCode targets and hash arithmetic/join order retained | CRUDE-BUT-EQUIVALENT |
| 1041-06000007 | four distinct EqualityComparer<!n>.Default and GetHashCode calls with original field snapshots and unchecked multiply/add order | CRUDE-BUT-EQUIVALENT |
| 1042-06000005 | two distinct comparer instantiations and fields, original unchecked hash sequence | CRUDE-BUT-EQUIVALENT |

Three EQUIVALENT, eight CRUDE-BUT-EQUIVALENT, zero SUSPECT for changed
call/conversion/identity semantics; no differences found in those semantics.
The Mailbox body retains a low-level mutable receiver slot (`this = ...`) and
scoped generic aliases; this is not certification that every sample compiles
as standalone C#. Original method/callee tokens, not source-name similarity,
determine the runtime operations.

The remaining pre-change samples did not all recover: Dispose still needs
interface assignability, the map partition has allocation requirements, and the
package iterator has independent delegate/value/address gates. They were not
dropped from scope or counted as recoveries.

Owned runtime evidence is separate: the reference-call fixture runs production
default bodies with original MemberRef leaf adapters. Each adapter emits one
call/callvirt, not an executor for a selected body. Leaf resolution is lazy at
the operation, not eager initialization of all target methods. Class/interface/
indexer dispatch, hidden/overridden members, generic owner identity, argument
effects, receiver state and throwing targets are compared. Reference-unbox
variants retain their original mutated owned IL and compare null/wrong-type/
identity/array clone behavior. Five semantic mutants are rejected per variant
and configuration. Prior property/cast/equality/external/virtual/generic and
MethodSpec gates also pass; source-property refusals retain exact leaves rather
than fabricated property syntax.
