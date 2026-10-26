# EH self-review against ILSpy and original IL

Main-session review; no subagents, builds or third-party execution performed in
this review. Production default readable output was compared with ILSpy and the
original IL. This is heuristic static evidence, not an independent proof.

Inputs: `manifest.json` records image id, exact token, path and supplied reference
set. Each slug has `.ours.txt`, `.ilspy.txt` and `.il.txt` captured with the same
member selector. All ILSpy invocations succeeded and produced nonempty source.

| slug | method | verdict | decisive observation against original IL |
|---|---|---|---|
| 0002-060003AD | YieldReturnWithTryFinally Dispose | CRUDE-BUT-EQUIVALENT | states -3/2 enter the try; leave exits through the one finally thunk before returning |
| 0003-060021FE | InheritanceHelper GetBaseMembers Dispose | CRUDE-BUT-EQUIVALENT | inner cleanup token 06002201 precedes outer cleanup 06002200; exits at 001C and 001F run the right subset |
| 0004-06000138 | Binder.BindElementAccess | CRUDE-BUT-EQUIVALENT | argument acquisition occurs before try; both result branches store the result then exit through Free at 0055; exceptions also run Free |
| 0005-0600058E | PdbWriter.Finalize | CRUDE-BUT-EQUIVALENT | Close call remains in try and exact base Finalize call remains in finally; ILSpy folds that pattern into destructor syntax |
| 0006-06000B93 | ModuleDefinition GetTypes Dispose | CRUDE-BUT-EQUIVALENT | states -3/2 enter cleanup once; state read and direct thunk call preserve their IL locations |
| 0007-06000F0C | Newtonsoft Values Dispose | CRUDE-BUT-EQUIVALENT | switch maps -5/3 to third cleanup and -4/-3/1/2 to the outer cleanup path; nested cleanup runs only for -4/2 |
| 0008-06000386 | ImmutableArrayExtensions SelectManyIterator Dispose | CRUDE-BUT-EQUIVALENT | states -3/1 run the exact constructed-owner cleanup token once before returning |
| 0009-06000ADD | ByteArrayMemoryProvider.Finalize | CRUDE-BUT-EQUIVALENT | virtual Dispose(false) runs in try, direct base Finalize runs in finally, matching IL call vs callvirt |
| 0019-06000001 | catch Program.Main | CRUDE-BUT-EQUIVALENT | inner Baz, ordered ArgumentException/Exception catches, then outer Baz preserve clause nesting and order |
| 1041-06000459 | OpenRA.ObjectCreator.Finalize | CRUDE-BUT-EQUIVALENT | direct Dispose(false), then direct Object.Finalize on either exit; no extra allocation or receiver check |
| 1042-0600193D | PlayerResources.GiveCash | CRUDE-BUT-EQUIVALENT | independent Cash/Earned guards and checked adds retain two separate OverflowException handlers, each saturating only its own field |

No EH control-flow differences found in these 11 samples: 11
CRUDE-BUT-EQUIVALENT, 0 SUSPECT, 0 INSUFFICIENT-EVIDENCE for the region semantics
under review. Filters and faults remain Partial and are not claimed here.

## Limits

The two generic iterator samples retain existing open-owner naming and explicit
input-scoped call tokens, with generic body context recorded in the output. This
review checks region transfer semantics in that C#-like representation; it does
not certify the generic owner's standalone C# spelling. Likewise destructor and
exact base/direct-call syntax is not a standalone-compilation claim.

The region behavior gate is separate and stronger for owned inputs: it compiles
the default text without handler adapters or no-op cleanup helpers, and compares
results, static traces, exception types and original object identity. It rejects
four deliberate semantic mutations in release and debug configurations.
