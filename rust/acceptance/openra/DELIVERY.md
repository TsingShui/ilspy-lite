# Class-level delivery acceptance

The three-milestone behavior/output-reading gates passed at `74407e18f`, but the
initial completion request at `05a056ac7` was rejected: ClassLayout,
CustomAttribute, DeclSecurity and MethodImpl evidence was silently omitted from
selected class results and the independent inventory checker. Passing method
bodies did not satisfy metadata completeness. These omissions are corrected by
`0408e6d2e`, `2ddcf8930`, `accdeec22` and `b4f25e943`, respectively. Independent
review then found invalid-owner omission, lost endpoint failure evidence and an
insensitive identity checker; `4a9ec2d43`, `209aede11` and `a50b1f77c` correct those
findings. The review's four-byte RID follow-up is corrected by `f01ba7876`, which
validates untrusted simple/coded RIDs before token construction can truncate them.
The four-table correction review found no remaining issues and returned OK, but
the next completion audit at `140d8aeaa` found omitted parameter FieldMarshal
records. `b538ca77f` adds uniform field/ordinary-parameter/return-parameter native
descriptor evidence, local limits and independent inventory checks. Final
corrected production/test revision: `b538ca77f`; `d1f1f7bea` fixes the independent
oracle's unrelated-blob filtering and adds reproducible owned oracle checks.
The following audit at `cd329d082` found unreadable field names discarding other
readable metadata; `a0d3d16f4` preserves field name-index/token alias and continues
flags/signature/constants/layout/RVA evidence. `8c878ac2f` fixes the analogous
method-name early return, preserving signature/flags/parameters/import/generic
context. Current validated production/test revision: `8c878ac2f`.
This acceptance record supplies evidence, not a substitute for the goal completion
auditor's decision.
[SEMANTICS.md](../../SEMANTICS.md) defines output notation. Historical baseline,
stage and first delivery evidence are retained rather than rewritten as broader
success claims.

## Public interface and member completeness

```sh
(cd rust && cargo build --release --locked)
rust/target/release/ilspy-lite decompile Assembly.dll --type Namespace.Type
```

Selection is by exact metadata TypeDef name, independent of body support, and
includes recursively owned nested/generated types. Empty and generic types,
private/generated members, overloaded methods, constructors, properties/events
and accessors remain visible. The original identity, flags, base/interfaces,
generic parameters/constraints, fields/constants and all method signatures/statuses
are retained. There is no production acceptance-class/token whitelist.

Unsupported methods keep complete readable IL and reasons independently of
successful peers. Unavailable/invalid evidence is explicitly failed, not invented.
Exit 0 means successful bodies/legal declarations; 2 means fallback without
failure; 1 means input/selection/body/metadata failure. Metadata failures are
separate from failed-method counts. Agent-readable temporaries/labels/intrinsics
are permitted; compilable source/project export is not the contract.

Independent System.Reflection.Metadata PEReader snapshots match all five frozen
public class outputs, including complete ownership/member identities, flags/raw
signatures, generic context, parameters/constants, accessors, ClassLayout,
CustomAttribute, DeclSecurity, MethodImpl, field/parameter FieldMarshal and
status/exit accounting. Corrected
results have no failed bodies or metadata failures:

| Class | Decompiled | Partial | Exit | Required behavioral closure |
|---|---:|---:|---:|---|
| PlayerResources | 7 | 3 | 2 | Three resource bodies passed |
| AmmoPool | 4 | 7 | 2 | Both required getters passed |
| MersenneTwister | 7 | 0 | 0 | All seven methods/dependencies passed |
| Activity with four owned generated types | 14 | 48 | 2 | Frozen Queue body passed |
| Wait | 1 | 2 | 2 | Frozen Tick body passed |

Counts describe these selections, not semantic coverage percentages. Unrelated
partial members are still limitations, not successful decompilations.

## Metadata completeness corrections

The corrected public class result includes each owned ClassLayout size/packing
and raw row. CustomAttribute records retain parent, exact constructor
identity/signature, original blob index, complete raw payload and row; they are
explicitly uninterpreted and never executed. Attached members/return and ordinary
parameters/generics/relations/security records and image-level attributes are
included. DeclSecurity retains class/method/assembly scope, action, permission-set
blob/index/raw row and a local runtime-owned enforcement limit. MethodImpl retains
both MethodDefOrRef endpoints, full scoped identities/signatures and raw row,
without pretending to resolve dispatch/access/ancestry or guess source overrides.

