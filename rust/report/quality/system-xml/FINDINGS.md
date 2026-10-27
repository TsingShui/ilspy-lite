# Quality comparison: `system-xml` slice

Image: `ILSpy-tests/dotnet/netcore-2.2/System.Private.Xml.dll`
Types: `System.Xml.XmlReader`, `System.Xml.XmlSqlBinaryReader`, `System.Xml.DtdParser`

Inputs were read only. Nothing was executed, no other assembly was decompiled. The only files
written are `FINDINGS.md` and `findings.json` in this directory.

## 1. Headline numbers

The harvested `ratio` (our lines / ILSpy lines) is 5.86 / 3.09 / 4.14, but line counts flatter
us: our output uses K&R (`if (c) {`) while ILSpy uses Allman, so a single ILSpy `if` costs three
lines and ours costs one. The statement count is the honest measure.

| | DtdParser | XmlReader | XmlSqlBinaryReader | Slice total |
|---|---|---|---|---|
| our lines | 32,270 | 7,732 | 12,661 | 52,663 |
| ILSpy lines | 5,504 | 1,866 | 4,092 | 11,462 |
| **line ratio** | **5.86x** | **4.14x** | **3.09x** | **4.60x** |
| our `;` statements | 24,525 | 5,350 | 8,079 | 37,954 |
| ILSpy `;` statements | 1,990 | 545 | 1,362 | 3,897 |
| **statement ratio** | **12.3x** | **9.8x** | **5.9x** | **9.74x** |
| our bytes | 1,717,098 | 402,801 | 554,885 | 2,674,784 |
| ILSpy bytes | 143,186 | 46,562 | 99,518 | 289,266 |
| **byte ratio** | **12.0x** | **8.7x** | **5.6x** | **9.25x** |

Two global facts drive almost everything below.

- **We emit compiler-generated async state machines as nested types.** 61 of them (49 DtdParser,
  12 XmlReader, 0 XmlSqlBinaryReader) occupy 23,387 lines = 44% of our whole output. Excluding
  them the line ratio falls to 2.55x. ILSpy reconstructs 61 `async` methods instead
  (49 + 12 `async` keywords, 161 `await`).
- **We emit one IL basic block per labeled statement group.** 4,336 `IL_xxxx: ;` labels and 6,003
  `goto IL_xxxx` in the slice. 1,739 of 1,834 `if` statements (94.8%) are two-arm goto dispatch,
  and there is not a single `while`, `for`, `do`, `foreach`, `++`, `+=`, `?.` or `??` in the
  output. ILSpy has 108 loops, 4 `foreach`, 264 `++`/`+=`, 41 `?.` and 1 `??` for the same code.

## 2. Category counts

Occurrences are counted on code lines only (comment lines excluded), per site, and each site is
assigned to one category so the table does not double-count.

| # | Category | DtdParser | XmlReader | XmlSqlBinaryReader | Total |
|---|---|---|---|---|---|
| 1 | Identifier naming | 35,563 | 7,397 | 10,165 | **53,125** |
| 2 | Semantic compression | 13,113 | 2,541 | 4,803 | **20,457** |
| 3 | Type and member spelling | 7,635 | 1,669 | 1,850 | **11,154** |
| 4 | Structure and layout | 1,029 | 290 | 293 | **1,612** |
| 5 | Comments and diagnostics | 789 lines | 271 lines | 344 lines | **1,404 lines / 441,297 chars** |
| 6 | Missing source-level constructs | 597 | 144 | 234 | **975** |

Per-category breakdown (sums to the table):

**1 Identifier naming — 53,125.** `v<N>` synthetic locals/temporaries 38,227; reusable-slot locals
spelled `l<N>` 7,479 and exception/branch slots `s<N>_<M>` 863; call targets spelled
`__cil_direct_/callvirt_/call_/newobj_<token>` with the callee's name lost 3,007; compiler field
names `__il_field_<token>` 2,141 (all inside state machines); hex-encoded type and method names
`__name_<hex>` 67 declarations (61 state-machine types, 4 `IDtdParser` explicit
implementations, 2 `IXmlNamespaceResolver` explicit implementations); `get_`/`set_` accessor
names standing in for 157 properties (367 accessor methods); enum member names replaced by
`unchecked((__cil_enum_<token>)(N))` at use sites 974.

