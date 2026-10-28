# Retained onboard selector completed-query consumer

Confidence: **high** for recovered source control/data flow; numerical estimate,
reduction, signed-zero and exceptional arithmetic remain the existing explicit
host boundaries. No deterministic per-tick Xenon trace or corpus execution is
claimed by this packet. Root owns Cargo and integration validation.

## Ownership and scope

`Sk8::Physics::TrajectorySelector::Update`82dc6528 consumes its retained
`mPhysicsPredictorQuery` futures, already produced/harvested by the actual World
predictor path. The new consumer operates on `OnboardTrajectoryOwner`; it does
not instantiate the older `physics::TrajectorySelectorState` diagnostic owner.
Same-pair bindings are checked against the existing PhysicalPlayer and Processed
Arc. Board foot attributes, Global trajectory attributes, cvars, World query
owner, original ground and actual DMO collection are reused. Only immutable
Vault bindings are cloned; Processed reads remain lazy.

This packet completes CalculateScores, AddWallRideScores, stable winner copy,
and the near-landing return. The other branch returns `BeforeCom`, without
publishing ready. COM82dc02f8, second-pass decision82db98a0 and launch82dc3fe8
remain an explicit continuation. No caller may treat `BeforeCom` as a successful
ready result. `winning_result=Some(i)` is presence/provenance metadata: native
pointer targets the separately copied `winning_buffer`, which future consumers
must read, not the later-mutated query slot.

Host must pass `GroundPreparationGrindPolicy`. ParkedByUser maps explicitly to
ParkedForFirstSkatingTest and excludes grind score/penalty calls, using zero
grind contribution and the non-near-grind wall branch. This is a deliberate
first-test deviation, not a native query miss or complete frame parity.
RunRecoveredPrefix maps to Native and stops at the first needed unrecovered
grind scoring call. There is no default parked policy. Legacy grind helpers and
legacy selector are unchanged.

## Source graph and layout

- Update82dc6528: clear changed even on no-work return; query0 readiness gate;
  pass2 changed publication; raw-debug load; clear started/best/winner then
  increment signed pass; CalculateScores65d0; clear grind list65d8;
  AddWallRideScores65e0; strict `>` scan from -100000000 preserves earliest tie;
  full result/data copy and targeting-wall flag. Live layout1e8 is loaded before
  collision-time assertion. For time below minimum, publish suggested normal,
  ready=false, suggested-valid=true, changed=false and return.
- CalculateTrajectoryScores82dc5e00 reads global Reckoning FETCH
  30774f8a272f682b as signed minimum frames (null DefaultDataArea4=0), captures
  generated Global trajectory layout24c, reads 1d4/1dc/1d8/1ec/1c8 in order.
  Each candidate resets scores/wall flag before future-readiness assertion.
  Frame threshold and all-four-lane time>=0 gates precede all candidate tuning.
  Central first-pass off-vert bonus; SelectCentral is read on every eligible
  candidate and overrides index0 to1000000. Full typed landing vectors preserve
  unknown W. Direction uses normalized landing velocity cross normal length;
  force uses signed normal dot unnormalized landing velocity. Both use graph8
  f0 and separately rounded factor products. Time bonus graph8 at10 is added
  before final surface contribution using native operand/grouping.
- PredictionResults.GetCollisionPosition82d8a648: abs(time)<100 ordered assert,
  separately rounded t*t, velocity FMA into p0, half-gravity FMA into that result.
  CalculateLandingVel82db9b68 uses signed collision frame converted to float,
  multiplied by exact fixedDT3c888889, then gravity FMA with v0. Fractional
  collision time is not the landing-velocity input.
- CalculateSurfaceScore82db9dd8 uses `(surface>>7)&31`; 6/7 capture trajectory,
  read Processed up220, load1cc/1d0 then publish up. 8 reads1d0 without normal
  mutation. CalculateScores copies resulting normal back into candidate data.
