# Generic member binding repair group

Pre-change default output, ILSpy and original IL are retained under `audit/`.
The manifest records exact selected references and command exit codes. All
ILSpy/IL commands succeeded; our methods remain Partial. Five samples are
ILSPY_YES_OURS_NO, zero BOTH_NO; this is not a whole-bucket gain prediction.

| sample | IL observation |
|---|---|
| FSharpRef.set_Value 0600017A | forwards caller !0 to set_contents on FSharpRef<!0> |
| DiffLib Diff constructor 06000009 | constructed EqualityComparer<!0>.Default, then this constructor delegation |
| test anonymous GetHashCode 06000008 | comparer on caller !0, field read and virtual hash call |
| Decompiler anonymous GetHashCode 06000005 | separate !0 and !1 comparer calls, ordered arithmetic |
| FSharp Choice proxy.get_Item 06000078 | field of another instantiated generic declaring type, then !0 field return |

## Findings and bounded group plan

1. `member_parameter` handles a constructed open-self member's unmodified
   parameter with `Ty::from_param` (empty context). This loses the caller's proven
   type/method variable owner even though the supplied context is available.
   Preserve that context through open-self member parameter and result lowering.
2. A TypeSpec shaped C<!0,...> is not necessarily the caller's own declaring
   C. Existing open-self registration checks only argument positions, conflating
   different generic owners. Definition/arity and the current type-owner must
   agree before collapsing it to Ty::Class. Other instantiations keep the full
   structural identity and substitute their arguments in the caller's scope.
3. Reuse the existing exact call operations and typed signatures. Do not erase
   constructed identity, change dispatch, guess external definitions or relax
   managed addresses. Audit each additional generic call capability before
   implementation; no blind promotion from blocker strings.

## Public validation seams

Owned decompile/scan fixtures first red then green: generic member parameter and
result forwarding, a second generic class instantiated with the caller's
variables, arity mismatch and wrong variable-owner negatives. Executable default
readable bodies must compare generic value/reference copies and receiver state
against the owned original with semantic mutants. Existing generic/reference,
receiver/allocation/interface gates remain in force. Combine related fixes into
one full regression, behavior validation, same-scope corpus scan and static
source/IL self-review milestone. No third-party execution or scope changes.
