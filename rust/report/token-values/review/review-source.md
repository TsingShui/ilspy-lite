# Token operands: main-session source/IL self-review

Bounded main-session self-review, not independent or formal verification.
`manifest.json` keeps commands/statuses and three views per method: production
default with `--details`, original IL, and ILSpy. All runs exit 0 and ILSpy
prints no unknown-result diagnostic. No corpus assembly was executed.

| Method | Checked obligations | Verdict |
|---|---|---|
| 0000-06000482 FSharpExpr.Value | Handle for the same TypeRef feeds the original `GetTypeFromHandle` call; enumeration and constructor arguments keep their order. | CRUDE-BUT-EQUIVALENT |
| 0000-06000616 ExtraTopLevelOperators.CreateDictionary | `ldtoken !!0` precedes the type-handle call and the `IsValueType` callvirt; both branches keep the original method-spec calls and return paths. | CRUDE-BUT-EQUIVALENT |
| 0000-0600067A QueryModule.get_boolTy | `ldtoken [mscorlib]System.Boolean` then `Type.GetTypeFromHandle`; no `typeof` fold, no reflection, single returned handle. | CRUDE-BUT-EQUIVALENT |
| 0000-060009D3 LeafExpressionConverter.NullableConstruction | Null tests, field reads and the `ldtoken int?` comparison stay in original order; the handle is only passed to `equivHeadTypes`. | CRUDE-BUT-EQUIVALENT |
| 0000-06000A5B ConvExprToLinqInContext.Invoke | Handle value is produced where the original produced it and consumed once; no duplicate evaluation. | CRUDE-BUT-EQUIVALENT |
| 0000-06000A63 ConvExprToLinqInContext.Invoke | Same shape; branch and call order unchanged. | CRUDE-BUT-EQUIVALENT |
| 0000-06000BA0 WebExtensions.handler | Local closure allocation precedes `ldftn` of its `Invoke`, then the external delegate constructor receives the exact pointer; no receiver duplication or early null check. | CRUDE-BUT-EQUIVALENT |
| 0000-06000BC9 AsyncDownloadString.Invoke | `ldftn` value and surrounding state effects keep original order. | CRUDE-BUT-EQUIVALENT |

No changed-semantic SUSPECT in this sample. `ldftn` is not folded into a method
group and `ldtoken` is not folded into `typeof`, so the output stays a faithful
handle value rather than ILSpy's presentation-level sugar.

## Retained

- Delegate creation over a *constructed generic* delegate type still stops at the
  external constructed-member gate; that boundary is unchanged and counted.
- `ldvirtftn`, function-pointer signatures and varargs are not admitted.
- The original review was static-only. A later owned adapter executes recovered
  generic/closed type-token, delegate and array bodies with exact operation leaves:
  20 comparisons and three semantic mutants per configuration. This does not
  establish whole-corpus or cross-runtime equivalence.
