# Exception identity, construction and EH audit

Baseline: local `8c0918962`, 496,379 Decompiled / 274,369 Partial of 770,748
bodies; 35.597756984124516% fallback. Fixed inventory/references, no new scope
exclusions or third-party execution. `audit/manifest.json` retains six exact
production default/ILSpy/original-IL triples and commands/reference selections.

| method | original IL and decision |
|---|---|
| 0000-06001F8D Operators.Raise<T> | ldarg Exception; throw. Core Exception identity is enough; no constructor or value conversion is present. ILSpy succeeds. |
| 0003-06000083 CecilExtensions.ResolveOrThrow | Resolve callvirt; branch; local ReferenceResolvingException newobj; throw. Local ancestry must prove Exception without losing exact constructor identity. ILSpy succeeds. |
| 1042-06000ECD DevCommands.Crash | exact local nested DevException constructor, then throw. Never execute this production method. Static candidate only; ILSpy succeeds. |
| 1041-06000119 FieldLoader.ParseBooleanExpression | catch exact core-scoped System.IO.InvalidDataException object; Message callvirt; local YamlException(string); throw. Preserve caught object, allocation, message and cleanup. ILSpy succeeds. |
| 0004-06000075 BindTypeParameterConstraints | factory returns System.Exception; throw. Earlier loops/arrays have independent gates; no recovery forecast from this sample. ILSpy succeeds. |
| 0000-060004B7 Trampoline.Execute | catch System.Object, cast to Exception, rethrow under nested finally. Catch-object and non-Exception wrapping semantics are a separate boundary, not an ordinary typed exception catch. ILSpy succeeds. |

Continue typed IL/CFG + lift. Prove exact core exception identity or checked local
ancestry, never names/suffixes alone. General constructor support retains the
original token/signature, evaluation order, message/inner exception objects and
freshness; it must not guess an overload from argument count or replace a throw
with a generic exception. Catch entry needs the exact admitted reference type,
not an ExternalClass interpretation of a local TypeDef. Unknown foreign ancestry,
generic catch shapes, catch-object wrapping and filters/faults remain separate
proofs/gates until captured and validated.

Public seams are decompile/scan E2E, malformed/counterfeit metadata negatives and
owned original-vs-recovered default-output execution. Cover exception identity,
null throw, constructor overloads/argument side effects, subtype handler matching,
rethrow and finally. Keep typed exception construction separate from general
reference allocation; exact operation leaves are allowed, selected body proxies
are not. Semantic mutations must fail for relevant observations, not bad carriers.
