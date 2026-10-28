# Mandatory skateboard hook assembly and cross-assembly drive

2026-09-07. Read-only live IDA/PDB/PPC recovery and current Rust producer audit.
Confidence: **exact** for cited layout, constants, calls, ordering and valid-domain
control; **high** for the typed ownership mapping below. The initial read-only
packet is retained below; the implemented checkpoint follows immediately.
No original binary, PDB or asset bytes were copied into the repository.
Host tests are not a native numerical/runtime trace.

## Implemented checkpoint: definition through actual solver and teardown

This supersedes older not-implemented statements in the historical sections.
`skateboard_hook.rs` retains distinct shared frames/dynamics, executes child then
parent quaternion-candidate setters, and creates the original disabled box with
half-extents bits3dcccccd and direct mass100/density1 calculation. It reuses
`fat_box_unit_mass`, not aggregate/Jacobi. Definition static=false remains
distinct from the runtime Static1 admission request. There is no Hook Vault
fetch. Numeric callback failures retain the completed prefix and prohibit retry;
immutable definition publication and Rust allocation are explicit host policies.

`dynamic_assembly_hook_pool.rs` owns real sessions, membership and zero bounds,
bound to both manager and body-pool authorities. Capacity is16 for flags&1,
otherwise2. Opaque Arc-identity handles are not native indices or body tokens.
The manager's shared u32 counter starts0 and wraps without reserved-value skips.
Insert consumes an index before Add; successful membership publication follows.
Remove detaches before swap-last publication and does not reclaim the index.
Part world index remains0, Part userData is1, Assembly group is1, detached
Assembly world index starts0000ffff. Assembly userData is uninitialized in the
source, represented as unknown rather than asserting null.

The source constructor inserts the board into SkaterPool before HookPool. The
current component admits the board directly, so the hook legitimately receives
the first manager index0. No fake counter burn/preseed claims constructor parity;
future SkaterPool must consume this SAME manager counter. Native allocation/job
storage layout and complete pool update scheduling are not implemented.

Host safety preflights capacity, authority and membership instead of reproducing
native unsafe paths. Native Insert ignores failed Assembly Add; host failure
returns the detached candidate, preserving its consumed index without publishing
an unusable member. Independent review caught an error path that dropped an
already-attached candidate: both candidate-construction and insertion failures
now return the actual owned session, permitting explicit recovery/cleanup.

Each frame visits real HookPool members in current dense order, then their Parts
in ordinal order, writing -1 using actual occurrence-qualified body observations.
There is no state, volume-enabled or world-index filter. This runs after actual
StartFrame and before DriveNormalizer and island-map capture. Manager identity,
current frame and allocation reuse checks protect the host label store.

The mounted corpus now executes an additional four frames with nine bodies,
six joints and three real drives. Deck6/bodyB and hook0/bodyA are verified in the
actual drive pool. Hook static state stays unchanged, its label is-1 and board
labels are0. Actual post-Horus record rows6..11/W contain six zero limits;
rows2/3 XYZ contain zero accumulated impulses. All seven board COM/quaternion/
velocity/omega bit patterns match the truck-only fifty-iteration control.

The initial test incorrectly expected immediate deck movement from adding the
fresh hook. Source reinspection established that HARD2 with zero spring,
damping and strength is precisely the disabled state: angular disable82b24c98,
linear disable82b24cb8. Build82ae4bf8 scales limits by ((strength*h)*h), and
Horus82ae3024..3038 clamps against upper and 0-upper. Production arithmetic was
unchanged; the test expectation was corrected. A separate nonzero-RF/bias unit
test checks zero-cap clamping without RF changes. Original later activation
includes EnableBoardAnimation82b2a140 and angular-only82b2aca8; it is NOT inserted
artificially into construction to force motion.

Cleanup removes parent truck drives then hook drive, removes the hook before
board bodies, and checks retained definition/payload lifetimes. Free drive slots
retain payload owners until reuse/pool destruction, as the existing pool does.
Independent source/code/test review PASS. Twenty-one new tests: eight prefix,
eight HookPool, two manager counter, two label-store and one zero-cap Horus.
Strict gate61189 exit0:2356unit+6doctests, fmt, all-targets Clippy-Dwarnings and
all four original-data core tests. App ignored corpus not rerun. Final spawn,
SkaterPool, authored collision/pair publication, full Tick and skater/input remain
open; these contactless frames are not playable skating or native trace parity.

### Additional manager/publication source proof

Manager ctor82e0c298 initializes mNextObjectID+50 with zero at82e0c324; dedicated
Static Add82e0c740 does not consume it. Insert82e0dbb0 loads/increments/stores
counter atdbd0/dbd4/dbd8, writes old value to Assembly+14 atdbdc, calls Adddbe8,
publishes memberdbf8, zero boundsdc7c/dc80, increments countdc8c. Part initializer
82ae8fc8 stores world index0 at82ae901c. These were independently checked live.
SHA-256 raw [start,end):

