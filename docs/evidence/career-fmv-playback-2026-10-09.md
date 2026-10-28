# Career FMV playback (2026-10-09)

The front end's movies (FMV) in the native order: the EA chunked `.vp6` files decoded in safe
Rust (VP6 video, EA-XA sound), FE state 28 and the front end manager's movie player in the core,
and the presentation's `AptMovieIntegration::Render` in the PC app. All addresses are
`sk82_na_m.xex` (Jan 17 2009) VAs read from the live IDA database, read-only.

## Files

| File | What |
| --- | --- |
| `crates/skate2-formats/src/ea_movie.rs` | `ContainerParser_EAChunk::ParseFunction` (0x82b88ff8): chunks, `MVhd`, `SH`/`SD`+language, `SNDI_patchtohdrgen` (0x82b8d508) header tags. |
| `crates/skate2-formats/src/ea_xa.rs` | `AudioDecoder_EAXA::Decode` (0x82b955b8), `CEAXABLKDecf::Decode` (0x82b94ef0), `decodexac` (0x82b94b40), raw `0xEE` frames (`process_raw_block` 0x82b948d0). |
| `crates/skate2-formats/src/vp6.rs`, `vp6/{tables,idct,tests}.rs` | The `PB_INSTANCE` decoder (`VP6_DecodeFrameToYUV_internal` 0x82f10318, `VP6_LoadFrame` 0x82f14788, `VP6_DecodeFrameMbs` 0x82f14040, token readers, bilinear prediction, 48-px UMV border) with the VMX128 IDCT blocks of `VP6_DecodeBlock` (0x82f17c30 DC, 0x82f17ce4 ten-coefficient, 0x82f18390 full). |
| `crates/skate2-audio/src/rwac/plugins/packet_player.rs` | `rw::audio::core::PacketPlayer` ('PPl0', desc 0x8300f740). |
| `crates/skate2-audio/src/movie.rs` | `rw::movie::AudioRenderer_Rwac` ("RWMovie Voice"). |
| `crates/skate2-audio/src/{rwac/core.rs,rwac/plugins/mod.rs,system.rs,lib.rs}` | The plug-in kind, its three command-buffer events, the movie renderer in `AudioSystem` and its `TIMERSTAGE_BEFORECOMMANDS` timer. |
| `crates/skate2-core/src/ui/movie_player.rs` | FE state 28, `FEMoviePlayer`, the front end's half of `AptMovieIntegration`, `GetFMVMovieKeyFromName`. |
| `crates/skate2-core/src/session/selected_session_movie.rs` | The session's movie player, the new career's `NewCareerIntro`/`RedaIntro`. |
| `crates/skate2-core/src/session/selected_session_career_movie.rs` | Career pushes, `cMsgFMVFinishedPlaying`, the sign-up poster's tutorial video. |
| `apps/skate2-pc/src/movie_playback.rs` | `AptMovieIntegration::Render` with the game's `MoviePlayer` and `rw::movie` pacing; reader and decoder threads. |
| `apps/skate2-pc/src/gpu_renderer/movie.rs`, `movie.wgsl` | `VideoRenderer_RwTexture` and the `vp6` effect. |

Small edits elsewhere: `challenge/signpost.rs` (message type 15), `challenge/system.rs`
(`Output::PlayMovie` carries the key), `challenge/tests.rs` (the start machine now waits for the
movie: `new_career_start_state_machine_reaches_first_mission` expects `tutorial_skate01` and
posts `FmvFinishedPlaying`, and the three first-mission drivers post it after `HudYes`),
`session/selected_session_career.rs` (field, output, poster X), the facade (field, hook, new
career, audio state, `audio_is_fe`), `ui/mod.rs` (`FrontendAudioState::MOVIE_PLAYER`),
`ui/frontend_fade.rs` (`EnableBackground` snap), `gpu_renderer.rs`, `game_audio.rs`,
`live_skating_view.rs`, `live_skating_client.rs`, `render_inspection.rs`, `main.rs`,
`docs/career/README.md`.

## Native flow

### Start sites

