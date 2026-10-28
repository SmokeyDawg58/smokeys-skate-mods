# Waypoint failures during replay scrubbing and game changes

PC source changes, October 14, 2026. Solo investigation. No build or runtime
test performed. Android and saved profiles untouched.

## Reports and limits

- Career traversal -> main menu -> Big Black Freeplay: TerrainResident,
  normal registration operation 3, Registry(Waypoint), WaypointRefusal.
- Rapid back-and-forth Replay Editor scrubbing: Visual, normal registration
  operation 2, Registry(Waypoint), WaypointRefusal.

Both reach the same scoped waypoint callback. The previous adapter retained
the callback's String internally but returned only WaypointRefusal, so these
reports do **not** identify which callback operation failed. The source audit
below establishes two real mismatches that can cause this class of failure;
it does not establish which one fired in either reported run.

## Native evidence

Live IDA was unavailable. Used IDA 9.3 in batch mode against an
independent copy of the existing symbolized reference database instead:

`world_probe_temp\waypoint_replay_20261014.i64`

The source copy was `world_probe_temp\sim_adapter.i64`. IDA's stored input
SHA256 is `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.
The original working database was not edited. Decompilation, raw PPC and
reference lists are saved under `.scratch/waypoint-streaming-20261014/`:
`native.json`, `spatial-native.json`, `final-native.json`, with extraction
scripts and IDA logs. Batch processes exited normally.

### 1. Entity registration tolerates a full temporary KD index

- Prop EntityMan Init `82e67788` requests 256 blocks of 80 bytes from
  MemStore.Init `82a6dd98`.
- MemStore.RequestBlock `82a6e450` returns null when the free-list index is
  negative, without an assertion or allocation. KDTree<uint>::Insert
  `82710e28` returns false on that null branch, leaving the tree unchanged.
- Prop AddEntity `82e67910` first inserts the entity in its map, then calls
  KDTree.Insert at `82e6797c`. The next instructions restore the stack and
  return; there is no check of the boolean in r3.
- Vehicle `82d26dc0` / call `82d26e2c` and pedestrian `82e92ee8` / call
  `82e92f54` have the same void AddEntity contract.
- UpdateSpatialLookups also ignores Insert's boolean and continues its map
  traversal: prop `82e677f0` / call `82e678d8`, vehicle `82d26c28` / call
  `82d26d10`, pedestrian `82e92dc8` / call `82e92eb0`.
- RemoveEntity retires map membership only. Manager.Update1 `82727ee0`
  subsequently clears and rebuilds all four spatial managers. SimUpdate1
  `8289f410` gates LivingWorld updates on simulation flags and its enable bit.

**Confirmed mismatch:** the shared host EntityMan converted the native
nonfatal false into `EntityMutationError::TreeCapacity`. Waypoint registration
then wrapped `Prop AddEntity: TreeCapacity` as WaypointRefusal. Repeated paused
streaming can consume retired KD nodes even though the live entity map has
room; the same risk exists while the main-menu world disables LivingWorld.

Changed `living_world_grind_entities.rs` to preserve the original ignored
Insert result in AddEntity and UpdateSpatialLookups. The live map retains the
entity and the ordinary later spatial rebuild publishes its coordinate.
Removed the now-unused TreeCapacity error variant. DmoObjectMan already
followed this native contract and required no change. Capacities, map order,
duplicate handling, node retirement and scheduled rebuild timing are intact.
Missing initialization, map capacity and invalid core errors still propagate.

This refines the October 1 prop spatial lifetime evidence: the scheduled
four-manager rebuild added then remains necessary and stays in place, but
native AddEntity does not make transient KD exhaustion fatal. No eager reset
or extra LivingWorld/physics tick is introduced here.

### 2. GameSim dispatch continues while paused

- SimController.Update1 `828a72c8` calls its Simulation broadcaster's Dispatch
  at `828a739c`, before the `mCachedIsActive` branch that gates normal updates.
- ThreadSafeSinglePortBroadcaster.Dispatch `824eccf8` locks, calls
  EventSys.Flush on its existing queue, increments its dispatch tick, unlocks.
- RegisterWaypointData `82719e60` and Prop.OnCullRequested `82e65bf0` publish
  waypoint/cull events into that same GameSim route (queue 4, 24576 bytes).

**Confirmed mismatch:** the host Replay Editor returns before SelectedTick's
normal dispatch. It still streams terrain and emits events, but did not flush
them while scrubbing. They could exhaust the queue independently of KD nodes.

Changed `selected_session_replay_editor.rs` to dispatch the existing
SelectedMoodFrame/GameSim subscribers at the beginning of every editor update,
before terrain streaming. The call also occurs on the export early-return
path. The PC render loop continues frontend ticks during video export, so no
second flush was added to the per-output-frame preparation method. Uses the
same recipe, trigger, painter, ground, Vault and physics-world owners as the
existing paused StartHub dispatch. Does not step physics, run AI, discard
messages, create another queue or enlarge its capacity.

### 3. Preserve the actual callback error

`simulation_normal_arena_adapter.rs` now returns the failed waypoint asset ID
and callback detail through both registration and deregistration errors.
Display (Replay Editor) and Debug (live terrain) both retain that information.
The existing latched fault and retained partial operation prefix are unchanged.
The three exhaustive collection-adapter matches classify the new variant as
ReachedFault, preserving consumed registration authority after effects.

## Validation and remaining work

- All four changed Rust files parse with rustfmt (stdout only, no formatting).
- Scoped diffs against pre-edit backups reviewed. Native return handling
  checked in raw PPC as well as pseudocode for all three shared entity types.
- No compilation, new tests, test execution or game launch. Type checking and
  runtime verification remain pending.
- Revisit repeated long-distance scrubbing, saved-replay export and return to
  skating after rebuilding. Also retry Career -> menu -> Freeplay. If a
  waypoint callback still fails, the new error includes its actual cause and
  asset instead of only the generic WaypointRefusal label.
- Other unrelated waypoint callback errors are not suppressed. The original
  two crash logs alone cannot prove that both crashes are resolved.

Backups of changed source and status documents:
`.scratch/waypoint-streaming-20261014/before/`.