| Range | SHA-256 |
| --- | --- |
| `82e0c298..82e0c7e0` | `1e67ff30a5d8cba9b10967480e8a8f725373dada8ed6b18383f8f3775bdd1bef` |
| `82ae8fc8..82ae9058` | `32096d0f7dbc1bb379cd9f18c10767366ae824495516c9366690877d48371c6f` |

Existing Insert/Remove/Setup/SetAssemblyIsland fingerprints remain below.

## Outcome and connected scope

The mandatory hook is a SEPARATE one-part Assembly in the real HookPool. It is
not an eighth Part in the board definition, not the manager's dedicated static
body, and not a serialized collision asset. Its one post-admission Drive crosses
from board deck Part6 (parent/bodyB) to hook Part0 (child/bodyA).

With the shipped optional wheel-drive switch false, the graph is:

- Manager's dedicated Static body: one.
- Board Assembly: seven Active bodies, six definition joints, two later truck
  drives, retained original wheel/truck/deck geometry and definition owners.
- Hook Assembly: one Static body, no definition joints/drives, one later
  deck-to-hook Drive owned by SkateboardBody.

The total is therefore at least nine body slots, six joint slots and three Drive
slots. The current eight-slot manager-plus-board diagnostic cannot admit it.
Hook exclusion labels its body -1, but does NOT discard its Drive: source table
selection first chooses bodyB's label, which is the Active deck's label.

## Reference functions and owners

The primary decorated symbol is
`?CreateHookDrive@SkateboardBody@Physics@Sk8@@AAAXXZ`, reference
`Sk8::Physics::SkateboardBody::CreateHookDrive`, `0x82b2e4b8..0x82b2e718`.
PDB/member names and call-target addresses, not decompiler ABI guesses, identify
the following graph:

| Function | Reference address | Relevant ownership/effect |
| --- | --- | --- |
| SkateboardBody constructor | `0x82b38270` | Retains frames, dynamics and hook pointers; creates hook after other drives |
| CreateHookDrive | `0x82b2e4b8` | Creates and admits the separate hook and its cross-assembly Drive |
| JointFrames::SetChildAngularFrame | `0x82b22f58` | Writes quaternion A, using unnormalized quaternion-candidate graph |
| JointFrames::SetParentAngularFrame | `0x82b23108` | Same graph, writes quaternion B |
| PhysicsUtility::CreateAssemblyDefinition | `0x82e4a4d8` | Allocates definition with requested parts/joints and zero definition drives |
| AssemblyDefinition::Initialize | `0x82ae13f0` | Initializes PartDefinitions, names and null culling pointer |
| PartDefinition::Initialize | `0x82ae9058` | Native inertia/material/static/inverse-present defaults |
| PhysicsUtility::CreateCollisionBox(float,float,float) | `0x82e4ae50` | Allocates native 0x60-byte/16-aligned BoxVolume |
| BoxVolume::Initialize(float,float,float) | `0x82adcc38` | Inputs are HALF extents; fresh transform and volume fields |
| ComputeMassProperties | `0x82ae9a50` | Direct simple-box inertia and shared mass suffix |
| ComputeSimpleInertia | `0x82ae93c0` | Box branch calls ComputeFatBoxInertia and returns success |
| ComputeFatBoxInertia | `0x82ae7ee0` | Original rounded-box scalar arithmetic |
| PhysicsUtility::CreateAssembly | `0x82e4a540` | Allocates live Assembly retaining its definition |
| Assembly::Initialize | `0x82ae1338` | Initializes actual Parts and copies definition culling |
| Assembly::InitializeComponentsFromDefinition | `0x82ae0bf8` | Part, joint, drive definition order |
| Part::Initialize(definition) | `0x82ae9820` | Retains definition-backed volume/material/inertia/inverse pointer |
| Part::SetTransform | `0x82b25480` | Detached raw matrix copy; attached body update is a different branch |
| Island::InsertHookAssembly | `0x827b2cf8` | Locked HookPool insertion |
| AssemblyPool::Insert | `0x82e0dbb0` | Manager world index, body admission, pool publication and bounds |
| Simulation::AddDrive | `0x82ae43f0` | Free-head allocation, Active-tail insertion, retained endpoint/payload pointers |
| SkateboardBody::SetTransform | `0x82b34a90` | Spawn board Parts first, then hook with original input matrix |
| Island::SetupSimulationIslandsForSinglePlayer | `0x827b2880` | All HookPool Part labels become -1 |
| SimulationManager::SetAssemblyIsland | `0x82e0aff8` | Writes labels by actual Part/body slot, with no state filter |
| SkateboardBody destructor | `0x82b2b040` | Removes all external drives before hook/board bodies and owner frees |
| Island::RemoveHookAssembly | `0x827b2d60` | Locked HookPool removal |
| AssemblyPool::Remove | `0x82e0c1c8` | Detach, linear search, swap-last and decrement |

