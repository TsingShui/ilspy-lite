# General catch clauses

ILSpy keeps a catch clause's variable and omits its type when the variable is
unused and the type is `System.Object` (`CSharp/StatementBuilder.cs`), which is how
a `catch (object)` in IL becomes a plain `catch` in C#. The same source shows the
type printed whenever the variable is used, so the caught object stays available.

## Adopted

- A catch whose exact core type is `System.Object` is admitted as a general catch.
  The handler still binds the thrown object, so a following store is preserved
  instead of dropped.
- `System.OutOfMemoryException` joins the bounded core parent table with parent
  `System.SystemException`; the parent was read from `System.Private.CoreLib`
  metadata (`.extends 0x0200019D System.SystemException`), not inferred from the
  name.
- Neither change widens throw targets or accepts a name suffix: a catch type in a
  non-core assembly scope still needs its own admitted identity.

## Evidence

Owned `ExceptionRegionCases` gains `GeneralCatch` (`catch { }`) and `OutOfMemory`;
both were red before the change and green after it. A new frozen negative,
`ForeignException` (`catch (System.Net.WebException)`), keeps the scope boundary
visible: it must stay Partial with the exception-identity reason.

Fixed corpus after three groups since the previous scan: **626,242 Decompiled /
144,506 Partial of 770,748**, fallback **18.7488%**, **+578 in 88 images** against
the token-operand scan, **zero regressions**. OpenRA.Game +5, Common +5.

Limits: the core parent table is still bounded, so other exception types remain
Partial; filters, fault handlers and non-core ancestry are unchanged.
