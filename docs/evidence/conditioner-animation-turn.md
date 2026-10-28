# Animation conditioner: required Turn callback

Fresh decompile and raw instruction review, 2026-09-19. Unregistered
`physical_player_conditioner_animation_turn.rs`, intended child of retained
`conditioners::animation`, implements the full82e2b470 callback. Root owns the
actual PhysOut adapter and registration. No guest execution, replacement PID
owner, synthesized neutral input or skipped callback is introduced.

## Original data and ordered operations

Native source reads Animation.is_mirrored, SkateboardMotion.deck_speed;
live lean-angle-scale, SystemReckoning.target_lean_angle; live speed-scale;
Ground.turn; graph; wobble scale, SkateboardMotion.speed_wobble_effect;
actual-lean blend. Raw scalar arithmetic uses fdivs/fmuls/fsubs and fmadds;
wobble weight specifically uses fnmsubs, negating the rounded fused result.

Then refresh actual slow-lean filter control from physics_animation layout1f0,
update its retained history; refresh fast-lean control from200, update history;
refresh direction control from210, THEN fetch direction-scale and update that
history. All three are existing ScalarPid fields. Lean filter dead zones affect
the returned values, not retained PID values. Target is stored first; slow/fast
store integral then derivative-input; direction stores derivative-input then
integral. Each stores value before its derivative. Native FMA graph is preserved.

Read is_carving once for animation-lean delta and retain last_animation_lean;
fetch quickness max delta, blend and normalization scalar separately; retain
input_quickness then last_normalized_quickness. Evaluate speed and normalized
quickness graphs. Read is_carving AGAIN for actual-lean delta; retain
last_actual_lean; fetch holding blend and retain holding_integral, then fetch
holding threshold. Capture holding-attack attribute pointer, capture release
pointer, then read attack followed by release. Existing AttributeRuntimeData
retains the captured source range; do not replace these with two eager scalar
reads. Fetch holding max delta; retain last_holding_delta, unclamped last_holding,
then clamped last_holding. Read wobble AGAIN and its output-blend graph; retain
last_output_lean before output guards.

Eight independently checked output stores preserve partial prefixes:
Turn c038, TargetLean c064, InputLean c090, Quickness c0bc, Direction c0e8,
Holding c118, CameraTurn c148, CameraInputLean c17c. Mirroring is reapplied to
both camera outputs. Only after all stores read the live kDrawCarveInfo value.
When true, the actual DebugOverlay singleton is required. Three draw calls
c200/c228/c250 all target the release-build BLR825187c0.

## Floating branches: raw CR bits are authoritative

Do not interpret decompiler C or PPC alias spelling as ordered comparisons:

* b864 and b940 are4099000c (BO4/BI25): branch when GT clear. The .01 dead-zone
  branch includes unordered and returns positive zero for NaN. Retained PID
  values remain unchanged by this selection.
* bdc8 is4098000c (BO4/BI24): branch when LT clear. NaN holding integral or
  threshold selects zero holding target.
* FSEL selects nonnegative for either signed zero and its other arm for NaN.
  The two-step source unit/signed-unit graphs produce1 for NaN, unlike clamp.
* PointGraph824e2d10 is40990030 (BO4/BI25), not an ordered <= comparison.
  Existing PointGraph.evaluate correctly returns upper sample on NaN width.
  An initial audit inference of a shared helper defect was retracted after
  root identified and raw review confirmed these CR bits. No shared change needed.

Existing PointGraph is reused after checking endpoint/search branches and FMA
interpolation. Graph layout reads capture one live220 layout each: 8 points at
X/Y d0/f0,10/30,90/b0; 4 points at120/130. PIDs refresh through live turn_control
at byte offsets1f0,200,210. Both control and graph access require an actual cached
layout; missing layout returns AnimationTurnLayoutError::MissingLayout. The older
constructor control helper's zero fallback is not reused for direct dereferences.

## Scalar attribute keys

