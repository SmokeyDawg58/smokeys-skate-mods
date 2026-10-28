# Retained landing-quality consumer

Confidence: high source reconstruction; independent review and root gates pending.
No native numerical trace parity claim. New drafts only; no registered/shared
edits or Cargo by output_continuation. Direct rustfmt passed.

`Sk8::Physics::PhysOutConditioner_Physics::CalcLandingQuality`82e2aa88 is
Physics.Update82e2f970 sixth call at82e2f9ac, after CalcFootSideOfDeck.
New same-output consumer consumes PhysicsBeforeLandingQuality and returns
PhysicsBeforeStairs; WatchForStairs82e2cb70 remains outside this packet.

The actual retained Physics last_filtered_state+90 is read first(aab8). If
Ground1, return with no output guards, tuning or numeric access. Otherwise read
filtered_state+94(aac4); only Ground1 enters analysis. This includes constructor
Invalid->Ground first publication. No Physics histories are changed here.

## Actual fields and access order

GetCollisionaad4 guard then average_ground_normal+20 loadaae4; first Motion
getaae8 retains deck_velocity+50 addressaaf0; second Motion getaaf8 precedes
skateboard_flipped+111 loadab04; Board getab08 precedes late retained velocity
loadab10. Dot(normal,velocity)ab14 occurs before Board.transform.zAxis+20
loadab34 and dot(normal,board)ab3c. Actual Board.Fill, motion publication and
Collision.Fill already own these output fields. Use actual transform, not effective.

Project both vectors with rounded normal*dot followed by subtraction(ab5c..68).
Dot projected velocity squaredab6c, projected board squaredab70, and their
unscaled mutual dotab78; abs(mutual dot) is the published `sketchyness_cosine`
(the source name does not imply a normalized cosine). Rsqrt estimatesab80 and
ab94 occur even for zero. Each gets two Newton refinements, exact VNMSUB final
negation and fused half-seed residual addition; squared*refined reciprocal-sqrt
is the length, with exact zero-squared selecting +0 only after this arithmetic.
Both estimate calls precede the refinements. W remains lane-aware.

Cross(projected board,normal) uses VMX permute/product/VNMSUB/permute, with
separately rounded product and final fused negation. Dot against original
velocityac20, absolute valueac24 yields side speed. This dot occurs even when
the earlier product of both lengths is degenerate. Raw ac30=4098001c tests !LT and enters active for unordered too. Only
ordered length-product<f32 bits3727c5ac selects degenerate Straight0 and
sets scalar, cosine and side speed all +0 without cvars/global reads.

## Complete active branch

Reciprocal estimates for velocity lengthaca4 then board lengthacb0, each two
Newton steps; multiply projected vectors by refined reciprocal lengthsacdc/e0.
Cross(normalized board,normalized velocity)acfc.Y is sign-flipped when captured
skateboard_flippedad00. Guarded Air getad28; negate actual body_spin_speed+bc.
Capture LoBodySpin targetad48 and HiBodySpinad4c before abs spinad50 and float
readsad54/ad58. Rounded width, delta, divisionad5c..64. Fresh actual global
physics_animation_default instance/layout fetchad68..7c, upper-only FSEL
`1-fraction>=0 ? fraction : 1`ad8c, four-point Evaluate824e2c40 atad90 using
layout X170/Y180. No lower clamp; NaN fraction selects1.

Capture LoSideSpeedadac and HiSideSpeedadb0, readsadb4/b8, rounded delta then
width then divisionadb c..c4. Fresh global/layout fetchadc8..d8; same FSEL
bound and four-point graph X190/Y1a0 atade8. Then live StraightSpeedThresh
targetadf8/valueadfc. Raw ae04=40980010 continues on !LT, including unordered;
only ordered speed<threshold selects Straight scalar0.

If proceeding, flip normalized board if captured flipped, dot with normalized
velocityae44, ordered alignment>=0 comparisonae68. Then BodySpinThresh target
ae78/valueae7c. Raw ae84=40990048 tests !GT: nonpositive comparison or unordered
selects Sketchy1 and side response. Else
rounded signedSpin*crossYae8c chooses Spin2 when alignment and positive-product
disagree, Sketchy1 otherwise (unordered product is not positive). Response is
native FSEL of rounded spinResponse-sideResponse: nonnegative chooses spin,
otherwise side; this deliberately is not generic f32::max.

Capture StraightAngleThresh targetaee0 BEFORE signed response FSELaee4; load
thresholdaee8. Raw aef0=40980044 tests !LT and keeps the result even for unordered.
Only ordered abs(crossY)<threshold reads
ForceSpinThresh targetaf00/valueaf04 even if result is not Spin; retain only
Spin with abs(scalar)>force. All others reset result Straight0/scalar+0.

