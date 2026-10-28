# Ground control after WallRide

Source audit and **unregistered draft**, 2026-09-12. No Cargo or original
corpus validation was run by this author. Root owns integration and gates.
Confidence: high for source ordering and ordinary scalar operations; native
estimate exceptional behavior remains an explicit numerical policy boundary.

## Default first-frame branch

`PhysState_PhysicsGround::UpdateSkateboard82d9a1d8` reads the actual
`kApplyFakeTorque` target and bool. Its initializer82f8f0c8 stores **true**
(`li r7,1` at82f8f0f0). This branch cannot be bypassed as a debug-default-false
shortcut. It executes the native inline power graph for constant positive
base10, live signed integer exponent, then live float multiplier, makes the
full-known local vector `[+0,magnitude,+0,+0]`, and calls
`Skateboard::ApplyLocalTorqueToDeck82b35728` at82d9a42c. Only afterward does
the caller read Ground.doJump+e4 at82d9a430.

Reserved host slots (independent of native static-table positions):

| Slot | Reference | Initializer | Native table | Label / default / GUI range |
|---|---|---|---|---|
|125|830d8da4|82f8f0c8|82fd5720|kApplyFakeTorque / true|
|126|830d88d4|82f8f118|82fd5724|kFakeTorqueValue / c0c1999a / -9..9|
|127|830d8118|82f8f178|82fd5728|kFakeTorqueExponent / -1 / -10..5|
|128|830d7258|82f8ef78|82fd5710|kDotToStopPush / bf400000 / -1..0|

First three section `Physics_Hacks`, titles `Apply Fake Torque`,
`Fake Torque Value`, `Fake Torque Exponent`. Final section
`Physics_Skateboard`, title `Stop push DP`. All callbacks null, verified raw
r8 for bool and r10 for float/int. Native Dot registration precedes existing
DrawSBJoints5718, then Apply5720/Value5724/Exponent5728. No shared registry
change is included in this draft.

## Numerical and retained body proof

`skateboard_fake_torque.rs` preserves log estimate(a258), exponent target/value
(a26c/a288), exp estimate(a318), multiplier target/value(a354/a37c), reciprocal
estimate(a3e0). Log C0/C1 are820f0fb0/fa0; Exp C0/C1 are821ee8c0/b0. It preserves
the two cubic branches and fourth-power combination. In particular the
mantissa delta and binary exponent are multiplied by the live exponent BEFORE
the final log FMA; computing host `10.powf(exponent)` or multiplying a finished
log value would change the source graph. Both reciprocal refinements use
negative rounded FMA, and the source first-refinement NaN mask selects the
original seed. The exponent-zero select occurs after the full calculation.
The positive constant base eliminates only proven negative/zero-base cases.
No production log/exp estimate policy is silently chosen; the draft trait
requires these primitives explicitly. Tests use named host numerical choices.

`simulation_rigid_body_local_torque.rs` uses the SAME selected body's basis
right/up/at(+40/+50/+60) and cached WORLD inverse tensor full(+70), split(+80).
Local→world order is right*X, FMA up*Y, FMA at*Z. Tensor columns are
`[xx,xy,xz]`, `[xy,yy,yz]`, `[xz,yz,zz]`, preserving the same multiply/FMA
order. The source captures old torque XYZ, computes Z then Y, writes Y,
computes/writes X, writes Z, then cooldown+ac=0. It updates neither omega nor
the force queue. W is never routed into any output XYZ; source adjacent
pointer/tag/inverse-mass/state lanes need no fabricated values.

Proposed integration is exclusively these new modules plus a narrow backend
bridge delegating to its existing rigid-body pool:

- force_runtime child `fake_torque`: calculation and focused tests;
- input_cvar_runtime child `fake_torque`: current live refs125..127;
- rigid_body_pool child `local_torque`: actual mutation and non-axis/off-diagonal
  test, foreign-observation rejection, preserved force/omega;
- skater_pool child `local_torque`: actual board Part6 observation to backend;
- constraint_session child `ground_fake_torque`: consumes existing
  `GroundBeforeFakeTorque`, verifies SAME pair owner, retains its force/owner,
  returns `GroundBeforeJump` immediately beforea430. Disabled reads no body
  or tuning; enabled obtains the actual deck after the numerical calculation.

