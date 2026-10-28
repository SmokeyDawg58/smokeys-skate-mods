# Braking expression 1 and scoring/HUD re-audit

A playtest hit `Actor(Action(Update(PendingExpression(1))))`; this also reviews
scoring, multiplier, text-colour and animated-ring parity.

## Braking fix

Fresh live IDA `PreEvaluateExpression` 0x828ff990:

| Expression | Branch | Existing condition | Authored comparison |
|---|---|---|---|
| 0 | 0x828ffb78 | 3, PhysicsSpeedCompare | speed > 5.0 |
| 1 | 0x828ffbd4 | 4, PhysicsSpeedCompare | speed < 0.5 |
| 2 | 0x828ffc30 | 5, InParentStateForTime | parent time > 10.0 |

Verified original `kConditions` 0x8217c460 and `kAttribs` 0x82174cd8 directly:
attributes 317, 318, 316 respectively. The unusual ten-second comparison is
authored; it was not shortened. Transition 25 goes StartBrake14 -> WaitBrake15,
transition 26 goes WaitBrake15 -> OnBoard6. Expression 0 was already connected;
added only the missing 1/2 dispatch and the native no-read Sustain0..2 branches
(jump table 0x82906084 -> default return-true 0x8290a90c).

Reused the existing speed comparison, tagged-parent timer, condition cache,
state update and graph selection. No second braking implementation, fallback
predicate, tuned threshold, or suppressed pending error.

## Scoring paths checked against live IDA

- ScoreModule.Update 0x82dfcd18: collector lifecycle, pending air reward,
  meter decrement, sequence/line completion, output publication ordering.
- CheckSequence 0x82df61e8: bank with the multiplier from BEFORE the finishing
  sequence raises momentum; stoke the line meter with raw points only when the
  resulting multiplier exceeds 1.01.
- RewardAirSequence 0x82df2888: switch/fakie exclusivity, grind/manual landing
  factors, sketchy factor, existing attribute lookups and pending reward owner.
- CheckLine 0x82df2b00: bail, runout, timer expiration and board retrieval reset
  the line/multiplier and repetition history; ordinary empty-timer history
  clearing still waits for the sequence to finish.
- ScoreHolder.EndSequence 0x82dee660: banked/finished points, first-line event,
  completed trick histograms and reset of current sequence totals.
- CalcPointPenalty 0x82dee2e8: category5 exemption and eight-point authored
  repeat curve at layout +276/+308; the existing helper's offset260 adds the
  curve header16. No invented fixed percentage penalty.
- MomentumMeter.EndSequence 0x82dea488: highest eligible authored threshold,
  native strict allow-drop comparison, existing multiplier attributes.
- ScoringMeter.Decriment 0x82dea250: ordered fraction checks, timestep-based
  decrement and native prevent-zero constants.
- TrickDisplayHUD.Update 0x826bda40: multiplier notification, line-score/full
  meter notification, then sequence/trick/stance change notification. Full
  meter remembers a refresh for the NEXT update. Seconds alone do not seek
  the timer every frame.
- GetLineTimeRemaining 0x82605e50 returns an AptInteger. Original AS computes
  `maxFrames - clamp(seconds * 60 - 30, 0, maxFrames)` with maxFrames500;
  numeric frame arguments are one-based. The retained animation advances
  between seeks. Do not replace this with a render-FPS-dependent timer.

These reviewed paths matched the existing implementation; no scoring tuning
or color changes were justified by this pass.

## Original HUD and colours

Re-extracted the original `hud2/trickdisplay2.apt/.const/.geo` from screens.big
into ignored audit artifacts. All 1,565 placement controls are ordinary
PlaceObject controls, with no positive clip-depth masks or version3 blend
controls. Thus the ring does not require a newly invented stencil pipeline.

The retained original x1/x1.5/x2/x3 timelines drive the multiplier background,
special ring pieces, glow, score backer and highlight. Some old ActionScript
instance names are absent from this shipped movie; native calls on those
undefined objects do not introduce new visible pieces.

Original text ARGB values:

| Text | ARGB |
|---|---|
| Trick name | FFFFFEEC |
| Score digits (both sizes) | FF000000 |
| Line label/digits | FFF3C81A |
| x1.5 localized text | FF084D15 |

The other multiplier glyphs/backgrounds are authored geometry/texture symbols,
not substitute font strings. Special top-left ring tint: 02BF38 for x1/x1.5,
F7EE0D for x2, black for x3; the separate top-right piece has its own authored
tint and timeline. Keep these independent instead of tinting the whole circle.

Live rendering checks: appendColourTransform 0x829e72a8 and the optimized
0x829e5d68 path multiply byte scales and saturating-add byte translations.
AptTextureUnit.Draw 0x82609d38 applies element colour * scale + translation
BEFORE texture sampling; DrawRenderingUnit 0x8261da48 uses source-alpha /
inverse-source-alpha blending. Existing GPU submission agrees. The tempting
change to add the bias after texture sampling would be incorrect here.
SkateAptString.Draw 0x82a717f8 retains the same original colour inputs.

## Validation and limits

Regression additions check braking predicate false/true behavior, no condition
reads for native Sustain defaults, and actual source declaration thresholds.
Expanded original-asset score test checks submitted text vertex colours,
multiplier background identities, x1.5 localization, timer frame/movement,
ring tint reaching geometry vertices, upward/downward multiplier transitions,
context badges and bail clearing.

Validation completed:

- `cargo test --release -p skate2-pc -j1 --no-run`: PASS, including the current
  release core library. Initial build4m36s; incremental fixture update27.75s.
- `original_score_movie_trick_multiplier_timer_and_bail --ignored`: PASS with
  the original mounted assets and expanded checks above.
- Full `skate2-core --lib` test compilation: FAILED with
  `rustc-LLVM ERROR: out of memory` / exit0xc0000409. No Rust type error was
  reported; do not represent this as a passing full core test suite.
- To avoid repeating the memory-heavy build, generated an ignored source
  harness at `artifacts/braking-scoring-focused-tests.rs`: verbatim current
  expression function, original declaration types/table, new braking test,
  plus the actual `trick_display_hud.rs` module. Four tests PASS. Unrelated
  expression delegates panic if reached; this does not fake Actor physics or
  claim full graph execution. The application build separately verifies the
  real surrounding types and integration compile.

No normal executable was rebuilt: run `build-release.bat` before playtesting.
No normal save or live game session was modified. This is a source/asset and regression
audit, not a claim of screenshot-perfect parity for every trick, career mode,
localization or frontend audio effect. Existing queued multiplier sounds still
await the actual audio consumer.

Fresh raw source artifacts: `artifacts/action-score-audit-0x828ff990.txt`,
`artifacts/action-score-audit-0x82906048.txt`,
`artifacts/scoring-hud-colour-audit-20261001.txt`,
`artifacts/scoring-reaudit-native-20261001.txt`.