**2 Semantic compression — 20,457.** `goto IL_xxxx` 6,003; `IL_xxxx: ;` empty label statements
4,336; `unchecked(...)` 4,949 (plus 24 legitimate `checked(...)`); `this` re-alias temporaries
(`T vN = this;` 1,554 + `ref __cil_value_… vN = ref this;` 1,859) 3,413; `__cil_bool_load(...)`
774 and `__cil_bool_store(...)` 562 = 1,336; assign-then-return 236; redundant trailing `return;`
156; array-initializer expansion via `__cil_newarr_*` 23; `) : base()` on constructors 5.

**3 Type and member spelling — 11,154.** `global::` prefix 4,293; opaque aliases used as declared
types/casts `__cil_value_<sha>` 4,941, `__cil_ref_<sha>` 892, `__cil_type_<token>` 488 = 6,321;
fully-qualified BCL spellings where the keyword/short name applies 540 (`System.Exception` 295,
`System.String` 130, `System.Type` 53, `System.Object` 13, `System.Collections.Generic` 44,
`System.Int32` 4, `System.Boolean` 1). Measured and **not** a problem here: `\uXXXX` escaping
appears only 5 times in 177 `ldstr` payloads, all genuine control characters (`"\u000A"`,
`"\u000D\u000A"`).

**4 Structure and layout — 1,612.** All locals hoisted to the method top as `= default;` 1,598;
4 enums emitted as `private sealed class X : System.Enum` with `public int value__`; 6 explicit
interface implementations carrying `private virtual` modifiers; nested types emitted after all
members in all 3 files; 1 operator emitted as `public static bool op_Equality(...)`.

**5 Comments and diagnostics — 1,404 lines, 441,297 chars (16.5% of our output bytes).**
`// structural signature …` 982 lines / 353,661 chars but only 150 distinct signatures (85%
verbatim duplicates; one QName signature repeats 42 times, one DtdParser signature 64 times);
`// enum storage …` 364 lines / 87,636 chars but only 27 distinct (93% duplicates);
`// property … { get: get_X; }` index 58 lines.

**6 Missing source-level constructs — 975.** 61 state-machine structs instead of `async`/`await`
(deliberate, see §3.10); 367 accessor methods instead of property declarations; 108 loops
(78 `while`, 21 `for`, 9 `do`) lowered to goto; 4 `foreach` lowered; 264 `++`/`+=`-style
compound/self updates lowered to temporaries; 41 `?.` and 1 `??` lowered; 78 `switch` statements
with 857 numeric `case N:` labels and zero named labels; 1 `is T x` pattern expanded; 23 array /
collection initializers expanded.

## 3. Examples, highest impact first

### 3.1 Every same-image call target loses its name (naming, biggest single win)

`XmlReader.ReadContentAsObject` (`System.Xml.XmlReader`, ours 81-88 vs ILSpy 155-162):

```csharp
// ours
string v3 = __cil_ldstr_7000D079("ReadContentAsObject");
global::System.Exception v4 = __cil_direct_06000F29(this, v3);
throw v4;
```
```csharp
// ILSpy
throw CreateReadContentAsException("ReadContentAsObject");
```

The callee is `internal static Exception CreateReadContentAsException(string, XmlNodeType,
IXmlLineInfo)` in the same class. The string literal `"ReadContentAsObject"` and the receiver are
both present; only the member name is gone. 3,007 sites in the slice do this
(`__cil_direct_` 1,139, `__cil_callvirt_` 1,098, `__cil_newobj_` 189, others 581). Calling
conventions are already distinguishable (`direct`/`callvirt`/`newobj`/`ctor_call`), so the
mechanism that decides "name vs token" exists and is applied inconsistently: cross-image members
resolve to names (`System.SR.get_Xml_ReadContentAsFormatException()`,
`System.Xml.XmlReader.HasValueInternal(...)`, `System.String.op_Equality(...)`), same-image ones
usually do not. **Fix:** resolve same-image member names the same way cross-image ones are
resolved; keep the token only as a comment. **Risk:** low - the token already identifies the
member uniquely, so this is display only.

### 3.2 `if`/`else` never guards a block; loops never exist (compression + structure)

`DtdParser.ParseSubset` (`System.Xml.DtdParser`, ours 739-760 vs ILSpy 331-336):

