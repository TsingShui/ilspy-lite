# Independent class 2: Wait.Tick

The frozen class and Tick MethodDef `0x0600241A` are unchanged from
[classes.json](classes.json). The public class output keeps all three methods
and its field; Tick is Decompiled while the two constructors retain explicit IL
fallback. Full member metadata/status output matches the independent PEReader
snapshot. Class exit 2; no failed bodies or metadata failures.

```sh
python3 rust/tests/reference-roundtrip.py --wait --from-class
```

The recovered body uses an opaque image-scoped Actor TypeRef alias and retains
the exact external Activity.get_IsCanceling MemberRef `0x0A001629`, including
owner assembly identity and raw signature. Local metadata proves Wait derives
from that exact external TypeRef; no further external inheritance or overload
binding is guessed. Production rules contain no Wait/Activity names or tokens.

## Independent execution

The original side invokes pinned Wait.Tick by exact IL call. The recovered side
executes the body extracted from actual --type output on a metadata-derived
Wait shell, inheriting the unchanged external Activity class. The MemberRef
adapter resolves and executes only that original external getter leaf, by exact
IL call in a separate load context. It never calls or reconstructs the selected
Wait.Tick body. This unchanged getter boundary is explicitly allowed by the
frozen acceptance contract; general dependency decompilation is not claimed.

The input references System.Runtime 6.0.0.0. This optional gate intentionally
uses the installed .NET 10.0.12 runtime's framework-reference unification,
acknowledging Roslyn CS1701 only for this gate. Production output retains the
original identity; neither input bytes nor root SDK are changed.

## Results

**193 return/exception/state comparisons passed** on local 64-bit .NET 10.0.12:

- Base states Queued, Active, Canceling and Done; actual getter result is used,
  not synthesized from the selected Tick source.
- remainingTicks int.MinValue, -1, 0, 1, 2, int.MaxValue.
- Four repeated calls per state/count case, with both null and distinct Actor
  arguments. Actor is unused by Tick and not dereferenced or substituted.
- Canceling returns true without countdown mutation. Otherwise store the
  unchecked old-minus-one and return whether the old value was zero, including
  int.MinValue wraparound and calls after the first true result.
- Observer verifies one exact getter call before mutation, on the original
  receiver. Every base field remains unchanged; countdown and return match.
- Null receiver still reaches the exact getter operation once before its own
  body faults. No receiver check is inserted ahead of the call. Exception
  details agree.

Owned external-call regressions remain green: 48 comparisons per Debug/Release
configuration cover null receivers, independent leaf side effects and direct
virtual target dispatch. Focused Rust Debug/Release, fmt and strict clippy pass.
Logs and GPL-derived output remain in ignored `rust/report/openra/wait/`.
Existing input provenance is reused without rehashing.

The two frozen independent classes now pass their required behavioral bodies;
other members remain complete with honest fallback. They belong to one OpenRA
project and share an activity abstraction, so this is not broad ecosystem or
whole-game compatibility. Output-only Agent reading and final gates remain
required before delivery completion.
