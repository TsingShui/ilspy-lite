# External type facts: scope question before implementation

Written while the runtime-type-check review was still running. No code was changed
for this note.

## Observation

Three of the largest remaining first blockers all need facts about types that are
not defined in the input image:

| First blocker (latest scan) | Count | Needs |
|---|---:|---|
| external enum storage (`value__` width) | 59,191 | the defining assembly's metadata |
| `callvirt` on an opaque external receiver | 39,220 | whether the target type is a class or interface, and its member signature |
| external/interface `castclass`/`isinst` target | part of ~14k | target identity and kind |
| constructed generic member binding | part of 60k | definition arity, member signatures, substitution |

The approved design (D2) already says to reuse the existing `DependencyContext`
binding rather than a second resolver, and that byte-only interfaces must stay
filesystem-free while dependency-aware decompilation and scanning receive the same
explicit immutable context.

## The blocker

Production `scan` is byte-only. The corpus acceptance run invokes it with just an
image path, so no reference set is supplied. Recovering any of the four capabilities
above therefore *cannot* move the corpus metric unless the scan itself is given the
frozen inputs as a dependency context.

That is a scope and architecture decision, not an implementation detail:

1. **Option A - scan gains an explicit context.** The acceptance tooling builds a
   `DependencyContext` from the frozen manifest (all managed images already hashed)
   and passes the same reference set to both `decompile` and `scan`. Pros: the four
   capabilities become measurable, and scan and method output stay identical as the
   design requires. Costs: memory and time per image (the manifest is ~1k images),
   and we must define which references an image is allowed to see so the metric
   cannot be gamed by injecting unrelated assemblies.
2. **Option B - keep scan byte-only.** Then these capabilities only improve
   `decompile` with explicit references, the corpus rate stays where it is, and the
   remaining path to <= 1% per assembly must come from things provable inside one
   image (EH, control flow, more local type/member shapes).
3. **Option C - hybrid.** Scan stays byte-only but gains a deterministic, manifest
   scoped "reference bundle" only for assemblies that the manifest itself declares
   as references of the scanned image, recorded per image in the acceptance output.

## What each option implies for the goal

Option B keeps the accounting simple but likely cannot reach <= 1% for assemblies
whose bodies are dominated by calls into their own dependencies.
Option A makes the metric reachable but changes the production scan contract.
Option C is A with a tighter, auditable reference set.

All three keep the existing guarantees: reference contracts stay excluded from the
denominator, no candidate is skipped by name, and missing or conflicting facts stay
visible rather than silently substituted.

## Implementation plan (verified against the source, not yet coded)

Touch points found by reading the current code:

- `rust/src/main.rs:69` — the argument validator accepts `scan` only when
  `args.len() == 2`, so reference options are currently rejected before they reach
  anything.
- `rust/src/main.rs:87` — the `scan` branch calls `ilspy_lite::scan_methods(&bytes,
  ...)`, which has no context parameter.
- `rust/src/lib.rs:38` — `pub fn scan_methods(bytes: &[u8], record)` parses the image
  itself and lowers each method through the byte-only entry point.

So the first commit is a plumbing commit with **no capability change**:

1. Identify the existing context-aware decompile entry point (the one the
   dependency/property tests already use) and thread an `Option<&DependencyContext>`
   through `scan_methods`, defaulting to the current byte-only behaviour so every
   existing caller and the frozen baseline stay byte-identical.
2. Let `scan` accept the same `--reference` / `--reference-dir` /
   `--no-auto-references` options as `decompile`, and print the selected reference
   identities (count + assembly names) on stderr next to the existing summary.
3. Wire the acceptance tool: for each manifest image, pass exactly the managed
   inputs that the manifest lists as its references, and store that list in the
   per-image scan record.
4. Acceptance test for the plumbing: scanning with references must produce the same
   status for every method as scanning without them while no capability consumes
   the context yet, and the recorded reference set must match the manifest. That
   proves the plumbing is inert and auditable before any capability depends on it.

Only after that commit does external enum width (the cheapest of the four) get its
own fixture, contract table, behaviour gate and independent review.

### Status

Steps 1-4 have landed as plumbing with no capability change; the evidence,
selection statistics and negative coverage are in
EXTERNAL_REFERENCES_PLUMBING.md. Lowering still decides admission from the image
bytes alone, so every fallback count is unchanged on purpose. The remaining work
is the four capabilities, starting with external enum storage.

## Decision (confirmed)

**Option C.** The user confirmed it on 2026-10-26.

Consequences to implement, in this order:

1. The acceptance tooling computes, per scanned image, the reference set declared
   by that image and recorded in the frozen manifest (plus admitted core
   contracts), and passes those bytes to the production scan as an explicit
   immutable context. No free filesystem search, no name-based discovery.
2. The scan result for each image records the selected reference identities next
   to its counts, so a reader can audit exactly which assemblies were visible.
3. `decompile` and `scan` must share one admission path: the same explicit
   context has to be able to produce the same status for the same method.
4. Only then do the four capabilities become implementable and measurable:
   external enum widths, opaque external `callvirt`, external/interface cast
   targets, constructed generic member binding. Each still gets its own fixture,
   contract table, behavior gate and independent review.
5. Unchanged guarantees: reference contracts stay out of the denominator, nothing
   is skipped by name, missing or conflicting facts stay visible rather than being
   silently substituted, and the per-image integer <= 1% gate is not relaxed.

This note is the design confirmation record for that decision; it is not itself
an implementation or a capability gain.

## Recommendation (superseded by the decision above)

Option C, recorded explicitly: the acceptance tool passes, per image, exactly the
managed inputs the frozen manifest already lists as that image's references (plus
core contracts), and the scan output records the selected reference identities next
to each image. Then external enum widths, opaque dispatch, external casts and
constructed generic members become implementable and measurable without weakening
the per-image gate.

This needs user confirmation before implementation, because it changes the
production scan interface and the evidence recorded per image.
