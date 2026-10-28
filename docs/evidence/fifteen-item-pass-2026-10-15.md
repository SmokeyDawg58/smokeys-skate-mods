# Fifteen-item pass (shadows, AI spins, replay cameras, particles, online)

PC source changes, October 15, 2026. Built with
`cargo build --release -p skate2-pc` and run off-screen with scratch saves only
(`SKATE2_SAVE_ROOT` under a scratch folder; the real `saves/SKATER.P` was
copied read-only where a progressed career was needed). Native addresses are the
debug `sk82_na_m.xex`. Selected-state parity (`perf-skate`, Permission Pool,
6000 lines against the 10-14 baseline): **identical** after item 2 and again after
the particle work.

## Status

| # | Report | Result |
| --- | --- | --- |
| 1 | AI dynamic shadow vanishes when spectating a far AI (S.K.A.T.E.) | Fixed, checked in captures |
| 2 | AI skaters never spin (Shingo, Alphabet Yard S.K.A.T.E.) | Fixed, verified |
| 3 | Sign-up prompt flashes after a challenge-map teleport | Fixed, verified |
| 4 | Replay camera in a strange place after Get Blind With Jake | Fixed in code; NIS verified, highlight reel not reproduced |
| 5 | PostFX on in cutscenes with PostFX off | Fixed in code |
| 6 | Filming/photo AI does not aim its camera at the player | **Open**, needs the exact reported case |
| 7 | Missing video settings per mode | Added; not live-checked this session |
| 8 | Replay camera goes underground on hills | Ported; not live-checked this session |
| 9 | No smoke from the boot-wall manhole; stadium lights look off | Particles ported and verified; stadium glare audited 1:1, **needs a reference** |
| 10 | Filmer Pack tutorial does not play from Extras | DLC movie absent; returns cleanly, verified |
| 11 | Online wall: palms without wind, blocky tree shadows, exposure swing | Wind and exposure fixed, verified; shadows are native |
| 12 | Pedestrian hand-prop arm lift repeats | Fixed in code |
| 13 | Throwdowns from the services menu stay on a white screen | Fixed, verified |
| 14 | "Xbox LIVE" title in the Online section | Now "Online", verified |
| 15 | Online research, nickname config, first-visit popup | Done, verified |

## 1. Spectated AI shadow

The CSM depth range followed the human skater. Native `GetLocalIndex` (0x827d0090)
answers the presentation's local flag, which `Online.SwitchCamera`
(`SkaterManager::SetLocalIndex`) moves to the spectated AI. The port now resolves
the local presented skater across party and AI skaters
(`selected_session_public_facade.rs`, `local_presented_skater`). The log shows the
`SetLocalIndex` switches; captures show the AI's shadow while spectated.

## 2. AI spins

`PhysState_KnownAir::CalculateBodySpinSpeed` (`lfs +0x960` at 0x82d8bc24) and
`Reckoning::StartFlipping` (0x82dd8880, times 4.0) read the processed input's
`mBodySpin` (+0x960, the negated BodySpin animation attribute). The port read
`mPhysBodySpin` (+0x9f8, `GroundScalar::BodySpin`), which is 0 for AI. Fixed in
`physical_player_known_air_rotation.rs` and `reckoning_air.rs`; the Reckoning
air/grind/ground reads of +0x9f8 were already right. Run sp5: the AI landed a 540
(three spins) and a 180, its root following the path rotation.
`SKATE2_TRACE_AI_SPIN` (path controller) stays as a diagnostic.

## 3. Sign-up prompt after a map teleport

Added the native gates: the memory-card check (0x825a4488) and the flow's
`STATE_Loading` (0x825ada38). Run mt3 (progressed save, Map, Pro, Navarrette,
Teleport): one sign-up prompt publish after the load, steady capture. A fresh
career's map lists nothing, and the first mission's pause menu has no Map row;
both native.

## 4. Replay camera after Get Blind With Jake

ICE avatar 1 (the replay camera, `CameraPresentation` current) was unresolved,
so the camera stayed where it was. Added the replay and hips-forward anchors
(0x826d1608, 0x826cf5f0). Run jake5: NIS `09_ProChallenge_JakeBrown` played without
"ICE avatar not resolved". With forced objectives there is no recorded replay, so
the highlight reel itself was not reproduced.

## 5. PostFX in cutscenes

