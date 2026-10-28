# Pushing501 planted-foot integration

A playtest reached 603 ticks, then RB near an object failed with
`OffBoardPushing(GrindError("Targets(PendingState(0))"))`.
The startup camera fix thus progressed to gameplay; moving/grabbing parity is
still unverified. State0 here is OffBoardFeetIK LEFT_FOOT_FIXED, not an action
graph ID. UpdateState already selects it and captures the animated toe position.

## Existing implementation reused

Search found CalculateIKTargets in physical_player_offboard_feet_ik_prefix.rs
supporting states2/3/4 and explicitly rejecting0/1. Its existing free-foot
height correction/blend helper remains shared by the unfixed foot. The existing
SkeletonIK range clamp, world/local setters, retained OffboardFeetIk owner,
pair validation and foot-normal application are reused; no second IK solver.

## IDA checked this turn

- CalculateIKTargets `0x82dd0928`, left fixed `0x82dd0a34..ae4`, right fixed
  `0x82dd0bf4..c90`; decompile and decoded raw VMX checked.
- Both sides compare XZ squared displacement with kOBIK_IKMaxDist squared.
  Native bge is !LT (including unordered); below threshold copies the stored
  anchor. Clamp path restores original Y and stores target AND retained anchor.
- Native asymmetry: left flattens both clamp inputs' Y; right passes full XYZ
  to ClampVectorAWithinRangeOfVectorB, then restores Y. Do not symmetrize it.
- Existing clamp `0x82e496d0` is already implemented in skeleton_ik_line_frame.rs.
- ApplyIK `0x82dcd8e0`, setters/timers `0x82dcd98c..9f0`: planted side uses
  world setter at1, then reloads TimeToBlendOutFixedFootPos into its timer.
  Free side uses the existing local setter with IKBlendVal (air uses1).
  Left precedes right, then existing normal setters. Merely allowing state0
  through target calculation would have left this second behavior incorrect.
- kOBIK_IKMaxDist initializer `0x82f98e18`, reference `0x830db56c`, table
  `0x82fd5e2c`: Physics_Systems_OffboardIK / IK MaxDist, default bits3da3d70a
  (.08), GUI0..1, raw r10=0 callback. Added linked cvar896; no hardcoded tuning.

## Validation / next

Five changed Rust files passed scoped rustfmt syntax parsing. Cvar definition
count897 and registration indices0..896 form a complete unique permutation.
Two unexecuted regression tests cover native left/right clamp asymmetry and
the below-range XZ gate retaining the full anchor even with a large Y delta.
No Cargo or gameplay run.
Existing stale historical cvar title-list test remains a separate known issue.
Rebuild, then try holding RB near the prop, moving it and releasing it.
