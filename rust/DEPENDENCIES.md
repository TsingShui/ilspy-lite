# Static dependency analysis

Production reads local PE/metadata bytes only. It never starts dotnet, executes
input/dependency code, reflection-loads assemblies, restores packages or installs
an SDK. Existing byte-only `decompile` / `decompile_type` APIs remain detailed and
perform no implicit filesystem search. `DependencyContext` is the explicit
metadata/search interface. Resolving metadata does not expand body lowering.

## Inputs, identity and cache

`DependencyContext::new(Vec<u8>)` owns the main snapshot. `add_reference(source,
Vec<u8>)` and `add_reference_reader(source, reader)` accept immutable supplied
content; source strings are provenance labels, never paths to open. Defining
module indexes, not public labels, select snapshots. Reader overflow is rejected
before cache matching, so a truncated prefix cannot masquerade as a complete image.

Assembly matching includes name, four-part version, culture, public-key token and
content type. Names/cultures compare ASCII case-insensitively. Full public keys
use the standardized reversed final eight SHA-1 bytes. Tokens are legacy identity
labels, **not publisher authentication or strong-name verification**. File names
and framework-directory versions are not identities. Nominal signature keys quote
structured identities; ambiguous human-readable display labels are not keys.

Different explicit reference contents with the same identity conflict; identical
reference bytes cache once and retain source aliases. Changed bytes at a cached
source are rejected. Ordered search directories instead deliberately select the
first exact identity, not an arbitrary enumeration winner. Distinct filesystem
aliases can be conservatively rejected near the read budget; complete byte aliases
still deduplicate. Malformed AssemblyRef rows retain token/reason/raw-row evidence.

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
Explicit files precede explicitly ordered directories (DLL before EXE), then
these automatic families:

1. The current referring image's directory.
2. Adjacent `.deps.json` runtime assets in local NuGet caches, using metadata
   target-framework hints and safe, case-preserving cache-relative paths.
3. Installed .NET shared frameworks and separate SDK reference packs.
4. Windows .NET Framework/GAC and common Mono layouts.

A wrong/damaged early candidate does not hide a later exact identity. Selected
references are searched transitively in each referrer's scope. Cycles, missing
images, invalid requests and truncated searches retain diagnostics. `--no-auto-`
`references` disables automatic families, not supplied file/directory inputs.
Embedding defaults use empty injected roots; the CLI obtains host roots through
environment/filesystem inspection only. Missing SDK/runtime installations are
ordinary missing metadata, not prerequisites for independent bodies.

Framework hints come from static attributes, not the analyzer's runtime. Hints
order relevant versions before deterministic alternatives; directories themselves
never prove identity or compatibility. NuGet path containment and canonical
candidate deduplication prevent traversal/alias loops. Silverlight, WinMD/Windows
SDK specialization, archives/single-file containers, remote restore, RID-specific
runtimeTargets selection and full runtimeconfig roll-forward are not implemented.

## Exact versus metadata-only replacement

`ReferenceVersionPolicy::Exact` is the default. Explicit `SameMajor` selects the
nearest nonolder version with the same major/name/culture/token/content type only
when no exact candidate exists. Exact candidates are searched first. Inspection,
read/enumeration errors and resource truncation explicitly mark discovery
incomplete; incomplete discovery cannot authorize replacement. Exact matches
found independently remain usable.

Requested/selected identities, source and policy remain auditable. A substituted
type has `exact_identity=false` through forwarding hops. This is **not** API or
runtime compatibility proof. Property source recovery additionally checks owner,
accessor-reference, accessor-definition and PropertySig type paths; an exact
owner cannot launder substituted return/argument/property types into C# proof.

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
provenance. `ImplementationImage` means no recognized contract marker was found;
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
`DotNetCorePathFinder.cs`, `DotNetCorePathFinderExtensions.cs` and
`TypeSystem/DecompilerTypeSystem.cs`. Upstream MIT attribution remains in adapted
source. Short-name/highest-version unification was deliberately not adopted.

Owned tests cover isolated no-SDK/local/explicit order, deps/NuGet/shared/ref,
Framework/GAC/Mono layouts, identity conflicts/missing/corrupt requests,
replacement policy, generic positions, signature corruption, forwarding and
resource boundaries. Windows/Mono directory tests are injected layouts on macOS,
not native Windows execution. A native macOS .NET 10 filesystem-only lookup is
recorded separately. The optional [independent binding oracle](tests/DEPENDENCY_ORACLE.md)
compares 406 requests with PEReader catalogs and rejects 18 output mutations;
required fixture-shape preflights prevent vacuous omission of interfaces and
other promised shapes. Static binding is not generated-C# or runtime proof.
