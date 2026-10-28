# Readable view: properties, events and backing-field names

Quality group 1 of `report/quality-full/MERGED.md` (84 examples across the 24
review slices). A property was printed as a `get_X`/`set_X` method pair plus a
`// property ...` trailer, and its backing field as `__il_field_<token>` or
`__name_<hex>`.

## What follows ILSpy, and where

ILSpy writes an accessor as a property access whenever the method is an accessor
and its parameter count fits member-access syntax; the **opcode does not decide**
that (`CSharp/CallBuilder.cs`, `HandleAccessorCall` / `IsWrittenAsMemberAccess`).
Only the qualifier depends on dispatch (`CSharp/ExpressionBuilder.cs`,
`RequiresQualifier`, with `TranslateTarget`'s `ShouldUseBaseReference`):

- static member: bare in its own or a containing type, otherwise `Type.`;
- instance member on `this`: bare, except that a **non-virtual** call to an
  **overridable** member outside the current type becomes `base.`;
- otherwise the target expression qualifies it.

`property_presentation` now implements exactly those three rules instead of the
previous "only `call`, only non-virtual, only an immediate base" shape. Three
consequences are deliberate and visible in the diff:

1. A non-overridable base accessor is now written bare (`Enabled`, not
   `base.Enabled`), which is what ILSpy emits and what C# resolves to the same
   member.
2. `callvirt` on a virtual accessor reached through `this` is now admitted and
   written bare, which is exactly the `callvirt` C# re-emits.
3. An interface accessor reached on `this` is still refused: ILSpy would print the
   explicit-implementation name, which is not a valid instance access in C#.

A property is only declared when no body of the same type still calls one of its
accessors as a method; otherwise the accessor methods stay and the exact
relationship comment names the property, so the output never references a member
it does not declare.

Backing fields are spelled `__backing_<Property>` from the
`<Property>k__BackingField` metadata name instead of an opaque token. The prefix
keeps it distinguishable from a real field of the same name.

## Tests

New: `readable_view_renders_properties_and_names_backing_fields` (red first: the
accessor methods and the `// property` trailer were still printed).

Updated because the old spelling was a deliberate placeholder, not a semantic
invariant:

- `backing_field_declarations_and_accessors_use_the_same_token_alias` asserted the
  `__il_field_` alias; it now asserts `__backing_Auto` and that no alias or
  hex-encoded name survives.
- `property_and_event_relationships_show_names_types_and_accessors_not_tokens`
  asserted the relationship trailer for a simple property; it now asserts
  `public int Only { get; }` and `public event EventHandler External;`, and keeps
  the indexer trailer, which needs syntax we do not invent.
- `exact_base_property_relationships...`, `reserved_property_identifiers...`,
  `helper_like_user_member_names...` and
  `substituted_signature_types...` asserted `base.Enabled` / `base.@keyword` for
  non-overridable base members. Under the ILSpy rule those are bare, so the
  assertions now expect the bare spelling; each keeps its original negative.
- `class_fields_...` and `references::direct_instance_calls_and_backing_fields...`
  accepted either spelling and now assert the named backing field.

Rust: 503 tests, `cargo fmt --check` and clippy `-D warnings` pass. The six owned
behaviour drivers (value calls 720/80, virtual calls, method instantiation 125/14,
external allocation 41/13, static constructors 13, external base constructors 12
per form) pass unchanged.

No input code was executed.
