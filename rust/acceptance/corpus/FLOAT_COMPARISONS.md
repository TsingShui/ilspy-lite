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

## Binary64 (double) support

`binary64` is now a first-class stack/storage type: signature `R8`, `ldc.r8` with
bit-exact payloads, `ldind/stind.r8`, `conv.r8` from integers and floats,
`conv.r4` rounding a binary64 value to binary32, and `div` with its own
`__cil_div_F8` helper. Float operands share one stack category, so `expect` now
accepts equal float types and refuses mixed ones instead of demanding integers;
`neg` is allowed for floats, `not` and the `.un` division forms stay refused.
Loads, stores, call arguments and returns of binary64 need no rounding helper,
because the storage and evaluation widths agree.

Owned red/green: `DoubleSum`, `DoubleLess`, `DoubleStore`, `DoubleFromInt`,
`DoubleToFloat` were Partial before the change and Decompiled after it. The float
fixture's frozen `UnsupportedDouble` negative became `RecoveredDouble`; a new
negative `UnsupportedDoubleUnbox` keeps a double-specific boundary visible
(`unbox.any double` still needs an admitted boxed scalar), and the
`conv.r.un`/unsigned-source negative is unchanged. The relation driver executes
528 comparisons / 52 closed selections per configuration with double NaN, `-0`
and both infinities, and rejects ten mutants including `DoubleSum`.

473 Rust tests, fmt, clippy and the release build pass. Targeted scans:
FSharp.Core 7,177 -> 7,276 (+99), System.Private.CoreLib 13,431 -> 13,607 (+176).
A full batch scan is due with the next group.

## Unsigned integer to float conversion

`conv.r.un` reads its integer source unsigned and produces CLI `F` with
round-to-nearest. The lift now emits a width-specific helper (`__cil_conv_r_u4`
for an `I4`-carried source, `__cil_conv_r_u8` for `I8`) and pushes binary64, so a
following `conv.r4` performs the binary32 rounding exactly as the IL requests.
Native-width unsigned sources remain refused with their own reason.

Owned evidence: the float fixture's `UnsupportedUnsigned` became
`RecoveredUnsigned` (red before the change, green after), and a new negative
`UnsupportedUnsignedNative` keeps the native-source boundary visible. 474 Rust
tests, fmt and clippy pass. The 49 first-blocker occurrences in the last full scan
make this a small group; the full batch scan lands with the next groups.
