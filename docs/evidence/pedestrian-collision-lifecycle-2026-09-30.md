# Pedestrian collision lifecycle and classification, 2026-09-30

Source inspection only. No compilation, tests or runtime was run.

## Native evidence and implementation

`cLWCollisionBody::AddToWorld` (82e50cb0) positions each detached Part before
pool admission; group10 selects Vehicle, groups11..13 Pedestrian, groups14..16
DMO. Island registration precedes ascending Part spy registration.
`Island::AddPedestrianBody` (827b68e8) inserts into the separate base
EA::Physics::AssemblyPool, sets owner Island, then uses the common static
(state1, bodyID0) or dynamic list and monotonically allocated body ID.
`CleanUpPedestrian` (827b6c88) removes spies and contact entries before base
pool removal; `RemovePedestrianBody` (827b7640) then erases list membership.

The host `pedestrian_collision.rs` retains either a constructed character
candidate or the admitted handle for each world/skater body. Admission,
removal, live Part transform reads and collision-body drive use the existing
physical assembly session. Partial admission failures retain the actual
candidate or admitted handle. World precedes skater for add/remove/destroy.

The shared Island implementation now has a distinct Pedestrian body kind and
handle variant. A borrowed pool adapter exposes the existing common
`DynamicIslandBody` Parts view, allowing the same ID allocator, lists, spy map,
contact ownership and drive routine to serve the base pool without introducing
another registry or publishing DynamicPool line meshes. Owner/group resolution
and skeleton collision counterpart group reads resolve the actual pedestrian
candidate. The separate base pool implementation is owned by the root agent;
its synchronous refresh contract leaves no pending worker job during removal.

`DetermineOverallCollisionType` (82e808d8) is translated in
`pedestrian_collision_contacts.rs`. The six return values retain native values
0..5. World classification updates its Part flag first, distinguishes vehicle
and interacting DMO owners, and preserves cross-product and forward/velocity
gates. Skater classification preserves ignored-owner, state, LOD, chase,
Part-flag, distance and arm-animation gate order; it updates the shared actual
collision flags. The final arm animation predicate remains an explicit input
callback to its actual owner rather than a manufactured animation result.

## Remaining integration boundaries

Native FillWorldBodyTransforms (82e90e30) and FillSkaterBodyTransforms
(82e91530) depend on actual simulation pose, animation/bone and plugin facts.
The lifecycle accepts a typed transform producer callback; no vehicle box
transform or guessed pose is substituted. The manager must supply that real
producer and invoke lifecycle/update at native scheduling points.

`Pedestrian::OnEntityCollision` (82e95158) is not implemented by this change.
Collider selection, light/full reaction animation, knockdown and recovery must
still be connected to actual pedestrian and animation owners. Classification
alone does not establish complete live NPC collision response. No end-to-end
or playtest readiness claim is made.

## Live collision job submission

Direct IDA reads of `Island::InitWorld` (827b42c0),
`Step_UpdatePrePhysics2` (827b28d0), and `Step_Collision1` (827b3818)
confirmed the following configured ordinary CPU worker routes:

| Pipeline | A assemblies per job | Multiple writes | Extra padding |
| --- | ---: | --- | ---: |
| Pedestrian ground | 5 | true | ground Params default |
| DMO/pedestrian | 5 | true | 0 |
| Pedestrian/vehicle | 10 | true | 0 |
| Pedestrian/pedestrian | 5 | true | 0 |
| Skater/pedestrian | 2 | true | 0 |

Pedestrian ground leaves `applyObjectFixes` at its default false; DMO ground
explicitly sets true. The original ground collection count is one (the static
GroundPool); the DMO collection pointer at index1 is not in that count.
Skater/pedestrian uses the ordinary AssemblyAssembly worker, unlike the custom
skater/DMO and skater/vehicle mesh worker.

`skateboard_ground_frame.rs` now submits actual base-pool character
publications through the existing authored ground traversal and shared
assembly-pair kernel. PrePhysics2 order is DMO ground, DMO/DMO, pedestrian
ground, DMO/pedestrian, DMO/vehicle, pedestrian/vehicle, pedestrian/pedestrian.
Collision1 inserts skater/pedestrian before the existing skater/vehicle route.
The existing skater/skater pipeline is not supplied by this patch; native runs
it between skater/DMO and skater/pedestrian.

All producers share remaining contact capacity and the same pair/thaw frame.
Pair/thaw admission remains the shared kernel's order (including A then B
thaw append, no deduplication), followed by the existing activation,
partitioning, solver and contact-spy consumers. Pedestrian ground chunks
retain full-pool source ordinals through the output owner's offset method.
No pedestrian collision kernel or contact registry is duplicated.

Root owns live cvar registration and settings capture. The settings fields
are `run_pedestrian_ground`, `draw_pedestrian_ground_contacts`,
`run_dmo_pedestrian`, `run_pedestrian_vehicle`,
`run_pedestrian_pedestrian`, and `run_skater_pedestrian`.
Native triangle-cache allocation tuning and debug triangle/spew settings are
not newly implemented; this retains the shared host ground worker's existing
cache/traversal behavior. Source/whitespace review only, no compile or runtime.

## Sole roster payload and populated consumers

The existing Manager EntityMan40 now holds `PedestrianConstruction` instead
of `()`. Its existing KD implementation reads the same core `position_xz`
cache as other LivingWorld entities; no second roster/tree was added. The
new `pedestrian_entity_roster.rs` child supplies SpatialEntity plus concrete
horn and cull methods. This migration does not publish a construction prefix;
the factory must finish construction and attachment before `add_retained`.

`GetPedestriansWithinRange`82e9ab88 now uses populated KD traversal, clamps
to40 before the query, clears the result count before traversal, preserves
unreturned output entries, and reports insufficient output capacity.

