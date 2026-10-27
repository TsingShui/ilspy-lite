# Quality comparison: `openra-game` slice

Image: `rust/acceptance/openra/assemblies/OpenRA.Game.dll`
Types: `OpenRA.Map`, `OpenRA.World`, `OpenRA.Exts`, `OpenRA.Actor`, `OpenRA.Game`
Inputs: the harvested `.ours.cs` / `.ilspy.cs` pairs and `index.json` in this
directory. No other image was decompiled and no input code was executed.

Ours 20,380 lines vs ILSpy 4,260 lines -- **4.78x**. Per type: Map 5.75x,
World 4.82x, Game 4.41x, Actor 4.18x, Exts 4.06x.

Two caveats that shape every number below:

- **1,777 of our 20,380 lines (8.7%) are compiler-generated closure / display /
  iterator-state-machine classes** that ILSpy does not print at all (see
  category 1/4). Closures and iterators are the "deliberately kept lowered"
  case; the *names* we give them are not.
- **2,919 lines (14%) are `// IL fallback:` raw IL dumps.** Those are an
  admission gap, reported separately at the end, and excluded from the quality
  discussion.

The catalogue below quotes 23 examples rather than 8-15: the brief's
"at least three real examples per category" rule forces 18 or more for six
categories, so the per-category rule is applied and the total is allowed to
exceed the guideline.

## Summary of categories

| # | Category | Occurrences (unit) | Share of the gap |
|---|----------|--------------------|------------------|
| 1 | Identifier naming | 16,463 tokens | Largest single cause of unreadability; ~10.9k synthetic locals plus ~5.5k anonymous type/field/method spellings |
| 2 | Semantic compression | 4,623 rewrite sites | Stack-shaped statements: gotos, labels, `unchecked`, reference temps |
| 3 | Type and member spelling | 8,323 tokens | Zero `using` directives, so every type is fully qualified |
| 4 | Structure and layout | 1,107 sites | Properties/events emitted as accessor methods + a comment block at the end of the type |
| 5 | Comments and diagnostics | 4,089 comment lines (20% of output) | Mostly evidence that belongs behind `--details` |
| 6 | Missing source-level constructs | 199 constructs | `foreach` (79) dominates; all semantics preserved |

Category 1 breakdown (7 signals summing to 16,463):

| signal | count |
|---|---|
| synthetic locals (`v<N>`, `l<N>`, `s<N>_<M>`) -- `index.json` `locals` | 10,929 |
| `__cil_ref_`/`__cil_generic_`/`__cil_value_` hash type aliases | 3,000 |
| `__cil_type_<token>` opaque BCL type aliases (21 distinct) | 403 |
| `__il_field_<token>` anonymous field names (115 distinct) | 378 |
| `__name_<hex>` anonymous type names (44 distinct) | 351 |
| token-numbered call sites (`__cil_callvirt_06000C99`, `__cil_direct_...`, `__cil_newobj_...`, `__cil_methodspec_call_...`, `__cil_ldstr_...`) | 1,390 |
| `arg0`/`arg1` parameters | 12 |

Category 2 breakdown: 1,566 `goto` + 1,400 stray `IL_xxxx: ;` labels + 690
`unchecked(...)` + 377 lambda-cache slot stores + 304 `__cil_bool_load/store`
+ 182 assign-then-return pairs + 52 `get_Item`/`set_Item` + 29
`LongLength`->`nint`->`int` + 20 `System.String.Concat` + 3
`ReferenceEquals(x, null)`.

Category 3 breakdown: 6,618 fully-qualified type tokens (`OpenRA.` 4,267,
`System.` 2,351) + 1,594 `global::` + 83 `__cil_enum_` uses + 13
`__cil_ldc_r4_bits(0x...)` + 6 `@set` + 5 `System.Int32`/`Int64`/`Int16`/`Byte`
+ 3 `unchecked((char)(124))` + 1 `\uXXXX`-escaped `const string`.

---

## 1. Identifier naming (16,463)

### 1.1 Opaque `__cil_type_<token>` aliases for standard BCL types (403, 21 distinct)

The single highest-value fix per unit of risk. `Action` alone is 219 of them.

`OpenRA.Exts.FormatInvariant`:

```csharp
// ours
__cil_type_01000113 v0 = System.Globalization.CultureInfo.get_InvariantCulture();
string v3 = System.String.Format(((__cil_type_010000FF)(v0)), format, args);
return v3;
```
```csharp
// ILSpy
public static string FormatInvariant(this string format, params object[] args)
{
	return string.Format(CultureInfo.InvariantCulture, format, args);
}
```

