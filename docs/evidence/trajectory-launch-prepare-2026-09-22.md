# Retained private Launch preparation, 2026-09-22

Status: unregistered review draft. Author ran no Cargo commands and edited no
registered modules. Root owns application, validation and final acceptance.

Artifacts: `physical_player_trajectory_launch_prepare.rs`,
`physical_player_trajectory_launch_prepare_tests.rs`, `physics_trajectory_fan.rs`,
and `artifacts/trajectory-launch-prepare-2026-09-22.patch`. The neighboring
`build-trajectory-launch-prepare-2026-09-22.py` reconstructs the patch using exact
replacement assertions and rustfmt stdin; registered files are read only.

## Existing work and scope

Existing `OnboardTrajectoryOwner` remains the only retained selector. Existing
public `begin_launch` and source-backed `try_adjust_starting_velocity` precede
this method. Root's `try_maximum_time` and actual predictor queue follow it.

`try_prepare_private_launch(owner,attributes,trajectory,numeric,rounding)` verifies
the existing physical/processed bindings and returns `BeforeCalcMaxTime` exactly
after native stores through `82dc2af0`, before the call at `82dc2af4`.
It neither queues work nor changes justStarted/ready, futures, or winner pointers.

The patch makes only data.velocity and these retained vectors typed:
initial_offset, com_position_local, board_position_local, start_board_position,
start_trajectory_position. Constructor values remain known zero lanes. Existing
launch override fields remain their established known four-lane arrays.

## Fan reuse and source order

Existing `physics::initialize_trajectory_candidates` had an early return for
count<=1, host sin/cos/length arithmetic, host clamp/min, and manufactured W=0.
The patch replaces its body with an adapter to the same new source kernel that
directly updates the real retained owner. `FanLaunch` is an ephemeral argument
record and `FanStore` routes ordered stores; neither owns trajectory state.

Fresh IDA `InitTrajectoryInfo82db9018` and raw disassembly establish:

1. Unsigned requested count is capped at seven and stored as signed halfword.
   highestWallRideCollPos becomes -1000000 and velocity0 copies all launch lanes.
2. Cross(up,start) is NormalizeSafe'd with zero fallback; then cross(right,up)
   is NormalizeSafe'd with zero fallback. Existing typed cross/normalize helpers
   preserve unknown W and source refinements.
3. Raw `9110`, `9134`, `913c`, `9158` issue dot(start), dot(beforeJump),
   rsqrt(start), rsqrt(beforeJump), even when playerJumped is false. Existing
   length-from-squared-and-estimate reconstructs both lengths after both estimates.
4. angleX is multiplied by literal degrees-to-radians `3c8efa35`. A live generated
   trajectory layout capture evaluates graph X+130/Y+150 at selectedSpeed*0.05.
   angleZ multiplies graph output then degrees-to-radians in separate operations.
5. A separate live layout capture reads bounds+1b8/+1bc. Scalar FSEL clamps to
   the lower then upper bound, preserving unordered alternatives. XMVectorSin
   runs first for X then Z, before the count-1 loop gate. Existing `try_sine`
   and `try_cosine` are reused, not combined SinCos (different power graphs).
6. Loop step is native scalar fdivs(TWO_PI, count-1); cosine(angle) runs before
   sine(angle). Candidate uses two FMAs, length then NormalizeSafe, and source
   FSEL caps speed. Angle increments before that candidate's velocity store.
   All other candidate fields retain their prior values.

The generated trajectory constructor `828b1068` substitutes DefaultDataArea24c
for a null layout, establishing zero layout reads in the existing capture API.
No new tune defaults are invented.

## Private preparation source

Fresh `Launch82dc2800` and raw `2820..2af4` establish:

- targetingWallRide=false before InitTrajectoryInfo. startBoardPos copies original
  launch.boardPos only after the entire fan returns.
- Read processed groundNormal+1d0 Y with its sign bit cleared; evaluate live
  global reckoning graph X+260/Y+280. Separately read processed velocity+190 Y
  and evaluate global reckoning X+220/Y+240 at velocityY*0.1. Scalar FSEL selects
  the maximum graph factor with its source unordered alternative.
- Read processed groundPosAllStates+1f0; dot(launch.comPos-groundPos,
  launch.reckoningOrient.yAxis). FETCH global reckoning key
  `c6710942e297d587` (radius), subtract it, then separately FETCH
  `f85a91f6a233ddae` (offset).
