# Specific-body contacts required by Skeleton Audio.Fill

Read-only source/caller audit, 2026-09-12. Confidence: exact caller addresses,
stores and existing ownership; high integration scope. No runtime changes or
Cargo validation performed. This is a scheduling gap, not a missing contact
initialization leaf.

## Existing producer and native ordering

`SkeletonCollision::UpdateSpecificBodyParts` at `82b3eeb0` has one native code
caller: `SkeletonCollision::Update` at `82b4bbb8`. Collision.Update itself is
called by `Skeleton::UpdatePostPhysics` at `82b57888`, after
UpdatePhysicsRecords `82b576d8` and actual Island.GetCollisions `82b57714`.
UpdateVolumesStates `82b57890` and UpdateRecords `82b5789c` follow.

The registered `skeleton_collision_update.rs::update_specific_regions` already
implements the selected normal-mode producer on the SAME `SkeletonCollisionData`:

* Caller first sets `is_wiping_out=0`. Each region's native recent-contact
  expression is `oldRecent && isWipingOut`, hence false even when the native
  constructor did not initialize oldRecent. Both current contacts become zero.
  This is an evaluated normal-update result, not constructor initialization.
* Raw `82b3ef28/ef2c` store recent/current bytes before position calculation.
* `ef50` addresses SAME State.PhysRecord transforms at State+610, using the
  actual configured part. Full-vector position is I*offsetX+P, then
  J*offsetY+previous, then K*offsetZ+previous. Standard VMADD words
  `10e06aae`, `10cc3a6e`, `10ab322e` confirm those fused stages; `ef70`
  publishes all four lanes.
* Live draw cvar is read at entry and per region. Enabled entry reloads actual
  InitSpecificBodyParts; the existing explicit unsupported debug-reload path
  must not become a no-op. Per-region enabled branch only requires DebugOverlay.
* Subsequent real contacts can set current/recent to one: live allow-any-contact
  or matching configured part, then ordered distance-squared < radius-squared.
  NaN fails this comparison. Stopping after the zero writes omits real contact
  processing and is not an adequate producer integration.

Audio.Fill reads current face byte contact[1] at `82b3f5ac`, before clamping its
accumulated totals. Its checked unknown-byte failure correctly exposes that this
normal update never ran on the selected startup owner.

## Actual caller gap

HiLOD `Update4_PostEngineAdjust` begins `82e01c78`: EndSkeletonLineTests,
board.UpdatePostPhysics `82e01cac`, bad-acceleration request handling, and
deck-in-hand/body-byte360 logic precede Skeleton.UpdatePostPhysics `82e01d4c`.
Raw `1d34` reads actual Processed+93c dt; `1d44` passes retained r30 as the
partial-ragdoll request. State.UpdatePostPhysics, wipeout decision and Skeleton
PostWipeoutCheck follow before Update5.

`physical_player_update1_corpus.rs` currently explicitly skips those intervening
phases: it collects line tests, selects a board surface, then invokes Update5.
The existing world `try_update_skeleton_normal_post_physics` is invoked only by
`physical_player_collision_corpus.rs`, using controlled collision inputs.
Its production ownership chain is already useful: current solved-frame checks,
same paired history, actual retained contact list, normal collision, volume
states and records. Preserve those checks; do not invent an empty contact frame
or call the specific-region leaf on constructor history.

## Smallest next implementation boundary

Reuse that chain with actual UpdatePostPhysics preparation, rather than adding
another specific-parts module. The missing preparation comprises:

1. Actual Body.IsBadData gate and existing reset/IK-reset/respawn effects on the
   taken branch; fresh board Part6 position minus retained Skeleton+3ce0 into
   existing constructor outer skateboard_error, plus live draw cvar.
2. Existing UpdatePhysicsRecords prefix, then its currently omitted `cc70`
   suffix. The suffix transforms OLD retained COM+3d00 and the captured board
   Part6 position through SAME world-to-animation, storing +3d40 and +3d60.
   It uses full-vector fused affine stages; no fresh COM or board requery.
3. Actual collision input assembly: state KnownAir selects PhysRecord COM
   velocity+2840; other states select Skeleton system velocity+3cf0. Actual
   Processed ground position is +1e0 for Boneless/Footplant, otherwise +1f0;
   Reckoning ground normal is +1f0. COM velocity is OLD Skeleton+3d20, already
   corrected in the existing normal collision adapter. Flags+8c8 supply danger
   zone and AI; state/category determine offboard/offboard-air/ground-trick.
   Grab rigid-body exclusion and offboard ground override must retain source
   conditions and actual owners, not default None/zero substitutions.
4. Live services for existing history COM cvar, collision cvars and post-record
   proxy-offset/scale/debug reads. Current implementations of these interfaces
   are predominantly controlled corpus fixtures; merely exposing another
   callback facade would not close the actual first-frame path.

The solved-frame prerequisite is independently necessary: current Update5
fixture explicitly omits solver phases. First extend the actual completed-frame
collision corpus through Audio.Fill for same-owner validation, then mount the
real Update4 caller when its preceding board/solver stages are admitted. This
tests the existing producer without falsely claiming the startup frame complete.
