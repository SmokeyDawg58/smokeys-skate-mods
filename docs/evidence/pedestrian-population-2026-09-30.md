# Pedestrian census and authored RAIN navigation

Source audit and implementation, September 30 2026. No compilation, tests, or
game launch performed. `rustfmt` syntax parsing
was used on changed Rust modules. This document covers the census/navigation
adapter only; concrete pedestrian, graph, collision, animation and session
integration have separate owners.

## Population

`CensusMan` constructor `827208d0` hashes `pedestrians` into its range field
`+0x68`, samples `kInitialSpawnPedestrianCount` as its current pedestrian
allowance, and initializes reserve to zero. Added optional retained fields to
the existing shared CensusState rather than a second population manager.

Verified original AI_Spawner cvars and linked new definitions/accessors using
the existing registration mechanism:

| Initializer | Name | Initial | Range |
|---|---|---|---|
|82f5d740|kMAXPEDESTRIANS|39|0..39|
|82f5d938|kEnableSpawn_Pedestrians|true|boolean|
|82f5da28|kEnableSpawn_Security|true|boolean|
|82f5dec8|kInitialSpawnPedestrianCount|12|0..36|

`SpawnPedestrian82720fe0` first applies the security switch to security_nsz01,
security_nsz02 and security_street01. NSZ entities additionally require an
authored WorldPainter layer 6 region. World enable bit 0x80 follows, then
entity model resolution, the existing RecipeMan selection/RNG/model identity,
and required model residency. Force spawn uses MAXPEDESTRIANS; otherwise the
initial or retained current allowance minus the pedestrian reserve applies.
The adapter calls the concrete owner only after these gates pass.

`EvaluativeSpawnPedestrians8271ea90` captures the population before iteration,
requires the debug-manager layout byte 121 and excludes ghost/network modes.
Regular evaluation permits two placement attempts and one successful spawn;
initial evaluation uses 6000 attempts, 600 successes and radii 8..80. Shared
GetRandomPos consumes the original Manager RNG draws; an additional yaw draw
0..2PI occurs before weighted target selection. Rotation uses the existing
native sine/cosine kernel. ABI checked against raw instructions: initial is
r6, allowCull r7=true, force r8=false, worldCollision r9=true,
ignoreNav r10=false, and the stack hand-prop flag is true (r23=1).

`EvaluativeCullPedestrians8271ef70` includes vertical distance, uses inclusive
boundary culling, and signals protected security guards using their original
outside-cull-radius callback rather than deleting them.
`EvaluatePedestrianCensus82723d90` uses the retained range key and existing
census-circle interpolation, velocity-normalized center shift, and km/h
conversion. Culling precedes spawn. Prop-driven spawning precedes ordinary
spawning when bit0x10 is enabled.

The prop-driven path is a required service method, not a successful empty
pass. `EvaluativePropDrivenSpawnPedestrians827215f8` extends through82721dc0;
HexRays misleadingly truncates the body after GetPropsWithinRange. Raw calls
show waypoint-group unlock, prop category membership, PropUseProbabilities,
scaled weighted RNG, look-at transform and spawn callbacks. That owner must
be implemented before claiming prop-driven behavior complete.

## Navigation

Extended the existing PedRainNode/RAIN owner, keeping streamed lookup and
authored links. No inferred road connections, replacement node registry or
invented waypoint placement.

* IsValid82e52db0 resolves Road or Intersection; uninitialized is false.
* GetNext/PrevRoad[12]82e53320..53620 selects outgoing/incoming branch IDs.
* GetNext/PrevCrossWalk[12]82e53720..53ab0 resolves the corresponding authored
  intersection and pin0; absent owners return a default uninitialized node.
* RequestStart/EndNavTarget82e53be0/53c80 uses the existing intersection target
  list. Get82e53d20/53db8 indexes its fixed slots. Free82e53e50/53ef8 clears
  occupancy and entity ID. Calls retain lazy NavMesh target generation.
* GetStart82e52e68/GetEnd82e530b0 uses authored crosswalk corners. Road start
  reads first segment vectors0x70/0x80, end last segment0x50/0x60. Delta is
  second minus first, unsafe-normalized, then offset beyond the second edge
  by 1 plus parking width when SuperRoad flags&1. Missing SuperRoad yields
  zero padding. Missing Road/Intersection returns native (0,1000,0,0).

Raw VX128 checked to avoid IDA register alias errors: instruction17fad87d at
82e52f84 has VA58, VB59. Instruction1674f837 at82e53040 has VA52, VB127.
Thus the final sum is first+delta, followed by the padded direction FMA.

Authored graph located read-only in original miscload.big:
`data/state/livingworldentities/pedestrian/Pedestrian.xml` (offset225408,
compressed size327). It includes InitialState, colliding, patrol boundary,
wander, wander-on-road, run-from-honker/chaser, plugin, patrol, address-wants
and chase-chasee graphs. Wander's authored SuggestVelocity is2.0; its road
branch requires HasRoadWanderTarget, CheckForTargetTimeout, crosswalk states
and Wander. NoRoad uses CheckForRoadTarget and NoRoadWander. These callbacks
and PedNavController belong to concrete entity integration; the topology
adapter alone does not constitute moving pedestrians.

## Navigation base and no-road target iterator

