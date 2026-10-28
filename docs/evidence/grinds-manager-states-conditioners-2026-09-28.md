# Grinds resumed: manager, states 400..404/701, Update5 fill, conditioners (2026-09-28)

Scope (2026-09-27): resume the parked grinds from the existing spline parsing and finish
them, so every spline in the city grinds as in the real game, with every value taken from the
game code or its vaults. This supersedes the 2026-09-12 grind parking.

Status: source written and statically reviewed against IDA (debug `sk82_na_m.xex`).
rustfmt PASS on every new file and on every own hunk (one own hunk in
`physical_player_system_logic_air_tests.rs` shares a pre-existing unformatted
block and keeps that block's style). NOT compiled, NOT tested.

The live facade (`selected_session_public_facade.rs`) still passes
`GroundPreparationGrindPolicy::ParkedByUser` and
`GrindManagerFillPolicy::ExcludeForFirstSkatingTest`. They flip only after the
TrajectorySelector grind chain below exists: `SelectorGrindPolicy::Native` still
returns `NativeGrindPending`, and FootPlant's CheckForGrinds still needs
`GrindData::TrajectoryBoxTest`. No hybrid native/excluded schedule is introduced.

## Update1: `GrindManager::Update` 0x82de1830

`physical_player_grind_session.rs::try_update_player_grind_manager`, reached from
the ground pre-solve under `RunRecoveredPrefix`: prefix (reset, flicker,
permission, time-in-grind, nearby query), `GrindMaterialsManager::Update`
0x82dd3178, Investigator 0x82de1718 (collision intersectors 82cfbd38 / 82cfa600 /
82cfb990), friction-vs-time 0x82dd7678, `CalcGrindGeometry` 0x82cfc2d0 with the
`GrindGeometryAnalyzer::DoGeometryTest` lines (flags 7, fatness 0) against the
world line owners, coping assistance 0x82ddbc48, Balance TargetUp / ExitLean /
ForceExit 0x82dd2168 / 0x82dd64d8 / 0x82dd26c0, Engagement 0x82ddf5f8, Controls
0x82dd3270, Jumper 0x82dcb350, then publication of the whole investigation to the
processed input. Suffix side effects (overlay lookups, 8c4 0x40000, wipeout
requests of value 0, 8cc 0x08000000) keep their order through `apply_effects`.

## State selection and transitions

- `CheckForGrind` 0x82dd3ac8: 0 when not possible or 8d0 has 0x00080000 or
  0x08000000; grind type 0..4 maps to 401, 400, 402, 403, 404.
- `CalcSuggestedState` 0x82ddbd58: Ground/Slide/Air/KnownAir return a nonzero
  CheckForGrind; grind case 300 / 502 / 200|bit / CheckForGrind or 701;
  Nonspecific case with `kNonspecificToGroundNearGrindFrames`,
  `kNonspecificCollFreeStickyFrames`, `kNonspecificCollExitFrames` (now through
  `StateChangerCvars::nonspecific_frames`).
- `GetPhysicsCategory` 0x82d83b80: 400..499 -> 400, 700.. -> 700.
- `physical_player_ground_to_air.rs` allowlist, Enter/Exit dispatch and selected
  stages for `Grind(kind)` and `Nonspecific`.

## Grind states (`physical_player_grind_states.rs`, `_forces.rs`, `_orient.rs`, `_fill.rs`)

- Enter 0x82d85098 (Boardslide 0x82d85e80 also clears E0/E1): board animation off,
  skeleton 3E41, feet collisions off, `kAngularDamping` via `SetAngularDamping`.
- Exit 0x82d8fa10: `EnableFeetCollisions` 0x82b4f4b0 (parts 19, 15, 20, 16),
  standard angular damping, board animation off, wheel spin cleared.
- Update 0x82d9b3e8: `UpdateGrindReckoning` 0x82d85298, ShouldExit 0x82d85d90,
  `UpdateSubstate` 0x82d90230 (collision force, ApplyJump/LaunchSkateboard,
  engagement velocity, per-type Update_Contact / Update_InvoluntaryExit), the
  board prediction publication, then the invExit / crouch / steering / head tail.
- `ReorientSkateboard` 0x82d9b580 with `CalcOrientationControlResult` 0x82d859e0,
  SLerp 0x82b4b418 and ApplyNoise through `SkaterMotionGraph::GetRandomInt`
  0x825fe1a0 (`actor_motion_random_int.rs`).
- UpdatePostPhysics 0x82d85e78 -> `CheckForGrindWipeout` 0x82ddf248.
- FillPhysOut 0x82d90460 and the per-type overrides; Nonspecific Enter 0x82d86710,
  Update 0x82d9bfc0, Fill 0x82d86820, PostPhysics 0x82d86768 (kDoWipeoutsTest).
- Update3 binding arms and Update5 type publication (`grind.id()` / 701).

## Update5: `GrindManager::FillPhysOut` 0x82dd3970 at 0x82e04f30

Between Pumping (4f24) and TrajectorySelector (4f3c): `GrindJumper::FillPhysOut`
0x82dc7140 stores `mJustJumpedFromGrind` (jumper+8) to Grinds+0x13E, then
investigation flags bit 27 (manager+0x1BC, `extrwi 1,4`) to Grinds+0x13F. Runs
under `GrindManagerFillPolicy::RequireNative`; the trick payload's
`GrindManagerPending` gate is removed because its prerequisite now exists.

## `PhysOutConditioners::UpdateAll` 0x82e29f88

Constructor 0x82e316f0 slot order: Intents, Grinds, MissionSystem, Audio,
Camera, Physics, Animation, Scoring, Stats, PathRecording, GameCam. Intents,
MissionSystem, Camera and Stats use the vtable at 0x82383190 whose Update slot is
the empty 0x825187c0. The port now runs Grinds before Physics and GameCam after
Animation. Audio (0x827c6848), Scoring (0x82e2b438) and PathRecording remain
unported, as before this batch; none of them is read by the grind path.

### Grinds 0x82e2f9d0 (`physical_player_conditioner_grinds_update.rs`)

- `UpdateGrindChromosome` 0x82e2ebe8: latched state category, then
  `WatchForApproach` 0x82e2e9e0 -> `ManageApproachReckoning` 0x82e2cca8
  (`mIsPreparingToJump` snapshots the ollie approach and clears the history;
  category 100 adds a snapshot with tick `mUpdates`), history
  `FiniteHistory<GrindApproachReckoning,30>` Add 0x82e2c5d8 (slot `count % 30`,
  count 59 -> 30) and RetrieveIMPL 0x82e2a250 (asserted age, oldest entry once 30
  are held), `CalcApproach` 0x82e29190 only on grind entry after more than
  `kMinApproachNonGrindFrames`, `CalcLocation` 0x82e29340 every frame.
- `SetGrindApproachReckoning` 0x82e29490: transform X, transform T, deck
  velocity, right/left foot positions, right/left foot looks, each latched.
- `GetGrindOrientation` 0x82e29838, `GetGrindTilt` 0x82e29760 (`kDeckTiltForUp`),
  `GetGrindTwist` 0x82e29610 (0.01 length gate, 0x3f70902e alignment, the two
  Newton rsqrt/reciprocal refinements already in `reckoning_numeric` /
  `simulation_drive_jacobian`) and the grind-type read are direct PhysOut loads
  without the write-latch check.
- Assertion 82e2ed28 (grindconditioner.cpp:151), candidate equality through
  `GrindChromosome::operator==` 0x82b38ff8 (all six fields), the entry frame
  count `max(kMinChromosomeEqualityFrames_Scoring, kMinChromosomeEqualityFrames)+1`,
  signed stable/scoring promotions, then the three PhysOut chromosomes.
- `NameGrind` 0x82e2aff0 into the retained 256-byte buffers; DebugDraw 0x82e2b2f8
  only asserts the DebugOverlay singleton (both text calls fold to the empty stub).
- Type, name (FastString<30>) and scorable ID come from the table built by
  `InitGrindChromosomeFunction` 0x82e25c80 and copied by
  `GrindNameTableInitializer` 0x82e2e890. The 13.5 KB initializer was emulated
  in IDA (scratch `emu_gcf.py`); all 2560 bytes of `grindChromosomeFunctionArray`
  were covered. Shipped-data quirk kept: entries 52, 62 and 72 pair `FS_TAILSLIDE`
  with `eScorableID_bs_tailslide` (20); entry 42 uses `fs_tailslide` (36).

### GameCam 0x82e2d380 -> `UpdateGrindTarget` 0x82e2d000

On grind entry the smoother error takes its zero and both values take the grind
position; a pop is a grind-type change or any change of the primitive midpoint
`(start+end)*0.5` (unordered compares pop). `PopSmoother::Update` 0x82e2a1c8: on
pop `error = (cur + (cur - prev)) - next`; then `error *= smoothness`
(`kGameCamPopSmoothness` 0.95, already constructed), result `next + error`,
`prev = cur`, `cur = result`. The result is `mCamTargetRoot`.

## Camera: `GlobalConditionData::UpdateGrindLedgeData` 0x82e350b8

Replaces the port's `PendingGrindLedgeEntry`. On grind entry onto a ledge (const
PhysOut reads, no latch): if `kInsignificantGrindSpeed^2 > |deck velocity|^2` the
previous sides are kept; otherwise the rail direction is multiplied by -1.0 when
`-1.0 > dot(dir, velocity)` (the shipped constant is -1.0, not zero), and
`left = cross(dir, ledgeHigh).y > 0` computed as the fused
`dir.z*high.x - dir.x*high.z`; right is its negation. Non-ledge entry clears both.

