# Known air after manual pops: parity audit + trace (2026-09-26)

**Report:** tricks out of a manual *sometimes* land in plain Air (201 not reached): slower body spin, the camera follows, and the pop looks lower. It happens without a revert, apparently depending on how deep the manual is. Flat-ground tricks reach KnownAir correctly.

**Status:** static IDA comparison of the whole KnownAir decision chain found **no divergence**. An opt-in diagnostic trace was added (no behaviour change). rustfmt parse PASS; NOT compiled/tested. SOLO.

## How KnownAir is chosen (native, all confirmed)

1. **Pop detection.** `Update1_ProcessInput` 82e03944..39e4:
   - `mFramesSinceJump` (+0x128) increments every frame.
   - A pop is detected on the rising edge of `mStartJump` (flags8c4 0x400000; the latch is stored at +0x120 bit 0x40000000) or on `mStartFootJump` (flags8d0 0x10000, read through `lhz`).
   - Detected: the counter resets to 0 and `mStartJump` is kept.
   - Not detected: `mStartJump` is cleared (`rlwinm 0,10,8`).
   - Port: `physical_player_process_input_jump_publication.rs` matches.
2. **Selector poll.** Update1 82e03a88..3aa4 writes `TrajectorySelector::Update()` into flags8c4 0x400 (`insrwi 1,21` = `mTrajectorySelectorReady`). The port matches.
3. **State change (Update2).** `CalcSuggestedState` 82ddbd58, all cases compared (Ground, GroundAnimation, SlideGround, RevertGround, Air, KnownAir):
   - Ground (100) and GroundAnimation (103): `mFramesSinceJump == 1` gives `200 | ready`.
   - Air (200) gives 201 whenever 0x400 is set.
   - SlideGround and RevertGround never go straight to 201.
   - Port `physical_player_system_logic_session.rs` matches.
4. **Pop velocity.** Only GroundAnimation pops: `Toolkit_CalcGroundJump` 82b315a8 has one caller, 82d8a7ec. PhysicsGround's `mJumpInfo` is wall-ride only.
   - 103 is entered when `Skeleton::IsSkateboardAnimated` 82b4cd08 is true: mode +0x3df0 == 2, or `|animated board zAxis.y (+0x3060)| > threshold` with mode != 1.
   - `ForcePhysics` sets the mode: `OnGround24` = FORCE_PHYSICS (1, which covers manuals), `Trick19` = FOLLOW_ANIMATION (0), `InAir21` = FORCE_ANIM (2). End is a no-op.
   - Ports `dynamic_assembly_skeleton_animation_query.rs`, `skateboard_ground_jump.rs` and `actor_motion_neutral_lifecycle.rs` match. The field names come from the IDA type `ProcessedPhysIn`: `mJumpNormal` +0x240, `mGroundPos` +0x1e0, `mBodyZAxis` +0x160, `mJumpStrength` +0x950, `mJumpAdjustX/Z` +0x954/958, `mGravity` +0x964, `mSurfaceSpeed` +0x96c.
5. **Launch at the pop.** GroundAnimation `UpdateSkateboard` 82d8a7b0: `GetLaunchInfo` (82b46098), then `startVel` = jump velocity (not `velBeforeJump`), `playerJumped` = 1, `Launch` then `Update`, then selector+0x600 is copied to +0x50.
   - PhysicsGround's wall-ride path (82d9a430) also sets `velBeforeJump` and Processed+0x190.
   - Port `physical_player_ground_jump_runtime.rs` matches, including the GroundAnimation-only `startVel` write.
