# Connected skating continuation, 2026-09-24

## September25 moving collision collection admission

Resumes this formerly deferred work. Solo, no compilation or runtime replay.
Code formatted; cvar declaration/registration permutation statically checked.
The existing terrain corpus now covers spawn-neighbor loading, crossing east and
returning without new registrations; this added test is NOT executed yet.

Found gap: SelectedSessionTerrain retained one manually selected resident
collection, and live ticks never changed its focus or requested other collections.
Existing streaming.rs and formats/stream.rs already implement focus movement,
membership and non-XML spatial selection. Reused those, resident loader, normal
arena adapter, environment router, mesh/grind services and collision pool.

IDA evidence:
- Sim_LocalPlayerStreamerController::UpdateStreamer8277d4d0: offline and manual
  camera-off gates, get local Actor, Actor::GetPosition, MoveStreamFocus with
  unlocked/flags255. This change targets the existing offline controlled profile.
- Actor::GetPosition825f9df0: PhysOut pointer -> mSkeleton+14 -> vector at1a0.
  PDB PhysOut_Skeleton shows mSkeletonRootMatrix at170, so1a0 is its translation,
  NOT hips40 or camera/deck position. Live integration reads this exact published
  output after the first completed frame, before next physics query borrows.
- MoveStreamFocus824d8250 checks kDisableStreamFocusMovement and rejects NaN.
  Existing StreamFocusState::move_to carries original824d78b8 lock semantics.
- AddStreamFocus824dacb0: simulation/presentation radius195; existing formats
  constant reused. FindCollections824df068 has non-XML hull AABB overlap and XZ
  distance ordering; existing selector retains LOD0 filtering and halfheight8000.
- AssetStreamSystem::FindCollections824dfea8 caps offline non-explicit-XML Sim
  membership by kSimMaxFocusCollections. New live refs:519 movement disable
  initializer82f4e168/table82fd0364 false;520 cap82f4e310/table82fd0378 int4[0,9];
  521 XML enable82f4e3c0/table82fd0380 true. Texture329/table82fd0368 stays between
  519 and520. All522 definitions appear once in REGISTRATION_ORDER.
- Simulation ReceiveRegCollection8277e988 walks arena entries forward, normal
  Simulation then Location, followed by DMO pass. Existing selected-profile
  normal adapter is reused; this does not newly admit parked Location/DMO work.

Implementation: SelectedResidentStartup now accepts sequential additional IDs
after previous completion, retaining prior collection holds in the SAME registry.
SelectedTerrainStartup appends completed plans/tokens to its SAME adapter. The
source XSS is taken from the catalog owner of the startup simulation collection.
IDs already requested are not loaded again. New registration failures retain
their prefix and terminate this selected session; no retry or fabricated success.
An overlapping arena ID is explicit pending ownership, not silent duplication.

Limits and confidence: high for Actor position and native constants/cvar reads;
reuses existing approximate host distance math and serialized-hull traversal.
Full outer native load/unload scheduling is NOT connected. Loaded regions remain
retained, bounded by original ground capacity128 and available memory; no claim
of unlimited map traversal. XML data present with enabled XML selection stops
explicitly rather than silently using wrong membership. Only the selected base
world is queried, with no additional-world merge. No native streaming stall or
teleport barrier is added. Renderer remains its existing preloaded static map;
dynamic camera trigger roster refresh is not added. Build/gameplay pending.

## September25 seven-error batch: grabs, flip updates, manual flag

Source checked; compilation and controller validation remain
pending. No runtime replay or build started. Confidence high for recovered routing
and flag semantics; no claim of complete manual gameplay parity.

- Action2190400336 =828edb50, Regular_163 folded update: all four states160..163.
  Calls LeftGrab828ed890 -> OnGround ancestry; then packed36 ConstMGIntent and
  packed37/38 CreateMGIntentFromAGIntent. Exact39-slot lists use existing dispatch.
