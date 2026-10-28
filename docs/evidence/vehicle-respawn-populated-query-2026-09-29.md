# Vehicle respawn query: populated tree

Reported: `Before(TeleportingActor(Select(LivingWorld(Vehicle(Entity(PopulatedTreeUnrecovered))))))` after 4390 frames.

## Cause and correction

The breadcrumb bridge still called an empty-only vehicle range wrapper. Live cars
were already in the SAME EntityMan/KD tree, so that guard rejected a normal scene.
There was also a second gap: any returned vehicle candidate would have produced
`UnexpectedRangeCount`, without the native per-car overlap test.

`living_world/interaction/living_world_grind_services.rs` now routes its vehicle
range wrapper through existing `EntityMan::find_near` and shared locator KD
traversal. It retains the source 24-item clamp, sorted result order, zero result
count on no hits and untouched unused output entries. Host slice capacity failures
remain explicit errors. No second tree, entity list or physics world was created.

Extracted the already implemented vehicle sphere and local-coordinate math from
`physics/player/lifecycle/physical_player_update0_collision_status.rs` into
`living_world/vehicles/living_world_vehicle_queries.rs`. Both collision status and
`physics/player/runtime/physical_player_living_world_respawn.rs` now call that ONE
sphere implementation. The existing inside-vehicle query reuses its local-point
helper and the corrected range wrapper as well.

Breadcrumb uses candidate translation unchanged; collision status retains its
existing upward radius offset. Vehicle hit returns false to breadcrumb selection
before pedestrian queries or lazy capsule attribute reads. Vehicle miss continues
to the existing pedestrian and DMO checks. Missing/stale KD IDs are skipped after
lookup, as in native GetVehicle; no additional visibility/active filtering.

## IDA verification this session

- `LWVehicleMan::GetVehiclesWithinRange` **82d29030**, raw **82d29034..40**:
  unsigned clamp of argument r6 to 24, then same KD FindNear **82d28c00**.
- `KDTree<uint,Vector2,2>::FindNear` **82d28c00**: count reset, 256 limit,
  existing shared node traversal and distance-ordered output. Reused existing
  DMO/RAIN KD implementation rather than creating another traversal.
- `Manager::DoesAnyVehicleIntersectSphere` **82710760**, decompile + raw PPC:
  XZ query, radius+8, max6 (**827107fc**); resolve each ID and skip nulls
  (**82710860..74**); inverse-orientation/translation; strict Z then X then Y
  expanded local-box tests (**82710990..10a2c**).
- Raw **827107a8..b8** proves extra Y clearance is **1.0** (0x3f800000);
  decompiler alone hides the initializing floating load. Half dimensions are
  fused with radius, then Y adds 1. No Euclidean sphere/OBB replacement.
- `BreadCrumbManager::TestForLWRespawnContraints` **82ce1320**: vehicle at
  **82ce139c**, pedestrian **82ce13d8**, DMO capsule **82ce149c**; reject immediately
  on hit. Candidate position is passed directly; capsule attributes are lazy.

## Validation and remaining limits

Scoped rustfmt and `python tools/check_core_layout.py` pass. No Cargo command,
compilation, unit-test execution or game run this turn.
Three regressions added (UNRUN): actual vehicle fixture in populated EntityMan
with output ordering/tail preservation/stale entries/no-hit; rotated local box
versus broadphase-only candidate and strict boundary; vertical padding/NaN.

This fixes the populated vehicle query and its missing respawn narrowphase, not
every possible recovery failure. Existing host numeric policy remains an
approximation of Xenon estimates; exact runtime parity is not claimed. Populated
pedestrian narrowphase, network predicate and breadcrumb last-resort limitations
were not expanded or suppressed. No renderer/driving/handling changes.

Next playtest: compile normally, skate near traffic, bail/respawn and allow any
vehicle penetration recovery to run. Confirm this pending error is gone and
respawn spots occupied by cars are avoided.