```csharp
// ours
int v5 = unchecked((int)(l1));
int v7 = unchecked(v5 - 11);
switch (v7) {
    case 0: { goto IL_0049; }
    case 1: { goto IL_0054; }
    case 2: { goto IL_005F; }
    ...
    default: { goto IL_003C; } }
IL_003C: ;
int v8 = unchecked((int)(l1));
if (v8 == 32) { goto IL_0126; } else { goto IL_0044; }
IL_0044: ;
goto IL_01E4;
IL_0049: ;
__cil_direct_06001BF8(this);
goto IL_01E4;
```
```csharp
// ILSpy
while (true)
{
    Token token = GetToken(needWhiteSpace: false);
    int currentEntityId = _currentEntityId;
    switch (token)
    {
    case Token.AttlistDecl:
        ParseAttlistDecl();
        break;
    case Token.ElementDecl:
        ParseElementDecl();
        break;
    ...
```

One `while(true)` + `switch` + `case Token.X: …; break;` becomes a flat label soup: the loop head
is `IL_01E4`, every `break` is `goto IL_01E4`, every case is `case N: goto IL_xxxx`, and the arms
are emitted after the dispatch. 94.8% of `if` statements in the slice have `goto` in both arms, so
a reader must simulate the stack machine to recover even a two-branch conditional. **Fix:** order
blocks so a `goto` to the immediately following block becomes fall-through, then fold
`if (c) goto A; else goto B;` into `if (c) { A } else { B }`, then fold the canonical
`Label: if (c) goto Exit; … goto Label;` into `while`/`for`. **Risk:** medium for the loop fold
(it needs the back-edge and exit to be the only edges into the head); low for the fall-through and
`if`/`else` folds, which are pure presentation.

### 3.3 Enums are classes at declaration and anonymous ints at use sites (naming + spelling)

`XmlSqlBinaryReader..ctor` (`System.Xml.XmlSqlBinaryReader`, ours 84-95 / 12112-12125):

```csharp
// ours - declaration
private sealed class ScanState : System.Enum
{
    public int value__;
    public const System.Xml.XmlSqlBinaryReader.ScanState Doc = 0;
    ...
}
// ours - use
global::System.Xml.XmlSqlBinaryReader v33 = this;
int v34 = 5;
v33._state = unchecked((__cil_enum_0200006C)(v34));
```
```csharp
// ILSpy
private enum ScanState { Doc, XmlText, Attr, AttrVal, ... }
_state = ScanState.Init;
```

The enum's members and values are recovered correctly, but the declaration is a `class :
System.Enum` with a public `value__` field (not an `enum`), and **every** use site is
`unchecked((__cil_enum_<token>)(N))`. That costs 974 use sites plus 364 `// enum storage`
comment lines explaining the alias. The same pattern erases switch labels: 78 `switch` statements
produce 857 `case N:` labels and 0 named labels, against ILSpy's 1,016 named cases. **Fix:** emit
`enum` for `System.Enum` subclasses, print uses as `EnumType.Member` using the already-recovered
constant table, and print `case EnumType.Member:`. **Risk:** low - the token and the constant
table are both already resolved; this is printing, not analysis.

### 3.4 `unchecked(...)` noise, including triple-nested casts (compression)

`DtdParser.GetToken` (`System.Xml.DtdParser`, ours 3478 vs ILSpy 3461-3463):

```csharp
// ours
global::System.Xml.DtdParser v2 = this;
__cil_ref_b1a6d254c63137a2f722e310f69dee01c797e9e8 v3 = v2._chars;
global::System.Xml.DtdParser v4 = this;
int v5 = v4._curPos;
int v6 = unchecked((int)(unchecked((ushort)(unchecked((int)(v3[v5]))))));
l0 = unchecked((char)(v6));
```
```csharp
// ILSpy
switch (_chars[_curPos])
{
case '\0':
```

4,949 `unchecked(...)` occurrences, of which 381 are the triple-nested
`unchecked((int)(unchecked((ushort)(unchecked((int)(x)))))` form (char -> int widening printed as
three CLI conversions). The same printer turns char literals into `unchecked((char)(34))`
(`System.Xml.XmlReader.get_QuoteChar`, ours 65 vs ILSpy `'"'`), so our output has **zero** char
literals against ILSpy's 273. **Fix:** collapse conversion chains that are widenings into one
`(char)`/`(int)` spell, and print printable char constants as char literals. **Risk:** low -
each step is a value-preserving CLI conversion on a known-width operand; the reducer must only
collapse when every intermediate conversion is `conv.u2`/`conv.i4`/`conv.u4` widening.