The principal decorated spelling was checked directly in the live IDA name
database. Hex-Rays falsely presents several utility calls with extra floating arguments
and confuses hidden result/float argument registers. In particular, the optional
volume-result pointer of ComputeMassProperties is NULL in r6 here, not the
cleared-flags value left in r4.

## Fresh retained object state and layouts

SkateboardBody size is 0x20a0. Relevant fields:

| Offset | Field |
| --- | --- |
| `+0x374` | `mPhysicsIsland` |
| `+0x380` | `mDriveFrames`, 0x40 bytes |
| `+0x3c0` | `mDriveDynamics`, 0x20 bytes |
| `+0x3e0` | `mDrive` |
| `+0x3e4` | `mHookAssembly` |
| `+0x3e8` | `mHookAssemblyDefinition` |
| `+0x1dc0` | Six truck Drive pointer storage slots |
| `+0x1dd8` | Four wheel Drive pointers |
| `+0x1de8` | Optional wheel drives present |
| `+0x1dec` | Actual truck drive count |

DriveFrames has quaternionA +0, positionA +0x10, quaternionB +0x20,
positionB +0x30. DriveDynamics has linear +0 and angular +0x10; each Params
stores spring +0, damping +4, strength +8, raw type +0xc.

Before CreateHookDrive, the outer constructor initializes both quaternions from
gLVector `[0,0,0,1]`, both positions to positive zero, both Dynamics Params to
NO_DRIVE/zero scalars, mDrive null, and mHookAssembly null
(`0x82b38308..0x82b38398`). The hook-definition pointer has no corresponding
observed initialization in this constructor prefix; successful creation writes
it before use. Do not promote an invented zero for untouched raw storage into
a recovered observable field.

PartDefinition size0xa0: volume+0, Inertia+0x10, inverse matrix+0x40,
Material+0x80, inverse-present+0x8c, static word+0x90. Fresh defaults are
SimulationInertia::native_default: inverse tensor all1, inverse mass1,
spherical1, max velocity/max omega bits7f7fffff, zero linear/angular drag.
Material static friction bits3f000000, dynamic bits3e99999a, restitution
bits3ecccccd. Both inverse-present and static words start zero. The inactive
inverse-matrix storage is not initialized by this default routine.

Volume size0x60: raw transform+0, vtable/type+0x40, shape words+0x44/+0x48/+0x4c,
radius+0x50, volume group+0x54, surface+0x58, flags+0x5c. Part size0x60:
transform+0, volume+0x40, inverse+0x44, inertia+0x48, body+0x4c,
material+0x50, world index+0x54, internal flags+0x58, user flags+0x5a,
Part userData+0x5c. These distinguish volume group, assembly group and Part
userData; they are not a single collision-group field.

## Exact successful CreateHookDrive chronology

1. Obtain the rw::physics::Simulation through the already assigned base
   `mIsland->GetRWSimulation` (`0x82b2e4e0`). Retain this simulation result for
   the eventual AddDrive, rather than rereading a guessed global later.
2. Write positionA and positionB zero (all four lanes) at `0x82b2e504/50c`.
   Construct raw I/J/K Matrix33 in scratch storage, then call
   SetChildAngularFrame at `0x82b2e544`. Reconstruct I/J/K scratch and call
   SetParentAngularFrame at `0x82b2e574`. The setters' quaternion-candidate
   estimate calls remain observable: do not replace these with a literal
   identity quaternion merely because the source matrix is identity.
   Child/parent each call their four-lane reciprocal-square-root estimate,
   refine twice and select the quaternion candidate, without the separate
   body-conversion final normalization. The already accepted shared
   `world::try_simulation_quaternion_candidate` is the reuse seam.
3. Store angular raw type2 HARD_DRIVE, then angular spring/damping/strength
   positive zero (`0x82b2e588..598`). Then store linear type2 and its three
   zero scalars (`0x82b2e59c..5a8`). This is not NO_DRIVE and must flow unchanged
   to the original Drive Build; no interpretation as a disabled boolean or
   an arbitrary spring is justified.
4. Resolve the PhysicsSkater resource allocator. A missing allocator singleton
   takes FailAssert+DebugBreak and reloads the singleton, not a successful
   fallback. CreateAssemblyDefinition(allocator,1,0) at `0x82b2e5e8` uses
   GetResourceDescriptor(parts1,joints0,drives0), allocates named
   `AssemblyDefinition`, and initializes the definition. It constructs its
   one PartDefinition with null volume, hashed Part name0, null culling;
   no joint or DrivenPair exists in this definition.