Frequency table of the aliases actually seen in this slice:

| alias | resolved identity | count |
|---|---|---|
| `__cil_type_01000072` | `System.Action` | 219 |
| `__cil_type_01000131` | `System.Delegate` | 72 |
| `__cil_type_01000026` | `System.Type` | 21 |
| `__cil_type_0100003C` | `System.IO.Stream` | 18 |
| `__cil_type_0100003B` | `System.Text.StringBuilder` (ctor site) | 15 |
| `__cil_type_010000FF` | `System.IFormatProvider` | 12 |
| `__cil_type_01000116` | `System.Globalization.NumberFormatInfo` | 9 |
| `__cil_type_0100008F` | `System.Text.RegularExpressions.Match` | 5 |
| `__cil_type_01000038` | `System.Text.StringBuilder` | 5 |
| `__cil_type_0100008B` | `System.IO.DirectoryInfo` | 1 |
| `__cil_type_010000A4` | `System.Text.Encoding` | 1 |
| 10 further aliases | `Stopwatch`, `IConnection`-adjacent refs, `Predicate`, ... | 24 |

Note that the *member* is named (`System.Text.RegularExpressions.Regex.Match`,
`System.Text.Encoding.get_UTF8`) while its return type is not:

```csharp
// ours, OpenRA.Map
__cil_type_0100008F v13 = System.Text.RegularExpressions.Regex.Match(v11, v12);
// ILSpy
Match match = Regex.Match(...);
```

**Fix:** resolve the TypeRef to a name and rely on `using` (category 3). The
token numbers already present in the output prove the identity is known; only
the printing step drops it.

### 1.2 `__name_<hex>` type names (351 occurrences, 44 distinct)

Hex is the ASCII of the metadata name, so the information is present but
unreadable:

```
__name_3C3E63                                              = <>c
__name_3C3E635F5F446973706C6179436C6173733131375F30        = <>c__DisplayClass117_0
__name_3C497465726174653E645F5F3236                        = <Iterate>d__26
__name_53797374656D2E49446973706F7361626C652E446973706F7365 = System.IDisposable.Dispose
```

The last form is the worst case because it is an *explicit interface
implementation* of a real method on a real type -- it appears on every
iterator's `Dispose`.

**Fix:** print the decoded text (escaped where necessary: `<`, `>`), e.g.
`<>c__DisplayClass117_0` or `__DisplayClass117_0` after stripping the angle
brackets. Zero semantic risk -- this is a printer change.

### 1.3 `__il_field_<token>` field names (378 occurrences, 115 distinct)

Auto-property backing fields and closure captures:

```csharp
// ours, OpenRA.World field list
private OpenRA.Player __il_field_040004C0;
private bool __il_field_040004C2;
private readonly bool __il_field_040004D0;
```
```csharp
// ILSpy
public Player LocalPlayer { get; private set; }
public bool IsGameOver { get; private set; }
public bool RulesContainTemporaryBlocker { get; }
```

Also used for display-class captures that do have readable metadata names
(`mapHeight`, `allowOutsideBounds`, `maxRange` -- Map even emits the real name
`mapHeight` for one closure but `__il_field_04000247` for a real backing
field).

**Fix:** auto-property backing fields should never surface (see 4.1); capture
fields should use their metadata name.

### 1.4 Token-numbered call sites (1,390)

```csharp
// ours, OpenRA.World.SetLocalPlayer
int v9 = __cil_bool_load(this.IsReplay);
// ours, OpenRA.World.EndGame
global::OpenRA.Traits.IGameOver v11 = __cil_callvirt_0A0005A6(v10);
__cil_callvirt_0600097E(v12, v13);
```
```csharp
// ILSpy
if (!IsReplay)
{
	LocalPlayer = localPlayer;
	renderPlayer = LocalPlayer;
}
```
```csharp
foreach (IGameOver item in WorldActor.TraitsImplementing<IGameOver>())
{
	item.GameOver(this);
}
```

`__cil_direct_06000656(v0)` also loses the receiver's identity: the reader
cannot tell that this is `get_IsGameOver()` on `this`.

### 1.5 Synthetic locals (10,929) and `arg0` parameters (12)

Every method declares its slots at the top with `l<N>` / `s<N>_<M>` names and
threads values through `v<N>` temporaries:

```csharp
// ours, OpenRA.Exts.PolygonContains
int l0 = default;
int l1 = default;
__cil_value_4b07... l2 = default;
__cil_value_4b07... l3 = default;
l0 = 0;
l1 = 0;
```
```csharp
// ILSpy
int num = 0;
for (int i = 0; i < polygon.Length; i++)
```