### 3.5 Comment banners bury the first statement (comments)

`IXmlNamespaceResolver.GetNamespacesInScope` (`System.Xml.XmlSqlBinaryReader`, ours 3847-3861):

```csharp
// ours
private virtual System.Collections.Generic.IDictionary<string, string> __name_53797374656D2E586D6C2E49586D6C4E616D6573706163655265736F6C7665722E4765744E616D65737061636573496E53636F7065(System.Xml.XmlNamespaceScope scope)
{
    // structural signature __cil_ref_b758b12e7d24d465b924587007d48b63b400c590 bytes=1D1181B8: valuetype System.Xml.XmlSqlBinaryReader/ElemInfo[]; ...
    // structural signature __cil_ref_73775db26a5f38ea473b02c84831bde5dbb186af bytes=151280D1020E1281C0: class [System.Collections]...Dictionary`2<string, class System.Xml.XmlSqlBinaryReader/NamespaceDecl>; ...
    // structural signature __cil_value_2c7f38b104c9db9ebc1a998fd885c6ab6bbae4ea bytes=1181B8: valuetype System.Xml.XmlSqlBinaryReader/ElemInfo; ...
    // structural signature __cil_ref_2e1a08d1919f285a7d76c39fa6eb309d4f5573c7 bytes=15128151020E0E: ...
    // structural signature __cil_ref_d6e5e9ef9c597b9b43ef419a8a03a2a1efd10719 bytes=151280D1020E0E: ...
    // structural signature __cil_value_6534f25fef1b3b133ef35d116f2ae149b918aa23 bytes=1511814D020E1281C0: ...
    // structural signature __cil_ref_b96c34a4c67f46dc257ee1bf65a223c208c498aa bytes=15128149020E1281C0: ...
    // enum storage token=0x0200006C alias=__cil_enum_0200006C underlying=int: global::@System.@Xml.@XmlSqlBinaryReader.@ScanState; ...
    // enum storage token=0x020000B2 alias=__cil_enum_020000B2 underlying=int: global::@System.@Xml.@XmlNamespaceScope; ...
    System.Xml.IXmlNamespaceResolver l0 = default;
