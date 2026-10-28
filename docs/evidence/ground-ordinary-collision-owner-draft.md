# Ordinary Ground collision owner continuation

2026-09-12. NEW/unregistered draft, no Cargo or original corpus execution.
Source ordering confidence high; existing collision numerical-policy boundaries
and the pending normalization callback split remain explicit.

`physical_player_ground_collision.rs` consumes the actual `GroundBeforeJump`
returned by the fake-torque stage. It checks the same pair/Arc/selected Ground
bindings and reads Ground.doJump at82d9a430. True is an explicit
`JumpLaunchPending` boundary before the jump branch, never a fabricated false.
False follows the ordinary source block82d9a4d8..a658.

The body uses existing `SkateboardSlideCollisionServices::try_collision` and
the existing unregistered `GroundForceInput` reader against the actual
Processed Arc and retained Reckoning. The latter now carries an optional
borrowed Reckoning and reports absence only when the source first requests
its ground normal. Source early-false collision does not require Reckoning,
surface data or attributes. This is a late-read correction to that unregistered
reader, not a new owner or a default normal.

Both native force output vectors are explicitly zeroed beforea52c; optional
newVelocity is null. Native torque scratch is not shown initialized. The Rust
scratch uses non-admitted NaN XYZ/unknown W backing solely to call the existing
write-only API: every true leaf return has replaced torque; both false routes
discard it. No scratch is published on error. Existing collision leaf tests
already discriminate early false from late false and their output writes.

For a true response, actual stores and reads are:

1. a560 Ground.collision_force[0], a568 [1], a56c collision_torque, with typed
   W knowledge preserved from the produced vectors.
2. Actual SAME runtime.slide_collision_attributes captures global Collision
   (+10c), looks up `d613692c606fe326` at a5b4, loads floata5c8, stores
   Ground.collision_timer+aca5d0. This reuses class7379c4467aa75df9/default
   d7edbd362d7d2152 already retained by the runtime; no fresh binding is needed.
3. a5d8 invokes the new actual Part6 TargetAngularDisplacement producer. Its
   late failure leaves the preceding Ground copies and timer store intact.
4. a5ec SAME Entry native530/80000000 flag is set; a5f4 queues kind15 with
   the produced force. Queue uses existing capacity21/overflow behavior;
   it is neither cleared nor immediately applied here.
5. a60c NormalizeSafe(world force,+0), a62c fresh Processed190 velocity,
   dot3a630, then current cvar target/valuea634/a638.
6. a650 always writes request_push_stop+f2. Raw NOTLT at a648 skips only
   `li r29,1`; equal, greater and unordered therefore CLEAR any old true.
   Then this UpdateSkateboard branch exitsa998.

A false collision returns a `BeforeBraking` token at a658, retaining the same
WallRide force and owner. It does not manufacture a collision-exit token or
consume the later threshold/foot/brake/force stages. The first next call remains
the actual feet threshold lookup followed by Brakinga6bc, Pusha6c8, absorption,
front/rear, SpeedModel and further documented force calculations.

## Exact shared scope, not applied

- Register this constraint-session child as `ground_collision`.
- Register existing `skeleton_processed_ground_force_input.rs` under Contact
  as `ground_force_input`.
- Migrate only SAME PhysicsGroundFields.collision_force to
  `[SkeletonRootVector;2]`, collision_torque to `SkeletonRootVector`.
  Constructor and Enter known-zero stores remain known zero; amend their
  existing raw-bit oracles accordingly. No parallel shadow fields.
- Add previously proposed SAME Entry.reset_target_after_collision native530
  flag with its actual constructor clear82b38b58. Register new
  `skateboard_entry_collision_response.rs` Entry child; its method sets that field
  before queue append. It is the identical field consumed by the earlier
  unregistered SpeedModel prefix, not a new per-stage flag.
- Register `input_cvar_collision_response.rs` and actual slot128 definition:
  ref830d7258, init82f8ef78, table82fd5710, Physics_Skateboard,
  labelkDotToStopPush, title `Stop push DP`, defaultbf400000, range-1..0,
  nullcallback. Registration precedes existing DrawSBJoints5718.
- TargetAngular child/owner bridges and shared normalization callback proposal
  are documented in `skateboard-target-angular-displacement.md`; they remain
  prerequisites, not asserted already registered services.

No new runtime attributes or broad field visibility are required. Root owns
all shared edits, registration and actual corpus mounting. Required integration
oracles: false response leaves collision history/queue untouched; positive
response retains the three vector copies before duration failure; angular
failure preserves those copies/timer but adds no queue entry; a late missing
cvar retains angular mutation and queue append; equal/NaN dot clears previous
push-stop true. The actual same-Actor corpus must establish which branch ran;
success through an empty/false branch does not validate the positive route.

Raw source evidence and checksum for82d9a4d8..a658 are in
`ground-fake-torque-collision-continuation.md`; the new caller preserves that
sequence and does not reuse the differently ordered GroundAnimation tail.

Recovery note, 2026-09-12: the draft originally collided with the existing registered untracked `skateboard_collision_publication.rs`. Root restored that module by replaying its recorded Add/Update patches and two historical re-export amendments; independent snapshots matched. The new Entry child is now `skateboard_entry_collision_response.rs`. Workspace43 failed during the overwrite; workspace44 is the recovery validation run.
