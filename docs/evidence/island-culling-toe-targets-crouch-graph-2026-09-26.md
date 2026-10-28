# Island pipeline culling, SkaterSkater refutation, toe-target trace, crouch-graph correction (2026-09-26)

**Status:** rustfmt parse PASS on every touched hunk. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Goal 2 (off-board hover): Island culling table was missing from the live SkaterGround job

- **Native.** `Island::InitCullingTable` 0x827b8c10 builds a 19x19 `rw::BitTable` named "culling%d" (by `mFlags`). Both the Island ctor (0x827bad10, right after InitWorld) and `ChangeFlags` (0x827ba5c8) call it. It then stores the same table into every Island pipeline's `mCullingTable`, including SkaterGround and SkaterSkater.
- **Native ground job.** `AssemblyGroundCollisionJob` Run 0x827cd2b0 reads the table from job+0x28 and tests bit (columns * VolumeData+0xD4 + groundInfo+0x14). A set bit skips the volume. The ground group is `GroundPoolParams.collisionGroup` = 0: World ctor 0x827b1a00 stores r28 = 0 at params+0x20. The 0x12 in that block is `stepSelectorBatchCount`.
- **Port bug.** The live flags3 frame passed `pipeline_culling: None`, behind a facade `no_pipeline_culling` gate. With no table, no volume was ever culled against static ground.
- **Effect.** `SetOffBoardCollision` 0x82b4f038 gives userdata 0x11 (LightDMO-only) to the feet and lower legs (parts 0xF, 0x10, 0x11, 0x13, 0x14, 0x15). It gives 0x12 (LW-only) or 0x11 to the core parts (6, 0xA..0xE, 0x17). Native culls (17, 0) and (18, 0), so off-board feet, legs and core never touch static ground, and only the root proxy (userdata 6) does. In the port they did, pushing the off-board skeleton up.
- **Unchanged groups.** The on-board groups are not culled against ground 0: skeleton 6, board 4, ragdoll 7, world-only 3 and board world-only 2.
- **Table words.** They were derived by emulating the raw PPC (0x827b8d74..0x827ba2dc) with Island.mFlags = 3 and columns = 19. The transcribed set/clear sequence reproduces them exactly for both flags 3 and flags 1.
  - flags3: `fffe2502 fd7fffbf 4186efff 60fbfeff 2fc305d8 ffe077fd 77fc09cb 97ebbfec c88cbbef a32ea665 fff3df5c 000001ee`
  - flags1 differs only in words 2..4 (the 4/6/7 self pairs).
- **Port changes:**
  - New `simulation_island_culling_table.rs` (InitCullingTable create branch plus `rw::BitTable::Initialize` 0x82bc3308: (rows * cols + 31) >> 5 zeroed words), with word-exact tests.
  - `SimulationAssemblyConstraintSession.island_culling` is built in the ctor from `island_flags`.
  - The flags3 schedule of `try_step_authored_ground_with_locomotion` now uses `input.pipeline_culling.unwrap_or(&island_culling)`. Isolated component probes keep their explicit input.
  - `SelectedQueuedFramePolicy.no_pipeline_culling` and its Pending gate were removed (facade, app, corpus).

## Goal 1: SkaterSkater feet-vs-deck theory REFUTED

- The SkaterSkater `AssemblyAssemblyCollisionJob` 0x827cc410 skips a pair when the Island table bit (columns * A.group + B.group) is set (0x827cc76c).
- InitCullingTable sets (4, 6) and (6, 4) unconditionally (raw words 0x827b9030..9058 and 0x827b905c..908c). Board (group 4) versus skeleton (group 6) is therefore always culled. Feet never collide with the deck in native, so that pipeline is not the missing piece.
- The pipeline only matters for self pairs and other skaters, so it was not ported.

## Goal 1: further native-identical items (do not re-audit)

- **DangerZone frame-2 tie.** `PhaseBlend::AddTime` 0x82d6bd48 uses `fdivs dt/L` then `fmuls * L1`. Both are correctly rounded singles, the same as Rust f32, so the F+1 `_A` time 0x3d088888 equal to beginTime 0x3d088888 is identical in native.
- **Frame order.** `SimUpdate1` 0x8289f410 runs UpdateAnimation1/2/3, then UpdatePhysics, then World::Update1. `ProcessAnimAttributes` 0x82b59de8 is reached only via `Skeleton::ProcessData` from `Update1_ProcessInput` and reads `mAnimOutPhysIn->attributes` (GetPhysUpdateData). The port's actor prefix order matches.
- **Pop-path foot collision disables.** None exist in native: the Disable*FootCollisions callers are KnownAir (grind targeting only), Boneless, FootPlant, Grind, HandPlant, OffBoard_Pushing and Skitching.
- **Ground trick.** `ground_trick` is the physics-state category 600. At F+1 the state is KnownAir, so it is 0 in both.
- **Already ported.** `Skeleton::UpdateGroundSkateboard` 0x82b5e468 is `try_update_paired_skeleton_grounded_live`; the handplants "not completed here" comment was stale and is fixed. `UpdateAnimatedSkateboard` 0x82b5bc58 callees (UpdateRootTransforms, ApplyBoardAnimation, GeneralUpdate) are ported.
- **Remaining unknown.** The toe world position during the GroundAnimation jump-into. In the failing capture the toe contact is at -6.25 cm at the pop frame's collision and -0.45 cm at F+1, and no toe contact occurs during the manual itself. New opt-in `SKATE2_TRACE_TOE_TARGETS`, set in `play-known-air-trace.bat`, prints the drive-target translations of volume 0 and parts 15/16/19/20 each frame. It shows whether the anim target itself is below ground (placement) or the body lags an above-ground target (drive/solver).

