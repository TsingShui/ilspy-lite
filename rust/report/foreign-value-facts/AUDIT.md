# ILSpy reference resolution alignment

User direction: inspect ILSpy and use its mechanism, rather than inventing a
new strict/same-major rule for unresolved forwarder targets.

The existing production fixed-reference baseline fails FSharp.Core KeyValuePair
get_Key/get_Value and the captured constructed owner. The unchanged supplied
references select a netcore-2.2 mscorlib facade, whose forwarder asks for
System.Private.CoreLib, Version=0.0.0.0. binding-probe.json retains the static
resolution errors. Target bodies were not executed.

## Source mechanism

- ICSharpCode.Decompiler/Metadata/UniversalAssemblyResolver.cs:
  FindAssemblyFileCore/ResolveInternal choose search paths; SearchDirectory
  returns name.dll/exe by existence, without exact identity/version validation.
  IsZeroOrAllOnes treats 0.0.0.0 and 65535.* as special versions.
- Metadata/DotNetCorePathFinder.cs: TryResolveDotNetCore searches main/explicit
  paths and packages, then installed shared framework; GetClosestVersionFolder
  chooses the first installed version not older than the target framework.
- TypeSystem/DecompilerTypeSystem.cs InitializeCoreAsync:
  queue direct references, then every loaded module's assembly-implemented
  ExportedType forwarder targets; deduplicate loaded short names by highest
  AssemblyDef version, with a later equal version replacing an earlier one.
- TypeSystem/MetadataModule.cs ResolveModuleUncached calls
  TypeSystemExtensions.FindModuleByReference: first try the complete requested
  identity across compilation modules, then the case-insensitive short name.
  Referenced modules have already been deduplicated by highest version, while
  the main module remains distinct.
- MetadataModule.ResolveForwardedType.ResolveModule looks up the next module
  by case-insensitive short assembly name. Follow real definitions; missing
  endpoints stay UnknownType, not invented struct/enum layouts.
- Broken cycles have a separate next-to-forwarding-module repair pass.
  The missing CoreLib endpoint diagnosed here is not evidence of a cycle.

## Scope

Align the existing Rust discovery/binder's normal name-based selection and
forwarder closure with these ILSpy mechanisms. Keep requested identities and
selected definitions/provenance distinct; retain exact identity flags so a
substitute is not mislabeled an exact assembly match. Preserve the explicit
Exact mode for reproducible earlier scans and byte-only embedding behavior.

The user explicitly authorized ILSpy-style reference selection. Any new
reference-policy scan must be labeled separately from instruction implementation
gains, retain the old exact baseline and preserve the fixed input denominator.
No new target inputs/exclusions, implicit fabricated definitions, dependency
execution, original-body proxies or network acquisition.

## Selected-definition reference relationships

The first ILSpy-policy scan has 1,186 losses, all incompatible value/receiver
types. The registry still requires literal AssemblyRef identities even after
name-based ILSpy binding selects actual definitions. Inspecting ILSpy before
changing this seam: MetadataModule.ResolveType/ResolveForwardedType obtain the
selected module's actual type definition. CSharpConversions.IsSubtypeOf walks
GetAllBaseTypes; IdentityOrVarianceConversion compares those type definitions,
not the versions of the original AssemblyRefs. TypeSystemExtensions.GetAllBaseTypes
collects DirectBaseTypes. Therefore relationship facts in IlSpy mode must use
the selected definitions consistently; Exact/SameMajor policy keeps its
existing exactness guard. Requested identities and exact_identity=false remain
visible. No new casts, copies or runtime relationship checks are inserted.

The initial incomplete full scan is retained separately. A report-only failure
on three netmodules and one malformed/bodyless image aborted enumeration;
recording identity-unavailable as diagnostic repairs the report and a bounded
four-image rescan completes the unchanged denominator. Neither that bookkeeping
repair nor policy differences are instruction-level semantic gains.
