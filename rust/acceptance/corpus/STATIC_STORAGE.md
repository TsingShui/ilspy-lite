# Typed generic and foreign static storage

## Representation and proofs

metadata/static_fields.rs owns static load/store/address operations behind
Image.static_field(token, caller_context). Existing named nongeneric FieldDefs
keep their representation. Referenced storage retains exact original
__cil_ldsfld_TOKEN, __cil_stsfld_TOKEN and __cil_ldsflda_TOKEN operations.

Local constructed owners require matching class/value kind, checked generic
arity and exact requested/declaration stored signatures after simultaneous
caller-scoped substitution. Static storage on a value container needs neither
an instance receiver nor host layout. The shared class-only callable member
binder remains class-only.

Foreign storage needs the exact selected field definition, scoped signature and
flags from dependency binding. Static flags are proved independently of instance
class/layout checks. Literal, marshaled, interface and unproved foreign cases
stay gated. This is not a broad foreign member, layout or allocation admission.

Reads produce typed snapshots; addresses retain acquired storage and checks.
Stores retain value copies/conversions and original acquisition/initialization
timing. Runtime leaves preserve one storage per closed declaring instantiation
and thread-static identity, rather than replacing a field with a global cache.
Readonly direct stores require the same initializer AND identity-mapped closed
owner. A static address grants no direct-store permission. Unknown foreign
readonly writes remain refused.

## Audit, tests and owned execution

Nine pre-change production default/ILSpy/original-IL triples are retained under
report/static-fields/audit/, with decisions in AUDIT.md. Public seams are
decompile/scan and original-versus-recovered actual default-output execution;
no private implementation tests or selected original-body proxies are used.

Six public static-field tests cover typed generic loads/stores, local/foreign
value containers, exact missing/wrong/added references, signature-index forgery,
nonstatic/literal definitions and wrong closed readonly initializer storage.
Retained reds: generic static read; generic store's former integer-only check;
literal admission; foreign field; local value static owner. Production remains
typed IL/CFG + lift and does not execute inputs.

Owned static-field execution per compiler configuration:
- **864 state/result comparisons / 96 steady-state closed selections**.
- **20 barrier-synchronized thread-static comparisons**.
- **20 fresh-process first-touch sequences**, each containing two invocations,
  covering read, store, acquired address, readonly read, failing initializer,
  argument failure, poison/one-time behavior and trace order.
- Twenty public steady-state methods, four first-touch methods (two closures
  each), and recovered argument support. Int/string/object/nullable-int and
  reference-bearing Pair values; mixed type/method variables are independently
  closed in a full matrix. Other closed storages are observed too.
- Eight rejected semantic mutants: early initialization, late address,
  wrong closed storage, copied-value snapshot drift, collapsed thread storage,
  wrong address, dropped store and wrong loaded storage. Mutants must compile
  and fail the intended behavior, not an unrelated runtime/binding error.

Field leaves emit only the original operation. The private argument support
binds recovered code. Fresh processes prevent reflection state snapshots from
initializing the tested type before its first access. The observer does not read
uninitialized/poisoned storage to fabricate state.

Three historical promoted negatives execute before frozen table removal:
- Scoped static cache: **1,440 comparisons / 260 closures / 26 methods per
  configuration**; copied cache identity mutant plus seven existing mutants.
- External static int: **26 comparisons / 9 methods per configuration**, including
  five distinct static values; wrong-static and null-allocation mutants.
- Generic initializer: **13 state comparisons / 9 closed recovered types per
  configuration**, with four mutants. Generic readonly Stamp is assigned inside
  the recovered mirror's real CLR initializer. Exact field operations are
  relocated individually to its corresponding owned storage, not a body proxy.

Fixture-authoring errors, the first Debug address mutant's compilation failure
and frozen-table failures were not counted as semantic validation. Corrected
mutants compile and fail behavior. Full commands/statuses are in
report/static-fields/behavior-manifest.json.

## Milestone

- fmt/clippy and **430 Rust tests** pass; **30 owned driver invocations** pass.
- Complete fixed-inventory scan: **531,492 Decompiled / 239,256 Partial of
  770,748 bodies**, fallback **31.0420526553426%**.
- **+9,808 methods in 112 images**, decrease **1.27253006170629 points**.
  Game **2,757 / 2,010 (+3)**; Common **11,870 / 6,537 (+234)**.
- delta.regressions is empty; zero incomplete images. Fixed 614 implementation
  images / 319 body-bearing images, hashes, exclusions and reference policy.
- Prior first blockers of gains: generic static owner 6,093; external field 3,715.
  Attribution is not isolated causal proof or a forecast.
- Twelve production triples self-reviewed CRUDE-BUT-EQUIVALENT; no
  changed-semantic SUSPECT in the bounded sample. See review/review-source.md.

The audit's Path.DirectorySeparatorChar and DBNull.Value methods remain Partial
for unavailable exact foreign field facts. GVM, unsupported prefixes, escapes,
unproved value layouts and other unrelated gates remain. No third-party runtime
execution, independent/formal review or whole-corpus equivalence is claimed.
Aggregate **<=5% is still unmet**. Baseline, per-image tables and all residual
methods/reasons remain in report/static-fields/corpus-scan/.
