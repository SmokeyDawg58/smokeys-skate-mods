# World line-query runtime integration

2026-09-08, accepted checkpoint **47724/092bf0**, original final
**93229/a48538**. Independent source, production, tests,
shared static-owner integration and corpus review PASS. The full playable
goal remains ACTIVE. This completes the actual GroundPool child query;
default World routing still requires actual DMO/conditional vehicle owners.

## Connected ground path

`StaticGroundPool::try_run_line_test` borrows the same dense published slots,
original mesh bytes and retained bounds used by ground collision. Each slot
retains the source cofactor inverse of its actual identity placement. The
ordered collection scan transforms the query AABB even for identity, performs
source ordered culling, traverses KD ranges, lazily decodes reached units and
vertices, constructs complete modern GP primitives and runs thin triangle
intersection plus normal calculation. The local job publishes only on normal
return. Source Ground AddVolume supplies null userPtr, represented by None.

The cursor preserves ceil-half chunks, range/resume state and late failure
prefixes. Initial eager cluster parsing was rejected during review because
unused normals/vertices/tail bytes could fail before the selected primitive.
The corrected borrowed view reads only reached fields; five added regressions
bring cursor coverage from ten to fifteen. Existing compressed/uncompressed
vertex decoding is reused, with no alternate whole-cluster authority.

Modern GP keeps full position and normal W beside existing XYZ storage. Thin
intersection preserves determinant tolerances, ordered NaN branches and late
numerator writes. Its wrapper normal calculation still runs before nearest
selection, but the stored winning normal comes from GP. Strict clamped-parameter
comparison keeps the first encountered tie, while position retains the
unclamped parameter. Every hit increments the wrapping count. There is no
ClipEnd, sort, deduplication or substitute spatial index.

One explicit host numerical service supplies both modern GP and triangle
testing. The existing Reckoning dot graph was extracted unchanged; existing
ground-physics numeric callers retain their prior graph. Host reduction,
estimate/denormal and exceptional-sign behavior is not a captured Xenon trace.
Nonzero/NaN combined fatness refuses at the selected unsupported branch.

## Validation and limitations

Added 61 units: triangle18, cursor15, modern GP/full-W5, mesh15,
collection7 and shared numerical-policy1. Workspace **29948/6c3e8a** passed
**3,113 units** (2,870 core + 202 formats + 38 app + 3 symbols), six docs.
Normal checks retain core9/app1 ignored. Initial Clippy18076/c3766d exposed
unused pending-live entries, documentation markup and exact finite fixture
comparison lints. Explicit boundary annotations and test style corrections
changed no production arithmetic; final47724/092bf0 all-target -Dwarnings PASS.
Final93229/a48538 fmt/diff and all nine original core tests PASS.

The original repeated DMO/Rob's Park corpus now queries its already admitted
ground owner. An existing independent simplified geometry implementation
checks XYZ, interior parameter and surface133 within declared tolerances.
Observed native-order occurrence is slot0/cluster18/unit351/primitive0,
hitcount1, parameter bits3f00000f and position XYZ
44133726/c259b959/4479204f. Normal XYZ observed 0/3f800000/0; its arithmetic is
independently covered in leaf tests rather than asserted by this corpus oracle.
An initial W=5 midpoint assumption failed: the authored surface is slightly
below the nominal probe centre. Reviewer-approved correction checks interior
t, exact full-lane propagation `4*t+3` using FMA and independent reference W
tolerance. Observed W40a00008 is host regression evidence, not native trace.
Original5015/02583d and final93229/a48538 both passed all nine checks.

## Next connected owners

Ground and DynamicPool constructors select the no-spatial path. The generic
octree survey is separately recovered but does not prove a live DMO index.
Actual DynamicPool BeginUpdate must synchronize its four real batch owners,
clear count, visit actual members/Parts, read volume BEFORE live Part transform,
traverse original SMA occurrences with lazy disabled-volume cvar reads, and
retain count-before-Reset prefixes. Filling the last mesh slot retains it and
skips the inherited serialized refresh tail. Removal synchronizes, performs
base removal, then repeatedly swap-removes matching mesh owners.

Generic DMO admission is missing from the existing source-bound staged Part
session. The next implementation connects source-arena inertia and full packed
Part transforms to the SAME manager backend, then actual candidate/Insert/
Create/member ownership and mesh publication. Keep Assembly world ID, dense
ordinal, model ObjectId and Island BodyID distinct. Current image/resource
bases remain asserted host inputs; real allocator provenance is still open.
No absent DMO/vehicle owner may be represented as an empty successful route.

Related reviewed source: [geometry](skeleton-ik-world-line-geometry-source.md),
[owners](skeleton-ik-world-line-owners-source.md),
[IK suffix](skeleton-ik-update-suffix-source.md),
[DMO admission](dmo-dynamic-pool-admission-source.md), and
[backend bridge](dmo-shared-backend-bridge-source.md). The last packet's new
source review and bridge Rust implementation are separate from this accepted
ground checkpoint. Active IK still stops at UpdateIkContact; Actor/input/
history/combos and portable window/GPU/camera remain required.