5. Store mHookAssemblyDefinition (`0x82b2e5f4`), read its PartDefinition0,
   then CreateCollisionBox with three identical source constants
   bits3dcccccd (`0x82b2e600..60c`). The scalar BoxVolume initializer names
   them halfX/halfY/halfZ and stores them directly: HALF extents0.1, full
   side lengths0.2. There is no multiplication by0.5 in this call chain.
6. Fresh box has type4, radius+0, group0, surface0, enabled flags1, transform
   I/J/K with translation `[0,0,0,0]`. Clear only enabled bit0, yielding flags0
   (`0x82b2e610..61c`), and install the volume pointer in PartDefinition0
   (`0x82b2e624`). Its disabled state does not suppress mass calculation.
7. ComputeMassProperties(part, mass bits42c80000=100, density bits3f800000=1,
   optional volume-result pointerNULL) at `0x82b2e638`. Return success is
   ignored. Its simple box path succeeds, as detailed below. No material,
   drag, maxima or static-word override is performed after the defaults.
8. CreateAssembly(allocator,definition) at `0x82b2e644`; store mHookAssembly.
   Definition still owns the one Part's geometry/material/inertia storage.
   Assembly initialization sets group0, counts1/0/0, flags0/userFlags0,
   initial worldIndex0xffff, retains definition, initializes its Part and
   copies null culling. Assembly userData+0x28 is NOT initialized by this
   path. Do not set it to1 on the strength of the later Part userData write.
9. Construct I/J/K/zero raw affine matrix and Part::SetTransform(hookPart0)
   at `0x82b2e680`. Its body is not yet admitted, so this is the detached
   all16word copy branch. Set Assembly groupID1 (`0x82b2e698`), then walk
   its Parts in order assigning Part userData raw1 (`0x82b2e6b0`).
10. InsertHookAssembly(hook,STATIC_BODY raw1) through mPhysicsIsland at
    `0x82b2e6d8`. This differs from the unchanged PartDefinition static flag0.
    Part::AddToSimulation receives1, so it admits Static regardless of the
    flag that would only override an incoming Active state4.
11. Call Simulation::AddDrive at `0x82b2e700`: parent/B is the board's Part6
    body (+0x28c in the Part array), child/A is hook Part0 body (+0x4c), with
    the retained mDriveFrames and mDriveDynamics addresses. Store the returned
    Drive pointer, even null, at `0x82b2e704`. There is no local null check,
    rollback, count change to the board definition, or addition to its
    mNumDrives truck count.

No Vault access occurs in this function or its geometry/mass/assembly helpers.
The constants above are code constants, not a corpus-specific replacement for
runtime attributes. The preceding optional-wheel branch remains a genuine
runtime lookup: physicswheels key69cef3ca3f1480b0, index0, DefaultDataArea(1)
on missing pointer, and byte-nonzero condition. The shipped value0 skips four
wheel drives. Mandatory CreateHookDrive follows regardless of that result.

## Box mass and numerical reuse boundary

ComputeMassProperties calls ComputeSimpleInertia (`0x82ae9b34`); its BOX branch
reads the three half extents and radius and calls ComputeFatBoxInertia
(`0x82ae94fc`). It does not inspect enabled flags and returns success1. Thus
the hook does NOT use ComputeVolumeIntegral, mapped-array accumulation or
DiagonaliseVolumeIntegral. Reusing those to obtain a plausible box result
would change the source numerical graph.

The already source-audited scalar producer is private
`simulation_volume_integral.rs::fat_box_unit_mass`, reference0x82ae7ee0.
Use that same implementation via a narrowly shared seam, not a second formula.
For this fixed input the radius is zero and all three equal half extents are
positive, above the helper's safety floor. The ordinary real-number result is
diagonal `[2*h*h/3;3]` and volume8*h*h*h, h=0.1; this is explanatory algebra,
NOT permission to reassociate its existing f32/fused operation graph or freeze
runtime result bits from host arithmetic.

After simple inertia, ComputeMassProperties copies the volume's fresh identity
frame as the candidate body-to-part frame. The COM squared length is0; its
threshold `((diagY+diagZ)+diagX)*bits358637be` is positive for these source
constants; all identity-axis deviations are zero. It leaves inverse-present0,
without writing a matrix. This is unlike the deck's forced inverse-present1.
The shared `skateboard_wheels::apply_unit_density_mass_to_inertia` already
preserves the mass100 application: scalar inverse mass, mass-scaled principal
diagonal, reciprocal estimates Y thenX thenZ, both residual/correction rounds,
inverse-tensor W=X, and spherical selection. Defaults' drag/maxima stay intact.
`apply_fresh_mass_properties` currently accepts only Sphere/Capsule and has an
unreachable arm for Box; it is NOT currently a callable generic Box helper.

