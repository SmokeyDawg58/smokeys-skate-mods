# Performance: AI load, tick cost, present pacing — October 8, 2026

Playing the first missions showed stutter near the stairs at
Slappy's on the test machine, with vsync off.
The capture (`diagnostics/frame_profile.csv`) showed 4.3 ms ticks with one
skater and 13–18 ms with three ambient AI skaters. Over-budget ticks then ran
2–4 catch-up ticks in one frame (50–75 ms frames). Each AI spawn also cost
44–81 ms of `ai_dispatch`. Skating physics had to stay bit-identical.

## Measurement

- AI benchmark: `--live-controls-demo main-menu-freeskate` with
  `SKATE2_DIAGNOSTIC_FREESKATE_LOCATION=slappy`. Free Skate starts at Slappy's,
  and ambient AI spawn every 30 s up to three. `SKATE2_NO_VSYNC=1`.
- Mission start: `--live-controls-demo career` with `SKATE2_DIAGNOSTIC_CAREER_YES=1`
  and `SKATE2_DIAGNOSTIC_CHALLENGE=m_1a_07_follow_slappy` at tick 3000.
- Parity: the October 2 pool route, `SKATE2_TRACE_SELECTED_STATES=1`, 1500
  ticks, 6000 `selected (board|states|offboard|manual) frame` lines. They were
  byte-identical after every change below.
- New opt-in diagnostics:
  - `SKATE2_FRAME_PROFILE` gains `image_*` columns: the spacing of frames that
    show a new tick, with late and skipped counts. It also gains
    `fps_counter_min`, the on-screen counter's lowest 0.5 s value in the window.
  - `SKATE2_TICK_TRACE` writes each tick's due, start, finish and consumed times.
  - `SKATE2_PRESENT_TRACE` writes due, consumed and presented times.
  - `SKATE2_TRACE_RECKONING_NAN` reports the first degenerate reckoning basis.

## Changes

1. **Build:** `.cargo/config.toml` sets `target-cpu=x86-64-v3`. `f32::mul_add`
   was a CRT `fmaf` call (6–8% of the tick); it is now an FMA instruction with
   identical fused results. The release profile keeps line tables for profiling.
2. **Collision:** `KdTree::aabb_range_query` decoded the whole branch-node
   array into a `Vec` on every query (11.5% of SIM time). It now checks the
   allocation range eagerly, as before, and decodes each node when visited.
3. **Animation database:**
   - `FaceDatabase::parsed` is `Arc<AndaleDatabase>`. It is read-only after
     `SetDBContent`, so clip and pose decodes on the job threads
     (`execute_fetch`, `raw_sqt_for`, `constant_vbr_sqt*`) run outside the
     database lock. Before, they held the lock for whole decodes, which
     serialized the three job threads and blocked the SIM thread's attribute
     and blend reads (about 9% of SIM time).
   - Body dispatch keeps the `Arc` instead of cloning the hierarchy.
   - The presentation reads borrow the hierarchy instead of cloning it each tick.
4. **Drive pool:** `SimulationDrivePool::validate_integrity` caches a successful
   walk, as the rigid-body pool does. Every ring edit clears the cache.
5. **AI character loading:**
   - `AI::SkaterAsset`'s `CharacterRecipeLoader` is modelled for real. Each
     recipe the `SkaterAssetManager` holds has its marquee scene made on a loader
     thread (`MountedPlayerRecipeLoad::try_prepare_marquee_scene`) and kept
     resident while the asset keeps the recipe.
   - An asset's loader reports loaded once that scene has streamed
     (`SkaterAssetManager::streamed`).
   - `SkaterManager::SpawnAi` builds the skater once its character has loaded,
     as the native PlayerLoader does. Until then, teleports and requests to the
     player find no skater, as natively.
6. **Render side:**
   - A changed AI roster keeps the other skaters' GPU objects and lighting.
     Before, it rebuilt every skater: archive reads plus texture uploads.
   - The DMO bank decodes on a thread from `DmoRender::new` (the first DMO
     frame took about 20 ms).
   - `marker_ui::Assets::texture` memoizes its suffix scan; adding a texture
     clears the memo.
   - The Free Skate viewer streams its character's recipe scene
     (`SceneCharacterModels::present_streamed`); the menu hitched 40–60 ms per
     selection. NIS characters keep the synchronous path.
7. **Terrain:** `drain_prior_wake` no longer waits on a collection job whose
   decode thread is still running. It and the jobs behind it complete at a later
   update, in order, as the native decompression thread's completions arrive.
   Teleports had stalled 50–66 ms in `terrain_poll`.
8. **Present pacing (host):** without vsync, a frame showing a new tick presents
   a fixed time after that tick fell due: about the 95th percentile of the last
   second's due-to-ready times. A slower image presents when ready. This trades
   about 4 ms of display latency for even image spacing. Under vsync the refresh
   times the flip.
