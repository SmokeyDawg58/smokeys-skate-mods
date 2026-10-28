# Retained Predictor queue integration

In-progress, unregistered draft: `physical_player_predictor_runtime.rs` and
`physical_player_predictor_runtime_tests.rs`. It extends the existing
`PhysicalPredictionOwner`, not a second predictor or selector. Independent raw
source review approves queue/store semantics and the later manager guard.
Tests await the reviewed ballistic query patch and registration.

## Source and ownership

- Predictor constructor82e4b6c8 retains native fixed capacity90. Existing
  `PredictorQueue` currently uses an uninhabited placeholder; registration must
  replace it with `PendingPredictionQuery`, preserving the same queue owner.
- AddQuery82e4b868 validates XYZ self-equality for position/velocity only.
  Raw VMX replaces W with X before comparing. Infinity passes; unknown W must
  survive. Radius branchb924 rejects less than0.01; unordered passes.
- Full future resetb948..b9e4 precedes capacity testba14. Overflow therefore
  preserves reset fields and acquires nothing. The source stores ready=false,
  zero position, up normals, time=-1, frame=-1, surface0, full input trajectory,
  draw flag, radius and both error ratios in order.
- Queue appendba50..ba8c retains future identity and null batch before acquiring
  the actual World trajectory batchbaa8. Batch storesbaac before Begin/Add/End.
  AddTest's boolean return is not a source branch.
- UpdateQuery82e49ae0 checks unready then nonnull batch, Syncb54, GetResult(0)b6c,
  optional full collision copyb84..bb4, Releasebc8, batch=nullbd8, ready=truebdc.
  A failed release retains copied data, batch and unready state.
- Update82e4ac68 holds queue authority, requires the actual debug overlay only
  when nonempty, visits insertion order, and clears entries only after all
  completeace4. A late failure leaves completed prefixes and full queue intact.

The host target is a weak actual physical-owner reference plus its existing
onboard future slot. It never copies futures into a parallel owner. Expired
owners and invalid slots fail explicitly. Both the physical owner and supplied
query world must share the predictor's manager authority; this prevents a
foreign world paired with its own valid DMO from accepting another player's
future. Actual query batches retain their stricter occurrence/lease provenance.

The single host simulation lane serializes batch submission with the queue lock
after acquisition so failures retain the published batch. This does not claim
native multithread scheduling equivalence. No new game task/thread model is
introduced.

## Typed data and scheduling

Existing `RetainedTrajectory` position/velocity/gravity and
`RetainedPredictionResult` collision vectors migrate to `SkeletonRootVector`.
All constructor bits remain the same known zeros. Unknown W copied from live
inputs or geometry remains unknown. Time vectors remain four known scalars.
Existing handplant fixture expectations adapt to typed lanes without changing
their values or expanding parked behavior.

Actual World batch Begin/Add/End may execute host collision jobs synchronously;
the selector future remains unready until actual scheduled Predictor.Update.
Native harvest points include SimUpdate1 8289f8f0 and SimUpdate2 8289fce8.
`try_update_pair_prediction_before_state` is board extrapolation, not harvesting.
This integration does not install schedule calls, private Launch, scoring, or an Air
completion token. Full Air runtime remains incomplete.

## Validation boundaries

Seven passing tests exercise actual query/DMO/manager/future ownership: submitted
jobs remain unready; capacity resets before rejection; exceptional outer inputs;
foreign-world rejection before reset; late query failure preserving completed
prefixes; two players sharing a slot number; empty/nonempty overlay requirements;
collision copies preserving trajectory and unready state. Test geometry is an
actual constructed empty collection, explicitly a controlled fixture.

The collision-copy unit test is not a full injected release-failure test.
Original-data launch/harvest and deterministic console traces remain required.
The ballistic patch's original numeric bypass was rejected in independent review
and replaced with the existing fallible numeric service and source refinement
graphs before integration. Workspace-predictor-1 passed4,350 units+6docs;
original-predictor-1 passed16/17, failing at the existing repeated-frame Air guard.
The seven queue tests and twelve ballistic request tests pass.