The renderer now takes PostFX from the NIS's ICE PostFX while a cutscene runs
(`set_cutscene_postfx`), whatever the menu setting, as the switch happens under
the fade.

## 6. Filming and photo AI (open)

In `m_1a_05` the filmers are AI skaters (players 1 and 2, `FilmAIID`, spawned,
pinned and pathed); the skater graphs have no camcorder or film animations, and
`OB_LookAt` intents come only from the human listener
(`ActionGraphInputListener::Fill`). The marquee photographer pedestrians
(`Motiongraph_Photographer`: TrackTarget, PHOTO_SHOOT_BLEND,
`PhotographSkater::Update` 0x82717018 setting PitchToTarget/BearingToTarget) are
ported 1:1, but none spawn in `m_1a_05` (`SKATE2_TRACE_PHOTOGRAPHER` never fired).
The reported case is needed.

## 7. Video settings

The native per-mode screens: wall (calibration, subtitles); career (camera, grind
transparency, calibration, subtitles); Free Skate (plus minimap and HUD). Saved in
the profile at +0x8a (minimap), +0x8d (HUD), +0x9c (camera), +0xa3 (grind). The PC
Display Settings page stays separate.

## 8. Replay camera under hills

Ported ICE `UpdateConstraints`, `ResolveAgainstStaticObjects` and `HandleCollision`
(0x826d0268, 0x826cff00) with the world fat line.

## 9. World particles; stadium lights

### Native

`Sk8::ParticlePresentation` (Init 0x827ec4a8, Update 0x827ed528, Draw 0x827ea9a0,
called from `RenderMan::RenderModeScene` at 0x82857d14 after Render3D and the crowd
imposters, before the visual indicators and `PostFX::End`).

- Descriptors: `cParticleEffectDescManager::Init` (0x82de5838), the leaf
  `particleeffects` collections (class 0x492acce7fbc7cfdf), the last of each type
  wins; layout 0xc0 with a `Texture` string pointer at +0x80. Types: Smoke, Steam,
  Leaf, DustLand, DustSlide, Pebble, GrassLand, GrassSlide, Water, DMODustLand,
  DMODustSlide, Fluff, Custom0, Custom1.
- Emitter waypoints: `Presentation::RegArena` (0x827f1430) gives each asset of type
  0x1d to `cWaypointManager::AddWaypointData` (0x827ed340). These
  `tEmitterWayPointData` objects (arena type 0x00eb001c) live in the **streamed
  presentation arenas**, not the global locator arenas. `AddBlockToKDTree`
  (0x827ec3c8) keeps only groups whose instance key is a `particleeffects_locators`
  collection, in a 256-waypoint tree. `RemWaypointData` (0x827ee790) stops each
  group's effect (`RemWaypoint` 0x827ebdc8 → `DestoryEffect` → `StopUsingEffect`)
  and regenerates the tree (0x827ec8b0).
- World controller (`cWorldParticleController::Update` 0x827ee178): up to 50
  waypoints within 250 m on x/z; a group starts its effect at its waypoint inside
  `visible_distance`, stops it past 1.5 times that distance squared. The ambient
  effect (`particleeffects_ambient` default, RefSpec 0x138ba0c311b204f8) follows the
  local skater outside x (−360, 215), z (−2200, −1850).
- Emitter (0x82de67c8), particle (0x82de59e0) and quad (0x82de6ac8) arithmetic,
  random stream (seed 0x12345678 + 42, `seed * 69069 + 41`), effect limits (64 per
  type, 2000 particles per type buffer). The colour is `fctiwz(lane * 255)` per lane,
  shifted together unmasked and stored as A, R, G, B bytes (0x82de6c78..cdc).
- `lionREC` (render.big `final/lionREC.xml`): position times `g_matVP`, texture
  times vertex colour; SRCALPHA/INVSRCALPHA, depth test without write, alpha test
  GREATEREQUAL 1, no sampler block (pass reset: WRAP, LINEAR, POINT mip).

### Port

- `crates/skate2-core/src/rendering/world_particles.rs`: descriptors and locator keys
  from `skater.vlt` (the layout read in two plain halves around the text pointer),
  `EmitterWaypointData`, the world controller, the ambient effect, the particle
  system and the quads.
- `PresentationArena::emitter_waypoints` is filled in `own_presentation_arena`;
  the inspection keeps the resident arenas' blocks across map builds, and
  `sync_waypoints` follows registrations and departures.
