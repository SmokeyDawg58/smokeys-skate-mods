CORRECTION September25: the stationary-turn key below was recorded with
reversed32-bit halves. Raw PPC82da042c proves **9bf48a8551975cef**. The prior
key returned zero and prevented right-stick rotation. See
walking-fidelity-audit-2026-09-25.md for the live trace and correction.

# Reached OffBoard bad-ground correction

The mounted raw-Y frame 59 enters physical OffBoard 500 and completes
`OffBoardFeetIK::UpdateGround`. `PhysState_OffBoard::Update` at `0x82da03c8`
first loads FeetIK+`0x110` (height above ground) into f30, then calls
`CalculateBadGroundCorrection` at `0x82da03dc`, before
`Skateboard::UpdateSteeringTilt` at `0x82da03f0`. The live caller passes
exact positive zero to the existing retained board steering owner, reusing
the Ground control input adapter and steering implementation. It then stops
reads `Processed+0x8d0` at `0x82da03f8`, taking bit `0x00200000`. If set,
`fsel(-height, height, +0)` retains nonpositive height and otherwise selects
positive zero. `UpdateLocomotionViability` at `0x82d86f48` first clears
OffBoard `mAutoTurn` at `0x82d86f64`. Its next call,
`OffboardLocationTest::IsDataReady` at `0x82d86f6c`, remains a typed boundary.
The live prefix validates the source owner link and the retained
`OffboardLocation` world binding after the auto-turn store. The correction
and selected height remain carried in the boundary.

Further IDA inspection shows `IsDataReady` `0x82dd1fb8` is the byte at
LocationTest+`0x40`, the retained `OffboardLocation.sync_allowed` field.
Its constructor `0x82dd1f08` clears the byte at `0x82dd1f88`; `StartQuery`
`0x82dd48e8` also clears it at `0x82dd4c9c` after acquiring world line and
trajectory batches. `GetResults` `0x82dd4cc8` synchronizes both batches.
The retained fields establish the readiness read, but no query producer or
batch result has yet been connected to this live OffBoard path.

For the reached initial `IsDataReady=false` branch, `0x82d86fd4..e4` reads
Processed+`0x8d4` sign bit. If clear, `0x82d870c8` writes
`mIsLocomotionViable=false`; if set, it would build a location-test input and
call `StartQuery`, which remains typed. On the clear branch the enclosing
`OffBoard::Update` resumes at `0x82da0418..044c`, reads the first scalar of
attribute key `0x51975cef9bf48a85` from the retained OffBoard Vault instance,
and uses native default zero when absent. Rust reuses the existing
`remaining_states::read_floats::<1>` reader. It next reads the retained
Processed `mOB_Turn` at `0x82da046c`; if its magnitude is below source f32
`0x3c23d70a`, it adds retained `mAutoTurn`. After clearing `mGrabObject` at
`0x82da04cc`, it computes turn as attribute factor × f32 degrees-to-radians
`0x3c8efa35` × selected turn × f32 frame time `0x3c888889`.