Owned Debug/Release public-interface regressions cover explicit layout and invalid
packing; attributes on all supported owner categories and a nested type;
class/method/assembly security; overloaded local and external/generic explicit
method mappings. Invalid constructor/blob/mapping endpoints retain their readable
raw evidence, metadata failure and peer fields/methods. Null/out-of-range owners
in all four tables are retained as explicitly unowned image context; readable
endpoint rows/name/owner/signature bytes survive failed signature/reference
interpretation. An authored wide-table PE fixture with 65,536 TypeDefs checks
four-byte simple/coded owners against an otherwise aliasing RID, for both the
apparent target and unrelated selections. Full invalid raw RIDs are retained,
not truncated into valid owner tokens. The oracle additionally checks exact
endpoint names/raw owners and rejects identity-only mutations. All were observed
red before implementation. The separate PEReader oracle and checker compare
these four record families; negative tests reject their omission, mutation or
unsupported interpretation claims. Ordinary body status is not a claim that
attribute arguments, permission-set policies or complete dispatch were recovered.

The focused read-only correction review initially returned BLOCK with two raw
preservation defects and an endpoint identity-check gap. Parent reproduced/fixed
these findings in the commits above; its follow-up found the wide-RID edge, which
was also reproduced/fixed. Final focused verdict: **OK; no issues found**. Reports
are session-managed `reviews/class-metadata-corrections.md`,
`reviews/class-metadata-corrections-followup.md` and
`reviews/class-metadata-wide-owner-followup.md` (final run
`20f1dfff-78c3-43ff-a7ee-55222b474782`). Reviewers inspected sources/logs, not
executed inputs; parent validation supplies the execution evidence.

The subsequent completion audit's FieldMarshal finding is fixed independently
of body semantics: owned managed return/ordinary bool descriptors and a PInvoke
LPArray parameter retain exact parent/blob/row evidence, explicitly uninterpreted
runtime-owned interop policy. Bad blob indexes or association flags retain records,
metadata failures and peer methods. Field and parameter records use one output
path, avoiding duplicate field declarations. The independent oracle/checker now
compares every owned FieldMarshal row, including return parameter sequence 0;
negative checker mutations were observed red before the fix. This is metadata
preservation, not a CLR marshaling interpreter. The focused marshalling review
returned OK with notes and confirmed the production omission resolved. Its oracle
scope note was reproduced red then fixed in `d1f1f7bea`: unrelated malformed
marshal blobs are filtered before heap decoding. Owned Debug/Release class output
checks and unchanged unrelated-corruption snapshots are saved in
`report/class-marshalling-owned-oracle.log`, reproduced by
`python3 rust/acceptance/openra/test_oracle_marshalling.py` after building the
oracle with `inventory.py`. Review artifact: `reviews/class-marshalling-correction.md`,
run `599198f0-6390-422e-a7e6-024434738f10`. Parent execution, not reviewer statements,
supplies the validation evidence.

The field/method name-error corrections have Debug/Release public regressions
that were red before implementation: invalid Fields::Limit name keeps flags,
signature/default bytes and peers; invalid Methods::Optional name keeps original
index, full method/parameter/default evidence and independent body status.
Property/event/parameter/generic name diagnostics already retain their independent
records. Focused review returned **OK; no issues found**, also confirming the prior
marshal-oracle note resolved. Artifact: `reviews/class-member-name-corrections.md`,
run `d007fd4e-0df2-468e-969c-334ef511697b`.

## Semantic and independent-class evidence

Actual bodies are extracted from public --type output and executed, not
regenerated via --method or proxied to the original selected method. State shells
and single-operation/exact-call adapters are owned test tooling only.

- Seven-body random-class assembly: **22,941** return/exception/state comparisons.
  Seed 0/1/-1/int.MinValue/int.MaxValue initialization compares all 624 words;
  at least 1,250 consecutive Next and NextFloat calls per ordinary seed compare
  returns/binary32 bits and full scalar/array state. Default construction uses
  an independent original-IL oracle with only the clock boundary controlled.
  Range checks include unchecked endpoint overflow and detailed exception
  type/parameter/message/HResult/ActualValue. See
  [SEMANTIC_MILESTONE.md](SEMANTIC_MILESTONE.md) and its linked slice contracts.
- Existing resource/ammo state/return/exception gate: **837**, still passing.
- Frozen Activity.Queue: **20** finite-chain link/identity/null comparisons;
  existing and prelinked appended chains are preserved. Cycles are not executed
  as termination proof. See [ACTIVITY_QUEUE.md](ACTIVITY_QUEUE.md).
