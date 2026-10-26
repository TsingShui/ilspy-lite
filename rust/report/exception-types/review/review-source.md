# Exception identity/construction: main-session source/IL self-review

This is a bounded main-session self-review, not independent or formal
verification and not execution of any third-party method. Selection is retained
in selection.json; manifest.json records all 12 production default-readable,
ILSpy and original-IL commands, exact input paths, statuses and reference lists.
All twelve are gained methods in delta.json. No reference policy or denominator
changed. Original IL is authority, including when the surface is less idiomatic.

| Method | Changed-semantic review | Verdict |
|---|---|---|
| 0000-06001F8D Operators.Raise<T> | The exact incoming Exception is thrown; null retains runtime null-throw behavior. Generic result is never fabricated. | EQUIVALENT |
| 0003-06000083 CecilExtensions.ResolveOrThrow | Resolve callvirt precedes null test; only the null branch allocates the exact local ReferenceResolvingException and throws it. Existing nonnull result is returned. | CRUDE-BUT-EQUIVALENT |
| 1042-06000ECD DevCommands.Crash | Exact nested local DevException constructor and subsequent throw; no use of unused arguments or replacement exception. Static review only; never invoked. | CRUDE-BUT-EQUIVALENT |
| 1041-06000119 FieldLoader.ParseBooleanExpression | Null branch stays outside try; cache read/call is inside. Catch retains exact InvalidDataException object, reads its Message before local YamlException construction, and throws the new object. Success leave returns saved object; InvalidValueAction argument order is unchanged. | CRUDE-BUT-EQUIVALENT |
| 0000-060001AE FSharpValueOption<T>.get_Value | Original whole-value receiver copy into local precedes Tag call. Zero tag constructs InvalidOperationException with original literal and throws; other branch reads the copied item's original generic identity. | CRUDE-BUT-EQUIVALENT |
| 0626-06000E58 TextWriterTraceListener.Close | Initial writer null test remains separate from later read/call; only ObjectDisposedException is swallowed, other exceptions escape before writer=null. Clear occurs after either successful call or that typed catch. | CRUDE-BUT-EQUIVALENT |
| 0626-06000158 MobileAuthenticatedStream.GetInvalidNestedCallException | Name does not imply a return: original IL constructs InvalidOperationException then throws it. Recovered body does likewise with exact token/literal. | CRUDE-BUT-EQUIVALENT |
| 0003-0600004E DecompilerSettings.set_CSharpFormattingOptions | Parameterless ArgumentNullException occurs before field comparison/write; object identity comparison, field store and OnPropertyChanged virtual call retain order. | CRUDE-BUT-EQUIVALENT |
| 1041-0600059B StreamExts.ReadBytes | Negative-count two-string exception is first; each Read uses current offset/count, zero read constructs EndOfStreamException, positive read updates both scalars unchecked before back edge. Existing byte array identity and callvirt token retained. | CRUDE-BUT-EQUIVALENT |
| 0061-06000843 ModuleReader.TokenOutOfRangeException | Int32 boxing and virtual Name read precede String.Format, then exact two-string exception construction; subtype proof permits return as Exception with no runtime cast/copy. | CRUDE-BUT-EQUIVALENT |
| 0911-06000019 ThrowHelper.CreateArgumentOutOfRangeException_PrecisionTooLarge | Resource accessor, byte-99 boxing and format precede exact allocation; original subtype returned without substituting a generic exception or boxing byte as Int32. | CRUDE-BUT-EQUIVALENT |
| 0425-060013A2 ADP.StreamClosed | SR lookup, Format and InvalidOperation factory call remain ordered. Static subtype-to-Exception proof adds no runtime operation or allocation. | CRUDE-BUT-EQUIVALENT |

No changed-semantic SUSPECT found in this sample. Source-level extension/property
presentation and scratch locals differ from ILSpy. This does not establish
whole-corpus or cross-runtime equivalence. Unreviewed methods, missing references,
generic/foreign exception ancestry and catch-object wrapping remain limitations.
