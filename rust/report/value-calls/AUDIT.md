# Constructed value calls: pre-implementation audit

Baseline: scoped reference relationships, committed `a5b6d4eef`.
472,156 Decompiled / 298,592 Partial of the unchanged 770,748 bodies;
aggregate fallback 38.74055%. No scope/reference policy or execution changes.

Reuse retained default/ILSpy/original-IL triples, without reacquiring inputs:
`report/reference-relations/audit/manifest.json` and its matching files.

| method | original operation and authority |
|---|---|
| 0004-06000012 SyntaxList.IndexOf | ldarga acquires the exact external SyntaxList<!!0> value location; direct MemberRef substitutes owner arguments and calls IndexOf(int) |
| 1041-06000478 Player.RelationshipWith | generic LongBitSet<PlayerBitMask> field locations feed direct Overlaps calls; early branches and enum result stay ordered |
| 0000-0600018F FSharpResult.NewOk | newobj constructs a generic value, not a fresh reference-class identity |
| 1041-06000097 CVec.Minus | ldobj whole-value copy, unary call, local struct box and reference constructor; ILSpy reports unknown result, so INCONCLUSIVE for its success, not BOTH_NO |

The first three are ILSpy-success/ours-fallback candidates. Boxing is related
downstream work, not a reason to erase the value identity. No bucket is excluded
and no gain is predicted from first-blocker totals.

## Boxing extension audit

Added retained triple `audit/0000-0600006E.*` and `audit/manifest.json`:
FSharpChoice<T1,T2>.GetHashCode (0x0600006E). Original IL boxes the exact
caller-scoped !0 / !1 values after field snapshots, before GenericHashParamObj.
ILSpy succeeds; production baseline falls back at boxing. This is a capability
candidate, not a forecast that its later operations will all recover. The local
CVec.Minus and BlobBuilder value-box leads remain covered by the reused triples.

Boxing must distinguish known value copies/fresh boxes, nullable runtime boxing,
and reference/generic identity conversions. Unbox.any keeps exact runtime type
checks and null behavior, including generic reference/nullable instantiations.
No address is boxed as its contents without an explicit value read.

## Typed seams

Continue the existing typed IL/CFG + lift model. Constructed value receivers
carry the exact existing structural value identity and an actual managed
location, never a copied/boxed receiver. MemberRef signatures substitute owner
arguments simultaneously in the caller scopes. Direct calls stay exact token
operations; no source overload/property inference is required. A plain value
callvirt still needs boxing/constrained dispatch; GVM remains independently gated.

Owned public decompile/scan red/green tests cover local/external constructed value
methods, caller-scoped parameters, ref and value arguments, static calls and
whole-value copies. Support bodies must actually lower. Owned execution compares
reference-bearing values, state, aliasing and writes visible before exceptions,
with meaningful dispatch/copy mutants. Unavailable definitions remain unproved.

Treat value constructors separately from allocation of a reference object.
Preserve original checks, argument evaluation and constructor effects; do not
auto-admit them merely by admitting ordinary value receivers. Ref-like types,
unknown modifiers/layouts and invalid owner/signature shapes keep their gates.
