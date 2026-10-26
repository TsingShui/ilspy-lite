# Exact exception identities, constructors and local typed catches

## Scope and representation

Production Rust remains typed IL/CFG + lift, without executing inputs or using a
C# AST. Checked core assembly identities admit a bounded named exception family
in System and System.IO. Namespace, assembly name/key/culture/flags are checked;
names or Exception suffixes in arbitrary scopes are not ancestry evidence.

The bounded parent table proves core subtype assignment (for example,
InvalidOperationException to Exception). The owned runtime gate checks all 21
listed immediate parent facts; ObjectDisposedException derives from
InvalidOperationException and InvalidDataException from SystemException.

For local nongeneric nominal classes, a checked finite local base chain must end
at one admitted exact core exception. Throw accepts that reference or null;
the exact original thrown object is not reconstructed. Catch TypeDef entries
retain Ty::Class and the precise local type, rather than interpreting local RIDs
as external identities. Normal CLR typed matching, object identity, rethrow and
finally are retained by the existing lexical EH lowering.

Core exception newobj checks an ordinary instance, non-vararg, unmodified
void constructor signature and substitutes each parameter in the caller scope.
The shared __cil_newobj_TOKEN operation retains the original token/signature,
argument values, freshness and constructor effects. It does not pick overloads
from argument counts or imply arbitrary foreign allocation facts.

Unknown foreign ancestry, generic catch/throw identities, catch-System.Object
wrapping, filters/faults and unsupported EH transfers stay independently gated.
No generic virtual dispatch lookup-order refusal is relaxed.

## Audit and red/green evidence

report/exception-types/AUDIT.md records six pre-change production triples with
default readable output, original IL and ILSpy. FSharp catch-object is an
explicit independent boundary, not a failure-based corpus exclusion.

Five public tests in tests/exception_types.rs cover core throws, exact
constructor overloads, local ancestry/catches, System.IO identities and
counterfeit/malformed boundaries. Retained reds include core throws,
constructor return subtyping, local throw and namespace construction.
Negative mutations cover core public-key identity, static/nonvoid constructor
shapes and a local exception ancestor replaced with System.Object. Historical
two-string constructor signature/name/scope/stack mutations remain tested.

## Owned execution

tests/exception-type-roundtrip.py compiles actual default readable output.
Twelve public methods plus two recovered private argument helpers execute in
both compiler configurations: **181 comparisons per configuration**.

Checks include incoming/caught/returned exception identity, null throw,
message/inner/actual-value references, parameter names, fresh allocation, local
Derived/Custom handler order, rethrow/finally and constructor side effects.
Argument helpers bind recovered bodies and preserve the original declared
static storage; failure at each argument prevents later arguments/construction.
Constructors are exact one-operation leaves, not selected method-body proxies.

Five semantic mutants fail per configuration: replacing thrown identity,
dropping inner identity, swapping range arguments, reversing argument
evaluation, and copying the original message string. Rejection requires the relevant behavioral mismatch, not failure
to compile or bind a carrier.

Historical UnsupportedType/UnsupportedOverload execute before their frozen
Partial rows are updated: **6 comparisons / 2 methods per configuration**.
Existing exception-object gate still passes **259 comparisons / configuration**;
lexical EH passes **39 comparisons / 8 methods / configuration** and four
semantic mutants. A first local-constructor oracle reset the wrong static
storage; its failure is retained and the shared declared storage is now reset
before each original/recovered run.

## Milestone validation

- cargo fmt --check; clippy --locked --all-targets -D warnings.
- **423 Rust tests**, all passing.
- **28 owned driver invocations**, all passing; full commands/statuses in
  report/exception-types/behavior-manifest.json.
- One complete fixed-inventory production scan, no recovered-method regressions
  (delta.regressions is empty) and zero incomplete images.
- **521,684 Decompiled / 249,064 Partial of 770,748 bodies**,
  aggregate fallback **32.31458271704889%**.
- **+25,305 methods in 213 images**, decrease **3.2831742670756265 points**.
  Game: **2,754 / 2,013 (+86)**; Common: **11,636 / 6,771 (+408)**.
- Unchanged 614 implementation images / 319 body-bearing images, fixed hashes,
  exclusions and reference policy; no corpus redownload, replacement or execution.

Prior first blockers of gained methods: allocation 13,680; old exception identity
10,357; local throw 934; catch identity 290; assignment 29; core resolution 15.
These are attribution by previous first reason, not isolated causal estimates.

Twelve gained production triples were self-reviewed: one EQUIVALENT and eleven
CRUDE-BUT-EQUIVALENT, no changed-semantic SUSPECT in the bounded sample.
report/exception-types/review/review-source.md records method-level decisions.

No third-party runtime execution, independent/formal review or whole-corpus
equivalence is claimed. Aggregate **<=5% remains unmet**. Baseline, per-image
tables and every residual method/reason remain in
report/exception-types/corpus-scan/; delta.json and milestone.json retain totals.