```
```csharp
// ILSpy
IDictionary<string, string> IXmlNamespaceResolver.GetNamespacesInScope(XmlNamespaceScope scope)
{
    if (ScanState.XmlText == _state)
```

Nine comment lines, ~3,400 characters, before the first statement. Across the slice 1,346 such
lines carry 441,297 characters but only 177 distinct facts; 85% of the structural-signature lines
and 93% of the enum-storage lines are verbatim repeats. They also duplicate information that the
declared types will carry once §3.1/§3.3 are fixed. **Recommendation:** keep them as evidence, but
move them to `--details`; emit at most one line per distinct alias per type (or a single
per-type alias table at the top), and never inside a method body. **Risk:** none - they are
comments. This is the single cheapest change: removing them alone drops 441 KB, 16.5% of our bytes.

### 3.6 Struct member writes go through a `ref this` temp (compression + naming)

`QName.Clear` (`System.Xml.XmlSqlBinaryReader.QName`, ours 12172-12188 vs ILSpy 40-42):

```csharp
// ours
string l0 = default;
IL_0000: ;
ref __cil_value_432c10019b924dafd8c4ad80009d2e1c6a150d01 v0 = ref this;
ref __cil_value_432c10019b924dafd8c4ad80009d2e1c6a150d01 v1 = ref this;
ref __cil_value_432c10019b924dafd8c4ad80009d2e1c6a150d01 v2 = ref this;
string v3 = __cil_ldsfld_0A000002();
l0 = v3;
v2.namespaceUri = v3;
string v4 = l0;
l0 = v4;
v1.localname = v4;
string v5 = l0;
v0.prefix = v5;
return;
```
```csharp
// ILSpy
prefix = (localname = (namespaceUri = string.Empty));
```

`QName` is written three times per member access: `ref __cil_value_…`, a `ldsfld` null, a spill to
`l0`, and a redundant `v4 = l0` round trip. The struct's own type is spelled by a 40-hex SHA, so
the local declaration reads `ref __cil_value_432c10… v0 = ref this;`. 1,859 `ref … = ref this;`
and 1,554 `T vN = this;` aliases in the slice. **Fix:** give the struct its metadata name
(`QName`) instead of the SHA alias, drop the `ref` temp when the alias is immediately used once,
and propagate the value into a chained assignment when a single value is stored to several
targets. **Risk:** low for the naming; medium for chained-assignment reconstruction (must keep
store order).

### 3.7 `bool` conditions and `bool` results routed through helper casts

`QName.MatchNs` (ours 12191-12208 vs ILSpy 44-51):

```csharp
// ours
string v0 = lname;
ref __cil_value_432c…  v1 = ref this;
string v2 = v1.localname;
int v3 = __cil_bool_load(System.String.op_Equality(v0, v2));
if (v3 == 0) { goto IL_001B; } else { goto IL_000E; }
IL_000E: ;
string v4 = nsUri;
ref __cil_value_432c… v5 = ref this;
string v6 = v5.namespaceUri;
int v7 = __cil_bool_load(System.String.op_Equality(v4, v6));
return __cil_bool_store(v7);
IL_001B: ;
int v8 = 0;
return __cil_bool_store(v8);
```
```csharp
// ILSpy
if (lname == localname)
{
    return nsUri == namespaceUri;
}
return false;
```

`__cil_bool_load` (774) and `__cil_bool_store` (562) wrap every bool, so no condition reads as a
condition and no `return <bool expr>` reads as one. `System.String.op_Equality(a, b)` (51 sites)
is emitted as a static call rather than `a == b`. `int v8 = 0; return __cil_bool_store(v8);` is
`return false;` (41 sites; there are also 236 assign-then-return sites for non-bool returns).
**Fix:** erase the bool load/store helpers (they are CLI stack normalisations with no C# meaning)
and print `op_Equality`/`op_Inequality` on `System.String`/`System.Uri`/same-type operands as
`==`/`!=`. **Risk:** low - both are exact identity mappings.

### 3.8 Array initialisers expand to one statement per element

`QName.CheckPrefixNS` (ours 12266-12282 vs ILSpy 64-70):

```csharp
// ours
string v8 = System.SR.get_XmlBinary_NoRemapPrefix();
int v9 = 3;
__cil_ref_7bdb48770b79a01877dba09fc617d959d2ebf4a4 v10 = __cil_newarr_01000003(v9);
int v11 = 0;
string v12 = prefix;
v10[v11] = v12;
int v13 = 1;
ref __cil_value_432c… v14 = ref this;
string v15 = v14.namespaceUri;
v10[v13] = v15;
int v16 = 2;
string v17 = namespaceUri;
v10[v16] = v17;
global::System.Xml.XmlException v18 = __cil_newobj_06000A0A(v8, v10);
throw v18;
```
```csharp
// ILSpy
throw new XmlException(SR.XmlBinary_NoRemapPrefix, new string[3] { prefix, this.namespaceUri, namespaceUri });
```

15 statements for 1. `newarr` + consecutive `ldc.i4 index` / `stelem` pairs is the exact
initialiser shape; 23 `__cil_newarr_*` sites in the slice. **Fix:** fold
`newarr n; stelem i0..in` whose indices are the literal 0..n-1 into `new T[n] { … }`, and fold
`newobj` + `throw` into `throw new T(...)` (189 `__cil_newobj_*` sites, all currently split into a
temp plus a separate `throw vN;`). **Risk:** low - the index sequence is structurally guaranteed.

### 3.9 Explicit interface implementations are hex-named and carry wrong modifiers

`IDtdParser.ParseInternalDtd` (`System.Xml.DtdParser`, ours 347-354 vs ILSpy 345-350):

```csharp
// ours
private virtual System.Xml.IDtdInfo __name_53797374656D2E586D6C2E494474645061727365722E5061727365496E7465726E616C447464(System.Xml.IDtdParserAdapter adapter, bool saveInternalSubset)
{
    __cil_direct_06001BE8(this, adapter);
    int v3 = __cil_bool_load(saveInternalSubset);
    __cil_direct_06001BF2(this, __cil_bool_store(v3));
    global::System.Xml.Schema.SchemaInfo v5 = this._schemaInfo;
    return v5;
}
```
```csharp
// ILSpy
IDtdInfo IDtdParser.ParseInternalDtd(IDtdParserAdapter adapter, bool saveInternalSubset)
{
    Initialize(adapter);
    Parse(saveInternalSubset);
    return _schemaInfo;
}
```

The name is the UTF-8 bytes of `System.Xml.IDtdParser.ParseInternalDtd` hex-encoded, so the
interface and member are both already known - only the decoder is missing. `private virtual` is
an IL modifier pair that C# does not spell for an explicit implementation. 6 such methods in the
slice. **Fix:** decode `__name_<hex>` to its metadata name, print
`<Interface>.<Member>`, and suppress `private`/`virtual` on explicit implementations. **Risk:**
none - pure printing.

### 3.10 `async` state machines emitted as hex-named nested structs (deliberate, low priority)

`IDtdParser.ParseInternalDtdAsync` (`System.Xml.DtdParser`, ours 12179-12311 vs ILSpy 3166-3171):

```csharp
// ours
private struct __name_3C53797374656D2D586D6C2D494474645061727365722D5061727365496E7465726E616C4474644173796E633E645F5F313533 : System.Runtime.CompilerServices.IAsyncStateMachine
{
    public int __il_field_04000E7F;
    public System.Runtime.CompilerServices.AsyncTaskMethodBuilder<System.Xml.IDtdInfo> __il_field_04000E80;
    public System.Xml.DtdParser __il_field_04000E81;
    public System.Xml.IDtdParserAdapter adapter;
    public bool saveInternalSubset;
    private System.Runtime.CompilerServices.ConfiguredTaskAwaitable.ConfiguredTaskAwaiter __il_field_04000E84;
    private virtual void MoveNext() { ... 132 more lines ... }
    private virtual void SetStateMachine(...) { ... }
}
```
```csharp
// ILSpy
async Task<IDtdInfo> IDtdParser.ParseInternalDtdAsync(IDtdParserAdapter adapter, bool saveInternalSubset)
{
    Initialize(adapter);
    await ParseAsync(saveInternalSubset).ConfigureAwait(continueOnCapturedContext: false);
    return _schemaInfo;
}
```

Marked **deliberate, low priority**: reconstructing `async`?`await` is a compiler-feature
reconstruction we keep lowered on purpose, and doing it safely needs the whole state-machine
contract (`MoveNext` dispatch, `SetResult`/`SetException`, awaiter `IsCompleted`, hoisted-locals
liveness), not a local pattern. It is nonetheless the dominant *volume* driver: 61 state machines,
23,387 lines, 44% of our output, and it explains why DtdParser is 5.86x while XmlSqlBinaryReader
(no async) is 3.09x. Two sub-items are cheap and are *not* part of the deliberate cost, because
they do not require reconstructing `async`:

- Decode `__name_<hex>` to the readable metadata name
  (`DtdParser.<ParseInternalDtdAsync>d__153`) so the type is findable, and print movenext-only
  fields with their metadata names rather than `__il_field_<token>` (2,141 sites).
- Emit the `async` method's readable `MoveNext` as the method the source declared, i.e. keep the
  body but attach it to a method named like the state machine's declaring method.

**Risk:** naming only, none. Reconstructing `async` itself: high.

### 3.11 Fallback blocks are admission gaps, not quality gaps

23 `// IL fallback:` blocks in 18 methods (DtdParser 5 methods + 4 inside `MoveNext`, XmlReader 4
methods + 1 inside a state machine's `ToString`, XmlSqlBinaryReader 9 methods) = 3.9% of the 463
methods. Reasons: `invalid user-string encoding: terminal flag differs from code units` 10,
`incompatible value/receiver types` 5, `pinned/byref/modified local needs IL fallback` 2,
`conv.u source category needs IL fallback` 2, `unsupported opcode conv.ovf.i4` 2,
`conv.i8 source category needs IL fallback` 1, `expected int[] or uint[] evaluation-stack value` 1.
ILSpy decompiles all of these; e.g. `XmlSqlBinaryReader.ReadContentAsInt` is a 40-line method there
and a byte dump here, and `DtdParser.ParseComment`/`ScanAttlist7`/`ScanPublicId1`/`ScanSystemId`
likewise. The evidence format itself (`.maxstack`, `.code-bytes`, `.local`, per-instruction IL) is
correct to keep; it is already gated behind a banner and does not pollute readable output.

### 3.12 Local declarations are hoisted and named by slot

`DtdParser.ParseSubset` / `XmlReader.ReadContentAsBoolean` (ours 85-87 vs ILSpy 164-178):

```csharp
// ours
bool l0 = default;
System.FormatException l1 = default;
System.FormatException s4_0 = default;
IL_0000: ;
```
```csharp
// ILSpy
try { return XmlConvert.ToBoolean(InternalReadContentAsString()); }
catch (FormatException innerException) { throw new XmlException(..., innerException, this as IXmlLineInfo); }
```

1,598 `= default;` declarations at the top of methods, and `l<N>` names are IL slot numbers, not
the C# local's role (`l0` is the `try` result, `l1`/`s4_0` are two names for the same `catch`
variable). Assign-then-return adds 236 more; `System.FormatException v10 = l1;` then
`__cil_newobj_06000A0E(v8, v9, v10, v12)` re-reads it. **Fix:** declare at first store, name from
the pattern (exception variable, result temp) where the shape is unambiguous, and drop
`vN = vM;` copies where `vM` is dead afterwards. **Risk:** medium - first-store placement needs a
dominance check; the copy elimination needs liveness.