Retain this computed definition inertia even though runtime STATIC AddRigidBody
clears its live body inertia pointer and does not use the supplied inertia.
Definition/Part data and body solver data are different observables. Reuse the
existing real Static pool admission, which also resets motion/force/RF state,
uses the configured cooldown and appends to the Static ring. Do not invent
an active100-mass anchor or use the manager's dedicated static body instead.

Native estimate and exact per-tick output traces remain required for Xenon-bit
claims. Host estimate providers must retain their honest existing boundary.

## Actual HookPool constructor and insertion

Island::InitWorld (`0x827b42c0`) constructs AssemblyPool Params for HookPool.
The PDB Params layout is size0x10: maxAssemblies+0, bool
createSerialisedAssemblies+4, numAssembliesPerJob+8, bool updateInParallel+0xc.
Params constructor `0x82e0ad90` defaults0/true/1/false respectively.
InitWorld sets Hook createSerialisedAssemblies=false at `0x827b4340`.
For flags&1 (including offline Both3), maxAssemblies16 is stored at
`0x827b4374`; otherwise2 at `0x827b4384`. Job size1 and parallelfalse remain.
The 0x420-byte HookPool object is constructed at `0x827b4544`, retained at
Island+0x6cd4. Allocation-null is assigned as null; later callers assume a pool.

AssemblyPool constructor (`0x82e0d9c0`) initializes count0, retains the actual
SimulationManager/allocator, allocates pointer array and 32-byte BBox array,
but leaves mSerialisedAssemblies null because of the false flag. It also
allocates existing job-input and Active-list storage as prescribed by Params;
those allocations do not make the pool a collision producer. InitWorld gives
the collision pipelines Skater/Dmo/Pedestrian/Vehicle pools, not HookPool.

InsertHookAssembly holds Island's critical section around HookPool::Insert,
passing fresh SerialisedAssembly::Params (`0x82e0b680`, threshold bits3f7fbe77,
0.99900001). The parameter is not consumed by a serialization call in HookPool.
AssemblyPool::Insert does:

1. Read manager mNextObjectID, increment it, store old value to Assembly
   worldIndex (`0x82e0dbd0..dc`). This is not a decoded asset ObjectId.
2. Assembly::AddToSimulation in source Part/joint/Drive order; for hook it is
   only Part0. It returns0 if the body pool is exhausted. Pool Insert IGNORES
   that return value.
3. Publish Assembly at pointer-array[count]. With null serialized array, write
   both BBox vectors to four-lane zero (`0x82e0dc70..80`), not the box geometry
   bounds and not fabricated serialized storage.
4. Increment count. Native code does not capacity-check the HookPool array here.

Source failure behavior is unsafe, not transactional: allocation/null/body
capacity failures can publish an unusable hook and then reach AddDrive with
null endpoints. The typed host must reject invalid capacity/authority and
protect memory, while explicitly describing any rollback as host policy.
Successful source order and prior successful allocation occurrences must not
be silently rewritten. Successful AddDrive uses Free head -> Active tail,
retains body/payload pointers, sets spy0 and leaves tag unchanged; a capacity
null does not remove the already admitted hook or earlier truck/wheel drives.

## Spawn placement and later per-frame exclusion

Outer constructor order remains Init -> AddSkaterBody Active4 -> SyncCheckBody
-> CalcTruckTransforms -> CreateDrives -> optional CreateWheelDrives ->
CreateHookDrive (`0x82b388a8`) -> SetTransform(spawn) (`0x82b388b4`).
Creation-time hook position is origin, not the final spawn position.

SkateboardBody::SetTransform (`0x82b34a90`) first handles the seven board Parts
with its deck-relative matrix logic and normalization, then asserts nonnull
hook and calls Part::SetTransform(hookPart0, ORIGINAL supplied matrix) at
`0x82b34d84`. That is now an attached Static body update. This packet identifies
that dependency but does not claim the entire board spawn-transform numerical
routine is already implemented. Do not leave the actual hook at origin when
the board is moved or substitute a normalized temporary for that last input.

Step_Solver2 (`0x827b35b8`) for flagsBoth3 invokes
SetupSimulationIslandsForSinglePlayer at `0x827b35dc`, before DriveNormalizer
at `0x827b3610`, optional thaw, partition and tables. Setup snapshots HookPool
count at `0x827b2894`, then visits pointer-array entries ascending. For each,
SetAssemblyIsland(assembly,-1) at `0x827b28b8` visits every Part in Part order,
without checking Static/Active/Frozen flags; it writes the low signed byte in
the manager's body-slot map at `0x82e0b038`. Real pool membership supplies this
authority. A previously initialized static slot already containing-1 is not
proof that the required per-frame producer has run, especially after reuse or
state changes.

