# HiLOD constructor tail: retained fields, services and publication

Fresh independent IDA/PDB/raw inspection, 2026-09-12, ground-path agent.
Confidence: **exact** source ordering and fields below; **high** native ownership
mapping. This packet is not a completed constructor/runtime parity claim.
Root owns caller integration and independent source verification. No Cargo run
by this agent; new Rust modules were individually rustfmt formatted.

## Source sequence

`Sk8::Physics::PhysicalPlayerHiLOD::PhysicalPlayerHiLOD` is82e03c50..82e04ddc.
After the final OffBoard_Jump ctor BL82e04c10, pointer publication82e04c1c:

1. Required XAllocator lookup/assert; LivingWorld resource allocator lookup
   BL82e04c50. `PhysicsUtility::CreateCollisionCapsule` BL82e04c60 receives
   radius1 and length1, publishes return at HiLOD+334 at4c64.
2. Three records loop4c80..4cfc: each `SurfaceQueryResult` has position/normal
   four lanes zero, surfaceID0 and validfalse. Results start+180, stride30;
   start positions+210, stride10, all four laneszero; trajectory batch pointers
   +328, stride4, NULL. Padding is not claimed initialized by these stores.
3. Load actual Sleeping+240 at4d00, publish current state+2a8 at4d0c, call actual
   `SetPhysicsState(100)` BL4d10. This is the initial Sleeping→Ground transition,
   not a direct state enum assignment. Existing `try_start_pair_ground_state`
   already preserves its actual retained pair/query/entry operations.
4. On successful return, store float bits3d23d70a (0.04) to Wipeout+ac at4d24;
   then HiLOD.mStateIntendedJumpVel(+110) zero vector store4d28.
5. Optional INetworkGameQuery+314: if nonnull, virtual0 call4d44. Present false
   selects world-only; NULL or true selects normal. HiLOD flags+120 bit01000000
   (`mCollideWithWorldOnly`) decides whether the transition calls are needed.
   Set/clear that bit4d6c/4d98 BEFORE invoking board then skeleton controller:
   world-only BL4d70,4d78; normal BL4d9c,4da4. Equal flag means neither call.
6. Required IPhysicalWorldServices+318 Add virtual+8 call4dbc, passing this same
   early registered physical allocation. Only after return: JPhy+174 zero4dc4,
   ctor return4dd8. Actor cached physical pointer store825ff300 follows returned
   physical construction, not allocation or early UCOM registration.

Partial errors must preserve the reached source prefix. In particular failed
board mutation leaves the HiLOD flag changed, failed skeleton mutation leaves
board changes, and failed Add must not publish the final return/Actor cache.

## Capsule resource constructor

`PhysicsUtility::CreateCollisionCapsule`82e4afa0 raw corrects misleading decompile
register reuse: descriptor[0] size60, alignment10 from afb8/afbc. Remaining four
descriptors are size0/alignment1. Allocator virtual+10 `DoAllocate` BLb01c,
debug name CapsuleVolume, then `CapsuleVolume::Initialize` BL82e4b02c.

Initialize82ad7fa0 returnsNULL if resource base0NULL. Otherwise it sets vtable
from Volume::vTableArray[2], group/surface0, flags1, radius parameter, capsule
halfheight parameter; I/J/K vector axes and all-zero translation. Initial
radius0 store precedes final radius parameter. Union spare words+48/+4c are
not written. Existing serialized `Volume` requires all three shape words;
new `IntersectionCapsule` retains only the proven active member rather than
inventing spare values. Future projection must retain this distinction.

This resource allocation is a real first-skater constructor dependency. It does
not create a LivingWorld object or authorize parked gameplay expansion.

## Actual network and World service producers

Actor825fe850 obtains the actual Simulation singleton and supplies interfaces
to `Init_IPhysical.simulationdata`: Simulation+0 INetworkGameQuery, Simulation+4
IPhysicalWorldServices. It supplies actual Actor IActorQuery, actual physics
World, Simulation workspace LivingManager, and real human/player query result.
HiLOD copies those bindings; they are not abstract constants attached later.

`Simulation::IsNetworkGame`8289e448 reads current
`CommonBackEnd::sGameInfo.isNetworkGame` (PDB GameInfo+293).
`GameInfo::GameInfo`82559418 explicitly stores false825594d4. This provides a
source constructor producer for initial false; it is not inferred from the
word offline. New `PhysicalGameInfo` retains that selected mutable field in an
Arc owner. The service queries it each time. Remaining GameInfo fields and
later gameplay writers are not claimed implemented. A future actual writer
must mutate this same field, not replace the service with a cached answer.

