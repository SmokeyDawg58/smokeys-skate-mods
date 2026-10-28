# Physical-player Entity identity required by contact feedback

2026-09-07. Read-only source packet; no actual Actor/UCOM/Entity manager or
paired PhysicalPlayer owner is implemented by this evidence. High confidence.

## Actual propagation

Actor ctor passes its own Entity-at-offset-zero reference to PhysicalPlayerHiLOD
at825ff2f0/2f4. HiLOD retains that input at+8, store82e03ca8. It passes that owner
to board at82e0412c/413c; Skateboard forwards it at82b38bcc/38bd4; SkateboardBody
stores base Body.mpOwner+20 at82b387a4, BEFORE Init/admission.

HiLOD reloads SAME retained owner at82e041b0 and passes it to Skeleton at
82e041c8. Skeleton retains it at82b56e28; Skeleton::Init loads the Entity at
82b62988 and calls SkeletonBody at82b6298c (82b62990 retains its return).
SkeletonBody stores dedicated owner+b74 at82b62428
and base Body.mpOwner+20 at82b625ac, before Init82b625c4.

Body base ctor825ea770 initially sets owner null at825ea7d8. Both derived
constructors overwrite this; their completed owner must NOT be treated as null.
Their Entity::GetPhysicsIsland calls require the actual supplied owner.
Consequently board and skeleton are different Bodies but self-colliding through
the same Entity identity in CreateContactSpyList.

## Existing host gap and minimal connected producer

Current player.rs has loading/registration requests, not an actual Actor owner.
Existing board/skeleton component owners are real; MotionGraphEntityAuthority
is Motion-only, and LivingWorld entity IDs belong to a different system.
Neither, nor player index/asset ID/BodyID/world index, is Entity identity.

Next actual integration should introduce a retained physical-player construction
owner coordinating the paired board/skeleton components and their character
inputs. Its privately shared retained reference can project this bounded native
owner-identity relationship, and must be propagated BEFORE each component's
admission. Distinct physical-player constructions with identical assets must
remain distinct. Do not independently mint an owner token during registration
or present a free-standing empty token as complete native Entity behavior.

Until that aggregate exists, component Entity provenance is UNKNOWN, not native
null. Refuse owner-sensitive classification rather than compare unknowns as null.
A known-null variant belongs only to separately proven Body producers that
actually retain null. Actual Island membership must reuse the same retained
component owners, actual body capabilities and source constructor ordering.
This is the necessary producer for honest A/B self-collision classification,
not a reason to substitute a diagnostic-only registry.

## Hashes

```text
Body ctor825ea770..825ea808 dcfc46fd18a9eea80ae0dd0da4fe08d305fff79bb68fc553f780b6c3ff0a53c2
Skateboard ctor82b38918..82b38c30 549f427fc9d4992c37323a8f70b6d3bbadd88502f7434bd8ac3ebd6d98d8792f
SkateboardBody ctor 572d9a001bca3cc986eb0f1f234b44f8deb0026ab61639ac070e5a8c8597f123
Skeleton ctor 2b18fc26c4b0b779768694008612f44554820e923aeb3e3459606204293adfc5
SkeletonBody ctor 8dd1b36ab7ee3920c57f3e88de88025327b2a2c59c068575607c239793881e73
PhysicalPlayerHiLOD ctor 3acf66b57c929cabff86c71671972c9eb254fa93716205e8d9d91ff0ebbd8ded
```

## Constructor and connected API addendum (2026-09-07, after checkpoint2554)

This addendum preserves the earlier identity packet and makes its proposed
producer concrete. It is SOURCE/API EVIDENCE ONLY: no paired physical owner,
complete Actor, complete HiLOD constructor, or new registration integration was
implemented by this investigation. Native layouts/order below are `exact` where
checked in live IDA; the proposed Rust composition is `high` confidence and
remains subject to implementation and independent source review.

### Entity selects an Island through the actual physics world

`Sk8::Entity::GetPhysicsIsland`, `0x825f1758..0x825f180c`:

1. Obtain the Simulation singleton; assert if absent.
2. Obtain its workspace physics world; assert if absent.
3. Invoke the Entity's virtual flags getter and test bit1 at `0x825f17d0`.
4. Bit set: call mutable `IWorld::GetSimulationIsland` through virtual `+0x20`
   at `0x825f17ec`.
5. Bit clear: call mutable `IWorld::GetVisualIsland` through virtual `+0x28`
   at `0x825f1800`.

This is not an Entity-stored Island field, and is not simply an unconditional
return of the world argument in `Init_IPhysical`. A bounded simulation-only
host construction must retain/check the actual selected manager/world and
declare unsupported visual-Island routing. It must not fabricate singleton
services. The nearby symbol labelled `Actor::GetFlags` at `0x825f9db0` has a
misleading shared/thunk decompilation; it is NOT proof of the concrete Actor
flags implementation.

Subsequent independent raw call-site correction: the board invokes this getter
at82b382e8 and caches its result at82b382f4, BEFORE Init82b387e8. Its
Add82b38808 uses that cached selection. Skeleton invokes the getter at82b62120,
AFTER actual Assembly construction. Sampling both immediately before admission
would reorder the board. Mask1 (clrlwi31 at825f17d0) is the exact bit test.

PDB Entity size `0x20`: Resource base at0, motion interfaces at `+0x14`, `+0x18`,
`+0x1c`. The const/mutable IWorld Island virtual slots are `+0x1c/+0x20`
simulation and `+0x24/+0x28` visual. These offsets are evidence, not proposed
Rust layouts or host address encodings.

### Actual HiLOD inputs and construction order

`PhysicalPlayerHiLOD::PhysicalPlayerHiLOD`, `0x82e03c50..0x82e04ddc`, retains
the incoming Entity at `+8`. PDB HiLOD size is `0x340`.

`Init_IPhysical` size `0x30`:

| Offset | Actual constructor input |
|---|---|
| `+0` | Animation hierarchy pointer |
| `+4` | PhysicsParam pointer |
| `+8` | Eight-byte affine PoseBufferExt, with its own hierarchy/buffer references |
| `+0x10` | Bouncy-ragdoll byte |
| `+0x14` | Grab-spline query manager |
| `+0x18` | Network-game query |
| `+0x1c` | Physical-world services |
| `+0x20` | LivingWorld manager |
| `+0x24` | Actor query |
| `+0x28` | Physics world |
| `+0x2c` | Human byte |

