# Ground torque producers after ordinary force calculations

2026-09-12 NEW/unregistered drafts, source read directly from reference IDA/PDB.
No Cargo/tests run by this agent. Root owns independent review/integration.

Caller82d9a730..a774: fresh Ground+a8 time, StraightenOutTorque82b28560
at a73c, HeadingTorque82b30cc8 at a748, AntiFlipTorque82b299a8 at a754,
full AntiFlip result to SAME Ground+90 at a770, then ManualEffect(false) a774.
None can be reordered for an assumed flat-ground inactive branch.

## SAME retained Ground output

Exact migration: PhysicsGroundFields.anti_flip_torque Option<[f32;4]> becomes
Option<SkeletonRootVector>. Constructor None stays None; existing Enter and
Ground.Update0e50 fullzero stores become Some(Vector::known([0;4])). Actual
a770 copies the whole returned typed vector. Graph agent's Ground.Fill kernel
already accepts typed vectors and agrees to remove the adapter array promotion.
No second torque field or default when the producer has not run.

## AntiFlip complete draft

`skateboard_antiflip_torque.rs` implements82b299a8..29dcc, hidden result r3 and
actual board r4. Source99e0 reads Processed8c4 BEFORE99fc/9a00 output fullzero;
manual mask20000000 returns without any other read. Nonmanual reads balance9a0,
captured deckZ60, captured Processed up220, dot(up,Z), captured deckX40, then
NormalizeSafe(Z-up*dot,+zero), dot(up,X), NormalizeSafe(X-up*dot,+zero).
Normalizations call actual Math.NormalizeSafe at9a70/9a9c. Shared one evaluator
uses explicit final-negated-FMA refinement (raw824b1988/1998 VNMSUB).

Both direction cosines are dot(original,normalized projected). Source clamps
with VMX max(0,dot), then min(1,result), NOT [-1,1]. Typed cross uses the existing
reviewed YZXW product/VNMSUB/permute graph. Pitch sign is dot(X,cross(Z,planarZ));
roll sign is dot(Z,cross(X,planarX)). Both signs captured before either acos.
First XMVectorACos9b78 then multiply3f22f983, second acos9bb4 then same multiply.
fsel sign selects+1 for either zero/nonnegative and -1 for negative/NaN.

Fresh actual global feet layout each graph:9c34 eight points Xb0/Yd0 with pitch,
9c7c eight points X60/Y80 with roll. Existing feet generated layoutf0 retained
binding is reused; no fresh Vault instance. Pitch graph*sign rounded, roll
graph*sign rounded, pitch result*balanceZero rounded. Balance!=0 includingNaN
disables only pitch contribution; its entire numeric/graph path still executes.

Critical raw ordering:9cc4 target and9cc8 bool read BEFORE final output; then
Z*rollScalar9cdc, X*pitchScalar+previous fused9ce4, fullstore9ce8. Enabled debug
requires actual overlay afterstore9cf4; no draw operation. Missing cvar preserves
earlier zero output; missing overlay preserves computed output. Current host
VMX min/max exceptional selection and dot/estimate policies remain explicit,
not a bitwise Xenon execution claim.

New adapters: physical_player_antiflip_source.rs uses actual Processed/feet/
cvar/overlay owners; skateboard_antiflip_attributes.rs is a child of existing
feet; input_cvar_antiflip_torque.rs reads reserved142. Reference8308dc44,
initializer82f7b0e8, table82fd4410, section Physics_Skateboard, label
kDrawAntiFlipTorque, title Draw Anti Flip Torque, defaultfalse/nullcallback.
Initializer rawb10c r8=0 callback, b110r7=0 default. Native table order must use
4410, not host slot position. Tests cover early flag failure/manual laziness,
late debug/overlay partial writes, mixed unknownW, and nonzero/NaN balance.

## Straighten complete numerical draft

skateboard_straighten_torque.rs implements28560..28820. Processed hardness9c8
precedes actual wheels global+9c keydf474d0a659a7ae3. Assertion checks ordered
minimum<0 OR minimum>1; NaN passes BO4/BI25 at28608. Factor is fused
(1-minimum)*hardness+minimum. Capture selected surface904, read layoutec,
multiply factor, source fsel clamp time0..duration, divide. Heading global+fc
graph16 X0/Y40 is evaluated before second fresh selected surface capture286a4.

