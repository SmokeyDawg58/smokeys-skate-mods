# Marker return world-origin jump and terrain crash, 2026-10-21

Status: source patch prepared, **not compiled or runtime tested**. The user
requested no rebuild because they intend to mod Wispp's upcoming version.
Frozen Test 6 executable and source archives are preserved. Do not label this
patch runtime verified when carrying it into that update.

## Observed failure

Test 6 source fingerprint
`94848dc9b5166bdfd9c280883cc3f3179d355a70780d62e42a4ede82d199507f`,
run `run-1792629481183057900-12496`. User reported flying far away after returning
to a marker, then the error `Skate 3 player requires actual Skate 2 terrain`.
The retained journal confirms the displacement precedes the terrain error:

| Source tick | State | Root position | Observation |
| --- | --- | --- | --- |
| 7837 | 100 | 954.61273,-59.762543,940.2192 | Before return |
| request | offboard, foot LEFT | 920.86,-59.7,927.2049 | Native Actor generation9 |
| 7838 | 702 | 920.86,-59.600002,927.2049 | Requested position reached |
| 7839 | 200 | 0.022454,-0.89647,-0.009391 | Root jumps about1308 metres; COM near zero |
| 7840 | 200 | 655.3616,-379.70862,-3.18213 | Solver follows the invalid air trajectory |
| after7970 | failure | resident sources0 | Terrain construction rejects empty geometry |

This is a displacement of the solved character and board, not just the camera.
The incoming marker transform is finite and near the previous position. Source
data, extracted samples, request and exact failure are in development workspace
`work/marker109-diagnosis.json`; `diagnose_marker109.py` verifies the sequence
against the retained actual failure and original TU3 image.

## Original ordering and mismatch

Original TU3 physical image SHA-256
`f4aa113eb541bfba03dbc108cf5ab43f58c965b20fa3b82f9c40938a0ad841c4`.
`PhysicalPlayer::Teleport`82DB8998 compares current state with100 at82DB8DD4;
82DB8DDC loads100 and82DB8DE4 calls `SetPhysicsState`82DB8540. Reset always
returns through PhysicsGround before ordinary graph/selector transitions.

The host `player_state::enter_after_teleport` instead forced BipedGround500 for
offboard markers. Native selector82D8ADE8's BipedGround branch can choose air200
when the offboard attributes/contact publications are not ready. The reset has
just cleared PhysOut; the subsequent ProcessInput publication can copy its zero
COM into trajectory592. PhysicsAir Enter82D34388 uses that field when entering
from state500. This explains the observed zero-origin air frame. Confidence:
**high** for the original reset ordering and captured jump; the complete causal
fix still needs the newly added runtime regression.

The stock MotionGraphIncludes/teleport.xml runs ResetSkaterAnimation,
ResetToGivenStance and TELEPORT playback under filtered teleport state. The
host must expose this pending reset for either board target. Authored
ActionGraphIncludes/onboardgeneral.xml then uses PhysicsRequestsDismount and
TeleportOffBoard to acquire normal offboard motion; forcing a physical500
before those outputs are ready is unnecessary.

## Source changes

* Restore unconditional PhysicsGround100 in the reset continuation.
* Both board targets pass through the pending teleport graph observation.
  Preserve an offboard target until an actual category500 output appears;
  preserve an onboard target until a nonteleport onboard output appears.
* A host reset cannot be acknowledged through the automatic-bail fallback
  while its requested board family is still pending.
* If a retained source player encounters zero published collision volumes,
  defer its physical step and retain the queued marker/geometry cache. Resume
  one ordinary tick when real collision returns. Do not replace it with empty
  or fabricated terrain, run another solver, or accumulate catch-up ticks.
* Suppress fresh pose/reset/scoring publication while that source step was
  deferred, preventing the old packet's one-shot events from being replayed.
  Journal `skate3_scene_wait`/`skate3_scene_resume` records the transition.

This changes only the selected skater/reset admission. Skate 2's world streaming
code, assets, renderer, network transport, physics tuning and gesture systems
are not modified.

## Checks and next-version validation

All seven changed Rust files parse through rustfmt's nonwriting stdout mode.
Existing matching line endings are preserved. The actual failure and original
reset opcode checks pass. These are static checks, **not** a successful build
or gameplay regression run.

Added pending checks for the next build:

* `teleport_state::marker_reset_tests`: target retention through702/100 and
  release only on the requested actual board family.
* `skate3_marker_offboard_return_after_bail_keeps_world_position`: real gamepad
  bail, native Actor offboard marker request with the captured LB/Up sample,
  every-frame root/board destination bounds and eventual acknowledged offboard.
* Existing twelve-case spawn corpus now checks every intermediate root and
  board position. Its previous final-frame assertions missed transient jumps.
* Embedded-player corpus tests a retained player against an empty ground
  publication for three frames, no replay/reset completion, then one resumed
  step when actual road geometry returns.

The five-metre regression bound is a diagnostic rejection threshold for the
1308-metre jump; no position clamp or physical tuning value is added at runtime.
After merging the upstream update, compile and run these actual-data tests,
then a bounded rendered marker return and in-game offboard return after a bail.
Do not replay historical one-shot patch scripts during the upstream merge.


## Later headless validation

The subsequent source checkpoint compiles and passes the actual-data bail-to-
offboard marker return, twelve-case banked stance replay, and terrain-admission
regressions. The early duplicate GivenStance consumption was also removed;
the native graph consumes the retained saved stance once. See
`skate3-gesture-actor-and-marker-regressions-2026-10-21.md` for run IDs, moving
marker coverage and remaining intermittent wheel-sticking report. This later
record does not modify the historical source-patch ZIP or frozen Test 6 build.
No rendered/playable build is implied by the headless validation.