`GrindDetector.mIsActive` is copied each frame from `GameCameraSettings` +6,
itself copied by `CameraManager::Update` 0x82584448 from `ProfileBackEnd` +0xA3,
the Options "GrindTransparency" toggle (0x8265c8b8). `tSettings::Default`
0x825644d8 stores 0, matching the port's constructor value: the camera's rail
candidate search stays inactive by default.

## Cvars appended (registry 798..810, `GrindCvar`)

| Index | Label | Default | Range |
| --- | --- | --- | --- |
| 798 | kMinChromosomeEqualityFrames | 0 | 0..60 |
| 799 | kMinChromosomeEqualityFrames_Scoring | 12 | 0..60 |
| 800 | kbGrindConditionerDebugDraw | false | bool |
| 801 | kDeckTiltForUp | 0xbdcccccd (-0.1) | -1..1 |
| 802 | kNonspecificSideJumpScalar | 0x3f333333 (0.7) | 0..1 |
| 803 | kNonspecificReckoningSmoothness | 0x3f7ae148 (0.98) | 0..1 |
| 804 | kNonspecificCollExitFrames | 60 | 0..200 |
| 805 | kNonspecificCollFreeStickyFrames | 2 | 0..60 |
| 806 | kNonspecificToGroundNearGrindFrames | 10 | 0..60 |
| 807 | kPlayerWeight | 40.0 | 0..200 |
| 808 | kDoCollisionResponse | true | bool |
| 809 | kDoWipeoutsTest | true | bool |
| 810 | kInsignificantGrindSpeed | 0x3e99999a (0.3) | 0..0.5 |