- `apps/skate2-pc/src/gpu_renderer/particles.rs` and `particles.wgsl`: the `lionREC`
  pipeline, one quad list per type with `particletextures.rx2`'s texture, drawn in
  the scene pass before the visual indicators.
- `apps/skate2-pc/src/particle_view.rs`: loads once, steps at 60 Hz, in game and in
  the front end. `SKATE2_TRACE_PARTICLES` logs the waypoint count and quads per type.
- Not ported: the skater and living-world particle controllers (dust, grass and
  pebble kicks), `cPebbleEffect`'s own quads. The ambient anchor is the local hips
  plus one metre; native uses the skater's audio centre of mass plus an up offset.

### Verified

- pt3/pt4 (main menu, Extras): 41 waypoints, steam (type 1) at about 35 particles
  from the boot-wall manhole; the capture shows the plume.
- st4 (`F_WF_StadiumInterior`): 49 waypoints, the ambient Fluff effect at 90
  particles around the skater.

### Stadium lights

The stadium interior and exterior use only ported effects (no diagnostic draws):
Glare (2 and 3 instances), the incandescent family, no GodraySpotlight. The shipped
`glare_defaultVS`/`PS` microcode (`tools/xenos_disasm.py`) matches the port: the
quad is rebuilt on the view rows scaled by `glareScale`, its depth pulled to a point
half a scale along view row 2, and the pixel program returns the texture; states ONE/ONE
add, no depth write, alpha test 1. The one difference is the sampler: `glare.xml`
asks for 8x anisotropic with the reset's POINT mip filter, which wgpu cannot express
(anisotropy needs a linear mip filter); on camera-facing quads it has no visible
effect. Without a reference image of what looks wrong, nothing else was changed.

## 10. Filmer Pack tutorial

Extras row 6 requests `data\movies_dlc\Tutorial_AdvanceReplay_english_ntsc.vp6`,
which is DLC and not in the game data; the open fails, playback stops and the
Extras list returns (run ex2). The row only shows with `REPLAY/filmpackmovie`
unlocked (`GetNumOptions` 0x82643090). Placing the DLC movie at
`data/movies_dlc/` under the game root plays it.

## 11. Online wall

- Wind: the front end's world animation clock never advanced. It now advances
  with the front end's frame time (`WorldAnimationClock::advance_scene`), and the
  palms move.
- Exposure: it swung even before this pass. Hitches of up to 1.5 s (front-end AI
  skaters rebuilding objects) let the adaptation run many steps on stale
  luminance. Ported `PostFX::Tonemap`'s `m_bFEOnlineMenu` hold (0x82829248; set by
  `FrontEndState_MainMenuOnlineOptions::OnEnter` 0x8264b6b0, cleared OnLeave
  0x8264b724: exposure steps −0.05 to a 0.59 floor) and at most one adaptation step
  per rendered frame. Run fe4: brightness constant.
- Front-end shadow casters (camera and the first front-end AI skater) so the
  world and CSM passes run in the front end, whose world pass is ungated natively.
- Blocky tree shadows: baked lightmap texels; the shipped tree/base PS microcode is
  a single lightmap fetch. Native, unchanged. A texture-foci attempt had no effect
  and was reverted.
- Not fixed: the front-end AI skaters' periodic object rebuilds (~1.5 s hitch).

## 12. Hand-prop arm lift

Native `PlayChannelAnim` passes cycle 1 for the hand-prop idle (`UpdateHandProp`
0x82e9d6f8, `UpdateHandPropAction` 0x82e9cf40), HandPropReadCyc,
HandPropPhoneTalk and TazerCyc; 0 for throws, start actions, ReadOut and
PhoneHangup. Added `request_channel_cycle` in `pedestrian_hand_prop.rs`. Not
checked visually.

## 13. Throwdowns

`Output::TeleportStartChallenge` was unhandled; it now runs
`TeleportLocalPlayerStartChallenge` (0x828a9678): map travel, then the start
during the load. Verified.

## 14–15. Online

The letterbox title reads "Online". The first visit asks for a nickname
(`ui/profile_name.rs` prompt), saved to `online-config.json` (run fe3:
`{"nickname": "Skater99"}`). Research: `docs/online/replay-data-netcode.md`
(replay frames as the netcode: 1,068 bytes per skater per tick measured with
`SKATE2_TRACE_PRESENTATION_SIZE`, about 1.37 KB with audio).
