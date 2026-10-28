# Ordinary Ground force composition after collision

2026-09-12: fresh raw82d9a658..a998 and current production-owner audit.
This packet is an implementation plan, not an integrated force schedule.
The actual original corpus now reaches BeforeBraking on its real collision result.

## First consecutive composition

Preserve the existing GroundBeforeFakeTorque owner and wallride force carried
by GroundAfterCollision::BeforeBraking. Do not reconstruct a fresh owner or use
the differently ordered GroundAnimation force tail.

1. a658..a6b0: actual global feet+bc key21b212a085d31d3b, null-only4-byte
   DefaultDataArea, retain loaded f30 threshold for a7ec after all calculations.
2. a6bc: existing SkateboardBrakeDragServices.try_braking.
3. a6c8: existing drafted PushRuntime/try_calculate; actual retained entry owns
   the abort flag and receives any prefix mutation before later errors.
4. a6d0: existing drafted try_absorption with SAME runtime.pumping Arc and
   actual owner.pumping_attributes. Retain the resulting scalar.
5. a6e4: fresh Processed9c0; a6ec: existing front-foot producer.
6. a700: fresh Processed9c0 again; a704: existing rear-foot producer using SAME
   previously calculated absorption. No snapshot shared between the reads.
7. a710: SAME entry.try_speed_model_prefix then try_speed_model_update.
   Preserve target/manual torque/reset-flag writes on later errors.
8. a71c: fresh Ground+a8 state timer; a720: existing try_slide.
9. a72c: existing drafted try_pump using SAME Pumping owner.

The caller must retain all seven force records (brake, push, front, rear, speed,
slide, pump), the earlier wallride record and threshold before proceeding to
the torque calculations. A force record is merely local output until the
specific later AddSkateboardForce call. No early queue publication is allowed.

## Existing owner seams and narrow integration needs

`try_with_ground_control_board` already yields SAME runtime and SAME
SkateboardBodyConstruction.attributes_mut(), so SpeedModel manual attributes
must reuse that argument. No second SkateboardAttributes binding is needed.
Its source additionally needs the actual board session for current effective
transform and body mass. Add a sibling new child callback exposing a borrowed
candidate.session with those two disjoint mutable owners; avoid per-call
transform precomputation before its source read.

The existing runtime needs the already-drafted once-bound PushAttributes and
ConservationAttributes capabilities. Existing wallride/feet/brake/friction and
body Manual bindings remain their actual current owners. Pumping already uses
the real owner Arc and owner.pumping_attributes mutex.

GroundForceInput currently implements only brake/collision readers. Actual
FootInputRead implementations exist solely in controlled corpora: add a new
live child on SAME GroundForceInput for balance9a0, surfaceSpeed96c, bodyY180,
velocity190 and SAME Reckoning up1b0, with source-late individual reads.
Fresh raw confirmed helper2892c surface96c and28a6c/84 bodyY180; up1b0
comes from actual board Reckoning pointer104 at28ad8/28aec, not Processed up220.

The feet helper captures the absorption debug target before its projection
dot, then reads the captured target later. Existing PumpingForceCvars immediate
boolean119 is insufficient for that interface. A narrow new cvar child should
implement FootDebugRead with a cloned real target119 and the live overlay slot;
it must preserve rebind-versus-captured-target behavior, not prefetch a bool.

Root identified SpeedModel's NormalizeSafe call needing the source VNMSUB
refinement callback now provided by shared numeric code. Apply that correction
when the leaf registers, preserving its SAME evaluator and output sequencing.

## Remaining mandatory tail

No existing implementation found by source-symbol search for the following
callees: StraightenOutTorque(a73c), HeadingTorque(a748), AntiFlipTorque(a754),
CalcManualEffect(false,a774), UpdateAntiFlipNudge(a7e4), ManageHangUps(a984),
ManageHalfpipeWheelCatches(a98c), ConsiderPinning(a994).

AntiFlipTorque output is stored full-vector to SAME Ground+90 at a770, BEFORE
ManualEffect. Its existing anti_flip_torque field must retain typed W and be
coordinated with the graph agent's Ground State.Fill consumer.

LinearDrag(a77c) is an existing leaf but its live input currently requires the
distinct Processed push9a4 producer, not animation push speed940. The later
brake+push+ pump calculations and thresholds require raw VMX128 operand review.