## 4. Deliberate differences, not defects

- `async`/`await` (61 methods, §3.10): deliberate, low priority.
- `checked { }` blocks: ILSpy has 13 `checked` occurrences (6 statement-level `checked { }` blocks
  plus 7 expression-level); we emit per-operation `checked(...)` (24 sites, all in
  XmlSqlBinaryReader). Semantically identical, presentation only, low priority.
- `this.` qualification: we always write `this._field`, ILSpy writes `_field`. Cosmetic; no action.
- `@`-escaped identifiers appear only inside comment text (`global::@System.@Xml.@XmlNodeType`),
  never in emitted code, so this is a comment-cleanup item folded into §3.5.
- `string` escaping: not a problem in this slice (5 of 177 literals; all genuine control chars).

## 5. Partial / fallback blocks

| Type | Fallback methods | Nested fallbacks (inside state machines) | Methods in type | Share |
|---|---|---|---|---|
| `System.Xml.DtdParser` | 5 | 4 | 158 | 3.2% |
| `System.Xml.XmlReader` | 4 | 1 | 166 | 2.4% |
| `System.Xml.XmlSqlBinaryReader` | 9 | 0 | 139 | 6.5% |
| **total** | **18** | **5** | **463** | **3.9%** |

These are admission gaps and are excluded from the category counts above.