6. **Selector.**
   - `Launch(info)` 82dc4098 ignores the launch while `mJustStartedQuery` is set; otherwise it resets, copies the info and sets `mReady = 0`. `AdjustStartingVelOnVerts` 82dbfbc0 is a no-op on flat ground.
   - Private `Launch` 82dc2800 builds the start position from the slope/vertical-velocity graphs and the COM-above-ground distance along the Reckoning Y axis. `InitTrajectoryInfo` 82db9018 builds the fan (cone speed = `playerJumped ? |velBeforeJump| : |startVel|`, confirmed from the raw instructions). `CalcMaxTimeForTrajectory` 82dc0118.
   - `Update` 82dc6528: winner collision time < physics_trajectory_default +0x1e8 gives `ready = 0`; otherwise `ready = 1`.
   - Ports matched: `physical_player_onboard_launch_prefix.rs`, `physical_player_trajectory_adjust_start.rs`, `physical_player_trajectory_launch_prepare.rs`, `physics_trajectory_fan.rs`, `physical_player_trajectory_max_time.rs`, `physical_player_selector_update.rs`.
7. **Air relaunch.** `PhysicsAir::Update` 82d9f4b0: `addic.` then `bgt` on the timer (CORRECTED: 32-bit record-form compare, so a blocked relaunch retries every frame while the timer is <= 0; see `air-relaunch-timer-cr0-2026-09-26.md`), then `!SkeletonInContact`, `mPhysicsState != 0` and `!mJustStartedQuery`, then `LaunchTrajectorySelector` 82d99010 and timer = 7.
   - The port's unsigned "old <= 1" check is equivalent.
