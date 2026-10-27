# decompiler-lib slice: class-level quality comparison

Image: `ILSpy-tests/ICSharpCode.Decompiler/ICSharpCode.Decompiler.dll`
Types: `ICSharpCode.Decompiler.IL.ILInstruction`, `ICSharpCode.Decompiler.CSharp.OutputVisitor.CSharpFormattingOptions`,
`ICSharpCode.Decompiler.CSharp.OutputVisitor.CSharpOutputVisitor`,
`ICSharpCode.Decompiler.CSharp.Syntax.DepthFirstAstVisitor`.
Inputs were read only; neither decompiler nor the input assemblies were executed.

## Headline

| type | ours | ILSpy | ratio |
|---|---|---|---|
| `CSharpFormattingOptions` | 2483 | 369 | **6.71x** |
| `ILInstruction` | 3608 | 1283 | 2.81x |
| `CSharpOutputVisitor` | 6846 | 2525 | 2.71x |
| `DepthFirstAstVisitor` | 745 | 611 | 1.22x |
| total | 13682 | 4788 | 2.86x |

Four orthogonal defects account for almost all of the 2.86x:

1. **Member references that never got named.** 3193 call sites use an opaque token
   helper (`__cil_callvirt_060017AD`, `__cil_newobj_0A0000D8`, `__cil_ldstr_70008716`,
   ...) instead of the member name; 658 distinct helper names.
2. **Loop and branch structure kept as `goto`.** 994 `goto IL_xxxx` and 952
   `IL_xxxx: ;` label lines; ours contains **zero** `while`, `for` and `foreach`
   statements, ILSpy has 37.
3. **Accessors never folded back into properties.** 498 `get_*`/`set_*` method
   shells plus 185 `// property ...` comment lines stand in for 185 properties.
4. **No `is T x` patterns, no `return true/false`, no short type names.** 114
   `as`+`is null` pairs, 818 `__cil_bool_load/store` wrappers, 4045
   namespace-qualified type references and 2011 `global::` qualifiers, against
   **0** `using` directives in any of our four files.

## Category counts

An instance can be counted in two categories when it is both a compression loss
and a missing construct (the 114 `is T x` gaps appear under 2 and 6); the table is
per-category instance counts, not unique lines.

| category | FormattingOptions | OutputVisitor | DepthFirstAstVisitor | ILInstruction | total |
|---|---|---|---|---|---|
| 1. identifier naming | 785 | 4079 | 122 | 977 | **5963** |
| 2. semantic compression | 573 | 2558 | 127 | 1475 | **4733** |
| 3. type/member spelling | 188 | 2439 | 246 | 1172 | **4045** |
| 4. structure and layout | 664 | 105 | 1 | 168 | **938** |
| 5. comments and diagnostics | 259 | 265 | 0 | 76 | **600** |
| 6. missing source constructs | 0 | 69 | 0 | 89 | **158** |

Category 1 (5963): 3193 opaque helper calls (658 distinct names), 888
`__cil_ref_<40-hex>` / `__cil_value_<40-hex>` / `__cil_enum_<token>` aliases used
as declared types, 219 `__il_field_04xxxxxx` fields, 46 `__name_<hex>`
occurrences (14 distinct names), 1413 `v0/v1/...` slot locals and 204 `l0/l1/...`
IL-local mirrors.

Category 2 (4733): 994 `goto IL_`, 952 stray labels, 818
`__cil_bool_load`/`__cil_bool_store` wrappers, 524 `unchecked(...)`, 359 `else`
branches (ILSpy: 0), 496 trailing `return;` in `void` methods, 171 `ref` locals
standing in for `out` parameter assignment, 114 `as`+`is null` tests that ILSpy
writes as `is T x`, 130 `return __cil_bool_store(...)`, 95 `int vN = 1;` and 7
`switch` statements whose every arm is a bare `goto`.

