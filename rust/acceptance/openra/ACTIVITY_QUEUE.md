# Independent class 1: Activity.Queue

The frozen Activity class and Queue MethodDef `0x06000F46` are unchanged from
[classes.json](classes.json). Queue is recovered by general reference truth
branch lowering plus the existing field/CFG rules; no Activity name/token occurs
in production admission or lowering.

```sh
python3 rust/tests/reference-roundtrip.py --activity --from-class
```

The actual public class output includes Activity and its four recursively owned
generated types. Complete original type/field/method/property identities,
metadata and status accounting match the independent PEReader snapshot. Queue
is Decompiled; unrelated lifecycle/iterator/generic bodies retain visible
fallback. The class still exits 2, honestly reflecting those other bodies.

The behavior gate extracts Queue from public class output and executes it on a
separate metadata-derived state shell. The original side makes an exact IL call
on a minimal concrete Activity subtype with no behavior overrides. No Activity
constructor, lifecycle, virtual override, game or original Queue proxy is invoked
by the recovered side. Setup retains original private base fields through their
exact declaring-type reflection identity.

Finite chains have 0, 1 or 3 existing successors. Appends are null or distinct
objects with 0, 1 or 3 prelinked successors. After each call, every original link
and mapped recovered link is checked by reference identity; existing nodes and
appended suffix links cannot be replaced or modified. All other original fields
remain unchanged. Null receiver faults and append-state preservation also match.
**20 return/exception/state comparisons passed** on local 64-bit .NET 10.0.12.

Cyclic/nonterminating chains are not executed or timed out as semantic evidence.
The output's repeated field read and back edge remain visible. These finite
inputs do not prove all-input equivalence or concurrency behavior.

Owned public regressions validate brtrue/brfalse with reference/null operands,
without dereferencing or overloaded equality, while unsupported address-taking
remains fallback. Relevant Debug/Release Rust tests, fmt, strict clippy and the
522-comparison/configuration existing reference behavior gate passed.

Logs and GPL-derived class/body artifacts remain in ignored
`rust/report/openra/activity/`. Existing input provenance is reused without
rehashing. This same-project independent case is not broad ecosystem coverage.
Wait.Tick and output-only Agent reading remain required for delivery.
