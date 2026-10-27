# Fields of referenced constructed types

A `MemberRef` whose parent is a generic instantiation names a field of that
*instantiation*. The definition's field signature is written with the definition's
own parameters (`!0`), and the site's arguments substitute them; the instance or
static split is not in the MemberRef, and the defining image's `initonly` flag is
not read on this path. Previously the field path required a local definition, so a
field read on `Dictionary<K,V>` or `ValueTuple<A,B>` stopped at
`constructed member needs a local definition`.

## Admission

- When the parent's definition is not local, `Image::field` decodes the MemberRef
  signature, substitutes the TypeSpec's arguments (the same substitution the local
  path already performs), and takes the field type in the caller's scope. The owner
  identity is the instantiation itself: a constructed class stays a structural
  reference, a constructed value type keeps its admitted value identity (so a
  constructed struct read still requires the receiver to be a managed location of
  that exact value type).
- The instance/static split comes from the opcode that names the field
  (`ldfld`/`stfld` versus `ldsfld`/`stsfld`), because it is not encoded in the
  MemberRef. This is exact for code generation: the emitted access has the shape
  the IL asked for.
- Writes stay refused with `readonly field write needs IL fallback`, since the
  defining image's `initonly` flag is not read here. Reads are unaffected.

No definition lookup, layout assumption or member-name search is added.

## Evidence

- Red: owned `ReadForeignValue` and `ReadForeignHolder` were `Partial` on
  `constructed member needs a local definition` (recorded log); both now decompile
  to a field read with the substituted element type (`string` and `int`).
- Negative: `WriteForeignValue` stays `Partial` on
  `readonly field write needs IL fallback`.
- The relation driver executes both reads against the owned library with a
  populated constructed instance and a populated struct instance
  (`Sample` now covers value types): 653 comparisons / 63 closed selections, all
  mutants rejected.
- 483 Rust tests, `cargo fmt --check`, clippy `-D warnings` and the release build
  pass. The float-fixture summary moved from 24/2 to 25/1 because
  `RecoveredDoubleUnbox` is now admitted; the negative was renamed to say so.
- Fixed-corpus batch scan: **690,515 Decompiled / 80,233 Partial of 770,748**,
  fallback **10.409753%**, **+2,529 in 47 images, zero regressions, zero
  incomplete**. OpenRA.Game +34, OpenRA.Mods.Common +256. Gate unmet:
  8,023,300 > 3,853,740.