Entity, supplied spawn matrix and `Character::eHatType` are separate constructor
arguments. Actor obtains the hierarchy, `phys_Tpose` via
`PlaybackData::GetPhysicsParam(0x82d7cb30)`, and bind pose; see
`local-skater-spawn-path.md`. No render mesh, texture, or board presentation
recipe is passed directly to these physical constructors. Board collision
geometry comes from Vault physics attributes. Appearance contributes the
selected hat enum; it must not be inferred solely from a presentation recipe.

Relevant HiLOD prefix order:

1. IPhysical/UCOM base and own defaults; retain owner and simulation services.
2. Construct ProcessedPhysIn (`0xa30`), PhysOut (`0x50`), then conditioners
   (`0x2c`), SurfacePhysics (`0xa4`), GrindAirAdjust (`0xe90`), Reckoning
   (`0x370`), TrajectorySelector (`0x23a0`), OffboardTrajectorySelector
   (`0x14e0`), HandPlantManager (`0xa00`), Pumping (`0x60`), and AnimOut (`0x18`).
3. Construct Skateboard (`0x550`) at `0x82e0413c`, passing this physical owner,
   SAME Entity, ProcessedPhysIn, Reckoning, Pumping, and supplied spawn matrix.
   Its actual board admission, drives/hook and Set happen before it returns.
4. Construct PhysicsDebugSystem, then set the board Body's `mRemainOnReset=1`.
5. Construct Skeleton (`0x3e80`) at `0x82e041c8`, passing SAME Entity,
   ProcessedPhysIn and Reckoning, plus GrindAirAdjust, hat type, bouncy byte and
   Actor query. The SkeletonBody does not yet exist at this point.
6. Construct StateChanger (`0x38`) referencing PhysOut, ProcessedPhysIn, board
   and skeleton.
7. Call `Skeleton::Init` at `0x82e04228` with the original hierarchy,
   PhysicsParam and pose. This creates SkeletonData, then the actual body,
   admission/normal/drives/placement/history path.

Important retained HiLOD fields: PhysOut `+0x2ac`, ProcessedPhysIn `+0x2b0`,
AnimOutPhysIn `+0x2b4`, AnimOut `+0x2b8`, Reckoning `+0x2bc`, Pumping `+0x2c0`,
trajectory selectors `+0x2c4/+0x2c8`, HandPlantManager `+0x2cc`, Skateboard
`+0x2f8`, Skeleton `+0x2fc`, StateChanger `+0x300`; query/world services occupy
`+0x314..+0x324`.

Native Skeleton retains `mBouncyRagdoll` at `+0x3e45` (next to
`mTeleportingNow+0x3e44`); SkeletonBody retains it at `+0x1e62`. Current
`skeleton_*.rs` does not retain this flag. The supplied choice must not silently
disappear or be claimed implemented merely by retaining it in a new input owner.

### Failure and teardown are not an atomic paired transaction

Native allocation sites often store null on allocation failure, then later
unconditionally dereference required collaborators. This does not establish a
recoverable native Result protocol. Safe host errors may retain completed
prefixes, but must disclose the failure boundary.

Board admission precedes SkeletonData construction. Preconstructing both
definitions and then atomically admitting both REORDERS the source. The proposed
production path must construct/admit/finish the board first, then construct the
skeleton; failures preserve actual prior members, candidates, capabilities and
issued manager IDs. No ID refund or hidden whole-pair rollback is implied.

HiLOD destructor `0x82e00f88..0x82e01680` destroys Skeleton at call
`0x82e01368`, then Skateboard at `0x82e01394`; the shared processed input and
other collaborators remain alive through both body teardowns. Skeleton
destructor `0x82b574f0..0x82b575d0` destroys SkeletonDrives at `0x82b57518`
before SkeletonBody at `0x82b57540`. Reuse the separately accepted source
hook/bone-drive cleanup and RemoveSpy/pool-removal order, not unordered clearing.
Physical-world/UCOM removal occurs later in the enclosing HiLOD destructor.

### Concrete retained constructor inputs and lifetime

A proposed `PhysicalPlayerAssemblyConstruction` should invoke the existing
producers, not accept arbitrary preconstructed board/skeleton definitions.
Its retained input types can be the actual existing:

- `Arc<attribute_runtime::AttributeRuntimeVault>`;
- mapping `Arc<skate2_formats::andale::AndaleHierarchy>`;
- `Arc<skate2_formats::andale::AndalePhysicsParameters>`;
- separate pose `Arc<AndaleHierarchy>` and `Arc<[animation::AnimationMatrix]>`;
- supplied `[f32;16]` spawn, raw `u32` hat type, bouncy/human bytes or their
  proven bool projection, and Entity flag selection;
- the actual selected world/manager binding.

The pose hierarchy must remain distinct from the mapping hierarchy. Construct
`SkeletonPoseView { hierarchy: pose_hierarchy.as_ref(), matrices:
pose_matrices.as_ref() }` only when entering the skeleton phase. A retained
immutable matrix snapshot is an explicit host policy, not a live Actor pose
producer. If database provenance is retained, use the actual `AndaleDatabase`
owners/selected records, not a label asserting authenticity. Existing
`skeleton_data_corpus.rs` derives a controlled original RAW rig pose; it does
not establish actual live Actor pose selection.

The exact existing world type is
`SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>`
under `simulation_assembly_backend::constraint_session`. A construction object
may borrow this world mutably for its construction lifetime, preventing world
substitution. A child module there can read the private contact manager's
`assembly_pool_authority()` without exporting mutable manager access. Completed
pair handles retain the real `SimulationManagerContactAuthority` and actual
member handles. Shared Entity projection must refer to this real input/member
construction lifetime, not a new empty unit token or a reused unrelated ID.

### Board producer phases and gaps

`skateboard_definition_corpus.rs` demonstrates component composition, not the
complete constructor. Native `SkateboardBody::Init(0x82b374e8)` first samples:

- Trucks key `0xb1cbe3e929afc8a0`, lookup `0x82b37580`, byte load
  `0x82b37594`: zero or SIX joints. Skip CreateJoints on zero; the current
  corpus composes six unconditionally after separately asserting the real byte.
- Trucks key `0x2be08dd7a9207622`, lookup `0x82b37670`, float load
  `0x82b37684`, store `mWheelDistFromDeck+0x2094` at `0x82b3768c`.

Use `SkateboardAttributes::lookup` followed by `byte`/`float` for these source
reads. Existing `try_bind_skateboard_assembly` accepts a joint Vec, so zero
joints does not require fabricated joints or a new generic template.

Actual component APIs, in order:

1. Retain `SkateboardAttributes::bind(&vault)` and
   `SkateboardWheelAttributes::bind(&vault)`.
