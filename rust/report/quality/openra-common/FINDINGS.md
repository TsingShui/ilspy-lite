# Class-level quality comparison: `openra-common`

Slice: `openra-common`. Image: `rust/acceptance/openra/assemblies/OpenRA.Mods.Common.dll`
(read-only). Both sides already have a body for every method.

Types in scope (5):

| Type | ours lines | ILSpy lines | index ratio |
|---|---:|---:|---:|
| `OpenRA.Mods.Common.Traits.Aircraft` | 6,242 | 1,351 | 4.62 |
| `OpenRA.Mods.Common.Traits.Mobile` | 4,716 | 1,034 | 4.56 |
| `OpenRA.Mods.Common.Traits.ProductionQueue` | 3,045 | 616 | 4.94 |
| `OpenRA.Mods.Common.Server.LobbyCommands` | 6,860 | 1,374 | 4.99 |
| `OpenRA.Mods.Common.Scripting.Global.ColorGlobal` | 401 | 148 | 2.70 |

Method: paired `.ours.cs` / `.ilspy.cs` read side by side, plus mechanical counting of
the shapes named in the brief. No input code was executed; no other assembly was read.

## 1. Recalibrating the ratio before comparing anything

The raw line ratio overstates the quality gap, because a large share of our lines are
dumped IL inside `// IL fallback:` blocks (admission gaps, section 5) and diagnostic
comments (F13). Counting only non-blank, non-comment lines:

| Type | ours code lines | ILSpy code lines | code-line ratio |
|---|---:|---:|---:|
| `Aircraft` | 4,522 | 1,211 | 3.73 |
| `Mobile` | 3,391 | 901 | 3.76 |
| `ProductionQueue` | 2,654 | 561 | 4.73 |
| `ColorGlobal` | 229 | 106 | 2.16 |
| `LobbyCommands` | 814 | 1,287 | **0.63** |

`LobbyCommands` is the outlier in both directions: 75 of its 120 methods are fallback
blocks, so 85.7% of the file is comment text (83.7% is dumped IL alone) and its *code* is
shorter than ILSpy's.
Treat its ratio as an admission signal, not a quality signal. The three Traits classes
are the real quality evidence in this slice.

## 2. Category summary

| Category | Occurrences | Where the count comes from |
|---|---:|---|
| 1. Identifier naming | 5,712 | 952 token-named call sites, 2,938 structural alias type names, 821 stack-temp locals, 580 `__name_<hex>` names, 421 `__il_field_*` fields |
| 2. Semantic compression | 3,604 | 406 fall-through `else { goto L }`, 548 other `goto IL_`, 870 stray `IL_xxxx: ;`, 475 dead `= default;`, 229 trailing `return;`, 169 assign-then-return, 462 `unchecked(`, 422 bool-shuttle helpers, 11 self-increment, 12 float-bit helpers |
| 3. Type and member spelling | 3,467 | 2,141 `System.*` FQN prefixes, 1,026 `global::`, 188 `@`-escaped identifiers, 75 `op_Equality`/`op_Inequality`, 37 over-escaped string literals (1,321 char escapes) |
| 4. Structure and layout | 706 | 93 attributes dropped (0 emitted), 102 `get_`/`set_` accessor methods + 90 trailing `// property` comments in place of 77 property declarations, 421 backing fields hoisted into the field block, nested types moved to end of class |
| 5. Comments and diagnostics | 8,897 | 598 `// structural signature`, 51 `// enum storage token`, 21 `// Exact call ... kept`, 90 `// property`, 110 dependency-substitution/unresolved footer lines (22 per file), 7,904 dumped-IL lines |
| 6. Missing source-level constructs | 309 | 40 `foreach`, 32 `lock`, 11 `yield return`, 9 `?.`/`??`, 6 `++`, 8 `+=`, 1 collection initializer, 202 `__cil_newobj_*` instead of `new T(...)` (all zero on our side) |

Source-level construct counts on the ILSpy side are for the same five types as a whole;
a portion of them sit inside methods that are fallback on our side and are therefore
listed in section 5, not here.

## 3. Findings, ordered by impact

### F1 (cat 1) Call sites are printed as token helpers instead of member names - 952 sites

The printer already resolves member names on *field* access (`v0.Queue`, `v1.Yaw`) but
falls back to an opaque token helper whenever the member is *invoked*.

`ProductionQueue.ClearQueue`:

```csharp
// ours
int v6 = __cil_callvirt_06001994(v5);
int v7 = 0;
if (v6 <= v7) { goto IL_0043; } else { goto IL_001F; }
IL_001F: ;
global::OpenRA.Mods.Common.Traits.ProductionQueue v8 = this;
global::OpenRA.Mods.Common.Traits.PlayerResources v9 = v8.playerResources;
global::OpenRA.Mods.Common.Traits.ProductionItem v10 = l1;
int v11 = __cil_callvirt_06001994(v10);
__cil_callvirt_0600193B(v9, v11);
```