Actual publication order, after all above calculations and nudge:

- a7ec threshold branch: conditional force4 front, then5 rear.
- Always force2 brake then3 push.
- Positive collision timer: late global collision fade, fresh DT, subtract/store
  timer, full retained collision torque scaling/store, TargetAngularDisplacement.
- Straighten TargetAngularDisplacement, then heading TargetAngularDisplacement.
- AntiFlip ApplyAngularDisplacementToDeck, then Manual angular displacement.
- Conditional speed force6 using actual Processed6b0 high bit.
- Force7 manual contribution, force1 slide, force8 pump, force16 wallride.
- SetLinearDrag, then actual squared net brake/push/pump force gate sets SAME
  entry530/80000000 without an unconditional clear.
- ManageHangUps, ManageHalfpipeWheelCatches, ConsiderPinning, native return.

The new TargetAngular backend bridge is reusable. ApplyAngularDisplacementToDeck
is a distinct actual body mutation and cannot be replaced by TargetAngular or
a force-queue record. Queue insertion does not imply solver execution.

## New unregistered caller draft

`physical_player_ground_ordinary_forces.rs` now consumes the actual
GroundAfterCollision variant. CollisionExit refuses this route. The ordinary
variant validates SAME pair/Ground, then executes all nine steps through pump
using existing algorithms and actual bound owners. Its
GroundBeforeOrdinaryTorques retains the original owner/wallride force, threshold
and all seven outputs. No queue entry is created. Earlier Push/SpeedModel owner
mutations remain if a later source call fails. Scratch is explicit write-only
unknown-W backing, never a claimed native initialization or output on failure.

New `physical_player_ground_force_board.rs` exposes disjoint borrows of SAME
board session, runtime and construction attributes. New
`skeleton_processed_ground_foot_input.rs` implements actual late feet reads as
a child of existing GroundForceInput. New `input_cvar_ground_foot_force.rs`
implements captured119/late-read/actual-overlay behavior and a focused test of
mutation, reference relinking and late actual singleton construction.

Required shared amendments before compilation (NOT applied here):

- Register those children, existing push_source/pumping_force_source and
  speed_model_input/source drafts in their stated parent modules.
- Runtime retains push_attributes: PushAttributes and
  conservation_attributes: ConservationAttributes, once bound during selected
  admission. Entry gets its narrow existing abort flag setter.
- Existing feet front/rear methods gain source-refinement siblings forwarding
  one callback to the SAME helper, with old wrappers retaining old policy.
- Existing try_pump and entry.try_speed_model_update similarly gain
  *_with_refinement siblings through their SAME numerical body. Caller passes
  explicit final-negated-FMA Newton refinement. Raw feet28af4 and pump290b4
  both call actual Math.NormalizeSafe; raw824b1988=110c6aaf and
  824b1998=10ab69ef are VNMSUB. No broad drive numerical change.
- New caller currently targets these proposed exact method names:
  try_front_with_refinement(time,absorption,out,refine),
  try_rear_with_refinement(time,absorption,out,refine),
  try_pump_with_refinement(source,numeric,out,refine),
  try_speed_model_update_with_refinement(prefix,source,numeric,out,refine).

No Cargo or tests run by this agent. Root must compile after shared amendments,
run the focused target test and original SAME Actor continuation, confirm the
queue count/contents remain unchanged through this stage, and independently
review source timing before mounting the remaining torque stages.

Root integration review: fresh raw a6b0..a730 matches all force calls and two separate9c0 reads. Push27384..274fc matches clamp comparisons/late pointer load/mass/DT/vector order. Pump28ef0..290c0 matches flags/acceleration, actual f8 binding keys, disabled normalization and output prefix. Feet2892c reads96c and28a84 reads180; Reckoning104+1b0 at28ad8/aec verified. Root wired VNMSUB-specific callback through same evaluators; legacy wrappers retain previous refinement. Independent graph reviewer found and verified correction of host Reckoning double lock: end guard afterrear, fresh guard beforeSlide. Original corpus now checks unchanged force queue; validation pendingworkspace50.

Acceptance: workspace50=4020units+6docs, original44=17/17, Clippy81/fmt37/diff37PASS. Original mounted neutralGround reaches all seven force records and confirms identical queue records before/after. Actual feet threshold1.5; neutralforces retain zero signs and unknownW as produced. No source-positive force/complete tick parity claim.
