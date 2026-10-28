# Pedestrian chase arrows — October 19, 2026

The arrows over pedestrians that warn, chase, tire of or give up on the local skater
(`Sk8::FE::ChaseArrowMan`, `FrontEndManager::mChaseArrowMan`), from `sk82_na_m.xex`
(Jan-17-2009).

## 1. Messages and the arrow list

- A pedestrian's chase graph sends `cMsgChaseState { fID, fFEChaserState }` (port:
  `LivingWorldMessage::ChaserState` in `pedestrian_graph_frame_chase_callbacks.rs`) on `LW::Man`'s
  event broadcaster. `FrontEndManager::StartInGame` (0x82640450) registers
  `ReceiveChaseStateMsg` (0x82634db8) there, which calls `AddArrow` (0x826347e0).
- `eFEChaserState`: warn 0, chase 1, tired 2, give up 3, end 4.
- `AddArrow`: a pedestrian without an arrow gets one (time 0) unless the state is end or ten
  arrows show (a fixed vector of 10). An arrow keeps an unchanged state, goes at end, and
  otherwise takes the new state with its time restarted.
- `GameModule::Update` calls `FrontEndManager::Update` (0x82640130) with 0x682, which divides
  it by 100: 16 ms. While `FEBackEnd::mFEState` is `eFE_STATE_IN_GAME`, paused or not, it runs
  `ChaseArrowMan::Update` (0x82631328). An arrow goes when
  `cLivingWorldPresEntityManager::GetPresPedestrian` has no pres pedestrian for it. Otherwise its
  time grows by 16, and a warn arrow goes at `kFEArrowTimeWarn` (6000), a give-up arrow at
  `kFEArrowTimeGiveUp` (3000). Chase and tired arrows stay until their state changes.
- `FrontEndManager::GameUnload` (0x82628c40) calls `RemoveAllArrows` (0x8261cfb8).

## 2. Placement (`Update`)

- The anchor is the pres pedestrian's local-to-world translation raised by
  `kFEChaseArrowSecurityGuardOffset` (0.72).
- The displacement is the anchor less the local skater's hips (`SkaterPresentation::GetHipsPos`
  of `GetLocalIndex`). `GetScale` (0x82607440) of its length gives 1 below
  `kFEChaseArrowMinScaleDistance` (1), `kFEChaseArrowMinScale` (0.4) from
  `kFEChaseArrowMaxScaleDistance` (50), and a straight line between.
- The anchor raised by `kFEChaseArrowSkaterOffsetY` (1.15) goes through
  `PostFX::CalculateScreenPos` (0x82828310, `CAMERAIDX_GAMEVIEWPORT1`): x = 1280 (ndc x + 1) / 2,
  y = 720 (1 − (ndc y + 1) / 2), and it answers the clip z. It also works out a fisheye-warp
  correction, but overwrites both results with the plain position (0x8282852c, 0x82828548).
- The arrow is on screen when the clip z is positive and `IsInTitleSafe` (0x82618b68) holds:
  inside the middle `mTitleSafeFactor` (0.85) of the 1280x720 front buffer.

## 3. Drawing (`Render`)

- `FrontEndManager::Render2D` (0x82633be0) calls `ChaseArrowMan::Render` (0x82631638) only while
  `HUDManager2::mVisibleFlags` is 0, before `AptIntegration::Render2D`. The quads draw with
  alpha blending, no depth test, on `CAMERAIDX_ORTHO_SCREENSPACE`, as `QuadListTextured` with
  `STANDARD_UV_7` ((0,0), (1,0), (1,1), (0,1)), white, alpha 1 (0 at scale 0).
- Textures (`GetArrowAsset` 0x82618aa0): `arrow_warn.Texture`, `arrow_chase.Texture`,
  `arrow_tired.Texture` and `arrow_giveup.Texture`, 128x128 in
  `data\fe\source\images\icons\minimap.rx2` (`screens.big`). Each is an arrow pointing down.
- On screen (`RenderOnScreenArrow` 0x8262f740): the corners (32s, 0), (−32s, 0), (−32s, −64s),
  (32s, −64s), turned by π (`RotatePoint` 0x82618998) about the point. The arrow hangs 64s below
  the point, over the pedestrian's head.
- Off screen: the displacement and `CameraPresentation::GetMatrix(true)`'s row 2 are flattened
  (y = 0) and normalised. θ = acos(clamp(dot)), negated when (view direction × displacement).y > 0,
  and the angle is θ − π/2. `RenderOffScreenArrow` (0x8262f2e8) puts the centre at
  (640 + cos(angle) (0.85·640 + `kFEChaseArrowRadiusOffsetX` 20),
  360 + sin(angle) (0.85·360 + `kFEChaseArrowRadiusOffsetY` 20)), held 32 inside the title-safe
  frame. The corners (±32s, ±32s) are turned by angle − π/2, so the arrow points toward the
  pedestrian.

## 4. Port

- `skate2_core::ui::chase_arrows` holds the list, the placement and the quads.
- The list lives on the `LW::Man` event route (`GameSimEventRoute::chase_arrows`):
  - `dispatch_game_sim` adds to it.
  - `try_tick_once` counts it every frame outside the main menu, beside the time-played
    statistic of the same `FrontEndManager::Update`.
  - `unload_game` clears it.
  - A pres pedestrian is a renderable living-world pedestrian with its presentation, as for the
    pedestrian visuals.
- The visual snapshot carries the arrows. Each frame the app places them at that frame's
  restored pedestrian, the local skater's hips and the camera. It draws `GpuScene::chase_arrows`
  before the APT levels while `MarkerUi::chase_arrows_shown` holds: no front-end state, NIS or
  settings hides the HUD, and the challenge HUD manager has no visibility flag set
  (`HudBinding::hidden`).