- Frozen Wait.Tick: **193** cancellation/countdown/old-value/repeated-call/null
  comparisons. The exact unchanged external base getter remains a permitted
  independent leaf, never a replacement for selected Tick logic. See
  [WAIT_TICK.md](WAIT_TICK.md).

Neither independent sample was replaced after lowering evaluation or failure.
They are purpose-selected, nontrivial real classes, but belong to the same OpenRA
release and share an activity abstraction. This proves the frozen acceptance
contract, not broad ecosystem compatibility or whole-game support.

## Output-only Agent reading

Before metadata corrections, a fresh-context read-only Agent read only the
then-current SEMANTICS.md and supplied public
class outputs for resources, ammo, random, Activity, Wait, addresses and integer
conversions. It did not read implementation, original source/DLLs, tests, behavior
logs or prior answers, and did not execute methods. Verdict: **PASS; no issues**.

Its explanations correctly traced raw nextActivity traversal rather than property
filtering; null/cyclic behavior; Wait's exact external call and pre-decrement
returns including wraparound; random seed/default/state/range/float ordering;
original-array addresses after rebinding; I4/native signedness; raw denominator
bits and the possibility of 1.0 after rounding. It explicitly refused to invent
external getter behavior, infer dependency closure from inventory or treat
local CLI F precision as universal. This is one bounded reading exercise, not a
formal proof or model benchmark.

Artifact: session-managed `reviews/delivery-output-reading.md`, run
`b30f5f1e-fcd5-486e-8c03-4b167b3dadd6`. Inputs are preserved under ignored
`report/openra/delivery/`. The separate milestone B semantic review found no
scope-local diff-caused issues; parent validation, not reviewer claims, supplies
fresh execution evidence.

## Final gates

At corrected revision `8c878ac2f`, locked Cargo Debug/Release suites, fmt and
strict all-target clippy passed. The metadata checker and its ten
positive/negative tests passed. The oracle-only fix at `d1f1f7bea` also passed its
independent snapshot/output regressions; these were rerun after the name fixes. The five frozen class inventories and the
random-class, resource/ammo, Activity and Wait behavior gates passed again from
current public output. The lowering engine and remaining owned behavior inputs
are unchanged; their previous final-gate evidence is reused. Owned comparisons per
Debug/Release configuration:

| Gate | Comparisons |
|---|---:|
| References | 522 |
| Arrays | 557 |
| Native storage | 197 |
| Integer conversions | 385 |
| Stable element addresses | 1,892 |
| UTF-16 literals/references | 124 |
| Exception allocation/throw | 259 |
| Float conversions/constants/division/exact calls | 2,010 |
| Core constructor | 22 |
| Controlled clock | 5 |
| Delegated constructor | 20 |
| External identities/exact calls | 48 |
| Integer differential regression | 735 |

Behavior logs before correction: ignored `report/openra/final-gates/`.
Corrected final Rust/inventory/real-class behavior logs:
`report/openra/metadata-final/`. Reproduce the distinct real-class gates:

```sh
python3 rust/acceptance/openra/test_inventory.py
python3 rust/acceptance/openra/inventory.py --check-cli
python3 rust/tests/reference-roundtrip.py --random-class --from-class
python3 rust/tests/reference-roundtrip.py --openra --from-class
python3 rust/tests/reference-roundtrip.py --activity --from-class
python3 rust/tests/reference-roundtrip.py --wait --from-class
```

## Remaining boundaries

Production Rust uses pinned cildec 0.1.0 and no .NET runtime. Optional execution
uses local 64-bit .NET 10.0.12/macOS arm64; native width and CLI F precision belong
to the target runtime. The Single/literal adapters provide local evidence, not
32-bit execution, all-runtime F/NaN/subnormal/interning equivalence, general
external resolution or a complete constructor/hostile-input verifier. Wait's
.NET 6 framework references intentionally use .NET 10 runtime unification.

General value-type/generic/byref/virtual/EH semantics, full dependency resolution,
source/project export, source-style loop recovery and full .NET method support
remain out of scope. Other precise class selections can legitimately contain
partial methods. The large upstream ILSpy-tests submodule and .NET 11 test suite
were not run. No game installation/execution, input reacquisition/rehashing,
source recount or unrelated root/C# experiment edits were performed during final
acceptance. GPL inputs/generated outputs remain ignored. All intended changes
are local commits; no push was authorized or performed.
