# Actual physical skater counterpart: SkeletonData and SkeletonBody

2026-09-07, read-only next-path audit by mounted_probe after the2392 real board
pool checkpoint. Root read the full returned packet and inspected existing
ragdoll/dynamics/Andale types. Confidence **high** for the dependency graph;
constructor numeric/layout implementation requires independent source audit.
No SkeletonData or SkeletonBody construction is implemented by this packet.

Update: the next input-producer implementation and independent review are in
`skeleton-data-and-attributes.md`. SkeletonData and Vault attribute remapping
now exist; actual SkeletonBody construction/admission remains the next step.

## Outcome

The real second SkaterPool member is SkeletonBody, not AnimationSkeleton or a
caller-prepared ragdoll setup snapshot. Build its original retained inputs and
24-Part definition before extending the current concrete board pool to accept
it. Do not insert a placeholder to satisfy the flags3 even-count assertion.

Next connected slice: recover SkeletonData::Init and the generated attribute
inputs/remapping required by SkeletonBody::InitAttribData. These must produce
the normal physical-skeleton definition, then actual body/joint admission,
normal setup, skeleton drives/hooks and initial pose. Existing numerical and
pool consumers should be reused where their source semantics match.

## Source call chain and shared indices

PhysicalPlayerHiLOD constructor `0x82e03c50`:

1. Skateboard construction at `0x82e0413c`, including the board HookPool insert.
2. Skeleton constructor `0x82b56d70`, called at `0x82e041c8`.
3. Skeleton::Init `0x82b62710`, called at `0x82e04228`.
4. SkeletonData::Init `0x82b56778`, hierarchy name resolution and exact24-bone
   PhysicsParam requirement.
5. SkeletonBody constructor `0x82b62200`, calling InitAttribData `0x82b3c3b8`,
   Body Init `0x82b61738`, then TeleportReset `0x82b3bf88`.
6. Body Init's actual AddSkaterBody(Active4, bending=false) at `0x82b6213c`.

For a pristine component manager this is board world-index0, board hook1,
skeleton2. SkaterPool dense order is board then skeleton; the separate HookPool
does not occupy a SkaterPool ordinal. Indices are observations of actual shared
allocation order, never hardcoded runtime values or invented native pointers.

## First missing retained producer

SkeletonData::Init constructs:

- 24 PhysicsParam bone references in source record order.
- Physics Part/animation-bone mappings, physics parents and22 joint endpoints.
- Up to three children per physics Part, gathered in ascending Part order.
- Bone-to-volume and inverse volume-to-bone matrices for every Part.
- 24 physics poses copied from the supplied actual pose by resolved animation ID.

PhysicsParamData::GetParentID `0x82b566e8` walks animation ancestors until the
first name represented in PhysicsParam. It does not merely take an animation
bone's immediate parent.

PDB SkeletonData size0x18f0, source offsets (not proposed Rust ABI):

| Offset | Data |
|---|---|
| 0x70 | physics poses[24] |
| 0x670 / 0xc70 | volume-to-bone / bone-to-volume matrices |
| 0x1270 | animation bone IDs[24] |
| 0x12d0 | physics parent Parts[24] |
| 0x1330 | Part-to-joint indices[24] |
| 0x1518 | bone-to-Part map[100] |
| 0x183c / 0x1894 | joint children / parents[22] |

Quaternion expansion and a **general matrix inverse with reciprocal refinement**
need dedicated numeric recovery. The board's rigid inverse is not a substitute.
Retain original record ownership and the actual hierarchy/pose selection rather
than inventing a skeleton from names or a convenient database first record.

## Normal body is more than insertion

SkeletonBody::Init creates24 Parts with either0 or22 joints, selected by the
current attribute byte at `0x82b61848..0x82b61878`. It constructs original
box/sphere/capsule volumes with special root-capsule and hat paths. Ragdoll mass
properties are calculated and cached BEFORE animated mass properties and the
source inertia-adjustment branch. PhysicsParam's mass field is not a replacement
for those calculations.

The rest of this source sequence includes:

1. InitJoints `0x82b464f8`.
2. Normal culling `0x82b5f348`, then ragdoll culling `0x82b5f7d0`.
3. Actual Assembly initialization, group1 and every Part userdata1.
4. AddSkaterBody(4,false), sync,24 SpyTargets, joint spy flags.
5. Root zero material, then SetUpNormal `0x82b50508`.

**SetUpNormal changes Assembly group and Part userdata to6 after insertion.**
It installs animated inverse dynamics and normal materials, resets collision
state/drag, and selects normal/no-collision through a current attribute. A
fixed group1 or permanently immutable constructor material/inertia owner cannot
represent this completed initialization. Recover the original update visibility
and mutation timing before deciding how the shared pool accepts this owner.

Skeleton::Init later constructs SkeletonDrives `0x82b59b08`; its InitDrives and
InitHooks retain a distinct two-Part hook Assembly. The board's one-Part hook
does not satisfy this dependency.

## Existing useful code and remaining gaps

Existing formats/andale.rs parses AndalePhysicsParameters and bone orientation,
position, size, parent/joint orientation, twist/swing/mass and name fields.
ragdoll.rs implements exact24-Part identity and ragdoll culling/setup; its
SkeletonRagdollSetupState and PreparedRagdollDynamics are caller-prepared
boundaries, not construction. ragdoll_dynamics.rs preserves per-bone mass-pass
ordering but still delegates shape creation, numeric mass calculations and
division. Existing AnimationSkeleton, pose evaluation, mounted databases,
Vault ownership/reads, primitive integrals and real assembly/body/joint/drive
consumers can support the new producer without replacing missing source logic.

