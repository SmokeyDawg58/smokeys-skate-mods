# Motion expression 219 (kickturn from fakie) and the grounded AnimationToBoard assertion (2026-09-27)

**Status:** rustfmt PASS on every new hunk. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## 1. `Tick(Actor(Motion(PendingExpression(219))))`

The failure came while experimenting with the controls, after 4685 frames.

- **Graph.** Expression 219 guards transitions 1524 and 1525, both owned by KickTurn715 with source FromFakie1016. They go to Right1017 and Left1018. The target Pre (347 = c316 && c291, 348 = c317 && c291) picks the side through ValidTarget.
- **Native.** Motion Pre switch 0x8290aa8c case 219 is at 0x82910654. It reads `sConditions+0x4ec`, i.e. condition 315, `StateConditionDirect_WillExpire`, cached at word 0x24 bit 0x08000000 (index 315 = word 9, bit 27) with its value in word 0x58.
  - Condition 315 is `WillExpire(InTime=0.25)` in the authored data.
- **Port.** `219 => c(315)?` in `generated_motion_runtime_selection.rs`. The generic `ConditionPayload::WillExpire` arm evaluates it; there is no index gate.
- **Rest of the family was already covered:** KickTurn715 (413 Pre/Sustain), FromFakie1016 (346), Right/Left (347/348 Pre, Sustain c291), updates 1016 (828f2aa0) and 1017/1018 (828f2b18), and exits 1521/1522 (41)/1523.

## 2. `Tick(Before(Skateboard(Update(Update(Roots(Assertion(AnimationToBoard)))))))`

The skater landed at an unusual angle; this came after 433 frames.

- **Path.** The grounded Skeleton update (`try_update_grounded_read`) calls `SkeletonRootTransforms::try_update_grounded`. `GroundSkateboard` 0x82b5e488 re-arms the heading every ground frame. Then `UpdateRootTransforms` 0x82b42938 runs.
- **Native check.** 82b42d34..2e94 compares every xyz lane of `mAnimToBoardPos` with itself (`vcmpeqfp128.`). Any NaN hits `xcore::FailAssert("physicsskeleton.cpp", 3128)` and DebugBreak. The port's `Assertion(AnimationToBoard)` is that same debug assert.
- **AnimToBoard** = Reckoning.mTransform * (AnimAngle * translate(-skate position)).
  - The heading angle (`AngleBetweenVectors` 0x829b0628, port `try_signed_angle`) cannot create a NaN: degenerate input returns 0 and the dot is clamped before ACos.
  - So the NaN came in through the Reckoning transform or the skate animation.
- **Reckoning inputs.** `Reckoning::UpdateGroundStates` 0x82de0408 stores its two vector arguments raw (+0x1a0, +0x1e0) and tail-calls `UpdateGroundStatesFixedHeading`. `CalculateTransform` 0x82dd3da0 builds x = up × heading and z = x × up with unguarded `vrsqrtefp` plus two Newton steps. The port matches both.
  - A heading parallel to up gives a zero cross product, and the original would produce the same NaN.
- **Conclusion.** The port reached a grounded frame whose heading (the live board's forward axis) was parallel to the Reckoning up, i.e. a near-vertical board treated as grounded. The original presumably never reaches that state (a landing that steep bails first). The cause is therefore upstream, and is NOT yet identified. No guard was added, because native has none, and one would hide the real divergence.
- **Diagnostic.** When this specific assertion fires, `try_update_grounded` now prints one fatal-path line with `std::eprintln!`, so it shows even in quiet mode:
  - Reckoning up, heading and transform;
  - `skate_animation`;
  - the heading-yaw matrix;
  - the board transform.
  The error itself is unchanged. The next repro should show which input carried the NaN and the board orientation at that moment.

## 3. `Tick(Actor(Motion(PendingTransitionHook(4))))`

A session marker was placed while on foot and used to teleport; this came after 706 frames.

- **Graph.** The number is the transition index. Transition 4 goes from SpawnDismount31 (OffBoard/OBGround8, playing `OFB_SPAWN`, blend 0.1, speed 0.5) to Locomotion33, guarded by expr 2. Its only hook is HOOKS[0], `OverideNextAnimTransitionHook` with `time=0.0, transType=play`. No other transition uses hook 0.
- **Why it was gated.** Earlier batches admitted only the audited blend/channelblend Override hooks. Play hook 0 and blendWithCurrentFrame hook 8 were left gated.
- **Native audit.**
  - The ctor `OverideNextAnimTransitionHook` 82c8e428 defaults to Blend (2). It maps "play" to 1 (82c8e4d8 `li r11,1`), "sequence" to 4 and "channelblend" to 3. `time` becomes the blend length (default 0.2), and it reads the blendWithCurrentFrame (bit31), transitionUnder (placement) and useChannelFromWeights (bit30) flags. The port parser matches this exactly.
  - `OnTransition` 82ca6be8 resolves the entity IMotionGraph and passes `mTransitionParams` to vtable slot 6. That is the same write the port does for every Override hook.
  - The consumer, `PlayAnimation::Begin` 82cc9818, uses the override when its type is non-zero (Play = 1) and clears it. `dispatch_request` maps Play to `AnimationDispatch::Play` (time 0). That dispatch path is already exercised by authored `transType=play` animations such as Teleport5's TELEPORT.
- **Port.** Transition 4 is admitted in the Override gate in `generated_motion_runtime_lifecycle.rs`. Hook 8 (channelblend + blendWithCurrentFrame, 32 transitions) stays gated because its blend-with-current-frame consumer path is still unaudited.