2. `SkateboardWheelDefinitions::try_create(wheel_attributes.try_snapshot()?,
   0, &mut HostSkateboardWheelMassNumeric)`.
3. `SkateboardTruckDefinitions::try_create(&mut attributes,
   wheels.debug_end(), &mut HostSkateboardWheelMassNumeric)`.
4. `SkateboardDeckDefinition::try_create(&mut attributes,
   &mut HostSkateboardDeckNumeric, trucks.debug_end())`.
5. Selected `try_create_skateboard_joints` with `HostSkateboardJointNumeric`,
   `initialize_skateboard_culling_table`, retained
   `SkateboardCollisionDefinition`, and `try_bind_skateboard_assembly`.
6. `try_initialize_skateboard_transforms` with
   `HostSkateboardInitialTransformNumeric`, then
   `SkateboardAssemblySession::try_new(template, initial.by_part().to_vec())`.
7. `SimulationSkaterAssemblyCandidate::try_new`, then actual
   `world.try_insert_board(candidate, 4, bend_threshold)` with the new owner
   registration binding supplied at the proper source phase.

After admission, retain `SkateboardTruckDrives::new(try_calc_truck_transforms
(...))`; call `world.try_initialize_board_truck_drives` with the same reader
and `HostSkateboardTruckDriveNumeric`. Sample Wheels key
`0x69cef3ca3f1480b0` at the native optional-wheel-drive point. Its nonzero branch
is not implemented: return an explicit unsupported error retaining the admitted
board/truck-drive prefix, not an early reordered sample or silent omission.

Next use `SkateboardHookConstruction::new/try_prepare`, its real template in
`DynamicAssemblySession::try_new(..., vec![FRESH_VOLUME_TRANSFORM])`,
`SimulationHookAssemblyCandidate::try_new`, and `world.try_insert_hook`.
Call `world.try_connect_registered_hook` with the actual retained frames and
dynamics, then `world.try_set_registered_board_transform` with the supplied
spawn and current `SkateboardSpawnServices`. Source board spy registration for
Parts0..6 follows Set, not initial attachment. Retain the real hook construction,
candidate/handle, truck-drive owner and board candidate/handle on failure.

Remaining concrete board gaps: `mRemainOnReset`, retained wheel distance, and
native post-initial-transform standard/wipeout/grind material and mass-cache
reads are not a complete live inventory in `SkateboardCollisionDefinition`.
Do not call this composition complete SkateboardBody::Init. The source debug
Instance construction between descriptor allocation and wheel-distance lookup
also lies outside the existing geometry-only composition. Existing wheel
sampling explicitly uses immutable Vault snapshots, not mutable native timing.

### Skeleton phases must not copy the corpus's reordered setup

`with_mounted_original_normal_skeleton` front-loads attributes for test
convenience. Native order is SkeletonData, then SkeletonBody's collision-data
constructor, then InitAttribData and geometry/joints/culling. In particular,
`SkeletonBody` invokes `SkeletonCollision::SkeletonCollision` before
`InitAttribData(0x82b3c3b8)` and `Init(0x82b61738)`.

After board completion, invoke:

1. `SkeletonData::try_new` with the retained actual hierarchy/parameters/pose.
2. `SkeletonCollisionData::try_new` using
   `SkeletonCollisionAttributes::bind(&vault)`.
3. `SkeletonBodyAttributeData::try_read` using
   `SkeletonAttributes::bind(&vault, skeleton_body_attributes::DEFAULT_COLLECTION)`
   and `skeleton_attributes::SkeletonDriveAttributes::constructor_value()`.
4. Retain `SkeletonBodyAttributes` and `SkeletonJointAttributeProvider` bound
   to that same default collection. Retain `HatAttributes::bind(&vault, hat)`;
   supply its cached layout for the actual nonzero-hat branch.
5. `SkeletonBodyDefinition::try_create` from the SAME data/attributes Arcs,
   body reader, hat/layout and live `FnMut()->bool` enable callback, using
   `HostSkeletonBodyMassNumeric`, `HostSimulationVolumeIntegralNumeric`,
   `HostHatNumeric`.
6. `SkeletonBodyJoints::try_create(&body.data, &body.attributes,
   body.allocated_joint_count, &mut joint_reader, &mut HostSkeletonJointNumeric)`.
7. Retain `SkeletonCullingInventory` normal and ragdoll resources; pass its
   actual normal Arc into `SkeletonNormalAssembly::try_create` with body,
   joints and collision data.
8. `SimulationSkeletonAssemblyCandidate::try_new`, then actual
   `world.try_insert_skeleton` in the SAME world.

This closes the present same-data/body/joints caller invariant through real
construction instead of validating unrelated prepared snapshots. Native IK and
head-ancestor operations between SkeletonData and Body creation remain excluded
from this bounded assembly projection.

After actual registration of Parts0..23, existing narrow world consumers are:

- `try_set_skeleton_pre_normal_properties` (joint spy then root zero material);
- `try_set_skeleton_normal` with retained body reader and explicit host cosine;
- source constructor TeleportReset: existing retained owner method exists, but
  a narrow managed-world operation is still needed;
- `try_initialize_skeleton_drives`, using the DIFFERENT concrete provider
  `skeleton_drive_attributes::SkeletonDriveAttributes::bind(&vault)`,
  `HostSkeletonDriveNumeric`, `HostSkateboardWheelMassNumeric`, and current
  `SkeletonDriveCvars`;
- `try_set_skeleton_pose_with_hooks`;
- `try_initialize_physics_history` using current `SkeletonHistoryInput` and
  `HostSkeletonPhysicsHistoryNumeric`.

Keep `SkeletonDriveCvars`' ordered hard-displacement/strength calls and
`SkeletonHistoryInput`'s acquire/cvar/lazy-up-vector contract. Keep both live hat
enable samples. None becomes a hardcoded constructor profile. Existing host
numeric implementations retain explicit reciprocal, sqrt, trig and dot-reduction
limits; they are not a Xenon bit-parity claim.

Recommended API phases retain actual progress and use mutable methods returning
errors without consuming the construction. Do not automatically retry an entire
partially completed phase and repeat source reads or allocations. Pool settings,
bend-threshold sampling, actual Island registration, full processed-input
construction, remaining HiLOD services, bouncy behavior and complete Actor pose
selection are explicit remaining prerequisites, not empty collaborator stubs.

### Additional live code hashes

SHA-256 of the indicated live IDA code ranges:

```text
Entity::GetPhysicsIsland 825f1758..825f180c 40a18246097dfd814f80bd9ddd44ee6b0f679ea7157d82798534eddee128e362
PhysicalPlayerHiLOD ctor82e03c50..82e04ddc 3acf66b57c929cabff86c71671972c9eb254fa93716205e8d9d91ff0ebbd8ded
PhysicalPlayerHiLOD dtor82e00f88..82e01680 af4b103bd8c379f9ae670dd271087116232e20e6b51cde5afa4b995a554d4fc8
Skeleton dtor82b574f0..82b575d0 444a75c7b00e69a80675adfeb200928c44b37a621096112b7642e8f7b7077bd8
Skateboard dtor82b352c8..82b3537c 5aa6213a6fa587cb416111e2a81aea0cdce17b06965f8b134f85ca3fa755e685
```

Validation for this addendum: live IDA decompilation, raw constructor-prefix
instructions, PDB fields, and read-only inspection of current production APIs
and corpus compositions. No new runtime trace, constructor execution, Rust edit,
Cargo run or formatter run was performed by this task.

## Board Init material suffix correction and source packet (2026-09-07)

**Correction to the earlier API addendum:** the post-initial-transform suffix
of `SkateboardBody::Init` initializes retained MATERIAL records. It does NOT
calculate, fetch, or store a mass/inertia cache. Earlier references to missing
"material/mass-cache reads" overstated this source. Existing component mass
producers are separate. This section supersedes that terminology; no speculative
mass-cache producer is required to fill this constructor gap.

### Exact source position and initial defaults

Full live function `0x82b374e8..0x82b37b84` was read as raw PPC and decompiled
with PDB field names. Chronology:

1. Joint-count byte, definition allocation, local debug Instance construction,
   wheel-distance read/store, geometry, selected joints, culling.
2. Assembly Initialize at `0x82b376fc`; InitializeTransforms at `0x82b37708`.
3. Assembly group4 store at `0x82b37720`; all actual Part userdata4 stores in
   ascending Part order at `0x82b37738..44` (Part stride `0x60`, userdata `+0x5c`).
4. Eleven lookup/load pairs below, then all cached material stores.
5. Destroy the temporary debug Instance at `0x82b37b70`, then return.
6. Enclosing SkateboardBody constructor calls AddSkaterBody(Active4,bend=true)
   at `0x82b38808`, SyncCheckBody at `0x82b38820`, CalcTruckTransforms at
   `0x82b38828`, CreateDrives at `0x82b38830`, optional wheel drives, mandatory
   hook, Set and spy registration. Thus every material suffix read/store is
   BEFORE admission/truck drives/Set, not a later constructor phase.

Before calling Init, the enclosing constructor initializes all SEVEN cached
materials at `0x82b38748..0x82b387a0`, in table order below, each with static
friction then dynamic friction then restitution:

```text
static_friction = bits 3f000000
dynamic_friction = bits 3e99999a
restitution = bits 3ecccccd
```

Constant loads are `0x82b38628`, `0x82b3862c`, and `0x82b386f8`, from the
corresponding IDA `__real_*` globals. These are constructor defaults, NOT the
Vault fallback values. Missing scalar pointers in Init use DefaultDataArea(4),
which is zero instead. A safe host error during the eleven reads should leave
the already constructed material inventory at its constructor values, not
partially commit previously read values.

### Eleven exact scalar reads

Each entry is `GetAttributePointer(class.default,key,index0)` followed by null
fallback to DefaultDataArea(4), then one scalar `lfs`. Each scalar load happens
before the next GetAttributePointer call. The singleton pointer is reloaded
between accesses; the immutable retained Vault branch explicitly excludes
concurrent global-rebinding/editor mutation.

| Order | Class | Key | Lookup VA | Load VA | Destination meaning |
|---|---|---|---|---|---|
| 1 | physicswheels | `7d777041dbfa9234` | `82b37788` | `82b377a0` | wheel static friction |
| 2 | physicswheels | `2a1082ddfeef3ea7` | `82b377e0` | `82b377f8` | wheel dynamic friction |
| 3 | physicsdeck | `254d958a72d59a2d` | `82b37838` | `82b37850` | deck restitution |
| 4 | physicswheels | `5d095e6365260b84` | `82b37890` | `82b378a8` | wheel restitution |
| 5 | physicstrucks | `329805a82950ab54` | `82b378e8` | `82b37900` | truck restitution |
| 6 | physicsdeck | `aae143f564f263aa` | `82b37940` | `82b37958` | deck static friction |
| 7 | physicstrucks | `b34a64870f3f5944` | `82b37998` | `82b379b0` | truck static friction |
| 8 | physicsdeck | `ac32e9a7f0056c6d` | `82b379f0` | `82b37a08` | deck dynamic friction |
| 9 | physicstrucks | `7e135b944834707e` | `82b37a48` | `82b37a60` | truck dynamic friction |
| 10 | physics_wipeout | `990448fc235751f6` | `82b37aa0` | `82b37ab8` | wipeout static AND dynamic friction |
| 11 | physics_wipeout | `d0fb2f7d77b0bea4` | `82b37af8` | `82b37b0c` | wipeout restitution |

All classes use collection `d7edbd362d7d2152`. Actual class keys:

```text
physicswheels   46add2eea90cc062  GlobalAttributeCollection+9c
physicsdeck     9ed291a274453fc0  GlobalAttributeCollection+94
physicstrucks   af781d7b614dab50  GlobalAttributeCollection+dc
physics_wipeout eb844cfdbf928efd  GlobalAttributeCollection+108
```

There are NO floating-point calculations in this suffix: no multiply, divide,
FMA, friction ordering correction, finite check, absolute value, or clamp. The
source scalar load/store graph is the numeric boundary; avoid claiming host
signaling-NaN payload/exception parity. Preserve raw values rather than applying
the distinct material checks used by an earlier geometry constructor.

### Retained material layout and final store order

Each native `rw::physics::Material` is three floats in order mus/mud/res,
directly represented by existing `AssemblyPartMaterial { static_friction,
dynamic_friction, restitution }`.

| Body offset | Native owner field |
|---|---|
| `2030` | mWheelMaterial |
| `203c` | mDeckMaterial |
| `2048` | mTruckMaterial |
| `2054` | mWheelGrindMaterial |
| `2060` | mDeckGrindMaterial |
| `206c` | mTruckGrindMaterial |
| `2078` | mWipeoutMaterial |

Only after read11 completes does the source store:

1. Wheel restitution/static/dynamic at `82b37b14/1c/20`.
2. Deck restitution/static/dynamic at `82b37b24/28/2c`.
3. Truck restitution/static/dynamic at `82b37b30/34/38`.
4. Wipeout restitution at `82b37b3c`, then static/dynamic from the SAME read10
   value at `82b37b44/48`.
