# Closure gate (provisional)

> Historical: this closure report is from the boxing baseline (33/319, 473,770
> unrecovered). The active goal is per-sample 1%; current per-image counts are in
> `report/current-head/implementation-scan` (92/319 at 730,866/39,882; 89/319 at
> the 728,704 anchor). The aggregate 1% is diagnostic, not the acceptance.

Sourced from `report/boxing/corpus-scan/baseline.json` (before local-enum boxing).
Integers only: pass iff `100 * unrecovered <= bodies`. Re-check with
`python3 report/per-assembly-gate.py <baseline.json>`.

- body images: **319**, passing <=1%: **33**
- failing images: **286**; of those with <=20 unrecovered bodies: **49**
- absolute unrecovered bodies still to clear: **473,770**

## Failing images with the fewest remaining bodies

| rate | unrecovered/bodies | assembly |
|---:|---|---|
| 100.000% | 1/1 | csi |
| 100.000% | 1/1 | csi |
| 100.000% | 1/1 | csi |
| 50.000% | 1/2 | cs20-SecurityAction |
| 50.000% | 1/2 | nullable-parameter |
| 50.000% | 1/2 | switch |
| 50.000% | 1/2 | nunitlite-runner |
| 50.000% | 1/2 | nunitlite-runner |
| 50.000% | 1/2 | nunitlite-runner |
| 33.333% | 1/3 | System.Xml.XPath.XDocument |
| 20.000% | 1/5 | catch |
| 2.381% | 1/42 | mock-assembly |

## Failing images by rate (top)

| rate | unrecovered/bodies | assembly |
|---:|---|---|
| 100.000% | 3/3 | PortableClassLibrary |
| 100.000% | 3/3 | RetargetableExample |
| 100.000% | 3/3 | varargs |
| 100.000% | 6/6 | System.Diagnostics.StackTrace |
| 100.000% | 101/101 | TestCase-1 |
| 100.000% | 1/1 | csi |
| 100.000% | 1/1 | csi |
| 100.000% | 1/1 | csi |
| 99.466% | 4656/4681 | ExplicitConversions_32 |
| 99.008% | 14472/14617 | ExplicitConversions |
| 97.436% | 38/39 | System.Runtime.CompilerServices.Unsafe |
| 97.222% | 35/36 | System.Runtime.CompilerServices.Unsafe |

## Reading

The tail is what decides the gate: the smallest failing images (a handful of bodies
each) are cheap to close, while the largest need whole capabilities. Slice order
therefore alternates: land a capability, then sweep the small images it unblocks.

## The 49 small failing images and what they need

Each has 1-20 unrecovered bodies, so any single blocker decides whether it passes.
Their blocker mix (counts are occurrences, an image can need several):

| occurrences | blocker |
|---:|---|
| 39 | exception/data sections need IL fallback |
| 34 | external allocation needs dependency facts |
| 21 | enum storage requires an exact local definition |
| 18 | callvirt on an opaque external receiver needs dependency facts |
| 18 | array element type needs IL fallback |
| 15 | empty/static-initializer/runtime-special method needs IL fallback |
| 11 | external field needs IL fallback |
| 7 | incompatible value/receiver types |

Consequences for ordering:

- The tail cannot be swept early: **39 of 49** need exception handling, so EH is not
  only the broadest big-image bucket, it is the gate on the cheap tail as well.
- External allocation (34) and enum storage (21) land before EH anyway, and will shrink
  the tail first; whatever remains after them is dominated by EH.
- `array element type needs IL fallback` and `empty/static-initializer/runtime-special`
  are two small, self-contained rows that can be cleared independently and will close
  several 1-body images.

Revised order: external constructed values and enum signatures -> accessors ->
external callvirt receivers -> external allocation -> small rows (array element type,
empty/runtime-special methods) -> EH -> interface declaring types.

## The near-passing wave (within 10 recoveries)

38 images are at most 10 bodies away from 1%; between them they have 131 remaining rows.
Their composition decides which capability flips the most assemblies at once:

| rows | blocker |
|---:|---|
| 27 | external allocation needs dependency facts |
| 16 | exception/data sections need IL fallback |
| 12 | array element type needs IL fallback |
| 10 | empty/static-initializer/runtime-special method (`.cctor`) |
| 10 | callvirt on an opaque external receiver |
| 8 | enum storage requires an exact local definition |
| 8 | external field needs IL fallback |
| 7 | incompatible value/receiver types |
| 6 | constructed value type needs a local definition |
| 4 | unsupported opcode `ldtoken` |

So the fastest gate-image wins are: external allocation (27 of 131 rows), then EH (16),
then the self-contained `array element type` (12) and `.cctor` (10) rows. This is why
external allocation moves ahead of the larger constructed-value rows for the *pass count*,
even though constructed values unlock more bodies in total.