```csharp
// ILSpy
if (item.ResourcesPaid > 0)
{
    playerResources.GiveResources(item.ResourcesPaid);
```

Family breakdown: `__cil_callvirt_*` 377, `__cil_newobj_*` 202, `__cil_direct_*` 199,
`__cil_ldftn_*` 84, `__cil_ldstr_*` 41, `__cil_ldsfld_*` 23, plus 26 misc. `ldftn` sites
are the worst: `nint v24 = __cil_ldftn_06001987;` hides a method group that ILSpy prints
as `State`/`StartGame` in a dictionary initializer (example in F12).

Caveat, because it changes the fix: some token forms are deliberate precision.
`__cil_direct_06000F2E(this)` records "use `call`, not `callvirt`, on a virtual
this-accessor", and the printer says so in a comment
(`// Exact call 0x06000F2E kept: direct virtual this accessor needs exact call notation.`).
The fix is naming, not lowering: keep the `call`/`callvirt` distinction, put the resolved
name next to it.

Fix: resolve the member name from the call token where the name is already available
(it is, for 952 sites) and print `receiver.Member(args)`; keep the token as a suffix or
`--details` note for the call-kind cases. Presentation only, no operation moves.
Risk: low, except where the token is a generic method instantiation
(`__cil_methodspec_call_2B0000FC` = `int.Clamp`) which needs the instantiation rendered
before the name is complete.

### F2 (cat 4) Accessor methods and `__il_field_*` instead of properties - 102 accessors, 77 properties

Every property in the slice is emitted as two methods plus a comment at the end of the
class, and its backing field is emitted as a `__il_field_<token>` field.

`ProductionQueue`:

```csharp
// ours
    private OpenRA.Actor __il_field_04001542;
    private bool __il_field_04001543;

    public OpenRA.Actor get_Actor()
    {
        __cil_type_01000025 v1 = this.__il_field_04001542;
        return v1;
    }

    public bool get_Enabled()
    {
        return this.__il_field_04001543;
    }

    protected void set_Enabled(bool value)
    {
        int v1 = __cil_bool_load(value);
        this.__il_field_04001543 = __cil_bool_store(v1);
        return;
    }
```

```csharp
// ILSpy
	public Actor Actor { get; }

	[Sync]
	public bool Enabled { get; protected set; }

	public string Faction { get; private set; }
```

`ColorGlobal` is 100% affected: 36 accessors for 37 properties, each body
`v0 = ...; return v0;`, each preceded by a 200-character `// structural signature` line,
plus 36 `// property OpenRA.Primitives.Color Aqua { get: get_Aqua; }` comments at the end.

