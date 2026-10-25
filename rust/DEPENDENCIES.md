# Static dependency analysis

Dependency metadata is additional evidence, not execution and not a prerequisite
for all single-image lowering. The byte-based `decompile` / `decompile_type`
interfaces keep their existing contracts and never search the filesystem.

## Explicit context (implemented)

`DependencyContext::new(Vec<u8>)` owns an input snapshot.
`add_reference(source, Vec<u8>)` supplies another immutable snapshot; the source
string is provenance, not a file to open. `references()` exposes AssemblyRef
requests and `resolve(token)` returns `Resolved`, `Missing`, `Conflicting` or
`Invalid` results. Malformed request rows remain in `reference_failures()` with
raw bytes and diagnostics, without suppressing independently analyzable bodies. Its method/class decompile entries append dependency evidence separately
from body and metadata status counts. The byte context does not perform automatic lookup or change source binding.
The CLI now probes neighboring DLL/EXE files by default after explicit inputs;
system/package discovery and source binding are separate implementation milestones.

Identity matching uses assembly name, four-part version, culture, public-key token
and content type. Names/cultures compare ASCII case-insensitively. Full keys use
the ECMA-335 reversed final eight SHA-1 bytes, with the standardized ECMA public
key/token regression. Neither token equality nor this context authenticates a
publisher or verifies a strong-name signature. File names are never identities.

Different explicitly supplied byte/file contents with the same requested identity
are conflicting, not silently first-file-wins. Directory arguments instead define
an ordered search path: the first exact identity in that deliberate order is used,
not an arbitrary filesystem enumeration winner. Identical bytes are cached once; additional source
labels are retained. Replacing a cached source's bytes is rejected. Limits include
256 owned images (including main), 512 MiB total owned bytes and 65,536 references
per image by default; 24-bit token-width validation also applies. These are bounded
analysis guards, not a complete hostile-input verifier.

CLI automatic neighboring lookup and explicit inputs:

```sh
ilspy-lite decompile Assembly.dll --type Namespace.Type \
  --reference Dependency.dll --reference-dir ./references --no-auto-references
```

Options may follow the selector in any order; file/directory options repeat.
Explicit files are considered before directories. For an unresolved identity,
directories are checked in supplied order, DLL before EXE. An earlier wrong
identity or damaged file cannot hide a later exact match. Missing/rejected optional
inputs are diagnosed without suppressing independently recovered method bodies.
Body summary/exit meanings remain separate from dependency completeness.

Detailed output includes `.dependency-candidate` provenance/identity and
`.dependency` request/resolution records. Readable output omits resolved metadata
noise but retains missing/conflicting input warnings. Path labels and diagnostic
text are escaped. Directory probing refuses reference names containing path
separators, drive syntax, nulls or dot-directory names. File reads are bounded by
the remaining context budget and filesystem candidates are canonicalized/cached.
`add_reference_reader` accepts an explicit reader without opening any file itself.
Its overflow sentinel is rejected before cache matching, so a truncated prefix
cannot masquerade as a complete cached image. A distinct filesystem alias can be
conservatively rejected near the read budget; complete byte inputs still deduplicate
without charging image bytes again, and canonical same-path cache hits avoid I/O.

## Discovery integration and version policy (in progress)

Common candidate-path discovery is integrated: static framework attributes guide
local/deps/NuGet/shared/reference-pack/Framework/GAC/Mono probes. In embedding,
`ReferenceSearchOptions::default()` uses empty injected roots; the CLI injects
filesystem-only host discovery. No dotnet process or SDK is required. Explicit
files precede explicit directories, then automatic families. Every candidate still
passes complete-byte identity checks. Transitive discovery, reference-image
classification and bounded forwarder chains remain separate milestones.

Version policy defaults to `exact`; `--reference-version-policy same-major` is an
explicit metadata-only opt-in to the nearest nonolder version with the same major,
name, culture, token and content type. Search completes exact matching across all
candidate locations before selecting an alternative. Every substitution is reported
as requested versus selected identity and provenance, with a readable warning.
This is not an API-compatibility or runtime-behavior guarantee; type bindings carry
`exact_identity=false` through substituted assembly hops. Exact mode never takes
this fallback. Same-version conflicting explicit snapshots remain conflicting.

Type/member/property binding is implemented incrementally against explicit cached
bytes; reference-versus-implementation classification and bounded type-forwarder
chains are still pending. A resolved assembly is not proof that every member binds.
Do not infer these from a resolved AssemblyRef status in the explicit-context
milestone. `--no-auto-references` preserves the isolated mode as discovery is added.

Discovery will adapt the existing ILSpy rules, not copy their permissive identity
shortcuts. Source map:

- `ICSharpCode.Decompiler/Metadata/UniversalAssemblyResolver.cs`: probe families,
  directory order, Framework/GAC/Mono handling, filesystem-only path search.
- `ICSharpCode.Decompiler/Metadata/DotNetCorePathFinder.cs`: adjacent deps file,
  local NuGet directories, shared frameworks and separate reference-pack lookup.
- `DotNetCorePathFinderExtensions.cs`: metadata target-framework hints and
  reference-assembly classification.
- `TypeSystem/DecompilerTypeSystem.cs`: facade/exported-type traversal. Its
  short-name/highest-version unification is not an identity proof and is not the
  intended Rust policy.

Important distinctions from a wholesale port: ordinary ILSpy core lookup does not
include its separate reference-pack API; a version directory is not an assembly
identity; an unrelated DLL in a directory does not prove a requested assembly
exists; package cache paths must retain case; framework replacement must be
explicit/auditable, not inferred from the analyzer's host runtime. A reference
assembly supplies an API contract, not a runtime implementation to execute.

Read-only source research is retained as
`research/ilspy-dependency-discovery.md`, run
`d16bbd96-c195-42a1-bb29-bae5da5eca48`. It maps local/deps/NuGet/shared/ref/GAC/Mono
rules and proposes isolated fixtures; it did not execute DLLs or certify the Rust
implementation. Production discovery must not invoke dotnet, install frameworks,
fetch packages or require an SDK. Silverlight, WinMD and containers remain outside
the confirmed first-round scope.
