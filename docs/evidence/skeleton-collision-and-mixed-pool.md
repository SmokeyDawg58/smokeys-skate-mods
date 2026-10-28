# Skeleton collision publication and shared SkaterPool

Follow-on: [retained-pose placement](skeleton-retained-pose-placement.md) now
places the actual 24 skeleton Parts before refresh. The unplaced-skater test
boundary below records this earlier checkpoint.

2026-09-07. Confidence: **high** for source control flow and ordinary host
bounds arithmetic; **exact** for copied Volume words, Part mapping and flag
selection. Extends [live normal setup](skeleton-live-normal-setup.md).
Validation results are recorded below.

## Evidence and native ordering

The symbolized `sk82_na_m.xex.i64` remains external. Read-only IDA source:

| Native function | Address | Relevant behavior |
|---|---|---|
| `SerialisedAssembly::Create` | `82e0c850` | Flatten actual Part geometry, retain inline Volume snapshots and body mapping |
| `SerialisedAssembly::Update` | `82e0cf30` | Refresh current flags, transforms, materials, groups and body state |
| `CreateHatVolume` | `82b4fae8` | SimpleMappedArray with Cylinder then Sphere |
| AssemblyPool Insert / Remove | `82e0dbb0` / `82e0c1c8` | Shared index, actual attachment, publication; detach before swap-last |
| Pool bounds job / EndUpdate | `827cdf78` / `82e0bb10` | All bounds before publication and live active scan |
| Skeleton Init admission | `82b6213c` | Admission precedes spies, root zero material and normal setup |
| Skeleton SetUpNormal | `82b50508` | Mutates actual retained state and inertia after admission |

Read-only full-function SHA256 evidence:

```text
Create 82e0c850..82e0cf2c
c29dbd5dc8928a10c82c41e4fbffcf340cc6554689703342db074b35280283e2
Update 82e0cf30..82e0d35c
113a7064f503b753d0f45bd36d4102a000b2730381c1a9862b4873121f64aa91
CreateHatVolume 82b4fae8..82b4ffd4
fd6a093fda92509d7a1c09d8f9e8e134edcdbe70e5e103a48436cd48945831cb
```

Update clears `numStatic` at `82e0cf4c`, tests each Part body state bit0 at
`82e0cfa0`, and increments the count at `82e0cfac`. It preserves body/Volume
counts, not the static count. Header group/culling are sampled at
`82e0d2d0`/`82e0d2d8`. The normal 24x24 self-culling table retains its original
resource; this is distinct from ground pipeline culling.

## Actual publication

`SkeletonCollisionPublication` reads private attached Part handles through the
complete `SkeletonNormalAssembly` owner. It retains the exact session template,
binding and all 24 allocation-qualified body observations. No synthetic arena,
packed inertia address or replacement session establishes ownership.

Ordinary Parts emit their real primitive. A hat emits its two actual children
in Cylinder/Sphere order, both mapped to Part1 and the same body. No outer
Aggregate record exists: 24 bodies / 24 records without hat, 24 bodies / 25
records with hat, within the source 26-record cap.

Create's ordinary primitive inline copy overlays the live top-level flags and
Volume group. A hat child retains its own Volume words instead. Create reads
child flags at `82e0ca8c`, Update at `82e0d0a4`; neither combines them with the
outer Aggregate enabled flag. NoCollision disabling the outer hat therefore
does not disable its serialized Sphere child. No extra outer transform is
applied. Material, userdata group, body state and Part transform still come
from the parent Part.

Update changes enabled, transform, current material, userdata group and frozen
state, preserving Create's inline geometry/header words, surface, padding and
Part/body mapping. Source padding remains the existing 0.05 constant. Bounds
reuse the accepted primitive/swept kernels and include disabled, frozen and
static records. Create unions each calculated primitive immediately; the
later pool job stages the complete separate bounds array first.

The established authored boundary remains explicit: semantic body XYZ with
canonical affine W, zero motion-link W, real force/kine/torque/cool, and one
immutable snapshot per Part rather than concurrent native repeated reads.
This is not a packed guest-memory reproduction or a per-tick parity trace.

## One correlated pool owner

`SimulationSkaterAssemblyPool` now retains a closed Board/Skeleton member
variant. Each variant owns its matching publication and whole candidate.
Skeleton retains the entire normal wrapper, including runtime state; a host
Box keeps the dense member representation compact without cloning that owner.
Removal returns the whole wrapper-bearing candidate, not an extracted session.

