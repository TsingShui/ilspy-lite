# Readable view: string literals printed as literals

Quality group 1 of `report/quality-full/MERGED.md`, first slice. The 24 review
slices recorded 34 examples of `__cil_ldstr_<token>("...")` standing in for a
plain string literal (e.g. `string v0 = __cil_ldstr_70000003("matchCasesIncomplete"); return v0;`
against ILSpy's `return "matchCasesIncomplete";`).

## Change

`readable::method_body` now rewrites `__cil_ldstr_<HEX>("literal")` to the literal
itself. The helper existed only to record the original module string-literal load
position, and the value it yields is exactly the literal, so nothing is dropped
from semantics: the load still happens at the same instruction, in the same order,
and no new allocation is implied. `--details` is untouched and keeps both the
helper call and the `original module string-literal policy` note.

The rewrite parses the literal conservatively: it requires an all-uppercase-hex
token, a single quoted run with backslash escapes, and a closing `)`; anything
else is left as written, so a future helper shape cannot be mangled.

## Red then green

`strings::readable_literals_are_printed_instead_of_load_helper_calls` failed first
with the helper call still present, and passes after the change. It asserts the
literal text, the absence of `__cil_ldstr_` in the readable view, ASCII-only
output, and that the details view still carries the helper and its note.

`tests/strings.rs` keeps its existing assertions on the details view unchanged;
the earlier `naming` fix (metadata string constants) is a different path and stays
covered by its own test.

## Behaviour

Owned drivers re-run on the release binary, unchanged results:

- static constructors: 13 state comparisons over 9 recovered types
- virtual calls: all receivers and mutants rejected
- method instantiation: all mutants rejected
- value calls: 720 comparisons, 80 closed selections
- external base constructors: 12 comparisons per call form

Rust: 500 tests, `cargo fmt --check` and clippy `-D warnings` pass. Note that
`cargo nextest` must run with `DOTNET_ROOT` set, otherwise
`cli::type_output_keeps_nested_state_machine_methods_visible` fails on reference
resolution rather than on the change under test.

No input code was executed; only owned fixtures were.