The cross-assembly Drive's B is Active deck6 and A is Static hook0. Existing
Drive Build/table admission `(stateB|stateA)&4 != 0` succeeds and label choice
B-unless-minus1 assigns the deck's label. It therefore remains in the actual
Drive Jacobian batch. All Active Drive frame owners, including this one, are
normalized by the newly implemented source DriveNormalizer before Build.
Do not manually provide allzero island maps or create native-address tokens.
World collision and assembly-pair partition are still distinct prerequisites
for claiming a complete original offline Tick.

## Teardown, mutation visibility and failure boundaries

SkateboardBody destructor (`0x82b2b040`) unregisters the command receiver,
asserts its wheel line batch is absent, and obtains its simulation. Relevant
order then is:

1. Remove truck drives in ascending ordinal while ordinal<mNumDrives.
2. If mHasWheelDrives, remove four wheel drives in order.
3. Unconditionally RemoveDrive(mDrive) at `0x82b2b128`. A null hook Drive is
   NOT guarded; Simulation::RemoveDrive (`0x82ae3360`) dereferences it.
4. Obtain allocator, assert nonnull hook. If mPhysicsIsland is nonnull,
   RemoveHookAssembly (`0x82b2b1a4`) before RemoveSkaterBody (`0x82b2b1bc`).
5. Free the hook's volume (`0x82b2b1fc`), definition (`0x82b2b234`), then live
   hook Assembly (`0x82b2b26c`). Other board aggregate/body teardown follows.

RemoveHookAssembly holds the same Island lock around pool removal. PoolRemove
first calls Assembly::RemoveFromSimulation, then linearly searches the current
pointer array for matching Assembly; if not last, replaces its entry with the
last Assembly and moves that BBox (and serialized record for other pool types),
then decrements count. This is swap-remove, not stable-order erase. It assumes
membership/nonempty pool; native missing-entry/underflow behavior is not safe.
For this hook, RemoveFromSimulation captures current Part transform, removes
its body, clears its body pointer, and has no internal joints/drives to remove.
The external hook Drive must already be gone.

Native Drive removal leaves old pointer payloads in recycled slots. The current
safe pool retains Arc owners while those slots retain semantic pointers, even
after removal, until overwrite/pool destruction. This established safe lifetime
policy differs from literally freeing a still-addressed native allocation, and
should remain explicit. Releasing the high-level hook owner must not expose a
dangling frame/definition/body capability. Keep mutable frames/dynamics in the
actual retained owners so per-frame normalization and later gameplay writes
are visible; copied snapshots are observations, not a replacement owner.

## Current Rust reuse and smallest missing producer

Audit of current files, after the source-normalized four-frame authored-board
constraint checkpoint was added by the root:

- `simulation_volume_integral.rs::fat_box_unit_mass` contains the accepted
  original simple box arithmetic but is private. Share it narrowly for the
  direct hook mass path; do not route through an invented aggregate.
- `skateboard_wheels.rs::{FRESH_VOLUME_TRANSFORM,
  apply_unit_density_mass_to_inertia}` and SimulationInertia::native_default
  cover existing transform/default/mass suffix semantics. The small
  fresh-centered Box inverse-presence decision needs the source branch above.
- `world::try_simulation_quaternion_candidate` and the truck-drive numeric
  estimate service cover both angular-frame setters. Call twice in source
  order, preserving mutable-owner writes and explicit host numeric failures.
- `DynamicAssemblyPartAttachmentTemplate::try_from_parts` and
  `DynamicAssemblyRuntimeTemplate::try_from_owned_parts` already admit a real
  retained authored geometry owner without BAM, arena or clustered-mesh
  fabrication. A separate one-Part hook template with empty definition joints
  and drives can use the existing Static attachment path.
- `dynamic_assembly_post_drives.rs::try_add_post_attachment_drive` currently
  only indexes parent and child inside ONE session. The genuine new admission
  seam is an owner-checked cross-session deck6 -> hook0 Drive, retaining the
  returned occurrence and preventing either session from detaching while that
  relationship remains. Do not merge the hook into the seven-Part board or
  expose unchecked arbitrary body handles to work around this boundary.
- `simulation_drive_pool.rs` already retains actual frames/dynamics and both
  endpoint occurrences. New source-normalization in
  `simulation_drive_normalization.rs` mutates those owners in real Active
  order. The existing Drive Build/Horus need no synthetic record inputs.
