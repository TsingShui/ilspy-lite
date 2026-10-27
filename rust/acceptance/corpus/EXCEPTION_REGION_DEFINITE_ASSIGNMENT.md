# Definite assignment across exception regions

Any body with `initlocals=false` **and** an exception handler was refused wholesale
(`uninitialized local exception-region analysis is not implemented`), so methods
whose locals were assigned before the protected region were rejected even though no
read could observe an uninitialized slot (537 in `System.Private.CoreLib` alone).

## Analysis

`require_definite_local_assignment` is now a two-phase dataflow over the CFG:

1. **States.** Each block entry carries a may-be-unassigned bit set; the entry block
   starts with every local unassigned and `stloc` clears its bit. Normal successor
   edges OR the predecessor's exit state. A handler block is seeded with the
   **union of the states of every block whose offset lies in its protected range**,
   because an exception can leave the region at any instruction; the seed is
   iterated into the fixed point (a raised seed re-propagates from the handler).
2. **Check.** With the converged states, each block's instructions are scanned in
   order: reading a local whose bit is set (`ldloc`/`ldloca`) is refused with
   `local may be used before definite assignment`.

The seeding is deliberately conservative: an assignment that happens after a
possible throw does not make the local definite for a handler, which is exactly the
rule C# applies before a local may be read in a handler or `finally`. `ldloca`
still counts as a use, so address-taking of a maybe-unassigned local stays refused.

## Evidence

- Promotions are explicit: `RelationCases.LocalInitNegative.ProtectedRegion`
  (assignment before the `try`) now decompiles and is asserted as `Decompiled`; the
  old expected-partial entry and its blanket reason were removed.
- A new mutation keeps the boundary: patching the method's `stloc.0` to `pop` drops
  the assignment and must still be refused with
  `local may be used before definite assignment` (the test asserts both the status
  and the reason).
- `System.Private.CoreLib` 15,739 -> 16,082 (+343). The fixed-corpus batch together
  with the local-core-scalar mapping: **720,202 / 50,546**, +2,437 in 29 images,
  zero regressions, zero incomplete.
- 488 Rust tests, `cargo fmt --check` and clippy `-D warnings` pass; the exception
  behavior driver still reports 51 comparisons and 10 recovered methods with its
  mutation control.