The following linked cvars are registered once at indices 378–379 in their
initializer table order: `kDoCOMVelBlendOut` `0x82f92b58/0x82fd599c`
(default true), and `kFramesToBlendOutStartingComVel`
`0x82f92ba8/0x82fd59a0` (default 10, range 0..30). Native reads the frame
count, Processed state timer and bool gate; only when enabled and timer is
strictly below frames × frame time does it scale the full retained
`mStartingCOMVel` by the remaining-time fraction, using the already shared
reciprocal refinement. Native then overwrites its Y lane with positive zero
at `0x82da05bc`, keeping X, Z and typed W. Otherwise the outgoing adjustment
vector is full zero.
Before the next call, OffBoard+`0x74` and conditionally +`0x73` select the
`reorientUpright` argument. `Skeleton::UpdateOffBoard` at `0x82b5c3f0` first
sets the retained Skeleton `mUpdateHeading` byte at `0x82b5c428`, then calls
`SetOffBoardSkateRootTransform` at `0x82b5c430`. Rust reuses the existing
Skeleton root-transform owner for that first byte.
`SetOffBoardSkateRootTransform` at `0x82b4d198` first copies the full
animation-to-world translation and reads scalar key
`0x31bd1d86_86684e85` from the retained `physics_skeleton` Instance (native
four-byte zero fallback). It replaces only the translation Y with that scalar
plus the old Y, then tests Processed+`0x8d0` mask `0x8`. The clear branch
copies animation-to-world columns in source order 2, 0, 1 followed by the
modified translation into the existing `SkeletonGeneralUpdate.skate_root`
owner at Skeleton+`0x3c80`. The set branch reads the existing root half-length
and radius keys, then stops typed before its next-step position/cvar clamp.
The next clear-branch typed boundary is `UpdateRootTransformsOffBoard` at
`0x82b5c450`, carrying turn, selected height, correction, typed adjustment,
and reorientation flag. The original raw-Y replay in
`artifacts/raw-y-offboard-skate-root-2026-09-24.log` passes the clear branch
at frame59 and reaches this boundary. An independent IDA review approved the
read order, mask, columns, and retained destination.
The next live prefix snapshots the full animation-to-world translation before
the source `XMVectorSinCos` call, constructs the native yaw columns with X/Z
W lanes copied from cosine/sine, and conditionally runs the independently
reviewed upright reorientation leaf. The same skeleton root transform is
written only when that leaf returns a new matrix. The raw-Y replay in
`artifacts/raw-y-offboard-reorient-live-2026-09-24.log` passes this prefix at
frame59 and stops before `UpdateRootTransformsOffBoard`'s first matrix
assertion. Native SinCos/VMX exceptional-bit parity remains open.
The source's first two post-reorientation assertions are now connected in
their original order: retained animation-to-world XYZ, then constructor-owned
current trajectory XYZ. The W lanes are excluded. The fresh raw-Y replay in
`artifacts/raw-y-offboard-root-assertions-2026-09-24.log` passes both at frame59
and stops before the transform composition at `0x82b44cc8`. The source's
DebugBreak behavior is represented by distinct typed errors on assertion
failure; failing-path later-column reads are not reproduced.

`UpdateRootTransformsOffBoard` then composes the constructor-owned current
trajectory with the source yaw (`0x82b44cc8..0x82b44d50`), followed by the
possibly reoriented retained animation-to-world matrix with that intermediate
(`0x82b44d54..0x82b44db0`). The existing full-lane affine composition helper
uses X multiply, Y FMA, Z FMA, including translation; unknown W remains typed.
The existing Skeleton root owner receives all four resulting columns at
`0x82b44db8..0x82b44dc4`. An independent IDA review confirmed matrix order,
owner, lane handling, and the next boundary before the XYZ assertion at
`0x82b44dc8`. Native stores are ordered columns 0, 3, 1, 2; the Rust owner
publishes a completed matrix in one assignment, with no intervening native
call. `artifacts/raw-y-offboard-root-composition-2026-09-24.log` passes the
reached frame59 path and stops at `PendingOffboardRootSecondAssertion`.

The composed root's second XYZ-only assertion at `0x82b44dc8..0x82b44f48`
passes on the same frame. The existing typed `Math::OrthoNormalizeRotation`
helper runs the native reverse-axis Gram-Schmidt order and preserves the full
translation and unknown W lanes. Its four resulting columns are written back
to the same Skeleton root owner at `0x82b44f5c..8c`. Independent IDA review
found no duplicate normalizer or owner. The fresh
`artifacts/raw-y-offboard-normalized-root-2026-09-24.log` stops before the
HookToHips displacement block; its `kMaxSkeletonDisp` cvar is distinct from
the already linked OffBoard `kOB_MaxSkeletonDisp` cvar.