- Motion2190418328 =828f2198, LeftGround_600: all48 matching native descriptors.
  Calls shared flip828efae8 (Trick828ee4e0/OnBoard828edf40), then PlayAnimation28.
  Unlike previously admitted828f2138, there is no SetTrickHeight29. Reused existing
  scoring/attribute/animation callbacks and exact29-slot checks.
- Motion2190405904 =828ef110, StartAnticipation_90 and state25: OnBoard then
  AttachIntent20 (occurrence93). Both21-slot lists admitted without duplicate ancestry.
- Motion111: Pre8290db3c reads HasIntent191 (sConditions+2fc); Sustain switch
  entry at8291e254+111*4 selects default829221c8, true with no read.
- Motion244: Pre82911074 and Sustain82920120 both read PhysicsSpeedCompare352
  (sConditions+580). Existing shared condition implementation reused.
- Action99: Pre82904b10 is c40 && c41 && c36 && c38 && c39; Sustain829098c8
  omits c40. Raw branch targets, condition loads and final true/false tails checked.
- Callback1929 address2194275536 =82c9fcd0, IsManualing.Begin: checked cached
  ISkaterMotionGraph, virtual+60, argumenttrue. End82c9fdc8 uses the same checked
  owner with false. Setter825fe0a0 updates only bit28 in Motion+ca4; implemented
  through existing with_grounded_motion_state owner checks and flag known-mask.
  Null CreateInstance82bb3da8 already handled; Update825187c0 remains empty.
  Existing manual-out timer and physical-state consumers read this retained flag.
- Native Action table821766b0 and Motion82195fa0 audited: all54 relevant state
  addresses, ancestry occurrence ranges and newly admitted callback lists agree
  with existing Rust declarations; exactly one new dispatch arm per state.

No original physics tuning, pushing, steering, or force calculations changed.

## September25 Action4 / Motion828f13a8 / Motion828f2138 follow-up

Source checked and connected. No compilation or runtime replay.

- Action expression4: Pre828ffce8 and Sustain8290628c both load condition17
  (sConditions+0x44), call IsMirrored, cache the positive condition result and
  return its negation. Added `(_, 4) => !c(17)?` to the existing selector; the
  checked Action mirror adapter already exists. Raw fresh/cache branches verified.
- Motion828f13a8 (decimal2190414760), ToDBLGrab655/666: calls shared ancestor
  828eedc0, which expands OnBoard828edf40 through packed19, then empty ForcePhysics
  Update20, AttachIntent21 and CreateAttribute22. Four empty HandBusy/EndGesture
  Updates23..26 precede PlayAnimation27. Both occurrence lists admitted.
- Motion828f2138 (decimal2190418232), LeftGround: calls shared 828efae8, which
  expands Trick828ee4e0/OnBoard once, then ScoringTrick23, empty SetTrickAttr24,
  CreateAttribute25/26, empty IsDoingTrick27. Leaf updates PlayAnimation28 then
  empty SetTrickHeight29. Raw offsets0xe0/0xe8 and branch order checked. All48
  native state descriptors using this address admitted with their own exact lists.
- Enumerated native Motion kStates82195fa0 (1354 records, stride32), parent+8,
  behaviour count/index+16/+18, callback+24. Behaviour sections are contiguous
  occurrence ranges, not index-array lookups. Static audit compared native ancestry
  to existing Rust declarations and each new match arm for all50 states, passed.
- Existing lifecycle dispatch already handles these payloads, including actual
  PlayAnimation and ScoringTrick updates and Begin/End work for empty Update classes.
  No new physics/animation implementation and no blanket success fallback added.

Confidence: high for expression and callback routing; gameplay testing remains
pending. These changes do not claim completion of other unimplemented graph paths.

## September25 five-error batch (code ready, uncompiled)

No compilation or runtime tests. Formatting
completed and static declaration audit checked all36 shared Action states.

