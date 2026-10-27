# AGENTS.md

Guidance for coding agents working on the Rust ILSpy Lite decompiler.

## Project structure

- `rust/` contains the Rust CIL-to-C#-like decompiler and CLI.
- `rust/src/` contains the metadata, typed IL, control-flow, lifting, and output code.
- `rust/tests/` contains Rust tests and owned .NET fixtures.
- `rust/acceptance/` and `rust/report/` contain acceptance tooling and evidence.
- `ICSharpCode.Decompiler/` is the ILSpy implementation used as a reference.
- `ILSpy-tests/` is a large git submodule containing real-world assemblies and fixtures. It is not checked out by default. Keep its `Directory.Build.props` and its intentionally warning-generating fixtures unchanged.

## Implementation approach

- **Study ILSpy before implementing.** Before each development group, read the corresponding ILSpy implementation and trace its actual call chain. Record the source paths, key functions, and mechanism in the group's audit before writing code. Follow that mechanism when implementing the Rust counterpart instead of independently inventing an alternative design.
- Decompilation pattern matchers should require the structurally guaranteed shape. Loosen them only when a legitimate fixture is rejected; capture that fixture as a regression test first.
- When asked to implement a C# language feature, require the relevant speclets from the csharplang or csharpstandard GitHub repository. Ask the user about architectural decisions, including the intermediate representation, transformation order, type system, metadata, resolver, and conversions. If the answers are insufficient, stop. Validate the spec against compiler implementations. Do not put more than one C# language feature on one branch.
- Investigate NuGet dependency internals with this repository's `ilspycmd`, not by grepping binaries.

## Code conventions

- Do not undo changes made outside the session without explicit confirmation.
- Comments must stand on their own and describe the code as it exists, without references to the chat, PR, or implementation history.
- Use ASCII-only source code and comments unless a non-ASCII character is genuinely necessary. Use en-US English in source, identifiers, comments, and log or error strings.
- Tests must assert when an expected component is missing; do not silently return early.
- New `.cs` fixture files must use the repository's standard MIT X11 license header, with the human contributor's name. Do not rewrite existing headers or add headers retroactively.

## Build and validation

Run Rust commands from `rust/`:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo nextest run --no-fail-fast
cargo build --release --locked
```

Keep validation proportional to the change. Validate each group with focused tests, `fmt`, `clippy` and the owned behavioral drivers that cover it. Run the full fixed-corpus scan and the gained-method self-review in batches of at least three completed groups rather than after every group. Test changed behavior; do not expand test infrastructure or matrices without a concrete need. Preserve failures, and reproduce them before calling them flaky.

A batched scan detects a regression later than a per-group scan would. Keep each group as its own local commit so a reported regression can be attributed by rebuilding and rescanning the individual commits instead of guessing. Never report a batch's net gain without checking `regressions`.

Reuse recorded hashes for unchanged inputs; verify them when acquiring or replacing inputs, investigating corruption, or explicitly requested.

## Commit and review

- Use a succinct commit subject, preferably at most 72 characters. Explain the reason and non-diff context briefly in the body.
- Every agent-authored commit must include `Assisted-by: AGENT_NAME:MODEL_VERSION:HARNESS`, using the actual agent, model, and harness. Do not use an AI `Co-Authored-By:` or add `Signed-off-by:`.
- Let the pre-commit hook run; do not use `--no-verify` for commits containing `.cs` files. For partial commits, stash the unstaged remainder before committing so the formatting hook can run.
- On Linux and macOS, leave new files with LF line endings. On Windows, save new files with CRLF.
- For a code review, follow `.github/CODE_REVIEW.md`. Report findings without posting to a PR or setting approval status unless explicitly requested.
