# General catch clauses

ILSpy keeps a catch clause's variable and omits its type when the variable is
unused and the type is `System.Object` (`CSharp/StatementBuilder.cs`), which is how
a `catch (object)` in IL becomes a plain `catch` in C#. The same source shows the
type printed whenever the variable is used, so the caught object stays available.

## Adopted

- A catch whose exact core type is `System.Object` is admitted as a general catch
  when its handler first discards the caught object. It renders as `catch { }`.
  A handler that uses the object remains Partial; no raw exception object or
  runtime wrapping behavior is invented.
- `System.OutOfMemoryException` joins the bounded core parent table with parent
  `System.SystemException`; the parent was read from `System.Private.CoreLib`
  metadata (`.extends 0x0200019D System.SystemException`), not inferred from the
  name.
- Neither change widens throw targets or accepts a name suffix: a catch type in a
  non-core assembly scope still needs its own admitted identity.

## Evidence

Owned `ExceptionRegionCases.GeneralCatch` demonstrated a red-to-green check for
bare catch emission. `OutOfMemory` executes both a normal path and a thrown
exception. The reused exception driver runs 51 comparisons / 10 methods per
configuration and rejects six semantic mutants, including GeneralCatch and
OutOfMemory. The earlier output emitted `catch (object ...)`, which failed C#
compilation; that failure is retained, not counted as executed validation.
`ForeignException` (`catch (System.Net.WebException)`) remains a frozen
negative for the non-core identity boundary.

Fixed corpus after three groups since the previous scan: **626,242 Decompiled /
144,506 Partial of 770,748**, fallback **18.7488%**, **+578 in 88 images** against
the token-operand scan, **zero regressions**. OpenRA.Game +5, Common +5.

The cited scan predates the correction for a used catch object. The next scan
must account for any bodies that become Partial under that necessary restriction.

Limits: the core parent table is still bounded, so other exception types remain
Partial; filters, fault handlers and non-core ancestry are unchanged.