Then bodyZ160, velocity190, dot, selected surface layoute8 strength, multiply
curve. Only orderednegative forwarddot XOR-negates bodyZ; NOTLT includesNaN.
Load normal1d0 and turn980, then dot(cross(selectedZ,velocity),normal). Multiply
strength, store full normal*scalar28764. Only orderednegative angular*turn
reads late heading keyb494e9b58d1c8ba2. Its source fsel clamp abs(turn) to0..max,
divide and1-minus scale the already-published output. Errors on that late
attribute preserve the first output. No cvars/normalization in this leaf.

Existing selected surface read service needs narrow layoute8 support; durationec
already exists. Existing Body Wheels attribute reader and physical owner
HeadingAttributes global+fc can be reused. Their generated/lookup interfaces
need narrow methods rather than new runtime bindings. Live adapter remains pending.

## Heading complete numerical draft and remaining live adapter

Heading82b30cc8 has mandatory manual and nonmanual arms. SAME board+fc is
the distinct PDB mManualSpinAngle field, missing from Entry before integration.
SpeedModel uses mManualTorque at +f4; these are independent native fields. Manual arm samples
actual balance9a0 and manual turn97c, uses actual Body Manual bindings, smooths
retained manual_spin_angle, and multiplies Processed normal1d0.
Nonmanual first clears SAME manual_spin_angle, reads actual Reckoning+20 ground
transform and performs native affine transpose/inverse graph; it must not use
Part6 effective transform or manufacture new Reckoning.

`skateboard_heading_torque.rs` now drafts the complete leaf as an Entry child.
First balance9a0 read cf8; zero skips initial manualTurn97c read d04 entirely.
Manual path: global Manual+c8 key02b08c48a9f13390 then captured8c8 and fresh97c;
scale only flags10000000 with balance<0 OR08000000 with balance>0. NaN comparisons
do not scale. Keye56ac6f581f2799c supplies blend, then actual Manual graph8
X10/Y30 capture BEFORE fresh deckSpeed968. Graph degrees times exact
3c8efa35 from DEGREES_TO_RADIANS_109@8225c454. Source smoothing is
(1-blend).mul_add(oldManualSpinAngle, radians*blend); store same Entry+fc at30ebc
BEFORE DT93c and normal1d0 reads. Output full normal*((DT*smoothed)*turn).

Nonmanual f00 clears same+fc before matrix payload reads. Existing typed
Matrix.inverse reuses the matching source transpose Wzero and inverse position
Z multiply/Y FMA/X FMA; basis W is established by that source, not invented.
The inverse local-down graph executes X*0, Y*(-1) FMA, Z*0 FMA (do not elide
zero terms). Clamp inverseY.Y with scalar fsel -1..1, XMVectorACos, multiply
3f22f983, orderednegative assertion, ordered>3f99999a resets angle+0.
Set localDown.Y+0 then actual NormalizeSafe with source VNMSUB policy.

Heading graph8 X210/Y230, forwardSpeed944, ZAxisVelocity1b0; transform velocity
through SAME inverse basis X multiply/Y FMA/Z FMA, zero local Y, cross(localVel,
normalizedLocalDown*graph). Heading speed-limit key97bb45bba1c368c7 is read
twice, first for source fsel clamp abs(speed), second for its divisor. Then
graph8 X1d0/Y1f0, keydc4ddf765f8d599f, fresh VertTurn998 BEFORE products
cross.Y*coefficient*graph; key3a716c5be8bd0df0, fresh normal1d0 and angular
velocity2d0, dot3, multiply -VertTurn, fsel nonnegative (NaN->0), graph8 X110/Y130.
Fresh headingFactor9b4 then three rounded products factor*graph*coefficient*turn.

Raw3142c=41980018 BLT then3143c=41990008 BGT is a critical exception:
if candidate*old is NOT LT and abs(candidate) is NOT GT abs(old), use old.
Unordered comparisons therefore choose old too. The decompiler's ordered
`>= && <=` expression is not equivalent. Focused test supplies NaN candidate
and finite old torque to preserve this discriminator.

Fresh normal1d0 then debug target/bool31468/46c, scale full output, actual overlay
require if enabled, and ONLY THEN full output store31588. This differs from
AntiFlip's pre-overlay publication. Tests check manual smoothing before failed
DT, nonmanual clear before failed transform, short-circuit manualTurn read,
unknownW preservation, unordered candidate and failed-overlay output lifetime.