`OpenRA.Exts.WindingDirectionTest(int2 arg0, int2 arg1, int2 p)` vs
`WindingDirectionTest(int2 v0, int2 v1, int2 p)` -- the parameter names are
genuinely absent from metadata (no PDB), so `arg0` is not worse than ILSpy's
`v0`, but mixing the two schemes (`arg0` here, `v0` there) costs
discoverability.

### 1.6 `this.` prefix (297) -- ILSpy emits 7

`public OpenRA.Player get_LocalPlayer() { global::OpenRA.Player v1 = this.__il_field_040004C0; return v1; }`.
Trivial to suppress on member access.

---

## 2. Semantic compression (4,623)

### 2.1 Goto/label control flow (1,566 `goto` + 1,400 `IL_xxxx: ;` labels)

Every branch is `if (c) { goto A; } else { goto B; }` followed by bare labels.
524 `} else {` blocks exist solely to carry a `goto`.

```csharp
// ours, OpenRA.World.ShroudObscures(WPos)
global::OpenRA.Player v1 = this.RenderPlayer;
if (v1 is null) {
    return false;
} else {
    goto IL_0008;
}
IL_0008: ;
global::OpenRA.Player v3 = this.RenderPlayer;
global::OpenRA.Traits.Shroud v4 = v3.Shroud;
int v6 = __cil_bool_load(__cil_callvirt_0600089D(v4, pos));
return (v6 == 0);
```
```csharp
// ILSpy
public bool ShroudObscures(WPos pos)
{
	if (RenderPlayer != null)
	{
		return !RenderPlayer.Shroud.IsExplored(pos);
	}
	return false;
}
```

Semantics are identical; the removal of `goto`/labels is a pure structuring
pass (the CFG already exists -- the labels are its spelling).

### 2.2 `unchecked(...)` noise (690)

`unchecked` is applied to every integer operation regardless of whether the
enclosing context is checked:

```csharp
// ours, OpenRA.Exts.IsPowerOf2
return (unchecked(v & unchecked(v - 1)) == 0);
// ILSpy
return (v & (v - 1)) == 0;
```
```csharp
// ours, OpenRA.Exts.NextPowerOf2
int v0 = v;
v = unchecked(v0 - 1);
int v3 = v; int v4 = v; int v6 = (v4 >> 1);
v = unchecked(v3 | v6);
```
```csharp
// ILSpy
v--;
v |= v >> 1;
```

`C#`'s default is unchecked, and the whole image is compiled without
`/checked`, so the wrapper is provably removable here (and would need one
`checked` block if it were not).

### 2.3 Redundant reference temporaries + assign-then-return (182 + 1,894 `__cil_*` aliases)

```csharp
// ours, OpenRA.World.get_LobbyInfo
global::OpenRA.Network.OrderManager v1 = this.OrderManager;
global::OpenRA.Network.Session v2 = v1.LobbyInfo;
return v2;
```
```csharp
// ILSpy
public Session LobbyInfo => OrderManager.LobbyInfo;
```

`return v<N>;` with `v<N>` assigned on the immediately preceding line occurs
222 times.

### 2.4 Lambda-cache scaffolding (377 slot stores, 263 `s<N>_<M>` slots)

Every non-capturing lambda is re-expanded into the compiler's
`if (cache__N != null) goto; cache__N = new <>c().<M>b__x;` dance:

```csharp
// ours, OpenRA.Exts.GetNamespaces (30 lines total)
__cil_ref_0d9e... s1_0 = default;  __cil_ref_5e37... s1_1 = default;
__cil_ref_0d9e... s2_0 = default;  __cil_ref_5e37... s2_1 = default;
...
__cil_ref_5e37... v2 = OpenRA.Exts.__name_3C3E63.__il_field_04000958;
if (v2 is not null) {
    __cil_ref_0d9e... e0_2_0 = v1;  __cil_ref_5e37... e0_2_1 = v2;
    s2_0 = e0_2_0;  s2_1 = e0_2_1;  goto IL_0025;
} else {
    __cil_ref_0d9e... e0_1_0 = v1;  __cil_ref_5e37... e0_1_1 = v2;
    s1_0 = e0_1_0;  s1_1 = e0_1_1;  goto IL_000E;
}
IL_000E: ;
...
__cil_ref_bc4d... v12 = __cil_methodspec_call_2B00002B(s4_0, s4_1);
return v12;
```
```csharp
// ILSpy
public static IEnumerable<string> GetNamespaces(this Assembly a)
{
	return from n in (from t in a.GetTypes()
			select t.Namespace).Distinct()
		where n != null
		select n;
}
```

