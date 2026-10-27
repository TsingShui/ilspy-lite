# Foreign hidden-argument relationship self-review

The main session compared default output, original IL and ILSpy source for the
two owned recovered methods below. The IL and ILSpy commands exit 0; ILSpy emits
no unknown-result diagnostic. The owned executable behavior driver independently
compares original and recovered operations, but this is not an independent code
review or a proof for other assemblies. No third-party method body was executed.

| Method | Verdict | Observations |
|---|---|---|
| `ForeignHiddenToBase` | EQUIVALENT for the owned selected library | IL is `ldarg.0; ret`; output returns that same reference. `HiddenChild` implements covariant `IView<Hidden>` and the selected library's hidden `Hidden` derives from `VisibleBase`. The proof introduces no cast, allocation, null check or side effect. |
| `ForeignHiddenOpen<A,B>` | EQUIVALENT for the owned selected library | IL also returns its original input. The selected interface argument `HiddenBox<A>` derives from `VisibleGeneric<A>`; the proof retains the caller's `!!0` rather than `!!1`. No extra operation is emitted. |

The missing-library, invariant-interface, different-base, different-assembly,
wrong generic-position and class/value mutations remain Partial. Returning null
instead of the original input fails the executed behavior comparison for both
methods. The four real-input targeted scans contained no newly recovered bodies
relative to `596e59416`; their output is not presented as a gained-method review.
The earlier Roslyn +22 apparent gain was caused by comparing with a baseline
that predated the constructor group, not by this group.
