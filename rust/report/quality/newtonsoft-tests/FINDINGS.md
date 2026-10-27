# Quality comparison: `newtonsoft-tests` slice

Image: `ILSpy-tests/Newtonsoft.Json-pcl-debug/Newtonsoft.Json.Tests.dll`
Types: `Serialization.JsonSerializerTest`, `Linq.JObjectTests`, `Linq.JsonPath.JPathParseTests`,
`JsonTextReaderTests.ExceptionHandlingTests`.
Every method in both outputs has a body; the admission gaps are listed separately in
[Partial / fallback blocks](#partial--fallback-blocks).

## Where the lines go

| Type | ours lines | ILSpy lines | ratio | ours comment-only lines | ILSpy comment-only |
|---|---|---|---|---|---|
| `JsonSerializerTest` | 14,612 | 5,238 | 2.79 | 3,673 (25.1%) | 2 (0.0%) |
| `JObjectTests` | 4,105 | 1,254 | 3.27 | 376 (9.2%) | 0 (0.0%) |
| `JPathParseTests` | 2,362 | 735 | 3.21 | 139 (5.9%) | 0 (0.0%) |
| `ExceptionHandlingTests` | 5,277 | 1,023 | 5.16 | 3,503 (66.4%) | 0 (0.0%) |
| **slice** | **26,356** | **8,250** | **3.19** | **7,691 (29.2%)** | **2** |

The whole slice is NUnit test code: many small methods, heavy `Assert.AreEqual` use,
heavy compiler-synthesised machinery (lambdas, closures, `dynamic`, `foreach`).
That makes it the most hostile possible input for a one-to-one stack-shaped printer,
and it is why the ratio here (3.19) is far worse than the numbers a plain
business-logic type would give. Two structural facts explain most of the excess:

* **Opaque token-named calls.** `__cil_callvirt_0A00092A(v1)` etc. — 2,978 sites.
  Because a call carries no name, every call needs its own temporary, which is the
  direct cause of 8,203 `vN` declarations.
* **Compiler machinery printed literally instead of reconstructed.** Lambda caches
  (`<>c` / `<>c__DisplayClassN_0`), `dynamic` binder sites, and `foreach`
  enumerator loops account for 112 of the 119 total emitted nested types and most
  of `ExceptionHandlingTests`' 5.16 ratio.

## Category summary

"Occurrences" = concrete sites in our output that a fix would touch.

| # | Category | ExceptionHandling | JObjectTests | JPathParseTests | JsonSerializerTest | Slice |
|---|---|---|---|---|---|---|
| 1 | Identifier naming | 557 | 4,136 | 3,017 | 11,044 | **18,754** |
| 2 | Semantic compression | 231 | 1,316 | 731 | 3,687 | **5,965** |
| 3 | Type & member spelling | 121 | 998 | 482 | 5,239 | **6,840** |
| 4 | Structure & layout | 157 | 102 | 80 | 397 | **736** |
| 5 | Comments & diagnostics (lines) | 3,503 | 376 | 139 | 3,673 | **7,691** |
| 6 | Missing source constructs | 88 | 28 | 17 | 80 | **213** |

Category 1 breakdown (slice totals): opaque token-named calls 2,978 (1,601 `__cil_callvirt_`,
962 `__cil_newobj_`, 415 `__cil_methodspec_call_`); `vN` locals 8,203 declarations
(18,326 references); `lN`/`sN_`/`eN_` locals 890; `__cil_type_` annotations 2,615;
`__cil_ref_`/`__cil_value_` hash aliases 2,819; `__name_<hex>` type names 755
references over 112 declarations; `__il_field_<hex>` fields 494.

Category 2 breakdown: tail `return;` 806; `((object)(...))` boxing wrappers 2,323;
`unchecked(...)` 434; enum-to-int literals `(E)(6)` 666; `? 1 : 0` + `__cil_bool_store`/
`__cil_bool_load` 329; `goto`/`IL_XXXX: ;` jump diamonds 682 (315 + 367); spill/phi
locals 707; `System.String.Concat(...)` instead of `+` 18.

Category 3 breakdown: `global::` prefixes 857; `\u000D`/`\u000A` literal escapes 2,008
(1,004 `\r\n` pairs across 139 lines); fully-qualified `NUnit.Framework.` /
`Newtonsoft.*` / `System.*` references made necessary by emitting **zero** `using`
directives 3,970 (ILSpy emits 53); `@`-escaped identifiers 5.

Category 4 breakdown: missing attributes 610 (`[Test]` 605, `[TestFixture]` 4,
`[Category]` 1); synthetic `: base()` constructors 117; nested types printed after
every method in 4 types; accessors kept as `get_X`/`set_X` methods with a
`// property ...` comment 5.

Category 6 counts only the constructs with a definite site count in our output
(208 lambdas/closures + 5 `foreach` loops). The lower-priority constructs described in
6c-6e add 197 array initializers, 57 `using` declarations, 11 object/anonymous
initializers and 1 `dynamic` site.

---

## 1. Identifier naming (18,754 occurrences) — biggest gap

### 1a. Calls lose their name entirely (2,978 occurrences)

This is the single highest-leverage defect in the slice. The receiver, arguments and
result type are printed, but the method name is replaced by a metadata token, so the
reader cannot tell `Assert.AreEqual` from `JPath..ctor`. The mapping is clearly
available — the same token is resolved to a readable type name elsewhere in the same
line group (`Newtonsoft.Json.Linq.JsonPath.QueryFilter`, `v1.Add`, `v15.TryGetValue`),
so this is an inconsistency in the call printer, not missing information.

`JPathParseTests.SingleProperty`:

```csharp
// ours
string v0 = __cil_ldstr_70005A51("Blah");
__cil_type_01000158 v1 = __cil_newobj_0A000929(v0);
object v3 = ((object)((int)(1)));
__cil_ref_20577db61ee5e5fab706b97ef9c04b2f6070d475 v5 = __cil_callvirt_0A00092A(v1);
int v6 = __cil_callvirt_0A00092B(v5);
```
```csharp
// ILSpy
JPath jPath = new JPath("Blah");
Assert.AreEqual(1, jPath.Filters.Count);
```

`JPathParseTests.RootWithBadWhitespace` — even `ExceptionAssert.Throws` appears as a
token (`__cil_methodspec_call_2B00001F`), and it is the 17-times-repeated shape of
the whole fixture:

```csharp
// ours
__cil_type_010001A4 v8 = __cil_methodspec_call_2B00001F(s2_0, v5);
```
```csharp
// ILSpy
ExceptionAssert.Throws<JsonException>(() => { new JPath("$ .Blah"); },
	new string[1] { "Unexpected character while parsing path:  " });
```

Why it hurts: without call names nothing in the file reads as a statement; the reader
must cross-reference the 750 `// structural signature` comments and the raw IL dumps.
Fix: resolve `call`/`callvirt`/`newobj`/`methodspec` tokens through the same
token-to-name path already used for type casts and the `v1.Add`/`v15.TryGetValue`
cases; print `recv.Name(args)` / `new Type(args)`. Risk: **none** semantically — the
helper is still the same call with the same operands, only the spelling changes.

### 1b. `vN` locals instead of leaves (8,203 declarations, 18,326 references)

`JPathParseTests.BooleanQuery_TwoPaths` vs ILSpy:

```csharp
// ours
__cil_ref_20577... v17 = ((__cil_ref_20577...)(object)v16);
__cil_type_0100015A v24 = __cil_callvirt_0A00092C(v17, 0);
__cil_type_01000213 v25 = ((__cil_type_01000213)(object)v24);
string v26 = ((Newtonsoft.Json.Linq.JsonPath.FieldFilter)v25).Name;
NUnit.Framework.Assert.AreEqual(((object)(v21)), ((object)(v26)));
```
```csharp
// ILSpy
List<PathFilter> list = (List<PathFilter>)booleanQueryExpression.Left;
Assert.AreEqual("price", ((FieldFilter)list[0]).Name);
```

Why it hurts: temporaries exist only because the stack had to be modelled; the source
had 2 locals (`list`, `list2`). Fix: name the temporary after the member it was
loaded from (`left`, `price`, `filters`) and fold load-bound-and-immediately-consumed
temporaries back into the consuming expression. Risk: **low** — same operations in the
same order; only the number of named slots changes. Any folding must not move a load
across a store.

Related but separate: `lN` (183), `sN_0`/`eN_M_0` spill slots (707). The `sN`/`eN`
slots are an artefact of modelling the jump graph as assignments; in
`RootWithBadWhitespace` they exist only to carry `s2_0` through the lambda-cache
diamond (see 2e).

### 1c. `__name_<hex>` compiler type names (755 references, 112 declarations)

`<>c` becomes `__name_3C3E63`; `<>c__DisplayClass0_0` becomes
`__name_3C3E635F5F446973706C6179436C617373305F30`:

```csharp
// ours — ExceptionHandlingTests
private sealed class __name_3C3E635F5F446973706C6179436C617373305F30
{
    // Metadata name: <ReadAsBytes_MissingComma>b__0
    internal void ReadAsBytes_MissingCommaLambda0() { ... }
```
```csharp
// ILSpy — no such type at all; the lambda is inlined into ReadAsBytes_MissingComma
```

The method is renamed to a readable `ReadAsBytes_MissingCommaLambda0` (good), but the
enclosing type keeps a hex-escaped name while the same file already carries a
`// Metadata name:` comment mechanism. Fix: apply the same readable-name rule to the
type (`<>c__DisplayClass0_0` -> `DisplayClass0_0`, `<>c` -> `LambdaCache`). Risk:
**none** — naming only, and the `// Metadata name:` evidence can stay. Note that the
*survival* of the display classes is a deliberate lowering choice; only the hex
spelling is the defect.

### 1d. `__il_field_<hex>` field and `__cil_type_`/`__cil_ref_` annotations (3,109 sites)

```csharp
// ours — JsonSerializerTest.AlmostImmutableStruct
private readonly string __il_field_04000766;
private int __il_field_04000767;
```
```csharp
// ILSpy
public struct AlmostImmutableStruct(string value, int value2)
{
    public string Value { get; } = value;
    public int Value2 { get; set; } = value2;
}
```

`__cil_type_01000023`, `__cil_ref_20577d...`, `__cil_value_eece7...` are type
annotations. The full type name *is* printed — but only inside the
`// structural signature ...` comment above the method. Fix: substitute the type name
into the declaration from that comment, and give fields their metadata names.
Risk: **none** for the annotation; **low** for fields (the `Type`/`Value` property
reconstruction in 4c must land first or the fields need to stay as `m_`-style names).

---

## 2. Semantic compression (5,965 occurrences)

### 2a. Boxing wrappers around every `Assert` argument (2,323)

```csharp
// ours
object v3 = ((object)((int)(1)));
object v7 = ((object)((int)(v6)));
NUnit.Framework.Assert.AreEqual(v3, v7);
int v13 = 0;
object v14 = ((object)(__cil_bool_store(v13)));
```
```csharp
// ILSpy
Assert.AreEqual(1, jPath.Filters.Count);
Assert.AreEqual(false, jObject.TryGetValue("sdf", out var value));
```

The `((object)((int)(x)))` pattern is ILSpy Lite's explicit rendering of `box`. Every
NUnit assertion needs it because NUnit's parameters are `object`, so this single
pattern appears in essentially every test method. Fix: fold `box T` into the enclosing
call when the parameter type is `object` (the conversion is the language's implicit
boxing). Risk: **none** — the box remains, only implicit.

### 2b. Enum members as integers (666)

```csharp
// ours
object v30 = ((object)((Newtonsoft.Json.Linq.JsonPath.QueryOperator)(6)));
int v32 = unchecked((int)(((Newtonsoft.Json.Linq.JsonPath.QueryExpression)v14).Operator));
object v33 = ((object)((Newtonsoft.Json.Linq.JsonPath.QueryOperator)(v32)));
NUnit.Framework.Assert.AreEqual(v30, v33);
```
```csharp
// ILSpy
Assert.AreEqual(QueryOperator.GreaterThan, booleanQueryExpression.Operator);
```

Both directions are wrong: the literal `6` should be the member name, and
`unchecked((int)(enumValue))` should not be printed at all when the enum's underlying
type already is `int`. Fix: resolve the enum constant from metadata (the
`// enum storage token=` comment already identifies the enum type); drop widening
`unchecked` casts where source and target are the same underlying width. Risk: **none**.

### 2c. `unchecked(...)` noise elsewhere (434) and bool round-trips (329)

```csharp
// ours — JObjectTests.KeyValuePairIterate
int v3 = 1;
long v4 = unchecked((long)(v3));
__cil_type_010000B6 v5 = __cil_newobj_0A000730(v4);
```
```csharp
// ILSpy
jObject.Add("PropertyNameValue1", new JValue(1L));
```

Also `int v7 = (v5 == 1 ? 1 : 0); l1 = __cil_bool_store(v7); int v8 = __cil_bool_load(l1);
if (v8 == 0)` where ILSpy has `if (jToken.Type == JTokenType.Object)`. Fix: `? 1 : 0` +
`bool_store`/`bool_load` round-trip should collapse to the comparison itself; the
`unchecked` for a same-width or widening integer conversion should be omitted. Risk:
**low** — dropping `unchecked` is only safe for widening/same-width conversions, which
is what these are.

### 2d. Tail `return;` in every void body (806)

```csharp
// ours
NUnit.Framework.Assert.AreEqual(((object)(v9)), ((object)(v11)));
return;
}
```
```csharp
// ILSpy
Assert.AreEqual("test", listSourceTest2.strprop);
}
```

Fix: omit a trailing `return;` that is the last statement of a void body. Risk: **none**.
Cheapest line-count win in the slice: 806 lines.

### 2e. Jump diamonds instead of `if` (682) and spill locals (707)

`JPathParseTests.RootWithBadWhitespace`, ours (complete body):

```csharp
__cil_type_0100005D v0 = ...JPathParseTests.__name_3C3E63.__il_field_04000E0E;
if (v0 is not null) {
    __cil_type_0100005D e0_2_0 = v0;
    s2_0 = e0_2_0;
    goto IL_0020;
} else {
    __cil_type_0100005D e0_1_0 = v0;
    s1_0 = e0_1_0;
    goto IL_0009;
}
IL_0009: ;
```

Four defects stacked: (a) `if (c) { A; goto L1 } else { A; goto L2 }` where both arms
perform the same assignment and differ only in target label; (b) `IL_0009: ;` an empty
labelled statement; (c) `s1_0`/`e0_1_0` duplicated spill slots; (d) the branch is the
standard `Interlocked`-free lambda-cache idiom `if (cache == null) cache = ...;`.
Fix: fold a diamond whose arms end in `goto` into a nested-if / tail-jump form, and
delete `IL_XXXX: ;` statements that carry only a label (returned by the
`stray_labels` counter as 367 but visible as 367 extra lines). Risk: **low** for (b)
and (c); **medium** for (a) — collapsing a diamond must preserve which arm is taken.

### 2f. `String.Concat` instead of `+` (18)

```csharp
// ours
string v19 = System.String.Concat(((object)(v16)), v18);
```
```csharp
// ILSpy
Assert.AreEqual("PropertyNameValue" + num, item.Name);
```

Low count because the containing methods are mostly `foreach` loops (see 6b).

---

## 3. Type and member spelling (7,090 occurrences)

### 3a. No `using` directives at all — every reference fully qualified (3,970)

Our files begin directly with `namespace`; ILSpy emits 53 `using` lines across the
slice. Consequence:

```csharp
// ours
namespace Newtonsoft.Json.Tests.Linq.JsonPath;

public class JPathParseTests : Newtonsoft.Json.Tests.TestFixtureBase
{
    public void SingleProperty()
    {
        NUnit.Framework.Assert.AreEqual(v3, v7);

// ours — JObjectTests.FooJsonConverter.ReadJson signature + body
public override object ReadJson(Newtonsoft.Json.JsonReader reader, System.Type objectType, ...)
__cil_type_01000007 v8 = System.Type.GetTypeFromHandle(v7);

// ours — ExceptionHandlingTests display-class return type (aliased, not spelled)
__cil_ref_032b9a04b56fba320e3ea99685cb1b602d797648 v2 = v1.ReadAsBytes();
```
```csharp
// ILSpy
using NUnit.Framework;
using Newtonsoft.Json.Linq.JsonPath;
...
public class JPathParseTests : TestFixtureBase
{
	[Test]
	public void SingleProperty()
	{
		Assert.AreEqual(1, jPath.Filters.Count);

// ILSpy
public override object ReadJson(JsonReader reader, Type objectType, ...)

// ILSpy — byte[]
byte[] bytes = Encoding.UTF8.GetBytes("Hello world");
```

`System.Type` -> `Type`, `Newtonsoft.Json.JsonReader` -> `JsonReader`,
`Newtonsoft.Json.Tests.TestFixtureBase` -> `TestFixtureBase`. One caveat for the
shortlist item that substitutes the `__cil_ref_`/`__cil_type_` aliases: the alias
comments spell CLI element types (`uint8[]`, `int32`), so the substitution must map
them to `byte[]`/`int` - today those names appear only in comments, never in code.

Fix: emit the `using` set that ILSpy would (collect the distinct namespaces of the
referenced types, then print the shortest unambiguous name — `System.Int32` -> `int`,
`System.Type` -> `Type`). Risk: **none** — must keep the fully qualified form when two
namespaces hold the same simple name; that check is the only real work.

### 3b. `global::` prefixes (857)

```csharp
// ours — JsonSerializerTest.ListSourceSerialize
global::Newtonsoft.Json.Tests.TestObjects.ListSourceTest v0 = __cil_newobj_06000555();
```
```csharp
// ILSpy
ListSourceTest listSourceTest = new ListSourceTest();
```

`global::` here is not needed for disambiguation (`JsonSerializerTest` imports exactly
one `ListSourceTest`). It is emitted on 735 of the sites in `JsonSerializerTest` alone.

### 3c. `\r` and `\n` printed as `\u000D` / `\u000A` (2,008 escapes over 139 lines)

```csharp
// ours
string v0 = __cil_ldstr_7001CF9F("{\u000D\u000A  \"String\": \"string\",\u000D\u000A  \"Int32\": 2147483647,...");
```
```csharp
// ILSpy
text2 = "{\r\n  \"String\": \"string\",\r\n  \"Int32\": 2147483647,...";
```

Scoped precisely: the escape set in the whole slice is `\u000D` (1,004), `\u000A`
(1,004), `\u0000` (1 — correct, a literal NUL in JSON), and `\\u003c` (1 — correct,
matches ILSpy). So the printer is **not** escaping every character; it escapes CR/LF
only. Consequence: expected-JSON literals — the core payload of a serializer test
suite — are unreadable. Fix: use `\r`, `\n`, `\t`, `\0`, `\"`, `\\` short escapes;
fall back to `\uXXXX` only for other control characters. Risk: **none**, purely
lexical; a round-trip test on the printer proves it.

### 3d. Accessors named as methods (5) — see 4c for the structural half

`public override bool get_CanRead()` on `JObjectTests.FooJsonConverter` where ILSpy
has `public override bool CanRead => false;`.

---

## 4. Structure and layout (505 occurrences)

### 4a. Zero attributes emitted (610 missing)

Ours contains **no** attribute at all. ILSpy: 605 `[Test]`, 4 `[TestFixture]`,
1 `[Category("JsonTextReaderTests")]`.

```csharp
// ours
public class JPathParseTests : Newtonsoft.Json.Tests.TestFixtureBase
{
    public void BooleanQuery_TwoValues()
```
```csharp
// ILSpy
[TestFixture]
public class JPathParseTests : TestFixtureBase
{
	[Test]
	public void BooleanQuery_TwoValues()
```

This is not cosmetic: without `[Test]` the output does not describe a runnable test
class, and 605 methods read as ordinary helpers. Fix: emit custom attributes on types
and members. Risk: **none** — attributes are pure decoration in the printed form.
Second-order: `[Category]` requires rendering the attribute *arguments*, which is
extra machinery; `[Test]`/`[TestFixture]` are attribute-type-only and cover 609 of 610.

### 4b. Synthetic `: base()` constructors (117)

```csharp
// ours — ExceptionHandlingTests
public ExceptionHandlingTests() : base()
{
    return;
}
```
```csharp
// ILSpy — no constructor at all (implicit)
```

112 of these 117 are the constructors of the synthetic closure classes (`__name_*`);
the other 5 are the test/`FooJsonConverter` default constructors, which ILSpy omits
because they are implicit. Fix: omit a constructor whose body is only
`return;` and whose only work is `: base()`. Risk: **none**.

### 4c. Nested types after all methods; properties as accessor pairs (5)

`JsonSerializerTest` prints 365 methods first, then `AlmostImmutableStruct` at line
13,354 (ILSpy prints it at line 42), then 30 synthetic classes. Ours:

```csharp
public string get_Value() { IL_0000: ; ref __cil_value_... v0 = ref this;
    string v1 = v0.__il_field_04000766; return v1; }
public void set_Value2(int value) { ... }
// property string Value { get: get_Value; }
// property int Value2 { get: get_Value2; set: set_Value2; }
```
```csharp
// ILSpy
public struct AlmostImmutableStruct(string value, int value2)
{
	public string Value { get; } = value;
	public int Value2 { get; set; } = value2;
}
```

Two things: property reconstruction is deferred to a trailing comment, and initializers
(`= value`) are missing. Fix: reconstruct `get_`/`set_` pairs into a property (only
`// property ...` comment is emitted today, so the matching already exists — it just is
not applied); order nested types before methods, as ILSpy does. Risk: **low** for the
property (need the accessor pair and the same type); **none** for the ordering.

---

## 5. Comments and diagnostics (7,691 lines, 29.2% of our output)

| Comment kind | count | verdict |
|---|---|---|
| `// IL_XXXX: ...` raw dump lines | 5,046 | **(a)** necessary evidence, must move to `--details` |
| `// structural signature __cil_... bytes=...` | 750 | **(a)** evidence, move to `--details` |
| `// .code-bytes IL_....: [..]` | 679 | **(a)** evidence, `--details` only |
| `// .local [n] ...` | 345 | **(a)** evidence, `--details` only |
| `// enum storage token=0x... alias=...` | 226 | **(a)** evidence, `--details` only |
| `// IL fallback: ...` banners | 130 | **(b)** noise for a *readable* view; the method has no body, so the banner must become a single short marker |
| `// Metadata name: <ReadAsBytes_MissingComma>b__0` | 209 | **keep** — this is the readable name and costs one line |
| `// Dependency unresolved: ...` / `// Dependency identity substituted: ...` | 25 | (a) — 5 per type, pure boilerplate at file end |
| `// property bool CanRead { get: get_CanRead; }` | 4 | keep until 4c lands |

`ExceptionHandlingTests` is 66.4% comments because 74 of its 77 methods are `IL
fallback`, and each fallback dumps `.maxstack`, `.initlocals`, `.code-bytes`,
`.local` and one `// IL_` line per instruction. The structural-signature comment is
genuinely load-bearing today (it is the *only* place the type name of a
`__cil_ref_` alias appears) — so it must stay in the evidence channel but should not
be on by default once 1d is fixed.

---

## 6. Missing source-level constructs (213 sites)

### 6a. Lambdas / closures — 0 emitted vs 208 in ILSpy

Every `ExceptionAssert.Throws<T>(() => {...})` in the fixtures is expanded into a
`<>c`-cache dance in the caller plus a `<>c__DisplayClassN_0` nested class holding the
captured local. `ExceptionHandlingTests` is the extreme case: 77 display classes
(lines 3,651-5,277, 1,631 lines = 31% of the file) and 74 consequent admission
failures. ILSpy inlines all of it. The IL shape is
`ldsfld <>c::<>9__N_M; brtrue; newobj <>c::.ctor; ldftn b__N_M; newobj Action::.ctor;
stsfld` for the static cache, and `newobj <>c__DisplayClassN_0::.ctor; stfld <local>;
ldftn <caller>b__0; newobj Action` for captures.
**Deliberate, low priority**: the brief's rule about compiler features applies — the
lowered form preserves semantics, and reconstructing lambdas needs whole-method
closure analysis. But the *cache boilerplate* alone (6a's diamond in 2e) is 100+
lines that a local `if (cache == null) cache = ...` shape would cut in half.

### 6b. `foreach` — 0 emitted vs 5 in ILSpy

`JObjectTests.KeyValuePairIterate`, ours:

```csharp
__cil_ref_7739... v13 = v12.GetEnumerator();
l2 = v13;
try {
    IL_0037: ;
    goto IL_0080;
    IL_0039: ;
    __cil_ref_7739... v14 = l2;
    __cil_value_0138... v15 = __cil_callvirt_0A0005D4(v14);
```
```csharp
// ILSpy
foreach (KeyValuePair<string, JToken> item in jObject) { ... }
```

IL shape: `call GetEnumerator; try { br; loop: call get_Current / stloc; body; call
MoveNext / brtrue loop } finally { call Dispose }`. **Presentation change needing new
analysis** (must prove `GetEnumerator`/`get_Current`/`MoveNext`/`Dispose` shape or an
`IEnumerable<T>` target). Affects 5 methods here, so low priority for this slice.

### 6c. `using` declarations — 0 vs 57; array initializers — 0 vs 213; object/anonymous initializers — 0 vs 11

```csharp
// ours — JObjectTests.FooJsonConverter.WriteJson
__cil_type_01000026 v1 = __cil_newobj_0A000112();
__cil_type_010000FC v2 = __cil_newobj_0A0003F3();
((Newtonsoft.Json.JsonSerializer)v1).ContractResolver = ((__cil_type_01000103)(v2));
```
```csharp
// ILSpy
JToken jToken = JToken.FromObject(value, new JsonSerializer
{
    ContractResolver = new CamelCasePropertyNamesContractResolver()
});
```

Array initializer case (`17` sites in `JPathParseTests`, `87` in
`ExceptionHandlingTests`, `23` + `86` elsewhere):

```csharp
// ours
__cil_ref_...- v22 = __cil_newarr_010001DC(1);
v22[0] = v25;
```
```csharp
// ILSpy
new string[1] { "Unexpected character while parsing path:  " }
```

Array initializer reconstruction is the cheapest of these: `newarr n; dup; ldc.i4.i;
ld...; stelem; ...` with no intervening control flow. Risk: **low**. Object
initializers and `using` declarations need statements-between-`newobj`-and-store
analysis (`low` and `medium` respectively; `using` also needs the
`Dispose`/`finally` proof).

### 6d. `dynamic` — 0 vs 1 (`JObjectTests.EmbedJValueStringInNewJObject`)

```csharp
// ours — ~90 lines, abridged
__cil_ref_b6f1... v5 = ...JObjectTests.__name_3C3E6F5F5F30.__il_field_04000B1B;
if (v5 is null) { goto IL_001F; } else { goto IL_001D; }
...
__cil_type_010001DF v11 = Microsoft.CSharp.RuntimeBinder.Binder.Convert(
    unchecked((Microsoft.CSharp.RuntimeBinder.CSharpBinderFlags)(0)), v8, v10);
```
```csharp
// ILSpy
dynamic val = JObject.FromObject(new { title = jValue });
string actual = val.ToString();
```

**Deliberate, low priority.** Reconstructing `dynamic` from `CallSite`/`Binder` calls
is a whole separate analysis. But note the anonymous type `new { title = jValue }`
also disappears into `__name_3C3E6F5F5F30` + `<>f__AnonymousType8`'s constructor.
This one method is 1/94 of `JObjectTests` but ~90/4105 of its lines.

### 6e. `out var` declarations, `num++`, `+`

`out var value` (`TryGetValue`) prints as `ref l1` plus a `__cil_methodspec_call_`;
`num++` never appears (`+ 1;` = 0 sites); see 2f for `String.Concat`. All are small
counts here, all `low` risk.

---

## Partial / fallback blocks (admission gaps, not quality gaps)

| Type | methods (incl. ctor) | `IL fallback` bodies | share | dominant cause |
|---|---|---|---|---|
| `ExceptionHandlingTests` | 77 | 74 | 96% | `newobj <>c__DisplayClassN_0` — "external allocation needs dependency facts" |
| `JsonSerializerTest` | 365 | 45 | 12% | same (lambda display classes) + `FileStream`/`StreamReader` allocation |
| `JObjectTests` | 94 | 3 | 3% | `ReadWithSupportMultipleContent`, `GenericValueCast`, `LoadFromNestedObject` |
| `JPathParseTests` | 77 | 0 | 0% | — |
| **slice** | **613** | **122** | **20%** | |

`ExceptionHandlingTests` supplies 3,503 of the slice's 7,691 comment lines, almost all
of it raw IL inside fallback bodies. The quality work above cannot help that type until
the admission gap closes; conversely, closing the display-class admission gap (74 of
122 fallbacks) would be the single biggest line reduction available in the slice.

## Deliberately kept lowered (not defects)

Closures/lambdas (6a), iterator state machines (none in this slice), `dynamic` (6d),
and `foreach` (6b) stay in lowered form; they would each require a new whole-method
analysis. Their *identifier* and *comment* cost is still reported above, because
renaming `__name_3C3E63` -> `LambdaCache` and moving IR dumps behind `--details` is
pure presentation and does not require any of that analysis.

---

## Ranked shortlist — best readability gain per unit of semantic risk

1. **Resolve call/newobj/methodspec tokens to member names** (2,978 sites -> also
   removes most of the 8,203 needless `vN` temporaries, because a named call can nest
   into its consumer). Risk: none. Expected: the largest single ratio improvement in
   the slice; `JPathParseTests` goes from "no statement has a name" to readable.
2. **Emit `using` directives + short type names, drop `global::`** (4,827 sites,
   including `__cil_type_`/`__cil_ref_` aliases -> real names). Risk: none, with one
   ambiguity check. Expected: `System.Type` -> `Type`, `uint8[]` -> `byte[]`,
   `Newtonsoft.Json.Tests.TestFixtureBase` -> `TestFixtureBase`, 3,970 shortened
   references.
3. **Move all IR evidence comments behind `--details`** (`// IL_` 5,046,
   `// structural signature` 750, `.code-bytes` 679, `.local` 345, `enum storage` 226
   = 7,046 lines, 26.7% of output). Risk: none for a readable view; the evidence
   channel keeps them, so no information is lost. Must land after #1 and #2, which are
   what make the comments non-load-bearing.
4. **Printer/compression cleanups: `\r\n` short escapes (2,008), drop tail `return;`
   (806), implicit `box` into `object` parameters (2,323), enum member names instead of
   `(E)(6)` (666), omit same-width `unchecked` (434), `IL_XXXX: ;` deletion (367)** —
   6,604 sites in total, all mechanical and all verifiable by re-running the corpus.
   Risk: none-to-low (only the `unchecked` omission and box-folding need a width /
   parameter-type check).
5. **Emit attributes (`[Test]` 605, `[TestFixture]` 4, `[Category]` 1) and drop
   synthetic `: base()` ctor bodies (117), and promote `// property ...` comments to
   real properties (5).** Risk: none for attributes and constructors; low for
   properties. These are what make the output look like the test class it is.

Not on the shortlist on purpose: reconstructing lambdas (208 sites), `foreach` (5),
`using` declarations (57) and `dynamic` (1) — all need new analysis, and this slice has
too few instances per unit of work. Closing the `<>c__DisplayClass` *admission* gap
(74 fallbacks) is the highest-value work in the slice but belongs to the recovery
audit, not the quality pass.
