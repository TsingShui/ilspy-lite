# ILSpy-style reference selection

## Why this exists

The fixed exact reference selection left whole families of bodies blocked on
"external/value signatures need IL fallback" and "incompatible value/receiver
types". Inspecting one case showed the cause: the fixed netcore-2.2 `mscorlib`
facade forwards to `System.Private.CoreLib, Version=0.0.0.0`, and an exact
identity rule cannot select any installed CoreLib. ILSpy resolves that reference.
The user directed that this repository follow ILSpy's mechanism instead of
inventing a stricter one.

## Copied mechanism

- `Metadata/UniversalAssemblyResolver.cs`: search directories are probed for
  `name.dll`/`name.exe` by existence; `IsZeroOrAllOnes` treats `0.0.0.0` as a
  special version.
- `Metadata/DotNetCorePathFinder.cs`: installed shared frameworks are searched,
  and `GetClosestVersionFolder` takes the first installed version not older than
  the target framework. Without a managed host's preferred major, the newest
  installed version is used.
- `TypeSystem/DecompilerTypeSystem.cs`: the reference closure queues direct
  references and then every assembly-scoped exported-type forwarder of each
  loaded module; loaded short names are deduplicated to the highest version.
- `TypeSystem/MetadataModule.cs` and `TypeSystem/TypeSystemExtensions.cs`:
  `FindModuleByReference` tries the complete requested identity first and then
  the case-insensitive short name; forwarded types resolve by short name.

The Rust counterpart keeps those steps but keeps its own reporting: requested
identity, selected identity, source, and policy are emitted per reference, and a
selection that differs from the request is `Substituted` rather than `Resolved`.
`ReferenceVersionPolicy::{Exact, SameMajor}` remain available for the earlier
reproducible scans and for byte-only embedding callers.

Selected-definition ancestry is also used by reference relationships in this
mode, matching ILSpy's `IsSubtypeOf`/`GetAllBaseTypes`, which compare type
definitions rather than the versions of the calling AssemblyRefs.

## Evidence

Owned tests:

- short-name selection keeps the highest loaded version of a name while the main
  module stays distinct;
- a direct reference prefers a full-identity match over a same-name main module,
  while a forwarder resolves by short name;
- the shared-framework probe follows the target-framework hint, or the newest
  installed version when the hint names another family;
- exact and same-major behavior is unchanged.

Owned behavior drivers: 143 comparisons / 36 closed selections per configuration
over a version-substituted library (four mutants rejected), plus the existing
external enum (30 comparisons, three mutants), external allocation (26
comparisons, two mutants) and external property drivers.

Fixed corpus, same 614 managed images / 319 body-bearing images / 770,748 bodies,
zero incomplete images:

| Selection | Decompiled | Partial | Fallback |
|---|---:|---:|---:|
| fixed exact references | 566,419 | 204,329 | 26.51048% |
| ILSpy-style discovery | 616,563 | 154,185 | 20.00459% |

50,146 bodies newly recovered in 211 images, with **two recorded coverage
losses**: `NUnit.Framework.Syntax.TestCompiler.CompileCode` in the net-2.0 and
net-3.5 inputs. Both previously recovered because the older fixed `System`
definition proves the `CSharpCodeProvider` receiver type; the installed `System`
facade forwards CodeDom to `System.CodeDom.dll`, which is absent from the selected
host framework. ILSpy prints an explicit cast in the same position. No guessed
ancestry or added cast suppresses the loss; both remain Partial and counted.

OpenRA.Game 3,644 / 1,123 (+531); OpenRA.Mods.Common 13,489 / 4,918 (+1,360).

## Limits

This changes which metadata is selected, so it is not a like-for-like instruction
gain and is reported separately from matcher work. The exact-policy scan is
retained for comparison. Not reproduced: cyclic forwarder repair, multi-module
`AssemblyFile` exports, host-runtime/preferred-major selection, RID-specific
assets and full path ordering. Selected metadata is not proof of compatibility
with an arbitrary target runtime; the only claim is that a body now has the facts
it needs to be recovered, with requested/selected identities retained.