Category 3 (4045): namespace-qualified type and member references in a slice with
**zero** `using` directives (ILSpy's four files have 15); 2011 of them also carry
`global::`. Includes 29 `System.Object.ReferenceEquals` and 6 `System.String.*`.
Positive: `@`-escaping is used correctly (`@operator`, 7 sites, identical to
ILSpy) and the blanket `\uXXXX` string escapes are **not** present in this slice
(1 occurrence, the legitimate `"\u0009"` tab).

Category 4 (938): 498 accessor method shells, 219 unnamed `__il_field_` fields,
214 locals hoisted to the top with `= default;`, 2 field initializers moved into
constructors, 1 explicit `: base()`, 2 nested types emitted after the methods they
serve (ILInstruction: `ChildrenCollection` starts at line 3056 of 3608).

Category 5 (600): 248 `// structural signature ...` lines, 164
`// enum storage token ...` lines, 185 `// property ...` lines, 3
`// IL fallback:` blocks. Counting the per-block fallback detail lines, 725 of all
our output lines (5.3%) are comments; ILSpy has none.

Category 6 (158): 30 `foreach` loops lowered to enumerate/move-next/try/finally,
114 `as`+null tests where ILSpy writes an `is T x` pattern, 2 `yield return`
iterators as full state machines, 1 `?.`, 1 `??`, 6 `++`, 6 `for`, 1 `while`.

## Examples

### 1. Naming: every cross-member call is an unnamed token (category 1, highest count)

`DepthFirstAstVisitor.VisitSyntaxTree` / `VisitErrorNode` ... 121 sites in this
file alone; 110 of them call `VisitChildren` on `this`.

ours (DepthFirstAstVisitor.VisitSyntaxTree):
```csharp
    public virtual void VisitSyntaxTree(ICSharpCode.Decompiler.CSharp.Syntax.SyntaxTree syntaxTree)
    {
        __cil_callvirt_060017AD(this, ((ICSharpCode.Decompiler.CSharp.Syntax.AstNode)(syntaxTree)));
        return;
    }
```
ILSpy:
```csharp
	public virtual void VisitSyntaxTree(SyntaxTree syntaxTree)
	{
		VisitChildren(syntaxTree);
	}
```
Why: the receiver type is known (`this`, `AstNode`, `SyntaxTree`), yet the callee
is printed as its metadata token. Fix: resolve call/newobj/ldstr/box tokens
through the same member-name path already used for fields and direct calls, and
drop the redundant up-cast when the argument type is assignable. Risk: none for
printing; the token->name mapping must fall back to the token form when the
member is out of scope (as today).

### 2. Semantic compression: `is T x` pattern and `return true` folded away

`ILInstruction.MatchLdThis` (1 of 75 `Match*` methods, 80 `as`+`is null` pairs):

ours:
```csharp
        ICSharpCode.Decompiler.IL.LdLoc l0 = default;
        global::ICSharpCode.Decompiler.IL.LdLoc v1 = ((object)this as ICSharpCode.Decompiler.IL.LdLoc);
        l0 = v1;
        global::ICSharpCode.Decompiler.IL.LdLoc v2 = l0;
        if (v2 is null) {
            return false;
        } else {
            goto IL_000A;
        }
        IL_000A: ;
        global::ICSharpCode.Decompiler.IL.LdLoc v3 = l0;
        global::ICSharpCode.Decompiler.IL.ILVariable v4 = __cil_callvirt_06000BA7(v3);
        int v5 = unchecked((int)(__cil_callvirt_06000F31(v4)));
        if (unchecked((uint)v5) != unchecked((uint)4)) {
            return false;
        } else {
            goto IL_0018;
        }
        IL_0018: ;
```
ILSpy:
```csharp
	public bool MatchLdThis()
	{
		if (this is LdLoc ldLoc && ldLoc.Variable.Kind == VariableKind.Parameter)
		{
			return ldLoc.Variable.Index < 0;
		}
		return false;
	}
```
Why: `as`+null-test in one basic block plus a single-predecessor label block is
exactly the `is T x` shape; the following `VariableKind` compare then joins with
`&&`. Fix: introduce one pattern-combining lift (as/is + null branch -> `is T x`,
then `&&` chaining for a successor block), delete the redundant `l0`/`v1` copy and
the byte->int round trip. Risk: medium; must keep evaluation order (the `as` and
the null test are side-effect free, so hoisting the property read behind `&&` is
safe only because short-circuiting matches the original branch target).

### 3. Naming: opaque structural type aliases as declared local types

`CSharpOutputVisitor` constructor (575 `__cil_ref_<hash>` occurrences in the file):

ours:
```csharp
    public CSharpOutputVisitor(System.IO.TextWriter textWriter, ICSharpCode.Decompiler.CSharp.OutputVisitor.CSharpFormattingOptions formattingPolicy)
    {
        // structural signature __cil_ref_25a85dcf25a90ee17c389ecb4157a25ef2a57d30 bytes=151280CD011287F4: class [System]System.Collections.Generic.Stack`1<class ICSharpCode.Decompiler.CSharp.Syntax.AstNode>; ...
        __cil_ref_25a85dcf25a90ee17c389ecb4157a25ef2a57d30 v1 = __cil_newobj_0A0000D8();
        this.containerStack = v1;
        this.isAtStartOfLine = __cil_bool_store(1);
```
ILSpy:
```csharp
	protected readonly Stack<AstNode> containerStack = new Stack<AstNode>();

	protected bool isAtStartOfLine = true;

	public CSharpOutputVisitor(TextWriter textWriter, CSharpFormattingOptions formattingPolicy)
	{
```
Why: the alias is a 40-hex local type name whose meaning is only in the comment
above it. Fix: print the structural type, and emit field initializers at the
field instead of in the constructor. Risk: none (printing only) for the alias;
low for the initializer (constructor-assignment -> initializer is equivalent when
the constructor has no observable code before it).

### 4. Structure: accessors never folded back into properties

`CSharpFormattingOptions` is 2483 lines against ILSpy's 369. 166 properties are
emitted as 498 accessor methods plus a 166-line comment trailer.

ours:
```csharp
    private bool __il_field_04000745;

    public bool get_IndentNamespaceBody()
    {
        return this.__il_field_04000745;
    }

    public void set_IndentNamespaceBody(bool value)
    {
        int v1 = __cil_bool_load(value);
        this.__il_field_04000745 = __cil_bool_store(v1);
        return;
    }
```
and at the end of the type:
```csharp
    // property bool IndentNamespaceBody { get: get_IndentNamespaceBody; set: set_IndentNamespaceBody; }
```
ILSpy:
```csharp
	public bool IndentNamespaceBody { get; set; }
```
Why: the backing field is unnamed, the property is only a comment, and the two
accessors carry a bool normalization wrapper each. Fix: pair `get_X`/`set_X` by
name and signature, emit a property whose accessors nest the existing bodies, and
name the backing field after the property. Risk: low; members are only moved,
never merged or dropped, and non-trivial accessors keep their bodies verbatim.

### 5. Structure: static collection initializers became a 60-statement constructor

`CSharpOutputVisitor` static constructor:

ours:
```csharp
        string v147 = __cil_ldstr_70003ACF("virtual");
        int v148 = __cil_bool_load(__cil_callvirt_0A0003C7(v0, v147));
        string v149 = __cil_ldstr_7000141D("void");
        int v150 = __cil_bool_load(__cil_callvirt_0A0003C7(v0, v149));
        ...
        ICSharpCode.Decompiler.CSharp.OutputVisitor.CSharpOutputVisitor.unconditionalKeywords = v0;
```
ILSpy:
```csharp
	private static readonly HashSet<string> unconditionalKeywords = new HashSet<string>
	{
		"abstract", "as", "base", "bool", "break", "byte", "case", "catch", "char", "checked",
		"class", "const", "continue", "decimal", "default", "delegate", "do", "double", "else", "enum",
		...
	};
```
Why: `newobj HashSet<string>` followed by only `Add(constant)` calls is a
collection initializer; each `Add` return value is dead. Fix: recognize
newobj+Add-chain in a static/instance constructor and hoist to a field
initializer with an initializer block. Risk: low if the field is assigned exactly
once and `Add` is side-effect free from the reader's point of view; otherwise
keep the constructor form.

### 6. Semantic compression: `foreach` lowered to enumerator + try/finally

`CSharpOutputVisitor.WriteCommaSeparatedList` (ours 40 lines, ILSpy 12). The
shape repeats 30 times in the slice.

ours (excerpt):
```csharp
        __cil_ref_9f837d23389b205546fa6d930e797839b9bbe063 v1 = list;
        __cil_ref_2471f4a20a3ecbb626e86f8e66193f5985d6f6a5 v2 = __cil_callvirt_0A00008A(v1);
        l1 = v2;
        try {
            IL_0009: ;
            goto IL_0028;
            IL_000B: ;
            ...
        }
        finally {
            IL_0032: ;
            __cil_ref_2471f4a20a3ecbb626e86f8e66193f5985d6f6a5 v14 = l1;
            if (v14 is null) {
                goto IL_003B;
            } else {
                goto IL_0035;
            }
```
ILSpy:
```csharp
	protected virtual void WriteCommaSeparatedList(IEnumerable<AstNode> list)
	{
		bool flag = true;
		foreach (AstNode item in list)
		{
			if (flag)
			{
				flag = false;
			}
			else
			{
				Comma(item);
			}
			item.AcceptVisitor(this);
		}
	}
```
Why: `callvirt GetEnumerator` / `callvirt MoveNext` / `callvirt get_Current` plus a
`finally` with an `is null` guard around `Dispose` is the structurally guaranteed
foreach shape. Fix: one `foreach` reconstructor over that (type-resolution-only)
shape; keep the explicit try/finally when any part differs. Risk: medium, but the
matcher is narrow and a rejected fixture just keeps today's output.

### 7. Structure: `else` after a jump on every branch

359 `else` branches; ILSpy uses `else` zero times in these four types.

ours (DepthFirstAstVisitor.VisitChildren, 22 lines):
```csharp
        goto IL_0019;
        IL_0009: ;
        global::ICSharpCode.Decompiler.CSharp.Syntax.AstNode v2 = l1;
        global::ICSharpCode.Decompiler.CSharp.Syntax.AstNode v3 = __cil_callvirt_06001703(v2);
        global::ICSharpCode.Decompiler.CSharp.Syntax.AstNode v4 = l1;
        __cil_callvirt_06001723(v4, ((ICSharpCode.Decompiler.CSharp.Syntax.IAstVisitor)(this)));
        l1 = v3;
        IL_0019: ;
        global::ICSharpCode.Decompiler.CSharp.Syntax.AstNode v7 = l1;
        if (v7 is not null) {
            goto IL_0009;
        } else {
            goto IL_001C;
        }
```
ILSpy:
```csharp
	protected virtual void VisitChildren(AstNode node)
	{
		AstNode astNode = node.FirstChild;
		while (astNode != null)
		{
			AstNode nextSibling = astNode.NextSibling;
			astNode.AcceptVisitor(this);
			astNode = nextSibling;
		}
	}
```
Why: a back edge to a condition-only block is a `while`; the `else` only exists
because both arms are gotos. Fix: fold forward/backward gotos into `while`,
`if`/`break` and early `return` before printing; drop the label-only blocks. Risk:
high (needs dominator/post-dominator reasoning); this is the single largest
readability gain but also the only category where a wrong fold changes control
flow, so it belongs after 1-6.

### 8. Semantic compression: `switch` arms that are only `goto`

`CSharpOutputVisitor.OpenBrace`, 1 of 7 switches; every arm jumps to a label block.

ours:
```csharp
        int v0 = unchecked((int)(style));
        switch (v0) {
            case 0: {
                goto IL_0027;
            }
            case 1: {
                goto IL_0027;
            }
            case 2: {
                goto IL_0054;
            }
            ...
            default: {
                goto IL_0022;
            } }
```
ILSpy:
```csharp
		switch (style)
		{
		case BraceStyle.DoNotChange:
		case BraceStyle.EndOfLine:
		case BraceStyle.BannerStyle:
			if (!isAtStartOfLine)
			{
				writer.Space();
			}
			writer.WriteToken(Roles.LBrace, "{");
			break;
```
Why: the case bodies live in out-of-line blocks so the switch conveys no
information; the enum is also converted to `int` first. Fix: resolve enum storage
so the dispatch prints `switch (style)` with enum case labels, and emit a case
body inline when its only predecessor is that case label. Risk: low for the enum
label spelling; medium for inlining (needs the label block to have a single
predecessor).

### 9. Semantic compression: `ref` locals for `out` parameters

`ILInstruction.MatchPinnedRegion` (171 `ref global::... = ref <param>` lines):

ours:
```csharp
        ref global::ICSharpCode.Decompiler.IL.ILVariable v3 = ref variable;
        global::ICSharpCode.Decompiler.IL.PinnedRegion v4 = l0;
        global::ICSharpCode.Decompiler.IL.ILVariable v5 = __cil_callvirt_06000A7F(v4);
        v3 = v5;
        ref global::ICSharpCode.Decompiler.IL.ILInstruction v6 = ref init;
        ...
        int v12 = 1;
        return __cil_bool_store(v12);
```
ILSpy:
```csharp
	public bool MatchPinnedRegion(out ILVariable variable, out ILInstruction init, out ILInstruction body)
	{
		if (this is PinnedRegion pinnedRegion)
		{
			variable = pinnedRegion.Variable;
			init = pinnedRegion.Init;
			body = pinnedRegion.Body;
			return true;
		}
```
Why: `ldarga` + single store is a direct `out` assignment; the `ref` local, the
intermediate value local and the int->bool round trip are all artifacts. Fix:
write `param = value;` when a `ref` local is stored once and never read
otherwise, and write `return true;` for `int vN = 1; return __cil_bool_store(vN);`.
Risk: none for the boolean literal; low for the ref-local collapse (guarded by
"single store, no other read").

### 10. Semantic compression: boolean and enum normalization noise

`ILInstruction.HasFlag`:

ours:
```csharp
    public bool HasFlag(ICSharpCode.Decompiler.IL.InstructionFlags flags)
    {
        // enum storage token=0x02000160 alias=__cil_enum_02000160 underlying=int: global::@ICSharpCode.@Decompiler.@IL.@InstructionFlags; exact input-scoped value type; loads normalize to CLI integer stack bits and stores truncate to the declared width.
        int v1 = unchecked((int)(this.Flags));
        return (unchecked((uint)unchecked(v1 & unchecked((int)(flags)))) > unchecked((uint)0));
    }
```
ILSpy:
```csharp
	public bool HasFlag(InstructionFlags flags)
	{
		return (Flags & flags) != 0;
	}
```
Why: `(x & mask) > 0u` on the unsigned form of a signed mask is `!= 0`; the
`unchecked((int)(...))` pairs exist only to bridge our enum-storage model.
Fix: print `(Flags & flags) != 0` for the masked-unsigned-compare shape and drop
the round-trip casts; keep `unchecked` only where an overflow can really happen.
Risk: none (`x != 0` and `(uint)x > 0` are equivalent for all bit patterns).

### 11. Structure: guard clauses, argument strings and `new` as token calls

`CSharpOutputVisitor` constructor:

ours:
```csharp
        if (textWriter is not null) {
            goto IL_0026;
        } else {
            goto IL_001B;
        }
        IL_001B: ;
        string v6 = __cil_ldstr_70008716("textWriter");
        global::System.ArgumentNullException v7 = __cil_newobj_0A0000C7(v6);
        throw v7;
```
ILSpy:
```csharp
		if (textWriter == null)
		{
			throw new ArgumentNullException("textWriter");
		}
```
Why: inverted null test, label-only blocks, and three helper calls
(`__cil_ldstr_*`, `__cil_newobj_*`) for a literal and an allocation. Fix: emit
string literals directly, `new T(args)` for newobj, and tail `throw` inline in the
negative branch. Risk: none.

### 12. Naming: hex-encoded compiler-generated names in readable positions

`ILInstruction.get_Descendants` and the two explicit interface implementations in
`CSharpOutputVisitor.VisitNullNode`/`VisitErrorNode` (46 sites, 14 distinct):

ours:
```csharp
        global::ICSharpCode.Decompiler.IL.ILInstruction.__name_3C6765745F44657363656E64616E74733E645F5F3839 v1 = __cil_newobj_060024B5(-2);
        v1.__il_field_04000C98 = this;
        return v1;
```
```csharp
    private virtual void __name_49435368617270436F64652E4465636F6D70696C65722E4353686172702E53796E7461782E4941737456697369746F722E56697369744E756C6C4E6F6465(ICSharpCode.Decompiler.CSharp.Syntax.AstNode nullNode)
```
ILSpy:
```csharp
	public IEnumerable<ILInstruction> Descendants
	{
		get
		{
			... yield return this;
```
```csharp
	void IAstVisitor.VisitNullNode(AstNode nullNode)
	{
	}
```
Why: `__name_<hex>` is a hex dump of the metadata name. The explicit interface
form has a real, readable spelling (`IAstVisitor.VisitNullNode`) and the iterator
class name is invisible in C#. Fix: decode the hex when the name is a legal C#
explicit-interface name; for compiler-generated names keep a short readable stub
and hide the state machine behind `yield return` when the shape matches. Risk:
none for the decoded name; the iterator work is the one deliberate-lowering item.

### 13. Type spelling: no `using`, fully qualified everywhere

`CSharpOutputVisitor.VisitIdentifierExpression` (4 lines of ILSpy, 12 of ours):

ours:
```csharp
    public virtual void VisitIdentifierExpression(ICSharpCode.Decompiler.CSharp.Syntax.IdentifierExpression identifierExpression)
    {
        // structural signature __cil_ref_3681fa81e970b76123c0932848c13919b7088424 ...: class [mscorlib]System.Collections.Generic.IEnumerable`1<class ICSharpCode.Decompiler.CSharp.Syntax.AstType>; ...
        __cil_callvirt_060015DB(this, ((ICSharpCode.Decompiler.CSharp.Syntax.AstNode)(identifierExpression)));
        global::ICSharpCode.Decompiler.CSharp.Syntax.Identifier v4 = __cil_callvirt_060019FD(identifierExpression);
        __cil_callvirt_060015E6(this, v4);
        __cil_ref_b4fda3a827877198952aac2f4da907572d4b2618 v7 = __cil_callvirt_060019FF(identifierExpression);
        __cil_callvirt_060015F2(this, ((__cil_ref_3681fa81e970b76123c0932848c13919b7088424)(v7)));
```
ILSpy:
```csharp
	public virtual void VisitIdentifierExpression(IdentifierExpression identifierExpression)
	{
		StartNode(identifierExpression);
		WriteIdentifier(identifierExpression.IdentifierToken);
		WriteTypeArguments(identifierExpression.TypeArguments);
		EndNode(identifierExpression);
	}
```
Why: 4045 namespace-qualified references and 2011 `global::` in a slice with zero
`using` directives; ILSpy's four files have 15. Fix: emit a `using` block from the
referenced namespaces and print short names, keeping qualification only on
collision. Risk: none; collisions are the only correctness concern and are
detectable at print time.

### 14. Comments: evidence lines outnumber the code they describe

`CSharpOutputVisitor.IsKeyword` starts with four diagnostic comment lines and
`CSharpFormattingOptions` repeats the same enum comment for every accessor pair.

ours:
```csharp
    public bool HasFlag(...)
    {
        // enum storage token=0x02000160 alias=__cil_enum_02000160 underlying=int: global::@ICSharpCode.@Decompiler.@IL.@InstructionFlags; exact input-scoped value type; loads normalize to CLI integer stack bits and stores truncate to the declared width.
```
```csharp
        // structural signature __cil_value_9a6ed8a960d15ffa8f9d7bf728b36ba9e718ff74 bytes=118D18: valuetype ICSharpCode.Decompiler.IL.ILInstruction/ChildrenCollection; type-owner=0x00000000 method-owner=0x00000000; tokens scoped to this input image; whole struct value snapshot/copy, preserving embedded references; no boxing, construction, host layout or address inference.
```
ILSpy: no comments at all in these four types.
Why: `// structural signature` (248) and `// enum storage token` (164) are
necessary evidence for the *type model*, not for the reader of a class; they
repeat once per member and hide the code. `// property ...` (185) is a
replacement for a real declaration (see example 4) and should disappear with it.
Fix: move the 412 type-model lines behind `--details` (they are still needed to
audit the lowering), delete the 185 property lines once properties are emitted.
Risk: none; this is a flag, not a transformation.

## Deliberate lowerings (not defects)

- `yield return` iterators: `Descendants` and `Ancestors` are emitted as
  `private sealed class __name_3C6765745F44657363656E64616E74733E645F5F3839`
  state machines (~290 lines for two 12-line properties). ILSpy reconstructs the
  iterator. Keeping the state machine is a *deliberate* limitation and it is
  semantics-preserving, but the class name and the `__il_field_` capture fields
  should still be readable. Priority: low.
- `foreach`/`using`: the 30 lowered enumerate loops preserve `Dispose` on every
  path including exceptions; reconstructing `foreach` is a presentation change
  that needs new analysis (see example 6). Priority: medium.
- Enum backing fields are typed as the enum and only converted at load/store, so
  `unchecked((int)(x))` pairs are a symptom of our model, not a lost operation.

## Partial / fallback blocks (admission gaps, not quality)

`ILInstruction`: 3 blocks, all in the nested `ChildrenCollection` /
`ChildrenEnumerator` types, matching `status: 2` in `index.json`:

| method | reason |
|---|---|
| `ChildrenCollection.__name_...49456E756D657261626C653C...3E2E476574456E756D657261746F72` (`IEnumerable<ILInstruction>.GetEnumerator`) | `// IL fallback: incompatible value/receiver types` |
| `ChildrenCollection.__name_...49456E756D657261626C652E476574456E756D657261746F72` (`IEnumerable.GetEnumerator`) | `// IL fallback: incompatible value/receiver types` |
| `ChildrenEnumerator.MoveNext` | `// IL fallback: EH fault handlers need exceptional-only cleanup` |

`CSharpFormattingOptions`, `CSharpOutputVisitor`, `DepthFirstAstVisitor`: 0
fallback blocks. No `// Dependency unresolved` or generic-body-context comments
appear in this slice.

## Ranked shortlist (readability gain per unit of semantic risk)

1. **Name member references and print literals directly** - kills 3193 opaque
   helper calls (658 distinct names) and enables `new T(...)`, `"literal"`,
   `VisitChildren(x)`, `pinnedRegion.Variable`. Printing only, no new analysis,
   affects all four types.
2. **Short type names + `using` block** - removes up to 4045 qualified
   references and 2011 `global::`; zero analysis, collision check only. Alone
   this shrinks `DepthFirstAstVisitor` from 745 to roughly the ILSpy 611.
3. **Fold accessors back into properties (with backing-field names)** - 498
   method shells and 185 comment lines become 185 declarations;
   `CSharpFormattingOptions` should drop from 2483 to about 500 lines. Members are
   moved, never merged.
4. **Pattern and boolean compression** - `is T x`, `&&` chaining, `return
   true/false`, drop `__cil_bool_load/store` (818) and the masked-compare casts
   (524 `unchecked`). Biggest win on `ILInstruction`'s 75 `Match*` methods; needs
   only the receiver type plus a single-store/single-use local check.
5. **Structured control flow** - fold `goto`/label blocks into `while`, `if`,
   early `return` and `switch` case bodies (994 gotos, 952 labels, 359 `else`,
   7 switches), then `foreach` from the enumerator shape. Largest gain of the
   five, but the only one that can change control flow if a matcher is loose, so
   it should land last and gated on structural proof.