The same manager counter, body authority, 16/2 source capacities and parallel
refusal apply to both kinds. Insert checks prerequisites, issues the shared
index, attaches, creates records and commits the owner. A failed Create returns
the real candidate with its issued index and attachment state: there is no
implicit detach, counter refund or claim of native failure atomicity.

Fresh source construction inserts the skeleton at group1/Active before the
checked pool-owned pre-normal and normal operations. These operations invalidate
all member publication stamps before possible partial writes, including within
the same body frame. Wrong kind, foreign authority and missing handles refuse
before mutation. Board-only truck/hook operations cannot access a skeleton as
a board. No mutable session or normal wrapper escapes membership.

Refresh preserves all-invalidate -> all-Update -> all-bounds -> all-publish ->
fresh live active scan. Removal detaches before swapping the complete last
member/publication/bounds into place. Ground consumes full dense membership,
not active ordinals. `AuthoredCollisionPublication` is only a closed borrowed
read adapter; both actual producers pass through the existing ground job,
validation, triangle traversal and contact generation. The serialized arena
route is unchanged. The manager's authored-ground frame refreshes both kinds.

## Remaining limits

This does not establish flags3 adjacent board/skater pair generation, humanoid
placement, actual Island BodyID/SpyTarget registration, Skeleton hooks/drives,
physical-player construction, live input/animation, a world scheduler, camera
or renderer. A board and unplaced skater sharing a real ground job is not a
skater/terrain contact demonstration and is not playable freeroam.

## Validation

Final gate62637 passed: fmt check, 2508 unit tests (2280 core +187 formats
+38 app +3 symbols), six doc tests, all-target workspace Clippy with warnings
denied, all seven ignored original core corpus checks and git diff check.
No app ignored probe was rerun. All code changes preceded this final gate;
only documentation changed afterward.

Independent source, production and test review passes. Focused tests passed:
three publication tests and ten pool tests (four newly added mixed-owner tests).
All seven original core corpus checks passed, including hats0/4/1 Create before
normal and refresh afterward. The mixed mounted case reuses actual board and
original tutorial no-hat skeleton producers and the production manager wrappers.
It processes both assemblies, prepares 28 actual joints and real board/terrain
contacts, retains body identities, removes the board and verifies the surviving
complete skater owner. This harness deliberately uses zero solver iterations,
controlled wheel translation, timestep, bend/ground settings and capacities.
The skater remains at source initial Part frames; this is preparatory integration
coverage, not correctly posed humanoid dynamics or a native Tick trace.

Existing Rob's Park board C/J/D remains 6/6/3,6/6/3,6/6/3,4/6/3. Its four
deck COM results are unchanged:

```text
[588.9575, -54.351295, 996.21686]
[588.9615, -54.343952, 996.2112]
[588.9654, -54.340397, 996.2055]
[588.9701, -54.33957, 996.1999]
```

## Next connected source slice

Read-only follow-on audit selects the actual 24-Part placement prefix of
`Skeleton::Set 82b4c940..82b4cc00`. It loads retained PhysicsPose and visits
Parts0..23 ascending; `GetVolumeMtxFromPhysicsBuffer 82b413a0..82b414d8` composes
physics pose with **bone-to-volume**, not its inverse, then calls
`OrthoNormalize3x3 82ae33b8`. The existing full-lane compose, source-derived
orthonormalization and actual in-place Part setter are candidate reuse points.
That work should mutate the same retained owner through a pool-owned operation,
invalidate before writes, preserve partial prefixes and refresh for ground.

This must be named a prefix, not full Set: Init constructs SkeletonDrives at
`82b629c8` before Set at `82b62a64`. Set's suffix places TWO separate skeleton
hook Parts (Hips23 then Root0) and writes the root-hook proxy target. Those are
not the board's single hook and do not exist yet. Full next dependencies are
SkeletonDrives constructor `82b59b08`, InitDrives `82b50770`, InitHooks `82b50868`.

Flags3's adjacent pair path at `Island::Step_Collision1 827b3818` is separate:
it consumes even full dense membership after EndUpdate, orders each pair by
unsigned native Assembly pointer, then uses a bounded atomic counter. No
existing connected AssemblyPairs producer/consumer exists in Rust; world IDs
or ordinals cannot silently replace pointer ordering. Do not build an unused
pair list instead of completing actual placement/hooks.
