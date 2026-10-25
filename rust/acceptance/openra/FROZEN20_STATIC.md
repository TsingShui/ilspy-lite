# Frozen twenty: optional static collector

`validate_random_classes.py` reads **all** selections from `random-classes.json`.
It does not sample, filter on support, download, rehash, load the input as a
runtime assembly, or execute input/recovered bodies. Existing provenance is
copied into the report, not independently reacquired or recertified.

From the repository root, using already acquired assemblies/dependencies:

```sh
(cd rust && cargo build --release --offline --locked)
python3 rust/acceptance/openra/test_validate_random_classes.py
python3 rust/acceptance/openra/validate_random_classes.py \
  --cli rust/target/release/ilspy-lite \
  --assemblies /absolute/path/to/existing/assemblies \
  --reference-dir /absolute/path/to/existing/runtime-deps \
  --output rust/report/openra/frozen20-static-NEW-RUN
```

Use a **new independent** output directory for each run. Never point it at
another worktree's reports or symlink the output directory there. GPL-derived
IL and body output must stay in ignored `rust/report/`, not tracked fixtures or
documentation. Inputs are read-only. No .NET build/restore is required: the
optional oracle is compiled directly using the existing .NET 10 SDK/reference
pack and `tests/dotnet/ClassInventory.cs`, following `inventory.py`.
`DOTNET_ROOT` selects its installation (default `~/.dotnet10`);
`--dotnet /absolute/path/to/dotnet` overrides its executable.
`--oracle /absolute/path/to/ClassInventory.dll` reuses that independent reader
with its matching runtime configuration, avoiding recompilation.

Each class has actual default `decompile --type` and `--type --details` process
records in both configurations:

- `automatic`: CLI defaults, with no reference-search overrides.
- `explicit`: `--no-auto-references --reference <companion assembly>`, plus
  repeatable `--reference` and/or `--reference-dir` arguments if supplied.
  Extra dependency arguments apply only to this configuration. Exact CLI
  reference-version policy is retained; merely supplying a directory does not
  guarantee resolution of differently versioned CLR references.

`report.json` retains the frozen identities/seed and links every class JSON.
Class reports retain independent member inventories, root/nested ownership,
all method statuses/signatures/tokens, original `il --method` evidence,
fallback diagnostics, supported-method candidate triage and errors. Every
process has raw stdout/stderr byte files, argv and exit code, including failures.
The independent comparison reuses `inventory.check_output`; additional checks
cover exact names, frozen MVID/root identity, original IL presence, complete
detailed/readable fallback IL, readable markers and status/exit/style counts.

Default/readable output is independently checked as a declaration tree using the
renderer's four-space indentation, header/open/close lines and member lists.
Each selected owner must retain its fields, methods/constructors (including
fallbacks), property/event relationship comments and directly owned nested types.
Method parameter counts come from the independent raw signature prefix, not Param
row counts; generic arity, static shape and body/declaration boundaries must match.
Indexer counts and accessor names/roles also match the independent catalog.

Each observed method retains its declaration line and body span (one-based line
numbers; body end is exclusive). Fallback IL is checked inside a distinct matched
body of the correct owner and method shape, never against the whole class output.
Same-shape overloads consume bodies as a multiset using their complete available
fallback evidence; one body cannot satisfy multiple methods. Every Partial body
must retain its own fallback notice and original IL. Failed bodies must retain
their own error and all available detailed raw evidence; they still fail the gate.

Names are predicted from catalog metadata, never learned from actual output:
invalid ASCII identifiers use `__name_<uppercase UTF-8 hex>`; invalid/reserved-prefix
field names use `__il_field_<eight-digit token>`. Leading `@` keyword escapes are
normalized. Type names drop namespace/ownership and the metadata backtick suffix.
Explicit `__type_<token>`, `__method_<token>` and `// Unreadable field 0x<token>`
markers can preserve declaration identity; method marker parameter/generic/static
shape is unavailable, not certified. These markers remain visible in the report.
Unidentifiable unreadable declarations fail rather than being skipped.

This is **static declaration presence/tree checking**, not semantic C# parsing.
Readable tokens in the report map catalog identities to matched declarations;
same-name/same-arity overloads are counted as a multiset, not type-bound to tokens.
Exact signatures and full metadata ownership are additionally compared against
**details**. Method operations, type binding and body semantics are not certified.

Exit 0 means all twenty **static** checks passed; marked unsupported CLR/body
partials are allowed and counted. Inventory disagreement, missing classes,
identity errors, failed bodies, lost original/fallback IL and inconsistent
status accounting exit 1 without dropping bad rows. Setup failures also retain
failed selection rows. Existing output directories are refused.

`Decompiled` means the CLI reported supported lowering, not recompilability,
behavioral equivalence or constructor-chain certification. Candidate applicability
is heuristic static triage, not approval of a runtime fixture. Every report says
`noRuntimeProof`; a baseline run is not a final semantic gate. Build/rerun at the
final checkout and perform separately authorized behavior comparisons before
making any final acceptance claim.