The lambda *bodies* do exist and are readable (`// Metadata name:
<GetNamespaces>b__4_0` -> `internal string GetNamespacesLambda4_0(System.Type t)`),
so the cache dance buys nothing.

### 2.5 `__cil_bool_load/store` wrappers (304)

`int v3 = __cil_bool_load(inherit);` then `__cil_bool_store(v3)` at the call
site, where ILSpy writes the `bool` directly
(`mi.GetCustomAttributes(typeof(TAttribute), inherit)`). Same for
`int v1 = 1; x = __cil_bool_store(v1);` for a literal `true`.

### 2.6 Missing `++`, `+=`, compound assignment

0 `++`, 0 `+=`, 0 `for`, 0 `while` in the entire slice; ILSpy has 53 `++`
and 21 `+=`. Example (`Map.ProjectCellInner`): `b++` becomes

```csharp
int v27 = unchecked((int)(l1);
int v29 = unchecked(v27 + 1);
int v30 = unchecked((int)(unchecked((byte)(v29))));
l1 = unchecked((byte)(v30));
```

---

## 3. Type and member spelling (8,323)

### 3.1 No `using` directives; 6,618 fully-qualified type tokens + 1,594 `global::`

Ours emits **0** `using` directives in all five files; ILSpy emits 10-15 per
file. Consequences are visible in every declaration:

```csharp
// ours, OpenRA.World field block
private readonly System.Collections.Generic.List<OpenRA.Effects.IEffect> effects;
internal readonly OpenRA.Network.OrderManager OrderManager;
private readonly System.Collections.Generic.Queue<System.Action<OpenRA.World>> frameEndActions;
```
```csharp
// ILSpy
private readonly List<IEffect> effects;
internal readonly OrderManager OrderManager;
private readonly Queue<Action<World>> frameEndActions;
```

`global::` is prefixed 1,594 times (`global::OpenRA.World v0 = this;
global::OpenRA.Traits.Shroud v4 = v3.Shroud;`) where ILSpy never needs it.

### 3.2 Opaque enum spelling (83 `__cil_enum_` uses)

```csharp
// ours, OpenRA.Game static ctor
OpenRA.Game.lastConnectionState = unchecked((__cil_enum_020001CE)(0));
OpenRA.Game.state = unchecked((__cil_enum_020000BC)(2147483647));
```
```csharp
// ILSpy
private static ConnectionState lastConnectionState = ConnectionState.PreConnecting;
private static RunStatus state = RunStatus.Running;
```

The second line is a genuine **information loss**, not just noise: the source
member `RunStatus.Running` has the value `int.MaxValue` and we print only the
number.

### 3.3 Numeric literal form (13 float bit patterns, 3 char codes)

```csharp
// ours, OpenRA.World ctor
__cil_F v160 = __cil_ldc_r4_bits(0x3F800000u);
__cil_callvirt_06000574(v159, __cil_call_arg_r4(v160));
// ILSpy
Game.Sound.SoundVolumeModifier = 1f;
```
```csharp
// ours, OpenRA.Map.Open
int v2 = __cil_bool_load(filename.Contains(unchecked((char)(124)));
// ILSpy
if (filename.Contains('|'))
```

### 3.4 `@`-escaped identifiers (6) and escaped `const string` (1)

`GetOrAdd<T>(HashSet<T> @set, T value)` -- ILSpy writes `set`; `@` is only
needed when the name is a keyword, which `set` is not in parameter position.

`OpenRA.Game.SavedScreenshot` is the only `const string` in the slice and it
is printed as UTF-16 escapes:

```csharp
// ours
private const string SavedScreenshot = "\u006E\u006F\u0074\u0069\u0066...";
// ILSpy
private const string SavedScreenshot = "notification-saved-screenshot";
```

Scope correction to the brief's hypothesis: the `\uXXXX` printer is **not**
used for `ldstr` literals -- those print literally inside
`__cil_ldstr_70000C98("Collection must not be empty.")` (146 instances of the
wrapper, all with readable text). Only metadata *constants* (`const` fields,
default parameter values) go through the escaping path. 1 instance here.

`System.Int32`/`Int64`/`Int16`/`Byte` appear 5 times where ILSpy writes
`int`/`long`/`short`/`byte`.

---

## 4. Structure and layout (1,107)

### 4.1 Properties and events become accessor methods (122 accessors)