Direct IDA reads verified `SetHonkingVehicle`82e6ed98 writes the signed
vehicle-ID word represented by `state.honking_vehicle_id`. The live vehicle
horn adapter mutably resolves the same roster; absent pedestrian IDs remain
a no-op as in the native GetPedestrian route. Vehicle destruction now calls
`NotifyPedsAboutCulls`82e84d90 in source map order. Its head owner dispatches
`NotifyEntityCulled`82e6b840: only active stack entries with target kind1 and
the culled ID are cleared to StraightAhead; base and dormant entries survive.

`RAINMan::RemoveRAINData`82727510 invokes the pedestrian component's
`InvalidateAllRAINData`82e82f00. The latter traverses the entire EntityMan and
nulls each mpRAINData, even for surviving entities. The selected removal
adapter now clears actual pedestrian `rain_data` before vehicle/DMO data.

Remaining populated consumers include native pedestrian AddRAINTestPoints,
camera obstacle details, respawn/collision sphere tests and scoring body
endpoints. They require their real producers; the roster migration does not
claim those previously explicit pending routes complete. Source/whitespace
review only; no compilation, tests or runtime was performed.

## Populated sphere and scoring consumers

Important audit correction: IDA decompilation of 82703600 and 827ac840 omits
the entire populated loop after GetPedestriansWithinRange, although the raw
PPC instructions retain those loops. Implementations below were based on raw
instruction reads plus PDB member/vtable layouts, not the truncated decompiler.

`DoesAnyPedestrianIntersectSphere`82703600 broadphases six IDs at radius+2.
For each surviving GetPedestrian result, LWCollideableObject+3c retrieves its
collision owner and collision vtable+28 gets the skater body. Body.GetPosition
part0 supplies the actual matrix. The shared transpose/translate routine
computes local point. The source checks strict Z, X, Y against half authored
length/width/height plus sphere radius, with no vehicle extra Y metre. The
position-debug cvar/overlay requirement occurs before those comparisons.
The new `pedestrian_sphere_query.rs` helper retains this sequence and the
Update0 consumer calls it with AnimToWorld translation raised by radius.
This helper can serve the remaining respawn query too; that callsite is
outside this change.

`CollectNearbyPedestriansForScoring`827ac840 queries eight IDs at radius3,
skips absent entities and invalid core proxies, then requires actual animation
data. PDB tAnimData.SimLODBoneGlobal is Matrix44Affine[9] at6490. Native loads
bone0 translation at64c0, bone7 first/X axis at6650 and bone7 translation at6680.
Torso=head*0.5+hips*0.5 with the second term multiplied before fused addition;
legs=hips-axis0*0.5. In particular the axis at6650 is not the up/Y axis.
Raw vmaddfp827ac9f4 encoding11606b2e has VA0, VB13, VC12 (head*.5+hipsHalf).
Entity ID, core entity key and the player's bit in mSkaterCollisionFlags follow.

The collector now consumes a retained optional SimLodBoneGlobal in the
existing PedestrianAnimation owner. It refuses absent pose data; no bind-pose
or guessed-position substitute is supplied. The actual simulation-LOD pose
producer remains required. The collision bitmask is grouped in the retained
PedestrianStateStorage: native constructor82e97774 sets r30=0 and82e97d38
stores it to entity+1a3c. ResetScoringCollisionFlags82e83648 clears the same
word; ordinary ResetCollisionFlags82e6e550 does not. A concrete manager reset
method is supplied for the scheduler, while OnEntityCollision still must
produce the real player's bit before scoring collection.

Scoped diff/whitespace checks passed. No compilation, tests or runtime run.

## Shared execution scheduling owner

Vehicle execution is now a specialization of `living_world_execution.rs`,
the same generic owner used by `PedestrianExecutionMan` (EntityMan40). There
is one implementation of bucket admission, proxy balancing, remove, rotation,
explicit preframe handling and target collection. Entity payloads remain in
their sole typed roster; group lists retain only IDs.

IDA verified pedestrian Init group82e93088 (60/rate buckets, assert count>1,
dt=1/rate, bucket0 PreFrame), GetCurrentBucketContents82e93210,
GetPreFrameBucketContents82e932e8, FillCurrent82e96c60,
FillPreFrame82e96cb8 and CreateGroups82e9ee10. Init82e9ef00 captures
HiFi/LoFi/NoFi cvars in that order; the rates are supplied by the concrete
getter registered separately. Current collection only visits the CurrentFrame
bucket. Preframe scans other buckets for explicit requests and clears those
flags before append, then appends scheduled PreFrame entries without clearing
their explicit flags. Lists preserve native group/bucket/insertion order.

Target ordering now uses the complete source algorithm through capacity256,
including pedestrian lists above28. quick_sort82e8df98 computes depth
2*floor(log2(n)); quick_sort_impl82d347c8 uses median82d2b198, copied-pivot
partition82e66780, right recursion before left iteration, cutoff28 and heap
fallback. adjust_heap82d1eab0/promote_heap82d1a5c8 compare actual distances
for child/promote decisions; bucket-proxy heap selection retains its distinct
native pointer-order comparison. Final insertion82d1e9a8/82e7c3e8 preserves
equal ties after partition. Raw comparator82e64da4..dac confirms ordered
left-distance<right-distance, with NaN false. Distances are recomputed from
actual core transform positions using the existing numeric owner.

The obsolete vehicle-only refusal above24 targets was removed because the
shared native target array supports256 and source insertion does not suppress
duplicates. EntityMan capacity still bounds actual entity payload count.
Existing vehicle tests were retained/adapted to the shared owner but not run.
Scoped whitespace/source review only; no compiler, tests or runtime invoked.
