# Static dependency analysis

Production reads local PE/metadata bytes only. It never starts dotnet, executes
input/dependency code, reflection-loads assemblies, restores packages or installs
an SDK. Existing byte-only `decompile` / `decompile_type` APIs remain detailed and
perform no implicit filesystem search. `DependencyContext` is the explicit
metadata/search interface. Resolved metadata can provide facts used by body lowering.

The closed-generic reference slice interns canonical structural signatures within
each parsed input. TypeId handles cannot cross inputs; metadata tokens retain
their requested scope, and local TypeRefs use the existing exact module resolver.
This does not add a second dependency binder, infer external layout or prove
generic member/variance relationships. Body VAR/MVAR identities carry their
actual TypeDef/MethodDef owner and position; member interpretation cannot capture
an unrelated caller's variable context. Generic values have a separate transfer
category and do not imply concrete layout or boxing. Opaque alias names
are not binding keys; full structural keys determine type equality. See
[closed generic references](acceptance/corpus/CLOSED_GENERIC_REFERENCES.md).

## Inputs, identity and cache

`DependencyContext::new(Vec<u8>)` owns the main snapshot. `add_reference(source,
Vec<u8>)` and `add_reference_reader(source, reader)` accept immutable supplied
content; source strings are provenance labels, never paths to open. Defining
module indexes, not public labels, select snapshots. Reader overflow is rejected
before cache matching, so a truncated prefix cannot masquerade as a complete image.

Assembly identities record name, four-part version, culture, public-key token and
content type. Exact matching includes all of them; ILSpy-style lookup can select a
different identity by short name. Names/cultures compare ASCII case-insensitively. Full public keys
use the standardized reversed final eight SHA-1 bytes. Tokens are legacy identity
labels, **not publisher authentication or strong-name verification**. File names
and framework-directory versions are not identities. Nominal signature keys quote
structured identities; ambiguous human-readable display labels are not keys.

Identical reference bytes cache once and retain source aliases. Changed bytes at a
cached source are rejected. Exact-policy binding reports conflicting contents with
the same identity; ILSpy-style binding retains the highest loaded version of each
short name, with the later file winning an equal-version tie. The main module
remains distinct. Malformed AssemblyRef rows retain token/reason/raw-row evidence.

## CLI and lookup order

```sh
ilspy-lite decompile Assembly.dll --type Namespace.Type
ilspy-lite decompile Assembly.dll --type Namespace.Type \
  --reference Dependency.dll --reference-dir ./references
ilspy-lite decompile Assembly.dll --type Namespace.Type --no-auto-references
ilspy-lite decompile Assembly.dll --type Namespace.Type \
  --reference-version-policy same-major --details
```

Post-selector options may appear in any order; file/directory options repeat.
Both CLI decompile and scan use ILSpy-style discovery by default. Explicit files
and ordered directories are considered before automatic candidates. Automatic
lookup uses the main image's directory, adjacent deps/NuGet assets, a selected
installed shared framework, and available Framework/GAC/Mono paths. It walks
assembly-scoped exported-type forwarders of selected references.

ILSpy-style binding compares a direct reference against the full identities of
loaded modules first, then uses the short name. Forwarders use the loaded short
name. This adapts ILSpy's core selection and forwarder mechanisms, not every
resolver feature: cyclic-forwarder repair, multimodule exports, host-runtime
selection and all path ordering cases are not fully reproduced. Search is local
and metadata-only; missing endpoints remain unresolved. `--no-auto-references`
disables automatic families but still accepts supplied files/directories. Embedding
uses injected roots; the CLI discovers host roots from the environment/filesystem.

Framework hints come from static attributes, not the analyzer's runtime. Hints
order relevant versions before deterministic alternatives; directories themselves
never prove identity or compatibility. NuGet path containment and canonical
candidate deduplication prevent traversal/alias loops. Silverlight, WinMD/Windows
SDK specialization, archives/single-file containers, remote restore, RID-specific
runtimeTargets selection and full runtimeconfig roll-forward are not implemented.

## Reference selection policies

The CLI and `ReferenceSearchOptions::default()` use `IlSpy`; byte-only
`DependencyContext::new` does not search and retains `Exact` until configured.
`--reference-version-policy exact` preserves exact identity matching;
`same-major` permits only the nearest nonolder version with the same
major/name/culture/token/content type when no exact candidate exists.

