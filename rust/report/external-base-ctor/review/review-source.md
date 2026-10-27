# Gained-method self-review: external constructed base calls

The main session selected five newly recovered constructor bodies across the
five targeted scans, including both OpenRA inputs. Each triple was captured with
production default output, production original IL and `ilspycmd -m` against the
same input; commands exited 0 for all three views. The selected methods are a
bounded judgment sample, not an independent review or whole-corpus proof. No
third-party body was executed. The IL is the semantic authority.

| Sample | Main-session verdict | Relevant observation |
|---|---|---|
| 0002-06000305 `RemoveCompilerAttribute..ctor` | CRUDE-BUT-EQUIVALENT | The single `call` of the constructed visitor base on `this` remains the exact token `0A0001F5`; no new allocation or dispatch. |
| 0003-06000507 `BaseTypeCollector..ctor` | CRUDE-BUT-EQUIVALENT | The `Stack<IType>` allocation and field write precede the exact `List<IType>` base constructor, as in IL. Neither operation is moved into a C# initializer. |
| 0007-06000720 `JsonPropertyCollection..ctor` | CRUDE-BUT-EQUIVALENT for the represented reference conversions | `StringComparer.Ordinal` is evaluated before the exact keyed-collection constructor; validation, the field write, getter, cast and second write follow in IL order. The explicit formal-reference cast retains the reference and the verifier-valid call's declared parameter; it is not an original-runtime dependency proof. |
| 1041-0600075C `WidgetArgs..ctor` | CRUDE-BUT-EQUIVALENT | The exact `Dictionary<string,object>` base-constructor token remains on the original receiver. Framework substitution notes do not claim runtime equivalence. |
| 1042-0600008B `FacingInit..ctor` | CRUDE-BUT-EQUIVALENT | The exact `ValueActorInit<WAngle>` constructor takes the original receiver and whole-value argument once. ILSpy prints `base(value)`; the generated exact operation does not allocate or dispatch. |

No ILSpy unknown-result diagnostic appeared in these five selected outputs.
`audit/owned.ilspy.txt` is separately marked suspicious for generic argument
ordering and is not used as an equivalence oracle. The newly found local
constructed-owner `callvirt` defect and its correction are documented in the
group audit, not hidden by the scan's unchanged recovered status.