5. Wheel grind restitution/static/dynamic at `82b37b4c/50/54`.
6. Deck grind restitution/static/dynamic at `82b37b58/5c/60`.
7. Truck grind restitution/static/dynamic at `82b37b64/68/6c`.

All nine grind scalar stores use the explicit positive-zero constant loaded at
`82b37b40`. They are not copied from the normal material or multiplied by zero.

### Ownership matters: these are not the initial Part material aliases

Init's suffix writes Body-owned material records but DOES NOT rebind
`Part.m_material`. Initially the Assembly retains the component definition's
actual material source. Later functions explicitly change the pointers:

- `Skateboard::SetMaterialsForGrind(82b29ee8..82b29f50)`: deck Part6 uses the
  retained deck-grind material; wheels0..3 share wheel-grind; trucks4..5 share
  truck-grind.
- `RestoreMaterialsFromGrind(82b29f50..82b29fb8)`: same source order, using the
  three normal Body-owned material records.
- `RestoreWheelMaterial(82b29ff8..82b2a034)`: only Parts0,1,2,3, ascending.
- `SkateboardBody::SetStandard(82b32d70..82b32e08)`: clear free flag, invoke
  SetStandardAngularDamping, then select normal Body-owned material records.
- `Skateboard::SetFree(82b30790..82b30830)` and
  `SetWipeout(82b30830..82b308bc)`: change free/damping state then bind all seven
  Parts to the SAME retained wipeout material; subsequently enable collision.
  Their whole behaviors are NOT implemented by material-record construction.

For all seven-Part rebinding paths above, pointer-store order is
`6,0,1,2,3,4,5`. The raw grind/restore functions reload Assembly and part-list
pointers between stores; targets are Part material `+0x50`, with `0x60` stride.
No copy into the immutable geometry definition is performed by these bindings.

Current `skateboard_collision_publication.rs` obtains material from immutable
`attachment.template.payloads[ordinal].material` (its live-part collection),
and copies it on Create/Update. Retaining this new cache alone is correct for
the constructor projection, but cannot claim grind/wipeout material switching.
That later connected change needs a single actual live selector authority
(Definition versus one of the seven retained records), invalidates publication,
and feeds the existing publication/ground consumer. Do not preemptively replace
initial definition materials with the cached normal material.

### Wheel distance and remain-on-reset lifecycle

`mWheelDistFromDeck` is a float at Body `+0x2094`. Its source Init assignment
is exactly the earlier Trucks lookup/load/store `82b37670/84/8c`, with no
arithmetic. No separate earlier default assignment was identified in the full
SkateboardBody constructor. An error-preserving host constructor can represent
this as absent until that load succeeds, then retain the actual f32.

Direct consumer `CalculateAverageWheelCompressions(82b2b538..82b2b818)` reads
the field at `82b2b744`. It computes wheel positions in the deck body frame and
uses this stored distance in the final compression subtraction, storing front
and back values at `82b2b7f4/7f8`. This identifies the actual lifetime/consumer;
its full VMX numeric graph is NOT independently recovered or approved here.
Do not substitute the constructor value for a complete compression producer.

Base `Body::Body(825ea770)` initializes the one-byte `mRemainOnReset+0x2c`
to false at `825ea7e4`. SkateboardBody does not set it true in its own constructor.
HiLOD obtains the completed board's Body at `82e0417c` and writes true at
`82e04188`, AFTER board admission/drives/Set/spy registration and BEFORE Skeleton
construction. Thus a whole paired builder must begin with false and perform
this transition when its board constructor phase returns. Retaining true from
the outset changes the source failure-prefix state. Actual Island reset-list
consumption of this field is outside this bounded packet and is not claimed.

### Minimal producer/provider addition, not a new generic subsystem

The exact retained data can be seven named `AssemblyPartMaterial` fields plus
`Option<f32>` wheel distance while incomplete and a bool remain-on-reset. Store
them in the actual board construction/runtime owner, not a discarded helper.
An `Init` material-read method may accumulate the eleven local scalar values
before committing its source-ordered writes; errors before read11 preserve the
constructor material values. A separate earlier method samples the wheel
distance. A later board-completed method sets remain-on-reset true.

Reuse `SkateboardAttributeRead` lookup/float with element0. The existing
`SkateboardAttributes` covers Wheels/Deck/Trucks but NOT Wipeout; add a bounded
Wipeout class/binding to that actual provider, or a constructor-specific retained
wipeout Instance with the SAME runtime lookup/default semantics. Do not import
the skeleton body reader merely to disguise missing board ownership. The
existing `skeleton_body_attributes::WIPEOUT_CLASS` independently records the
same actual class key; no new generic Vault parser or frozen tuning is needed.

High-value tests: eleven distinct ordered lookup/load pairs; failure after each
read leaves seven constructor triples intact; deliberately inverted/negative
frictions copied unchanged; shared wipeout source used twice without reread;
negative-zero/NaN inputs retained while grind outputs are positive zero;
constructor remains false until actual board-completed phase; initial Part
material selection stays Definition, distinct from normal cached records.
Original mounted values for every new key should be independently inspected
before a corpus oracle is frozen. This task did not derive expected tuning by
printing producer output or run a new mounted constructor test.

### Hashes and validation boundary

```text
Init82b374e8..82b37b84 bf4f1b2daafff14bfb7536615e13b17a7f8b0a35c8ae8590c37949351e998657
SetMaterialsForGrind82b29ee8..82b29f50 78d597d9b982aabd1f0330866d7cf779095530e7f836e981de4520d3e3516b47
RestoreMaterialsFromGrind82b29f50..82b29fb8 5126cdd7f1f5b198559c4356cb4a1072d0fff54dcce8ead38c8ec752b7b30743
RestoreWheelMaterial82b29ff8..82b2a034 02bc20619fa2b9b6a3a5c2a72a1b9aebc3eaab71b0b4e9588b27df5035a320d8
SetStandard82b32d70..82b32e08 e5d28d7542eac93e799b72f6ff6d4369c3cc55840fca9005b3eababc85432aa1
SetFree82b30790..82b30830 d38bb6b99161b54056c21efbdb748bc13271350efab2e65dfb73b877d30eb04d
SetWipeout82b30830..82b308bc b7aaec75de664481effb091fbe402868ed1edf7b760e733c4146bdba3e31d00b
CalculateAverageWheelCompressions82b2b538..82b2b818 120d1f9d54410cda9f4383da6086bbad02b57adae6b34135dad8f5ec4d8f3246
```