- Motion Pre370: raw jump table8290aa90 selects829170e8..7290 with shared
  tail82917090..70e4 and8291245c..247c. Exact lazy expression is
  (c49 && c46 && !c48) || (c47 && c46 && c48), including repeated cache reads.
  Added shared IsMirrored condition48 dispatch to existing checked ISkaterAnim
  reader; loader uses Action ConditionClass::IsMirrored/Plain, not Motion Plain.
  Activation82c9a210 verifies that owner then queries native mirror getter.
- Pre1108290dadc..db38 reads sConditions+2f8 =>190 HasIntent, direct positive
  cached result. Existing HasIntent runtime reused.
- Callback50882cb2db8 is SetTrickAttr Begin. Native null CreateInstance82bb3da8
  and empty Update825187c0 preserved. Begin sets construction key Trick to
  authored FastString30 mTrick via IAnimatable+64. End82cb2ea8 requires that
  owner, queries cached defaultcyc, removes Trick only if present. Existing
  construction map and required tree-attribute lookup reused; full lifecycle
  wired, not merely the failing Begin call.
- Transition1649: native kTransitions821a08e0 stride16 has one hook at index
  section17. IMPORTANT: kIndexArray82191708[17] ==16, NOT hook17. Hook16 is
  OverideNextAnimTransitionHook with authored blend/time0.2. Enabled this
  transition in existing source-backed adapter82ca6be8 -> NextAnimTransOveride
  825fdf20. No MongoPushToAntic channel behavior was added.
- Action828edc40: original kStates821766b0 identifies36 shared states. Native
  calls828ed980 (OnBoard then four transfers19..22), then Const slot23. Previous
  dispatcher admitted only138. Extended it for the native ground/air families,
  preserving exact occurrence lists and 24-slot order. Static audit confirms
  all36 have the four transfer classes and final const class. No new intent
  implementation; original authored names/values retained.

Confidence high for inspected branches/callback order. No numerical parity or
runtime success claim until compiled and tested. build-release.bat remains the
build entrypoint. No active builds, tests, live windows or agents started.


## Offboard Still752 update dispatch

Code-only fix plus a reusable build-release.bat.
Error2190417448 is828f1e28 StateUpdate_Still_752.
Fresh IDA confirms OffBoard828eddc8 first (Motion root0..3, then4..13), followed
by CreateAttribute packed14 and PlayAnimation packed15. Original declarations
map these to Standing308 occurrence755 and Still752 occurrence1430. Intermediate
OBGround/Locomotion/Idle ancestors have no behavior occurrences. Added exact
state/address -> [0..13,755,1430] in existing checked lifecycle dispatcher,
reusing existing callbacks and active-set validation. Confidence high for source
order; compilation/runtime unverified. Added build-release.bat at
repo root: release skate2-pc, -j1, working-directory independent, result/pause.

## Offboard PendingExpression279

Raw Pre jump-table279 targets8291218c..82912304. Condition pointers at offsets
AC/B0/B4/B8 identify43/44/45/46, all existing HasIntent callbacks. Original
branches implement (c43 && c44) || !c45 || !c46 in that lazy order; true return
829122f8, final false via829122f0/f4. Added only missing expression. Source
confidence high; release63948 exited0. Runtime tests pending
(release-only workflow). Log offboard-expression279-release-20260924.

## Offboard PendingExpression368

Same source-check/release-only workflow. Pre jump table
8290aa90[368] targets82916c74. Raw branches return true at829122f8 when
LocomotionValid38 is false; otherwise return false at82916ed4 on any of
IsDismounting39, IsLanding11, IsRunningOut12, IsLocomotionQuitNotAllowed42.
If all false, HasIntent40 false returns true; else HasIntent41 false returns
true, true returns false. Added !(c38 && (c39 || c11 || c12 || c42 ||
(c40 && c41))), preserving order/laziness/cache through existing condition
implementations. This is the complement of existing expression445. No new
condition implementations or tuning. Release27479 exited0; runtime testing
pending. Confidence high for inspected branch equivalence.