Each default is the `defv` register of its dynamic initializer (`li r7`/`lfs f1`).

## Remaining before the live policy flip

1. TrajectorySelector grind chain: `CalculateGrindScore` 0x82dc5c38 (box of
   half-size layout+0x234 around the predicted landing,
   `GrindData::TrajectoryBoxTest` 0x82cf9cf0 with its parallel-pair filter,
   `AnalyzeAndAdjustTrajectory` 0x82dc5160 -> ConsiderGrindPrimitive 0x82dba150,
   FindBestGrind 0x82dc2d08, CalculateGrindTrajData 0x82dc4ec0, landing / vertical
   normals 0x82dba758 / 0x82dba638; score layout+0x1E0), `CalculateGrindPenalty`
   0x82db9ee0, the gate `i == 0 && pass == 1` with the 8c8 bit 30 / AI-input
   bits, the wall-ride `score < 0` argument and the on-vert bonus graph.
2. KnownAir `GrindAirAdjust` Start/active adjustment (`physical_player_known_air_state.rs`
   still returns its parked errors when `targeting_grind`).
3. FootPlant `CheckForGrinds` 0x82dbbaf0 (TrajectoryBoxTest).
4. Motion graph grind animation conditions.
5. `grind.rs` helpers are audited per function when used; where their arithmetic
   differs from the native operation order (as with `SplineEval_One`), the
   exact path is written instead, per the parking handoff's own rule.

## Files

New: `physical_player_grind_states.rs`, `physical_player_grind_state_forces.rs`,
`physical_player_grind_state_orient.rs`, `physical_player_grind_state_fill.rs`,
`actor_motion_random_int.rs`, `physical_player_conditioner_grinds_update.rs`,
`physical_player_phys_out_grinds_conditioner_output.rs`.

Changed: `physical_player_grind_session.rs`, `physical_player_ground_pre_solve.rs`,
`physical_player_physics_state.rs`, `physical_player_remaining_states.rs`,
`physical_player_system_logic_session.rs`, `physical_player_system_logic_air_tests.rs`,
`physical_player_ground_to_air.rs`, `physical_player_ground_skateboard.rs`,
`physical_player_animated_skeleton.rs`, `physical_player_update3_prefix.rs`,
`physical_player_update5_state_publication.rs`, `physical_player_update5_manager_session.rs`,
`physical_player_update5_trick_payload.rs`, `physical_player_ground_wipeout_runtime.rs`,
`physical_player_post_solve_conditioning.rs`, `physical_player_selected_ground_conditioning.rs`,
`physical_player_selected_animation_conditioning.rs`,
`physical_player_phys_out_conditioner_physics_update.rs`,
`physical_player_phys_out_conditioners.rs`, `physical_player_phys_out_construct.rs`,
`camera_selected_runtime.rs`, `input_cvar_runtime.rs`, `input_cvar_grind_states.rs`,
`simulation_assembly_constraint_session.rs`, `skeleton_runtime_state.rs`,
`dynamic_assembly_skeleton_hand_plant.rs`, `skateboard_set_standard.rs`,
`skateboard_standard_session.rs`, `dynamic_assembly_skater_forces.rs`,
`physical_player_board_forces.rs`, `grind_data_runtime.rs`,
`physical_player_grind_types.rs`, `selected_original_asset_classes.rs`.

Corrected doc: `try_launch_skateboard` (`physical_player_grind_suffix.rs`) now
states that native `LaunchSkateboard` ends with `SetVelocity` at 0x82dcb8f0 and
that the caller performs that board write because the owner has no board access.