`Simulation::Add`828a7c98 adjusts interface this by-4, requires its actual World,
then loads Simulation+ c8 = Workspace(+a4).mPhysicsWorld(+24). It loads
World.mPhysicalCollection(+2c) and appends the SAME IPhysical pointer. Raw
7cfc advances end before7d04 pointer store. Full capacity enters DoInsertValue
at7d10. The decompiler's LineBatch/fixed_vector15 type is wrong: fresh PDB
`PhysicalCollection : Collector<IPhysical,9> : fixed_vector<IPhysical*,9,false>`.
There is no dedup, map replacement or Actor registry lookup in source Add.

World ctor827b15f0 initially nulls collection+2c at1654. After AttribulatorHelper
and collision-vtable/tolerance setup it allocates34 bytes PhysicsSystems at
BL827b1754. Inline constructor raw1760..1774 sets begin=end=object+10 and
capacity=object+34 (nine pointers), then publishes1798. AllocationNULL publishes
NULL, distinct from a successfully constructed empty collector.

New `PhysicalWorldServices::construct_collection(manager,game_info,allocated)`
models that selected actual World owner stage; caller must retain it before
Actor construction and bind it to the same world occurrence. Collection stores
weak allocation identities (source nonowning pointers) to avoid an Arc cycle.
Reads require every owner still exists; no missing entry is silently skipped.
`try_add` checks the actual physical owner's manager, preserves insertion order
and duplicates, and exposes capacity overflow as an explicit boundary. This
packet does not recover the fixed-allocator overflow path, World removal, or
physical update iteration. First actual skater insertion fits the nine slots.

## Board and controller mutations

Fresh board source `CollideWithWorldOnly`82b26880 sets actual Body assembly group2
then every actual Part.userData2. `CollideNormally`82b268c8 does the same with4.
These are not collision-volume group IDs. Root is implementing the actual same
pool owner mutations and collision-publication consumption separately.
Existing `try_pair_skeleton_collision_transition(StartWorldOnly/StopWorldOnly)`
uses the actual retained SkeletonController and normal assembly; reuse it after
the board mutation, without replacing the controller flag or collision result.

## Native module/caller requirements

New `physical_player_hilod_tail.rs`, proposed assembly child `hilod_tail`:

- Actual owner `Mutex<PhysicalHiLodTail>` initialized only for source Sim-cleared
  HiLOD allocation; its flag is initially knownfalse from that allocation.
- After all26 state publications, `try_initialize_pre_ground(capsule_allocated)`
  publishes the nullable capsule then allthree records exactly once.
- Root schedules actual initial Ground API, Wipeout+ac store, then tail
  `initialize_post_ground_jump_velocity`.
- Query optional retained Simulation interface at the actual suffix; pass
  None only for actualNULL. `select_network_collision` publishes flag and
  returns required change; root performs board then controller mutations.
- Root calls actual retained World service `try_add` with the SAME early Actor
  allocation; only success permits world_added, final actual JPhy0, returned,
  then cached Actor physical publication. Tail booleans are stage ledgers, not
  substitutes for performing those operations.

New `physical_player_world_services.rs`, proposed constraint-session child
`world_services`: source GameInfo network cell plus actual World collector.
Root supplies selected constructor stage ownership and same-world bindings.
Existing animation `is_network_game` callback response cannot stand in for
this live Simulation occurrence. Existing Actor early registry, actual skater
pair collection, and physics assembly manager are distinct from this IPhysical
collector; none alone proves Add executed.

Validation needed after root integration: actual corpus source ctor path with
all26 states; nonnull false query causing board2 and controller world-only;
real collector contains exactly same allocation; source Add→JPhy→Actor cache
order; null capsule still reaches line record initialization; null collector
stops final publication; previously demonstrated cachedNone Action failure
remains valid before this complete tail, and a separate actual finished Actor
can pass that guard without a fixture-written cache.

## Board mode implementation packet

Root delegated the actual board mode continuation after the initial read-only
review. `skateboard_collision_groups.rs` owns a distinct Arc per
DynamicAssemblySession. Generic session construction leaves its fields unknown;
the real `SkateboardBody.Init`82b374e8 selected group/Part stores initialize it
only after actual initial transforms. `SkateboardBodyConstruction` retains this
same Arc after its session handoff. Template sharing never shares group state.

Raw CollideWithWorldOnly82b26880: assembly from Skateboard+0c Body+18, assembly
part count+1c and part list+8; assembly group+18 store82b26898; ascending Part
userData+5c stores82b268b0, stride60. CollideNormally82b268c8 is identical with
group4: assembly store82b268e0, Part stores82b268f8. There are no backend calls
or numerical branches inside either mutation.