- `SimulationAssemblyConstraintSession::try_step_contactless` currently starts
  manager labels, normalizes Drives, snapshots labels, creates phase, builds
  real C/J/D and solves/integrates. Its declared restriction is non-Hook.
  `simulation_island_state.rs` has StartFrame and snapshot but no membership-
  backed SetAssemblyIsland producer. Introduce source HookPool exclusion BEFORE
  normalization/map publication, using actual retained members and current
  body occurrences, not an arbitrary manually authored map.
- Full AssemblyPool manager world-index/pool-membership/bounds publication is
  not established by a body session alone. HookPool is unusually bounded:
  explicit nonserialized membership, zero bounds, swap-remove, no collision
  arena. Keep that real effect distinct from general serialized pool work and
  do not manufacture native pointer ordering, world IDs or collision authority.

The next useful connected slice is therefore retained hook definition -> real
one-Part Static session -> cross-session retained Drive -> actual HookPool label
producer -> existing normalized constraint Build/Horus -> ordered teardown.
It remains a controlled collision-free solver test until actual collision,
pair partition, spawn movement and gameplay updates are connected.

## Focused acceptance checks still required

The later cross-session ownership subsection below supersedes only that
interface gap; the complete hook constructor, HookPool and frame checks remain
required.

1. Original code-constant geometry and default field packet, including all16
   transform words, half extent vs full size, flags0, group/surface0, Part
   static0 and inverse-absent. Confirm two separate quaternion estimate calls
   and direct Box mass service order with an injected numeric provider.
2. Actual manager+mounted board definitions/truck drives+hook produces
   Static2/Active7, Joint6, Drive3 in one authority. Hook is a second assembly,
   not manager static and not board Part7. Definitions keep their zero drives.
3. Original endpoints B=deck6/A=hook0, retained payload mutation visible to
   normalization/Build, deck island assignment with hook-1; nine-slot capacity
   and finite host numerical record/body effects tested in the real solver.
4. HookPool exclusion visits actual membership/Part order every frame, including
   state-change or label-reuse discriminators; swap-remove changes later order
   as source. No real collision capability or fake raw address is introduced.
5. Short body/Drive capacity, foreign backend, stale endpoint, poisoned owner,
   and detach-while-cross-drive-live fail safely with explicitly documented
   divergence from native unsafe/null behavior. Earlier native-success prefix
   must be preserved where the host API promises source ordering.
6. Cleanup removes truck/optionalwheel/hook drives first, then hook body before
   board bodies, and verifies retained-owner lifetime according to the actual
   pool policy. Reattach/reuse must not accept stale occurrences.
7. Native per-tick reference traces and actual later spawn/drive update path
   remain unverified. No host result or local count proves playable skating.

## Cross-session ownership implementation

The private `dynamic_assembly_post_drives.rs` extension now accepts a parent
session and a separately owned child session, allowing different retained
geometry types. `try_add_cross_attachment_drive` checks both attached backend
authorities and their respective Part indices, reserves both bookkeeping
vectors, and calls the existing real backend AddDrive with parent/B then
child/A. It acquires no payload lock and does not filter body states. Existing
AddDrive preserves its capacity-null-before-body-validation short circuit.

The parent stores the actual returned optional handle in chronological
post-admission order before return. The child stores a shared host detach guard;
no body handle, slot identity, native address or mutable pool is exposed.
Parent detach is blocked by its post-admission list and child detach by any
live incoming guard. Multiple independent parent relationships remain distinct.
These guards are host lifetime policy, not original memory fields, simulation
constraints, HookPool membership or island labels.

Native capacity null returns false, preserves both body assemblies and records
the attempt. Conservatively guarding even that null until explicit cleanup is
host recovery of the native unsafe/null lifecycle, not a live Drive claim.
Typed backend errors publish neither entry nor child guard; earlier successful
calls remain retained. Vector reservation can fail before Add; Arc allocation
uses Rust's allocation policy. No destructor performs implicit backend cleanup.

Explicit parent cleanup walks post-admission entries forward, allowing source
truck/optional-wheel/hook creation order to determine their removal order.
For each Some, backend removal completes before clearing that stored handle
and releasing the corresponding child guard. Independent code review caught
and corrected a stale-prefix problem: if a later backend removal panics and is
caught, completed handles must already be cleared so retry cannot remove them
twice. The failing entry and its guard remain intact. Skipping native-null
entries, per-entry clearing and retry bookkeeping are explicit host policies.
After all removals, the parent list clears. Successful child detach clears its
now-inactive incoming guards, so later attachment cannot inherit live old guards.