9. **Unchanged frames (host):** between ticks nothing in a gameplay scene
   changes, yet every frame rendered the whole city again, and a tick that
   finished meanwhile waited behind that redraw (100 of 109 late images in a
   trace). A frame the client marks unchanged is now not presented at all: the
   window waits for the SIM thread's next tick (`SimShared::ready`) and draws it
   at once. Menus, fades, map travel and the pause screen still render every
   frame. The on-screen counter counts real images: 60 a second in gameplay.
   (An interim version presented copies of the last frame instead; the counter
   then read 1,000–2,000 for the same 60 images.) The original draws frames
   between ticks by blending each presentation's last two submissions
   (`CameraPresentation::DoSimRestore` 0x827f60e0 and its siblings); the port
   does not yet.
10. **Map draws (host):** with one rendered frame per tick and a moving camera,
    each map bundle served one frame, so its validation and its triage on drop
    were overhead. A new selection now draws straight into the pass; one that
    renders again (a still camera) records a bundle once and reuses it.
11. **Native culling the port had left out** (`map_draw.rs` `select_visible`):
    - Island area rule: `WorldPresEntityOptimesh::CullMeshes_VMX` 0x827d0ae8
      culls a visible optimesh entity's islands through `DoFrustumCulling`
      0x828589c0 with the RModel's `m_pIslandAreas`. An island whose nearest
      corner is at least `minDist` (120) past the near plane is dropped when its
      view-projected area times `optimalCullDist` (900) over that depth squared
      is below `minAreaCull` (0.2) (`presentation_scene_culling`). The city's
      models are old-format WorldPresEntities, so on the cruise route it culls
      nothing.
    - Anti-frustum occluders (`presentation_anti_frustums`): every registered
      arena's `tAntiFrustumData` (type 0x00EB0025, 443 quads in 25 arenas along
      the cruise route) goes into a KD tree; each view selects up to 32 quads
      within 100 of the camera that the view does not wholly reject, builds the
      quad's plane plus one plane per edge through the camera, and culls
      entities and islands whose sphere lies behind all five. On the cruise
      route about 12 occluders are active, hiding about 73 entities per tick
      and up to 1,443 in the densest views; captures before and after are
      identical.
    - The shipped `minPixelSize` is zero, so the pixel test culls nothing.

## Results

| AI benchmark, vsync off | Before | After |
|---|---|---|
| Tick, 2 AI | 8.13 ms | 4.54 ms |
| Tick, 3 AI | 10.68 ms | 5.80 ms |
| AI spawn stall | 44–190 ms | none |
| New images later than 20 ms | 3.6% | 0.03–0.13% |
| Mean deviation from 16.67 ms | 1.15 ms | 0.05–0.11 ms |
| Lowest on-screen fps in game | — | 573 |

At the Follow Slappy start, Slappy's spawn waits about 4 ticks for his scene
instead of stalling 60–80 ms. No frame after the mission start exceeded 18 ms.

Cruise route (`perf-skate` from `Q_SmokeTest_1300_N900`, 2400 ticks, vsync
off), alternating runs under the same background load:

| | Before 9–11 | After 9–11 |
|---|---|---|
| Tick picked up after it finished, p99 | 6.0 ms | 0.02 ms |
| Tick due to on screen, mean | 16.8 ms | 14.3 ms |
| New images later than 20 ms | 14 | 5–7 |
| On-screen counter, mean / lowest | 179 / 117 (redraws) | 60 / 59 (real images) |

With the occluders, alternating against the same build without them: new
images later than 20 ms 6–7 against 12–16, worst image interval 25–27 ms
against 33–80 ms, lowest counter 560–570 against 325–470.

## Fixed on the way

- Movies 70 (`challengefailed_small`, screen 45), 71 (`endchallenge_small`,
  screen 42) and 72 (`huddistance`) had no texture owner. Failing Follow Slappy
  ended the game with "unknown UI texture owner".
- AI `HippyJump28` (`StateUpdate_HippyJump_28` 0x828ed448: `OffBoard_2`, then
  packed slots 38/39) was a `PendingStateUpdate` exit.

## Open

- A rare AI reckoning NaN (`Roots(Assertion(AnimationToBoard))`) ended one run.
  The board was finite; the reckoning's integrated state was NaN, so it entered
  earlier. `SKATE2_TRACE_RECKONING_NAN` names the update that first produced a
  degenerate basis. AI sessions of 14 and 6 minutes did not reproduce it.
- Loading boundaries still stall on the render or SIM thread: career start,
  edit skater, the first frame of a new area, and NIS character loads.
- The remaining late images come from dense city views (3,000–4,300 map
  draws: about 4.5 ms of recording and pass replay per tick) and 7–8 ms ticks.
  PROXY objects carry no entity box here, so the occluders skip them; the
  manager's asset order (a hash map) is replaced by collection order.
  Per-chunk bundles (64 objects each) and a wider recorded margin were measured
  and rejected: both lowered the mean but raised the spikes.
- The render thread is mostly wgpu 0.19 tracking: pass replay (texture init
  actions about 13%) and submit triage. A newer wgpu would cut it.
- `cargo test -p skate2-core --lib`: 5430 pass. The same 9 tests failed before
  this work (stale expectations from earlier feature work: AI path registry,
  DMO cvars, grind kernel, vehicle queries, head tracking).
