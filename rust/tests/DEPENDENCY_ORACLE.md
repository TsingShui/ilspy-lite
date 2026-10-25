# Optional static dependency-binding oracle

From the repository root, with the existing .NET 10 SDK/reference pack/runtime and
Cargo's pinned packages already installed:

```sh
python3 rust/tests/dependency-oracle.py --self-test
DOTNET_ROOT="$HOME/.dotnet10" python3 rust/tests/dependency-oracle.py
```

The runner uses direct `csc.dll` compilation, following `ClassInventory.cs` and
`acceptance/openra/inventory.py`. It does not restore, download packages, rebuild
fixtures, or invoke MSBuild. The Rust exporter builds with `cargo build --offline
--locked --example binding_inventory`. A prebuilt exporter can be selected with
`--exporter /absolute/path/to/binding_inventory`; `--output` changes the report
directory. Validation used SDK 10.0.401/runtime 10.0.12. .NET is optional test
infrastructure, never a Rust production dependency.

## What is compared

`dotnet/BindingInventory.cs` opens the owned BindingCases main, library, facade,
and reference-contract Debug/Release images with `PEReader`/`MetadataReader`.
It never reflection-loads or executes an input assembly. Its catalogs contain
assembly identities, MVIDs, definition tokens/owners/names/flags, raw signatures,
declared generic positions, decoded signatures, reference scopes, relation tokens,
MethodSemantics rows/accessor tokens, exported-type rows, and reference-contract
marker bytes.

`examples/binding_inventory.rs` accepts a main image, a JSON request list, and
explicit reference paths. It calls the public `DependencyContext` methods
`add_reference`, `resolve_type`, `resolve_method`, `resolve_field`,
`resolve_property_accessor`, and `type_relations`. It does not inspect metadata
rows itself. The Python comparison independently selects fixture definitions from
the .NET catalogs using scoped names and decoded *declared* signatures, retaining
VAR/MVAR positions rather than conflating `Echo(T)` and `Echo(int)` on
`Generic<int>`. It checks:

- Selected assembly identity/source/token, type/method/field flags, and owners.
- Original MemberRef signatures and TypeRef/TypeSpec scopes, generic arguments,
  raw selected method/field signatures, and MethodSpec target/instantiation bytes.
- Property token/name/owner/raw signature and MethodSemantics getter/setter
  role/selected-method endpoint, including a non-property `get_Pretender` method.
- Base/interface relation tokens and definitions, retaining unresolved framework
  relationships and requiring a diagnostic rather than fabricating definitions.
- Reference-contract classification against independent marker bytes.
- Forwarder provenance, including nested generic ownership through the facade.

Per configuration, the scenarios check 69 main-to-implementation results, 69
main-to-reference-contract results, 61 library-local results, and four forwarded
type results: 406 comparisons across Debug and Release. The forwarder scenario
redirects AssemblyRef.Name in a **report-only copy** of the main image, using
file offsets from the independent catalog. It repurposes the copy's
`get_VirtualFlag` name slot to avoid changing a heap entry shared with a namespace;
this scenario requests types only. That copy is cataloged again before binding.
The owned fixture sources and binaries are never modified.

## Mandatory fixture-shape preflight

Before generating binding requests, the runner asserts named shapes in the
independent catalogs rather than treating whatever rows remain as sufficient
coverage. Debug and Release must each retain:

- `Derived -> Base`, `Base -> IFlag` in both implementation and contract, the
  interface flag, and the class-to-framework base relationships.
- The contract's identity and reference marker; `Generic<T>`'s distinct declared
  `Echo(T)`/`Echo(int)` signatures; type/method generic positions; named generic
  TypeSpecs and the `Convert<string>` MethodSpec target/arguments.
- The `Outer<T>/Inner<U>` declaring type, inherited generic positions, scoped
  main references, and left/right field positions.
- Required property getter/setter endpoints and MethodSemantics rows, plus the
  non-property `get_Pretender` method and required scoped MemberRefs.
- Facade exports for `Peer`, `Outer`, and nested `Inner`, their targets/parent,
  and an actual facade hop in every forwarded request's catalog provenance.

These assertions prevent coverage from silently shrinking when a required
relationship disappears, even if binding results still match the reduced
catalog and request counts stay unchanged.

## Failure sensitivity and evidence

Every normal scenario also writes three mutated **exported JSON copies** under
`rust/report/dependency-oracle/`: a changed selected token, an appended raw
signature byte, and a removed property relation. Each must fail the same
comparison that accepted the unmodified output. Across both configurations,
18 mutations must be rejected. Eight self-tests include the four focused output
checks for missing/duplicate/error results and mismatched signatures. The other
four tests compile the independent reader into a temporary report directory,
read both configurations' owned fixture bytes, accept the original catalogs,
and reject 50 required-shape omissions/changes in copied catalogs or forwarder
request setup. These include removing `Base -> IFlag` separately from both the
implementation and contract catalogs and removing an accessor setter while its
getter remains. No fixtures are regenerated or modified. The self-tests require
the same optional .NET 10 installation as the full gate. Missing expected
fixtures, definitions, properties, overloads, or output requests fail assertions;
there are no successful skips.

The ignored report directory holds the independent catalogs, request lists,
actual exporter JSON, mutation copies, compiler/build logs, and `summary.json`
with counts and rejection reasons. The runner removes a stale summary at startup
and writes a new summary only after all scenarios pass. Run Python without `-O`
so assertions remain enabled.

## Bounds

This supplements `dependency_binding.rs`; it is not a general CLR binder,
execution test, runtime implementation proof, publisher authentication, or
proof of generated C# behavior. Framework TypeRefs/attribute constructors are
cataloged but not requested as successful bindings because only owned images
are supplied. TypeSpec handling deliberately requires the fixture's named
generic instantiations. Assembly-key checks deliberately require these unsigned
fixtures. Conflicts, missing references, wrong identities, malformed metadata,
and other failure-policy cases remain covered by the existing public tests.

Property.Flags and raw MethodSemantics row identifiers/bytes are **catalog-only**:
the public accessor result does not export those fields, so they are not claimed
as cross-reader comparisons. Accessor semantic roles and endpoint tokens are
compared. The Rust-formatted `signature_identity` is required to be nonempty but
is not independently reproduced; raw signatures, decoded declared selection,
and exported generic argument positions are the cross-reader evidence instead.
