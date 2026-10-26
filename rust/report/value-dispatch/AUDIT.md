# Ordinary constrained dispatch and value initialization audit

Baseline: local commit `7f0a225f4`; 492,717 Decompiled / 278,031 Partial of
770,748 bodies, 36.07287985178035% fallback. Scope/reference policy is unchanged.

`audit/manifest.json` retains production default/ILSpy/original IL triples:

- DiffLib.AlignedDiffChange<T>.GetHashCode (0001-06000007): two distinct T local
  snapshots acquire locations, then constrained T/Object.GetHashCode. The enum
  box and later ordinary call remain separate operations.
- TextTokenWriter.WritePreProcessorDirective (0003-06000070): enum argument slot
  constrained Object.ToString follows output.Write('#'); later lowercasing/output
  and null/empty branches remain ordered.
- DisposableEnumeratorAdapter<T,TEnumerator>.get_Current (0008-06000046): the
  value enumerator field location uses constrained interface get_Current only
  when the separate object enumerator is null. Caller !0 and !1 cannot merge.
- CVec.Minus (1041-06000097): boxing is recovered; the remaining external Lua
  constructor lacks facts. ILSpy explicitly has unknown-result diagnostics, so
  this is INCONCLUSIVE, never BOTH_NO or a promised recovery.

The first three ILSpy invocations succeed; the original constrained prefix and
following callvirt pair is the authority. It must retain both original tokens,
the exact typed managed location, interface/virtual dispatch, boxing fallback,
reference-slot dereference timing, copies, null behavior and argument effects.
It must not be translated to a plain callvirt on a copied/boxed receiver.

Public seams: owned decompile/scan E2E, negative prefix/branch/signature tests,
then execute recovered default bodies with same-state typed receiver cells and
single-operation runtime leaves. Compare receiver mutation, copied values,
derived class dispatch, throwing calls, null reference arguments and slot
replacement during argument evaluation. Mutants must fail semantically.

## Managed-address CFG extension

The first owned constrained red now passes simple calls but Order/Slot expose a
second real gate: Roslyn preserves the receiver reference across argument effects
using a default(T)-boxing test, an explicit reference snapshot for reference T,
and a managed-address stack join. The failing default output and original IL
are retained in `constrained-first-green.log` (an incomplete first-green attempt).

Permit only exact same-element managed locations at stack joins. Lower each edge
with a ref-bound temporary, then parallel ref reassignment of destination scratch
slots. Initialize those scratch refs with `Unsafe.NullRef<T>()`, a pure zero
managed-pointer constant, not a contents read. Existing reachability, stack arity
and same-type predecessor checks ensure each read is preceded by its edge write;
handler entry stacks cannot supply addresses. No original ref/out argument or IL
local slot is rebound, no address escapes via returns/results, and field/array
checks still happen when the original location is acquired. This does not admit
ref-like/unmanaged storage or erase the storage identity into a value snapshot.
Owned same/different-cell branches and slot replacement must execute before gains.

## Local reference slots and nullable result joins

Owned Local exposes a separate red for a typed byref IL local. Unpinned,
unmodified byref locals now have scoped managed-reference scratch slots and
stloc rebinds the exact location rather than assigning its contents. Original
argument slots, byref returns, pinned/custom-modified/unmanaged locals and
nested-address operations remain independently refused. Wrong local element
identity and pointer-kind mutations retain negative coverage.

Owned MaybeText exposes Debug Roslyn null/string result joins. Only Null plus
the same exact reference type merge; this is monotone and propagates successor
reanalysis. There is no common-base guessing, value/reference merge or implicit
boxing. Each edge temporary uses the destination stored identity; a statically
Null stack value emits null, not an object-typed intermediate assignment.
Original Null/return/cast behavior remains in same-state owned execution.

GVM remains gated by its existing runtime lookup-order counterexample. Plain
unprefixed value callvirt is not admitted. Duplicate, separated, cross-block or
otherwise malformed prefixes stay fallback. No third-party body is executed.
