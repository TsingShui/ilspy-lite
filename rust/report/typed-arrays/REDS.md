# Typed vector red/green record

Public production decompile output was observed Partial before each slice.
The following diagnostics are excerpts; complete local logs remain at the named
paths, rather than being represented as complete logs here.

| Local retained red log | Observed fallback |
|---|---|
| length-red.log | expected int[] or uint[] evaluation-stack value |
| reference-load-red.log | unsupported opcode ldelem.ref at IL_0002 |
| reference-store-red.log | unsupported opcode stelem.ref at IL_0003 |
| allocation-red.log | array element type needs IL fallback |
| tokenized-red.log | unsupported opcode ldelem at IL_0002 |
| address-red.log | array element type needs IL fallback |
| scalar-red.log | unsupported opcode ldelem.u1 at IL_0002 |
| two-byte-red.log | unsupported opcode ldelem.i2 at IL_0002 |

Green evidence: corresponding *-green.log files, focused scope/element boundary
logs, regression-final.log (443 pass), and actual default-output execution in
behavior-manifest.json (34 drivers, all exit 0).

The first consolidated regression had six frozen-promotion failures (434/440).
Promoted legacy cases executed before their rows were removed; logs named
legacy-*-execution.log and the final family logs retain this evidence.

Authoring errors (an omitted owned fixture rebuild, a Python regex escape, an
overbroad constructor selector and a carrier array alias) are not semantic reds.
The first Debug late-acquisition mutant did not compile because a ref binding
preceded its moved declaration; after moving both lines, the compiled mutant
failed the intended acquisition/trace observation. No compiler failure is
counted as a semantic mutant rejection.