All hashes are live IDA code-byte SHA-256. This packet's Init read/store sequence,
material layouts and constructor lifetime stores are source-ready, pending
independent implementation audit and mounted scalar oracle. Consumer identities
and alias relationships are recovered; their complete gameplay behaviors are
not newly implemented. Only this evidence file changed; no Rust/Cargo/formatting.

Independent raw-source gate PASS at checkpoint2567: reviewer recomputed
all eight hashes above, checked the complete Init suffix, all eleven key halves/
class offsets/load sites, constructor defaults, 21 final stores, absence of
arithmetic/Part rebinding, the later pointer-selection order, and false-to-true
remainOnReset timing. This approves source evidence, not an implemented cache,
paired constructor or complete gameplay material-switch path.

## Independent original-archive board scalar oracle (2026-09-07)

This section records an author-separated, read-only data oracle for the eleven
Init suffix reads and three earlier/optional reads. It does not derive expected
values from the new Rust board constructor and makes no new implementation,
native execution, live mutable-Vault, or complete player-construction claim.
The selected serialized words and metadata are `exact` for the identified files.

### Original bytes and independent inspection method

The original external source was
`<game folder>\data\db\db.big`.
Its named `BIG4` header directly identifies:

| Archive entry | Absolute archive offset | Byte length |
|---|---|---|
| `data\db\skater.vlt` | `0x00000100` | `0x000cfe70` = 851568 |
| `data\db\skater.bin` | `0x000cff80` | `0x000c65e8` = 812520 |

SHA-256, computed from those raw archive byte ranges in memory:

```text
skater.vlt 496215B210C3CE8BDA3F7A64B3CC3D59C56546E8D3026DBF1DA4A72F93FA7DDE
skater.bin 0D20A6CB05A21CF1F523478132319EA4B6BDFECA7FB29C7959A8BA069DCFE7AA
```

A read-only PowerShell script decoded the big-endian VLT chunk envelopes,
`ExpN` records and selected `CollectionLoadData` headers directly from the raw
archive range. Selection required collection `d7edbd362d7d2152` and the exact
class keys below, rather than scanning unrelated occurrences of an attribute
key. Entry addresses were calculated as
`collection_offset + 48 + types_length * 8 + entry_ordinal * 16`.

The existing read-only `inspect_vault.exe` additionally parsed the external
extracted VLT/BIN pair under `data\db\unpcaked#\data\db`, using the existing
formats metadata/pointer APIs to inspect selected class definitions and node
types. A separate byte comparison confirmed that ALL fourteen selected
16-byte nodes in that extracted VLT equal their raw archive counterparts.
This is not a claim that the entire extracted files equal raw mounted files:
the previously identified relocated-pointer differences elsewhere still exist.
The hashes above identify raw archive ranges, not that extracted pair.

Raw `PtrN` records were independently traversed with `SelectTargetBase` handling.
There were zero pointer-fixup slots overlapping any selected inline value's
four-byte range `[node + 8, node + 12)`. Thus these particular raw scalar/byte
payloads do not require relocating a native pointer before interpretation.
No producer output, new Rust code, Cargo invocation, or new extracted asset
was used to obtain this oracle.

### Selected collection and type provenance

All four selected default collections have parent key zero. Every selected
node below has node flags `0x40` (inline) and entry flags zero.

| Class | Class key | Collection VLT offset | Entry count | Type-vector length |
|---|---|---|---|---|
| physicswheels | `46add2eea90cc062` | `0x0005b5e8` | 13 | 2 |
| physicsdeck | `9ed291a274453fc0` | `0x0005b0f0` | 27 | 4 |
| physicstrucks | `af781d7b614dab50` | `0x0005b398` | 16 | 2 |
| physics_wipeout | `eb844cfdbf928efd` | `0x0005f360` | 83 | 4 |

Float nodes resolve their serialized type index to `e22228fbb3c209d8`;
their matching class definitions have size4, maximum_count1, flags4,
alignment2. The joint-enable and optional-wheel-drive nodes instead resolve
to `15c52cb4069556a3`, with size1, maximum_count1, flags4, alignment0.
These are explicit default-collection entries, not inherited or zero-fallback
results. Numeric zero in the table below is still a found explicit value.

### Eleven suffix words, in native lookup/load order

Offsets here are relative to the beginning of `skater.vlt`. The value is at
`node + 8`; its absolute `db.big` offset is therefore `node + 0x108`.
All values are hexadecimal IEEE-754 single-precision words, without decimal
rounding or any friction-order normalization.

| Read | Class | Key | Node VLT offset | Value word |
|---|---|---|---|---|
| 1 | physicswheels | `7d777041dbfa9234` | `0x0005b6c8` | `3f4ccccd` |
| 2 | physicswheels | `2a1082ddfeef3ea7` | `0x0005b688` | `3f333333` |
| 3 | physicsdeck | `254d958a72d59a2d` | `0x0005b210` | `00000000` |
| 4 | physicswheels | `5d095e6365260b84` | `0x0005b6b8` | `00000000` |
| 5 | physicstrucks | `329805a82950ab54` | `0x0005b438` | `00000000` |
| 6 | physicsdeck | `aae143f564f263aa` | `0x0005b220` | `3dcccccd` |
| 7 | physicstrucks | `b34a64870f3f5944` | `0x0005b458` | `3f666666` |
| 8 | physicsdeck | `ac32e9a7f0056c6d` | `0x0005b170` | `3dcccccd` |
| 9 | physicstrucks | `7e135b944834707e` | `0x0005b408` | `00000000` |
| 10 | physics_wipeout | `990448fc235751f6` | `0x0005f530` | `3eb851ec` |
| 11 | physics_wipeout | `d0fb2f7d77b0bea4` | `0x0005f540` | `00000000` |

Combining these independently read words with the already audited source
store graph gives the expected `(static_friction, dynamic_friction,
restitution)` cache triples:

```text
wheel   [3f4ccccd, 3f333333, 00000000]
deck    [3dcccccd, 3dcccccd, 00000000]
truck   [3f666666, 00000000, 00000000]
wipeout [3eb851ec, 3eb851ec, 00000000]
```

All three grind triples are `[00000000, 00000000, 00000000]` because the
audited code explicitly stores positive zero, not because those values were
looked up in the archive. The wipeout friction word supplies both friction
fields from one read. These expectations do not replace the initial Part
material bindings with the cached records.

### Earlier and optional reads

