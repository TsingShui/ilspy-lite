# External constructed base-constructor call audit

Baseline: `31562d2e9`, latest complete production scan
`report/modopt/corpus-scan/baseline.json` (728,029 / 770,748). The first-error
count for external constructed members includes multiple call shapes; it is not
a forecast of constructor recoveries. Inputs, references and scope are unchanged.

## ILSpy trace

`ICSharpCode.Decompiler/IL/ILReader.cs::DecodeCall` calls
`ReadAndDecodeMethodReference`, obtains arguments in evaluation order and creates
an exact `CallInstruction` for a reference-type `.ctor`; value-type constructor
calls are distinguished by the receiver address. `MetadataModule.ResolveMethod`
and `ResolveMethodReference` resolve a MemberRef parent in the caller context,
find a constructor against the definition signature and then specialize with the
constructed declaring type's arguments. Missing definitions can produce a fake
method carrying the reference signature, not a proof of runtime compatibility.

## Captured candidate triples

`audit/` contains production default output, production original IL, and
`ilspycmd` output for these methods. CLI exits were 2 for Partial and 0 for IL;
ILSpy exited 0. No third-party body was executed.

| Method | Observation |
|---|---|
| Owned `NestedForeign..ctor`, 0x06000068 | Original IL calls `OpBase<!1,!0>::.ctor(seed)` on `this`. ILSpy prints an explicit constructor call with the type arguments in a different order, so this sample's ILSpy source is suspicious rather than an oracle; original IL governs. |
| ILSpy-tests `RemoveCompilerAttribute..ctor`, 0002-06000305 | IL calls the referenced constructed `DepthFirstAstVisitor<object,object>..ctor` on the existing receiver; ILSpy prints an empty constructor with implicit base initialization. |
| OpenRA `FacingInit..ctor`, 1042-0600008B | IL calls `ValueActorInit<WAngle>..ctor(value)` on the existing receiver; ILSpy prints `: base(value)`. The default reference policy reports substituted framework references, which are metadata selections only. |

No unknown-result diagnostic appeared in the latter two ILSpy outputs. The
owned sample cannot establish semantic equivalence from ILSpy's source alone.

## Boundary to test before admission

The previous swap mutation changed a single TypeSpec shared by both the declared
base and MemberRef parent, so it did not create unequal operands. Build two
owned nested constructors whose base TypeSpecs contain `!1,!0` and `!0,!1`, and
patch only the first constructor's IL MemberRef token to the second constructor's
reference. Assert the rows are distinct and their owners have different signature
bytes. The patched body must remain Partial; the unpatched body is the red positive.
The existing constructor context, original-this, exact token, non-virtual dispatch,
argument substitution and immediate-base checks must remain in force. An owned
behavior gate must exercise the emitted exact operation, not proxy the whole method.
No allocation, generic-virtual dispatch or arbitrary constructed member is admitted.

## Red, green and bounded validation

- `red.log` records the public positive failing with the existing constructed
  owner refusal. `callvirt-red.log` records the separate local-constructed
  `callvirt` defect: its default output was `this..ctor(seed)` without the
  original constructor token. Both focused tests pass in `first-green.log`.
- The distinct-row mutation changes only the IL call token to a different
  MemberRef whose TypeSpec has the same generic definition and swapped type
  arguments. Public lowering stays Partial and the direct immediate-base test
  rejects the same mismatch. `direct-base-green.log` records an initial test
  authoring compile error (`Result` resolved to the crate alias rather than
  `cildec::Error`); `direct-base-corrected.log` records the passing test.
- `external-base-constructor.log` records 48 comparisons across Release/Debug
  and call/callvirt, plus 16 semantic mutant rejections. The driver invokes the
  owned original selected constructor only for the reference result; the recovered
  body uses its own default-output statements and an exact base-operation leaf.
  The unmutated original must execute and produce the expected exception for the
  failure input, so matching two failed calls cannot pass.
- `clippy-final.log` and `nextest-final.log` record the relevant complete Rust
  regression: fmt, clippy and 492 tests pass. The reference-relations,
  external-allocation, exception and value-call behavioral gates also pass.
- `targeted.json` compares all baseline body tokens on five images with individual
  production CLI scans: 88 gains and no status regressions among those images.
  It is not a complete corpus scan or an updated aggregate gate result.
- `review/review-source.md` documents five gained-method main-session self-reviews
  using the retained default/ILSpy/IL triples. No third-party method was executed.