The HookToHips block now reads the retained post-physics hook and computes
its XYZ squared length, captures `Physics_Skeleton/kMaxSkeletonDisp` before
the full current-minus-prior translation delta, and reads the cvar between
the seed square and source-ordered reciprocal-square-root refinements. The
separate `kCollisionPushFraction` is registered for the correction branch.
Independent IDA review checked the two cvar identities, reads, arithmetic
order, and strict length gate. The original raw-Y frame59 hook length is
zero, so `artifacts/raw-y-offboard-hook-length-2026-09-24.log` reaches
`PendingOffboardRootComBlend`; the correction branch and COM blend remain
explicit boundaries. Native VMX seed and exceptional-bit parity are open.
The next source gate scales the full second Update argument by the fixed
`kUpdateTime_55` bits `0x3c888889`, squares its XYZ lanes, and takes the
nonzero branch only above `Math::VECFLOAT_SMALL` (`0x37800000`). Independent
IDA review confirmed the branch's actual VMX compare source and found no
duplicate gate. The original raw-Y frame59 replay in
`artifacts/raw-y-offboard-com-gate-2026-09-24.log` takes the nonzero branch
with step `[-0.029968264, 0, -0.0015258789]` and squared length
`0.00090042513`. The following COM projection and possible root-translation
correction use the retained animation COM, Processed COM, and existing root
matrix. The same shared per-axis transform stage preserves IDA's interleaved
VMADD/refinement order; a strict length-versus-projection check writes only
the root translation column. The fresh
`artifacts/raw-y-offboard-com-projection-2026-09-24.log` reaches that write:
length `0.030007083` exceeds projection `0.0033517217`. Independent IDA review
confirmed the owner and ordered graph; host reciprocal estimates and FPSCR
exception behavior remain open. The next join resets current trajectory and
global/local pose matrix zero on their existing owners; its mounted replay is
still pending.

IDA `CalculateBadGroundCorrection` `0x82d870d8..0x82d87290` first copies the
full `OffBoardFeetIK.mBadGroundCorrection`, then reads `Skeleton::GetHookToHips`
and the translation column of `Skeleton.mTrajectoryTransform`. It clears the
retained valid-response flag if trajectory XYZ squared is strictly greater
than `kOB_MinTrajW` squared. It next checks hook-to-hips XYZ squared against
`kOB_MaxSkeletonDisp` squared; a strict exceedance normalizes the full hook
vector with `Math::NormalizeSafe` and stores it as the valid response direction.
If the direction is valid and its XYZ dot with the original correction is
negative, the late `kOB_DebugCollResponseA` read selects either correction
minus direction times dot or a full zero vector. Otherwise the original
correction is returned. Unknown W remains typed through all vector operations.

The three linked cvars are registered once at indices 375–377, using IDA
initializer/table pairs `0x82f92d20/0x82fd59b0` (max displacement,
`0x3c23d70a`), `0x82f92d80/0x82fd59b4` (min trajectory,
`0x3ba3d70a`), and `0x82f92de0/0x82fd59b8` (debug switch, false).
The live adapter reuses the retained FeetIK, Skeleton post-physics records,
Skeleton construction trajectory, and published OffBoard state. Focused tests
check source read order, trajectory invalidation, late debug access, and
unknown W on the projection result. Ordinary finite arithmetic follows the
host numeric policy; exceptional VMX comparisons remain unverified.

The live OffBoard join now resets the retained current trajectory and
`AnimOutPhysIn` global/local pose matrices at index zero in native order.
It samples distinct `Physics_States_OffBoard/kOB_GroundMaxDelta` and
`kOB_GroundPropBlend` cvars, applies the source `fsel` clamp graph to the
root translation, and stores only the P column. The clamp test checks
negative zero and NaN selection. IDA review confirmed the pose owners,
cvar registrations, store order, and no duplicate implementation.
`artifacts/raw-y-offboard-ground-clamp-2026-09-24.log` reaches the next
full-matrix assertion with finite XYZ and a vertical negative zero.