## Offboard idle PendingExpression8

Source check, minimal fix and release only; runtime testing is
pending. Raw Pre8290aa50 jump-table case8 targets
8290b46c. It reads sConditions+0x64 (index25), calls IsGrabbingObject, caches
and returns its positive result unchanged. Activation82c9aef0 reads actual
PhysOut.OffBoard+0x38; the existing shared condition loader/runtime and Actor
accessor already match that path. Added only expression8 => c(25), preserving
the existing cache/parent-context machinery. No duplicate condition or hardcoded
false. Source confidence high; runtime acceptance pending. Release73973
exited0 in4m21s; log artifacts/offboard-expression8-release-20260924.log.

## Held push versus short press: corrected Vault keys

In playtesting, holding either A or X looked like weak rapid-tap pushing;
stationary turning now feels correct and is preserved. Single-agent source
review found three reversed 32-bit halves in native 64-bit attribute keys:
holding acceleration, first-push teleport window, and push top-speed limit.
See motion-push-cycle-2026-09-23.md for raw instruction addresses. Original
mounted data resolves the corrected settings to4.0,0.5seconds,8.5 respectively;
the old holding key is absent and returned0. No curves or force tuning changed.

Original-data held-A trace previously set PushState.dv to0 during Motion1282,
and its strength blend reached0 at tick92. Corrected ticks72,92,112 retain
dv4 and strength1. Short-A still gives dv3.4919 and strength0.7555 at tick52,
versus held-A dv4.5/strength1. Held-X also retains dv4/strength1 in Motion1283.
These are native-port traces, not captured console parity traces.

Held-A, held-X, short-A480tick replays PASS. The >90degree turned-A route passes
held-strength/acceleration assertions, then fails during neutral continuation
at frame457, Motion PendingCondition337. This later integration boundary remains
unimplemented; do not call that entire replay passing. The regression now rejects
zero strength during held PushCycle; the original-data accessor test rejects the
old reversed key and asserts the real4.0 cap. Logs: held-push-attribute-20260924.log
(before correction), held-push-corrected-replays-20260924.log. Workspace/release
validation PASS:5014 core,214 formats,53 PC,3 symbols,6 doctests; fmt PASS.
Release build4453 exited0 in4m12s, executable updated23:15:18. Strict Clippy
was not rerun; previously documented repository backlog remains. No live launch.

## Controller feedback: push velocity lookup and wheel axle mode

Release handoff completed: optimized build75496 exited0 in4m03s. Runnable
target/release/skate2-pc.exe uses --live-skating and external SKATE2_GAME_ROOT.
Final workspace, formatting and test-target compile check passed. Strict
Clippy remains failed on the repository backlog. No interactive window was
launched during this handoff; controller feel comparison is still the
next check. Collision coverage/long mixed landing remains unresolved below.

Final focused validation: all four480tick pushing routes PASS, including
sustained A after fixing reached hand/body reuse boundaries; full-left165tick
heading/axle regression PASS. Core5014 PASS. Hand same-buffer segmented jobs
and repeated-compaction producer ledger are documented in
tree-eval-reuse-boundary-2026-09-23.md. Logs repeated-reuse-*gate*-20260924.log.
Long mixed landing remains failed as recorded below; final release gate is
tracked in STATUS. No hands-on claim of original-game feel yet.

Two concrete source mismatches were found in the connected path, without
retuning force, steering sensitivity or the fixed simulation timestep:

- `GetPushAttributesForAnim`82cb6d98 compares packed names. The port compared
  mixed-case literals against lowercase decoded names, leaving all eight
  push clips with zero begin/end velocity. Existing FastString30 identity
  comparison restores authored values. Four original-data push routes passed:
  shortA, heldA, heldX and heldA after an actual turn exceeding90deg.
- `CreateJoints`82b2de84 loads wheel axle mode3 and df0c/df78/dfdc/e048 store
  it into the four wheel limits. The port supplied4, which the existing solver
  treats as free swing. Corrected the constructor and its field regression;
  see skateboard-joints-source.md for assembly and field offsets.

