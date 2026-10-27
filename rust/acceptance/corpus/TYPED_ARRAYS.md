# Typed SZArray operations

## Representation and boundaries

arrays.rs separates ordinary vector shape, stored element type, exact operand
matching and scalar storage width. ldlen needs a checked vector, not an element
layout/evaluation capability. Thus double[] can have its length read and be
allocated without admitting R8 scalar evaluation.

newarr retains the exact original token and caller context in
__cil_newarr_TOKEN. Checked core primitives, local nominal kind/arity and
caller-scoped TypeSpecs form canonical vector identities. Unproved foreign
element kinds and uninstantiated generic definitions stay gated. Existing
int[]/uint[] syntax remains unchanged. Allocation occurs at the original
instruction: fresh identity even for zero length, zero initialization and
runtime length/allocation errors; no cache or initializer shortcut.

ldelem.ref/stelem.ref require reference storage. Loads produce snapshots; stores
preserve runtime covariance/type/null/bounds checks at the store, after argument
evaluation. Generic/whole-value ldelem and stelem retain exact operand tokens
after invariant stored-element proof. ldelema retains the acquired cell and
performs null/bounds/invariant runtime element checks before later effects;
it is not a delayed indexed lookup or copied receiver.

Scalar I1/U1, I2/U2/Char, I8/U8 and R4 operations check vector storage width/kind.
Signed interpretation, truncation, raw Boolean bytes and R4 storage conversions
remain explicit. Existing I4/U4 array operations are unchanged. Opaque generic
value operations can execute a double closure without introducing R8 arithmetic.

I4 indices/lengths remain the admitted subset. Native dimensions/indices, native
scalar element operations, new array covariance assignment proofs, wider
primitive aliases, multidimensional shapes, unsupported prefixes, modified or
pinned shapes and byref escape/lifetime extensions are not claimed. Runtime
covariant inputs already legal for a declared object[] retain their actual type.

## Public and owned gates

Nine pre-change production default/ILSpy/original-IL triples are retained in
report/typed-arrays/audit/, with AUDIT.md. Original IL is authority. Public tests
use decompile/scan and actual recovered default-output execution.

Thirteen new Rust tests cover shape versus element capability, reference
loads/stores, tokenized copies, acquired addresses, scalar widths, independent
owner/method variables, counterfeit core scope, reference opcodes on scalar
storage, wrong element tokens and wrong storage widths. Length, reference
load/store, allocation, tokenized/address and two-byte semantic reds precede
their implementations. Broader already-supported scope/order coverage starts
green rather than being labeled a new-feature red.

Per configuration, actual recovered default output passes **3,040 comparisons /
95 closed selections**, plus the recovered private argument helper. Closures
include int/string/object/double/raw Boolean/nullable-int/reference-bearing Pair;
declaring T and method U remain distinct in T[], U[] and Mixed<T,U>[] allocation.
Cases include null/empty/nonempty, negative/OOB indices, covariant runtime arrays,
value snapshots, acquired aliases, exception precedence, argument effects,
zero initialization and repeated-call freshness. No huge/OOM allocations run.

Thirteen compiled semantic mutants are rejected: reference identity, dropped
store, copied address, changed value snapshot, Boolean normalization, float
signed-zero drift, late acquisition, missing argument effect, wrong allocation
length, nonzero initialization, cached allocation, wrong length and erased null
check. A Debug late-acquisition mutant initially failed to compile because a ref
binding was left before its declaration; it was corrected, then compiled and
failed the intended trace observation. Authoring errors are not semantic reds.

Exact test leaves emit one original array operation, never a selected original
body. The private argument helper also executes recovered code. Production Rust
never loads or executes input assemblies.

Frozen promotions were executed before row removal:
- Vector-reference profile: **128 comparisons / 26 methods/config**, preserving
  prior helper/state/bit/reference mutants and adding all five promoted operations.
- Enum allocation: **32 comparisons / 8 exact enum arrays/config**, with a
  compiled allocation-length mutant; existing enum transfers/mutants also pass.
- Scoped allocations: **1,480 comparisons / 270 closures / 27 methods/config**,
  retaining eight existing mutation controls.
- Struct receiver: **41 comparisons / 20 methods/config**, including null/empty/
  nonempty acquired array receiver; three dropped-store mutants remain.
- Virtual profile: **64 comparisons / 13 methods/config**, plus prior 9/2 value
  receiver cases and three mutation controls.

The first full regression retained six frozen-promotion failures (434/440);
only after execution were their rows updated. No loss was concealed by a count
change. Final fmt/clippy and **443 Rust tests**, plus **34 owned driver
invocations**, pass. Commands/statuses: report/typed-arrays/behavior-manifest.json.

## Fixed-corpus milestone

**554,169 Decompiled / 216,579 Partial of 770,748 bodies**, fallback
**28.099845864017812%**. This is **+22,677 methods in 221 images**, a decrease of
**2.942206791324788 points** from static storage. delta.regressions is empty;
all 614 implementation images (319 body-bearing) are complete. Denominator,
input hashes, exclusions, identities and reference policy remain fixed.

OpenRA.Game: **2,885 / 1,882 (+128)**. Common: **12,103 / 6,304 (+233)**.
First-blocker attribution is not isolated causal proof or a recovery forecast.

Fourteen gained production triples were self-reviewed CRUDE-BUT-EQUIVALENT;
no changed-semantic SUSPECT in the bounded sample. See review/review-source.md.
Two audited methods remain Partial for other reasons and are not claimed gains.

No third-party runtime execution, independent/formal verification, whole-corpus
or cross-runtime equivalence is claimed. **Aggregate <=5% remains unmet.** The
complete per-image results and residual methods/reasons are retained under
report/typed-arrays/corpus-scan/.

## Foreign value elements

`newarr` over a foreign value type is admitted when that element's own value-type
fact is present: inline type operands now join the demand-driven fact priming, so
the array names the element identity without claiming its layout. A recorded
refusal keeps its own reason. Owned `TokenCases.Dates`/`Days` were red
(`array allocation needs exact external element kind facts`) and are green after
the change; `typed-array` behavior and mutants are unchanged. See
`report/external-array-elements/AUDIT.md`.