The path now repeats all four XYZ-only root checks, computes the existing
rigid inverse, and stores world-to-animation columns in native order
2, 1, 0, 3. The `Skeleton::UpdateOffBoard` caller repeats its XYZ-only
assertion before the animation-volume pose-buffer read. Independent IDA
review found the right retained owner and result; the Rust helper finishes
inverse arithmetic before its ordered stores, while native interleaves
some basis stores with the translation arithmetic. The original-data
frame-59 replay `artifacts/raw-y-offboard-inverse-2026-09-24.log` passes
these steps and stops at `PendingOffboardAnimVolumeRootComposition`.

The next same-owner composition reads mapped animation-volume row zero and
multiplies current animation-to-world by that full row. The result passes
all four XYZ-only assertion columns; Processed receives the full resulting
orientation before its animation-valid bit is set. The subsequent single
Processed+8d0 read tests mask `0x0004_0000`: the set path would call
`ApplyBoardAnimation(true)`, while the clear path enters `GeneralUpdate`.
Independent IDA reviews confirmed the order, owners, W propagation, and
gate. The current original-data replay
`artifacts/raw-y-offboard-post-orientation-gate-2026-09-24.log` takes the
clear branch on frame 59 and stops at the `GeneralUpdate` call. The shared
composition helper groups arithmetic per output column whereas native
interleaves stages across columns; ordinary finite values match, but FPSCR
timing has not been measured.

The clear branch now enters the existing `GeneralUpdate` prefix and
completion on the same attached Skeleton pair, retaining the current
Processed owner and current mapped animation pose. The seam supplies the
already bound World query, ground, cvar, numeric and hook owners. It does
not rerun the grounded or animated root setup. Independent IDA review
confirmed the call order and found no duplicate owner. The fresh
frame-59 replay `artifacts/raw-y-offboard-general-entry-2026-09-24.log`
passes the entire shared GeneralUpdate and stops before the caller's
`NextTrajectory` identity reset.

The caller now resets that retained `NextTrajectory` to identity using
the existing [0,3,1,2] column-store method after `GeneralUpdate` returns.
Independent IDA review confirmed the same matrix owner, order, and
no premature store on failure. The original-data frame-59 replay
`artifacts/raw-y-offboard-nexttrajectory-2026-09-24.log` passes the reset
and stops before `Reckoning::UpdateGrindStates`. Its source call uses the
same retained Reckoning owner, world-up grind normal, live animation root
Z heading, 0.5 smoothing, and the selected true body-spin argument.

`Reckoning::UpdateGrindStates` now runs on that retained owner. Its source
review caught and corrected the blend operand: native uses the caller's raw
normal even though it also updates the retained filtered ground normal.
The remaining dynamic-up, filter, transform, tilt, and grounded BodySpin
stores follow the checked IDA order. The frame-59 replay
`artifacts/raw-y-offboard-grind-2026-09-24.log` passes this update and
stops at `PhysState_OffBoard::Update`'s post-Skeleton suffix. Numeric
bit parity for Xenon VMX estimates remains unmeasured.

The post-Skeleton OffBoard suffix now rereads Processed+8d0, gates the
grab query on its low bit, bit 20, and the retained +0xa20 repush delay,
then checks Processed+8cc bit 0. On the clear grab branch it reuses the
existing PlayerGrabSpline invalidation and clears interactable readiness
in source order; the set branch remains typed before the unimplemented
grabbable-object query. An independent IDA review confirmed the gate
and owner order. The frame-59 replay
`artifacts/raw-y-offboard-suffix-2026-09-24.log` passes this suffix and
stops before `UpdateJumpingPreperation`.

The reached Processed+8d0 bit-1-clear branch returns immediately from
`UpdateJumpingPreperation` and OffBoard.Update. The existing Reckoning
completion phase is passed through the same-owner Update3 controller and
prephysics continuation, without rerunning the OffBoard work. Independent
IDA review approved this return and reuse. The frame-59 replay
`artifacts/raw-y-offboard-return-2026-09-24.log` confirms the branch
returns and reaches the controller's explicit active `UpdateState`
boundary; the controller update remains unmounted.