| Site | Movie (`fe_movie`) | How |
| --- | --- | --- |
| `cSignPostUtil::HandleCaughtMessage` (0x825de4e0, type 15 at 0x825de710) | `FMV_ID` → `GetFMVMovieKeyFromName` (0x825ca340): the last collection, in `GetCollections` order, whose layout name equals it | With `kEnableSignPostMedia` and `kEnableProgressionMovies` (0x82f55d90, default 1): `mMovieKey`, `PushState(28)`; key 0 sends `cMsgFMVFinishedPlaying` (0xACAEBECE) at once. |
| `StateMainMenu::NewCareerIntro` (0x82763790) | `god_intro1` (`intro_movie`) | Entered from `InFEState` (0x82777dec) once `EnterGame`'s (0x826788b0) flow message 28 set `mDoEnterCACFromMainMenu`; `ForceBlackOnClose`, `SetState(28,-1)`; on to `RedaIntro` once the state left the stack after `IsPlaying` was seen. |
| `StateMainMenu::RedaIntro` (0x8275f5f8) | `redacarride` | Same, then `NewCAC` (0x82757ba8). |
| `FrontEndState_SignUpPoster::ReceiveInput` (0x82657bf8) keycode 5 | `challenges.TutorialVideoAsset` | `GetHasTutorialVideo` (0x826480d8: `TutorialNISHalTitle` ≠ `ID_COMMON_NONE`): actions blocked, render effect and `DisableSignUpSlowDown` (0x82da1ae0), `cMsgSetSimRate` (0x3FBF5E43) of one 1/60 tick, `mDisabledSignupSlowdown`, `cMsgHUDOverlay` (0x4116DB66), `PushState(28)`; else `core_error` (0x47f720222f2caed4). |
| `StateBootFlow::Splash1State`/`Splash2State` (0x827773a0/0x827772a8) | `ea_intro`, `blackbox_bumper` | Boot only (not in the port). |
| `FrontEndState_BootFlow::OnUpdate` (0x8265ab7c), Extras, credits, `cServiceManager::PlayServiceMovie` (0x8289d0f8), online hub | attract, extras, credits, services | Outside the training area; `PlayServiceMovie` has no port native yet. |

Message type 15 senders (handler Lua scan of `progressioncore`): `Start State Machine` (FMV -
Tutorial Skate 01), `Trigger_m_1b_05_spotfilm_sponsor_me_5` (Tutorial Map),
`Trigger HOM_tutorial`/`END_HOM_TUTORIAL` (HOM), `Trigger_m_8_challenge_completed` (Sponsor
Training 01-14), unlock and service triggers. `FMV - Unlock Mongocorp Interior` names
`Unlock_MondoCorpInterior`, which has no `fe_movie`: it finishes at once.

### FE state 28 (`FrontEndState_MoviePlayer`)

* `gFEStateInfo` (0x82fded80) row 28: `mbPause` 1, `mShowHUD` 1, `mFEMusicState` 10.
* `SetState` (0x82643820) clears `mbPlaying` (and swaps `GameBackEnd::mGameState` for 2, not
  ported). `mIsSkippable_Override` is 1 from `StateManagerFrontEnd::Initialize` (0x8268ca08) and
  every `OnLeave`, never cleared.
* `OnEnter` (0x8265bb08): `PauseForMovie`, `LimitModelCategoryRecipes(MarqueePedestrian, 0)` (not
  ported), `mMovieState = 1`, screen 33; the 64-bit keys `tutorial_basicreplay`, `tutorial_map`,
  `tutorial_createaspot`, `tutorial_advancedreplay_dlc`, `hom` draw under the front end
  (`mbForeground = 0`) at `kTutorialMovieScale` (0x82f59e30, 0.85) and open FMV HUD templates
  0x8A, 0x8B, 0x8C, 0x8D, 0x8E (waiting for `FMVHUDScreenReady`).
* `OnUpdate` (0x8266a880): not playing → once the screens loaded `PlayMovie` (0x8265bf98); playing
  and the movie no longer `Playing` → `CanLoop` (0x82615378) replays, else `PopState`.
* `ReceiveInput` (0x8265bea0) while `Playing`: Start (0) / A (2) pop when
  `(mAlwaysAllowMovieSkip || IsInterruptible) && (IsInterruptible || !CanBackOut)`; B (3) pops when
  `CanBackOut` (0x826152f8). `mAlwaysAllowMovieSkip` (+0x2305) is cleared by the front end
  manager's constructor (0x82635f18).
* `OnLeave` (0x8265bd78): `mMovieKey = -1`, screens closed, `FEMoviePlayer::Stop`,
  `ResumeAfterMovie`, foreground and scale 1 again.

### `FEMoviePlayer` (front end manager +0x1528)

* Constructor values: `mInterruptTimer` 0, `mInterruptThreshold` 1.0, `mMovieState` 1.
* `Play` (0x82638fb8): `fe_movie` layout byte 12 clear → threshold = 0x0ac11eda5b6e4e50 (5000;
  10000 for `intro_movie`); subtitles bottom, `SubTitler` for a subtitled movie (0xa3a061ec3931b4df)
  with the profile option on (draws under the front end); timer 0; `Init("movies"` (0xc4b18d4b2056ccf1)`,
  "<FileName>_english_ntsc", GetMovieVolume × layout float 2, play)`; `PauseMan` push with
  0x7b15f1c243290782; `mbDisableBackground`.
