# Per-assembly view and slice order

> Historical analysis. The active goal requires the **per-sample** gate again:
> every body image must satisfy `100 * unrecovered <= bodies`; currently **92/319**
> pass at the 730,866/39,882 scan (89/319 at the 728,704 anchor). The 33/319 and
> median 60.4% below are from the old boxing baseline. Current per-image counts are
> in `report/current-head/implementation-scan`; current state is in `CHECKPOINT.md`.
> The aggregate 1% is diagnostic only.

Measured from `report/boxing/corpus-scan/baseline.json` (the scan before local-enum
boxing; enum boxing only removes methods, it does not change which shapes block).

## Where the goal actually stands

- 33 of 319 body-bearing images are within 1%; the median image is at 60.4%
  unrecovered. The gate is per assembly, so global totals are not the target.
- Reaching 1% per assembly therefore needs the *shapes that dominate the largest
  images*, not the globally largest buckets: the biggest single bucket can be a tiny
  share of one image and a third of another.

## Blockers inside the four largest images

Counts are first-blocker reasons within that image.

| image (bodies) | dominant blockers |
|---|---|
| `Microsoft.CodeAnalysis.CSharp` 37,159 | constructed value type needs a local definition / external instantiation 5,890; enum storage needs an exact local definition 5,133; constructed member parent not concrete 1,764; allocation owner 1,575; constructed base 1,316; byref/modified parameter 1,296 |
| `Microsoft.CodeAnalysis.VisualBasic` 33,022 | same order: 5,080 / 4,758 / 1,885 / 986 / 899 |
| `Microsoft.CodeAnalysis.CSharp` 30,106 | 3,265 / 1,853 opaque external callvirt / 1,615 allocation owner / 1,596 constructed member parent |
| `Microsoft.CodeAnalysis.VisualBasic` 32,181 | 2,005 allocation owner / 1,879 opaque external callvirt / 1,135 incompatible value-receiver / 1,135 constructed member parent not an instantiation |
| `mscorlib` 23,889 | 1,950 throw needs an admitted exception reference / 1,330 interface declaring type / 1,178 exception and data sections / 884 byref parameter / 796 byref-like struct copies |

## Slice order this implies

1. **External constructed value and enum signature facts** (one capability): admit a
   `GenericInst` value type and an enum whose *definition* lives in a referenced image,
   taking the name from that image. This is the single largest bucket in the two worst
   images and the same machinery serves both rows. Requires the cross-image type
   naming that the external-field slice is already introducing.
2. **Allocation owner / constructed member parent for concrete external classes**:
   separate from construction, it only needs the parent's kind and name.
3. **Opaque external callvirt** (dispatch facts) - already designed in
   `EXTERNAL_CALL_DISPATCH.md`; the remaining rows are receivers whose type the call
   does not name, so they need a receiver-type fact rather than a member fact.
4. **byref/modified parameters** - uniform across the large images, roughly 900-1,300
   each.
5. **Exception and data sections** - the change that moves `mscorlib` and the
   exception-heavy images; larger design, do it once the type shapes are in.
6. **Interface declaring types** - needs interface dispatch facts; interacts with 3.

Items 1-2 share the naming work and should land back to back; 3 depends on 1 only for
naming the receiver's type.

## Breadth: which capability moves the most images

Same scan, grouped by the *first* blocker prefix; "images" counts images where the
bucket appears, `medshare` is the median share of that image's unrecovered bodies.

| images | medshare | bucket |
|---:|---:|---|
| 50 | 10.2% | exception/data sections need IL fallback |
| 45 | 11.4% | callvirt on an opaque external receiver needs dependency facts |
| 44 | 7.3% | external allocation needs dependency facts |
| 24 | 6.1% | enum storage requires an exact local definition |
| 21 | 4.2% | interface declaring type needs IL fallback |
| 12 | 4.5% | constructed value type needs a local definition |
| 10 | 3.6% | allocation owner must be a concrete class |
| 9 | 15.2% | uninitialized local analysis is not implemented |

Reading both tables together:

- **Exception and data sections** is the broadest blocker (50 images) and also large
  inside `mscorlib`, so it is worth its design cost; it is the only bucket that is
  simultaneously broad and deep.
- **Opaque external callvirt** (45 images, 11.4% median) and **external allocation**
  (44 images, 7.3%) are broad and already designed; they should land before EH because
  they are cheap relative to their coverage.
- **Enum storage in signatures** is 24 images, but it is the same cross-image naming
  work as the constructed-value slice, so it rides along.
- **Uninitialized local analysis** is only 9 images but 15% inside them: a
  self-contained analysis, good filler between big slices.

