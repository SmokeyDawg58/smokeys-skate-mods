# Career cutscene t-shirt cloth (October 9)

Goal: T-shirt cloth working in cutscenes. The skater's
t-shirt is ROPA cloth. In gameplay the presentation restore steps and draws it
(`SkaterRestore::restore`). In a cutscene the scene's `current_player` borrows
the skater's entity, and `publish_borrowed` posed only the palette.

Before this change the t-shirt kept the cloth vertices and palette rows of the
last gameplay step. Those rows hold the gameplay local-to-world matrix, so the
shirt was drawn where the gameplay skater stood, in the gameplay cloth shape,
while the body moved with the scene.

## Native chain for the borrowed skater

1. `SceneCharacterManager::InstanceCharacter` 0x82876bd8 gives "current_player"
   the local skater's entity (`GetSkaterPresEntityFromIndex(GetLocalIndex())`,
   0x82876ed0) through `ControlCharacterRenderable` 0x82870fb0:
   - `WaitForJobsToFinish`;
   - the callbacks (+0x1E4) become the manager's (+0x18), `mIndex` (+0x1E0) the
     scene slot, the type 0x822;
   - `mIsGhosted` (+0x61A) = 0 and `i_params[0].x` (+0x870) = 1.
   - The cloth, its effects and their state stay as they were.
2. `SkaterPresentation::DoSimRestore` 0x827e6438 returns at once while
   `SceneManager::IsInCharacterNIS` 0x82862db0 (0x827e6488), so nothing restores
   the borrowed entity.
   - `IsInCharacterNIS`: `mNISHandle` (+0x8C) is set and
     `SceneBinDirectory::IsSceneWithCharacters` 0x8286ece0 holds, which is a type
     other than 0, 2, 5, 6 and 13 that does not control the replay.
3. Each drawn frame, `SceneManagerImpl::UpdateFrameBegin` 0x82878428 advances the
   scene. With `kNISUsesTheAudioClock` (default on, 0x82f71e80) the step is the
   audio system time since the last frame.
   - `Scene::Update` 0x82870bb8, `AddTime`, then `SceneCharacterAnim::EvalCurrAnim`
     0x82866cd0 fills the pose buffer.
   - The trajectory's slot takes the hips' rows and translation, w included
     (0x82867164..0x8286717c).
   - `SetObjectTransform` gives the entity its `m_MatLtoW`.
   - `SceneCharacter::UpdateCloth` 0x82865e90 (called at 0x82866f58) is empty in
     this build.
4. `SceneRenderView::Cull` 0x828500a8 calls the entity's `Update` (vtable +0x10):
   - at 0x8285043c when it is in view;
   - at 0x828504d8 when it is culled but within the shadow distance
     (`maxShadowDistSquared` 1300, 0x82feffdc).
   - `RenderModeScene` 0x82857a78 culls the main view once a frame (0x82857ac8).
5. `SkaterPresEntity::Update` 0x827e8568: not restored this frame and local, so
   it sets `mWasRestoredThisFrame` and runs `StartJobs` 0x827dbb90 and `EndJobs`
   0x827dbd50 itself.
6. `StartJobs`:
   - bones = `GetSkaterBoneBuffer(mIndex)`, here
     `SceneCharacterManager::GetSkaterBoneBuffer` 0x82865d60, the scene
     character's pose buffer;
   - `UpdateBoneTransforms`;
   - `mHipsTransform` (+0x240) = bones[0] with the axes' w cleared
     (0x827dbbf0..0x827dbc88);
   - `BeginSkinJob` 0x827d7c48 for each CPU-skinned model.
7. `BeginSkinJob` copies `mHipsTransform` into the skin parameters (+0x330) and
   builds the skin job (`SkinModel` 0x828442b8; with `kUseSkinningJob`, default
   on, it runs after this function). With `cv_EnableRopa`, no `mIsClothDisabled`
   and an effect it calls `ClothUpdate` 0x827d24f8.
