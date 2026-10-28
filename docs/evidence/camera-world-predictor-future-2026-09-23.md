# Camera future in the shared World predictor (2026-09-23)

`Sk8::Camera::TrajectoryPredictor::StartQuery` at `0x82e32ac0` passes its
retained `future<PredictionResults,1>` at camera `+0x10` and the existing World
`Sk8::Physics::Predictor *` at camera `+0xc0` to a predictor setup function.
The `r8` flag comes from `Iris::PathEvaluatorInput::mIsInAir` at evaluator
`+0x39`. True selects `SetupFakeBodyPrediction` (`0x82e4bb88`), which supplies
the World predictor's gravity at `+0x30`; false selects
`SetupLookAroundPrediction` (`0x82e4bbc0`), which supplies a zero vector.
Both receive error ratios `1.0f`; the caller sets the draw argument to zero.

`Predictor::AddQuery` (`0x82e4b868`) clears ready and resets the result before
the fixed 90-entry queue capacity check. It appends the address of the retained
future, then acquires a World trajectory batch and runs Begin/AddTest/End.
`Predictor::Update` (`0x82e4ac68`) processes entries in insertion order and
clears the queue after all entries finish. `UpdateQuery` (`0x82e49ae0`) checks
ready and batch state, syncs, copies result zero if present, releases the batch,
then sets ready. A miss retains the reset collision time of `-1.0f` in every
lane. No camera collision result is manufactured by this queue extension.

`Iris::PathEvaluator::UpdateStaticEvaluator` (`0x82e3c880`) submits only in
state zero, stores state one and the input in-air flag, and polls the same future
on a later evaluator pass. On completion it converts negative collision time to
`FLT_MAX`, sets valid and the saved in-air output, and returns to state zero.
It does not submit another query in that completion pass. The surrounding
`PathEvaluator::Update` (`0x82e46420`) resets collision time and valid first,
updates static before dynamic, and copies last-valid time only when valid.

The selected native `Simulation::SimUpdate2` calls camera Update and Wait at
`0x8289fc88` and `0x8289fcb0`, then the same World `Predictor::Update` at
`0x8289fce8` when skater update is enabled. Rust now routes a camera weak owner
through the existing World queue and harvests it at this post-camera point in
the selected flags3 profile. The earlier `SimUpdate1` harvest remains before
physics. The camera submission helper drops the camera lock before the queue
resets the retained future, avoiding recursive locking.

IDA layout/control/scheduling confidence is **high**. Host floating-point bit
parity remains **medium** without a PPC per-tick trace. The camera path
evaluator, obstacle catalogs and avoidance cvars remain separate integration
work; queue readiness by itself does not complete active avoidance or certify a
playable camera frame. The focused shared predictor tests include mixed
player/camera queue ownership, readiness timing, the native miss sentinel, and
the grounded/airborne gravity selection.

Validation: `cargo test -p skate2-core
camera_queries_share_world_queue_and_publish_only_at_predictor_harvest --lib`
passed (1/1). The freshly built test executable passed all eight
`prediction::runtime::tests` after the camera route was added. This verifies
retained queue behavior on the host; it is not an original-runtime trace.

## Static PathEvaluator prefix

The retained Rust `CameraPathEvaluator::try_update_prefix` now follows
`PathEvaluator::Update82e46420` through the static evaluator. Every call first
writes `FLT_MAX` to current collision time and false to valid. In state zero,
`UpdateStaticEvaluator82e3c880` submits the input position/velocity/radius/
lookahead and in-air flag, then writes state one and saves the in-air flag. In
state one, an unready camera future leaves this pass invalid and queued. A ready
future is read at `PredictionResults::mCollisionInfo.t.x` (future base `+0x30`),
negative time is converted to `FLT_MAX`, valid becomes true, state returns to
zero, and output was-in-air takes the saved flag. The scalar `fsel` at
`0x82e3c924` selects the candidate when current-minus-candidate is nonnegative;
the Rust branch retains that ordering for ordinary floating-point inputs.
Signed NaN/estimate bit parity is not established without a PPC trace.

The method then returns an explicit `PendingDynamicEvaluator` error. Native
dynamic obstacle evaluation (`0x82e45f00`) and the final valid-only copy to
last-valid time occur after this prefix and are not inferred from an empty
obstacle list. The focused host test uses a real empty World trajectory batch:
`cargo test -p skate2-core
static_path_submits_waits_then_consumes_retained_no_hit_future --lib` passed
(1/1). It verifies submission, waiting without resubmission before harvest,
miss sentinel consumption on the next pass, retained in-air output despite a
changed input flag, a later grounded resubmission, and no premature last-valid
copy. The freshly built binary also passed the eight predictor runtime tests.
