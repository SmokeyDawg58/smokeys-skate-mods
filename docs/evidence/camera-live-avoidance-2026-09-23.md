# Live selected camera avoidance integration (2026-09-23)

The selected camera now calls its three retained `Iris::PathEvaluator` owners
from `Rig::UpdateCollisionAvoidance` (`0x82e46490`), in native order: desired
camera path, tuck-in heading, tuck-in elevation. The first runs on grounded
counter multiples of ten or when `kbInterleaveAvoidance` is true; the latter
two run on grounded counter remainder five or when interleaving is true. The
default linked target is true. Each evaluator first advances its retained
World predictor future, then asks the *same* embedded obstacle server for a
stack-local 50-record list and merges the dynamic time. The World predictor
queue and evaluator code were already present; this integration does not
create another predictor or obstacle owner.

`PathEvaluatorObstacleServer::FindObstacles` (`0x82e3a7a0`) first stores the
input camera position, velocity, and radius, then samples the live
`kbPedestrianAvoidance` target and calls `AddPedestrianObstacles`
(`0x82e39eb0`) if enabled. That calls the already retained LivingWorld
pedestrian manager's checked radius query with `kActorSearchRadius`, maximum
20. The selected manager has a constructed and initialized 40-entry entity
store with a null KD root and no pedestrians, so that actual query publishes
zero results. An unpublished manager, missing store, or later populated tree
returns an explicit error; it does not become an invented empty list.
`FindObstacles` then samples `kbSkaterAvoidance`. `AddSkaterObstacles`
(`0x82e3a198`) reads the same live radius target, local and local-sim indices,
then scans the actual `SkaterManagerLocalIdentity` roster in order. It skips
the selected local Actor's player ID. A later nonlocal Actor remains explicit
pending until its native position/velocity getters and sphere append path are
connected. The first-player runtime has no other skater.

The grounded tuck-in heading query composes anchor X/Z with current logical
positioner camera Y (`0x82e4691c..38`), retaining the native X lane in W.
Airborne uses the desired camera position. The elevation query takes the
Subject's live skeleton hips point. Both use the source radius `0.02f`,
reference point velocity, lookahead and debug target. The strategy uses the
larger of the two retained last-valid tuck times (`fsel` at `0x82e46a70`),
sets flag 1 for ordered elevation >= heading *or unordered* (`fcmpu`/`blt`),
and follows request avoidance, on-board collision gating, smoothing and
ease-out in source order. The selected elevation self-compare at
`0x82e46780` skips all path queries only for NaN; positive and negative
infinity continue. `ble` at `0x82e46b2c` and `0x82e46bc0` takes unordered
comparisons, so ordinary strict `>` predicates preserve those branches.
The initialization-time `ble` at `0x82e46bd0` also takes unordered;
`!(initialization_time > 0.0)` preserves that edge in the ease-out gate.

Source confidence is **high** for object ownership, query and CVar order,
empty selected pedestrian/roster outcome, evaluator order and conditional
branches. Host vector math and `fsel` rounding remain **medium** pending a
reference runtime trace. This does not implement populated pedestrian or
nonlocal skater avoidance. It also does not claim the whole native
LivingWorld initialization, which is outside the selected query.

Validation: low-memory `cargo check -p skate2-core` passed. Focused
`camera_prediction::dynamic_obstacle_tests` passed **4/4**, including a live
server test that rejects unpublished pedestrian manager access and then
accepts the initialized empty manager and empty retained skater roster.
The new elevation NaN guard has a focused unit test; it awaits the next
post-edit Cargo gate because the concurrent grounded-Y test exhausted host
memory.
