# Ground collision JumpLaunchPending

Reported during skating over bumps:
`Before(Collision(JumpLaunchPending))`, tick312.

## Cause and source

Ground.UpdateSkateboard `82d9a018` calls the existing WallRide calculator,
which can populate retained Ground jump info. The host already runs that
calculator and retains do_jump, jump_direction and jump_com_height on the
same owner. The next do_jump branch in ground_collision was still a September12
placeholder, returning JumpLaunchPending before any launch work.

Fresh IDA decompile and raw PPC confirm the missing block:

1. `82d9a430..a43c` branches on Ground+e4 (doJump).
2. `a444` SetVelocity from retained mJumpInfo.mNewVelocity.
3. `a45c` copies that same vector to Processed+190 (mVelocity).
4. `a460` sets board linear drag to zero.
5. `a468..a474` constructs launch info then calls Skeleton.GetLaunchInfo.
6. `a490..a498` sets playerJumped and copies the jump velocity to BOTH
   startVel and velBeforeJump. This differs from the GroundAnimation ollie path.
7. `a49c..a4a4` calls the existing TrajectorySelector.Launch and Update.
8. `a4a8..a4d0` requires the retained debug-overlay singleton, then sets
   mExitingFromWallRide. `a4d4` skips to the UpdateSkateboard epilogue.

Raw words and decompile: ground-bump-launch-ida-2026-09-29.txt.

## Implementation and reuse

The collision gate now returns a typed JumpLaunch continuation with its
validated owner and retained velocity. It does not read collision inputs or
run ordinary braking/force queues. The existing pre-solve coordinator consumes
that continuation and resumes the common Ground.Update tail afterward.

Extended the existing ground_jump_runtime with an explicit GroundWallride
launch kind. GroundAnimation and SlideGround preserve their existing behavior;
the same board SetVelocity, Skeleton launch-info filler, retained selector and
selector completion methods are reused. No new predictor or launch calculator.

A narrow setter writes the existing Processed+190 velocity field, preserving
the vector's W knowledge. The launch record's before-jump velocity is explicitly
overridden for this native branch. No locks are held across selector calls.
Only successful selector completion reaches the overlay check and exit flag.
The existing force-prediction stage already skips hook offsets for this flag,
and the existing Ground PhysOut publication exposes wallride exit. Ground.Exit
already uses retained do_jump and jump_direction for its velocity clamp.

## Validation

Scoped rustfmt parsing passed for seven touched Rust files. The original large
Update1 corpus was syntax parsed without reformatting unrelated fixture code.
Module audit passed:1995 Rust files,1989 literal module/include paths.
All GroundAfterCollision matches updated; JumpLaunchPending is removed from
runtime code. Added UNRUN regression checks: the retained-pair do_jump branch
selects launch without appending collision forces or prematurely setting exit;
processed velocity publication updates the existing view and preserves other
fields and unknown W. Existing wallride force-prediction early-return test is
retained. No Cargo/rustc, test execution or game launch.

Next check: build-release.bat then play-quiet.bat and revisit the bumps/wall edges
that triggered the failure. Source behavior is checked; runtime parity remains
to be tested.