Revised order after the current external-field slice:

1. External constructed value and external enum signature facts (in flight as a brief).
2. External callvirt receiver type facts (45 images; design in `EXTERNAL_CALL_DISPATCH.md`).
3. External allocation with a concrete external owner (44 images; shares the owner
   kind/name work with 2).
4. byref/modified parameters and uninitialized local analysis as fillers.
5. Exception and data sections (50 images, the largest design).
6. Interface declaring types, then the long tail toward per-assembly 1%.

## The opaque-callvirt row is three different problems

Exact reason strings over the first 60 body images:

| count | cause |
|---:|---|
| 2,528 | `type binding requires a resolved assembly: <identity>` (ResourceManager 1,608; mscorlib 557; System.Runtime 363) |
| 1,500 | `external call target is a special-name accessor` |
| 1,073 | `external call target is declared on an interface` |
| 739 | opaque receiver with no further detail |

The first cause is not a decompiler limitation but a reference-set one: the declared
reference names a facade (`System.Resources.ResourceManager`, `System.Runtime`) or a
different version of `mscorlib` that the frozen selection did not resolve. Two
concrete follow-ups, both cheap:

- retry the scan with `--reference-version-policy same-major` now that external value
  and enum facts consume cross-image signatures (the earlier zero-gain test predates
  them); adopt only if the per-token parity check stays clean;
- make sure reference-only inputs serve as fact sources even though they are excluded
  from the implementation denominator: being skipped as a target must not make an
  image unusable as evidence.

The second cause is a self-contained slice: bind `get_`/`set_`/`add_`/`remove_`
accessors to property and event syntax (1,500 in 60 images). It is the cheapest large
row in the whole scan and should go before interface dispatch.

## Correction: a blanket gate on `.cctor` is uniform overhead in every image

`empty/static-initializer/runtime-special method needs IL fallback` is not a tail row.
It covers **13,335 bodies in 236 of 319 images** - about 2.1% of every large image
(2.10% CodeAnalysis.CSharp, 2.20% VisualBasic, 2.31% OpenRA.Mods.Common 426/18,407),
because it is a blanket refusal of static constructors, not a diagnosis of anything the
IL contains.

That single gate is larger than every capability bucket in the scan, and because the
gate is per assembly at 1%, the corpus cannot pass while it exists: any image with
`n` bodies and no other defects is still above 1% purely from its `.cctor`s.

Priority is therefore: static constructors first, then the previously planned slices.
The work is not "render a stub": the `.cctor` body must be lowered like any other void
body, keeping store order, one-time execution at type initialization, and leaving any
genuinely unsupported operation inside it to fail with its own reason.

### What is actually inside those `.cctor` bodies

Random sample of 20 blocked `.cctor`s across images: median 5 IL operations (max 100);
19/20 contain `stsfld`, 18/20 contain `newobj`, 7/20 `ldsfld`, 4/20 `call`, 3/20
`newarr`, 2/20 `ldftn`.

So a static constructor is typically "allocate one object or array, store it in a static
field". Removing the blanket gate therefore does not finish the job by itself: the
stored value's allocation becomes the next blocker for most of them, and that is the
`allocation owner must be a concrete class` / `external allocation needs dependency
facts` row. The `.cctor` slice still comes first, because it converts one uniform 2.1%
blanket row into real, countable reasons - and small `.cctor`s that only store constants
or local allocations should recover immediately.

### Correction to the correction: `.cctor` is a large slice, not a gate unlock

Recomputing with the `.cctor` rows removed entirely (as if every one recovered) leaves
the passing count at **33/319**, unchanged. Every failing image also has far larger
gaps, so no single bucket moves the gate until an image is close.

The right dashboard is *gate distance*: for each image, the number of bodies that must
still be recovered is `unrecovered - bodies/100`.

| quantity | value |
|---|---:|
| total recoveries needed for every image to pass | **466,031** |
| images needing <= 10 more | 71 |
| images needing <= 50 more | 111 |
| images needing <= 200 more | 176 |
| largest gap | 22,960 (`Microsoft.CodeAnalysis.CSharp`, 23,331 of 37,159 bad) |

So progress is measured two ways: absolute recovered bodies (how far the whole corpus
moves) and gate distance per image (how close each assembly is to passing). Pass counts
are a lagging indicator and will move late, in waves, as images cross from "needs a few"
to "passes".

`.cctor` stays first because it is 13,335 bodies (2.1% of every large image, 1.7% of the
corpus) and it is uniform, but it is a volume win, not a gate unlock.
