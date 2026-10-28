# Grinds: GrindAirAdjust, FootPlant CheckForGrinds, grinding Motion states (2026-09-28)

Continues `grinds-selector-chain-2026-09-28.md` with the same scope.

Status: source written and statically reviewed against IDA (debug
`sk82_na_m.xex`). rustfmt PASS on the new files and every own hunk. NOT
compiled, NOT tested. The live grind policies stay parked; the grinding Motion
behaviours `CreateGrindAttributes`, `ControlGrindCrouch` and `GrindControlFade`
are the next batch.

## KnownAir and `GrindAirAdjust` (`physical_player_grind_air_update.rs`)

- `PhysState_KnownAir::Enter` 0x82d9f828: when the selector targets a grind,
  `GetTargetGrind` (start/end; the selector never writes the header) and the
  possible orientations form `GrindAirAdjustIn` (its six target orientation
  matrices are the caller's unwritten stack and stay unmodeled), then
  `GrindAirAdjust::Start` 0x82d991c0 and `mTargetingGrind = 1`. Start sets
  `mStarted`, copies the input (flag bit 31 only), clears the chosen contact,
  zeroes the four adjustment vectors, sets chosen/last type and contact to -1
  and reruns `InitTargets` 0x82dc9a70.
- `PhysState_KnownAir::Update` 0x82d99820: with `mTargetingGrind`,
  `mGrindAirAdjustActivated` and `mAdjusting`, `DisableLeftFootCollisions(2)`
  then `DisableRightFootCollisions(2)`.
- `UpdateGrindAdjust` 0x82dd1518 (called by `Skeleton::UpdateGrindAirAdjust`
  0x82b4cd80 from `ProcessData` when 8cc bit 9 is clear): result flag cleared;
  inactive when not activated or `kGrindAssistActive` is off; 8d0 bit 27 or the
  8c8 halfword's low bit zero both totals; otherwise the frames/distances reset,
  `CalculateGrindTransform` 0x82dc6928, `ProjectPoints` 0x82dc90c0 (deck frame
  stepped by the angular velocity rotation, velocity integrating gravity
  -9.8*dt and the up vector times dt*attribute 0x6aa4b4ae2e34a5a5, five contact
  points and `CalculateAngleBetweenDeckZandGrind` 0x82dc6b18 per frame),
  `SelectContactPoint` 0x82dc6d70 (plane through the rail start along the
  velocity frame Y), `SelectGrind` 0x82dcf6d0 with
  `IsThisGrindTypeWithinRange` 0x82dc95e0, then `UpdateOffset` 0x82dc9780 and
  `UpdateAngle` 0x82dcf7d8 (with `CalculateAngleBetweenDeckZAndDesiredLandingZ`
  0x82dcf4f8). `mAdjusting` takes the result flag.
- Skeleton offset: Euler X about the grind direction applied to the AnimToWorld
  rows, WorldToAnim, the pivot matrix about the mapped skateboard position,
  Euler Y, the offset moved into animation space, stored as the full offset
  transform with `kLandingAdjustFramesToBlendAwayOffset` sampled after the full
  store and again after the unchanged W re-store (Y count).
- Parity notes: the `> 90` fold in `CalculateAngleBetweenDeckZandGrind`
  compares radians against degrees as the source does; `SelectGrind` indexes the
  per-frame angle array by grind type as the source does; output fields the
  update did not write are unmodeled stack and an error if the skeleton reads
  them; `kDrawGrindAssistInfo` needs the overlay singleton, which the
  ProcessData path does not reach, and is an explicit error when set.
- Cvars 818..821 (`kDrawGrindAssistInfo` false, `kGrindAssistActive`,
  `kGrindAssistApplyOffset`, `kGrindAssistApplyAngleOffset` true).
- `SkeletonAnimationUnsupported::ActivatedGrindAir` is gone.

## FootPlant `CheckForGrinds` 0x82dbbaf0

Box `kFP_GrindBoxSize` around `mLandingPos`, `TrajectoryBoxTest` against the
deck position; per kept rail `PhysicsUtility::TrajLinePlaneIntersectTest`
0x82e49c40 (plane through the rail start with normal
`NormalizeSafe(cross(cross(d, J), d), J)`, `CalcGreatestTimeToIntersectPlane`,
`CalcClosestPointOnLine` 0x82e48fb8) against the footplant prediction; a rail
closer than the best so far (from `kFP_MaxDistToAdjustFootPlant`) and not more
than `kFP_MaxDistBelowCollisionForGrind` below the current landing (re-read
after each accepted rail) sets time to land, landing normal and position,
surface 0 and results valid. `kDebugDrawFootPlant` overlay only when rails
were found. Cvars 822..823.

## Grinding Motion states

All 309 states under `OnBoard/Grinding` over ten update thunks (0x828ee468,
0x828eec60, 0x828eecd8, 0x828ef980, 0x828ef9f8, 0x828f0a48, 0x828f0aa8,
0x828f0ab0, 0x828f0b28, 0x828f1f10): each thunk's call sequence was matched
against every sharing state's class list (HandBusy, EndGesture, ForcePhysics
and SetGrabType updates are the shared BLR 0x825187c0). The verified lists live
in `generated_motion_runtime_grind_states.rs`, consulted by the lifecycle
fallback before `PendingStateUpdate`.
