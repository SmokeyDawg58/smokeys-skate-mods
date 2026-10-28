# Active Ground jump and retained trajectory launch

Status: source audit and NEW unregistered drafts only. No Cargo or original
corpus validation in this batch. This is not a complete launched jump path.
Reference build IDA/PDB was read directly on 2026-09-12.

## Ground caller and failure prefixes

Ground.UpdateSkateboard82d9a018 captures Ground+0xd0 at a170. This is mJumpInfo,
whose first full vector is the actual jump direction/velocity. At a430..a43c
the actual doJump byte selects the branch; no constructor-idle assumption.

True order: Board.SetVelocity82b29ee0 (a444), Processed190 full-vector store
(a45c), Board.SetLinearDrag82b23a98 with +0 (a460), local
TrajectoryLaunchInfo constructor82d83cf8 (a468), Skeleton.GetLaunchInfo82b46098
(a474), fresh Ground jump vector, playerJumped byte10c=true (a490), startVel80
(a494), velBeforeJump90 (a498), SAME TrajectorySelector.Launch82dc4098 (a49c),
SAME selector.Update82dc6528 (a4a4), actual DebugOverlay singleton require
(a4ac, unconditional, no draw call), Ground.exiting_wallride+f0=true (a4d0),
exit a998. A late failure does not roll back velocity, drag or earlier stores.

## GetLaunchInfo producer

Full source copy order:

| Read | Destination | Source store |
|---|---|---|
| Processed.deck.translation70 | launch.c0 board position | 460c8 |
| Processed.COM250 | launch.d0 COM position | 460e0 |
| Skeleton.mCentreOfMassLocal3d40 | launch.a0 animated COM | 460f0 |
| Skeleton.mDeckLocal3d60 | launch.b0 animated board | 46100 |
| SAME Reckoning.transform60 | launch.0, four vectors | 46134..46140 |
| SAME Reckoning.inverse e0 | launch.40, four vectors | 4615c..46168 |
| Processed190 | launch.80 start velocity | 46174 |
| Processed2c0 | launch.90 previous velocity | 46180 |
| Processed93c | launch.108 delta | 4618c |
| Processed8c4 mask2000 | launch.10e unsigned halfword: set7, clear1 | 46194..461ac |
| actual GlobalAttributeCollection+e8 layout248 | cone X100 | 461e0 |
| fresh same global+e8 capture layout240 | cone Z104 | 46214 |

The global is the already retained `WallRideTrajectoryAttributes` capability:
generated constructor828b1068 called828b291c, class429a8f4385dd539d,
defaultd7edbd362d7d2152, layout24c. Preserve two independent late layout captures
and generated absent-layout zero defaults. No new Vault binding per invocation.

`physical_player_skeleton_launch_info.rs` reads the actual constructor binding
and actual PhysicalPlayer/Reckoning Processed Arc, preserving prefix writes.
It needs the existing RetainedTrajectoryLaunchInfo vector and matrix fields
migrated to SkeletonRootVector/Matrix, including known-zero constructor lanes.
No alternative launch struct or unknown-W zero promotion is permitted.
`reckoning_launch_transform_read.rs` exposes only the SAME retained inverse.

Skeleton local COM and deck fields are the existing outer fields, not animation
record COM or a second cache. The Skeleton agent independently identified their
real UpdatePhysicsRecords producer: old COM3d00 and captured Part6 position are
transformed by SAME world-to-animation I*x+P, then J*y and K*z fused additions,
publishing3d40 before3d60. Its coordinated draft migrates those SAME fields to
typed vectors. Source input W is unused by this affine calculation; output W
knowledge depends on the actual four matrix W lanes.

## Selector Launch prefix and mandatory continuation

Raw82dc4098: justStartedQuery2392 nonzero returns without any mutation.
Otherwise stores mostVerticalValid2391=false at40e4, pass2380=0 at40ec,
best2384=-1 at40f8, mostVerticalNormal950=gJ at4104, native full launch
assignment82db6010 at4108, ready2393=false at410c, bonus238e=false at4110.
Then AdjustStartingVelOnVerts82dbfbc0 (4118), private Launch82dc2800 (4120).
`physical_player_onboard_launch_prefix.rs` retains precisely these stores and
returns an explicit pre-adjust continuation, never a completed Launch.

AdjustStartingVelOnVerts clears actual on_vert first. It performs unconditional
Reckoning tuning reads, normalizations, ground-normal projections, transition
input and blend calculations even for flat ground. A user-jump flag only skips
the later !playerJumped non-Air branch; it cannot skip the unconditional work.
The private Launch additionally needs InitTrajectoryInfo82db9018,
CalcMaxTimeForTrajectory82dc0118, the actual AI path branch and actual per-query
prediction submission. Existing generic trajectory helpers do not establish
this owner/queue protocol.

## Actual asynchronous prediction boundary

Predictor.SetupFakeBodyPrediction82e4bb88 forwards p0/v0 and SAME Predictor
gravity to AddQuery82e4b868. Raw review is still required for the decompiler's
spurious stack arguments and exact p0/v0 validity assertions before coding.
The recovered AddQuery body resets SAME future readiness=false and its result
before queue capacity admission: position0, normal/groundNormal gJ, time splat-1,
frame-1, surface0, trajectory p0/v0/gravity/time, radius/errors/draw payload.
It enters the actual predictor critical section, checks existing capacity,
appends the SAME future identity with initially null batch, releases the lock,
then calls SAME World.AcquireTrajectoryBatch, Begin, AddTest, End. Capacity
failure occurs after the future reset. Selector justStarted is published only
after all its submissions, not by the constructor or the initial Launch stores.

Current `PhysicalPredictionOwner` retains a 90-slot queue whose element enum
`UnlaunchedPredictionQuery` is uninhabited. Therefore actual submission/result
harvesting remains missing; real existing trajectory-world queries are reusable
but a synchronous helper cannot pretend this asynchronous future completed.
Existing selector polling explicitly stops on completed-query scoring rather
than inventing a winner. Full jump closure must implement that protocol before
the Ground branch can reach its final overlay/exit publication.

## Registration and validation plan

All three draft Rust files remain unregistered. Integrator must coordinate the
typed existing launch constructor and outer-field migration, register two
children under trajectory owners and one under Reckoning. Tests should cover
source partial writes on missing late layout/field, mixed known/unknown W,
2000->7 versus clear->1, preservation of unused override fields, and a live
justStarted owner returning untouched. Queue tests must additionally prove
reset-before-capacity-failure, same future identity, Begin/Add/End ordering and
completion only through the actual backend harvest. No test has yet run here.
