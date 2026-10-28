# Readable view: member names at call sites

Quality group 2 of `report/quality-full/MERGED.md`. The 24 review slices recorded
about 120 examples of a call site printed as `__cil_<kind>_<token>(...)` where
ILSpy shows `receiver.Member(...)`, `Type.Member(...)` or `new Type(...)`. It is
also the upstream cause of most `vN` temporaries: an unnamed call cannot fold into
its consumer.

## Change

The lift already prints `// call token 0x...: Owner::Member` (and the `newobj`
equivalent) for every call site, so the readable view builds a token-to-member map
from those comments and rewrites the call spelling. No new resolution happens and
nothing is guessed from the IL shape.

Rewritten, because the C# spelling has the same dispatch:

- `callvirt` on a proven reference receiver -> `receiver.Member(args)`
- static calls -> `Owner.Member(args)`
- constructors -> `new Owner(args)`

Deliberately not rewritten:

- `direct` and `memberref_call` are exact non-virtual calls whose virtuality is
  unproven; `receiver.M()` would change dispatch, so they keep the helper.
- `methodspec_call` needs its instantiation spelled out.
- A member whose recorded spelling carries generic arguments or scope variables
  (`OwnerEcho<!0>`) cannot be written as a plain call and keeps the helper.
- An argument list whose receiver is `ref ...` or empty is not a member call.

The exact token stays inline as `/* __cil_<kind>_<token> */`, so a reader can still
audit the call target from the readable view alone, and `--details` keeps the
helper form, the `call token` line and every note unchanged.

## Red then green

`classes::readable_calls_name_members_from_the_evidence_token_and_keep_the_helper_in_details`
failed first with the helper still printed, and passes now. It asserts the named
call, the inline token comment, and that `--details` keeps both the helper and the
`call token` evidence. The token value is asserted by shape, not by a hard-coded
number, because Debug and Release assign different tokens.

Existing assertions that pinned the old helper spelling in the readable view were
kept where they express a real invariant: `virtual_calls`, `reference_relations`,
`external_properties`, `abstract_members`, `exception_types`,
`external_allocations` and `readable_output` still pass unchanged because the token
comment keeps the helper name visible.

## Behaviour

Owned drivers re-run on the release binary, all passing with unchanged counts:

- value calls: 720 comparisons, 80 closed selections
- external base constructors: 12 comparisons per call form
- virtual calls: all receiver mutants rejected
- method instantiation: 125 comparisons, 14 selections, all mutants rejected
- external allocation: 41 comparisons, 13 recovered methods, no skipped selection
- static constructors: 13 state comparisons over 9 recovered types

Rust: 501 tests, `cargo fmt --check` and clippy `-D warnings` pass.

Two test adapters needed updating because they treat helper names as executable
symbols: `virtual-call-roundtrip.py` now strips the inline token comment, and
`external-allocation-roundtrip.py` reads the constructor spelling the readable view
prints while still failing on any remaining unexecutable helper. Neither change
weakens a check; the allocation adapter still refuses a skipped selection.

No input code was executed; only owned fixtures were.

## Follow-up: constructor owner spelling

Reviewing a real NUnit fixture exposed two defects in the first version of this
change, both now fixed and covered:

1. A `newobj` records its member as `.ctor`, which the identifier filter rejected,
   so constructor calls kept the helper form. The filter now accepts `.ctor` for a
   constructor and still rejects a generic-argument member spelling such as
   `OwnerEcho<!0>`.
2. The recorded owner carries metadata notation (`class [mscorlib]...`,
   ``List`1``, `/` for nesting). `short_owner` now drops the `class`/`valuetype`
   prefix, the `[Assembly]` qualifiers and the arity marker, and spells nesting
   with `.`, so the output is `new System.Collections.Generic.List<string>()`
   instead of `new class [mscorlib]System.Collections.Generic.List`1<string>()`.

`classes::readable_constructor_calls_spell_the_owner_without_metadata_qualifiers`
asserts that no `new class `, `new valuetype `, `new [` or `` `1< `` spelling
survives. A whole-image scan of `nunit.testdata.dll` reports the same
802 Decompiled / 12 Partial / 1 Declaration before and after the change, so the
fix is presentation-only.

Rust: 502 tests, `cargo fmt --check` and clippy `-D warnings` pass; the six owned
behaviour drivers pass unchanged.