Fix: pair `get_X`/`set_X` into one property declaration and print the accessor
accessibility that the metadata already carries (`protected set` is recoverable from
`set_Enabled`'s visibility, which we already print correctly). `__il_field_*` becomes
either `{ get; set; }` or a named backing field. This is a printing change; no accessor
is removed from the output.
Risk: low. Keep the emitted accessor bodies, just move them under `get { }` / `set { }`
so a reader can see the member shape.

### F3 (cat 2) Fall-through `else { goto L; } L: ;` - 406 sites; stray `IL_xxxx: ;` labels - 870

Our branch printer emits the true branch as the `if` body and sends the false branch
through an explicit `goto` to the label that is *already the next line*.

`Aircraft.AddedToWorld`:

```csharp
// ours
        int v11 = v10.Length;
        global::OpenRA.Mods.Common.Traits.AircraftInfo v13 = ((__cil_ref_656e6dffaba1c09d791add19616ed55de0dae54d)this).Info;
        int v14 = v13.MinAirborneAltitude;
        if (v11 < v14) {
            goto IL_003D;
        } else {
            goto IL_0037;
        }
        IL_0037: ;
        __cil_direct_06000F83(this);
        IL_003D: ;
```

```csharp
// ILSpy
		if (wDist.Length >= Info.MinAirborneAltitude)
		{
			OnAirborneAltitudeReached();
		}
```

`Aircraft.get_MovementSpeed` shows the same shape twice, and the second `else` is dead
in a second way (`if (!(v3 == 0))` is `if (v3 != 0)`):

```csharp
// ours
        int v3 = __cil_bool_load(__cil_direct_0A0010FE(this));
        if (!(v3 == 0)) {
            return 0;
        }
```

```csharp
// ILSpy
			if (IsTraitDisabled || IsTraitPaused)
			{
				return 0;
			}
```

Counts per type: `Aircraft` 181 fall-through `else`/181 total `else` + 431 gotos,
`Mobile` 118, `ProductionQueue` 100, `LobbyCommands` 7. Stray labels (labels with no
predecessor branch) total 870: `Aircraft` 388, `Mobile` 263, `ProductionQueue` 206,
`LobbyCommands` 13.

Fix: (a) drop an `else`/`goto` pair whose target is the immediately following label;
(b) drop the `IL_xxxx: ;` label statement when no remaining `goto` references it;
(c) negate the condition instead of branching around a single statement where the CFG
already guarantees a two-way split. All three preserve the emitted operations verbatim.
Risk: none for (a) and (b), which only delete unreachable/duplicate branch text. (c) is
a printer-level reordering of the same two edges and should still be validated by the
existing structural checks before it is turned on by default.

### F4 (cat 4) Attributes are dropped entirely - 93 in ILSpy, 0 emitted

This is the largest structural omission in the slice and it is uniform: no `[...]` line
appears in any of the five outputs.

```csharp
// ours (ColorGlobal, whole header)
namespace OpenRA.Mods.Common.Scripting.Global;

public class ColorGlobal : OpenRA.Scripting.ScriptGlobal
{
    public ColorGlobal(OpenRA.Scripting.ScriptContext context) : base(context)
```

```csharp
// ILSpy
namespace OpenRA.Mods.Common.Scripting.Global;

[ScriptGlobal("HSLColor")]
public class ColorGlobal : ScriptGlobal
{
	[Desc(new string[] { "FromHex(\"00FFFF\")" })]
	public Color Aqua => Color.Aqua;
```

Distribution: `ColorGlobal` 40 (1 type-level `[ScriptGlobal]`, 39 `[Desc]`),
`LobbyCommands` 45 (`[FluentReference]` per const, 37 of them), `ProductionQueue` 2
(`[Sync]` on `Enabled` and `IsValidFaction`), `Mobile` 4, `Aircraft` 2.

This is not only cosmetic: `[Sync]` and `[FluentReference]` are read at runtime, and the
`[Desc]` payloads are the only documentation of the scripting API. Flagging it as a
decision rather than a quiet fix, because rendering attribute arguments is new surface
(constructor args, `new string[]{...}` arrays, named args).
Fix: emit type/member attributes with reconstructed arguments; rely on the same metadata
tokens the resolver already reads.
Risk: low semantically (nothing is removed from the output), but it is new scope, so it
needs an explicit go-ahead rather than being folded into a formatting change.

### F5 (cat 1) Internal stack-slot locals leak into the output - 821

Temporaries that the lifter introduces for expression slots are printed with their
internal names (`e<slot>_<depth>_<n>`, `s<slot>_<depth>`).

`Aircraft.GetTurnSpeed(bool)` - 60 lines on our side, 1 line in ILSpy:

```csharp
// ours
        IL_0008: ;
        global::OpenRA.Mods.Common.Traits.Aircraft v2 = this;
        int v3 = __cil_bool_load(__cil_direct_0A0010FE(v2));
        ...
        global::OpenRA.Mods.Common.Traits.Aircraft v7 = this;
        __cil_value_4dc9b0074cc3358fbbb6a841eafae67a8806e8ae v8 = __cil_direct_06000F31(v7);
        __cil_value_4dc9b0074cc3358fbbb6a841eafae67a8806e8ae e5_9_0 = v8;
        s9_0 = e5_9_0;
        goto IL_0043;
        IL_003C: ;
        ref __cil_value_877f2d9f1d5ae9ff8420452e071399208150ae77 v15 = ref l1;
        __cil_value_4dc9b0074cc3358fbbb6a841eafae67a8806e8ae v16 = __cil_value_call_0A001112(ref v15);
        __cil_value_4dc9b0074cc3358fbbb6a841eafae67a8806e8ae e8_9_0 = v16;
        s9_0 = e8_9_0;
        goto IL_0043;
        IL_0043: ;
        l0 = s9_0;
        ...
        int v24 = __cil_methodspec_call_2B0000FC(v21, v22, v23);
        __cil_value_4dc9b0074cc3358fbbb6a841eafae67a8806e8ae v25 = __cil_newobj_0A000219(v24);
```

```csharp
// ILSpy
		return new WAngle(Util.ApplyPercentageModifiers(((!isIdleTurn) ? TurnSpeed : (IdleTurnSpeed ?? TurnSpeed)).Angle, speedModifiers).Clamp(1, 1024));
```

Distribution: `Aircraft` 320, `Mobile` 276, `ProductionQueue` 225, others 0. The `s9_0`
local is a phi/merge slot for the ternary; `e5_9_0`/`e7_9_0`/`e8_9_0` are the three arms.

Fix: two independent changes. (a) Rename slot temps to a non-`e`/`s` scheme so they do
not collide with source-level locals - naming only. (b) Fold the `sN_M` merge local back
into `?:` (or `??`) once the three arms are proven side-effect free and single-use. (b)
needs the merge-slot analysis the brief calls out as expensive.
Risk: (a) none. (b) medium - requires proving each arm is a pure value and the slot is
read once.

### F6 (cat 2) Dead declaration and return noise - 475 + 229 + 169 lines

Every local is declared at the top of its method and initialized to `default`, then
assigned before first read. Void methods end with an explicit `return;`. Single-value
returns go through a local.

`ColorGlobal.New`:

```csharp
// ours
        byte l0 = default;
        byte l1 = default;
        byte l2 = default;
        int v3 = __cil_methodspec_call_2B0000FC(hue, 0, 255);
        int v4 = unchecked((int)(unchecked((byte)(v3))));
        l0 = unchecked((byte)(v4));
        ...
        __cil_value_625aea2dd1a31ad130bf96e01b5f7559f7a115a6 v28 = OpenRA.Primitives.Color.FromAhsl(255, __cil_call_arg_r4(v19), __cil_call_arg_r4(v23), __cil_call_arg_r4(v27));
        return v28;
```

```csharp
// ILSpy
		byte b = (byte)hue.Clamp(0, 255);
		byte b2 = (byte)saturation.Clamp(0, 255);
		byte b3 = (byte)luminosity.Clamp(0, 255);
		return Color.FromAhsl(255, (float)(int)b / 255f, (float)(int)b2 / 255f, (float)(int)b3 / 255f);
```

`Aircraft.get_IdleTurnSpeed` shows the worst variant - `return null` becomes three
statements through a `ref`:

```csharp
// ours
        IL_001C: ;
        ref __cil_value_877f2d9f1d5ae9ff8420452e071399208150ae77 v7 = ref l0;
        v7 = default;
        __cil_value_877f2d9f1d5ae9ff8420452e071399208150ae77 v8 = l0;
        return v8;
```

```csharp
// ILSpy
			return null;
```

Counts: 475 `= default;` (Aircraft 174, Mobile 120, ProductionQueue 76, ColorGlobal 3,
LobbyCommands 1), 229 trailing `return;` (Aircraft 79, Mobile 64, ProductionQueue 57,
LobbyCommands 28, ColorGlobal 1), 169 assign-then-return (index).

Fix: (a) suppress the `= default` clause when a definite-assignment pass shows the local
is written before every read (or print `T x;` and keep the assignment); (b) drop a final
`return;` in a void body; (c) `v = expr; return v;` -> `return expr;` when `v` is a
single-assignment temporary. (c) is the only one with any proof obligation and the
assign-then-return sites here are all single-use.
Risk: none for (a) and (b). (c) is safe for single-use temps and must be restricted to
them, otherwise a `ref` alias or later read would change meaning.

### F7 (cat 3) Type and identifier spelling - 3,467 sites

Four sub-shapes, all formatting:

1. `System.*` fully qualified: 2,141. `System.Collections.Generic.Dictionary<OpenRA.ActorInfo, ...>`,
   `System.Nullable<OpenRA.WAngle>`, `System.ValueTuple<OpenRA.CPos, OpenRA.Traits.SubCell>[]`,
   `System.Collections.Generic.IReadOnlyDictionary<string, int>`.
2. `global::` prefix: 1,026 (`Aircraft` 371, `ProductionQueue` 359, `Mobile` 288,
   `LobbyCommands` 8).
3. `@`-escaped identifiers: 188, always on OpenRA's own namespace segments:
   `global::@OpenRA.@Mods.@Common.@Traits.@MovementType` (`Mobile` 106, `Aircraft` 80,
   `LobbyCommands` 2).
4. Operator methods: 75 (`OpenRA.CPos.op_Inequality(v1, v3)` for `FromCell != ToCell`;
   `OpenRA.WDist.op_Equality(...)` for `==`).

Also: 0 `using` directives emitted, vs 35 in the ILSpy counterparts.

`Mobile.get_IsMovingBetweenCells` is the operator case, and `Mobile.OccupiedCells` the
tuple-shorthand case:

```csharp
// ours
    public bool get_IsMovingBetweenCells()
    {
        __cil_value_13b48834ea748f1ea38d02def38ca26e6d26c625 v1 = this.FromCell;
        __cil_value_13b48834ea748f1ea38d02def38ca26e6d26c625 v3 = this.ToCell;
        return OpenRA.CPos.op_Inequality(v1, v3);
    }

    public virtual System.ValueTuple<OpenRA.CPos, OpenRA.Traits.SubCell>[] OccupiedCells()
```

```csharp
// ILSpy
	public bool IsMovingBetweenCells => FromCell != ToCell;

	public (CPos, SubCell)[] OccupiedCells()
```

Fix: emit a `using` block for resolved namespaces and print short names; keep `global::`
only for names that genuinely conflict; strip the `@` escapes (none of these identifiers
are keywords); map `op_*` to the operator token; `System.Nullable<T>` -> `T?`,
`System.ValueTuple<A, B>` -> `(A, B)`.
Risk: low, but note that `global::`/`@` were presumably added to guarantee unambiguity.
Removing them wholesale can change which type a name binds to if a local/field shadows a
namespace segment, so gate the short-name path on "no shadowing in scope" rather than
deleting the escape unconditionally.

### F8 (cat 3) String literals are escaped one character at a time - 37 literals, 1,321 escapes

Every character of every `const string` is emitted as `\uXXXX`.

```csharp
// ours
    private const string CustomRules = "\u006E\u006F\u0074\u0069\u0066\u0069\u0063\u0061\u0074\u0069\u006F\u006E\u002D\u0063\u0075\u0073\u0074\u006F\u006D\u002D\u0072\u0075\u006C\u0065\u0073";
    private const string OnlyHostStartGame = "\u006E\u006F\u0074\u0069\u0066\u0069\u0063\u0061\u0074\u0069\u006F\u006E\u002D\u0061\u0064\u006D\u0069\u006E\u002D\u0073\u0074\u0061\u0072\u0074\u002D\u0067\u0061\u006D\u0065";
```

```csharp
// ILSpy
	[FluentReference]
	private const string CustomRules = "notification-custom-rules";

	[FluentReference]
	private const string OnlyHostStartGame = "notification-admin-start-game";
```

All 37 occurrences are in `LobbyCommands` (its 37 `private const string` fields).
Notably the same printer *does* render ordinary literals readably elsewhere:
`string v11 = __cil_ldstr_70000931("state");`. So the defect is confined to the
const-initializer path. One literal must stay escaped: line 4841 emits
`__cil_ldstr_70001905("\u000A")`, which is correct (`\n`).
Fix: escape only `"`, `\` and control characters; print everything else literally.
Risk: none. Round-trip is preserved because non-printable characters still escape.

### F9 (cat 2) `unchecked(...)` and bool-shuttling noise - 462 + 422

`unchecked` wraps every truncating conversion, sometimes nested twice, and every `bool`
load/store round-trips through the CLI int stack.

```csharp
// ours (Mobile.set_IsImmovable)
    private void set_IsImmovable(bool value)
    {
        int v1 = __cil_bool_load(value);
        this.__il_field_04001344 = __cil_bool_store(v1);
        return;
    }
```

```csharp
// ILSpy
	public bool IsImmovable { get; private set; }
```

```csharp
// ours (ColorGlobal.New)
        int v4 = unchecked((int)(unchecked((byte)(v3))));
```

`unchecked` counts: `Mobile` 273, `Aircraft` 121, `ProductionQueue` 44, `ColorGlobal` 12,
`LobbyCommands` 2. `__cil_bool_load` 290 + `__cil_bool_store` 132.

Fix: (a) collapse `x = __cil_bool_load(b); store(x)` back to `b` when the value flows
straight through, and print the bool-typed local as `bool`; (b) collapse nested
`unchecked((T)(unchecked((U)x)))` to a single conversion when `U` is narrower than `T`.
On (b): dropping `unchecked` entirely changes behavior under `/checked+`, so the safe
first step is to stop nesting it, not to remove it. Turning `unchecked` off by default
for conv.* is a separate decision.
Risk: (a) none. (b) low if scoped to nesting only.

### F10 (cat 6) Missing source-level constructs - 308

Our output contains none of `foreach`, `lock`, `yield return`, `?.`, `??`, `++`, `+=`,
collection initializers; ILSpy reconstructs all of them in these same five types.

`ProductionQueue.ClearQueue` (4 lines vs ~55):

```csharp
// ours
        __cil_value_bf76482d0d9dd3fdbd4330e5362ee113342a6a90 l0 = default;
        OpenRA.Mods.Common.Traits.ProductionItem l1 = default;
        IL_0000: ;
        global::OpenRA.Mods.Common.Traits.ProductionQueue v0 = this;
        __cil_ref_363f0ff04a5b9e386ee41c3d2a45ee5f4aaf5f64 v1 = v0.Queue;
        __cil_value_bf76482d0d9dd3fdbd4330e5362ee113342a6a90 v2 = __cil_callvirt_0A001561(v1);
        l0 = v2;
        try {
            IL_000C: ;
            goto IL_005B;
```

```csharp
// ILSpy
	protected void ClearQueue()
	{
		foreach (ProductionItem item in Queue)
		{
			if (item.ResourcesPaid > 0)
			{
				playerResources.GiveResources(item.ResourcesPaid);
				item.RemainingCost += item.ResourcesPaid;
			}
			playerResources.GiveCash(item.TotalCost - item.RemainingCost);
		}
		Queue.Clear();
	}
```

The IL shape is present and intact on our side: `Queue.GetEnumerator()` -> `l0`, a
`try`/`finally` with `MoveNext`/`get_Current`, and `__cil_constrained_...Dispose` in the
`finally`. So this is a *recognizer* gap, not lost information.

ILSpy-side counts (whole slice): `foreach` 40, `lock` 32, `yield return` 11, `?.` 5,
`??` 4, `++` 6, `+=` 8. On our side 15 `try`-blocks in admitted bodies prove at least 15
`foreach` sites are reconstructable today (`Aircraft` 8, `Mobile` 5, `ProductionQueue` 2);
the remaining ILSpy occurrences sit in fallback methods. All 32 `lock` statements fall
inside `LobbyCommands` methods that are fallbacks, so `lock` is currently an admission
gap, not a quality gap.

`Mobile.set_CurrentMovementTypes` also shows the array-loop form of `foreach` plus the
missing `++`:

```csharp
// ours
        l2 = 0;
        goto IL_0052;
        IL_003D: ;
        ...
        int v23 = l2;
        l2 = unchecked(v23 + 1);
        IL_0052: ;
        int v26 = l2;
        __cil_ref_f61687ff1f2a60dadf9f80e4e9fbc58fbbc1bcec v27 = l1;
        nint v28 = unchecked((nint)(v27.LongLength));
        int v29 = unchecked((int)(v28));
        if (v26 < v29) { goto IL_003D; } else { goto IL_0058; }
```

```csharp
// ILSpy
				INotifyMoving[] array = this.notifyMoving;
				foreach (INotifyMoving notifyMoving in array)
				{
					notifyMoving.MovementTypeChanged(self, value);
				}
```

Fix, in ascending cost: `++`/`+=` and `nint`/`LongLength` -> `Length` are local peepholes;
`?.`/`??` need a null-test-plus-single-use proof; `foreach` needs the
`GetEnumerator`/`MoveNext`/`get_Current`/`Dispose` shape proof the brief describes;
`lock` needs `Monitor.Enter`/`Exit` in the try/finally plus the `lockTaken` refinement.
`yield return` and closure/iterator reconstruction stay deliberate and out of scope
(section 5).
Risk: `foreach`/`lock` medium (must not change Dispose/Exit ordering); `++`/`+=`/`?.`
low; none of them drop an operation as long as the recognizer keeps the enumerator
disposal and the Monitor.Exit in the same position.

### F11 (cat 1) `__name_<hex>` for explicit interface implementations and closure types - 580

Explicit interface implementations are emitted as `private virtual` methods whose name is
the hex of the full member name.

```csharp
// ours
    private virtual void __name_4F70656E52412E5472616974732E494E6F746966794164646564546F576F726C642E4164646564546F576F726C64(OpenRA.Actor self)
    {
        __cil_callvirt_06000F43(this, self);
        return;
    }
```

```csharp
// ILSpy
	void INotifyAddedToWorld.AddedToWorld(Actor self)
	{
		AddedToWorld(self);
	}
```

(`__name_4F70656E...` decodes to `OpenRA.Traits.INotifyAddedToWorld.AddedToWorld`.)
`Mobile` has 23 of these, `Aircraft` 18, `ProductionQueue` 9, `LobbyCommands` 2; they are
the only members in the output whose names a reader cannot recover without a hex decode.

The same `__name_<hex>` scheme names compiler-generated closure and iterator types:
`private sealed class __name_3C3E635F5F446973706C6179436C61737334395F30` is
`<>c__DisplayClass49_0`, and
`private sealed class __name_3C4765745661726961626C654F62736572766572733E645F5F313231`
is `<GetVariableObservers>d__121`.

Fix: for explicit implementations, print the interface-qualified name
(`void INotifyAddedToWorld.AddedToWorld(...)`) and `private` should disappear (the
existing `virtual` already records the `final virtual` newslot shape). For generated
types, print the demangled form (`DisplayClass49_0`, `<GetVariableObservers>d__121`
escaped as `@`-free readable text) even if we keep the type.
Risk: none for naming. Inlining the lambdas themselves is the deliberate closure gap and
is excluded here.

### F12 (cat 6) Collection initializer and method groups -> 29 `newobj` + 25 `ldftn` sites

`LobbyCommands`' constructor is the clearest case: ILSpy recovers a field initializer
with a dictionary initializer and 24 method groups; we emit 24 x 5 statements.

```csharp
// ours
        __cil_ref_60dc616c6864b639d0a2c165b44892c02f22fe64 v1 = __cil_newobj_0A000618();
        string v3 = __cil_ldstr_70000931("state");
        nint v4 = __cil_ldftn_06000325;
        __cil_ref_affef7f2d4008f057ef6d2e0bfffc711118b199f v5 = __cil_newobj_0A000619(null, v4);
        __cil_callvirt_0A00061A(v1, v3, v5);
        string v7 = __cil_ldstr_7000D2AD("startgame");
        nint v8 = __cil_ldftn_06000326;
```

```csharp
// ILSpy
	private readonly IDictionary<string, Func<OpenRA.Server.Server, Connection, Session.Client, string, bool>> commandHandlers = new Dictionary<string, Func<OpenRA.Server.Server, Connection, Session.Client, string, bool>>
	{
		{ "state", State },
		{ "startgame", StartGame },
		...
	};
```

Note the asymmetry: the constructor is *admitted* on our side (it is not a fallback), so
this is pure quality loss, not admission.

Fix: `new T(args)` instead of `__cil_newobj_<token>(args)` (202 sites), the delegate
creation `new Func<...>(target, __cil_ldftn_<token>)` folded to a method group
(`State`), and `dict.Add(k, v)` folded into an initializer when every statement in the
block is an `Add` on a just-constructed local. The last step is the only one that needs a
shape check; the first two are naming.
Risk: low for `new`/method-group naming; medium for the initializer fold (must confirm
the local is fresh and never aliased).

### F13 (cat 5) Diagnostic comments: necessary evidence that should move to `--details`

Seven banner families, 8,897 comment lines total (41.8% of the slice's output):

| Banner | count | verdict |
|---|---:|---|
| dumped IL inside fallback blocks | 7,904 | (a) necessary evidence, but it belongs to `--details`; it is the whole body of 118 methods |
| `// structural signature __cil_*` | 598 | (a) evidence; one line per aliased type per method, and it repeats verbatim across methods: `__cil_value_3c99b821...` (= `WRot`) alone carries the same 230-character sentence 32 times in `Aircraft` |
| `// property X { get: get_X; }` | 90 | (b) noise once F2 emits the property |
| `// enum storage token=...` | 51 | (a) evidence; but the text also carries the `global::@OpenRA...` spelling defect from F7 |
| `// Exact call 0x... kept: direct virtual this accessor...` | 21 | (a) evidence of a deliberate precision choice, should stay but move to `--details` |
| `// Dependency identity substituted: ...` | 105 | (b) noise at class scope: 21 identical lines appended to every one of the 5 files |
| `// Dependencies unresolved: 11; ...` | 5 | (a) keep one line |

Example of the repetition cost in `ColorGlobal`: 38 structural-signature lines for 40
methods that all alias the single type `Color`, i.e. the same 230-character sentence 38
times, plus a 36-line `// property` trailer at the end of the class:

```csharp
// ours
    public OpenRA.Primitives.Color get_Aqua()
    {
        // structural signature __cil_value_625aea2dd1a31ad130bf96e01b5f7559f7a115a6 bytes=118085: valuetype [OpenRA.Game]OpenRA.Primitives.Color; type-owner=0x00000000 method-owner=0x00000000; tokens scoped to this input image; whole struct value snapshot/copy, preserving embedded references; no boxing, construction, host layout or address inference.
        __cil_value_625aea2dd1a31ad130bf96e01b5f7559f7a115a6 v0 = OpenRA.Primitives.Color.get_Aqua();
        return v0;
    }
```

```csharp
// ILSpy
	public Color Aqua => Color.Aqua;
```

The `// Exact call ... kept` notes are the other family worth keeping but moving:

```csharp
// ours (Aircraft.get_MovementSpeed)
        // Exact call 0x0A0010FD kept: property syntax needs a nongeneric class owner.
        int v1 = __cil_bool_load(__cil_direct_0A0010FD(this));
```

```csharp
// ILSpy
			if (IsTraitDisabled || IsTraitPaused)
```

And the footer, identical in all five files:

```csharp
// ours
// Dependency identity substituted: System.Runtime, Version=6.0.0.0, ... -> System.Runtime, Version=10.0.0.0, ...; IlSpy, metadata only, not original-runtime behavior proof.
// [... 20 more identical lines ...]
// Dependencies unresolved: 11; exact references retained; see --details for identities and provenance.

// ILSpy: (no footer)
```

Fix: keep one line per distinct signature and emit it once above the method, or route
the whole family to `--details`; delete the `// property` trailer once properties are
printed; move the dependency-substitution list to `--details` while keeping the
`Dependencies unresolved: N` summary.
Risk: none; these are comments.

## 4. Deliberate, low priority (do not treat as defects)

| Gap | Evidence | Why it stays |
|---|---|---|
| Closure types not inlined | `__name_3C3E635F5F446973706C6179436C61737334395F30` (`<>c__DisplayClass49_0`) and `<>c` (`__name_3C3E63`) static cache classes emitted as members with their own `__cil_ldftn_`, `__cil_ldsfld_` bodies; 22 generated types in `LobbyCommands`, 11 in `Aircraft`, 11 in `ProductionQueue`, 9 in `Mobile` | lambda/closure reconstruction is a compiler-feature reconstruction we deliberately keep lowered; F11 covers only the *naming* of these types, which is cheap and independent |
| Iterators not reconstructed | `private sealed class __name_3C4765745661726961626C...3E645F5F313231 : IEnumerable<T>, IEnumerator<T>, IDisposable` with a full state machine (11 `yield return` sites in ILSpy) | same category: iterator shape reconstruction |
| `LobbyCommands` bodies mostly fallback | 75/120 methods; `ValidateSlotCommand`, `ValidateCommand` etc. are pure IL dumps | admission gap (section 5), not quality |
| `__cil_direct_` for virtual this-accessors | `__cil_direct_06000F2E(this)` + comment | deliberate call-vs-callvirt precision; keep the precision, only add the name |

## 5. Partial / fallback blocks (admission gaps, counted separately)

118 fallback blocks across the slice. These are not quality findings; they are the
reason `LobbyCommands` and parts of `Aircraft`/`Mobile` cannot be compared at all.

| Type | fallbacks / methods | reasons |
|---|---|---|
| `LobbyCommands` | 75 / 120 | 73 nested/generic/unresolved external class, 2 incompatible value/receiver types |
| `Aircraft` | 21 / 177 | 12 byref/modified parameter, 2 `ldvirtftn`, 2 incompatible value/receiver, 2 EH fault handler, 1 incompatible stack categories, 1 generic virtual dispatch, 1 external allocation |
| `Mobile` | 20 / 158 | 12 byref/modified parameter, 3 incompatible value/receiver, 2 `ldvirtftn`, 2 EH fault handler, 1 generic virtual dispatch |
| `ColorGlobal` | 1 / 40 | 1 external allocation (`FromHex`: `Color.TryParse` + `new LuaException`) |
| `ProductionQueue` | 1 / 90 | 1 generic virtual dispatch (constructor LINQ query + closure) |
| **total** | **118 / 585** | 73 nested/generic external, 24 byref/modified parameter, 7 incompatible value/receiver, 4 `ldvirtftn`, 4 EH fault handler, 3 generic virtual dispatch, 1 stack categories, 1 external allocation |

Consequence for the quality work: `LobbyCommands` contributes almost no usable quality
signal (its code-line ratio is 0.63), and `ColorGlobal.New`/`FromRGB` are the only
methods in that type that show the real gap. Conversely `ProductionQueue` (ratio 4.73,
89/90 methods admitted) is the cleanest place to measure a fix.

## 6. Ranked shortlist: best readability gain per unit of semantic risk

1. **Delete fall-through `else { goto L; }` and unreferenced `IL_xxxx: ;` labels**
   (`Aircraft` 181/388, `Mobile` 118/263, `ProductionQueue` 100/206, `LobbyCommands` 7/13).
   ~1,276 sites, removes two lines per branch, no operation is touched because both
   deletes remove text that the CFG already implies. Zero risk, largest single win.
2. **Dead declaration noise: drop `= default` when written-before-read, drop trailing
   `return;`, fold `v = expr; return v;`** (475 + 229 + 169 = 873 lines).
   Zero risk for the first two; the third is restricted to single-assignment temporaries.
3. **Emit properties from `get_`/`set_` pairs and stop printing `__il_field_*` for
   auto-property backing fields** (102 accessors, 90 trailer comments, 77 properties).
   Turns 4-8 lines per member into 1, and makes the type's API visible at a glance.
   Low risk: same accessors, same accessibility, only the member shape changes.
4. **Type and identifier spelling: `using` + short names, `T?`, `(A, B)`, operator
   methods, `@` removal** (2,141 `System.`, 1,026 `global::`, 188 `@`, 75 `op_*`,
   2,938 alias type names). Largest count in the slice; purely a printer change, provided
   the short-name path is gated on no-shadowing instead of deleting `global::` blindly.
5. **Resolve call-site member names** (`__cil_callvirt_`/`direct_`/`newobj_`/`ldftn_`/
   `ldsfld_`/`ldstr_`, 952 sites). Biggest naming gain, but it needs a decision on how to
   keep the deliberate `call` vs `callvirt` precision, so it ranks below the pure
   formatting wins. Do it after 1-4, which already shrink the surrounding shape.

Excluded from the shortlist on purpose: attributes (F4, zero semantic risk but it is new
scope and needs a go-ahead), `foreach`/`lock` reconstruction (F10, medium risk and the
most expensive recognizers), `?:`/`??` folding of merge slots (F5b), and everything in
the deliberate table in section 4.