Six fresh Animation getters precede six sequential writes: analyzed=trueaf40,
resultaf4c, anim_scalaraf58, cosineaf64, side_speedaf70, sketchynessaf7c.
Earlier stores survive later failed guard. Source values are never manufactured
to satisfy a missing active collaborator.

## Global/Vault adapter and remaining ownership obligation

Caller supplies `Option<&AttributeRuntimeInstance>` for its actual retained
global physics_animation.default (class74bc9c93affe6aac, defaultd7edbd362d7d2152),
not Skeleton's per-owner wrapper. Absence fails exactly at first reached global
fetch. Actual `try_cached_layout(0x220)` preserves retained layout identity and
source direct-layout route; a null layout fails (native direct dereference has
no fallback here). Each of the two graph evaluations fetches it independently.
No fallback graph/constants are used. Parent owns attaching this actual session
global instance; standalone kernels do not establish that ownership.

Four-point evaluator mirrors native824e2c40 endpoint/interval/FMA behavior and
reports the interval assertion. Existing physics::PointGraph has a width>0
branch (unordered width chooses upper), matching raw40990030 BO4/BI25 !GT.
The bounded helper now uses the same predicate; an earlier BLE-as-ordered
interpretation was incorrect. Missing intervals retain a checked assertion.
Numerical estimates/dot reductions use actual ReckoningNumeric, retaining its
declared host denormal/NaN-payload/reduction boundary. No normalization substitute.

## Eight cvars, reserved205..212

All section `Physics_Landing`, min0. Raw r10=0 immediately before each ctor
proves null update callback; decompiler surplus argument guesses are incorrect.
Author read actual table dwords and initializer immediate constant symbols.

|index|label|title|defaultbits|max|initializer|ctor|table|reference|
|---|---|---|---|---|---|---|---|---|
|205|kBodySpinThresh|Body Spin Thresh|3dcccccd|10|82fa0470|82fa04b0|82fd6458|8311e7a8|
|206|kLoBodySpin|Lo Body Spin|3dcccccd|20|82fa04d0|82fa0510|82fd645c|8311e544|
|207|kHiBodySpin|Hi Body Spin|40e00000|20|82fa0530|82fa0570|82fd6460|8311e86c|
|208|kLoSideSpeed|Lo Side Speed|3dcccccd|20|82fa0590|82fa05d0|82fd6464|8311e708|
|209|kHiSideSpeed|Hi Side Speed|41200000|20|82fa05f0|82fa0630|82fd6468|8311e4ec|
|210|kStraightSpeedThresh|Straight Speed Thresh|40000000|10|82fa0650|82fa0690|82fd646c|8311e8e8|
|211|kStraightAngleThresh|Straight Angle Thresh|3e4ccccd|1|82fa0710|82fa0750|82fd6474|8311e9c4|
|212|kForceSpinThresh|Force Spin Thresh|3e99999a|1|82fa0770|82fa07b0|82fd6478|8311e840|

Append definitions205..212 using these strings and Value::Float(from_bits).
Insert205..212 after existing index80(table6454) and before31(table6494) in
selected REGISTRATION_ORDER. Native unselected initializer82fa06b0/table6470
is intentionally not reconstructed by this set. Root owns202..204 slots.

## Registration and actual caller

conditioners module:

```rust
#[path = "physical_player_physics_landing_quality.rs"]
pub(crate) mod landing_quality;
```

construct module:

```rust
#[path = "physical_player_physics_landing_quality_output.rs"]
pub(crate) mod landing_quality;
```

input_cvar_runtime module:

```rust
#[path = "physical_player_physics_landing_quality_cvars.rs"]
pub(crate) mod landing_quality;
```

Call actual retained `conditioners.try_update_physics_landing_quality(before,
output,cvars,global_physics_animation,numeric)` after foot-side. It consumes
PhysicsBeforeLandingQuality, returns PhysicsBeforeStairs or explicit error.
No shared field additions. Do not rerun prior callbacks to manufacture tokens.

Seven tests authored: zero geometry lazy tuning plus exact numeric call order;
active late missing-global failure; full active positive/negative/zero spin
using actual synthetic retained layout and linked live cvars; numeric failure
without publication; retained history gate after actual prefix+DampedCOM+foot;
unordered geometry reaching live tuning; late ForceSpinThresh control of small angles.
Fixtures are synthetic, not reference trajectory traces. Root registration,
independent review and full gates remain required.
