# Readable view: real type names and no IR evidence

Quality groups 3 and 4 of `report/quality-full/MERGED.md`.

## Type spelling

Structural and enum aliases (`__cil_ref_<sha>`, `__cil_value_<sha>`,
`__cil_generic_<sha>`, `__cil_type_<token>`) are opaque in the lift, but every one
of them carries an evidence row with the exact metadata type. The readable view now
maps the alias to its C# spelling, which also covers the types the constructor and
static-call owners used to print as metadata notation:

- `class [mscorlib]System.Collections.Generic.List`1<string>` -> `System.Collections.Generic.List<string>`
- `int32[0...,0...]` -> `int[,]`, `int32[0...]` -> `int[]`
- `int32` -> `int`, `float32` -> `float`, `native int` -> `nint`, and the rest of the primitive table
- `Outer/Inner` -> `Outer.Inner`

`class`/`valuetype` prefixes, `[Assembly]` qualifiers and arity markers are dropped.
A whole-word replacement is used, so `int32` cannot corrupt a longer identifier.

Generic scope variables keep their positional identity in a standalone method view
(`!0`, `!!0`): only a view that also prints the declaring type declares the
parameter names, so only that view substitutes them. This is what makes
`--method` output honest about an open scope instead of inventing a name.

## Evidence moved to --details

The readable view no longer prints `// structural signature`, `// generic body
context`, `// enum storage`, or the trailing `// Dependency ...` report: those are
evidence rows and stay in `--details`, which is unchanged. A fallback body still
prints its IL in the readable view, because a body that could not be decompiled
must show what it actually is.

Measured on the NUnit `ActionAttributeFixture` that motivated this work: 73 lines
became 64, `// structural signature` went from 13 to 0, `__cil_ref_` from 13 to 0,
`global::` from 0 to 0 (it had none), and the only remaining helper is the
interface accessor call that C# cannot spell as an instance access.

## Tests

Assertions that pinned the old spelling were corrected or deleted:

- `enums`: three `underlying=sbyte` readable assertions deleted; the storage width
  is evidence and is still asserted where it lives.
- `dependency_output`, `dependencies`, `reference_search`, `property_output`: these
  asserted dependency warnings inside the readable text; they now assert its
  absence there and check the identity in the details view, except the one
  substitution note that no longer exists in either view.
- `readable_output`, `external_allocations`, `exception_types`, `abstract_members`,
  `virtual_calls`, `reference_relations`, `classes`: assertions that named the old
  `__cil_*` helper switched to the new inline note or to the spelled allocation.

Rust: 503 tests, `cargo fmt --check` and clippy `-D warnings` pass.

## Known gaps this exposed

1. A standalone `--method` view of a method whose local type is a declaring-type
   scope variable still prints `!0`; the type view prints the declared name.
2. A general-array local can print its element type (`Pair`) where the array type
   (`Pair[]`) belongs; the `--details` view has the full alias row.

Nine owned behaviour drivers cannot compile their carriers after this change:
`abstract-member`, `constrained`, `exception`, `external-value`,
`interface-reference`, `new-object`, `static-field`, `struct-copy`, `typed-array`.
Six of them already failed before this work; the other three fail because their
carriers are non-generic static classes that flatten methods out of a generic type
or out of a general-array slot. They need the carrier to keep the generic scope, or
an explicit gate; they were not deleted, so the gap stays visible.
