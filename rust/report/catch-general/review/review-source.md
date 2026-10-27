# General catch and OutOfMemoryException: main-session self-review

Bounded main-session review, not independent or formal verification. Five gained
methods retain production default, ILSpy and original IL in manifest.json; all
runs exit 0 and ILSpy reports no unknown-result diagnostic. No input assembly was
executed.

| Method | Checked obligations | Verdict |
|---|---|---|
| 0002-060002B6 ConditionalReturnInThrow | Try body keeps both virtual calls and the conditional return; the general catch binds the thrown object and stores it before leaving the region; the post-region return is unchanged. | CRUDE-BUT-EQUIVALENT |
| 0002-0600040D YieldBreakInCatch.MoveNext | Iterator state ordering and the catch region stay as emitted; the state machine's fields and returns are unchanged. | CRUDE-BUT-EQUIVALENT |
| 0004-060001DC FoldStringConcatenation | Catch-all branch keeps its store and continuation; no operand reordering. | CRUDE-BUT-EQUIVALENT |
| 0004-0600124F GetDescription | Same shape: catch-all region plus unchanged surrounding calls. | CRUDE-BUT-EQUIVALENT |
| 0005-060008A0 TryParseUInt64 | Out-parameter zeroing, prefix tests, the `Convert.ToUInt64(value, fromBase)` call and the catch-all return keep original order; the caught object is bound, not discarded. | CRUDE-BUT-EQUIVALENT |

ILSpy prints a bare `catch` because the caught object is unused; keeping
`catch (object __cil_exception_0)` plus the store preserves the same behavior with
the value explicit. No changed-semantic SUSPECT in this sample.

## Retained

- Any other exception type still needs an admitted identity: a non-core assembly
  scope stays Partial (owned `ForeignException` negative), and the core whitelist
  did not become a name/suffix rule.
- Throwing `System.Object` remains refused; only a catch clause treats it as a
  general catch.