| Purpose | Class | Key | Node VLT offset | Raw inline word | Actual consumer value |
|---|---|---|---|---|---|
| Joint enable | physicstrucks | `b1cbe3e929afc8a0` | `0x0005b3d8` | `01000000` | first byte `01` |
| Wheel distance | physicstrucks | `2be08dd7a9207622` | `0x0005b488` | `bd676c8b` | f32 bits `bd676c8b` |
| Optional wheel drives | physicswheels | `69cef3ca3f1480b0` | `0x0005b668` | `00000000` | first byte `00` |

Complete original 16-byte node bytes for these three discriminators:

```text
b1cbe3e929afc8a00100000000004000
2be08dd7a9207622bd676c8b00014000
69cef3ca3f1480b00000000000014000
```

The joint-enable assertion must inspect the one-byte value, not equate the
four-byte big-endian inline word with integer1. Likewise the optional-wheel
branch is explicitly disabled by the original byte, not by an absent lookup.

The minimal mounted-test application is to add the actual Wipeout class to
the existing selected-class Vault graph, extend the existing original board
test with these independent lookup assertions, and compare retained constructor
values against the source-derived triples. No additional ignored test, copied
runtime tuning, or implementation success is established by this evidence
append alone; the integrator owns subsequent execution and validation claims.

## Implemented pre-admission board producer, checkpoint2578

2026-09-07. `skateboard_body_construction.rs` now constructs and retains the
actual pre-admission board components. This is high structural confidence,
not complete Skateboard/HiLOD, Actor, Entity ownership, or native numeric-trace
parity. Source category/descriptor allocation and temporary debug Instance
lifecycle remain explicitly excluded; no invented allocator/debug service is
substituted. Existing component constructor allocation/numeric policies remain.

`SkateboardBodyConstruction::new` retains actual board and wheel Vault providers,
the seven native material constructor defaults, absent early scalars, and
remainOnReset=false. One `try_construct` attempt executes early joint byte and
wheel distance, wheels/trucks/deck, conditional0/6 joints, culling/template,
actual fresh seven-Part Session, initial placement, group4/userdata4 proof,
all11 ordered material reads, then the original final store sequence. The actual
completed component outputs/session survive a later error; a failed existing
leaf keeps that leaf's declared partial-output policy. No blind retry is offered.
The Wipeout class was added to the existing retained board attribute provider,
using its same element0 lookup/typed null-default semantics.

### Actual fresh Session and incremental placement

The legacy initial-transform value helper previously withheld all output on
failure. It now delegates the SAME numeric/read graph to
`try_initialize_skateboard_transforms_with_writer`, collecting into private
scratch for legacy callers. The new real constructor instead creates its Session
FIRST, then supplies `try_set_detached_part_transform` as the writer. Each
successful raw Part cache write becomes part of the actual retained Session.

Independent raw source confirms fresh native matrices are exactly
FRESH_VOLUME_TRANSFORM, including all W=+0: PartInitialize82ae8fc8 stores
I/Pzero/J/K at82ae9020/24/28/2c; globals8227e8f0/e900/e910 contain the unit axes.
The detached SetTransform branch at82b254b0 jumps directly to raw four-vector
copies82b25b70..8c. The host detached-only setter refuses any attached/partial
body membership before copying all16 values, without backend/inverse/finite math.
That conservative whole-session check is host policy.

Writer chronology: deck6 at82b2bc8c BEFORE first lookup; wheels at82b2bf2c,
82b2c008/0e8/1c4 immediately after their respective reads; truck4 at82b2c270
BEFORE SinCos82b2c28c; truck5 at82b2c36c AFTER final vertical read82b2c338.
No later read/write runs after a checked writer error. This closes the previous
initial-transform partial-session visibility gap for the new actual constructor,
without claiming native allocation faults, mutable Vault timing or Xenon math.

### Real candidate/pool retention

`SimulationSkaterAssemblyCandidate::try_from_construction` moves the successful
Session and the SAME boxed continuing reader/cache owner together. Transfer is
Complete→HandedOff and cannot substitute an arbitrary prepared session. Failed
construction is returned intact. Existing legacy session-only candidates keep
construction=None; neither kind currently claims known Entity provenance.

Existing Insert, failure return, swap removal and returned candidate retain
both fields. `into_session` now refuses to discard a real construction owner,
returning the whole candidate instead. Narrow pool/world methods read the SAME
retained provider for post-admission CalcTruckTransforms, actual truck drives,
and optional wheel-drive byte. These are explicit composition/sampling APIs;
they do not themselves enforce a full enclosing Skateboard constructor schedule.
Cached materials remain distinct from initial template Part materials; no runtime
grind/standard/wipeout selection is newly implemented. remainOnReset has no new
setter: only future genuine post-registration HiLOD completion may set it true.

The existing mounted hooked-board/terrain path now uses this actual constructor.
It checks independent original scalar/cache words, compares initial matrices,
forces an uninitialized-world admission refusal, then admits the returned owner
into the real world. It checks identical retained allocation across admission,
drives, frame execution and removal, and verifies lossy session extraction is
refused. The no-hook solver controls and other older component probes remain
explicit legacy compositions; no complete native player chronology is implied.

### Validation and remaining scope

Independent author-separated source, production/API and test review PASS.
Seven constructor tests cover early raw-byte interpretation/failure stores,
all22 material lookup/load events and every failure prefix, unsanitized -0/NaN/
inverted friction, positive-zero grind writes, actual missing-node defaults,
and actual malformed-Vault failure retaining owner/forbidding retry/handoff.
Four root tests cover writer failure/read prefixes and the real detached setter.
The writer-prefix tests use a Vec callback; the separate Session tests prove
raw actual-cache copying/refusal. They are not a combined late whole-builder
failure test. A complete successful zero-joint constructor case also remains
untested; its byte decision and source branch are independently checked.

Focused26753 EXIT0: two new writer tests (plus5 unrelated filter matches) and
two detached-setter tests. Focused63939 EXIT0: seven constructor tests and the
extended existing original mounted board test. Initial strict50319 passed the
workspace, stopped on documentation/import/raw-pointer-style and an unfulfilled
line-count expectation. Corrections changed no source behavior. FINAL42310 EXIT0:
fmt/check, workspace2350core+187formats+38app+3symbols=2578unit,6doc,
all-target Clippy-Dwarnings, ALL7ignored original-data core tests, gitdiffcheck.
No app ignored probe rerun. No code/test changes after that gate.

RobPark C/J/D6/6/3,6/6/3,6/6/3,4/6/3 and centers remain
[588.9575,-54.351295,996.21686], [588.9615,-54.343952,996.2112],
[588.9654,-54.340397,996.2055], [588.9701,-54.33957,996.1999].
DMO8 hash0e7c704e6a10f08a and final[588.8619,-54.40114,996.5048] unchanged.
These validate preserved controlled host behavior, not a native per-tick trace.