Full-stick original-data replay, unchanged input script and tuning: heading
change from steering tick7 to25 increased1.54deg ->14.13deg. At tick25 wheel
axle deviations from their truck axes fell from6.5..63.8deg to1.3..1.4deg.
These are port before/after measurements, not an Xbox runtime trace or proof
of numerical/feel parity. Core5013 PASS49ignored. Both partial/full steering
stages pass, but later ollie landing fails with the board below the map; the
continuous mixed regression is still failing. Collision coverage is deferred.
A focused165-tick full-stick regression now checks heading and wheel
alignment without continuing into the farther-away trick landing.

Validation/release status is maintained in
artifacts/push-feedback-investigation-20260924.md. The v24 gate below predates
these corrections; do not treat its complete mixed pass as current validation.

## Controller handoff gate v24

Release build completed successfully, exit0 in4m10s. Runnable executable is
`target/release/skate2-pc.exe`; no interactive test launched during this handoff.

Current safe fix, then a runnable release; expansion deferred.
Workspace v24 exited0: core5012, formats214, PC53, symbols3 and compile-fail
doctests6 passed. Mixed original-data1083 replay passed103.97s after all integrated
changes, including recovery and subsequent push. Logs end in v24-20260924.log.
No numeric mismatch was introduced in the recorded mixed stage outputs versusv22.
Three old corpus owner-path assertions were updated after the completion token
was simplified. Release build in progress. Remount implementation was not added;
parked source evidence documents remaining Motion34/112/113/114/317 admissions.
A0x1000 maps RightPush, X0x4000 maps LeftPush through action_graph_input_fill;
both use Pushing. DpadUp in the existing A replay is extraneous gesture input,
not a push prerequisite. X-alone physical replay has not been executed.

## Latest validation and integration

Core v22: 5008 passed, zero failed, 45 ignored. Continuous original-data mixed
replay v22: PASS, 1083 ticks, 107.23s debug harness. Stages cover actual pushing,
steering, held ollie (rise0.68717575), kickflip (rise0.2626419), grab
(rise0.33622742), bail/recovery and recovered pushing. Log:
`artifacts/astra-mixed-controls-v22-20260924.log`.

After that binary was compiled, integrated visual-island query flags, source
UpdateLocomotion82ca9bf8 second-future producer and Ground collision-success
continuation. Both new packets passed separate source review by motion agent;
root also checked the collision epilogue and unconditional caller suffix in IDA.
Proof packets are `artifacts/astra-locomotion-mid-proof.md` and
`artifacts/astra-ground-collision-exit-proof.md`. No duplicate leaf math or
fabricated ready futures. Workspace v23, walking420 and updated mixed replay
are the next gates. No new release binary. Strict Clippy is not passing.

This record distinguishes recovered source, integrated runtime, and executed
checks. The release executable has not yet been rebuilt for this batch.

## Current source integration

Full CreateLocomotionTrajectory82cac998 and cvars474..486 are integrated after
root independent source review. Final physical-action lookup reuses the existing
physics_action_probe, preserving negative+4 / nonnegative%4 foot normalization.
Existing470..473 accessors were preserved against the staged patch's accidental
removal. CalcAction82c8ae10 and495..505 passed independent raw/cvar review by
astra_today_review. Full UpdateLocomotionTrajectory82cb7d48 and Actor phase1
passed root review: actual futures, twelve-stage attributes, crossed normal
arguments, partial writes, ordered floating branches, actual base animation speed.

Motion778 and Actor phase3 passed astra_today_review's independent review after
correcting physical-interface validation timing. Actual StepSelector Launch runs
with Actor/animation/PhysOut guards released and uses the selected world's queue.
Impact's native Attribute+35 assertion checks scalar type, not normalization;
PDB and Init82d75200 confirm this. Existing typed scalar access checks it.

