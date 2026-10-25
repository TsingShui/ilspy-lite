# OpenRA baseline before instance/reference lowering

This records corpus setup, not completion of the first delivery milestone.
No Rust production code was changed. Environment: macOS arm64, Rust/Cargo 1.98.0.
CLI executable SHA-256:

    28c03bbc12d03238eea4a500c65621df6d89241cde6605bac8b5e6a0ce301f67

Input hashes, release provenance, tokens and signatures are pinned in
[manifest.json](manifest.json). CLI hashes identify a particular local build;
they are not required to match when re-running with another toolchain.

## Observed results

| Measurement | Result |
|---|---:|
| OpenRA.Game.dll inventory | 5,259 MethodDefs |
| OpenRA.Mods.Common.dll inventory | 18,767 MethodDefs |
| Selected primary methods | 10 |
| Selected primary IL instructions decoded | 332 |
| Primary methods reported Decompiled | 0 |
| Primary methods reported Partial, with full decoded IL fallback | 10 |
| Primary methods reported Failed | 0 |
| Support methods | 3, all Partial |
| Out-of-scope sentinels | 2, both Partial |
| Behavioral executions of OpenRA | 0 |

The inventory counts are not body-decoding or decompilation coverage counts.
Only the 15 explicitly selected primary/support/sentinel bodies were probed.

All primary methods hit the combined instance/implementation-flags fallback gate
before their body could be lowered. Inspection of their IL shows zero impl flags:
the immediate blocker is the unsupported instance receiver, not synchronized or
native methods. Arrays, conversions, managed element references and generic
MemberRef calls are additional blockers visible in the later-stage bodies.

## Commands and observed exit status

From the repository root:

| Command | Exit | Interpretation |
|---|---:|---|
| cargo build --release --locked (in rust/) | 0 | CLI builds |
| python3 rust/acceptance/openra/probe.py | 0 | Pinned inputs and output accounting verified; not semantic success |
| python3 rust/acceptance/openra/probe.py --require-stage 1 | 2 | First delivery still unmet: all four methods and supporting getter fall back |
| probe with missing corpus directory | 1 | Missing input is not silently skipped |
| probe with modified DLL in an isolated temp directory | 1 | SHA-256 mismatch rejected before analysis |

The default probe verifies exact token/signature identity, inventory counts,
primary instruction counts, raw-byte evidence, every fallback IL line and
consistency of stderr counters with process exit status. It retains generated
IL and C#-like outputs under ignored `rust/report/openra/current/`.

The four selected source files match both the official release source archive
and the pinned commit's raw files. No target code was run, no game assets were
downloaded by the game, no .NET 6/11 installation was needed, and no source-level
or behavioral-equivalence gate is claimed. First-stage behavior checks are
specified in the manifest but remain to be implemented.

## Milestone boundary

Stage 1: resources total, resources credit, resources debit and full-ammo check,
plus the one count getter they call. Finish and validate these, then stop and
report. Stages 2/3 and EH/iterator sentinels do not authorize scope expansion.