- Scalar FSEL selects min(availableDistance,offset); rounded offset*factor at
  `29f0` precedes `limited.mul_add(1-factor,offset*factor)` at `2a10`.
  startTrajectoryPos=comPos-up*displacement uses separate multiply/subtract.
- The optional override executes after all preceding reads/calculations/stores,
  then replaces startTrajectoryPos and startBoardPos.
- Local board offset uses **original launch.boardPos**, even when startBoardPos
  has been overridden. Board and COM offsets are transformed by the full inverse
  affine matrix, including translation, in the existing `transform_point` FMA
  graph. Source stores boardLocal+910, COMLocal+8e0, initialOffset+8d0 in that order.
  An unknown input vector W is not a coefficient in these point transforms;
  result W depends on actual matrix W knowledge, as represented by that helper.

## Validation and limits

Seven draft tests cover exact retained outputs, attribute order/graph inputs,
unknown W, original-board versus override behavior, translated inverse local
points, fan prefix stores on failure, late FETCH failure preservation, zero-count
mandatory sine execution, seven-candidate cap, and interleaved dot/estimate order.
The weak test binding is used only to exercise the private arithmetic kernel;
the live public method always verifies its real bindings.

Performed: rustfmt parse/format of owned drafts and proposed contents; patch
`git apply --check` passes. Compilation and tests remain root-only work.
Host seed/reduction/rounding exceptional semantics remain the established numeric
boundaries, not a claim of Xenon bit-identical traces. Source ordering and typed
lane knowledge are retained without host sin/cos replacement or new persistent
state. Root must call its actual maximum-time helper next, handle source AI
selection and submit all retained candidates through its same predictor queue
before publishing justStarted; none of that continuation is fabricated here.

## Read-only continuation audit for root

After `CalcMaxTime82dc2af4`, the AI gate reads processed+8c8 mask40000000 at
2b00/04. This is `known_flags_8c8()`, not the separate `physics_state.ai_flags`.
The existing actual ProcessInput producer sets it at
`physical_player_process_input_controls.rs:115` when `ai_input_active`; Reset
initializes zero and a false current input does not clear this sticky flag here.
When clear, AIPathIsUseable is not called. There is no grind branch in this
private Launch continuation.

`AIPathIsUseable82db6100` has no existing live implementation, but its actual
inputs already exist: processed controls.ai_vectors[5] is trajectory position
at+660 and [6] is velocity at+670. It checks squared distance between
startTrajectoryPos+candidate0Velocity*dt and AI position is strictly <1.5
(unordered fails); rejects dot(candidate0,AIvelocity)<0 (unordered passes);
accepts candidate speed<=1e-4; otherwise computes AI speed/candidate speed via
scalar fdivs and rejects ratio<live minimum or ratio>live maximum. Source
lengths use the usual dot/rsqrt/two-refinement/zero-select graph. Minimum cvar
reference830dac1c initializes at82f96ac0; maximum830daee0 at82f96b20.

The normal submission loop's source order is:

1. Capture live generated trajectory layout (`2b84/2ba4`).
2. Read live kShowTrajectories target/bool (`2b8c/2ba8`). This unregistered cvar
   reference830dadf0 initializes at82f96890, table82fd5c74, category
   Physics_Trajectory, label kShowTrajectories, description Show Trajectories,
   default false, null callback.
3. FETCH global reckoning radius c6710942e297d587 (`2bac`), including missing
   DefaultDataArea4 pointer, without decoding it yet.
4. Read launch.dt/candidateVelocity/startTrajectoryPos and form full-lane FMA
   `candidateVelocity*dt+startTrajectoryPos` (`2c0c`).
5. Load errorEnd at trajectory layout+1b4 (`2c14`), radius pointer float (`2c18`),
   then errorStart layout+1b0 (`2c1c`).
6. `SetupFakeBodyPrediction82e4bb88` calls actual AddQuery using predictor's
   retained gravity+30 and the captured show flag (r9, despite IDA's misleading
   drawFlag local name). A false return asserts; previously submitted futures
   remain submitted. Count is reread after each iteration.

Root must release the selector lock before AddQuery, whose reset accesses the
same retained future owner; only ephemeral request data crosses that boundary.

After AI or normal branch, even count<=0, native sets justStartedQuery at2c68
before reading kbDebugDraw at2c74. If debug is true, require real overlay, then
FETCH radius again at2ce4 including default pointer fallback. That final pointer
is never dereferenced; using a scalar convenience method would add an incorrect
late decoding failure. Root's actual debug getter index240 is reusable.