The biggest structural regression in the slice. ILSpy emits 68 properties and
12 events; ours emits them as ordinary methods named `get_X`/`set_X`/`add_X`/
`remove_X`, plus a comment block placed at the **end** of the type body:

```csharp
// ours, OpenRA.World
public void add_RenderPlayerChanged(System.Action<OpenRA.Player> value) { ...42 lines... }
public void remove_RenderPlayerChanged(System.Action<OpenRA.Player> value) { ... }
public OpenRA.Network.Session get_LobbyInfo() { ... }
private void set_OrderGenerator(OpenRA.Orders.IOrderGenerator value) { ... }
... 3,300 lines later, at the end of the class ...
    // property OpenRA.Network.Session LobbyInfo { get: get_LobbyInfo; }
    // property OpenRA.Player RenderPlayer { get: get_RenderPlayer; set: set_RenderPlayer; }
    // event System.Action<OpenRA.Player> RenderPlayerChanged { add: add_RenderPlayerChanged; remove: remove_RenderPlayerChanged; }
```
```csharp
// ILSpy
public Session LobbyInfo => OrderManager.LobbyInfo;

public Player RenderPlayer
{
	get { return renderPlayer; }
	set
	{
		if (LocalPlayer == null || LocalPlayer.UnlockedRenderPlayer)
		{
			renderPlayer = value;
			RenderPlayerChanged?.Invoke(value);
		}
	}
}

public event Action<Player> RenderPlayerChanged;
```

The header comment exists, so the merge is a printing decision, not new
analysis.

### 4.2 All locals declared at the top (761 `= default;` declarations)

```csharp
// ours, OpenRA.Map.ProjectCellInner
OpenRA.Map.__name_3C3E635F5F446973706C6179436C6173733131375F30 l0 = default;
byte l1 = default;
__cil_ref_029ee3187994124535fc787f4a7ea41c9d84547d l2 = default;
```
ILSpy declares at first use (`byte b = mapHeight[uv];`). This also forces the
`l<n>` naming scheme, because a slot reused across scopes cannot be given a
scoped name until it is moved (index metric `locals`: 10,929 vs 13).

### 4.3 Field initializers hoisted into static constructors (2 types, ~48 lines)

`OpenRA.Map` has 20 initializers (including the 20-element `YamlFields` array)
that ILSpy places at the declaration; ours moves all of them into
`static Map()`. `OpenRA.Game` does the same for 15 static fields.

Concrete scale (`Map.YamlFields`):

```csharp
// ours, static Map(): 46 lines
__cil_ref_cde7... v1 = __cil_newarr_02000066(20);
string v3 = __cil_ldstr_70004906("MapFormat");
global::OpenRA.MapField v5 = __cil_newobj_060002E6(v3, null, 1, null);
v1[0] = v5;
... x20 ...
OpenRA.Map.YamlFields = v1;
```
```csharp
// ILSpy: 24 lines, self-documenting
private static readonly MapField[] YamlFields = new MapField[20]
{
	new MapField("MapFormat"),
	...
	new MapField("LockPreview", null, required: false, "False"),
	...
};
```

Also note `__cil_newobj_060002E6(v3, null, 1, null)` fills the omitted
optional arguments with raw `null`/`1`; ILSpy elides them and names the
`required:` argument where it is explicit.

### 4.4 Compiler-generated nested types are printed (1,777 lines)

`Actor` 610, `Exts` 253, `Game` 274, `Map` 510, `World` 130 lines of
`<>c`, `<>c__DisplayClass*`, `<X>d__*` state machines, each with its own
fields, `.ctor`, `MoveNext`, explicit `IEnumerator`/`IDisposable`
implementations. ILSpy suppresses all of them. This is the "deliberately
lowered" case for the *bodies* (see 6.5), but for `<>c`/display classes the
suppression is free: their entire content is already reachable from the
enclosing method.

### 4.5 Enum emitted as a sealed `System.Enum` class (1)

```csharp
// ours, OpenRA.Exts
public sealed class ISqrtRoundMode : System.Enum
{
    public int value__;
    public const OpenRA.Exts.ISqrtRoundMode Floor = 0;
    public const OpenRA.Exts.ISqrtRoundMode Nearest = 1;
    public const OpenRA.Exts.ISqrtRoundMode Ceiling = 2;
}
```
```csharp
// ILSpy
public enum ISqrtRoundMode
{
	Floor,
	Nearest,
	Ceiling
}
```

Consequence for callers: `ISqrt(int number, ISqrtRoundMode round = 0)` instead
of `= ISqrtRoundMode.Floor` (4 signatures).

