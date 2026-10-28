# Private Launch query continuation, 2026-09-22

Status: unregistered review draft. No registered edits or Cargo runs by author.

Files: `physical_player_trajectory_launch_continue.rs`, its `_tests.rs`, and
`artifacts/trajectory-launch-continue-2026-09-22.patch`. The neighboring Python
builder reads registered files and writes only the review patch. All proposed
new paths were checked absent before creation.

## Actual integration

Call `launch_continue::try_continue_private_launch` after the existing retained
private preparation returns BeforeCalcMaxTime and its selector lock is dropped.
Arguments are the real physical owner, ReckoningAttributeRead, generated global
trajectory attributes, InputCvarReferences, DebugOverlaySlot, world-owned
PhysicalPredictionOwner, actual world queries, published ground, DMO collection,
and caller-owned ReckoningNumeric.

The implementation validates actual selector/physical/processed bindings, invokes
root's existing `try_maximum_time`, and submits through the existing
`PhysicalPredictionOwner::try_add_onboard_query`. No second owner, trajectory
array, queue, collision service or replacement maximum-time solver is created.
The internal source interface allows ordered failure tests; the live adapter
implements every ordinary continuation service using existing concrete owners.

Candidate0 velocity is captured under a short selector lock before maximum-time.
Each iteration separately reads current signed count, captures its attributes,
then captures dt/candidateVelocity/startTrajectoryPos and releases the selector
lock before any queue submission. The real queue resets that same retained
future under the same lock, so holding it across submission would deadlock.
No future/selector readiness is published by this continuation. Actual scheduled
Predictor.Update remains the only harvest.

## Source proof

Fresh raw `Launch82dc2800`, post-max region `82dc2af8..2d04`:

- `2b00/04` reads processed+8c8 mask40000000. Existing actual producer sets this
  for ai_input_active; it is distinct from the separate AI payload flags word.
  Maximum-time calculation precedes this gate. Clear bit skips AIPathIsUseable.
  Set bit returns explicit UnsupportedAiPath until the real AI predicate/launch
  is implemented; it is never treated as an invented false predicate.
- Each normal candidate reacquires generated trajectory layout (`2b84/2ba4`),
  captures live kShowTrajectories (`2b8c/2ba8`), then FETCHes global reckoning
  radius key c6710942e297d587 (`2bac`), retaining its pointer/default fallback.
- Native loads dt and current candidate/start vectors after the FETCH. Full-lane
  `velocity.mul_add(dt,start)` at `2c0c` retains unknown W in the typed request.
- Scalar reads occur in order: layout errorEnd+1b4 (`2c14`), radius pointer float
  (`2c18`), layout errorStart+1b0 (`2c1c`). This is not early batch decoding.
- `SetupFakeBodyPrediction82e4bb88` selects predictor.mGravity+30 and forwards
  the show flag from r9 to actual AddQuery; it introduces no separate physics
  body. Existing predictor submission is reused with that actual retained
  gravity. Count reload occurs after each submitted slot. A failure preserves
  earlier submissions and cannot reach justStarted or the final debug branch.
- `2c68` stores justStarted=true after all candidates, including a nonpositive
  count. Only then `2c74` reads live kbDebugDraw (existing index240). True requires
  real DebugOverlay and repeats radius FETCH (`2ce4`) without dereferencing the
  returned pointer. The draft's terminal lookup intentionally does not call float.
- No grind branch exists in this private continuation.

The generated trajectory layout fallback remains its established
DefaultDataArea24c. The radius pointer's DefaultDataArea4 behavior remains in
the actual ReckoningAttributeRead provider; the terminal dead FETCH does not
add a scalar-size/read failure.

## Real cvar amendment

New index241 is reserved for kShowTrajectories, reference830dadf0. Fresh IDA
initializer82f96890, table82fd5c74 constructs:

- section Physics_Trajectory
- label kShowTrajectories
- title Show Trajectories
- initial false
- null callback

Registration is placed immediately after existing index240/table82fd5c6c among
the retained registrations. `try_show_trajectories()` reads its current linked
target separately per candidate. Later root/runtime cvar additions reserved
indices242..244 and should compose after this amendment.

## Validation

Seven proposed tests exercise all seven original candidate slots, exact per-slot
read order and changing draw values, full-lane FMA with unknown W, selector lock
release at submission, actual same-owner justStarted stores, preserved unready
futures, post-maximum unsupported AI, earlier submissions retained on later
failure, end/radius/start scalar order, zero-count debug tail without radius
decoding, post-store overlay failure, and the real linked/live show cvar.

Queue scheduling/storage itself is covered by root's already registered actual
predictor tests; these sequencing tests do not claim another actual world query.
Performed here: rustfmt parse/format and `git apply --check`, both passing.
Root must apply after source review, run tests and required gates, then compose
public launch/adjust/preparation/this continuation under the real Air caller.