Added `pedestrian_navigation.rs` for Navigation constructor82e6ba80,
Init82e75358, Reset82e6bcd0, Update82e6bb70, HasArrivedXZ82e75468,
SetPathTarget82e7daf0, SetSeekDestination82e7db70 and direction/strafe methods.
The entity navigation RefSpec is0x4a7ac0104ee51208, verified raw register pair.
Existing Euler/angle and numeric kernels are reused. Raw Update instructions
show that the ramp comparison uses max increment before multiplying by dt;
the increment applied afterward is max increment times dt. Arrival uses an
inclusive horizontal circle and strict vertical threshold.

Added `pedestrian_navigation_wander.rs` for NoRoadWanderTargetIterator
IncrementTarget82e90100, FindValidTarget82e8cfd8, IsTargetableCell82e87640 and
OnPathingFailure82e6cdd8. The retained MeshStore now accepts a caller-supplied
Y tolerance and exposes live cell cost/centroid. Target probes test straight
ahead followed by alternating negative/positive angles. The angular step is
searchAngle times radians-per-half-degree divided by (attempts>>1). Quaternion
rotation uses the original half-angle sine/cosine and shared cross/FMA math.
Candidates must have a valid cell, cost not greater than threshold, and no
containing retained world-avoid volume. The avoid-volume callback is mandatory.
Successful targets use the authored cell centroid and emit the original timer
restart argument. Failed far/close searches use the original forward fail
distance target, which still requires the separate path solver to admit it.
Failure cases1/2 set failure;4 sets timeout; others return false.

Added ten existing-style cvars from82fa6af8,82fa6e48..70c8,82fa8cf8: target
timer speed0.5, enable true, far attempts5/distance40/angle90, close attempts9/
distance10/angle270, fail distance3 and Pathogen Y tolerance5. Static registry
audit found1026 definitions and1026 unique registration indices0..1025.

Concrete remaining integration is significant: MeshNavigation.Update82e7dc70
requires NavPath, FindFurthestValidPathTarget, Seek82e75b68 and ResolvePath
82e76120. Existing cell queries are not a substitute for these owners. No
linear interpolation or fake free-space path has been added. Wander Begin
and timer callback source mappings were sent to the concrete entity owner.

## Subsequent Pathogen and MeshNavigation implementation

The preceding missing-path audit prompted implementation of the actual owners,
not an interpolation workaround. `living_world_pathogen.rs` is a child of the
existing MeshStore, with one PathSolver in the existing MeshMan. Adjacency reads
authored cell edge IDs (12..20), neighbor IDs (24..32), edge cell refs (8/12),
edge unit direction (28) and magnitude (40). Packed cell references use high3
bits as the retained neighboring-tile slot, low29 as local index; absent and
unloaded tiles remain invalid. Weak handles cannot keep streamed tiles alive.

Sources: Solver ctor82e5fb88 (256 proxies), SolveAStar82e62ef0,
EvaluateNeighborCell82e617f0, PopulatePath82e58b58, SolvePathBoundary82e60750,
tNavPath.Reset824b3890, cell/edge getters82e4de08/82e52970.

The native solver uses target-centroid distance plus cell score, not accumulated
textbook A* cost. Raw82e63390 passes only-zero-cost=true in r9. A start cell's
higher cost raises zero threshold by0.01. Expanded-cell count controls the
requested depth; the allocator's254-slot guard publishes the native interim
path. Two GetFromClosed calls are present in raw82e619a0/82e61a14; no textbook
reopening algorithm replaces them. The comparison uses weight then underlying
cell address. Host retained arena addresses preserve that mechanism, but
allocation-dependent equal-weight ordering is not claimed Xenon-identical.

Boundary generation retains the three original toggles, one neighbor broadening
pass then two further passes for zero-score cells, unique perimeter collection
and120-edge publication limit. Linked cvars82fa36c8/3718/3768 default true.
FindFurthestValidPathTarget82e5d430 tries destination, then reverse cell
centroids and each authored vertex. LineIntersectsEdgeList82e58f00 uses authored
edge directions and distances, inclusive endpoints, and Line_Line_Test82cf9548
cross/dot/reciprocal math. The source ignores Line_Line_Test's separate status
flag and evaluates its returned distances; this is preserved.

`pedestrian_navigation_mesh.rs` now implements native Seek82e75b68,
ResolvePath82e76120 and Update82e7dc70, including explicit targeting, target
iterator calls, incomplete paths, native failure callbacks and off-mesh anchor
branches. The concrete adapter supplies entity pose/cell/AI depth, navigation
rig membership, state-storage flag changes and retained iterator ownership.
Failure enum was read from IDA type information: invalidStart0, invalidEnd1,
pathLength2, targetOffMeshAndOutOfRig3, timeout4. Small anchor comparison constant
is0x37800000; iterator increment delta is0x3e2aaa7e. Linked allowAStar82fa8df8
defaults true; offMeshAnchorArrivalRadius82fa9238 defaults1.5.

Verified MemStore Init82a6de38 clears the whole entity block and FreeBlock82a6df68
clears reused blocks, supporting zero values for embedded navigation flags not
explicitly overwritten by MeshNavigation constructor. Movement still requires
the concrete entity frame to consume navigation output through original
locomotion/animation/physics phases; these modules do not directly translate
an entity in world space or claim the integration is complete by themselves.
