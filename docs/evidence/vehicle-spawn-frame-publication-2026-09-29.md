# Vehicle spawn after a body velocity write

Reported failure:
`Vehicles(VehicleFrameError("Spawn(Collision(Insert(Publication(FrameUnavailable))))"))`.

Root cause: `CharacterCollisionPublication::try_create` required a valid host
body-pool frame identity during AssemblyPool insertion. The linear and angular
velocity setters intentionally clear that identity, because previous collision
certificates must not describe mutated bodies. Census can then insert a car
before the next actual physics BeginFrame. The error was an incorrect host
publication prerequisite, not missing vehicle geometry or a native spawn veto.
Without a runtime trace we cannot name which velocity setter fired on the reported
particular frame; both can produce this exact failing state.

Live IDA checks:

- `AssemblyPool::Insert 82e0dbb0`: assign world index, AddToSimulation, retain
  Assembly, call SerialisedAssembly.Create, copy initial bounds, increment count.
- `SerialisedAssembly::Create 82e0c850`: consume current Parts/bodies/transforms,
  volumes and CalculatePartBBox. No completed simulation-frame prerequisite.
- `AssemblyPool::BeginUpdate 82e0dc98`: update serialized records and schedule
  bounds work (or the parallel combined update path).
- `AssemblyPool::EndUpdate 82e0bb10`: wait/copy bounds and rebuild active list
  from current Part states.

The existing `serialised_assembly_create.rs` already preserves an optional frame
stamp at Create. The character publication now follows that same rule: create
real collision records and initial bounds from the attached Parts; retain None
if the current host frame is unavailable. No artificial BeginFrame, velocity
change, fresh certificate, skipped spawn or suppressed failure was added.

Safety of the subsequent collision flow remains unchanged:

- `try_update_volumes` still requires a valid frame.
- `try_stage_bounds` still rejects missing/mismatched volume frames.
- `has_bounds_for_frame` still requires both volume and bounds stamps to match.
- `skateboard_ground_frame.rs` starts the actual body frame, then refreshes DMO
  and vehicle publications before the collision/solver phase. No scheduling or
  timestep change was needed for this correction.
- Admission still places actual Parts, allocates native IDs and registers Island
  membership/spies using the same candidate. Remove/re-admission keeps identity.

Added regression in `living_world_vehicle_island_tests.rs`: linear and angular
velocity invalidation on an existing body, spawn another car, reject stale bounds,
refresh at a real frame, remove/re-admit after another velocity write, and refresh
again. Checks actual transform, Island membership, Assembly identity and freshness.

SOLO. Scoped rustfmt only; regression NOT RUN,
no Cargo/build/check/game execution. Source alignment of this lifecycle correction
does not establish full vehicle gameplay parity or prove the reported run now passes.