The reached controller state-1 `UpdateState` now snapshots the retained
board Body+0x364 contact flag and executes source-reviewed `UpdateGrip`.
It reads Processed deck translation before the selected forearm part
transform, then COM, runs the four existing NormalizeSafe calls, reads
the retained controller attribute, evaluates the existing cosine leaf,
and writes the board's existing grip cosine, direction, and enable flag
in native order. Independent IDA review found no second grip owner or
material source-order difference; Xenon estimate/rounding parity is
still unmeasured. The frame-59 replay
`artifacts/raw-y-controller-update-grip-2026-09-24.log` passes these
stores and stops before the post-grip controller state gates, with the
captured Body flag clear.

The post-grip gate now reads Processed+8cc bit14 before conditionally
reading +8d0 bit21, then runs the existing Controller `UpdateDriveFrames`
once when state 1 remains active. Independent IDA review approved the
source order. `artifacts/raw-y-controller-drive-2026-09-24.log` passes
pre-solve and Board.PostPhysics, reaching the held-deck post-solve branch.
Disassembly of HiLOD.Update4 0x82e01d04..1d24 corrected an older note:
the conditional +0x360 byte is `SkateboardBody` contact count, not Actor
input. The production schedule now reads the already-published `num_parts`
only when `IsDeckInHand` is true and passes its nonzero value to
Skeleton.PostPhysics. `artifacts/raw-y-postsolve-contact-2026-09-24.log`
passes this branch and reaches OffBoard state post-physics.

The selected OffBoard post-physics arm now reads the native outer and
inner enable cvars (defaults true), sets the retained Wipeout state to 4,
reads Skeleton+0x3da0 animation-world velocity, then obtains the first
`physics_wipeout/default` threshold with native hash
`0x0ae6_fd15_3560_f8c1`. It reads the display cvar before the speed
comparison, preserving a typed stop for the debug display and high-speed
collision paths. On the low-speed return, it reads the retained FeetIK
request byte and OffBoard entry flags in source order and uses the existing
Wipeout ledger for request types 25, 23, and 28. All three new cvars use
source initializer defaults and table-order registration. Independent IDA
review found no reached-path mismatch; Xenon VMX dot-product parity is
still unmeasured. `artifacts/raw-y-offboard-postphysics-2026-09-24.log`
proves the actual frame-59 speed exceeds the first threshold, so the
high-speed collision checks were the next source-gated seam.

Those high-speed checks now reuse the existing Skeleton squash,
displacement, and contact-force leaves through their retained Ground
runtime adapter, while the distinct OffBoard vehicle contact threshold
uses its own source hash. The source-ordered request types 7, 18, 1, and
0 reuse the same Wipeout ledger. Independent IDA review found no reached
owner or ordering difference. `artifacts/raw-y-offboard-highspeed-2026-09-24.log`
passes this callee and stops at Update5's missing OffBoard GetType.

The selected GetType now returns native 500 from the published OffBoard
state and validates its original pair bindings. The subsequent
`PhysState_OffBoard::FillPhysOut` copies grabbing, locomotion viability,
the two-word grab descriptor, then FeetIK left/right foot-down flags.
Each of the five destination writes samples the native write latch before
its source read. Independent IDA review found no duplicate Fill writer or
reached-path mismatch. `artifacts/raw-y-offboard-fill-2026-09-24.log`
completes the full selected frame 59 with `error=None`. The same test
harness can optionally continue after the first OffBoard frame by setting
`SKATE2_Y_FOLLOWUP_FRAMES` for a bounded original-owner follow-up probe.