No new runtime/global owner is needed for this stage. Backend bridge proposed:
`try_apply_skateboard_local_torque(observation, RootVector)` delegates directly
to `self.rigid_bodies.try_apply_skateboard_local_torque`.

## Next collision branch and missing exact helper

doJump true at a430 uses actual velocity publication, drag zero,
Skeleton.GetLaunchInfo82b46098 and TrajectorySelector.Launch82dc4098 before
its Update, then overlay/exiting-wallride and exit. Existing constructor-idle
selector polling does not implement this active launch producer.

doJump false zeroes both force output vectors before CollisionForcea52c;
optional newVelocity is null. Existing live collision leaf and draft actual
Processed reader can be reused. A true result stores Ground+30 world force,
+40 local position, +70 torque, then fetches global collision+10c attribute
`d613692c606fe326`, stores timer+ac, then invokes TargetAngularDisplacementa5d8.
Those Ground retained arrays must become typed vectors to preserve unknown W.

`Skateboard::TargetAngularDisplacement82b37e98` delegates to
`SkateboardBody::TargetAngularDisplacement82b361e0`, not a simple angular
velocity replacement or direct torque addition. It reads actual Part6 omega,
scales by fixed kUpdateTime (0.016666668), normalizes the requested adjustment,
projects current angular motion onto that axis, and subtracts it from the
requested adjustment. Ordered negative dot(correction,request) selects full
zero. The second gate uses dot(projectedMotion,request); ordered negative
selects original request, otherwise selects the corrected/zero result. Fresh
tensor reads follow. The remaining source inverts the full symmetric cached
world inverse tensor by cofactors and a determinant reciprocal, divides the
selected adjustment by fixed DT, applies the inverse tensor, divides by DT
again, then applies the cached inverse tensor and adds to torque/cooldown.
These operations MUST NOT cancel algebraically. Exact VMX lane/reciprocal
graph remains the next necessary implementation slice; no replacement helper
has been admitted. Actual tensor and omega owners already exist.

After this helper, source sets Board530 bit80000000, queues kind15 collision
response, normalizes the collision force, loads current Processed190, dot3,
then reads live kDotToStopPush. Raw a648 BO4/BI24 is NOTLT, including NaN.
**a650 ALWAYS stores request_push_stop+f2**: ordered LT stores true; all other
cases store false. This corrects the earlier handoff's suggestion of a
conditional-only true write. The branch then exitsa998 without brake/push.

False collision route starts a658 with feet threshold binding/read, then
Brakinga6bc, Pusha6c8, Absorptiona6d0, Fronta6ec, Reara704, SpeedModela710,
Slidea720, Pumpa72c. These have existing leaves or completed unregistered
same-owner drafts. Straighten/Heading/AntiFlip/ManualEffect and nudge remain
later boundaries. Source force queue append occurs after these calculations,
with timer-driven angular displacement between push and later force entries.
The existing whole GroundAnimation force-tail order is inapplicable here.

Raw SHA256, half-open spans:

- a1d8..a430: `ccd05c9a13ab380aa6fa102d74860b9b3fbcc102bb309dc99c148a4d0a900bc3`
- 82b35728..357ec: `4d26b6a202c29769644fb8c9bc38ff4832968aaddc015a0ac00a3f1f7cb42f09`
- 82b361e0..3660c: `a2cbaff54ae803ab5cec801f269a6dcea2a9740a64e38c1fcbea13c8bacf618f`
- a4d8..a658: `d9fcc0f808cfd046f6401765ca7daf1168ba99f6bd8867fe7f28b1bfe5ae7584`
- 82f8f0c8..f1cc: `7a59c10b0e66fb0dd5e675b7ffd4a79b76bc17e5aa57ec4b12ef2d0b184eb4f1`

Independent root review: full raw82d9a1d8..a430 confirms log/exp polynomial ordering, exponent-before-final-log-FMA, late multiplier, first-refinement NaN select and final exponent-zero select. Raw initializers82f8f0c8..f1cc and82f8ef78..efd8 confirm all four live defaults/null callbacks and GUI ranges. Raw constructor82b38b50/54/58 proves reset-target flag clear;38bfc/38c00/38c18 proves abort-push clear. Root has not registered this batch yet.