## 6. Ranked shortlist: best readability gain per unit of semantic risk

1. **Move the structural-signature and enum-storage banners to `--details`, dedupe per type.**
   1,404 lines / 441,297 chars / 16.5% of our bytes, 177 distinct facts, zero semantic effect.
   Cheapest change in the slice by a wide margin.
2. **Emit enum members and switch labels by name; declare enums as `enum`.** 974 use sites +
   857 `case N:` labels + 4 declarations become readable, and it retires the 364 enum-storage
   comment lines. The constant table is already recovered, so this is printing only.
3. **Resolve same-image call targets and decode `__name_<hex>` / `__il_field_<token>`.** 3,007
   `__cil_*_<token>` call sites, 67 hex names, 2,141 field names. The cross-image path already
   produces names, so no new analysis is needed; the token is uniquely identifying.
4. **Spell opaque type aliases with their metadata names.** 6,321 `__cil_value_<sha>` /
   `__cil_ref_<sha>` / `__cil_type_<token>` occurrences. Every alias already has a decodable
   metadata spelling in its own comment, so this is a lookup, not inference.
5. **Erase the CLI-shaped wrappers: `__cil_bool_load`/`__cil_bool_store` (1,336),
   `unchecked` chain collapsing (4,949, incl. 381 triple nests), char literals (273 lost),
   `op_Equality` -> `==` (51), and `newobj` + `throw` -> `throw new` (189).** All are exact
   identity mappings on operand shapes the IL guarantees.

Deferred as higher-risk-per-unit-gain, in order: fall-through + `if`/`else` folding (6,003
`goto`, 94.8% of `if`s - low risk for the fall-through step, medium for `if`/`else`); loop
reconstruction (108 loops); assign-then-return and dead-copy elimination (needs liveness); local
declaration placement (needs dominance); `foreach` reconstruction (needs the
`GetEnumerator`/`MoveNext`/`Dispose` shape proof, only 4 sites here); `async` reconstruction
(61 methods - highest gain, highest risk, and explicitly deliberate for now).
