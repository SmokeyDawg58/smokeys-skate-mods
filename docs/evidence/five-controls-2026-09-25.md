# Five requested controller flows, September 25

COMPLETE for these five scripted sequences: final release replays and155
focused regressions PASS. Work performed solo with visible
game replays. The requested sequences were:
offboard walk/jump; two pushes and bottom-left left stick; bail/respawn;
ordinary ollie with left stick left; offboard right stick left.

## Baselines and verified fixes

Scripts are raw input samples in `live_skating_client.rs`, selected with
`--live-controls-demo offboard-jump|push-bottom-left|bail-respawn|bail-auto|offboard-right-left`.
Ordinary ollie uses `--live-ollie-demo --live-ollie-spin-left`. New control demos
use truly neutral buttons (0); older demos historically retain D-pad Up.
Offboard jump is X, confirmed from Action input fill; bail recovery is A or X.
No states or physics outputs are forced by these scripts.

- Ordinary ollie / left spin: initial 200-tick launched-game replay PASS.
  `artifacts/five-controls-ollie-left-before-20260925.log`.
- Bail: `TeleportSetPhysicsState(WrongStage)` at frame 294. Native
  TeleportPlayerNow82e00618 explicitly calls SetPhysicsState(Ground) at82e00994,
  after conditioners reset and before normal collision restoration. The existing
  transition implementation already handles Teleporting but its admission table
  omitted 702->100. Added that route. Both button recovery (A160) and automatic
  recovery PASS 540 ticks, return to physical100 / Motion1170, in
  `five-controls-bail-respawn-round1-20260925.log` and `bail-auto-round1`.
  Note: baseline `bail-respawn-before` predates the A160 script addition and is
  an automatic recovery test. Round1 binary includes both separate variants.
- Offboard movement: frame173 contact-horizontal-length assertion. Gated
  diagnostic establishes all authored locomotion vectors remained zero, while
  sequence durations arrived. CreateLocomotionTrajectory82cac998 compares packed
  FastString words (vector branch82cad7b8..da34). FastString discards letter case;
  serialized clip names decode lowercase. Rust ingestion incorrectly used
  uppercase case-sensitive string comparisons. Fixed comparisons for the four
  vector names and two sequence timing names. Added encoder/decoder regression.
  Runtime revalidation pending.
- Offboard right-stick-left: SteppingBoardNoBoard1434 Begin is shared BLR,
  Update82ca68c8 reads PhysOut+48 OffBoard+3f mIsHoldingSkateboard (PDB checked),
  then IAnimatable+48 AddSettableAtt82d7f5f0 publishes Board/NoBoard value0.
  Connected existing retained owners and setter. Shared TurningRight753 /
  TurningLeft754 update828f1e88 invokes OffBoard1 then slots14..17; connected
  exact authored arrays. Runtime revalidation pending.
- Two pushes/bottom-left: IsPowerSliding1406 Begin82c9fec0 / End82c9ffb8 call
  ISkaterMotionGraph+70 ->SetIsPowerSliding825fe0f0, general flags04000000.
  Connected flag lifecycle (distinct from candidate sliding combined flags).
  Round1 reaches CreateSlide2266 next; full sliding branch now being connected.

## Slide branch source review

- CreateSlide.Begin82ca3160: ISkaterAnim+0c GetIsRidingFakie82cb7530; capture
  SlidingState+10 bit31 and reset left/right timers (+0/+8). Update82cc7e10:
  minimum strength key31503aab02ee76dd, minimum time09ef22b12364eb7f,
  turning keybd26094927a0947a. Select intent by right XOR captured fakie;
  preserve native minimum clamps and mirrored wrap at absolute0.5;
  publish existing MG `slide`, then `turn` attributes.
- PowerSlideDecel Create82ca3310 allocates10 bytes and initializes float+8=0;
  Begin82c954d0 resets it. Update82cb1e00 transforms Motion+50 deck velocity
  into Ground transform coordinates, zeros localY, takes length. Eight-point
  cached graph at Global anim_motion_power_slide layout+0 (x+10/y+30), then
  blend key2dc2d46dfba612cd with source FMADDS, retained instance value, optional
  debug singleton, IAnimatable `decel` setter. Authored `right` unused in native.
- PowerSlideSpin.Begin82cb2350 calls ISkaterMotionGraph+b0
  GetGroundHeadingCosine825f4ea8 (reuse existing), eight-point graph layout+50,
  optional debug singleton, sets `slidespin` to negated result. Update/End BLR.
- ShouldLeaveSlide82c9c918 selects side by captured from-fakie then reads end
  bit30 of selected SlideState. Existing PowerSliding.Update produces this bit.
- State update source thunks checked:828f2bb8 Sliding716/Left/Right parents;
  828f3fa8 Into1251/1253;828f4048 Out1252/1254;828f4a08 Into/Cyc leaves;
  828f4ab8 Early/Mid leaves;828f4a58 Default leaves. Each expands the shared
  Riding277 prefix once, followed by exact authored behavior slots.