The ten-frame follow-up first stopped before frame-60 physics at Action
`StateUpdate_Default_30` (`0x828ed4b0`). That dispatch now checks the
generated `0..37,101..103` active behaviours and calls their existing
handlers in native slot order 0..40. An independent IDA review confirmed
the callback order and reuse. The next replay reached the source physical
selector's previously unmounted OffBoard500 case. That case now adapts the
existing `OffBoardStateInput` decision tree to retained Processed flags,
checking the `+0x8d4` known bits only when the original branch reaches them.
Fresh IDA and independent code review confirmed the native priority and
state IDs. Frame 60 selects WipeoutGround300, then stops at the explicit
`PendingTransition(300)` boundary; `artifacts/raw-y-offboard500-selector-2026-09-24.log`.
The diagnostic replay records Motion conditions 2 and 3 false and condition
4 (`PhysicsWantsWipeOut`) true on frame 60, selecting WipeOut/Flailing99.
It publishes the authored wipeout attribute; the retained Processed
`+0x8c4=0x43000` sets `0x40000`, so the physical selector returns 300 at
its first OffBoard branch, independent of the `OB_Traj` bit. The existing
transition runner now admits only OffBoard500-to-WipeoutGround300 and guards
the native OffBoard.Exit path: it checks the published OffBoard state and
its location/jumping links, clears location sync readiness, and explicitly
stops on non-null batches until their native Sync/Release or results path
is implemented. Null-batch Exit reuses the existing WipeoutGround Enter.
The next mounted boundary occurs earlier: StopController reaches its
`LetGoOfSkateboard` call before transition fields. Evidence:
`artifacts/raw-y-motion-gate-2026-09-24.log` and
`artifacts/raw-y-offboard-exit-boundary-2026-09-24.log`.

The reached state-1 StopController/LetGo branch now invokes the retained
controller while its state and system remain live, then clears both after
the callback. Its ordered services are DisableBoardAnimation, SetFree,
the caller's second collision-volume enable, board grip clear, and the
hand-drive disable when hand is not 2. The state-4 velocity and throw-bit
branches remain explicit boundaries. An independent IDA review checked
LetGo `0x82dd03e8`, StopController, and the existing service owners; the
second volume enable is a distinct native call. Actor SetPhysicsState's
event hook then follows the debug image's uninitialized TelemetryHelper
return. A new original-data raw-Y replay completes all of frame 60 with
`error=None`, WipeoutGround300, Action30, and Motion99. Frame 61 first
stops at Action `PendingExpression(73)`, now under source review.
Evidence: `artifacts/raw-y-telemetry-gate-2026-09-24.log`.

Action expression 73 is the `Running` target Pre check from the state-30
transition search, not an expression owned by state 30. IDA Pre
`0x82902dcc` and Sustain `0x82908144` both read cached condition 134
(`PhysFilteredState`), then condition 141 (`IsInLocomotion`) only when 134
is true. Condition 141 at `0x82c98e78` resolves the Action-bound physical
owner and compares raw `PhysOut.State+0x10` with 504; it does not use the
similarly named animation flag. The mounted expression uses the existing
condition cache and a shared raw-state query also used by `IsLandingOnBoard`'s
state-505 comparison. Two independent IDA traces and a post-edit review
confirmed call order and owner reuse. Raw-Y frame 61 advances to
`PendingExpression(72)`; the full core suite passes 4,833/0, and the PC
app check passes. Evidence:
`artifacts/raw-y-action-expression73-2026-09-24.log`.

The following candidate is `TransToOnBoard` state 10, expression 72.
Native Pre `0x82902ba4` evaluates `(c134 || c135) && (c70 || c136)
&& !c127 && !c130`, in left-to-right lazy order; Sustain `0x82908020`
uses `(c134 || c135) && c136`. Both independent traces found all six
conditions already dispatched through existing retained owners. The two
expression arms reuse that cache and have a focused lazy-order test; the
independent post-edit review found no discrepancy. Raw-Y frame 61 advances
to `PendingExpression(75)`. The core suite passes 4,834/0 and PC check
passes; see `artifacts/raw-y-action-expression72-2026-09-24.log`.

