# External reference plumbing (option C, first increment)

Decision: `EXTERNAL_TYPE_FACTS.md` records **option C** - each image is scanned with
exactly the references it declares, and the selection is recorded per image. This
increment lands the plumbing only; no capability consumes dependency facts yet, so
the fallback numbers must not move. They do not.

## What landed

- `ilspy-lite scan <assembly> [reference options]` now accepts the same
  `--reference`, `--reference-dir`, `--no-auto-references` and
  `--reference-version-policy` options as `decompile`. With any of them it prints
  `references=<declared> resolved=<n> failed=<n>` plus one
  `reference=<token> <identity>` line per declared reference on stderr.
- `ilspy-lite identities <assembly>...` prints one JSON line per image with the
  image's own identity and the reference identities it declares. Both come from the
  image alone; nothing outside the file is read.
- `corpus.py baseline --references declared` builds one identity index over the
  managed inputs with that command, resolves every declared reference to exactly
  one corpus file, passes those files with `--no-auto-references`, and stores
  `declared`/`selected`/`unresolved`/`ambiguous` plus the CLI's own resolution
  counts in the per-image result. The reference selection is part of the resume
  stamp, so a changed selection cannot reuse a stale per-image summary.
- `--reference-version-policy same-major` is available for selection and is
  recorded per image; the default stays `exact`.

Selection never guesses: an identity that matches several corpus files is counted
as `ambiguous` and passed to nobody, and an identity that matches none stays
`unresolved`. Both are visible in the per-image record.

## Selection statistics over the fixed inventory

| quantity | value |
|---|---:|
| managed images | 614 |
| images with a usable identity | 610 (4 are netmodules without an Assembly row) |
| distinct identities | 496 |
| declared references | 3,010 |
| resolved to exactly one file | 1,179 |
| unresolved (no such identity in the corpus) | 1,399 |
| ambiguous (several files share the identity) | 432 |
| images with at least one selected reference | 313 |
| identities owned by more than one file | 87 |

## Evidence that the plumbing is inert

Full production scan with `--references declared`
(`report/external-facts/corpus-scan/baseline.json`), compared with the previous
milestone baseline (`report/cast-roundtrip/corpus-scan/baseline.json`,
`report/external-facts/plumbing-delta.json`):

- Decompiled 264,677, Partial 506,071, bodies 770,748, unrecovered 65.6597%:
  identical to the previous scan.
- `gained_images: 0`, no image changed a single status.
- OpenRA.Game 1,520 recovered and OpenRA.Mods.Common 5,974 recovered, unchanged.
- Gate run: `complete=true`, `pass=false` (complete because every body is
  accounted for, not passing because 33/319 body images are within 1%).

## Negative coverage

- `scan <assembly> --not-an-option` fails instead of silently ignoring the option.
- `scan ... --reference-dir <absent>` completes and prints
  `reference directory unavailable:`, so a missing input stays visible.
- `identities <non-assembly>` reports a per-file `error` and does not claim the
  whole run failed.
- A status-parity test runs the same image with and without a reference set and
  requires byte-identical NDJSON, which is what makes this increment provably
  behaviour-preserving rather than merely "not obviously" so.

## Not in this increment

Lowering still decides admission from the image bytes alone; the dependency
context reaches the scan but nothing reads it. The four capabilities that will
read it - external enum storage, opaque external `callvirt`, external/interface
cast targets, constructed generic member binding - each keep their own fixture,
contract table, behaviour gate and independent review.
