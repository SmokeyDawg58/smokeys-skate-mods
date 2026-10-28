# Ground control: retained speed wobble calculation

Status: source-reviewed draft, unregistered and untested. No Cargo run by this
agent. Confidence high for source order; host arithmetic/rounding is not an Xbox
bit-trace claim. Grinds and dynamic spawning are outside this slice.

## Real caller and owner

`PhysState_PhysicsGround::UpdateSkateboard82d9a018` calls existing board
PhysicsError at a06c, captures truck damping at a074, and calls
`Toolkit_CalcSteeringTilt` at a088 with the SAME Ground push scalar/damped turn.
At a098 it obtains Skeleton COM-to-deck from actual current-animation record
2a30 (not outer remainder3cc0), computes its XYZ length, then calls
`Skateboard::UpdateSpeedWobble82b30770` at a108.

The wrapper captures Processed+110, loads actual board wobble owner+10c, then
reads DT93c at3077c, truck tightness9c4 at30780, and ZSpeed944 at30784.
Caller tilt/length are carried into the complete `CalculateSpeedWobble82b2eb68`.
Existing `skateboard_speed_wobble.rs` owns this constructor-created state and its
class5be46545de485c98/defaultd7edbd362d7d2152 attribute instance. Reuse it; no
per-call binding, duplicate phase, invented amplitude, or assumed inactive branch.

## Attribute and mutation order

Every scalar uses `GetAttributePointer(key,0)` and native null fallback4 bytes.
Capture and dereference are deliberately separate. Key sequence:

| Lookup site | Key | Consumption |
|---|---|---|
| ebb4 | 804b14d3f9a140bb | first COM minimum, loaded ec38 |
| ebe8 | 19569b9f9dc7f291 | COM maximum, loaded ec40 |
| ec1c | 804b14d3f9a140bb | second independent minimum, loaded ec30 |
| ec88 | f702ddd8a4c78a7e | truck coefficient, loaded ed04 |
| ecbc | ed444c4e521926a0 | base threshold, loaded ed14 |
| ecf0 | 53e875b961b7f0d0 | COM coefficient, loaded ed10 |
| ed78 | 3959677ce772299e | active speed range, loaded ed90 |
| edc4 | 0f494df5beb3f126 | active time range, loaded edd8 |
| ee10 | fb3997a35341210c | frequency scale, loaded ee5c AFTER two graphs |
| eec0 | 74ac73d32ac5c98c | amplitude scale, loaded eedc AFTER effect store |

COM fraction is fsel-clamp of `1-(length-firstMinimum)/(maximum-secondMinimum)`.
Scalar clamp returns1 for NaN and +0 for either zero. Threshold first rounds
truckCoefficient*truck, then FMA COMCoefficient*COMFraction+truckProduct, then
adds base. Captured active byte ed08 precedes late scalar loads. Activation only
occurs for ordered speed>threshold (ed2c BO4 BI25 skips unordered), storing
time/frequency/lastTilt=0, active=true, angle/effect/wobbleTilt=0 in that order.

Active speed/time fractions each use their live scalar divisions and fsel clamp.
Graphs use the same cached 0x100 layout in order40(speed),0(time),c0(speed),
80(time). Frequency is `(scale*timeGraph)*speedGraph`, a LOCAL; retained
frequency+18 is not assigned that value. Effect=timeEffect*speedEffect is stored
eea0 BEFORE amplitude lookup. Angle=`(frequency*dt).mul_add(TWO_PI, oldAngle)`
is stored ef14 BEFORE sine. Native ef50 stores the sine*amplitude OFFSET f0,
not combined f7; returned tilt=(offset+callerTilt)*0.5.

Late debug cvar can require the actual DebugOverlay singleton; enabled+present
performs no draw call. Failure retains effect/angle/offset, before time increment.
Time increments ef9c. Ordered speed<threshold then clears time, active,
frequency, lastTilt, angle, effect, wobbleTilt; efa4 BO4 BI24 skips reset for NaN.
The already computed steering result is returned even after that reset.
Amplitude member+14 is never touched, including reset.

## Sine and cvar proof

`XMVectorSin824b0060` uses existing g_XMPiConstants0 at823cfea0 and sine tables
823cfe10/20/30. Word10000fef at824b00cc is standard VNMSUB, giving
`-TWO_PI.mul_add(nearestTurns,-angle)`. Native `vrfin` is a supplied numerical
service. Unlike the existing SinCos source, this function constructs x3=x2*x,
then x5=x3*x2, x7=x5*x2, continuing sequentially through x23. Its eleven
ascending FMAs reuse the existing coefficient constants in a separate child
`reckoning_sine.rs`; substituting SinCos would change the rounded power graph.
FP status, denormals, NaN payloads and Xbox rounding remain explicit limits.

Reserved cvar116: reference8308dbf4, initializer82f7bc00, static table82fd4490;
section8225df18 `Physics_Debug`, label8225df00 `kDrawSpeedWobbleInfo`,
title8225dee8 `Draw Speed Wobble Info`; defaultfalse and null callback atbc24/28.
No registration yet; slots114/115 belong the earlier joints/animation work.

## Remaining consecutive consumers and force boundaries

After wobble, source calls SetTruckDamping a118, UpdateSteeringTilt a124,
SetWheelMaterialForSurface a12c. These must consume the captured damping and
actual returned wobble tilt before any forces. WallRideForce a1a0 produces a real
force/jump-info pair and updates wallride time; it cannot be replaced by false.
The optional fake-torque cvar branch a1e4..a42c has its own log/exp numerical
dependencies. Jump flag a43c chooses SetVelocity a444, drag a460, actual launch
info/selector Launch a49c and Update a4a4, then singleton check.