- Pre221=c321,222=c322,223=c323;349=c319&&c320;350=c319&&c325.
  Raw switch checked at82910714/774/7d4,82915c14/cdc. All five Sustain cases
  dispatch to native default829221c8=true. No guessed defaults added.

## Validation still to complete

September25 continued source work (round4 release being built):

- Round3 offboard right-stick-left PASS540ticks/9seconds; returns to
  Standing/Still752 physical500. `five-controls-offboard-right-left-round3-20260925.log`.
- Ordinary ollie with left stick held THROUGH landing PASS300ticks, final
  Motion1170/physical100. `five-controls-ollie-left-round2-20260925.log`.
- Walk now passes the authored-vector assertion and reaches Motion319/physical504;
  round3 stops at expression422, not a contact failure. Added source expressions
  373..378,421/422 and distinct Sustain373 (no quit-lock gate). Native switch
  82917610..82917f0c,8291ae9c/8291b0b4; Sustain82921814/default829221c8.
- GrabSlide hook82ca6ac0 sets selected side force bit20000000 using captured
  fakie side, consumed by existing SetTurning. Added actual hook dispatch.
- Powerslide round2 reaches native selector request101. Constructor/payload
  existed, but CurrentPhysicalState, transition lifecycle, Update and output
  scheduling did not. Added distinct SlideGround101 using retained payload.
  Vtable823715a4: Update82d9afa0, Enter82d84c68, Exit82d84d48,
  Predict82d83ae8, GetType82dfe570, Fill825187c0 BLR, Post82d8e998.
  Enter enables truck tilt, resets spin, disables board animation, zeros actual
  deck angular XYZ, sets drag0, conditionally ResetManual, WipeoutGround, and
  initializes own fields. Exit resets push/damped turn then restores material.
- Slide Update82d8e2a8 reuses exact existing steering/manual/wallride/feet/
  anti-flip/collision calculators. WallRideRuntime now writes borrowed jump
  record fields, allowing Ground retained fields and Slide stack-local JumpInfo
  to use ONE calculator. Shared jump launch excludes state103-specific stores
  for the Slide caller. No Ground speed model/push/wobble path is substituted.
- Source orientation keys: Wheels ca0bc775dff39de6, surface f4/f8,
  Slide4515ae7409289c50, Manualcc65f65147211d65. Native physics_slide
  classfd2771103f9ad3db (ctor828b1158) added to original Vault selection.
- CalcPowerSlideForce82b27ff8 uses velocity190, ground1d0, deckX40;
  surface graph x10/y30 evaluated at absBodySpeed948; local height key
  27db05820432b632, hardness blend225b8e105a714976. Queue order/IDs raw
  verified82d8e944..974: front4,rear5,wall16,slide9, or collision15 only.
- Physics_Debug kApplyAntiFLipTOrqueTest ref830d7cd0, initializer82f8fa30,
  table82fd5788: false/null callback; registered before existing258/table578c.
- `cargo check -p skate2-pc -j1` PASS after new physics wiring; existing warnings.
  `five-controls-slide-check2-20260925.log`. Runtime validation still pending.

Finish focused tests and release build; replay all requested flows; fix any
further reached boundaries against IDA. Do not claim entire project parity or
all five scenarios complete until actual logs establish it. Existing four-control
and KnownAir milestones must remain intact.

## Round5/6 continuation

- Round5 SlideGround101 ran through the requested turn and returned to Ground100.
  At393ticks it reached missing IsRidingSwitch condition337. Native82c9a118
  calls checked ISkaterAnim.GetIsSwitch82cb7580: relative_stance==1, not the
  previous PhysOut switch flag. Connected that existing retained field.
- Slide state selection now follows its own CalcSuggestedState82ddc00c..c08c
  order: wipeout, revert, grind, contact/offground, then slide/ground.
- Offboard round5 reaches Motion319/physical504 then missing Pre431 at173ticks.
  Added native Pre431/432 at8291beb8/8291c13c: requested-air-mount AND matching
  jump foot AND past apex AND neither retrieving nor dropping, OR matching
  current state AND mount intent. Added Pre31/32 c61/c62; all four Sustain
  entries independently point to constant-return829221c8.
- IsFootDown82c8b788 uses trajectory foot_locked, current_state==0, time<0.1
  (native BGE permits unordered), and mirrored-bit40000000 parity. Distinct
  from mFootDown and two-foot-landing bit2. SprintMount samples sprint>=0.5.
- IsInJumpAir82c92f88 uses physical_feet[index].states[2].time, OffBoard
  attribute971e130c27193d76, and mirrored locked-foot parity. Activation82c9d8e8
  always queries TimeToFootChange82cb7720 then applies live kJumpCloseToLanding.
  Cvar initializer82f839a0/table82fd4da8: float0.3, range0..1, null callback.