Action expression 75 belongs to the OBAir Default candidate. Native Pre
`0x82902f64` and Sustain `0x82908270` both evaluate `c135 && c145`,
skipping `IsDoneAirDismount` condition 145 when filtered OffBoardAir
condition 135 is false. Two independent IDA traces and a post-edit review
confirmed this narrow arm. The missing condition-145 callback remains typed
pending when actually reached. In the raw-Y WipeoutGround frame, condition
135 rejects that candidate; existing OffBoard parent transition 110 then
selects Action Wipeout23 via condition 126. The original-data replay
completes frames 61–113 and reaches frame 114 Motion state30
`PlayWipeoutRecoverAnim::Begin` with `MissingOwner("SkaterManager recovery
graph")`. Two independent IDA reviews showed that the Begin code and graph
builder already exist: the direct corpus constructor bypasses the live
facade's push/recovery graph installation. The current work is to mount
those existing owners on this corpus Actor. Evidence:
`artifacts/raw-y-action-expression75-2026-09-24.log`; full core suite
4,835/0 and PC app check passed.

The direct queued corpus now uses its retained `Skater` playback and animation
attribute pool to build the existing push attributes followed by the existing
wipeout recovery graph, then attaches both to the same Actor before any tick.
The production selected facade already performs those operations; this is
fixture orchestration, not a second graph or recovery implementation.
Independent IDA and code review confirmed the source order and owner identity.
The test-only `SKATE2_Y_FOLLOWUP_FRAMES` limit now counts from the first
completed OffBoard frame even after transition to Wipeout, retaining the
default zero-frame stop. With 60 follow-up frames, original-data raw-Y
completes frame 119 with `error=None`, including recovery Begin at frame 114:
`artifacts/raw-y-recovery-followup60-2026-09-24.log`. Core 4,835/0 and
PC app check pass. This scripted result still does not prove a continuous
controller-playable loop.

The same test-only Y budget can now extend beyond the prior hard 120-frame
queued cap without changing non-Y tests. A 240-frame follow-up from first
OffBoard frame 59 completes through frame 299 with `error=None`; however,
the retained physical state is still WipeoutGround300, Action23, Motion30.
This proves sustained execution of the Y-to-wipeout path, not physical
recovery. The separate explicit-bail public facade returns Ground by frame
299, so the divergence is being traced rather than treated as equivalent.
Evidence: `artifacts/raw-y-recovery-followup240-2026-09-24.log`.

The apparent frame-299 delay follows the source minimum auto-recovery
duration plus the separate fade countdown: Y enters WipeoutGround at frame
60, whereas the explicit-bail comparison enters at step 8. Two independent
IDA reviews checked these gates and found no Y-specific prevent-respawn or
impalement bit in the logged Processed flags. A 300-frame follow-up resolves
the timing question: raw Y remains WipeoutGround through frame 303,
transitions to Teleporting at frame 304, returns to Ground at frame 306,
and completes frame 359 with `error=None` and Ground. There is a later
brief Air interval after teleport before frame 359, all with successful
selected ticks. This establishes the full scripted Y/offboard/bail/recovery
loop on the direct original-data fixture, not a live controller or camera
proof. Evidence: `artifacts/raw-y-recovery-followup300-2026-09-24.log`.

The release PC client now has an explicitly scripted `--live-y-demo` that
feeds the same bit-15 FaceY sample at selected frame index 32. It pairs the
synthetic state with the corpus's type-1/subtype-0/flags-0 capabilities;
without a demo flag, both state and capabilities still come from XInput.
The diagnostic state trace reads the same completed Actor's Action/Motion
controllers and raw PhysOut state, releasing the Actor prefix lock before
other owner locks. Independent post-edit review and a release five-frame
run checked this after an initial diagnostic-only reentrant-lock hang.
The live camera/render window then completed 360 Y-demo frames with a
state sequence matching the direct fixture: 500 at frame 60, 300 at 61,
702 at 304, 100 at 306, with normal on-board Action/Motion by 307.
`artifacts/live-y-demo-120-2026-09-24.png` shows the bail and
`artifacts/live-y-demo-360-2026-09-24.png` shows the skater back on board.
This closes the scripted live visual loop, not human controller input or
native tick-by-tick numeric parity.