Independent production and ten-test review pass after this correction. Root
ran all fourteen post-drive tests (ten new plus four existing), all passing.
The final strict gate 1448 completed exit0: formatting, 2335 passing unit tests
(2107core+187formats+38app+3symbols), six doctests, warnings-denied all-targets
Clippy, then all four explicitly enabled original-data core probes. The app
ignored corpus was not rerun. The new tests use synthetic owned primitive
definitions and actual pools; original-data probes keep their existing scope.
This bridge alone does NOT create the source hook definition,
publish HookPool/world indices, exclude hook labels, relocate a spawned board
or change the existing non-Hook four-frame corpus scope.

## Raw source fingerprints

Independent source gate: remapped_source_audit checked all 24 fingerprints
below against live IDA and separately checked constructor arguments, pool
configuration, membership-label order, unchanged spawn input and destructor
ordering. No packet correction was required. Root also inspected live
CreateHookDrive and destructor control flow. This approves source recovery,
not an implemented complete HookPool lifecycle or native output parity.

SHA-256 over live IDA bytes `[start,end)`; addresses are hex:

| Function start/end | SHA-256 |
| --- | --- |
| `82b2e4b8..82b2e718` | `0f452aea591bfa295edfdbea162c7fe0e2b926091550672f421f4e73f2f5ce1d` |
| `82e4a4d8..82e4a53c` | `edd3106657a2d91c81c7405405c52c8b669e86529a8943f8a63c931fdfc6b873` |
| `82e4a540..82e4a5b8` | `d97ebcb5f2a3c322c6b1bfeafd348df7da36f56fc855f0ff1770d00feb0fc8c3` |
| `82e4ae50..82e4af08` | `5a94c4b50f1a97722c81eae2207ae4b0641bd948930f06bbcc97e8133e83c78f` |
| `82adcc38..82adccf0` | `215864ac718beafa52a02552be4698f257cf8fc461e8a0bca53a27bfc5684da5` |
| `82ae9058..82ae90ec` | `e1fb5bebae72b2ea11ea5112b62983902814f69ea47cf52c26c6f94e9f30bb94` |
| `82ae13f0..82ae14f0` | `cbe1208f54e1f45aea3ead4925d784d6c46aac953096830aa52588677d302008` |
| `82ae1338..82ae13ec` | `0aaa886f0912d51660720587470c3f9b12fd9c169d7d6f66391073c88e3ce3fc` |
| `82ae0bf8..82ae0df0` | `007c0a98a60d5a7c3535621ca32e78e94a46087b292a1a4b77a6c477307cba51` |
| `82ae9820..82ae9874` | `e713562c048f22ec1969fd3da02a584ed230ccd3873c7a0593eb12c11d6d8e33` |
| `82ae7ee0..82ae8024` | `bea18d90fff2a8740294dfbc141dd4810447ef8b8282c658b349056fc1dcb553` |
| `82ae93c0..82ae9560` | `55e117e92491f05866fe79c00690346171834b5e862fdd8efa57831af53b35ff` |
| `82b22f58..82b23108` | `87ab1e713082262ad5ff55af873e86e9e89877a78fdb8a5f00716a4224c93a5c` |
| `82b23108..82b232b8` | `24162358d7515c009e158ce50419c1cc7af4afd7b8ab877191510f725168e12f` |
| `827b42c0..827b53b8` | `d0ae274841158fa6f2879d3a4ece911fb7a5b8a00fadcf22489931eaf8e1ef59` |
| `82e0ad90..82e0adac` | `0fdcf8d78b6397d636cd367cdf68c8a43806bb8f413d867ed7a52b1d1cd892af` |
| `82e0d9c0..82e0dbb0` | `9248adcfa59a92e43b3a805aa8f39a82240b55f51bc9ad58f7ae7d6bd82965fd` |
| `82e0dbb0..82e0dc98` | `5597112ff6c0264eaf69ef3c289855e188c8165662509a82b659141f7e87ee61` |
| `827b2cf8..827b2d5c` | `b184e1587eb414cf399e8b53b08030575b55ef7cb815f3edae9dc0d12d7d1148` |
| `827b2d60..827b2da8` | `f395e710ce6212e4afa5a16772b92ee2d2134d96f91b028a21b06b521a0fce55` |
| `827b2880..827b28d0` | `dc25e1d96fa9a817e7349739e38e9c26e5bd852e12f9148f448c9957ca9ae52c` |
| `82e0aff8..82e0b044` | `c0b5ff8e9a226ddf5175317dceb1639e8878e81136864061be928186c40e6307` |
| `82e0c1c8..82e0c298` | `0bbcfa07c1097bae6be9df0f1d37126908fde882525f1d7b787ad73f50695e1c` |
| `82b2b040..82b2b2f0` | `1f8a77fe6e5ddda3d98f8667984e533aaa1e94baa29b82e4c12be61ffcfa3c38` |
