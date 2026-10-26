# Static field storage audit

Baseline local 61194e4cd: 521,684 Decompiled / 249,064 Partial of 770,748
bodies (32.31458271704889% fallback). Fixed implementation inventory and exact
reference selections; no redownload, scope change or third-party execution.

audit/manifest.json retains nine production default-readable/ILSpy/original-IL
triples with exact methods, commands, statuses and references. ILSpy succeeds
without unknown-result diagnostics on these methods. Original IL is authority.

| Method | IL obligation and admission decision |
|---|---|
| 0000-060001B3 FSharpList<T>.get_Empty | Read exact closed/open-self generic static reference; no fresh allocation. Preserve one storage per closed declaring type. |
| 0003-060000C5 AnonymousType.GetMethods | Conditional local EmptyList<IMethod>.Instance read; preserve branch and original subtype result, not erased generic storage. |
| 0895-06000026 CSharpOperationFactory.GetIOperationChildren | Foreign constructed value owner's static Empty; field operation needs no receiver layout, but exact field/result/owner arguments need proof. Other value-call/iteration gates remain independent. |
| 1041-06000AFD LongBitSetAllocator<T>.Allocate | Generic static object and int64 reads/write under Monitor/finally. Retain acquired lock object, original read/write timing and overflow exception. |
| 1041-06000468 Platform.get_BinDir | ldsflda exact foreign Path.DirectorySeparatorChar before Char.ToString. Address is original static storage, not a copied char. |
| 1042-060000A6 AIUtils.BotDebug | Foreign Game.Settings static read followed by instance reads and branch. No moving reads past effects or fabricating foreign field facts. |
| 0626-06000007 CFObject.GetStringConstant | IntPtr.Zero foreign static read among native calls. Exact scalar identity/storage and null-result branches; never invoke this production method. |
| 0425-0600025A XmlDataDocument.SetRowValueToNull | DBNull.Value is existing singleton storage; conditional read precedes setter. No new object replacement. |
| 0626-0600416D ServiceCollection<T>..cctor | Exact construction then generic static readonly write. Owning TypeDef alone is insufficient: closed instantiation and original initializer context must match. |

First vertical seam: public decompile/scan and owned actual readable execution
for local ordinary generic static loads/stores, with exact requested/declaration
field signature comparison and caller-scoped simultaneous substitution.
Metadata parsing stays in process; no adapter/mock is introduced. Preserve
negative readonly/foreign/unproved shape gates until their own slices execute.

Retain original field tokens for runtime leaves. Static storage identity,
initialization/acquisition time, beforefieldinit freedom, thread-static storage,
aliases, copied values and early-throw effects are distinct obligations.
Do not infer new members or permissions from an address. A foreign field needs
checked selected-definition facts before using flags or source-member syntax;
unknown flags must not silently authorize readonly stores.