- Vert branch reads transition95c, ground1d0, damped-ground-velocity200 with
  only Y cleared; normalize; ordered collision-time assert; apex helper
  82db7198 asserts down gravity and uses reciprocal estimate plus two refinements.
  Apex penalty and direction grouping preserve source single precision.
- CalculateWallRideScore82dc0558 reads both feet FETCH keys
  baba91606102af9a and90c4e79cbfbaed5e before normal-Y gate. Probe starts at stored
  collision position plus normal*layout1a0, ends ten down; it is not ballistic
  position. Existing same-World line test preserves actual ground/DMO hits.
  Hit path captures layout then normal dot, per-height1c4. Slope uses start
  trajectory position minus ballistic collision position, horizontal length,
  conditional reciprocal for length>.01. Fresh max-slope1a8, min-height1a4,
  graph4 at180 and max-adjust1f4 are sampled only on their source branches.
  Accepted wall updates highest ballistic Y before trajectory adjustment;
  low height subtracts1000 without per-height multiplication, slope rejection
  and near-grind rejection multiply then subtract. Debug cvar is sampled at
  the original branches and final gate; actual debug-overlay presence required.
- tTrajectory.AdjustTrajectory82db6e78 asserts negative maximum before frame
  guard; frameDT reciprocal is refined twice, multiplied into delta; squared
  norm gates conditional length estimate, scalar division, then final velocity
  FMA. Existing generic clamp is not equivalent.
- AddWallRideScores82db96f8 independently rereads minimum frames, captures
  highest, asserts each future, repeats frame/full-time gates and computes
  ballistic position. A zero wall score above highest gets2000 even when its
  wallride flag is false. Wall score is left operand when added to score.

## Raw-branch review corrections

Raw PPC `blt` collision-time gates require ordered abs(time)<100; NaN asserts.
Other inverted scalar branches differ: Adjust maximum rejects only `<0`, and
length runs only squared>maximum². Wall slope accepts GT-clear and height
accepts LT-clear, including unordered. Transition chooses +1 unless `<.5`;
apex penalty is -500 only when absdiff<minimum. Focused regressions cover these
lazy branch distinctions. Existing fallible ReckoningNumeric dot/estimate and
refine_drive_reciprocal helpers are reused; their documented host seed/reduction
and VNMSUB exact-zero boundaries are not asserted as unconditional bit parity.

## Runtime cvars and integration

New indices252..255 exclusively reserved for this packet. All false, null
callbacks, section Physics_Trajectory. Initializer/table/reference:

| Label | Title | Initializer | Table | Reference |
|---|---|---|---|---|
| kShowTrajectorySelect | Show Trajectory Info | 82f96840 | 82fd5c70 | 830da52c |
| kShowRawTrajectory | Show Raw Trajectory | 82f968e0 | 82fd5c78 | 830daf88 |
| kSelectCentralTrajectory | No Trajectory Help | 82f96980 | 82fd5c80 | 830da96c |
| kShowWallInfo | Show Wall Info | 82f96a20 | 82fd5c88 | 830da67c |

Registration order inserts252 before existing241 and253/254/255 after241.
Actual title bytes read from82351f94,82352014,82352088,8235210c; label bytes
82351fac,82352028,8235209c,8235211c. Raw-trajectory drawing selected at pass0
stops explicitly as DebugDrawPending. ShowTrajectorySelect is reserved for
the later COM/second-pass continuation. Shared patch changes only three onboard
landing carriers to SkeletonRootVector, registers modules/cvars, and updates
the cvar section/title-order test (distinct from initializer order). Original
constructor known zero vectors preserved.

## Validation

Focused unregistered tests cover no-work laziness/changed clearing, stable
winning-buffer copy, explicit Native stop, no premature normal-path ready,
fourth-time-lane NaN skip, non-wall2000 bonus, failed surface scalar publication,
stored-position wall probe, unordered slope/height acceptance, unknown W,
frame-based velocity, unordered adjustment estimate laziness, negative maximum
assertion ordering. Standalone rustfmt passed. Cargo, strict lint, original-data
probe and trace remain root validation, not run by this agent.
