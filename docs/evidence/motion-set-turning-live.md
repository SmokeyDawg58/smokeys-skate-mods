# SetTurning1413 retained Motion callback

Author draft, 2026-09-19. Fresh IDA raw instruction checks plus decompilation;
not an original-executable trace or a claim that these unregistered files run.
No Cargo run by author. Standalone rustfmt completed successfully.

## Native ownership and lifecycle

SetTurning::InstanceCreate82cb27b8 allocates 0x14 bytes, alignment0x10;
null remains absent, success stores state+10=2, time+8=0, value+c=0.
Begin82cb2798 writes that same state/time/value order; End is BLR.
Update82cc8408 resolves IPhysical, IAnimatable, ISkaterAnim, IMotionGraph,
ISkaterMotionGraph. ISkaterAnim vtable82302c08+0c resolves GetIsRidingFakie
82cb7530, player flags bit20000000; +1c resolves GetIsMirrored82cb7598,
bit40000000. These are different flags and separately timed reads.

GetIntents8728 precedes five separately guarded PhysOut.Animation reads:
target lean+24 at8750, turn+20 at8770, direction+38 at8790,
quickness+34 at87b0, holding+3c at87e0. Four Reshape82c954e8 calls at
87f4/8828/885c/8890 use (turn,direction) quick, (target,direction) quick,
(turn,direction) slow, (target,direction) slow. Both target results are unused;
all their graph/cvar/scalar reads still execute. Blend quick*q+slow*(1-q)
uses rounded slow product then FMA, then XOR sign bit independently in X/Y.

Retained SlidingState is existing ActorSkaterGraphState slide_time/slide_bits/
combined_slide_bits, also produced by PowerSliding29. Capture left/right flags,
clear force20000000 left89bc then right89c4. State2 needs combined enabled40000000
and right/left start80000000 or captured force. Right wins both. Without either
force, reset left/right times and capture riding-fakie into combined80000000.
Transition debug reads cvar230; active states select angle +/-1 using freshly
sampled mirror and combined fakie. End40000000 or disabled transitions to2.
Changed state resets instance time; state store8aec precedes every tuning read.

Global anim_carving_default+174 time graph290 (8 points x2a0/y2c0) blends
old value toward target:8b40 rounds curve*target,8b50 fuses
(1-curve)*oldvalue plus that product. Independent review caught and corrected
the initial draft's reversed FMA; bitwise regression expects3ea2f584 (the
wrong order produces3ea2f585). Scalar
d35ca9898b86827a limits absolute per-update change, NOT dt-scaled.
Lower FSEL precedes GetDeltaTime8bbc, dt+oldtime store8bcc, upper FSEL then
value store8bdc. Motion.ZSpeed+a8 abs precedes scalar5339323f741710db.
Raw8c58 BO4/BI25 !GT skips speed graph for unordered; only ordered>
evaluates graph240 (8 points x250/y270) with speed-threshold.

Five AddSettableAtt writes at8cc4/8ce8/8d0c/8d30/8d54 publish angle,
direction, quickness, speed, holding, using actual authored payload names,
normalized=false, sequence=-1. Existing try_add_gesture_attribute preserves
non-NaN assertion and storage semantics. Every prefix stays published on failure.
Always read Global anim_motion_power_slide+14c scalarbd26094927a0947a afterward.
State1/0 append Motion turn +/-scalar, then read RightSlide/LeftSlide (missing0),
AdjustSlideForFakie82c8a5c0, append slide. Other states read fakieturn and append
turn only if present. Final debug230 sampled in every branch. DebugOverlay
Get remains required when true although native draw target is release BLR.

## Reshape arithmetic and tuning

Both polar passes multiply X*X and Y*Y separately, add, rsqrt estimate and
two refinements; length uses existing length_from_squared_and_estimate.
Threshold37800000 at822f21d0 is 2^-16; raw55a0/5828 !GT skips atan includingNaN.
Positive length computes one reciprocal refinement of X, ratio FMA(Y,inverse,0),
existing XMVectorATan helper, signed PI quadrant add for X<0, signed halfPI forX==0.
The reciprocal/atan calls still precede the X==0 selection.
Raw constant822f21dc is +PI40490fdb. Negative angles add PI (NOT 2PI) before graphs.
Magnitude graph quick120 or slow0 has16 points x+10/y+50; graph executes before
MagMap boolean, disabled substitutes1. Multiply original length, standalone
XMVectorCos824b0130 followed by XMVectorSin824b0060, separately scale and clamp
with lower then upper FSEL. Existing standalone helpers are reused, NOT SinCos
whose polynomial power dependencies differ. Host estimates/rounding, denormals,
NaN payloads, fused-zero signs and FP status remain existing explicit boundaries.

Second polar pass uses clamped XY; angle graph quick1b0 or slow90,16points.
Graph precedes live AngleMap; disabled uses positive mapped input. Source then
selects mapped or mapped+PI based on signed second angle. Fetch offset quick
1994feea33b86bdb or slow116645a04f17e2e7, add, sample AngleOffset; disabled
SUBTRACTS offset from rounded sum rather than replacing pre-add value. Cos/Sin
and second signed-unit clamps finish XY; native zeroes ZW (not stored host data).

Shared original attr class705bd1b12aefb73a, collectiond7edbd362d7d2152;
allocation828b3258, generatedctor828b1518 called326c, publish+174 at3278.
Generatedctor1568 explicitly supplies zero DefaultDataArea2e0 when no layout;
provider therefore retains that generated fallback. Missing scalar ->DefaultDataArea4.
Current Vault queried live, same physical Vault checked at first use.

## Cvar registration packet

All section `Animation_Skater_Carving`, bool default true, null callback.
Raw constructor calls ordered below, original initializer table also ascending.

|Index|Label|Title|Reference|Initializer / ctor call / table|Section / label / title strings|
|---|---|---|---|---|---|
|234|kUseMagMap|Use Mag Maps|830a380c|82f86528 /82f86554 /82fd4f18|822fb4f0 /822fb4e4 /822fb4d4|
|235|kUseAngleMap|Draw Angle Maps|830a3510|82f86578 /82f865a4 /82fd4f1c|822fb52c /822fb51c /822fb50c|
|236|kUseAngleOffset|Use Angle Offsets|830a42e0|82f865c8 /82f865f4 /82fd4f20|822fb56c /822fb55c /822fb548|

The middle title really says Draw Angle Maps despite controlling the map use.
Each of the four Reshape calls samples all three independently after its relevant
graph/scalar. Do not snapshot them at callback entry or use debug false defaults.

## Integration / validation

Mount actor_motion_set_turning.rs as motion_runtime_session::set_turning;
it owns kernel children motion_set_turning.rs and motion_set_turning_reshape.rs.
Mount actor_motion_set_turning_attributes.rs as input_cvar_runtime::set_turning.
Retain one TurningAttributes in actual runtime services alongside existing
PowerSlidingAttributes; no parallel Physical/Animation/Motion owner. Lifecycle
instance lives in existing generated behaviour instance enum, create allocation
must use existing instance allocator and preserve absent allocation.
Pass authored five names, live TurningRuntime services and controller dt.

Tests written in motion_set_turning_tests.rs cover four required reshapes/live
12cvar reads, ordered five writes, unordered speed gate, right start priority,
forced flag clearing/preserved slide times, partial publication failure and
instance persistence, fakie nonclamping fold and allocator-null/Begin behavior.
Author ran rustfmt only. Root must mount, compile, run tests and original neutral
1170 corpus continuation; source_review independently checks this packet/drafts.