`ilspy` follows loaded short-name selection, including version changes and
potentially other identity differences. Requested/selected identities, source
and policy remain auditable; substituted type paths retain
`exact_identity=false`. The selected metadata is not proof of compatibility
with every target runtime or assembly of the same name. Property source recovery
continues to require exact owner and signature paths. Selected-definition
ancestry is used for reference relationships in ILSpy mode, without adding
runtime casts, allocations or copies.

## Binding and metadata roles

`resolve_type`, `resolve_method`, `resolve_field`, `resolve_property_accessor` and
`type_relations` return actual scoped definitions, original signatures and
provenance. Binding covers assembly/module/nested scopes, named generic owner
positions, MethodSpec arguments, real Property/MethodSemantics relationships,
base/interface relation rows and bounded assembly type-forwarder chains. Forwarders
must terminate in actual definitions; duplicates/cycles/missing terminals fail.
Multi-module File exports remain unresolved. Method/field selection compares
**declared** signatures before instantiation: `Echo(T)` and `Echo(int)` must not
collapse on `Generic<int>`.

Checked signature preflight validates EOF, canonical compression, full coded-token
width and bounded work/depth before the pinned decoder can truncate evidence.
The context is a finite metadata binder, not a full CLR verifier, inherited-member
lookup engine, C# overload binder or generic/value-type/EH body implementation.
Unresolved relation edges remain separate; a missing base does not erase an
independently readable interface or method.

`ReferenceContract` means a recognized ReferenceAssemblyAttribute or reference-pack
provenance. The marker may use an external core-scoped MemberRef or a local
MethodDef on the top-level nongeneric marker type in an admitted core definition.
Constructor flags/signature and the attribute payload remain checked; a local
lookalike in an ordinary assembly is not core evidence. Core definition identity
is cached per parsed image, not by short name or shared across images. `ImplementationImage` means no recognized contract marker was found;
it is **not execution proof**. Malformed classification is `Unclassified`.
Explicit-context output labels reference input stubs as API contracts, never
runtime implementations. Dependency bodies are not executed or borrowed.

## Reports and limits

Body/metadata success counts and exit codes remain independent of dependency
completeness. Readable output keeps conflicting/invalid/substituted warnings;
large missing sets use one explicit unresolved-count notice rather than drowning
the body. Exact calls remain visible when source binding cannot be proved.
`--details` retains `.dependency-main`, `.dependency-candidate`, per-referrer
`.dependency-request`, probe/diagnostic and final main-reference `.dependency`
records, including requested/selected identities and metadata-only limits.
Discovery request records are a trace, not a promise that later incomplete search
or conflicting input could not change a final resolution.

Default owned-image limits are 256 images including main, 512 MiB owned bytes and
65,536 references/image; full 24-bit RIDs are checked. Closure limits include depth
32 and 4,096 requests. `max_loader_probes` has a hard ceiling of 4,096 loader
file/directory inspections, including missing candidates; zero stops before
automatic enumeration. The path enumerator has separate per-call bounds: 64
roots/family, 512 directory reads, 1,024 entries/directory, 32,768 total entries,
16,384 probes, 1,024 candidates, 1 MiB deps JSON and 4,096 manifest visits. These
are layered analysis guards, not a global 4,096-OS-call ceiling, memory sandbox or
complete hostile-input verifier.

## Sources and finite validation

Rules were adapted from ILSpy's `Metadata/UniversalAssemblyResolver.cs`,
`DotNetCorePathFinder.cs`, `TypeSystem/DecompilerTypeSystem.cs`,
`TypeSystem/MetadataModule.cs` and `TypeSystem/TypeSystemExtensions.cs`.
Short-name/highest-version selection and forwarder walking follow those mechanisms
within the documented limits. Upstream MIT attribution remains in adapted source.

Owned tests cover isolated no-SDK/local/explicit order, deps/NuGet/shared/ref,
Framework/GAC/Mono layouts, identity conflicts/missing/corrupt requests,
replacement policy, generic positions, signature corruption, forwarding and
resource boundaries. Windows/Mono directory tests are injected layouts on macOS,
not native Windows execution. A native macOS .NET 10 filesystem-only lookup is
recorded separately. The optional [independent binding oracle](tests/DEPENDENCY_ORACLE.md)
compares 406 requests with PEReader catalogs and rejects 18 output mutations;
required fixture-shape preflights prevent vacuous omission of interfaces and
other promised shapes. Static binding is not generated-C# or runtime proof.
