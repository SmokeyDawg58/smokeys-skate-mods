# Grinds: TrajectorySelector grind targeting chain (2026-09-28)

Continues `grinds-manager-states-conditioners-2026-09-28.md` with the same
scope (2026-09-27, finish the parked grinds 1:1 with IDA, real values only).

Status: source written and statically reviewed op by op against IDA (debug
`sk82_na_m.xex`, VMX128 decoded with the raw operand layout). rustfmt PASS on the
new file and every own hunk. NOT compiled, NOT tested.

The live facade still passes `GroundPreparationGrindPolicy::ParkedByUser`; the
parked branch keeps the grind term zero and never reaches these calls. Flipping
still waits for KnownAir `GrindAirAdjust`, FootPlant `CheckForGrinds` and the
motion-graph grind conditions.

## New module `physical_player_selector_grind.rs` (mounted as `selector_grind`)

- `CalculateGrindScore` 0x82dc5c38: time lanes >= 0 (assert), collision position,
  box = +-layout[0x234] (W zero), `mGrindTestResultsValid` false, the box test
  against the deck translation (+70), count 0 -> 0.0; else valid true,
  `AnalyzeAndAdjustTrajectory`; success sets `mGrindBonusAwarded` and returns
  layout[0x1e0]. The box test's own return value is ignored as in the source.
- `GrindData::TrajectoryBoxTest` 0x82cf9cf0: fresh local query (vertical results
  kept, no debug draw), out count cleared, directions `normalize_unsafe`, lower
  triangle `|cross(dir[i], dir[j])|^2` (i > j) mirrored, pair filter
  `m < 1e-4` (NaN skips), `kMinPairDist^2 < |perp|^2 < kMaxPairDist^2`
  (fmuls squares, NaN not paired), `0 > dot(start_j - mid_i, end_j - mid_i)`,
  reject when `(y > -kMaxPairDistY && |mid_i - takeoff|^2 > |closest_j - takeoff|^2)
  || y > kMaxPairDistY` with `y = dot(start_j - start_i, NormalizeSafe(cross(cross(dir_i, J), dir_i)))`;
  kept primitives append (count < 40, flag bit 31 copied, lower bits of the
  destination kept by `insrwi`); `kDrawGrindPairDetection` overlay sites; returns
  count != 0.
- `AnalyzeAndAdjustTrajectory` 0x82dc5160 with `CalculateGrindTrajData`
  0x82dc4ec0, `ConsiderGrindPrimitive` 0x82dba150,
  `tTrajectory::CalcGreatestTimeToIntersectPlane` 0x82db7098 /
  `_TrajectoryPlaneIntersection` 0x82db54e0, `FindBestGrind` 0x82dc2d08,
  `GrindPrediction::FindBestGrindUpVectorOnLanding` 0x82cfca88,
  `CalculateGrindLandingNormal` 0x82dba758, `CalculateGrindVerticalNormal`
  0x82dba638, `TrajectorySelector::AdjustTrajectory` 0x82dba940 (layout[0x1f4]
  into the existing `tTrajectory::AdjustTrajectory` port),
  `PhysicsUtility::ClosestAngleToXAxis` 0x82e48f10.
- `CalculateGrindPenalty` 0x82db9ee0: 1e6 start, per tested primitive
  `vmin(vmin(vmin(|perp|^2, |a|^2), |b|^2), best)`, guarded length, 1000 when the
  results are invalid, layout+0xa0 eight-point graph, positive result asserts.

Parity points that differ from a naive reading:

- The centre-lock distance gate is `bge` (fail on NaN): `!(dist < lock)`. The
  vertical/lateral/angle gates are `ble` (pass on NaN).
- `vmaxfp`/`vminfp` keep the first NaN operand (quieted) and order signed zeros.
- `FindBestGrind` copies the 0x7dfcd6f1d65b425c graph once (0x50 bytes,
  `DefaultDataArea` when null) after the empty-list check.
- `FindBestGrindUpVectorOnLanding` ignores its landing-velocity and takeoff-up
  arguments; the crossing velocity AnalyzeAndAdjust builds for it is therefore
  not modeled. IMPOSSIBLE leaves the result unwritten; the success path only ever
  consumes the same iteration's result.
- `mGrindPrimitive` receives start and end only (+960/+970); header and flags are
  never written by the selector.
- `ConsiderGrindPrimitive` plane normal is `normalize_unsafe(cross(delta, cross(J, delta)))`
  (horizontal for a flat rail), direction `ClampLength(delta, 1)`; the old
  `grind.rs` helper assumed a vertical plane and always normalized.
- Plane intersection: two roots -> `vmaxfp(t1, t2)`; one root (D == 0 or NaN)
  -> `t1 > 0`; `t1 = recip(2a)(-b + sqrt D)`, `sqrt D = D * rsqrt` with the zero
  guard, reciprocals `vrefp` + two fused refinements.
- Penetration index `fctiwz(t * 59.999996)`; out-of-range stays the port's
  explicit conversion error.

## `CalculateScores` 0x82dc5e00 (`physical_player_selector_scoring.rs`)

`Native` policy: central candidate (`i == 0 && pass == 1`) is grind scored unless
8c8 bit 30 (`mIsAIPathFollowing`) with AI word +6b0 `0x04000000` set and
`0x08000000` clear; then 8cc bit 27 or a zero score takes the penalty (NaN keeps
the score); the wall probe runs unless the grind score is ordered positive, with
`nearGrind = grind < 0`. On vert, `grind > 0 && sign < 0` clears
`mGrindBonusAwarded` and replaces the score with the layout+0xa0 graph at 0.
`NativeGrindPending` is gone.

## Wiring

- `SelectorSource`: `grind_scoring_allowed`, `avoid_grinds`, `grind_score`,
  `grind_penalty`. `Live` reads 8c8 / AI flags / 8cc from the processed contact
  (unknown bits are errors), builds the world from `PhysicalGrindServices`
  (`try_box_test`, vertical kept), `WorldLines` over the island queries,
  `GrindAttributes`, the `physicsdeck` default instance (LENGTH / EXTENT) and the
  captured `physics_trajectory` layout (`WallRideTrajectoryAttributes::try_data`
  for keyed graphs).
- `OnboardTrajectoryOwner::grind_test_results: GrindDataQueryResult` replaces the
  placeholder primitive array and count.
- Tests: the test source records the new calls; `native_grind_scores_only_the_central_candidate`
  replaces the old pending-error test; scoring tests cover the negative penalty
  wall probe and the on-vert reversal.