OffboardLocationTest and viability passed astra_motion_batch independent review,
including a corrected radius/count tuning-read order. Real line and trajectory
leases use existing world query machinery; result consumption and state exit
release them. General owner destructor cleanup is not newly reconstructed.
All these packets are integrated, with dense live cvars through518. V21
compilation found seven Location adapter API errors; fixes reuse the proper
constraint-session module, existing dual-trait numeric service, and explicit
trajectory flags. V22 compilation is underway. The next walking/mixed-control
replays are still pending.

A proactive source audit found the missing second-future producer:
SkaterAnim::UpdateLocomotion82ca9bf8 launches input1/result1 after the first
StepSelector harvest. Constructors correctly initialize both futures unready,
and actual startup.ini does not disable FutureAssert. Do not change readiness,
count1 or the cvar to conceal this gap. The mid-physics producer and native
scheduling continuation are being recovered. World query visual-mode override
also has a staged correction, preserving the constructor snapshot rather than
resampling mutable backend state.

Core v20 (before Update/Motion/Location):4995 passed,1 failed,44 ignored. Sole
failure was the existing test expecting one extra Vault Arc; there are now two
real instances. Assertion corrected; not yet rerun. New CalcAction, Create
attribute and tuning tests passed. Logastra-connected-core-v20-20260924.log.

## Executed checks

- V18 physical flip-family replay:1 passed in33.77s, four fresh sessions.
  Actual Part6 rises/minimum deck-Y are Kickflip .19981766/-.96075416,
  Heelflip .28969955/-.9241042, N_Kickflip .27420807/-.9969025,
  N_Heelflip .28144073/-.9617785. Each reaches takeoff, authored Out,
  physical Air and Ground landing. Log `astra-physical-flip-families-v18-20260924.log`.
- V16 strengthened crouch comparison:1 passed in61.86s; measured short/held
  Ollie rises .25219727/.6287956, Nollie .29550552/.6885414. Log
  `astra-crouch-comparison-v16-20260924.log`.
- V18 workspace reports every test suite passed: core4989 with44ignored,
  formats214 with1ignored, PC53 with3ignored, symbols3, six compile-fail
  doctests. Shell wrapper returned1 despite no Cargo/test error in complete
  log; future commands explicitly propagate LASTEXITCODE. Do not erase this
  harness discrepancy. Log `astra-connected-workspace-v18-20260924.log`.
- V18 Y/dismount replay passes former hook-correction tick60 and stops at100,
  first walking input, in PendingLocationTestStartQuery. Action now owns full
  location-query source recovery. Full walking scheduler is not yet admitted.
- Formatting applied workspace-wide. Strict Clippy v19 failed with910 core
  diagnostics (primarily documentation/style and unfulfilled lint expectations);
  it has not passed. Initial Andale documentation lint was corrected.

- V15 original-data grab/bail batch:5 passed,1 failed in85.64s. The strengthened
  bail replay completes480 ticks and ends in Ground; actual turning and all
  three grab checks pass. Newly added Y/dismount/walk/remount probe stops at
  tick60 in existing PendingOffboardHookCorrection (physical OffBoard), before
  its walking input. Reviewer owns that source suffix now. Log:
  `artifacts/astra-grab-bail-v15-20260924.log`.
- Core v15:4968 passed,1 failed,44 ignored. Remaining camera title oracle
  misplaced EnableAirCameraCollision; native section ordering puts it after
  SkaterAvoidance. Corrected. V16 compiling with walking producer prerequisites
  (independent Motion review) and physical trajectory consumers (root review).

- Core v13:4964 passed,1 failed,43 ignored. The failure was the camera cvar
  title-order oracle missing three newly registered labels; corrected before
  v14. V14 exposed one typed-vector fixture/access migration in 504 feet IK;
  corrected to XYZ height access. V15 is compiling; no passing claim yet.