The non-jump route calls CollisionForce a52c. Positive collision retains force,
torque and timer, targets displacement and accumulates collision force a5f4.
The false branch calls braking a6bc, push a6c8, absorption a6d0, front/rear foot
a6ec/a704, speed model a710, slide a720, pump a72c, straighten/heading/antiflip
a73c/a748/a754, manual a774 and linear drag a77c before applying those results.
Further real effects include anti-flip nudge a7e4, force accumulation a800..a930,
drag a93c, hangups a984, halfpipe wheel catches a98c and pinning a994.
None of these is covered by merely completing wobble; no flatground bypass or
fabricated collision outcome is selected here.

## Draft and next registration scope

New files: `skateboard_speed_wobble_calculate.rs`, its attributes/tests children,
and `reckoning_sine.rs`. Four controlled tests cover inactive NaN, captured
pointer/load ordering, activation offset versus result, delayed-debug failure,
and active reset preserving amplitude. They are drafts, not passed validation.
Next live integration needs child registration, narrowly mutable access to the
existing wobble owner, the live cvar116/debug/numeric service adapter, and the
actual caller's COM-length graph plus ordered wrapper reads. No shared file was
changed for this draft. Full Ground.UpdateSkateboard remains incomplete.

Root independently reviewed full Calculate raw82b2eb68..efe0 and scalar-splat
XMVectorSin824b0060..012c against the drafts. Repeated attribute captures and
late loads, local frequency, effect/angle/offset stores, delayed debug failure,
time/reset order and preserved return after reset agree. The sine recurrence
and final-negated reduction match the original graph. Algorithm approval is
pending registration and validation; the live caller/setter adapter has been
read and still requires its own final source/owner integration checks.

### Live caller draft through a12c

`physical_player_ground_control_complete.rs` is an unregistered constraint-session
child. Its one `try_update_pair_ground_control` invocation calls the existing
PhysicsError/damping prefix, mutates the SAME Ground pushScalar/dampedTurn via
the source steering evaluator, reads actual Skeleton COM2a30, computes length,
updates actual board wobble, then applies damping, steering, wheel material.
Both returned scalars remain local to this same invocation; callers cannot mix
the damping of one board with another board's steering result.

`physical_player_ground_control_retained.rs` is an unregistered board-construction
child granting narrowly scoped access to actual force runtime and original
constructor attribute provider. It creates no replacement runtime or bindings.
The outer caller uses this only after prefix has verified board, Skeleton,
Processed and selected Ground identities. Locks release before reacquiring the
same owner; no nested recursion into physics-state or Processed locks occurs.

The COM-length graph a0a8..a100 uses the already reviewed
`construct::board_fill::board_speed_from_squared_and_estimate`: unconditional
dot3 and estimate service, two native VNMSUB refinements, squared*inverse, then
exact-zero select. Raw a0e0/a0f0 VNMSUB final negation agrees with that shared
body. Unknown COM W is not read by dot3 and is never manufactured.

Required shared amendments, for root's post-gate window:

1. Register earlier PhysicsError pool/session children and cvar114
   `try_draw_board_joints`; register steering calculate/attributes plus Processed
   steering input child. These drafts already exist and are not duplicated here.
2. Register this retained-board child, complete session child, actual wobble
   calculate child and `reckoning_sin_cos::sine`. Register reserved cvar116 getter
   `try_draw_speed_wobble_info` (cvar115 remains the animation agent's slot).
3. Add `SkateboardSpeedWobbleOwner::value_mut()->Option<&mut SkateboardSpeedWobble>`;
   this is the same source+10c payload, preserving null/unconstructed admission.
4. Add `SkateboardTruckDrives::set_speed_damping_factor(f32)` storing ONLY its
   existing field. Raw29dd0 loads actual Body+0c,29dd4 stores f1 to Body+1dfc.
   Existing drive-dynamics consumer already reads that field; no new state.
5. Mount this full call once after actual Ground Pumping, replacing any separate
   prefix-only invocation. The old prefix helper remains internal reuse.

Existing `try_update_constructed_board_steering` owns the same live steering
attribute instance, old deck tilt and Body turn fields. New Processed input
captures only at the existing source callback, validates actual flipped-bit
knowledge and reads the same flags8c8. The returned 8c4 value is a consumed-bit
projection, not an assertion that unobserved bits are known. Existing
`try_set_board_wheel_material` captures actual selected material only after
steering succeeds, verifies its retained SurfacePhysics owner and writes four
actual Parts. No controlled material, copied contact booleans or per-call
attribute binding is passed to production.

The very next source producer is still WallRideForce82b29300. Its FIRST three
feet keys baba91606102af9a,19c804c3e3e2b566,90c4e79cbfbaed5e are read before
it clears actual Ground jumpInfo+14 doJump and +10 COMheight at2945c/29488.
It then reads current Processed groundNormal1d0 and up220. Native early-false
gates are firstValue>abs(groundNormal.Y) and dot3(up,groundNormal)>secondValue;
29500 is BO4 BI25, so NaN skips the active arm. Only a genuinely false measured
gate reaches the two source zero-force stores29974 without active queries.
Do not pre-clear the retained newVelocity merely because doJump becomes false.
The existing feet runtime pointer reader can be reused for these keys; active
wallride additionally needs its cached graph, trajectory layout, actual island
LineWorldCollisionTest, jump velocity publication and late debug cvar reads.
No wallride leaf exists in current core; trajectory wallride scoring is a
different function and cannot substitute for this producer.
