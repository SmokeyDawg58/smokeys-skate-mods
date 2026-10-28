# Ground queue and angular commit continuation

Read-only fresh raw audit82d9a7e8..a984 plus new, unregistered self-contained
caller prefix `physical_player_ground_queue_torque_prefix.rs`. No shared edits
or Cargo. Hang-up behavior is deliberately not implemented in this packet.

## Source ordering and retained arguments

Input is actual GroundBeforeForceQueues from Manual/drag/Nudge. Its preceding
Nudge may already have queued type17. All following AddSkateboardForce calls
copy the original full typed two-vector records, including local positions;
none reconstructs positions from current transforms or substitutes zero forces.

|Address|Operation|Condition / actual argument|
|---|---|---|
|a800|queue4 front foot|Ground+b0 time_wall_riding < retained foot_threshold|
|a810|queue5 rear foot|same branch, follows front|
|a820|queue2 brake|unconditional|
|a830|queue3 push|unconditional|
|a89c|TargetAngularDisplacement|positive collision_timer; scaled retained Ground+70|
|a8a8|TargetAngularDisplacement|original Straighten output|
|a8b4|TargetAngularDisplacement|original Heading output|
|a8c0|ApplyAngularDisplacementToDeck|original AntiFlip output, distinct missing callee|
|a8cc|ApplyAngularDisplacementToDeck|original ManualEffect angular vector|
|a8f0|queue6 conservation|Processed6b0/80000000 clear; fresh source read after angular effects|
|a900|queue7 manual|original ManualEffect.corrective_force|
|a910|queue1 slide|original SlideFriction output|
|a920|queue8 pump|original PumpForce output|
|a930|queue16 wallride|original earlier WallRide force record|
|a93c|SetLinearDrag|captured a77c result; actual Body all-Parts(-1) route|
|a97c|set board530/80000000|ordered squared net force > SMALL_FLOAT_163|
|a984|ManageHangUps|next missing state behavior boundary|

The first condition uses f30 loaded from the foot threshold attribute at a6b0.
It is NOT a comparison against zero or steering tilt: earlier f30 was reused.
a7f0 raw40980024 is NOTLT, including unordered, so NaN skips both feet.
All queue operations reuse existing SkateboardForceQueue capacity21 semantics:
copy existing record in insertion order, silently ignore excess entries.

## Collision damping prefix

a83c raw40990064 is NOTGT: only collision_timer>+0 enters, NaN skips.
Acquire actual global+10c Collision instance and attribute key1573c41add19aa95
at getter82d84690 (GetAttributePointer846b4, absent=>DefaultDataArea4).
Source caller then reloads Ground+ac timer a858 BEFORE pointer load a85c;
fresh Processed93c at a870, stores timer-DT a878, reads Ground+70 full torque,
multiplies every known lane by fade and stores full typed torque a898 BEFORE
calling actual TargetAngular at a89c. Missing late inputs retain prior queue
appends, and a later body/numeric failure retains timer/torque writes.

The draft uses existing retained runtime.slide_collision_attributes, its
capture_global/lookup/float contracts, existing Ground fields, same Processed
Arc and already reviewed pool.try_target_board_angular_displacement. After
conditional collision target it submits Straighten then Heading through the
same actual Part6/body occurrence and returns GroundBeforeApplyAngular.
No artificial completed-frame or inactive-state assumption is needed.

## Real blocking callee at a8c0

Outer Skateboard.ApplyAngularDisplacementToDeck82b37e90 dispatches to
SkateboardBody.ApplyAngularDisplacementToDeck82b36798. It is not the existing
TargetAngularDisplacement82b37e98/target body helper, nor ApplyLocalTorque,
nor Skeleton's translational Part displacement helper.

Fresh decompile and entry routing show actual deck Body's retained full inertia,
inverse/cofactor matrix arithmetic, reciprocal-estimate/refinement graph and
fixed1/60 scale, followed by additions into mTorque XYZ and mCool=0. The
decompiler loses stack scalar/vector values, so its apparent constant final
torque must NOT be transcribed. Raw numerical/body-owner reconstruction is
required before calling this method for AntiFlip and Manual, even when their
current vectors appear zero. No new caller beyond this point is drafted yet.

## Remaining self-contained suffix once ApplyAngular exists

After both angular calls, fresh Processed6b0 high bit gates conservation force6;
this read maps the existing Processed physics_state.ai_flags ownership and
requires actual known high bit. Then queue7/1/8/16 retains original local
positions. Existing pool.try_set_board_linear_drag / all-Parts Body helper
matches a93c and can be reused directly.

a950 computes `(captured push.world + brake.world) + pump.world` with two
separate full-vector rounded additions in this order. a95c dot3 of that
result with itself; a960 ordered GT against SMALL_FLOAT_163 at82341f10,
bits37800000. Success ORs board530/80000000; false/unordered leaves it intact.
This is the SAME retained Entry reset_target_after_collision currently written
by mark_collision_response at a5ec and read by SpeedModel, not a new force flag.
The method name is narrower than the physical flag's source uses; reuse the
owner/mutation rather than introduce duplicate storage.

Next calls are ManageHangUps82d8d720 at a984, ManageHalfpipeWheelCatches82d8da78
at a98c, ConsiderPinning82d8dc58 at a994. They are outside this audit's
implementation and prevent a claim of complete Ground.UpdateSkateboard.

## Integration status

Proposed sole new registration constraint_session::ground_queue_torque_prefix.
It requires preceding Manual/Nudge registrations and consumes their exact token.
No new attributes, cvars or retained fields are needed through a8b4. No tests
run; same-pair original corpus and targeted prefix-failure/queue-order tests
remain integration validation requirements. Source numerical policy remains
the existing reviewed TargetAngular collaborator, not host torque heuristics.

## Superseding ApplyAngular draft

The distinct callee is now recovered in new files documented by
`skateboard-apply-angular-displacement.md`. This unregistered caller draft now
extends through both ApplyAngular calls, source queue6/7/1/8/16, actual drag
and530 flag publication, ending at GroundBeforeManageHangUps beforea984.
Earlier missing-callee text above records the audit boundary and is superseded
by that source packet. No HangUps implementation or registered changes occurred.