`dynamic_assembly_skater_board_collision_mode.rs` validates the actual retained
board/skeleton pair and manager, invalidates pool publication, then updates the
same board group owner. `world.try_set_pair_board_collision_mode` is the outer
tail API, returning `skater_pool::board_collision_mode::BoardCollisionModeError`.
The group revision invalidates existing publications before group/Part changes.
`SkateboardCollisionPublication` now reads retained assembly group and each
actual Part userdata during Create/Update, rejects a different owner, and checks
its group revision along with materials and volume flags for bounds freshness.
It no longer hardcodes group4 in either headers or emitted records.

Original-data helper `check_group_publication` exercises source2→4 mutations,
old publication values before refresh, rejected stale bounds, refreshed header
and every emitted record, and an independently initialized group occurrence
remaining4. It restores the source starting mode through the actual setter.
Legacy template-derived corpus sessions explicitly execute the selected Init
group stores before collision publication. A generic session with no such
producer now returns Uninitialized instead of implicitly receiving board4.
These tests were added but await the root Cargo/original-data gate.

## Actual tail orchestration

`physical_player_hilod_tail_session.rs` is a constraint-session child so its
world/pool access retains the existing privacy boundary. The selected API is
`try_finish_pair_hilod_constructor(board,skeleton,actor,numeric,capsule_allocated)`.
It requires the actual `ActorConstruction`, which already implements the real
Ground transition Actor query; there is no supplied success callback adapter.
It validates the pair against this Actor's early registered allocation and the
retained service occurrence, checks all26 constructor sites completed, then
performs the source tail in order. Owner locks are released before nested
Ground, board and skeleton session calls. Mandatory serviceNULL fails at its
actual Add dereference rather than skipping earlier successful effects.

The final source JPhy zero modifies the existing physics state authority. Tail
`returned` is set only afterward. Actor cached physical publication remains the
root-owned source caller operation. Repeating a partial or finished tail is
rejected by the existing pre-ground one-attempt guard; a retry cannot duplicate
World.Add or replay initial Ground entry.

First root compile identified outer board mode impl in the wrong module and a
removed constant still referenced in a kernel fixture. The outer impl moved
to this actual constraint-session child; the fixture now explicitly supplies
its test group4. Both fixes and the new tail session were rustfmt formatted.
No agent Cargo execution occurred.

## Full-constructor closure audit correction

Independent fresh top-level/callee audit found that executing this suffix is
**insufficient** to claim full HiLOD construction. Root added a separate explicit
Skeleton constructor-completion guard before cached Actor publication.

Actual Skeleton ctor82b56d70 must return before HiLOD StateChanger82e04204,
which must return before Skeleton.Init. Current assembly projection schedules
StateChanger, then SkeletonData+Body, then IK constructor. That conflates two
different source lifecycles and changes live attribute-read order.

Required constructor stages include: actual attribute instances, SkeletonState
constructor and source pointers/physics island, inline matrices/scalars,
HeadTracking allocationB0 PhysicsSkater573f0/ctor573fc/publication57408,
IK allocationC80 Sim57420/ctor5743c/publication57448, then distinct Skeleton
SkateboardWobble allocationC0 PhysicsSkater5745c/ctor57468, and suffix fields.
HeadTracking and SkateboardWobble had no native retained owner at audit time.

Actual Skeleton.Init82b62710 sequence: SkeletonData.Init62730;
SkeletonIK.Init6273c; seven named truck/wheel/head lookups62780..628a0 plus head
ancestry; Body6298c; Drives629c8; both24matrix buffers identity; Set62a64;
InitializeMasses62a70; two records62a9c/62ac8; twelve IK blendszero and doIK
false→true; proxy flag/root-volume disable62b08..62b2c. Existing leaf methods
cover important pieces but do not yet represent this whole ordered lifecycle.
The existing SkeletonAnimationTargets starts with None rows, not source Init's
24identity matrices, and outer truck/head indices need explicit retained owners.

New unregistered HeadTracking and SkateboardWobble modules begin this closure:
HeadTracking class58dbc2815ca666fd wrapper82dca680 cached layout48 fallback,
control data+20, PIDFilter<Vector3>82e48e50 (copy initVal twice, zero three
histories), target/rotation/velocityzero and validfalse. Wobble class
1ca5e1dd0b2ea896 copies full80byte PointNegGraphData8 values, tilt key
68d5052a44cd5a16 BLaed4/memcpyaef8, squish93307f84fd4c8c1f BLaf18/memcpyaf3c,
then falsefalse/time0/strength0/mirror1. Both allocations are noncleared
PhysicsSkater; partial states preserve unknown fields and earlier successful
reads. Actual lifecycle integration remains necessary before completion guard.