Next integrate this actual retained producer into source-ordered paired
physical-player construction and Island registration/contact lists. Shared
Entity lifetime must predate each construction/admission; do not attach an empty
owner token to legacy members afterward. Board completion still precedes even
SkeletonData construction. All earlier routing, optional-drive, pose/provider,
cleanup and collision-response boundaries remain in force.

## Paired assembly producer and Island integration, 2026-09-07

Implemented after checkpoint2578; strict validation is recorded in the following
checkpoint entry in RESEARCH_LOG. Independent author-separated source,
production, unit and mounted-test review PASS. Structural confidence is high;
source call sites/bit tests are exact, host arithmetic is not a native tick trace.

`physical_player_assembly.rs` now issues a private unique Arc owner containing
the actual retained Vault, mapping hierarchy/PhysicsParam, separate pose hierarchy
and matrices, spawn, hat/raw bouncy/human inputs, and selected manager authority.
Board and skeleton retain this SAME meaningful owner before their construction.
Identical assets do not merge distinct players. This projects the native shared
Entity equality relation; it does NOT implement Actor/UCOM or full HiLOD.

The constructor exclusively borrows its selected world. Board flags are sampled
at the corrected BEFORE-Init call site above, with unsupported visual routing
explicitly refused. Board definition/retained inventory then actual pool admission
(state4,bend3f7fbe77), Island membership, same-reader truck transforms/drives,
optional-wheel byte, Hook construction/admission/connection, supplied Set, and
seven ascending spy targets run in source order. Nonzero optional-wheel byte
returns Unsupported AFTER the admitted truck-drive prefix; no silent omission.
Hook capacity-null continuation is source-correct: AddDrive82b2e700 returns,
82b2e704 stores its result unconditionally, and the parent continues Set/targets.
The existing optional post-drive slot/guard is retained even for null admission.
Only after checked registration does remainOnReset become true (82e04188).

Only THEN `skeleton_assembly_construction.rs` creates SkeletonData, collision
data, InitAttribData, actual body/joints, normal then ragdoll culling and Assembly.
Actual leaf outputs/readers survive checked errors; inert joint/collision copies
preserve consuming binder failure prefixes and are discarded after successful
handoff, not independently admitted. Culling inventory comes from the SAME world.
Skeleton's separate current flags sample follows its definition at82b62120,
then actual admission(state4,bend-1), membership and24 ascending targets,
joint-spy/root-zero properties, Normal through its SAME retained body reader,
TeleportReset, actual drive provider/construction, full retained-pose Set and
mass/two initial history updates. Live hat/drive/history contracts are unchanged.
Complete names this assembly projection, not omitted whole Skeleton services.

Both constructed candidates reject another manager BEFORE consuming a manager
index or creating bodies. Legacy candidates remain unknown Entity provenance,
not known-null. Board candidates retain their actual Insert requested state so
Island classification cannot be contradicted by a different membership argument.
Skeleton's requested state is its actual fixed4. Returned candidates retain
construction/BodyID; lossy session/normal-owner extraction is refused.

Explicit `try_teardown` destroys skeleton drives/body before board drives,
board Hook and board body. Native board order is truck-drive removal82b2b0dc,
optional-wheel removal82b2b110, hook-drive removal82b2b128, Hook assembly82b2b1a4,
then body82b2b1bc. Existing source drive cleanup is reused. Non-admitted candidate
cleanup is explicit host recovery, preserving attached candidates on checked
refusal; no Drop rollback or manager/BodyID refund. Cleanup retries keep their
completed prefix and cannot restart construction. External manual removal can
make a factory handle stale; teardown refuses rather than inventing repair.

### Scope and validation evidence

Four skeleton prefix tests plus six paired-owner/routing/recovery tests use real
factory issuance and deliberately malformed controlled Vault/hierarchy inputs;
they do not fabricate successful geometry or admitted identities. Four real
Part/pool spy tests cover state-only operations, guards and occurrence reuse.
Focused44158/7417/36813 passed6/4/4 tests respectively.

The existing ignored original-body test now calls the actual paired factory.
Its raw-input helper preserves four original record fingerprints and distinct
hierarchy checks WITHOUT constructing SkeletonData before the board. Mounted
counts are35 actual bodies (StaticWorld1+board7+boardHook1+skeleton24+
skeletonHook2),28 joints(6+22),49 drives(2truck+1boardHook+46skeleton).
Assembly IDs areboard0,boardHook1,skeleton2,skeletonHook3: two Hook assemblies,
not one assembly per skeleton Hook Part. BodyIDs are1/2. All31 actual target
Parts have state12; three Hook bodies and StaticWorld retain state1.

Mounted checks exercise source-order construction, same retained owner/readers,
two actual history samples, private target-corruption/refusal/restoration,
drive-guarded manual cleanup and a second complete pair's explicit teardown,
foreign-manager admission before any ID/body effect, request-state mismatch,
flags changing between board and skeleton, uninitialized pool failure retaining
the constructed board, BodyIDs surviving removal and stale-target refusal.
These are constructor/admission/history checks, NOT a full humanoid solver frame.

The supplied original RAW pose, spawn, tolerance, human/bouncy bytes, drive cvars
and history inputs remain controlled fixtures, not live Actor provenance.
Immutable Vault/pose ownership, host numeric and checked-allocation policies
remain explicit. Eager hat-layout validation is a host preflight before Body's
root/initial reads; native consumes fields later inside CreateHatVolume. No
exact malformed-layout/callback failure timing is claimed. IK/head tracking,
processed input/state/surface/trajectory collaborators, bouncy behavior, full
HiLOD and current live configuration remain open. Contact vectors/response are
the next connected consumer of actual Island targets, not implemented here.

Final strict76097 EXIT0: formatting/check, workspace2364core+187formats+38app+
3symbols=2592unit,6doc, all-target Clippy-Dwarnings, ALL7ignored original core
checks, gitdiffcheck. Initialstrict82960 passed all workspace tests then stopped
on four test-documentation/line-count/module-placement lints. Corrections only
added documented line-count expectations and extracted existing private registry
test helpers into physical_player_island_tests.rs; no behavior changed.
RobPark C/J/D6/6/3,6/6/3,6/6/3,4/6/3 and all four centers remain unchanged;
DMO8 hash0e7c704e6a10f08a/final[588.8619,-54.40114,996.5048] unchanged.
App ignored probe not rerun. All production/tests precede76097; later edits
are documentation only. Full freeroam goal remains active, not achieved here.
