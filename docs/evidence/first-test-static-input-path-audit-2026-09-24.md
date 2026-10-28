# Static first-test input-path audit (2026-09-24)

The human XInput runs are verification, not a discovery mechanism. The generated
Action and Motion descriptors already identify reachable instances, conditions,
transitions, and hook index sections. For the first skating test, audit those
descriptors from the input-reachable states, resolve each `Section.index`
through the generated `INDICES` array where the native graph does, and group
unhandled occurrences by their source callback. Preserve typed pending errors
until the corresponding callback and its owner checks are mounted; continuing
simulation after an unknown callback would make later observations unreliable.

The first bounded Action batch is Kickturn50. Its selection expression102 is
already mounted. It can reach `IsRidingFakie` condition100 at `0x82c9a308`
and `PhysicsSpeedCompare` conditions101–104 at shared `0x82c99808`; the latter
use distinct authored comparisons. The native callback reads Motion+0xa4
`mSkateboardSurfaceSpeed` when `alongSkateZ` is false and Motion+0xa8
`mSkateboardZSpeed` when true, then calls the existing comparison evaluator.
IDA's PDB distinguishes Motion+0xa0 `mDeckSpeed`; an older evidence note
misnamed +0xa4 and has been corrected. State50 Update at `0x828ed728`
expands root, OnBoard, and OnGround once, then updates occurrence129
`CreateConstMGIntent(NewKickTurn)` and occurrence130
`CreateMGIntentFromAGIntent(KickTurn)` at packed slots31/32. The current Rust
batch reuses the existing physical/animation owner readers and those two
behavior dispatchers. The independent IDA review passed, and `cargo check
-p skate2-core --lib` passed. The new ignored original-data raw left-stick
test reaches frame8 and then stops at Motion
`ResetTimeSinceKickturn::Begin` occurrence1404 (`0x82ca1378`) before a
completed Kickturn frame. That callback is now source-reviewed and mounted:
Begin and End both write +0.0 to the existing retained Motion timer through
the same owner-checking adapter; Update is the native shared BLR. Its
independent source/code review passed. A fresh original-data replay moved the
same frame8 boundary to Motion occurrence1908 `KickTurnSteering::Create`
(`0x82ca11b8`). After the shared class was mounted, the 45-frame replay
entered Action Kickturn50 at step8, sustained through step25, then exited to
Riding94 on left-stick release at step26. This is scripted state/path coverage,
not an Xbox numeric-parity claim.

The same static route identifies a larger shared dependency rather than a
string of unknown one-off indices: Motion KickTurn715 has children FromFakie
1016, Right1017, and Left1018. Right/Left use one KickTurnSteering class
(instances1905/1908), whose Create/Begin/Update and state Update thunk are
now mounted. IDA found a 0x1c runtime instance, source graphs
`anim_motion.kickturn_balance` and `.kickturn_spin`, two live cvars, and ordered
BalanceAtt/SpinAtt writes. The typed payload and graph/state declarations
already exist; the animation/intent/PhysOut output owners are present, and the
kickturn graph and cvar provider connect to the same runtime Vault.
Board `Skateboard::UpdateSteeringTilt` is a separate native class and must not
be substituted. This shared-class batch, with a sustained scripted left-stick
test, precedes another human turning check.

IDA details for the steering batch: native `KickTurnSteering` vtable
`0x823003b0` calls Create `0x82ca11b8`, Begin `0x82c95428`, Update
`0x82cc5860`, and shared-BLR End. Create allocates a 0x1c instance, sets its
first-update byte and four +0.0 floats. Begin resets elapsed/balance/input
and samples `kickturn_balance` at zero. Update resolves IAnimatable,
IMotionGraph, then ISkaterMotionGraph; first update calls SetAttributes and
gets positive base-animation length, later updates add dt to elapsed. It
evaluates balance and spin graphs in that order, applies live smoothing and
the original `fsel` clamps, reads the *unfiltered* KickTurn intent (or last
nonzero value), then appends BalanceAtt before SpinAtt to the retained Motion
attribute list. The graph provider is `anim_motion_kickturn` at global
attribute collection +0x144, class key `0x90c3decf0105631e`, collection key
`0xa168897919752875`, layout 0x5c0; spin graph +0xa0 and balance graph
+0xf0 are eight-point `PointNegGraphData8`. Mounted live definitions are
`Debug_Physics.kUseFineKickturn` (false) and
`Controller_KickTurn.kKickTurnSmoothing` (0.5, range 0..1). Native float
selection and one-shot flag effects are preserved in the mounted callback.
Independent IDA review corrected one instruction-order detail: the target
product precedes the live smoothing read.

The next reached Motion test is landing-to-anticipation transition1461, not
the initial Ollie entry. Land22 can select Anticipation1030 through expression
196 (`!HasAnimAttribute(absorbLand)`). Transition1461's hook section is
`{count:1,index:12}`; `INDICES[12]=11`, so the callback is hook object11
`OverideNextAnimTransitionHook`, authored with 0.8-second channel blend and
channel weights. IDA `0x8292b3d0` confirms the index indirection; hook callback
`0x82ca6be8` writes retained Motion transition parameters. The dispatcher
now resolves the retained object through `INDICES` after Exit and before
Enter. Neighboring Land transitions
1460/1462/1463 share this hook class with distinct objects10/12/10. A
scripted Ollie followed by a second right-stick-down point during Land should
assert the hook write, one-shot animation consumption, and subsequent state
entry before another hands-on run.

Later correction and validation: the first state-only steering probe
missed two real defects. Native global variable names BalanceAtt/SpinAtt must
resolve to literal strings Balance/Spin; the Vault collection's high/low
words must form A168897919752875. Both are fixed. Original-data steering now
asserts nonzero physical Balance/Spin AND substantial board part6 orientation
change, completing45frames. Fresh Ollie60frames also passes after correcting
PrepareLand's animation provider class to BABB3EC110849E08. Combined log:
`artifacts/astra-original-ollie-steering-20260924.log`. The integrated Up/Down
and29-state flip-family batch passes4861core units; family gameplay replay is
still pending and full numeric reference parity remains unproven.

Further static Action batches were Up/Down flick states151/155 (shared
expression11/12 and Update thunk, BoardAdjust145 pending), offboard Running27
and Dismount32 shared thunks, offboard-air condition145, and grounded grab
states81/82 shared Update. These are potential reachable gaps from compiled
data, not observations from the latest controller runs. Grinds and broader DMO
gameplay remain parked for the first skating test.