8. `ClothUpdate`:
   - Clock (0x827d2574..0x827d2684). In a character NIS: the active scene's
     `ScenePlayer::GetTimeElapsed` 0x8286c9e0 times 60, floored (`vrfim`), through
     `fctiwz`/`fcfid` for the frame, the rest for the fraction; state
     `LAST_CLOTH_STATE_NIS` (2). Otherwise the presentation's time controller,
     state `LAST_CLOTH_STATE_GAME` (1).
   - `PreClothUpdate` 0x827d2318 sets `mEffectScale` from `mFramesInWipeOut`
     (+0x2C4). Only `DoSimRestore` writes that (0x827e7294), so in a NIS it keeps
     the last restore's frames.
   - Restart (0x827d26a4..0x827d26dc) when the state is not `mLastClothState`
     (+0x280), the frames since the effect's last update are 15 or more either way
     (`CLOTH_MAX_FRAME_DELTA` 0x8210be90), or on the game clock without
     `mWasRestoredThisFrame`.
   - A restart sets `mLastClothState`, the effect's reset flag (+0x8C) and
     transform (+0x1D0), and copies the drivers the skin job left (effect+0x180,
     32-byte stride, 0x827d27bc..0x827d2810) into the particles. This frame's job
     has not run, so these are the last frame's drivers.
   - `mLastClothState` is one per entity: a change of clock restarts only the
     first effect it reaches. The others restart only on their own frame gap.
   - Otherwise dt = |((fraction - `mPrevDelta`) + frames) / 60| (`fsel`) and
     `ROPA::Effect::Update` 0x8285ff38 runs with `mHipsTransform`. Under 1e-5 s it
     stores the transform and returns no job.
   - `mPrevFrame` (+0x2A4) and `mPrevDelta` (+0x2B4) always take the clock.
9. The ROPA job (`Effect::DoUpdate` 0x8285fa50) integrates, runs
   `SolveConstraints`, then `WriteDestination` 0x8285cae0 (`Inverse` of the
   transform, then the positions into the skin job's destination). A frame
   without a ROPA job (a restart, or under 1e-5 s) draws the skin job's vertices.
10. `EndJobs`: `EndSkinJob` 0x827d7f38 waits and copies the vertices
    (`DoubleBuffer` 0x827d2840), then `PackAndMultiplyMatricesForUpload` with
    `m_MatLtoW`, the scene's object matrix.
11. No teleport, wind or other input reaches the cloth. Its inputs are the drivers
    (skinned through `mHipsTransform`, the hips' transform within the
    trajectory, so the scene moving the skater through the world does not reach
    it), gravity (`Effect::Update` 0x8285ffc4) and dt.
12. At the scene's end `RemoveCharacterRenderable` 0x8286d478 restores the type,
    the index and the callbacks. The next `ClothUpdate` reads the game clock and
    restarts.

The NIS draws at 30 Hz. `PostFX::Apply` 0x8284c7d8..0x8284c860 sets
`m_b30HzMode` while `mICEPostfxActive`, and `RenderMan::Render`
0x82858460..0x828584d0 presents at interval 2. The native cloth therefore steps
30 times a second, by about 1/30 s.

## The other cutscene characters

Reda, Slappy, Attiba and the other characters do not simulate cloth natively,
whatever their models carry:

- `InstanceCharacter` passes `clothDisabled` = !`IsCharacterClothOn`
  (0x82876d00..0x82876d1c). That is the `IScenePlayer` vtable +8, 0x82861b08,
  the cvar `kNISCharacterClothOn`, default off (0x82f72158).
- `CreateCharacterRenderable` 0x82871150 builds the `SkaterPresEntity` with
  `isLocalPlayer` 0 (0x82871244).
- The constructor sets `mIsClothDisabled` = !`mIsLocalPlayer` || `clothDisabled`
  || the excluded marquee schema (0x827e58f8..0x827e591c), and skips `InitCloth`
  (0x827e5920..0x827e5944).
- `Update`'s non-local branch never runs `StartJobs`.

The port already builds them with cloth disabled
(`CharacterMaterialAdmission::try_construct_scene_character`), so their shirts
are GPU-skinned as natively. Nothing else is ported for them.

## Shipped scenes

In `scene_000.bin` (miscboot.big), 39 scenes have a `current_player` record, and
all are character NISes:

- the story scenes (type 18), for example `01_Welcome_Back`, `02_Pros_Arrival`,
  `03_Pros_Leave` and `04_Arrival_of_Mikey`;
- the pro-challenge intros (type 15), for example `01_ProChallenge_Appleyard`;
- `Goto_d_0_0_dannyway_goto`, `ViewScene`, `Win_Big_1_Autographs` and
  `Win_Big_1_Autographs_02`.

`Goto_m_1a_01_goto_slappys_outro_1` to `_3` have no characters, and `_4` has
four `ai_skater_06`. The player's skater is not in those scenes.

## Port

- `skater_ropa_runtime.rs`, `SkaterRopaRuntime`:
  - `state` is `mLastClothState`, shared by the effects. The gameplay `step`
    also restarts when the last restart was by the NIS's clock. Without a NIS in
    between, that is the old condition.
  - `wipeout` keeps the frames in wipe-out of the last gameplay step.
  - `step_borrowed(ClothTime)` is `ClothUpdate` for the borrowed entity: the NIS
    clock (`nis_frame`), restarts on the last skin job's drivers, no step under
    1e-5 s, and the skinned vertices drawn on a frame without a step.
  - `skin_drivers`, `effect_scale`, `effect_time` and `particle_positions` are
    shared with the gameplay `step`, with the same arithmetic in the same order.
- `character_material_admission.rs`:
  `CharacterMaterialAdmission::step_borrowed_cloth`.
- `nis/characters.rs`: `SceneCharacter::pose_palette` is `publish` without the
  upload. It also returns the pose buffer's first matrix (`SceneSkin::first_bone`),
  the trajectory's slot with the hips' translation w (0x8286717c).
- `nis/manager.rs`: `NisSystem::character_nis_time` gives `IsInCharacterNIS` and
  `mTimeElapsed`.
- `skater_presentation_selected_runtime.rs`, `publish_borrowed(character,
  nis_time)`: the scene's pose, then `mHipsTransform` (the axes' w cleared), its
  inverse, the cloth's step and its publication with the scene's object matrix,
  then the palette's upload. Without a character NIS it steps by the tick clock,
  as the tick path does.