Keys use high32 inserted with insrdi into low32, not the truncated decompiler
argument. All source requests use index0 and missing pointers select actual
DefaultDataArea4. Implemented helpers use retained global class74bc9c93affe6aac,
defaultd7edbd362d7d2152 through existing AnimationAttributes.global.

| Use | Full key | Lookup call |
| --- | --- | --- |
| Lean angle normalization | b061790e1040392c | 82e2b558 |
| Speed normalization | 7525f3b3cd5dbea8 | 82e2b5f4 |
| Wobble lean scaling | 8b4f1a08951dc1d5 | 82e2b6dc |
| Actual lean blend | 198afc5b8490cfc4 | 82e2b768 |
| Direction normalization | 9659dfa3e06d48c5 | 82e2b9e4 |
| Quickness max delta | 742d5536361a5cd1 | 82e2bb04 |
| Quickness blend | c59ca518c35f0a0d | 82e2bb5c |
| Quickness normalization | 8f783a5ab6423a2d | 82e2bbc8 |
| Holding blend | 8cda9e8d6d31f331 | 82e2bd3c |
| Holding threshold | f87f5225afc9b077 | 82e2bda8 |
| Holding attack | 0af8487d6c704c52 | 82e2be18 |
| Holding release | 725bba7249297352 | 82e2be70 |
| Holding max delta | 3850e0aa166a0e17 | 82e2beec |

Names describe proven uses, not unverified original schema names.

## Live cvar registration packet

Both definitions are bool false, null update callback. These are the Physics
namespace definitions, distinct from another subsystem's same display title.

| Field | Carve | Tilt |
| --- | --- | --- |
| Ref | 8311e668 | 8311e5c0 |
| Initializer | 82fa0ad8 | 82fa0b28 |
| Constructor call | 82fa0b04 | 82fa0b54 |
| Initializer table | 82fd64a4 | 82fd64a8 |
| Section | Physics_Animation @82380d08 | Physics_Animation @82380d3c |
| Label | kDrawCarveInfo @82380cf8 | kDrawTiltInfo @82380d2c |
| Title | Draw Carve Info @82380ce8 | Draw Tilt Info @82380d1c |

Native adjacent table order: MinChromosomeEqualityFrames649c,
MinChromosomeEqualityFrames_Scoring64a0, Carve64a4, Tilt64a8,
SkaterStopThreshold64ac. Root assigns runtime enum indices in its batch.

## Integration and validation

AnimationTurnSource is the ordered live adapter interface. New unregistered
physical_player_conditioner_animation_output.rs implements it alongside
AnimationUpdateOutput, using the
same bound PhysOut occurrence and retained AnimationAttributes, map each read/
store to its original independent checked accessor, and dispatch the graph
point counts4/8 to existing turn_graph helpers. Associated Scalar is
Option<AttributeRuntimeData>. No fake runtime implementation is supplied.

New physical_player_conditioner_animation_update.rs consumes PhysicsUpdateComplete,
checks its output identity, Conditioners identity and the retained published
Animation child's bound identity, runs all five callbacks, and only then emits
AnimationUpdateComplete. This is not the whole eleven-child UpdateAll schedule.
Adapter uses actual outer ground_transform and motion_scalars[DeckSpeed], and
actual concrete component fields for other inputs/destinations. Captured deck
acceleration guards are checked without resampling. Two adapter tests cover
captured versus current latch and real component read/store bindings.

The five-callback caller must execute CurrentTrick, CompletedTrick, this Turn,
BodyTilt, DeckGroundAcceleration in that order. Each failure keeps prior writes;
do not prefetch attributes/inputs or validate all destinations first.

Three authored tests cover two-tick retained histories and distinct carving
samples, captured holding attribute order, failed third output store retaining
previous stores and filter updates, and raw CR/FSEL NaN/signed-zero semantics.
Standalone rustfmt passed. No Cargo or registered-file changes were performed.
Native NaN payload/denormal parity beyond existing host arithmetic policy is
not claimed. Independent source review requested before integration gates.
