# Selected ordinary-Ground pre-solve driver

2026-09-20. New unregistered `physical_player_ground_pre_solve.rs`, intended child
`simulation_assembly_backend::constraint_session::ground_pre_solve`. Root owns
registration and Cargo gates. No registered edits or Cargo runs by author.

Confidence: **high** for extraction/composition of existing source-backed helpers;
this does not increase their native numerical accuracy or active-branch coverage.
Primary inventory is `physical_player_update1_corpus.rs::check_mode`, ordinary
Ground arm with `solved_fill=true`, and every called production helper. This is
not a second implementation of any source calculation.

## Contract and retained ownership

`PhysicalAssemblyWorld::try_prepare_ground_pre_solve(board, skeleton, actor,
ground, grind_policy, numeric)` consumes the same retained world/pair/Actor and
static-ground pool. Caller constructs and retains query managers, physical world
services, grind residency and all Actor/physics dependencies before invoking it.
No ground asset load, query-manager initialization, grind mount/unmount, environment
variable, fixture assertion, dt literal or solver occurs in the driver.

At entry it resolves the Actor's checked cached physical allocation and clones that
actual owner's shared `InputCvarReferences`; this is not a tuning-value snapshot.
It captures the same Actor's published AnimOutPhysIn. Existing Begin Update1 checks
the pair and bound input at its original point, including preserved earlier
manager/query/reset writes. Existing SystemLogic additionally checks the registered
Actor/pair. No new per-frame constructor is introduced.

Returns `GroundPreSolveComplete` with the retained `Update1AfterTeleportCheck`,
the actual `PhysicalPlayerReckoningPhase`, and `OrdinaryUpdate3Complete` owner,
expected displacement and state timer. It also records explicit grind policy.
These are results of completed preparation, not permission to skip solver or
post-solve work. Complete the actual current frame before entering another Update1.

## Exact extraction order

1. Begin Update1_ProcessInput82e01f98: existing manager ticks/line launches, reset,
   bound animation input processing and teleport prefix. **ProcessAnimOut is
   already here; do not call `process_bound` or standalone ProcessAnimOut again.**
2. Existing controls; prior input copy82e02430..2a28; prior output tail, body and
   wheel timers; Ground input; toolkit; previous skeleton-line surface results;
   skeleton data prefix; input-after-skeleton.
3. GrindManager call82e038ec only for explicit `RunRecoveredPrefix` policy.
   `ParkedByUser` is the deliberate first-test exclusion, never a query miss
   or a claim of complete native Update1. Residency still belongs to caller.
4. Jump timing, trajectory selector, heading, permission latches, finish input;
   actual Update2 SystemLogic with Actor; Update3 prediction-before-state.
5. Ground.Update82da0de8: Reckoning through0e94, Handplants, actual Skeleton
   GroundSkateboard at0ea8, Pumping, control through82d9a12c, Wallride througha1d0,
   fake torque, collision througha658.
6. Ordinary branch: forces througha770, manual/nudge througha7e4, force queues and
   torque througha980, hang/pinning througha998, force-based prediction through
   82da1024, Ground suffix through109c, Update3 controller.
7. `try_finish_update3_prephysics`: ordinary suffix82dfe7f0..e834, native
   PrePhysics82b37cb8/wheel queries, force application, expected displacement and
   fresh processed-dt state-timer store. Stops before the solver.

Each production call is unchanged and reports its existing typed error in a named
driver variant. Source locks around prior input/output/processed remain in the
same order. All accepted writes and leased queries survive later errors. Errors
are not transactional and the caller must not restart from the beginning.

CollisionExit is explicitly `CollisionExitPending` after the actual collision
response, before ordinary forces/controller/prephysics. It is not converted to a
successful preparation. SystemLogic may genuinely change the physical state;
the existing Ground-only continuation then rejects a non-Ground state rather than
forcing Ground or fabricating a different state's update. Existing pending jump,
teleport, manager and active-branch errors are preserved.

## Numeric policy and integration validation

`HostGroundPreparationNumeric` retains the existing five concrete collaborators:
HostReckoningNumeric, HostReckoningSinCosNumeric, HostSkateboardSpawnMathNumeric,
HostFakeTorqueNumeric and HostCollisionLineNumeric. SystemLogic uses the existing
corpus's host `angle.cos()` callback. This is an explicit desktop numerical policy,
not a new claim of Xenon estimate/reduction/NaN/denormal or transcendental bit parity.
The same source-backed arithmetic and branch kernels still execute underneath.

Author validation: rustfmt succeeded; static comparison confirms the production
helper call sequence against the existing ordinary solved-fill corpus chain.
No redundant synthetic test of the call list was added. Root should register the
child and run the existing actual-owner solved corpus through this driver, keeping
startup outside it and keeping downstream solver/post-solve assertions. Retain
diagnostic assertions separately where they describe the first-frame fixture;
do not move old category0, zero timers, teleport counters, empty lines/queues or
fixed1/60 assertions into this reusable production path. A subsequent-frame test
must complete the intervening solver/post-solve/Actor publication before re-entry.
