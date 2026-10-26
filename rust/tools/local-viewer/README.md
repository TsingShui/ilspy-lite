# Local ILSpy / Lite comparison viewer

A loopback-only developer tool: choose a local test assembly and type, then view
**real ILSpy output on the left** and **current Rust CLI output on the right**.
No cloud service, frontend packages, restore, or input execution.

## Start

From the repository root, using the existing uv-managed Python 3.14:

```sh
uv run --no-project --offline --no-python-downloads --managed-python --python 3.14 \
  python rust/tools/local-viewer/server.py
```

Open **http://127.0.0.1:8765**. Stop with Ctrl+C. Use `--port 8766` for another
port, or `--samples /absolute/directory` to add a local sample directory.
There is no Python dependency or virtual environment to install.

The developer-only ILSpy bridge uses the existing .NET 10 SDK, reference pack,
and runtime in `DOTNET_ROOT` (default `~/.dotnet10`). It reuses an existing
`ICSharpCode.Decompiler.dll`, preferring the repository Release build and then
the cached comparison engine. Override with `--engine /path/to/the/engine.dll`.
It never downloads or builds the full .NET solution. An engine/framework
Metadata version warning (CS1701) is retained in the ignored adapter build log.

The Rust binary is built with `cargo build --release --locked --offline`.
Subsequent comparisons rebuild when Rust sources, Cargo.toml, or Cargo.lock are
newer than that binary. Use `--no-build` to use only an existing binary.
Changes to the Python server or ILSpy bridge require restarting the server;
HTML/CSS/JS changes need only a browser refresh.

## Use

- Built-in files: `rust/tests/fixtures/*.{bin,dll,exe}`, the existing
  `rust/report/openra/assemblies/` cache, and `ILSpy-tests/` if already present.
  Nothing is acquired or resampled. BIN fixtures are managed PE files too.
- Search **original metadata type names** or TypeDef tokens, including nested
  and generated types. Empty types and interfaces are present in the inventory.
- Quick picks include Wait, WithHarvestOverlay, and a generated singleton owner.
- Import or drop up to 32 companion assemblies together (64 MiB per file).
  They stay on this machine under the ignored report directory, with companion
  filenames preserved. Imported files are not executed or added to Git.
- Rust views: Readable, Details, or original IL for one declared method.
  ILSpy remains a C# reference pane. Rust auto-reference discovery is optional.
- Link scroll is proportional, not semantic line alignment. Optional line-change
  highlighting is textual, not an AST or behavioral comparison.
- Copy/save always uses the complete original engine output, not highlighted DOM.
  The partial/failure counts, exit codes, timings, and diagnostics remain visible.
- Run again bypasses the result cache. URL fragments retain assembly/type/view.
- **Fallback report** aggregates every method body owned by top-level TypeDefs and
  their nested types: recovered/fallback/failed counts, the first-failure reason
  distribution, and the types with the most fallback bodies. Click a type to open
  it in the comparison. The same collector runs without the viewer:
  `python rust/tools/local-viewer/fallback_report.py <assembly> [--json report.json]`
  uses the existing release binary and never executes input code. The server also
  exposes the report as `GET /api/fallback-report?assembly=<id>`. Append
  `&report=1` to the URL fragment to open it directly. Collector unit checks:
  `python3 -m unittest rust/tools/local-viewer/test_fallback_report.py`.

The local server rejects foreign Hosts and cross-origin writes. Uploads and
comparison requests require a same-origin session token. Only allowlisted static
assets and registered assemblies are exposed, not arbitrary filesystem paths.
Do not proxy or publish this development server to a network.

## Limits

This is a visual workbench, not a proof of behavioral equivalence or complete
C# recompilability. ILSpy and Rust use their own dependency resolution policies;
missing references, unsupported bodies, and engine failures remain visible.
The bridge uses PEReader and ILSpy's static metadata APIs, never runtime-loads
the selected assembly. Rust's production CLI stays independent of .NET.

Engine output is limited to 16 MiB per stream and each engine call has a timeout.
Large outputs beyond that limit must be inspected with the CLI. Textual diff
highlighting is skipped above 16,000 combined lines; the source itself is retained.
Fallback reasons are the first blocker reported for a body, so fixing one does not
guarantee recovery; later blockers are not included in the report. Dependency
resolution affects presentation only and does not change method statuses. If the
report says `Not found`, the running Python server predates the endpoint; restart
`server.py` and reload the page. The viewer reports its capabilities at startup and
shows that restart message instead of sending an unknown request.
Generated output, uploaded DLLs, compiler artifacts, and screenshots stay under
`rust/report/local-viewer/` (ignored).