## Goal 3 correction: the crouch graph is speed-dependent

- `anim_motion`/crouching `crouching_max_height` (layout +0x190, x at +0x1A0, y at +0x1C0): x=[0, 8, 11, 14, 17.44, 20.80, 24.44, 28], y=[0.95, 0.95, 0.95, 0.95, 0.875, 0.729, 0.604, 0.567].
- Native therefore lowers disttocog (auto-tuck toward the speed_tuck branch) above 14 m/s. The previous doc's "flat 0.95" was wrong and is corrected in place.
- ZSpeed (Processed+0x944) is copied from the prior PhysOut Motion+0xA8 at 0x82e026dc, which is `dot(deck velocity, effective z)` (0x82b3016c). The port's `try_fill_board_forward` matches.
- The whole goal-3 chain is still native-identical. The remaining question is whether the port ever exceeds 12 to 14 m/s (`play-speed-trace.bat`).

## Additional items verified native-identical this pass (do not re-audit)

- **Skater ground pipeline Params.** `allowMultipleContactWrites` = 1 (InitWorld 0x827b4b0c, r23 = 1), which matches the port's `allow_multiple_writes: true`. `spy_flags` bit 4 only enables spy export.
- **Ground run effective padding.** min(max(dot(-n, v*dt), 0), 0.5) + volume padding (0x827cd444) matches `try_assembly_ground_effective_padding`.
- **`kEnableIKContactOffset`.** Native default true (0x82f7db28), the same as the port.
- **`wait_for_bs_job` / Phase1..3 `GoWide`.** These are blend-space job fan-out only, not pose latency.
- **Wobble threshold inputs.** Truck tightness comes from the CAC record (`ApplyPlayerAttributes` 0x828159b0). The CAC defaults (InitPlayer 0x82573c38 / InitializePlayer 0x8258fbf0) are 0.3 / 0.7, which equal the port's PlayersData Reset values. The threshold is therefore 12 + 1.5 + 3 * comFraction, about 13.5..16.5 m/s, in both.
- **Other wobble systems.** `SkateboardWobble` (Skeleton+0x3E00, takeoff/landing deck wobble) is already ported (`skeleton_skateboard_wobble.rs`). The "speedWobble/speedWobbleBlend" print belongs to the verified `UpdateAnimationTurn`. `kSkitchingApplySpeedWobble` is skitch-only, and the "SpeedWobble" string at 0x820649a0 is an audio category.

## Extra goal-1 trace

`SKATE2_TRACE_TOE_TARGETS` also prints `toe bodies:` once per SkeletonCollision update with the ground point, the danger_zone flag and the post-solve body positions of parts 15/16/19/20. Together with the `toe targets:` drive-target lines, each frame of the jump-into shows the target versus the actual body.

## Goal 1: board-anchor mechanism measured (captures 06:52 and 07:10)

- **Feet track their targets exactly.** A rigid (Kabsch) fit of the part 15/16/19/20 drive targets onto their post-solve bodies leaves a residual of about 0 in steady frames. The drives and solver are not the cause.
- **The skater is anchored to the board.** UpdateRootTransforms 0x82b42938 sets `AnimToWorld = T(predictedBoardPos) * Reckoning * yaw * T(-SkateTransformAnim.P)`. SkateTransformAnim.P = (0, 0.089, 0) = anim board volume 0, which is fixed in board-centric anim space. The board is read from Part 6 (`Body::GetPosition(6)`) and the velocity from the Part 6 rigid body, and the port matches this.
- **During GroundAnimation the board is steered only by an angular soft hook drive.** GroundAnimation::Enter 0x82d83ea8 calls `EnableAngularDriveOnly(1.0)`: SOFT, spring = kSkateboardSoftDriveDisp 20000, strength = kSkateboardSoftDriveStr 10 * 3600, damping = 2 * sqrt(spring), with the linear drive zeroed. The target is `ApplyBoardAnimation` 0x82b357f0, which sets the transform on `SkateboardBody::mHookAssembly`, optionally after `BlendToAnimatedBoard` 0x82b25bd8. That blend is gated on `mPhysicsErrorBlending`, which `UpdatePhysicsError` 0x82b23aa8 sets every PhysicsGround frame. All of this is ported and native-identical, including the cvar defaults and GetBlend's smoothstep.
- **Measured board lag.** Anim board pitch over the jump-into is 0.280, 0.170, 0.256, 0.436, 0.598. Physical Part 6 pitch is 0.291, 0.234, 0.227, 0.312, then 0.585 two frames later. With the implicit soft-drive coefficients (k*h^2 = 5.6, c*h = 4.7), each frame closes about 50% of the error, so this lag is inherent and not a port bug. The animation rotates the board about its centre, and the design relies on the physical board pivoting on its tail to lift the anchor. The lagging board lifts the skater about 4 cm instead of about 15 cm, and the back toe reaches the ground at F+1 under DZ.
- **Ground reference.** CalculateGroundPos 0x82b2f1b0 returns the lowest wheel-CENTRE plane (no radius), in native too. The rest height of Part 6 above it, 0.0565, equals physicstrucks VERTICAL_OFFSET -0.0565, and the wheel radius is 0.031. The board is not riding low.
- **Also native-identical:** the frame order (JumpInto, IsSkateboardAnimated, 100->103, UpdateAnimatedSkateboard, solve, Update4), the spy bit on all skater parts, `nb_word & 8`, positive impulse, and compliance from the per-part drive attributes.
- **Still unexplained.** Nothing checked explains why native avoids the F+1 toe contact. In the 07:10 capture the toe bottom was 1.3 mm under the contact plane while moving up at 3.7 m/s. A small positive position-correction impulse flags the contact, so a few millimetres decide the outcome.
