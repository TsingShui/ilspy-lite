# Unordered float comparisons

The CLI defines the `.un` comparison forms on `F` operands as unordered-aware:
`cgt.un` is true when the first operand is greater *or* either operand is NaN,
i.e. `!(a <= b)`, and `clt.un` is `!(a >= b)`. Roslyn emits `cgt.un`/`clt.un`
for `<=`/`>=` on floats (followed by `ceq`), so this path is reachable from
ordinary C#.

Previously these forms were lowered as unsigned integer comparisons of the float
values, which compares representations rather than values and can never be
correct for NaN or infinities. The float branch now emits the negated opposite
ordered relation, and the branch forms use the same rule (`bgt.un`, `bge.un`,
`ble.un`, `blt.un`, `bne.un`, `beq`). Ordered forms (`ceq`, `cgt`, `clt`, `bgt`,
`blt`) are unchanged, and integer `.un` comparisons keep their unsigned casts.

## Evidence

- Owned red/green: `RelationCases.Ops.FloatLe`, `.FloatGe`, `.FloatNotLe` were
  Partial before the change ("left: Partial") and are Decompiled after it, with
  the emitted text asserted to use the unordered relation and not an integer
  cast (`tests/reference_relations.rs`).
- The relation behavior driver gained float candidates including NaN, `-0` and
  both infinities: 379 comparisons / 47 closed selections per configuration, and
  it rejects nine mutants, including `FloatLe` returning a constant.
- 472 Rust tests, fmt, clippy and the release build pass. No third-party body was
  executed.
- Targeted static scans: System.Private.CoreLib 13,425 -> 13,431 (+6), mcs System
  14,038 -> 14,039 (+1). The group is primarily a correctness fix; a full batch
  scan is due with the next groups.