Still missing: SkeletonData mapping/matrices; generated skeleton layout access
and InitAttribData remapping; actual normal geometry/material/dynamics owner;
normal culling/setup and live mutable inertia/material integration; skeleton
collision publication/member variant; skeleton drives/hooks and live pose.

Relevant attributes include physics_skeleton, physics_skeleton_bones,
physics_skeleton_drives, physics_skeleton_joints, physics_animation, selected
physics_hat, and directly referenced global tuning. Generated layouts require
their own proven fetch/fallback semantics. Do not extend the board scalar reader
with guessed layout offsets, frozen tuning or substituted defaults.

The previously verified phys_Tpose type5 record is identical in SkaterBodyPres,
OffBoard and OnBoard abin files:2752 bytes, SHA-256
`136d6dcaf907c96f51e6e5184f5c7ca46357ef1ef21ce0adf755162af7ed0ef5`.
This next-path audit did not rehash those files. Actual playback selection,
hierarchy and supplied pose remain inputs to recover; presentation probes'
first-record choices are not the original physical fetch service.

## Acceptance boundary

Start with retained original24-bone PhysicsParam/hierarchy/pose and source Vault
data. Validate exact mappings, ancestor lookup, ordered children/joints, matrix
arithmetic and source attribute timing before admitting the real normal body.
Then verify actual24-Part/selected-joint owners, normal setup mutation, shared
pool membership and source teardown. Preserve uncertainty for host estimates
until native numeric traces exist.

Even two genuine board/skeleton members do not complete flags3: the original
adjacent-pair producer, native-pointer-order compatibility, skeleton drives and
controls, Actor/input/animation, World scheduling and presentation remain required.

## Next concrete shape/mass producer source packet

Independent read-only recovery after input producer implementation;
root also read the complete Body Init decompile. Numeric implementation still
requires root raw-graph comparison and independent code review. Source
SkeletonBody::Init82b61738..62200 SHA-256
`41afa55915339a9393fe199df6c138c243d4a18e2f08140867f444b15b1d877e`.

Preloop reads: Bones07204f10dd937907 float capsule dimension scale;
global physics_animation dff972080e90f82f byte (any nonzero selects22joints,
else0); global physics_wipeout3689013d7adbdd2a float ragdoll inverse-mass
numerator. Raw bgt0 assertion excludes NaN as well as nonpositive values.

For each Part: Part1 plus nonzero hatType takes the hat branch BEFORE comparing
the resolved animation bone ID to the saved Part0 ID. Root matching is by ID,
not ordinal. Every matched root rewrites mSkateboardPart, effective mass
3a83126f, reads/loads half-length5e43a9ebd2bebbde BEFORE radius7e90113b05bacda6,
creates capsule(radius,halfLength), clears volume enable bit0, writes PartMass0.
Hat must use CreateHatVolume82b4fae8: actual cylinder+sphere aggregate, selected
hat layout, source trig and two separate enable-collision samples. No guessed
ordinary head replacement for a nonzero original hat type.

General shapes, with source operation order (not geometric textbook tuning):

- Box: half-extents `size * (VolumeScalar * 0.5)`, retaining full W in debug data.
- Sphere: radius uses FSEL of `(X*0.5)-(Y*0.5)`, selecting Xhalf if nonnegative,
  otherwise Yhalf (including unordered), then multiplies VolumeScalar. No Z max.
- Capsule: baseRadius=`(X * preloopCapsuleScale) * 0.5`, radius=baseRadius times
  the early VolumeScalar. remainder=`Z*0.5-baseRadius`; FSEL(-remainder,+0,
  remainder) supplies the clamp. Fresh globalSkeleton5fe9f585a20162a2 lookup
  occurs before halfLength=`(global * clampedRemainder) * rereadVolumeScalar`
  at82b61b40. Do not silently move that record reread before the lookup.

For ALL three general shapes, unscaled PartMass is `(X*Y)*Z`, not geometric
volume and not scaled by VolumeScalar/capsule tuning. Write it BEFORE fetching
live Skeleton9f4a03c75637e7e9; effective mass=scale*product.

CreateCollisionBox82e4ae50->BoxInitialize82adcc38,
Sphere82e4af08->82ad9438,Capsule82e4afa0->82ad7fa0. Fresh volumes have
group0/surface0/flags1, identityXYZbasis and four-zero translation. Capsule
axis is localZ and length parameter is half-height. No implicit bone placement.

Common installation uses Normal DYNAMIC friction for BOTH mus/mud, plus
normal restitution. Effective-mass bgt0 condition evaluates before install,
but assert branch follows all material writes. Ragdoll factor*mass then
fdivs(numerator,result), ComputeMassProperties(density1,nullout), immediately
cache all4 inverse-inertia words. Then animated factor*mass, fdivs(1,result),
ComputeMassProperties. Part1/currentHatPresent overwrites inverse-body matrix
with identityXYZ and translationALL0, including W0.

Each Part independently constructs fresh physics classbabb3ec110849e08/default
d7edbd362d7d2152 instance; reads mode6ddd9384be03a6ea asu32; ONLY modes1/2
fetch factor425c501754441f41 and call DebugMultiplyInertiaTensor1/2. Cache
animated4inertia words, then destroy temporary instance before next Part.
Existing ragdoll_dynamics orchestration is reusable but not a concrete backend;
do not retain fabricated PreparedVolumeIdentity values as runtime ownership.