- Grab/bail v13 reported5 passes, but inspection found its old bail test
  accepted a failure after entering teleport. The replay actually stopped at
  tick259 in an obsolete known-lanes board-transform adapter. The test now
  requires480 completed ticks and actual Ground recovery; the adapter now
  reuses the existing typed SetTransform engine. Independent Motion review
  checked82b35ae0/82b34a90 and actual pool/hook ownership. Replay is pending;
  the old five-pass count is not evidence of complete bail recovery.

- Core v10:4953 passed,43 ignored. Includes active board retrieval and exact
  angular-displacement kernel; log `artifacts/astra-connected-gameplay-core-v10-20260924.log`.
- Original-data v8 selected facade:5 passed in151.53s. Short/long crouch for
  ollie and nollie, four short flip families, fresh ollie,480 pushing ticks,
  visual publication. Log `artifacts/astra-connected-crouch-camera-v8-20260924.log`.
- Actual actor/channel startup v10:1 passed in1.84s. Includes existing-channel
  Fakie transition and pool reclamation. Log `artifacts/astra-channel-transition-corpus-v10-20260924.log`.
- Actual RunSequence/Selector asset construction and sequence attributes v12:
  1 passed in2.33s, log `artifacts/astra-sequence-corpus-v12-20260924.log`.
- Long-held kickflip v12:1 passed in25.15s, including the complete hold and
  release loop. Earlier v10 stopped at Camera.UpdateTransferAngle at step44;
  before active-controller integration it stopped at controller state2.
  Log `artifacts/astra-transfer-longflip-v12-20260924.log`.
- Core v12:4957 passed,2 failed,43 ignored. Both failures were test setup:
  missing cvar link in a new vector regression and missing transfer-cvar title
  in the ordering oracle. Both are corrected, awaiting the next build gate.

These execute original assets through native host code; they are not matched
per-tick Xbox reference traces and do not establish bit-identical physics.

## Source ownership and review

Controller.UpdateState82dd18e0/Update82dd1d68 now dispatch the actual retained
free, hiding, returning and animated states. InitRetrieval82dc77d8,
Hiding82dc7b68, Retrieval82dc7cf0, AutoRetrieve82dc7e20 use original attributes,
actual Processed data and existing board setters. Existing Hold is shared,
not reimplemented. Root independently checked the author's source packet.

Raw Skeleton animation getters82b45f58/82b3d498 use the saved animation record;
non-Raw82b3b878 uses current targets. Fakie calls TransitionTo on its own channel
and keeps the channel's actual transition owner and clock. Sequence9 and
Selector10 use original authored RunSequence children and runtime sequence IDs.
The native positive Selector transition assertion remains explicit.

LetGo82dd03e8 calls ComputeThrow82dcca10 and Righting82dc7eb8, queues force6,
then issues three Exact angular-displacement calls. Root implemented these
and the separate Exact82b36518 kernel; Action independently verified raw VMX128
operands, full64-bit attribute keys, reciprocal order, physical owner and
force publication order. Source really calls ATan for throw scale. It is not
replaced with a guessed tangent. The new throw path still needs runtime coverage.

Camera.Transfer82e41040 and AirCollision82e3ec80 reuse existing angle and actual
world-query services, with live debug cvars and original height-blend stores.
Root independently reviewed both packets. Source-default air collision is
disabled by its actual cvar initializer, not by a fabricated query result.

Walking vector migration retains unknown W in the existing typed owner rather
than adding a shadow cache or replacing it with zero. Reviewer independently
checked source arithmetic/publication order. Physical504 feet/root children
and full504 lifecycle are integrated after root source review; Actor walking
producer/consumer work remains in progress. Grabbable-object query gaps remain
explicit. No claim of completed offboard walking is made.

## Remaining validation

Compile the combined camera/504/throw batch and correct real failures. Run
original-data connected replays, including offboard/remount once its complete
producer path is admitted. Complete the repository workspace checkpoint before
publishing the next controller build. Preserve verified fixed1/60 timestep and
performance fixes; see `live-performance-fix-2026-09-24.md`.