- IsAirMountRequested82cb7658 reads player bit200. IsPastLocomotionApex82c94eb8
  resolves animation/physical then short-circuits IsInLocomotion82cb7600 with
  IsPastApex82cb7b48. The apex getter reads INPUT feet state2 velocityY/time;
  OneG initializer82f81038 supplies c11cf5c3, FDIVS then FMULS then orderedLT.
  PDB trajectory mIFC0/mPFC500 and interface offset52e0 verified.
- Round6 cargo check PASS (existing warnings). Release/runtime checks pending.

## Powerslide pass and offboard animation continuation

- Round6 two pushes/full bottom-left PASS540ticks/9seconds, final Motion1170
  physical100. Original-input replay logs state101 during the turn, then100.
  `five-controls-push-bottom-left-round6-20260925.log`.
- Round6/7 offboard walk reaches195ticks/Motion319/physical504 then HandsFace
  Stream Command UnsupportedSetup, before the scripted X press at220.
- Added source CanLandOnBoard.Activation82c9b588, existing OffBoard+42 flag
  read then optional debug singleton. Pre26782911988 and Sustain82920450
  both read condition400. This is reused output, no duplicate landing logic.
- Hand/face adapter investigation: SetUpBlendPoseStream825f8398 emits existing
  pose/blend commands then ExtractSQT82a2c600 -> ExtractData82a24400. Native
  extraction takes the LAST stack producer, with no depth-one requirement.
  EndCommands82a289e0 clears stack tops, retaining emitted commands. The host
  TreeCommands.output() formerly required exactly one root, so hand stream
  extraction now selects the top root for deferred output while retaining all
  commands. Body output validation remains unchanged. Regression added.
- FixedSqtId diagnostic now names bad IDs/capacity; round7 still reports the
  generic boundary, excluding that hypothesis. Round8 tests the extraction fix.

## Round8 offboard jump/recovery

- Top-producer extraction fixes the walking failure. Round8 reaches338ticks.
  Actual OffBoard jumping output is true at239..288 (50ticks), then false;
  walking resumes. The next stop is ground FeetIK Targets PendingState(3).
- The state3 target calculator ALREADY EXISTS and was used by the locomotion
  caller. Ground caller incorrectly restricted CalculateIKTargets to2/4.
  IDA UpdateGround82dd1e00 and UpdateLocomotion82dd1eb0 both invoke
  Update82dd1b70 -> UpdateState82dc89a0 -> CalculateIKTargets82dd0928.
  UpdateState prioritizes Processed+8cc bit40 (locomotion), even on the first
  ground tick. Added one shared target dispatcher used by both call sites,
  reusing existing state3 code (positive influence copies targets plus0 lanes,
  nonpositive preserves prior target; original debug singleton check).
  No duplicated foot-target algorithm. Round9 release/replay pending.

## Final launched scenarios (round9 release)

Release build PASS: five-controls-round9-build-20260925.log.

| Requested sequence | Ticks | Result/log in artifacts |
|---|---:|---|
| Y, forward, X jump, stop | 540 | PASS five-controls-offboard-jump-round9-20260925.log |
| Two pushes, full bottom-left | 540 | PASS five-controls-push-bottom-left-final-20260925.log |
| Spawn, explicit bail, A respawn | 540 | PASS five-controls-bail-respawn-final-20260925.log |
| Ordinary ollie, hold LS left through landing | 300 | PASS five-controls-ollie-left-final-20260925.log |
| Y, right stick left | 540 | PASS five-controls-offboard-right-left-final-20260925.log |

Offboard jumping output true239..288; finalMotion752/physics500. Powerslide
physics101 at162..236, then100. All scripted failures remain fail-closed, no
unknown-state skipping or success overrides. Final captures for offboard-jump
and push-bottom-left inspected/available in artifacts. This verifies these
input sequences, not exact numerical/visual parity or all possible controls.

Focused regression build initially found two test-double exhaustive matches
missing the new SlideSurfaceField variants. Added explicit rejection there
(the collision calculator must not read slide-orientation fields). Test-only
fix; the verified release code is unchanged. Validation results follow.

## Final verification complete

- All five requested live scenarios PASS on the same final release (see table).
- Final test build PASS, five-controls-final-test-build2-20260925.log.
- Generated Motion95, animation physics-output20, fixed-pose extraction3,
  offboard FeetIK11, power sliding4, collision21, original-asset bootstrap1:
  **155 passed, zero failures**. Collision filter leaves one external corpus
  test ignored; it is not counted as passed. Individual logs are
  five-controls-final-{motion,animation,fixed,feet,sliding,collision,assets}-tests-20260925.log.
- Only test-double coverage changed after the final release build. Production
  implementation matches the verified release. No background game/compiler
  remains. Existing build-release.bat remains usable; no recompilation is
  needed to try target/release/skate2-pc.exe --live-skating with the usual
  SKATE2_GAME_ROOT. Controller mode has no scripted completion limit.

Earlier pending notes above describe intermediate iterations and are superseded
by this final verification. Remaining project-wide boundaries (arbitrary graph
branches, exact numerical/visual parity, parked features) are not claimed done.