Needed live cvar: kDrawVertTurning8308d8fc, init82f7b1d8, table441c, section
Physics_Skateboard, title Draw Vert Turning Info, defaultfalse/nullcallback.
It is distinct from earlier kDrawVertTurningInput. Host slot request143 awaits
root reservation. HeadingAttributes existing global+fc class3a2a8b50453622bb
and layout250 already supplies all graph ranges; extend same retained reader's
scalar/graph capture methods, not a per-call binding. Existing Body Manual
reader should gain graph-layout capture without replacing its owner.

All numerical leaves and their tests are unregistered/unrun. Live Heading and
Straighten adapters, narrow existing attribute methods, and the source-ordered
caller/publication remain pending. AntiFlip has a concrete live source adapter,
but cannot skip these predecessors just to populate Ground.Fill.

### Corrected Heading retained-field admission

Fresh PDB and raw review distinguishes Skateboard +f4 mManualTorque, +f8
mManualUpdateTimeSinceReset, and +fc mManualSpinAngle. Constructor82b38b3c
stores +0 to +fc separately from +f4 at38b30. ResetManual82b23ef8..3f08
clears e8/ec/f0/f4/f8 only; it preserves +fc. ScaleManualForce likewise
changes only +f4/+e8. The draft now requires a distinct Entry.manual_spin_angle
field, initialized +0, and its tests discriminate both histories and reset
behavior. The earlier draft incorrectly reused manual_torque and is superseded.
No registered Entry field was changed by this draft.

### Live draft registration packet (not mounted or tested)

`physical_player_straighten_source.rs` reuses GroundForceInput late reads and
its actual selected surface handle, Body Wheels binding, and existing player
HeadingAttributes. `physical_player_heading_torque_source.rs` uses the same
processed Arc, Body Manual binding and owner HeadingAttributes. Its Reckoning
lock is acquired only at the source ground-transform read and released before
subsequent reads; callers must not retain an outer Reckoning guard across it.
No new attribute instance is constructed by either adapter.

New child registrations required: force_runtime::straighten/antiflip;
entry::heading_torque; constraint_session::straighten_source/heading_torque_source/antiflip_source;
Heading runtime child `physical_player_heading_torque_attributes.rs`;
SkateboardAttributes child `skateboard_heading_manual_attributes.rs`;
selected surface child `skateboard_straighten_surface.rs`; feet attribute
AntiFlip child; cvar children142/143. Capture helpers retain graph values from
the source cached layout before subsequent scalar reads. Native generated
Manual ctor828b0ecc..0ee0 explicitly requests DefaultDataArea(0xa0); the
Manual graph helper therefore uses the actual 0xa0 layout and X10/Y30.
Heading 0x250 layout supplies Straighten graph16 X0/Y40 and Heading graph8
X210/Y230, X1d0/Y1f0 and X110/Y130.

Shared state edits remain integration-owned: distinct Entry.manual_spin_angle
(+fc) and typed existing Ground.anti_flip_torque (+90), preserving each native
constructor/reset/store site. Source caller must execute Straighten, Heading,
AntiFlip in order and then copy AntiFlip output to Ground+90 at82d9a770. These
new adapters do not yet orchestrate that caller or publish its retained torque.

2026-09-12 root accepted torque integration. Latest accepted torque checkpoint: **workspace52 passes 4,033 unit tests and six doc tests; original46 passes all 17 core corpus tests; Clippy84, format39 and diff39 pass.** Actual mounted Ground now executes Straighten, Heading and AntiFlip in source order and publishes the same typed Ground+90 at82d9a770. Heading retains its separate manual spin angle (+fc), while SpeedModel retains manual torque (+f4); reset-preservation tests distinguish them. Neutral original data passes with the force queue unchanged. Manual effect, nudge/application/solver, full output conditioning and animated input consumption remain pending. Cvars129..143 are registered in native order, including a corrected older WallRide/Pump/Absorption ordering.

Independent raw root review completed Heading manual/nonmanual branches through315a8, caller a738..a770, separatefc constructor/reset, Manual layouta0, and float cvar137/138/140 defaults/ranges/NULL callbacks. Peer review found no owner or recursive-lock error. Clippy83 failed documentation and explicit unordered comparison lint;84 passes with source comparison preserved.
