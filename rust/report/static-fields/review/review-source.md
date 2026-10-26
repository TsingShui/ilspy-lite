# Static storage: main-session source/IL self-review

Bounded main-session self-review, not independent/formal verification or
third-party execution. manifest.json retains twelve gained-method triples with
production default readable output, ILSpy, original IL, commands, statuses and
exact references. Seven were pre-audited; five add multivariable/static-cache,
foreign value-return and reset paths. All are gains in delta.json. Original IL
is authority; ILSpy output is not an equivalence oracle.

| Method | Checked obligations | Verdict |
|---|---|---|
| 0000-060001B3 FSharpList<T>.get_Empty | One exact original static load under the declaring T, then existing reference returned; no singleton reconstruction or generic erasure. | CRUDE-BUT-EQUIVALENT |
| 0003-060000C5 AnonymousType.GetMethods | Enum bits/branch preserved; static EmptyList<IMethod> read only in ignore-inherited branch. Other branch retains compilation field read and virtual calls/formal operands. | CRUDE-BUT-EQUIVALENT |
| 0895-06000026 CSharpOperationFactory.GetIOperationChildren | ImmutableArray<IOperation>.Empty is a copied value from the exact foreign static field. Default/empty test precedes it; other branch preserves builder acquisition, enumerator location, repeated MoveNext/GetCurrent and final result. | CRUDE-BUT-EQUIVALENT |
| 1041-06000AFD LongBitSetAllocator<T>.Allocate | Original Bits acquired before try/Monitor.Enter; long static reads and shift/store remain distinct; overflow checks the stored value. Finally uses the saved lock object and Boolean acquired flag, including exception path. | CRUDE-BUT-EQUIVALENT |
| 1042-060000A6 AIUtils.BotDebug | Exact foreign Settings static read precedes Debug and BotDebug instance reads. Null/Boolean behavior and conditional call retain order and original argument array identity. | CRUDE-BUT-EQUIVALENT |
| 0626-06000007 CFObject.GetStringConstant | dlsym result is saved before first IntPtr.Zero load. Marshal read precedes second Zero load; both equality branches, null returns and exact later constructor/CIL Boolean operand remain. No native call executed for review. | CRUDE-BUT-EQUIVALENT |
| 0626-0600416D ServiceCollection<T>..cctor | Exact closed nested comparer construction precedes readonly static store to same declaring instantiation. Body stays a CLR initializer, not an ordinary helper method; one-operation store requires original initializer context. | CRUDE-BUT-EQUIVALENT |
| 0000-060017F0 PrintfImpl.Cache<T,TState,TResidue,TResult>.set_last | One exact generic static store of the existing nested tuple reference; all four owner positions retained, with no reconstructed tuple or added read. | CRUDE-BUT-EQUIVALENT |
| 0000-060017F5 PrintfImpl.Cache<...>.get | Dictionary static reference read, key snapshot and getOrAddFunc static reference read occur before original GetOrAdd callvirt. No cached/deferred load crosses the call. | CRUDE-BUT-EQUIVALENT |
| 1041-06000B02 LongBitSetAllocator<T>.Reset | Saved lock object remains separate from subsequent Bits load. Clear precedes int-to-long 1 store; original finally/Monitor.Exit and exceptional behavior remain. | CRUDE-BUT-EQUIVALENT |
| 1042-06000369 CPosGlobal.get_Zero | One foreign CPos.Zero value snapshot and return, preserving embedded identity/bits; static value storage needs no receiver/layout guess. | CRUDE-BUT-EQUIVALENT |
| 0895-0600002D CSharpOperationFactory.CreateIgnoredDimensions | Enum switch comparisons and casts remain checked/original. ImmutableArray<BoundExpression>.Empty is loaded only on empty local-declaration path; copied value join, MethodSpec call and default throw remain ordered. | CRUDE-BUT-EQUIVALENT |

No changed-semantic SUSPECT in this sample. OpenRA.Platform.get_BinDir and
XmlDataDocument.SetRowValueToNull from the audit remain Partial for unavailable
exact external static field facts and are not claimed gains. Fixed reference
selection was not relaxed to recover them. Native calls, production commands
and the rest of the third-party corpus were not executed. Results do not prove
whole-corpus or cross-runtime equivalence.