* `Update` (0x82628df0): `FrontEndManager::Update` (0x82640130) passes the raw step
  (`GameModule::Update` 0x8273ee88: 0x682), the timer adds `0x682 / 100` = 16 ms; background off
  (`EnableBackground(0, 0)`) once; not `IsPlaying` → `Done`, `cMsgFMVFinishedPlaying`.
* `Stop` (0x82639380): subtitler and line cleared, `AptMovieIntegration::Stop`, `PauseMan` pull.
* The boot flow's `ea_intro` and `blackbox_bumper` both set the threshold to 5000; every movie with
  byte 12 set keeps it. The port starts at the main menu, so `boot_flow_played` applies those two
  `Play`s' threshold when the movies are first read.

`ReceiveFMVFinishedPlayingMessage` (0x825d2f18) runs every running state machine's
`FMVFinishedPlaying` script; the file name in the message is not read.

### `AptMovieIntegration` and the players

* `Init` (0x82634c30): `data\movies\<name>.vp6` (`GetMovieType` 0x82601ea0), audio filter `EN`
  (FR/GE for French/German), refused while `mMoviePlayer` exists. `IsPlaying` (0x8260a1d8):
  playing, opening, or a request queued.
* `Render` (0x82630198), each `AptRenderingIntegration::Render2D` (0x82631b70; a movie not in the
  foreground before the APT levels, one in the foreground after them): a stop request draws the
  black background (0,0,0,1) for 3 renders then `DoStop`; requests: Play → `DoOpen`/`DoPlay`,
  Stop → `RequestStop` (snap black with `mbForceBlackOnExit`); while playing: background,
  `MoviePlayer::Update` and `Render`, `SetVolume` once, a non-playing status → `RequestStop`.
* `DoPlay` (0x8260a248): position (640, 360 − (1 − scale)·360), the scale; widescreen →
  `mFullScreen` 0. `MoviePlayer::Init` (0x826a3b18): `VideoDecoder_Vp6` (YV12, jobs), 4
  renderables, `VideoRenderer_RwTexture`, `AudioDecoder_EAXA`, `AudioRenderer_Rwac`.
