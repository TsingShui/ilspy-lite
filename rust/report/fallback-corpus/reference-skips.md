# Reference-only inputs skipped from implementation acceptance

**362** inputs are skipped, all classified `verified_reference_contract` with a
reference marker and an inventory path recorded per input. They stay in the input
manifest and in the raw (all-input) baseline; they are not counted in the
implementation denominator and no recovery is claimed for them.

Source: `report/core-type-roots/scope-policy/skipped-reference-inputs.json`.

Each record carries: `id`, `assembly`, `sha256`, `mvid`, `origins`, `reason`, and
`evidence.reference_marker` plus the inventory file it was derived from, so the
classification is auditable without re-reading the binaries.

## By origin directory

| inputs | origin directory |
|---:|---|
| 233 | `dotnet/legacy` |
| 129 | `nuget/Microsoft.NETCore.App.Ref-5.0.0.nupkg!ref` |

## Why this is not a way to hide difficult code

The classification uses the reference marker recorded in the input itself (plus the
pinned-source evidence), not filenames, directories, or decompilation failures. An
input that carries real method bodies is never skipped, and the one exception that was
examined in detail (`dotnet/legacy/mscorlib.dll`) is recorded separately in
`acceptance/corpus/reference-exceptions.json` with its zero-code method counts and the
15 nonempty contract stubs that motivated its classification.
