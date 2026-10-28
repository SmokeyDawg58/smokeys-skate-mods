# Pedestrian wall twitch: contact resolve order (October 8)

Pedestrians walking into a wall or rail flipped between touching the wall,
about 2 meters away, and back again.

## Native frame order

`Simulation::SimUpdate2` 0x8289f980:

1. `World::Update3` 0x827af8f0: physics output, `ResetCollisionFlags`, then
   `Island::ResolveContactSpyList` 0x827b3a90 (simulation Island call at
   0x827afa58).
   - A pedestrian World-body contact reaches `OnEntityCollision` 0x82e95158.
   - `AdjustWorld` moves `mTargetTransform` (core+0x50) by
     part − `SimLODBoneGlobal[8].w`.
   - `SetTargetTransform` 0x82e4cbf8 writes at once, so two contacts compound.
2. `Manager::UpdateSyncAnim` 0x82722588 runs `Pedestrian::UpdateTargetTransform`
   0x82e7f660.
   - That applies `mTargetTransform` × the root delta, then `SetTargetTransform`.
   - It then calls `tAnimData::UpdateSimLODGlobal` 0x82e6ab08, the only writer of
     `SimLODBoneGlobal`.
3. `Manager::UpdatePostTransforms` 0x827031e8 runs `LWPedestrianMan::UpdateCollisions`.
   - `FillWorldCollisionPartTargetTransforms` 0x82e90e30 targets
     `SimLODBoneGlobal[8].w`.
   - `UpdateCollisionBodyPartsFromTransform` 0x82e506d8 then sets the body velocity
     to (target − part)/dt, clamped to `kLWClampMaxVelocity` (60).

## Cause

- The port ran the three LivingWorld resolves (vehicles, pedestrians, DMO
  scoring) inside `try_update_post_transforms`, after the sync.
- `AdjustWorld` therefore used this frame's bones, while the fill still aimed the
  body at the pose from before the push-out.
- The body was driven back into the wall on the next step. The push-outs
  alternated sides in a contact, contact, none cycle and grew by about one
  walking step per cycle.
- Probe: 224 single-tick moves over 0.3 m for one pedestrian in 2400 ticks,
  with the target swinging +0.059, +0.084, +0.097 m past the blocked body on
  successive cycles.

## Fix

- `SelectedLivingWorldDmo::try_resolve_contact_spy_lists` runs the three
  resolves. `selected_player_tick` calls it after `reset_scoring_collision_flags`
  and before `try_sync_animation`. `try_update_post_transforms` no longer
  resolves.
- A pedestrian spawned this frame can now meet `AdjustWorld` before its first
  sync. `Spawn` 0x82e9a510 admits the bodies but never runs
  `UpdateTargetTransform`.
- Native then reads a zeroed block:
  - `MemStore::Init` 0x82a6dd98 zero-fills the pool.
  - `FreeBlock` 0x82a6df20 zeroes each freed block.
  - `RequestBlock` 0x82a6e450 does not clear.
- `AdjustWorld` now reads zero bones when none were built, instead of failing.

## Verification

- `pedestrian-approach`, 2400 ticks, single-tick moves over 0.3 m:
  - Before, by run: 224 (one pedestrian), 73 + 16, 0.
  - After, 7 runs: 1, 3, 1, 1, 2, 0, 2, all isolated, with no repeats.
- `offboard-city-walk`: 1 isolated move in each of 2 runs.
- `parity.sh`: IDENTICAL (6000 lines).
- 130 app tests pass.
- Both demos complete 2400 ticks without probes.

## Remaining single moves (native)

**Plugin ledge.** A pedestrian climbs over a 6.02 × 0.866 × 1.489 ledge DMO.
1. During the plugin, the fill locks the World body at the plugin point
   (FillWorld flag 0x80 → 0x40, `kPluginCollisionBodyShiftAmount`).
2. `DisableCollisionsWithBehaviourSource::End` 0x826ee490 clears the interacting
   object and arms timer 42 with `kPluginUnlockTimer` (2.0).
3. When the timer releases the lock, the body is driven back at up to 60 m/s
   and enters the ledge.
4. `AdjustWorld` moves the pedestrian 0.7–0.8 m back onto the ledge.

Checked in IDA:
- FillWorld
- that End
- `DetermineOverallCollisionType` 0x82e808d8
- `UpdateCollisionBodyPartsFromTransform`
- the two cvar defaults

**Wall edges.** The first contact with a vertical edge can push 0.1–0.2 m, and
its two capsule-end contacts compound as in native.