* `rw::movie::MoviePlayer::Decode` (0x82b85230): goal frame = `(int)((delay + elapsed) · fps ·
  0.001)`; delay = `GetVideoAudioDelay` = 0 (both renderers' delays are 0); elapsed =
  `MovieTimer::GetElapsedTime` (0x82b80a80): the audio renderer's rendering position, else the
  stopwatch. `MovieDecoder::Decode` (0x82b84380) keeps the oldest filled frame at or past the goal,
  else the newest. `Render` (0x82b85568): state 2 → 3 starts the stopwatch and `Begin`; the last
  frame with the streams done ends the movie (state 5) → `COMPLETED`.
* `VideoRenderer_RwTexture::Render` (0x826a3e18): three L8 textures (Y, blue, red), one quad
  `posX ± scaleX·w/2`, `posY ± scaleY·h/2` on the ortho screen camera; the `vp6` pixel shader:
  `c254 = (1.16438305, -0.391761988, -0.0625, -0.5)`, `c255 = (1.59602702, -0.812968016,
  2.01723194)`, colour write RGB.

### Sound

* `AudioRenderer_Rwac::Init` (0x82b8c678): `PacketManager` 16 × 256 samples; "RWMovie Voice" at
  stage 0: PacketPlayer, Rechannel, Resample, GainFader, Send (all with the movie's channel count),
  the fader at the constructor's volume 0 at once, the Send on the mastering sub mix.
  `PrepareRenderer` (0x82b80b80) passes length 0 and the finished callback stays null.
* `Begin` (0x82b8d320): play event (format 0, rate, channels), `RwAudioCoreTimer` (0x82b8ceb0,
  before commands): frees finished packets from the tail (event 3), fills up to two packets
  from the decoder (each filled packet allocates a second, unused ring slot), submits (event 2).
* `SetVolume` (0x82b8c988): 0.02 s fade. `GetRenderingPosition` (0x82b8c868): attribute 0 × 1000.
* PacketPlayer: decoder `P6L0` (`sDecoderGuidLut` 0x822a2a38); `Pcm16LittleDec::DecodeEvent`
  (0x82c14780) scales by `0x38000100`; `RwacTimerClient` (0x82c00918) sets attribute 0 =
  samples decoded / rate before each mix and starts and feeds the decoder; stop arms a 16-sample
  declick.
* `PauseForMovie`/`ResumeAfterMovie` (0x824f0a18/0x824f0a98) from `OnEnter`/`OnLeave`;
  `GetMovieVolume` (0x824e6808) is master output 8.

## Decoders and their checks

* Every shipped movie (40, incl. the 7595-frame `intro_movie`) decodes with no error; the decoded
  sound matches each header's sample count (a scratch probe over `data\movies`).
* VP6 against ffmpeg 4.2.3 (external oracle, scratch only): with the IDCT replaced by the integer
  reference and the ten-coefficient sign quirk off, frames are bit-identical (Tutorial_Skate01 120
  frames, Unlock_GVR and intro_movie 60 each). The native IDCT blocks were run in a VMX128
  emulator of the disassembly; the port's `idct.rs` matches it on random and recorded blocks,
  including the float products, saturating adds and the garbage `vperm` lanes; unit tests
  `vp6::idct::tests::*`.
* Display orientation is upright (rows reversed from the bottom-up decode), checked against
  ffmpeg's frames as PNG.
* `cargo test --release -p skate2-formats -- vp6 ea_movie ea_xa`: 14 passed; with
  `SKATE2_GAME_ROOT` and `--ignored`, `original_tutorial_movie_decodes` passed.
* `cargo check --release -p skate2-pc` is clean for these files. The core's unit tests
  (`ui::movie_player::tests`) and the edited career tests were not run here (core tests are
  outside this task's allowed commands).
* Movie sound through `AudioSystem` (scratch probe, Tutorial_Skate01): the rendering position
  advances 5.333 ms per 256-sample block to the full 32966.27 ms; the stereo output equals the
  decoded source folded as `render_stereo` folds the master mix (RWAC order FL, C, FR, SL, SR,
  LFE) times the volume: correlation 1.000000 at lag 0, largest difference 6e-8.

## Port wiring

* Message type 15 resolves the key in `signpost.rs` and outputs `PlayMovie { key }` (key 0 sends
  `FmvFinishedPlaying` at once); the career hands keys to the session's movie player.
* The session runs `update_movie` each front-end frame before the main menu: pushes, then
  `FEMoviePlayer::Update`, input, the state's update, the intro states; while FE state 28 is on
  the stack (or the new career's intro runs) the frame stops there and the SIM input's front-end
  pause is held. `cMsgFMVFinishedPlaying` goes to the career's challenge system.
* New career: the main menu's leave now starts `NewCareerIntro` (`god_intro1`), then `RedaIntro`
  (`redacarride`), then the existing `NewCAC` path.
* The sign-up poster's X plays the challenge's tutorial video and brings the poster back
  (`OnEnter` 0x82647f68: actions blocked until its intro, slow-down at rate 1).
* The PC's `finish_redraw` runs `AptMovieIntegration::Render` every redraw: requests, reader thread
  (chunks; EA-XA into the movie sound queue), decoder thread (VP6 frames, 4 renderables), frame
  choice from the sound's rendering position, plane upload, and the draw (black background +
  quad on the 1280×720 stage fitted to the window like the flat front-end screens).

## Movies in the training area

| When | Movie |
| --- | --- |
| New Career from the main menu | `intro_movie` (god_intro1), then `RedaCarRide` |
| "New To Skate?" Yes (Start State Machine `FMV_Basics`) | `Tutorial_Skate01`; `m_1a_01` starts on its `FMVFinishedPlaying` |
| X on the posters of `m_1a_03_tutorial_fliptricks`, `m_1a_04_tutorial_manuals`, `m_1a_05_photo_rail` | `Tutorial_Skate01`, `Tutorial_Skate05`, `Tutorial_Skate02` |

All skippable with A/Start after 5 s (the boot threshold; these movies keep it); none can be
backed out of with B.

## Not ported / open

* The FMV HUD screens (templates 0x8A-0x8E, e.g. `tutmap.apt`) are not drawn; they report ready
  at once, so those five movies play scaled at the top without their panel (none is in the
  training area).
* The subtitle line (`MovieSubtitle`) is produced but the PC's subtitle screen does not show it
  yet (subtitles default off for English).
* `RecipeMan::LimitModelCategoryRecipes` and `GameBackEnd::mGameState` swaps; the poster's
  `challenge_screen_amb` loop sound; `cMsgHUDOverlay` (only the Lua binding 0x828dfa90 handles it,
  and no challenge script defines a handler).
* `PlayServiceMovie`, boot, attract, credits and extras movies.
* The port's screens report ready once the presentation holds no player from the movie before
  (native: the APT screen load), so `Init` is not refused for a player still stopping.

## Live checks for the orchestrator

1. New Career: the intro movie plays with sound; A skips only after 5 s; RedaCarRide follows, then
   Create-A-Skater.
2. Career start, "New To Skate?" Yes: Tutorial_Skate01 plays, the game pauses under it, and
   `m_1a_01_goto_slappys` starts when it ends or is skipped.
3. The posters of `m_1a_03`/`m_1a_04`/`m_1a_05`: X plays the tutorial video; the poster returns and
   takes A/B after its intro.
4. Sound level: `GetMovieVolume` comes from the mixer's master output 8; confirm the movies are
   audible at the expected level.
