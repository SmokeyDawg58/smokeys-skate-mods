# Skater respawn opacity, sim to presentation: Update0 collision status, camera opacity and DoSimRestore (2026-09-27)

**Status:** rustfmt PASS. Every new hunk is rustfmt-neutral, the new files are fully formatted, and pre-existing diffs were left alone. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`. The renderer half is in `skater-ghost-render-2026-09-27.md`.

Scope: tie the programmed respawn opacity (going to a marker, etc.) to the renderer the same way the real game does, so respawns feel natural.

## 0. Why the opacity never reached a visible value

The sim opacity was already ported: `UpdateSkaterOpacity` 825f1240 feeds `SkaterAnim` opacity, and the frame header carries it. It could not move for three reasons.

1. **World-only was never cleared.**
   - The HiLOD ctor 82e04d2c..4da4 sets world-only collision for a non-network game (`INetworkGameQuery` at HiLOD+0x314 = Simulation; slot 0 `IsNetworkGame` 8289e448). The port matches this.
   - Native clears it every frame in `Update0_ProcessCollection` 82e01f18, and the port never ran Update0.
   - `UpdateSkaterOpacity`'s fade-in holds while PhysOut State+0x3D (HiLOD +0x120 bit 0x01000000, published by Update5 82e05178..51a4) is set and t > fadeIn*0.5. So the port's opacity stayed at about 0.5.
2. **The steady-state camera branch was unported,** which part B fixes.
3. **Nothing consumed the published opacity on the presentation side,** which parts C and D fix.

## A. `Update0_ProcessCollection` 82e01f18

`World::Update1` 827ae098 runs `Update0PhysicalGatherPrimitives` for every physical before the RunPhysicsJobs Update1/2/3. The port calls `try_update0_pair_process_collection` from the selected Ground pre-solve, after the cvar clone and before `published_animation_physics_input` (`physical_player_ground_pre_solve.rs`).

### `UpdateCollisionStatus` 82e016a0 (`physical_player_update0_collision_status.rs`)

1. **Candidate.** The skater is a candidate if either is set:
   - HiLOD+0x120 bit 0x01000000 (world-only);
   - AnimOutPhysIn +0x2aa0 bit 0x01000000. `Actor::SetUpPhysics` 825ff8d8 copies this from Actor+0x258 bit 0x02000000, which `LivingWorld::Manager::CheckSpawnedDynamicObjectAgainstSkaters` 82725914 sets.
2. **Radius and position.** It reads `kLWTestRadius` (82e01730..1748), then the Skeleton+0x2d80 AnimToWorld translation.
3. **Other skaters.**
   - Every other physical with the same `GetFlags` lowers the nearest squared distance to its `GetSkateboard` translation, seeded with 1e6 (0x49742400).
   - The LivingWorld tests run only when all lanes are beyond 1.5^2.
   - Any other registered physical returns `PendingOtherPhysical`, because the flag compare is unported.
4. **LivingWorld overlaps** (82e01874..1b98), stopping at the first hit:
   - **Pedestrian sphere.** `DoesAnyPedestrianIntersectSphere` 82703600 re-reads `kLWTestRadius` and queries (x, z) with radius r + 2, max 6.
   - **Overlay assertion.** `kbShowCollisionStateChecks` asserts the DebugOverlay.
   - **Dynamic objects.** `DoesCapsuleAndAABBoxIntersectAnyDynamicObject` 82725930 goes through `GetDynamicObjectsWithinRange` 82d3be78 (64, clamped to 50) to KDTree `FindNear` 82d28c00, which zeroes the count before it reads the point or radius.
     - On the constructor-empty store, the `BuildSkaterTestCapsule` 827041a0 / `AABBox` 82704488 outputs (HiLOD+0x334) and `kSpawnTestCapsuleMinDistance` are never consumed. So they are not ported.
     - A populated store stays an explicit error.
   - **Vehicle sphere.** `DoesAnyVehicleIntersectSphere` 82710760 uses the first radius read: r + 8, max 6.
   - **Overlay assertion,** again.
   - A non-empty broadphase returns `PendingLivingWorldOverlap`, because the narrow tests are unported.
5. **Collision change** (82e01b9c..1bf0). The flag is written first and only when it changes. Then the Skateboard `CollideWithWorldOnly`/`CollideNormally` runs, then `SkeletonController::Start/StopCollideWithWorldOnly`.
6. **Final overlay assertion** (82e01bf4..1c20).

### Vehicle impact, cvars and gameplay effect

**Vehicle impact** (82e01f30..1f80). The guarded PhysOut Collision+0x94 (skeleton collision audio `mVehicleImpact`) is compared ordered > 0. When it is, `CheckPlayerInsideVehicle` 82dfe690 would run; the port returns `PendingCheckPlayerInsideVehicle`.

**Cvars:**

| # | Name | Section / title | Default | init / table / ref |
|---|---|---|---|---|
| 610 | `kLWTestRadius` | Physics_Teleport, "LW Test Radius", GUI 0..2 | 0.6 (0x3f19999a) | 82f9e4e0 / 82fd6280 / 830e0cb4 |
| 611 | `kbShowCollisionStateChecks` | Physics_Debug, "Show collision state checks" | false | 82f9e720 / 82fd6298 / 830e0e74 |

Registration order places them between 190 and 82. The stale cvar title test is now 68 behind.

**Gameplay effect:** with no LivingWorld overlap, the first Update0 now clears world-only, exactly as native. The skater collides normally after spawn, and the fade-in is no longer held at 0.5.

## B. `UpdateSkaterOpacity` steady state (825f1368..13e0)

- `Simulation::GetCameraManager`, then `GetCameraSkater` 82581bf8 (GameCameraManager+0xD0).
- If the camera skater == this: `SetOpacity(GetCameraSkaterOpacity(), true)`.
  - `GetCameraSkaterOpacity` 82e34960 returns 0.0 when the CameraMan list [+0x50, +0x54) is empty.
  - Otherwise it returns the first CameraMan's +0x36C (written by `UpdateRig` 82e48330/833c).
- Otherwise: `SetOpacity(1.0, false)`.

Port: the `SkaterOpacityCamera` trait (`actor_animation_phase1.rs`). `CameraStartupFields::camera_skater_opacity` does a ptr_eq on `actor` and reads `camera_man_count` / `camera_man.opacity`. `SkaterManagerLocalIdentity` implements the trait, and the corpus uses a `NotCameraSkater` stub.

## C. `SkaterPresentation::DoSimRestore` render opacity (827e7298..73e0)

Port: `skater_presentation_render_opacity.rs`, mounted by `skater_presentation_selected_runtime.rs`.

### Inputs

`UpdateSkater` 827da1d0..1f0 copies the frame header into the post-blend flags:
- word 0xbc bits 0xff00 (byte 0xbe) becomes the opacity byte at post-blend 0xbd;
- word 0xc0 bit 0x4000 becomes collidable (0x02000000);
- word 0xc0 bit 0x2000 becomes main-camera-only (0x01000000).

The collidable source is `Ghost::IsActorCollideable`, which is true in solo (Ghost null). The port reads the same header bytes in `try_update`, along with the `m_MatLtoW` translation (hips) and the DataPostBlend root Y axis (up).

### Per-entity loop (raw instructions re-read)

1. **Target** (827e7298..72f8).
   - Collidable: 1.0 when main-camera-only && replay; otherwise byte * 0x3b808081 (1/255).
   - Not collidable: 1.0 when replay; otherwise 0.5.
   - `Presentation::IsInReplay` 827eed90 asks Presentation+0x84 for its active replay. The port has none, so it is false.
2. **Proximity.** `EvaluateProximityOpacity` 827d3458, with `CalcClosestPointOnLine` 82e48fb8 (VECFLOAT_SMALL guard):
   - inv = 1/fmsubs(far, far, near^2);
   - segment = hips ± up*L;
   - t = clamp((|closest - camera|^2 - near^2) * inv, 0, 1) (fsel);
   - prox = fmadds(t, farOp - nearOp, nearOp).
3. **Rate-limited approach** (827e7350..7390). cur = +0x870 x. dt = `kUpdateTime` 0x3c888889.
   - Ordered cur > prox: fnmsubs(FadeOut, dt, cur), then fsel(prox - f, prox, f).
   - Otherwise: cur + FadeIn*dt (fmuls before the compare, then fadds), then fsel(prox - f, f, prox).
4. **Clamp and store** (827e7394..73e0).
   - Clamp: fsel(-f, 0, f), then fsel(1 - f, f, 1). Then min with the target: fsel(target - f, f, target).
   - `mIsGhosted` (+0x61a) = final < 1.0 (fcmpu/blt).
   - The +0x870 vector keeps y/z/w; x = final.
   - The constructor 827e5948..5988 starts at (1, 0, 1, 0), not ghosted.

### `rendering` values and the camera

The `rendering` class 0x1ffdc8e3aca07c1f, collection `skaters` 0xda92275f3d735e7d, is at GlobalAttributeCollection+0x198 (layout 0x78). Values were dumped from the original `skater.vlt/.bin`; the collection is present in the mounted Vault.

| Offset | Field | Value |
|---|---|---|
| 0x38 | Fade_MinOpacity | 0.15 |
| 0x3c | Fade_MinDistance | 0.5 |
| 0x40 | Fade_MaxOpacity | 1.0 |
| 0x44 | Fade_MaxDistance | 1.0 |
| 0x48 | Fade_HeightOffset | 0.61 |
| 0x4c | Fade_FadeOutFrames | 5.0 |
| 0x50 | Fade_FadeInFrames | 15.0 |

Despite the "Frames" names, native multiplies both by `kUpdateTime` (827e736c, 827e7378).

The camera is the provisional CameraMan producer matrix the renderer draws with. That is the documented CameraPresentation restore gap; native uses `CameraPresentation::GetMatrix`.

### Order and cadence

1. `GameModule::Render` 827320f0 calls `DataSysMan::RestoreFrame` (827323ac) before `RenderMan::Render` (827324ec).
2. `RenderModeScene` 82857a78 runs `SceneRenderView::Cull` 828500a8. Cull calls each visible entity's vtable+0x10 `Update` (82850434 / 828504d0). Then come `SyncSkaterJobs` 82857b04 and Render.
3. DoSimRestore sets `mWasRestoredThisFrame` (+0x61b, 827e7090), and EndJobs 827dbe4c clears it. So `SkaterPresEntity::Update` 827e8568 selects the technique from this frame's `mIsGhosted` (827e8730).

Port: `try_restore_render_opacity` runs in the facade's Completed branch. It runs before `last_step_completed`, with the same camera projection the snapshot hands the renderer. `SelectedVisualSnapshot` gains `skater_opacity` / `skater_ghosted`.

`DataProvider::DoRestore` 82995ea8 restores only when the provider bits `& 0x30000000 == 0x20000000`, and clears them afterwards. That reads as a once-per-submission gate, but the setter was not traced. The render thread (`AppMan::RenderLoop` 82989fc0) free-runs, calling `Render` with its own stopwatch dT.

The port runs DoSimRestore once per completed sim tick. That equals native while the native render keeps up with the 60 Hz sim; the present interval was not traced. The visible respawn ramp is the sim-side `UpdateSkaterOpacity`, which is per tick either way.

## Remaining pending errors (no fallbacks)

- `PendingOtherPhysical`
- `PendingLivingWorldOverlap`
- `PendingCheckPlayerInsideVehicle`
- `UninitializedLivingWorld`
- `RenderOpacityError::MissingInputs` (unreachable after a completed presentation `try_update`)