8. **Scheduling.**
   - Native frame: Update1, Update2 (`SetPhysicsState` Exit/Enter), Update3 (the new state's `UpdateSkateboard`) in the same frame. The port matches.
   - Predictor `Update` runs in `SimUpdate1` 8289f8f0 and `SimUpdate2` 8289fce8; the port harvests at both.
9. **`SetTrickHeight::Begin`** 82cc6508 (FromManual: `from AnticStrength`) matches. The authored `intent` attribute is not read.

## Not yet compared

- The ballistic sweep, which is shared with camera prediction.
- `CalculateTrajectoryScores` / `AddWallRideScores`; these matter only when 7 trajectories are active (flags8c4 0x2000).
- `CalculateCOMTrajectory` / `LaunchSecondPass`; these run after readiness is decided.
- The animation-side producers of `mStartJump`, `mJumpStrength` and the animated board pose.

**Leading candidates for the failing case, to be decided by the trace:**
- (a) The pop is detected while still in 100: the animated pitch is below the threshold on the pop frame, so there is no physics pop and no launch.
- (b) The pop happens in 103 but the winner time is below the minimum: a weak FromManual/AnticStrength pop.
- (c) The launch is ignored because a stale query is in flight.
- (d) A divergence in an upstream input (COM, jump strength).

## Trace (`SKATE2_TRACE_KNOWN_AIR`, stderr; with `SKATE2_TRACE_TRAJECTORY` for queries)

Launcher: `play-known-air-trace.bat`, which writes `known-air-trace.txt`.

**Pitfall (first capture came back empty).** skate2-core's crate-local `eprintln!`/`println!` macros (`diagnostics.rs`, `#[macro_use]` in `lib.rs`) mute ALL output under `--quiet`, including every `SKATE2_TRACE_*` site. Trace launchers must omit `--quiet`; the launcher was corrected.

| Line | Emitted from | Meaning |
|---|---|---|
| `update1 state= start_jump= foot_jump= pop_edge=` | jump publication | Pop event and whether it was detected |
| `skateboard animated mode= board_z_y= threshold= result=` | the 100/103 selection (mode != 1 only) | The 103 gate |
| `update2 A -> B frames_since_jump= selector_ready=` | state changer | Every transition |
| `pop velocity= strength= jump_normal= com= ground=` | GroundAnimation | `CalcGroundJump` result |
| `selector launch <Accepted/AlreadyStartedReturn> ...` and `selector start_position=` | `Launch` | Launch inputs |
| `selector ready -> x` | Update1 publish | Readiness changes |
| `air relaunch ...` | Air update | Why a relaunch did or didn't happen |

`SKATE2_TRACE_TRAJECTORY` adds the ballistic query inputs/results and the selector's winner time against the minimum.

**Files:**
- `physical_player_process_input_jump_publication.rs`
- `physical_player_system_logic_session.rs`
- `dynamic_assembly_skeleton_animation_query.rs`
- `physical_player_ground_animation_update.rs`
- `physical_player_onboard_launch_runtime.rs`
- `skeleton_processed_contact.rs`
- `physical_player_air_update_prefix.rs`
- `play-known-air-trace.bat` (repo root)

## First trace result (3 manual tricks; the 3rd felt wrong)

**All three pops were normal:**
- They ran in GroundAnimation, strength 0.0, jump normal (0,1,0), vertical velocity about 2.2 m/s.
- The launch took 7 trajectories. The winner time was about 0.47 s against a 0.25 s minimum.
- ready = 1, and 103 went to 201 at `frames_since_jump == 1`.
- The pop and the prediction are NOT the problem.

**What differs:**
- **Trick 3:** one frame into KnownAir, `update2 201 -> 200`, followed by `air relaunch skipped (skeleton contact)`.
  - KnownAir left through the `mIsSkeletonInContact` (flags8c4 0x8000) branch of `CalcSuggestedState`, whose Exit resets the selector.
  - Air's only relaunch chance (timer 0 → -1 on its first Update) was blocked by the same contact, and the port's 64-bit `addic.` reading never retried. CORRECTED: native retries every frame (32-bit compare), which is the fix in `air-relaunch-timer-cr0-2026-09-26.md`.
  - It stayed in Air to landing (200 → 103 → 100).
- **Trick 1:** the same 201 → 200 happened three frames in, but the contact had cleared by Air's first Update. The relaunch launched, became ready, and went 200 → 201, so it felt normal.
- **Trick 2:** no contact.

**Native source of the flag:**
- `Skeleton::FillPhysOut` 82b596e4 copies `SkeletonCollision::mCompliantJustCollided` (+0xFBE) into PhysOut `mCollision+0xD75`, which becomes next frame's flags8c4 0x8000.
- `SkeletonCollision::Update` 82b4b8f0 (from `UpdatePostPhysics` 82b57888) sets it for a compliant part's non-self, non-ignored contact that is NOT filtered as ground plane.
- `CalculateIsGroundPlane` 82b3e720 is skipped when `mIsDangerZone` (flags8c8 0x10000 `mAnimationDangerZone`) is set and `mIsDoingGroundTrick` (category == 600) is not. Its test is signed distance < max && (offboard || |n·gN| > min).
- The port's info fill (`skeleton_postphysics_collision_inputs_live.rs`) and `is_ground_plane` match native.

**Next trace:** a `skeleton contact` line (part, group, compliant, danger_zone, height above the ground plane, normal) at each counted contact in `skeleton_collision_update.rs`. It decides between a foot against a pitched deck (group 4), a ground contact with the danger zone on, and a wrong ground reference.

## Follow-up (2026-09-27): nollie 360 inward out of a nose manual sometimes pushes right

**Report:** since the relaunch fix, 360 inwards out of manuals are correct. A nollie 360 inward out of a nose manual *sometimes* shoves the skater a little to the right, which never happens natively on the same build.

**Static so far:**
- The port has no other record-form countdown read as 64-bit; Air's relaunch timer was the only one.
- `CalculateCOMTrajectory` 82dc02f8 matches. It copies the winning trajectory (+0x4d0 -> +0x50) to +0x890 and shifts it to the launch COM (+0x650). It then calls AdjustTrajectory(frame at +0x40, landing normal (+0x860) * |up . (start +0x930 - COM)| * graph 0x60, max 0x1f4), only when frame > 0.
- Still not compared: `CalculateScores` 82dc5e00, the ballistic sweep, and the animation producers.

**Trace additions (no behaviour change):**
- `known air: air frame state= frames_since_jump= contact= ready= com_velocity= velocity=` is printed on every Update2 in Air/KnownAir (prior-output COM velocity and velocity, flags8c4 0x8000/0x400).
- The trajectory line adds `landing=` (the winner's landing position), `start_com=` and `start_velocity=`.
- `play-known-air-trace.bat` now also sets `SKATE2_TRACE_TRAJECTORY`.

**Reading a failing capture:**
- A sudden sideways COM-velocity change on a `foot contact`/`skeleton contact` frame points to a contact impulse (the toe pose under the ground).
- A winner other than 0, or a landing offset sideways from the start velocity, points to selector scoring.
- A sideways change at a mid-air `selector launch` points to Air's relaunch path.

### First nollie capture (2026-09-27, the previous build plus `SKATE2_TRACE_TRAJECTORY`)

The capture has three tricks: a 360 hardflip, a nollie 360 hardflip out of a nose manual (fine), and a nollie 360 inward out of a nose manual (pushed right).

**The push is already in the pop velocity.**
- The travel line was about (-0.72, 0.69); the three pop positions sit on it.
- Horizontal pop directions: trick 1 (-0.62, 0.79), trick 2 (-0.73, 0.68), trick 3 **(-0.96, 0.28)**, which is 28 degrees off.
- Trick 3 stayed in KnownAir from pop to landing, and the central trajectory won (winner 0). The selector, the COM trajectory and KnownAir only followed the pop.

**`CalcGroundJump` did the native thing.** It reads ProcessedPhysIn `mJumpNormal` +0x240, `mPrepareJumpVel` +0x2c0 and `mVelocity` +0x190 (tangent swap above `kMaxJumpError`), and `mBodyZAxis` +0x160 with `mJumpAdjustX/Z` +0x954/+0x958.
- `mPrepareJumpVel` is HiLOD Update1_ProcessInput 82e037a4. It copies +0x190 while flags8c4 0x00200000 (the PrepareJump event) is set; otherwise it coasts by slope gravity. It is published to +0x2c0. The port matches, including r23 = 0x190.
- ProcessAnimAttributes 82b59de8: PrepareJump stores LStickX/LStickY (0x4a/0x4b) to Skeleton+0x3e34/+0x3e38. The finish path stores LStickX/**RStickY** (0x4a/**0x4e**) to +0x954/+0x958. This is native (a real quirk) and the port matches.
- In trick 3 the current velocity (next point) was 28 degrees off the saved one. That is beyond the error limit, so the pop used the current direction at the saved magnitude times the angle response (3.93 m/s, matching the trace exactly).

**The divergence is the board's own motion in GroundAnimation, 1-2 frames before the pop.**
- The `ballistic query` velocities and the toe-target `board_p` deltas agree for trick 3: (-3.15, 0.08, 3.16), then (-3.19, -0.15, 3.11), then (-3.74, **0.91**, 2.17), then (-4.23, **1.89**, 1.22).
- The animated board yaw (`board_z`) over the same GroundAnimation frames was +2, +11, +25 and +40 degrees (the backside 360 wind-up). The board also started lifting early: +1.5 cm, then +4.6 cm.
- Trick 2's board yawed -25 degrees (frontside) over the same frames, but its velocity stayed within 2 degrees of the line and it lifted only at the pop.

**Also compared and matching:**
- GroundAnimation Update 82d98d68 (UpdateGroundStatesFixedHeading, UpdateAnimatedSkateboard, UpdateSkateboard).
- UpdateSkateboard 82d8a7b0: SteeringTilt(0), wheel material, jump then return. Otherwise it computes front and rear foot force (1.0, 0.0), braking, SlideFriction(`mStateTimer` +0x974, clamped [0, attribute]), drag and CollisionForce. On a collision only type 0xF is added; otherwise types 4/5/1/2 plus drag.
- The transition resets both the HiLOD timer and +0x974 (82dff1cc..204).

**Next trace (added, env-gated, no behaviour change):** each non-pop GroundAnimation frame prints `ground-animation wheels distances/height=` (four wheel-ground distances) and `ground-animation forces` (the collision override, or front/rear/slide/brake world force at their board-local positions, plus drag). This shows whether the sideways turn comes from slide friction on the yawed board, the collision force, or solver contact impulses (wheels off the ground but still turning).

### Second nollie capture (2026-09-27, with the GroundAnimation force and wheel lines)

Same sequence as the first capture: a 360 hardflip, a nollie 360 hardflip out of a nose manual (fine), then a nollie 360 inward out of a nose manual (pushed right).

**Toolkit forces are ruled out.** In every GroundAnimation frame before trick 3's pop, slide = 0, brake = 0, drag = 0 and there is no collision force. The front and rear foot forces are equal (5 to 8 N, mostly downward), so they add no yaw. Only solver contacts can turn the board.

**The mechanism.**
- Wheels 0/1 (truck 4, board +z, the nose) stay planted. Their ground distance goes 0.031, 0.031, 0.034, then 0.026: on the last frame they are pressed 5 mm into the ground, because the animated pitch (|z.y| 0.215 -> 0.598) keeps the nose down.
- Meanwhile the angular soft drive yaws the board to follow the `B_N_360INWARDHEELFLIP_G` wind-up (FromManual940: PlayAnimation 0.05, JumpInto manualInto). The board heading changes by +2.2, +8.8, +14.2, then +16.1 degrees per frame.
- The planted wheels grip, so the board's travel direction follows its heading. Frame-to-frame displacement direction: 186.7, 185.4, 212.3, then 236.8 degrees, running 8 to 17 degrees ahead of the heading.
- The pop took the current direction (|current - prepared tangent|^2 > kMaxJumpError), so it carried the turn.

**Why trick 2 did not push.** Its animated board first levels (|z.y| 0.104 against a physical 0.27). Wheels 0/1 therefore lift (0.044, then 0.064) before the frontside yaw (-24 degrees) builds up.

**Both nose-manual tricks were ridden fakie.**
- In tricks 2 and 3, board +z (the nose) trails the travel direction, and so does the animation foot axis (part15 -> part19). At trick 3's first frame the foot axis is at 13 degrees and travel at 187.
- Before trick 1 both pointed forward: foot axis -150, travel -144.
- The whole skater turned about 188 degrees during trick 1. The foot axis went -150 -> -110 (ground wind-up) -> -56 -> -94 -> +38, mostly in KnownAir.
- In KnownAir, AnimToWorld = Reckoning x frozen animation_angle. So the Reckoning heading itself spun: that is the Reckoning::UpdateAirStates body spin.
- The nose manual therefore had its planted wheels trailing. That puts the pivot behind the centre, so the centre's velocity leads the heading.
- Confirmed (2026-09-27): this was a deliberate left-stick spin, not a port divergence. The reproduction is therefore a fakie nose manual (planted nose wheels trailing) into the nollie 360 inward, which never pushes natively.

**IDA-verified this pass (all match the port):**
- The `kSkateboardSoftDriveDisp`/`Str` initializers: 20000 / 10, with maxima 100000 / 100.
- HookEnableAngularDriveSoft 82b24ad8: k = Disp x s, damping 2*sqrt(k), strength Str x 3600. EnableAngularDriveOnly 82b2aca8.
- CreateHookDrive 82b2e4b8: the hook Assembly is inserted as a STATIC body.
- The drive angular limit is strength*dt^2 divided by the effective-mass denominator, so it is effectively unclamped here.
- SetWheelMaterialForSurface 82b29fb8: four Part +0x50 stores of PhysIn+0x908.
- UpdateSteeringTilt 82b29de0 and SetForceTurnAngle 82b24cf0: front/back contact-bit swap on flags8c4 0x100000, contact timers, 0.983 decay.
- UsePhysicsTruckTilt (Skeleton+0x3e41) is read only by UpdatePoseBuffer, so it is visual only.
- Reckoning::UpdateGroundStatesFixedHeading 82ddd468 and CalculateTransform 82dd3da0: the heading is frozen during GroundAnimation.
- Skeleton::UpdateAnimatedSkateboard 82b5bc58 and UpdateGroundSkateboard 82b5e468 both write the animated board to PhysIn+0. The ground version clears 8c4 0x80000.
- ApplyBoardAnimation 82b357f0; BlendToAnimatedBoard 82b25bd8.
  - Fast graph is airstates +0x50: x 0..0.299 rad, y 1.0..0.268; blending finishes once y >= 0.95.
  - UpdatePhysicsError 82b23aa8 runs every frame from PhysicsGround::UpdateSkateboard.
  - GroundAnimation::Update 82d98d68 passes bUseFastBlend = 1.
- Toolkit_CalcGroundJump 82b315a8 branch semantics: vcmpgtfp. + mfocrf + extrwi 24 = the CR6 all-true bit.
- Island::Step_UpdatePrePhysics1 827b3700: island flags == 3 selects 50 iterations.
- Reckoning::UpdateAirStates 82ddfde8 matches `reckoning_air.rs`.

**Trace added (no behaviour change):**
- `known air: reckoning spin` on every air Reckoning update: the anim body-spin input (None for the direct-target branch), the target, speed, angle and heading.
- `known air: board target` on every animated-board frame: the anim and applied board Z, and the physics-error blend state and quaternion.

### Third nollie capture (2026-09-27, with the `board target` line): what "sometimes" is

Two nollie 360 inwards at the end of the capture. The second one pushed far more than native ever does. Both had the planted wheels trailing: the tail truck (wheels 2/3) was planted while the board travelled nose-first, and the skater was fakie on a flipped board.

**The animation is identical in both.** The drive target (`board target` anim_z) heads 137 -> 152 -> 170 -> 184 -> 201 degrees over the 5 GroundAnimation frames, a wind-up of about 64 degrees. Its pitch |z.y| goes 0.22 -> 0.29 -> 0.46 -> 0.57 -> 0.60.

**The only difference is manual depth at the pop:**

| | trick 3 (no push) | trick 4 (pushed) |
|---|---|---|
| physical pitch at the start of 103 | 0.367 (deeper than the anim's 0.22) | 0.172 (shallower) |
| drive effect on the planted wheels | levels the board, so they lift (0.031 -> 0.038) | tips it up, so they are pressed (0.029) |
| travel direction over the frames | 132.4, 131.8, 131.3, 133.9 (straight) | 136.3, 143.0, 156.0, 162.8 (dragged round) |
| pop direction | 132.7 | 162.8 |

**Rule.** If the pop comes before the physical manual is as deep as the trick clip's start pose, the angular drive pitches the board up. That presses the planted wheels, which grip through the wind-up yaw and drag the velocity round. If the manual is already deeper, the drive first levels the board and the wheels unload.

**How fast the manual sinks.** The port's physical manual sinks at about 0.009 |z.y| per frame (~0.5 deg per frame). Trick 4 was popped 18 frames after the manual started, at 0.164 (9.4 deg). This matches the earlier unexplained finding that the animation's toes sit 3-6 cm under the ground during manuals: the animation expects a deeper board than the physics has reached.

**IDA-verified this pass (all match the port):**
- **Foot loads.** Toolkit_CalcFront/RearFootForce 82b28b70 / 82b28d98 select the load from mBalance +0x9a0: front 2/1/0 and rear 0/1/2 for balance <0, =0, >0.
  - mBalance is written only by ProcessAnimAttributes 82b5aa00 (= -Balance).
  - Andale::Transition::GetAttributes 82a35120 forwards to mToAnim only.
  - ProcessedPhysIn::Reset runs every Update1_ProcessInput (82e0200c).
  - So the equal foot loads in GroundAnimation are native.
- **SetTruckDriveDynamics 82b2cb08.** All 7 attribute keys match, both deck profiles, and the force-turn branches.
- **CalcManualEffect 82b32130** vs skateboard_manual_effect.rs, with ApplyAngularDisplacementToDeck 82b36798 and TargetAngularDisplacement 82b361e0: torque = I * displacement / dt^2, added to the deck torque.
- **SetManualAngle::Update 82cc2920 / Begin 82caf320.**
  - Intent curve at anim_motion+0x420/0x440, with the negative branch = -curve.
  - Velocity clamp a353ce1670d3aa40, then acceleration clamp aed41843fd93af02.
  - AddSettableAtt(name, angle, false, -1); Begin zeroes angle and velocity.
- **Update1_ProcessInput 82e03130 8c8 wheel bits.**
  - Unflipped: 0x10000000 = wheels 2|3 and 0x08000000 = wheels 0|1. These two swap on 8c4 0x100000.
  - The left/right bits 0x04000000 / 0x02000000 come from wheel pairs 2|0 and 3|1.
- Note: tools/vault_dump.py cannot read explicit scalar entries. It shows `<no pointer>` even for physicsdeck width, which has a real value, so do not use it for scalars.

**Trace added (no behaviour change):**
- `known air: manual effect`: balance, flipped, target, actual, error, the P/I/D terms, torque, wrong-wheel flag, 8c8 and scale, on every active manual frame.
- `known air: manual angle`: intent, target, angle, velocity and limits, on every SetManualAngle update.

#### Manual values, controller rate and the trick crossfade (2026-09-27, static)

**Real vault values.** tools/vault_dump.py tested the inline flag at node byte +15. The Rust parser uses +14 (`node_flags`), so a scratch copy was fixed; the repo tool is untouched. The copy reproduces physicsdeck width 0.24 and length 0.59.

physics_manual.default:

| Attribute | Value |
|---|---|
| P / I / D | 0.05 / 0.03 / -0.8 (powerslide 0.03 / 0.04 / -0.4) |
| target | 24 deg |
| error clamp | 0.1 |
| D clamp | 0.04 |
| deck delta | 6.0 |
| wave | frequency 1.5, amplitude 0.05 |
| initial torque scale | 0 |
| wrong-wheel decay | 0.96 |
| wrong-wheel output scale | 0 (the wrong branch applies no displacement) |

The speed graph is at +0x50.

anim_motion (SetManualAngle):
- Acceleration limit 0.02 and velocity limit 0.04 per frame.
- The intent curve (+0x420/+0x440) is 0 up to |intent| 0.8049, then rises linearly to 1 at 1.0.
- So manual_angle takes about 26 frames to go from 0 to 1.

**Controller rate.** With the error clamped at 0.1 rad, P*0.1 + I*0.1 = 0.008 balances D = -0.8 * rate at about 0.01 rad (0.57 deg) per frame. The port's observed sink rate of ~0.5 deg per frame is therefore native controller behaviour. A pop 18 frames into a manual finds a shallow board natively as well.

**Crossfade.**
- PlayAnimation ctor 82cbdab0: `time` is mTransitionParams.mBlendLength (default 0.2). The default type is TRANS_TYPE_BLEND. transitionUnder sets placement 1, otherwise 0 (OVER).
- play_animation.rs parses the same values, and play_animation_lifecycle.rs dispatches Blend, so FromManual940 should crossfade from the manual pose over 0.05 s.
  - GetBlend: smoothstep above 0.05, giving about 0.26 / 0.74 / 1.0 over 3 frames.
- The Begin override (`next_transition_override`) is replaced only by OverideNextAnimTransitionHook. Of the 42 hooked graph transitions, none is on the nose manual -> NoseTrick73 -> FromManual940 path, and the manual's own PlayAnimations clear any earlier override.
- Yet the capture-3 drive targets for the deep and shallow manual pops agree to within a constant 0.8 deg heading offset from their first frame. Either the board's crossfade has no effect, or both manual poses carry the same board orientation.

**Traces added (no behaviour change):**
- `toe targets` now includes volume0_z, the animated board Z in anim space, on every frame, including the manual.
- `known air: transition blend` prints coefficient, time and length on every TransitionBlend eval.

### ROOT CAUSE (2026-09-27): GroundAnimation's pop read the wrong "max jump error" cvar

**Capture 4 disproved the crossfade suspicion.**
- The `transition blend` lines show the trick's 0.05 s blend running native smoothstep weights: 0.26, 0.74, 1.0.
- `volume0_z` eases the animated board from the fixed manual pose (13.9 deg tilt) into the clip.
- SetManualAngle never updated (0 lines). Balance comes straight from the tree: -1 in the nose manual, dropping when the 0.3 s manual blend starts.

**The divergence.**
- Native `Toolkit_CalcGroundJump82b315a8` loads `kMaxJumpError`:
  - 82b315d4/82b315e4 `lis/addi` cVarFloatRef 8308dbc8, 82b315f8 `lwz mTarget`, 82b31600 `lfs f30,0xd8`;
  - f30 is splatted (82b31690..316e8) and tested at 82b31748 with `vcmpgtfp.` against the squared tangent difference;
  - CR6 all-true enters the correction; otherwise it branches to 82b31960 and keeps the prepared tangent.
  - Its initializer 82f7b3d8 sets default 7.0 and range 0..10.
  - xrefs: CalcGroundJump (82b315e4) is its only reader.
- The port's live GroundAnimation path (physical_player_ground_animation_update.rs) read `AirJumpFloat::MaximumError` instead. That is cvar 246, `kPhysicsAirJumpFixMaxError`.
  - Initializer 82f8e9b8 sets default 3.0 and range 0..20.
  - Its only native reader is `PhysState_PhysicsAir::CalcVelFromJump` (82d8b0d4).
- kMaxJumpError was not registered in the port at all.

**Why this is the push.**
- The prepared velocity (Processed +2c0, physical_player_prepare_velocity.rs) follows the board while the PREPARE bit is set.
- After that it only gains slope gravity, so through the wind-up it keeps the pre-wind-up direction.
- The pop keeps that prepared tangent unless |current - prepared tangent|^2 exceeds the threshold.

Capture 4 pops. Velocities are board_p frame deltas x 60 (x, z). The squared error was not traced, so it is an estimate.

| Pop (trace line) | Before wind-up | At pop | Est. error^2 | Pop velocity (x, z) |
|---|---|---|---|---|
| 4166 (no JumpInto) | (-4.07, 4.27), 133.7 deg | (-3.93, 3.92) | ~0.14 | (-4.07, 4.26) = prepared |
| 4836 (nose trick) | (-3.65, 3.49), 136.3 deg | (-3.09, 2.92) | ~0.64 | (-3.65, 3.49) = prepared |
| 5759 (nollie 360 inward, pushed) | (-3.01, 2.49), 140.4 deg | (-3.50, 0.76), 167.7 deg | 3.2 to 3.7 | (-3.47, 0.76) = current direction |

The 5759 range depends on which pre-wind-up frame (5666..5732) the prepared value matches; every candidate lies between 3.0 and 7.0.
- The port therefore followed the dragged direction (+27 deg): the push.
- Native keeps the prepared velocity for any error^2 up to 7.0.
- The drag varies from pop to pop. Only pops whose error^2 lands between 3.0 and 7.0 differ from native, which is why the push appeared only sometimes.

**Fix (rustfmt PASS; NOT compiled/tested).**
- Cvar 596 added: `kMaxJumpError`, Physics_Skateboard, title " MaxJumpVelError", Float 7.0. It is registered between kGroundProportionalBlend (55, table 82fd4430) and kShowManualInfo (144, 82fd4438), matching native table 82fd4434.
- DEFINITIONS is now 597 entries; REGISTRATION_ORDER was reflowed by rustfmt for that hunk only.
- New `InputCvarReferences::try_capture_ground_jump_maximum_error` (input_cvar_air_jump.rs) is now used by the GroundAnimation pop.
- PhysicsAir's CalcVelFromJump keeps 246, as native does.
- New trace `known air: ground jump error_squared= maximum= corrected= current= prepared=` (skateboard_ground_jump.rs, env-gated) prints the exact values at every pop.
- The title-order oracle test is now 53 labels behind (pre-existing staleness).