- `selected_session_career.rs` and `selected_session_public_facade.rs` pass
  `career.nis.character_nis_time()`.
- PC renderer: unchanged. The borrowed skater draws its tick palette and the CPU
  rows of `CharacterMaterialAdmission::view`. The cloth vertices upload with the
  palette's generation, which `publish_borrowed` advances every NIS frame.

## Deviations

- Cadence: the port's NIS frame is the 60 Hz tick, as the port does not drop to
  30 Hz. The cloth therefore steps 60 times a second by 1/60 s, the gameplay
  cadence, where the native steps 30 times by about 1/30 s.
- Culling: natively the entity steps only when the main view's `Cull` reaches it
  (in view, or within about 36 m of the camera). The port steps it every NIS
  frame.
- Clock source: the port's scene time is the sum of the frames
  `update_presentation` is given. The native NIS follows the audio clock.

## Left

- Gameplay restarts (`SkaterRopaRuntime::step`, used by the restore and the tick
  path) still put the particles on this frame's drivers. A gameplay frame without
  a step publishes the solver's positions. Natively a restart takes the last skin
  job's drivers (0x827d27bc..0x827d2810), and a frame without a step draws the
  skin job's vertices (0x8285ff8c). This is left so that gameplay stays
  byte-identical.
- After a scene ends, the skater keeps the scene's last pose and cloth until the
  next simulation tick. Natively the entity's own `Update` re-poses it at once,
  from the callbacks `RemoveCharacterRenderable` 0x8286d478 restores.
- "dynamic" records with a player override, which natively also borrow a
  skater, are not instanced by the port.

## Verification

- `cargo check --release -p skate2-core --tests -j 1`: passes, with no
  warnings in the changed code.
- Unit tests in `skater_ropa_runtime.rs` (not run here):
  - `a_change_of_clock_restarts_on_the_last_jobs_drivers_and_draws_the_skin`;
  - `a_change_of_clock_restarts_only_the_first_effect`;
  - `the_nis_clock_floors_the_elapsed_sixtieths_through_an_integer`;
  - `an_effects_time_is_a_sixtieth_a_frame_within_fifteen_frames`.
- Game-data test in `nis/characters.rs` (`#[ignore]`, not run):
  `the_borrowed_rigs_first_bone_is_its_trajectory`.
- Not playtested.