### 4.6 `volatile` modifier dropped (1)

`private static volatile ActionQueue delayedActions` becomes a plain field plus
`System.Threading.Volatile.Write(ref OpenRA.Game.delayedActions, v27);` in the
static constructor. ILSpy keeps `volatile` and the initializer.

### 4.7 Attributes dropped (1 vs 1)

`[FluentReference(new string[] { "filename" })]` on `Game.SavedScreenshot`
appears in ILSpy and not in ours; no `[` appears on any of our declaration
lines across the slice.

Field/property/constructor ordering otherwise matches metadata order, so the
only ordering problem is the trailing property/event comment block (4.1).

---

## 5. Comments and diagnostics (4,089 comment lines = 20% of our output)

| kind | count | verdict |
|---|---|---|
| `// structural signature __cil_ref_<hash> bytes=...: ...; tokens scoped to this input image; exact reference identity, ...` | 687 | **(a)** necessary evidence, move to `--details` |
| `// generic body context type-owner=... type-arity=... method-owner=... method-arity=...` | 59 | **(a)** move to `--details` |
| `// enum storage token=... alias=... underlying=...` | 51 | **(a)** move to `--details` |
| `// Metadata name: <GetNamespaces>b__4_0` | 63 | **(b)** keep -- this is the *only* readable link to the real name; better: print the decoded name directly |
| `// Exact call 0x06000301 kept: property source signature requires exact type identities.` | 8 | **(a)** move to `--details` |
| `// Dependency identity substituted: System.Runtime, ... -> ...` (trailer) | 170 | **(b)** pure noise in the readable view; keep behind `--details` |
| `// Dependencies unresolved: 7; ...` (trailer) | 5 | **(b)** keep one line, drop the rest |
| `// property ...` / `// event ...` type-footer block | 80 | **(b)** delete once 4.1 lands |
| `// IL fallback:` dumps (see below) | 2,919 | admission gap, not quality |

A representative worst case -- five comment lines before the first statement of
`Exts.GetNamespaces`, each 200-350 characters, naming nothing a reader can use:

```csharp
// structural signature __cil_ref_bc4d36fe1c82f28064fcc35653d69d1d30dd7cbd bytes=15128085010E: class [System.Runtime]System.Collections.Generic.IEnumerable`1<string>; type-owner=0x00000000 method-owner=0x00000000; tokens scoped to this input image; exact reference identity, no member/layout inference.
```

The evidence itself is sound and should stay available; it just belongs behind
`--details`, not in the readable view.

---

## 6. Missing source-level constructs (199)

All of these preserve semantics in our output; each requires the indicated
amount of analysis to recover.

### 6.1 `foreach` (79 occurrences, 0 in ours) -- highest impact, proven IL shape

Every `foreach` is emitted as `GetEnumerator` / `MoveNext` / `get_Current` /
`Dispose` calls plus a `try/finally`: there are 69 `try`/`finally` blocks in
the slice and 37 of them are enumeration (`MoveNext` call shape).

```csharp
// ours, OpenRA.World.EndGame
__cil_ref_84ce... v8 = __cil_callvirt_2B00014D(v7);
__cil_ref_ac0f... v9 = __cil_callvirt_0A0005A5(v8);
l0 = v9;
try {
    IL_0027: ; goto IL_0037;
    IL_0029: ;
    __cil_ref_ac0f... v10 = l0;
    global::OpenRA.Traits.IGameOver v11 = __cil_callvirt_0A0005A6(v10);
    l1 = v11;
    __cil_callvirt_0600097E(v12, v13);
    IL_0037: ;
    int v15 = __cil_bool_load(__cil_callvirt_0A000018(v14));
    if (v15 != 0) { goto IL_0029; } else { goto IL_003F; }
} finally { ... if (v16 is null) goto IL_004A; __cil_callvirt_0A000017(v17); ... }
```
```csharp
// ILSpy
foreach (IGameOver item in WorldActor.TraitsImplementing<IGameOver>())
{
	item.GameOver(this);
}
```

The shape is structurally guaranteed (`IEnumerable<T>.GetEnumerator` on the
receiver, `IEnumerator<T>`/`IEnumerator` local, `MoveNext` in the loop test,
`get_Current` at the loop head, `Dispose` in the finally guard), so this is a
pattern match, not a heuristic -- but it is a new analysis and needs its own
regression fixtures.

### 6.2 `for` / `while` (27 / 12 occurrences, 0 in ours)

```csharp
// ours, OpenRA.Exts.MakeArray
l1 = 0;
goto IL_001D;
IL_000B: ;
__cil_ref_f962... v3 = l0;
int v4 = l1; int v6 = l1;
__cil_generic_a04f... v7 = __cil_callvirt_0A000182(f, v6);
__cil_stelem_1B0000BA(v3, v4, v7);
int v8 = l1;
l1 = unchecked(v8 + 1);
IL_001D: ;
int v11 = l1;
if (v11 < count) { goto IL_000B; } else { goto IL_0021; }
```
```csharp
// ILSpy
public static T[] MakeArray<T>(int count, Func<int, T> f)
{
	T[] array = new T[count];
	for (int i = 0; i < count; i++)
	{
		array[i] = f(i);
	}
	return array;
}
```

### 6.3 `using` (21), object/collection initializers (8), array initializers (9)

```csharp
// ours, OpenRA.World ctor
string v38 = __cil_ldstr_70007480("PrepareMap");
__cil_F v39 = __cil_ldc_r4_bits(0x00000000u);
global::OpenRA.Support.PerfTimer v40 = __cil_newobj_060007B3(v38, v39);
l4 = v40;
try {
    IL_00F4: ;
    global::OpenRA.World v41 = this;
    ...
    v41.Map = v44;
    goto IL_010F;
} finally { ... __cil_callvirt_0A000017(v46); ... }
```
```csharp
// ILSpy
using (new PerfTimer("PrepareMap"))
{
	Map = modData.PrepareMap(mapUID);
}
```

```csharp
// ours, same ctor
global::OpenRA.GameInformation v162 = __cil_newobj_0600019D();
... v162.Mod = v165; v162.Version = v169; v162.MapUid = v172; v162.MapTitle = v175;
v161.gameInfo = v162;
```
```csharp
// ILSpy
gameInfo = new GameInformation
{
	Mod = Game.ModData.Manifest.Id,
	Version = Game.ModData.Manifest.Metadata.Version,
	MapUid = Map.Uid,
	MapTitle = Map.Title
};
```

### 6.4 `?.` (30) and `??` (6)

```csharp
// ours, OpenRA.Game.get_IsHost (14 lines)
OpenRA.Network.Session.Client l1 = default;
global::OpenRA.Network.OrderManager v0 = OpenRA.Game.OrderManager;
global::OpenRA.Network.IConnection v1 = __cil_callvirt_06000C99(v0);
int v2 = __cil_callvirt_06000C2B(v1);
... global::OpenRA.Network.Session.Client v6 = __cil_callvirt_06000CC2(v4, v2);
l1 = v6;
global::OpenRA.Network.Session.Client v7 = l1;
if (v7 is null) { return false; } else { goto IL_0024; }
IL_0024: ;
global::OpenRA.Network.Session.Client v8 = l1;
return v8.IsAdmin;
```
```csharp
// ILSpy
public static bool IsHost
{
	get
	{
		int localClientId = OrderManager.Connection.LocalClientId;
		return OrderManager.LobbyInfo.ClientWithIndex(localClientId)?.IsAdmin ?? false;
	}
}
```

```csharp
// ours, OpenRA.Actor.get_Crushables
__cil_ref_ad5e... s1_0 = default;
__cil_ref_ad5e... s2_0 = default;
__cil_ref_ad5e... v1 = this.crushables;
if (v1 is not null) { ... s2_0 = e0_2_0; goto IL_002A; }
else { ... s1_0 = e0_1_0; goto IL_0009; }
IL_0009: ;
string v2 = __cil_ldstr_700000E8("Crushables for ");
... string v7 = System.String.Concat(v2, v5, v6);
throw __cil_newobj_0A00006E(v7);
IL_002A: ;
return s2_0;
```
```csharp
// ILSpy
public ICrushable[] Crushables => crushables ?? throw new InvalidOperationException("Crushables for " + Info.Name + " are not initialized.");
```

### 6.5 `yield return` (4) -- deliberate, low priority

`Exts.Iterate`, `Actor.Bounds`, `Actor.Renderables` are emitted as their
`<X>d__N` state machines (`MoveNext` with `this.<>1__state` switch, `Dispose`,
`IEnumerator` bridge methods). ILSpy reconstructs `yield return`.

Marked **deliberate**: iterator reconstruction is the async/closure class of
work the brief excludes, and the lowered form preserves the state machine
exactly. The visible cost is ~600 lines of `Actor` + `Exts` tail; the readable
fix that carries no analysis risk is suppressing the whole generated type (4.4)
and letting the enclosing method be `return new <X>d__26(...)`, which is what
it already says.

### 6.6 `is T name` pattern (3)

```csharp
// ours, OpenRA.Exts.IsTraitEnabled
object v1 = ((object)((__cil_generic_a61d...)(trait)));
global::OpenRA.Traits.IDisabledTrait v2 = ((object)v1 as OpenRA.Traits.IDisabledTrait);
l0 = v2;
... if (v3 is null) { return true; } else { goto IL_000F; }
```
```csharp
// ILSpy
if (trait is IDisabledTrait disabledTrait)
{
	return !disabledTrait.IsTraitDisabled;
}
return true;
```

Also `((object)x as T)` + null test instead of `x is T` in 3 places, and
`System.Type.op_Equality(v61, ((__cil_type_01000026)(null)))` for `x == null`
(World ctor).

---

## Partial / fallback blocks (admission gaps, not quality)

23 `// IL fallback:` methods across the slice, consuming **2,919 lines** of
raw IL comment dumps (Map 1,376 / Actor 724 / Game 614 / Exts 187 / World 18).
Reasons:

| reason | count |
|---|---|
| `incompatible value/receiver types` | 14 |
| `EH fault handlers need exceptional-only cleanup` | 2 |
| `conv.i4 source category needs IL fallback` | 2 |
| `EH leave must originate in a try or catch` | 1 |
| `unsupported reference relationship argument shape` | 1 |
| `generic virtual dispatch needs runtime lookup-order modeling` | 1 |
| `cast TypeSpec needs a proved reference shape` | 1 |
| `invalid/modified/open/managed-address struct field needs IL fallback` | 1 |

Notable victims: `Exts.ISqrt(int/long)`, `Exts.ColorLerp`, `Exts.ResizeArray`,
`Exts.SplitLines`, `World.ToggleInputMode<T>`, `Actor`'s `Bound`/field-bounds
helper, `Map`'s local function inside `PostInit`/`Save`, and
`Game.<>c.<ChooseShellmap>b__82_0`.

These methods have a body, so they are excluded from the ratio discussion, but
they are 14% of our printed lines and they sit *inline in the readable view*.
One line (`// IL fallback: <reason>`) plus `--details` would recover most of
that.

---

## Ranked shortlist -- best readability gain per unit of semantic risk

1. **Resolve `__cil_type_<token>` to the real BCL type name and emit `using`
   directives instead of fully-qualified names + `global::`.**
   403 opaque aliases + 6,618 FQN tokens + 1,594 `global::` in this slice.
   The identity is already computed (the token is in the text); the change is
   in the printer and a `using` collector. No IL analysis, no semantic risk.
   Touches nearly every line of every declaration.

2. **Move `// structural signature`, `// generic body context`,
   `// enum storage`, `// Exact call`, `// Dependency identity substituted` and
   the `// IL fallback:` dumps behind `--details`.**
   3,899 of 4,089 comment lines and 14% of total output, zero information lost
   (it moves, it is not deleted). Pure switch, no analysis.

3. **Fold accessor methods back into `property` / `event` declarations and
   delete the type-footer `// property ...` / `// event ...` block.**
   122 accessors + 80 header comments across all five types; the synthesis
   data already exists because the header block is being generated. Reorders
   nothing, drops no operation; the only care needed is `this.` suppression
   inside the folded bodies and event-vs-property disambiguation (already
   decided by the header).

4. **Structurize the CFG: drop `IL_xxxx: ;` labels and `goto` in favour of
   `if`/`else`/`break`/`continue`, then rebuild `for`/`while` from the
   back-edge shape.**
   2,966 goto+label sites collapse to plain nesting; the pattern is
   structurally guaranteed (single-entry single-exit regions already known to
   the structuring pass that emitted the labels). `for`/`while` recovery
   (39 constructs) rides on top and is separable if it ever regresses.

5. **Recover `foreach` from the enumerator try/finally shape.**
   79 occurrences -- the single largest missing construct -- and 37 of the 69
   try/finally blocks disappear with it. This is the one item on the list that
   needs new analysis (identify `GetEnumerator`/`MoveNext`/`get_Current`/`Dispose` on the
   same local and prove the `finally` guard only tests the enumerator), so it
   ranks last despite the biggest per-instance payoff. Requires its own
   fixtures before it is enabled by default.

Deliberately *not* on the shortlist: `yield return` reconstruction (6.5) and
object/array initializer recovery (6.3). The first is the excluded
async/closure family; the second is 17 sites whose lowered form
(`v162.Mod = v165; ...`) is verbose but faithful, and its `__cil_newobj(..., null,
1, null)` spelling would need default-argument elision to be worth anything.
