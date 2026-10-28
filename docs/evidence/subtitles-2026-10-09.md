# Subtitles and the Subtitles setting (2026-10-09)

Scope: the whole subtitle system, for cutscenes and wherever else the real game uses it,
with its settings. This records what the original
offers (debug xex, IDA), what the port had, what changed, and how it was verified.

## The original has no Accessibility page

- `eSk8FEGameOptions` has 25 entries (None, MusicPlayer, AudioSettings, VideoSettings,
  ControlSettings, OnlineSettings, DeviceSelection, SFX/Dialogue/MusicVolume, CameraAngleToggle,
  OffboardY/XAxisToggle, SubtitlesToggle, MiniMapToggle, HUDToggle, VibrationToggle,
  SixaxisToggle, AutoSignInToggle, ViewTOS, SignInOut, UnitsToggle, VideoCalibration,
  GrindTransparencyToggle, ResetCareer) and `eSk8FEGameOptionsScreens` has 10 screens; none is
  an accessibility group. No xex string contains "accessib", and the only English HAL string that
  does is an unrelated online message ("...displayed/accessible").
- The one subtitle setting is `eSk8FEGameOption_SubtitlesToggle`: `gGameOptionsItems`
  (0x82fdf9a8) row 13, title `ID_GAMESETTINGS_SUBTITLE_TOGGLE` ("Subtitles"), type `selector`.
  `GetStringValue` (0x82643d48) shows `ID_COMMON_ON` / `ID_COMMON_OFF` ("On"/"Off") from
  `ProfileBackEnd::tSettings::mSubtitles`.
- It is on every Video Settings screen (`gGameOptionsScreens` 0x82fdfa70, 8 rows each), after the
  camera and display rows; `OnMenuSelect` (0x826871a0) picks the screen:

  | screen (title `ID_GAMESETTINGS_VIDEO_SETTINGS`) | when | rows |
  |---|---|---|
  | 7 VideoSettingsMainMenu | `mFEState == eFE_STATE_IN_FE` | Video Calibration, Subtitles |
  | 3 VideoSettings | in game, not free skate, no rebroadcaster | Camera Angle, Grind-Vision, Video Calibration, Subtitles |
  | 4 VideoSettingsOnline | in game online | Camera Angle, Grind-Vision, Subtitles |
  | 5 VideoSettingsFreeskate | free skate | Camera Angle, Grind-Vision, Video Calibration, Subtitles, Minimap, HUD |
  | 6 VideoSettingsFreeskateOnline | free skate online | Camera Angle, Grind-Vision, Subtitles, Minimap |

- Input: `ReceiveInput` (0x82686978) keycode 14 (left) or 15 (right) on the row sounds
  `core_nav_left` / `core_nav_right` (`GlobalFEPlaySound` keys whose low words 0xb4997b49 /
  0x1c45ea28 are `Attrib::StringHash64` of those names), then `ToggleSubtitles` (0x8265c5e8):
  `mSubtitles = !mSubtitles` either way, telemetry, `mbAllowInput = 0`, `mIsDirty = 1`,
  `SetScreenState(mScreenId, 50)` (the row's refresh).
- Save: `RequestExit` (0x82681700) requests `eBeMemcardSave_Profile` when `mIsDirty` and no main
  challenge runs (or the game mode is Party Play). The flag is the fifth tuple of
  `ProfileBackEnd::Settings` v3 (`SaveSettings` 0x8255bc68, `WriteBool(mSubtitles)`; `LoadSettings`
  0x8255b950).
- Default: `tSettings::Default` (0x825644d8) sets `mSubtitles` only for Italian, Spanish, Japanese
  and Dutch: off for the English game.
- `ID_SETTINGS_SUBTITLE` ("SUBTITLES: {0:~}") sits in the HAL tables beside "MOTION SENSOR
  FUNCTION" and "VISUAL FX" summaries but nothing in the xex or the screens references it.

## Where the original shows subtitles

One line exists: `FrontEndManager::mGenericSubtitles` (`FESubtitle`, +0x1EC0), shown by main.swf's
subtitle layer (`subtitle.apt`: `RefreshSubtitle()` reads `Game.GetMovieSubtitleString()`
0x82636e50, "#" for no line, and shows `mcTextSubtitle` and the `mDimmer` band). Nothing in the
APT reads `mbSubtitleBottom`; that flag only moves the system notifications (`XNotifyPositionUI`).

- Lines arrive only as `cMsgSetSubtitle` → `ReceiveSetSubtitleMsg` (0x8261ba00) →
  `SetSubtitleText` (0x82615ec8). The message (kind 0x61363201) is built only by
  `SubTitler::Update` (0x8269fbd0) and the Lua exporter `cMsgSetSubtitle_PostMessage_LuaBinding`
  (0x828c4568). No shipped script calls the exporter: the challenge handler blobs in db.big and
  `global.lua` hold no `cMsgSetSubtitle` string, and the kind is in no vault as static message
  data. `cMsgNISSubtitle` (kind 0x92cf84c3) has no sender in code and only Lua handlers, which no
  script defines.
- A `SubTitler` is made only by `Scene::Init` (0x82877ac8) and `FEMoviePlayer::Play`
  (0x82638fb8). So the two places with subtitles are:
  1. NIS cutscenes: `ScenePlayer::Create` (0x82878590) sets `mIsControllingSubtitler` (SceneLite
     scenes do not; replay scenes go through `StartNIS` too);
     outside an online game `Scene::Init` starts `SubTitler(scene name)`, which loads
     `SubTitles_<language>\XBOX2_<name>.sub`, then the English, then without `XBOX2_`.
     `Scene::Update` feeds it the scene time. It runs whatever the option.
  2. FMVs: `Play` clears the line, turns the time-out off and, for a `Subtitled` `fe_movie` with
     `mSubtitles` (or `EuropeanSubtitles` for French or German in Europe, which also sets
     `mbOverrideProfileSetting`), draws the movie under the front end (`mbForeground = 0`) and
     starts `SubTitler(file name)`; `FEMoviePlayer::Update` (0x82628df0) feeds it.
     `Stop` (0x82639380) deletes it, clears and refreshes the line and turns the time-out on.
     46 of the 47 `fe_movie` collections are Subtitled (all but blackbox_bumper).
- Mission speech (presentation events 2 and 3), NPC and phone speech, tutorial callouts and the
  HUD have no subtitles: no other code reaches `SetSubtitleText`.
- Gating: `SetSubtitleText` formats the text only with `mSubtitles` or the override, and
  `RefreshSubtitle` (0x82614c80) calls the screen only with either; `ClearSubtitle` (0x82615e68)
  refreshes the same way. With the option off a NIS still sends its keys; nothing shows. The auto
  time-out (`kAutoTimeOutSubtitleMinTime` 1000 ms + `kAutoTimeOutSubtitleLetter` 45 ms per
  letter, or a leading `[ms]`; checked in `FrontEndManager::Update` 0x82640130) is off inside
  `FrontEndState_NIS` (`EnableAutoTimeOut` from `OnEnter` 0x82658610 / `OnLeave` 0x8266fd70) and
  between `Play` and `Stop`; turned back on with a zero time-out it clears a line left by a
  skipped scene on the next update. It would only time script lines, which the data has none of.
- Options cannot open mid-line: `FrontEndState_NIS::ReceiveInput` (0x826484a0) only skips on A,
  and FE state 28 (movie player) holds the front end. Its `gFEStateInfo` row (0x82fded80 +
  28 * 20) pauses the game and hides the HUD.
- Data: data\fe\subtitles\subtitles.big (inside miscboot.big) has 251 English .sub files: story
  NISs (01_Welcome_Back .. 22_SlappyEnder), challenge intros and outros (besttrick_*, film_*,
  photo_in_*, ownspot_*, Goto_*, c_*), tutorial NISs, and FMVs (intro_movie, RedaCarRide, HOM,
  GVR, SponsorTraining01-14, Tutorial_*, XBOX2_Tutorial_Skate01/03).

## What the port had

- NIS lines: `nis::subtitles` (SubTitler) → `NisSystem`'s `FeSubtitle` → `CareerHud.nis.subtitle`
  → `marker_ui/subtitle_movie.rs`, gated by `CareerHud.subtitles`, which the career read once
  from the save when it loaded.
- FMV lines: `ui::movie_player` ran the movie's SubTitler and kept `MovieSubtitle`, and drew a
  subtitled movie under the front end with the option on, but nothing showed the line.
- No Subtitles row in Options; `ProfileSettings.subtitles` was already read and written.

## Changes

- `crates/skate2-core/src/ui/options.rs`: `Row::Subtitles` (selector,
  `ID_GAMESETTINGS_SUBTITLE_TOGGLE`) closes the Video Settings page after the PC rows that stand in
  for Camera Angle / Grind-Vision / Video Calibration; `Snapshot.subtitles`; left or right sounds
  core_nav_left/right, flips `ProfileSettings.subtitles` and marks the profile dirty, so the exit
  saves it (`request_options_exit` → `save_audio_profile`). Unit test.
- `apps/skate2-pc/src/marker_ui/settings_movie.rs`: the row's On/Off value; the Video page test
  checks its title and values.
- `crates/skate2-core/src/session/selected_session_career.rs`: `CareerHud.subtitles` is the
  profile's `mSubtitles` as it is now (`take_career_hud`), so a toggle in game applies at once.
- `apps/skate2-pc/src/marker_ui.rs`, `live_skating_view.rs`, `live_skating_client.rs`: while FE
  state 28 is on the stack the subtitle layer takes the movie player's line (gated by the option
  or the override), the HUD is hidden, and a front-end frame is drawn when no other front-end
  state draws (the movie under the front end used to show the last HUD frame over it). When the
  state leaves, its cleared line refreshes the layer.
- `marker_ui/subtitle_movie.rs`: docs; `SKATE2_TRACE_SUBTITLES` logs each refresh.
- `nis/manager.rs`: the time-out comment names both line sources.
- Diagnostics (career demo): `SKATE2_DIAGNOSTIC_SUBTITLES_OPTION[=<tick>]` drives the main menu's
  Options → Video Settings → Subtitles → right → Back, Back (save) before New Career at tick 900,
  and prints the saved value; with a tick the demo ends there.
  `SKATE2_DIAGNOSTIC_SUBTITLE_SHOT=<n>[,<ms>]` ends the career demo (and captures) that long after
  the n-th front-end line, shown or not.

## Verification (2026-10-09)

- Release build of skate2-pc; `PARITY IDENTICAL (6000 lines)`.
- `cargo test --release -p skate2-core --lib ui::options`: 3 passed (with
  `subtitles_close_video_settings_and_toggle_the_profile_setting`). With SKATE2_GAME_ROOT,
  `cargo test --release -p skate2-pc --bin skate2-pc -- --ignored subtitle
  display_preferences_original_options_layout_and_lifecycle`: 3 passed (the Video page test
  checks the Subtitles row's title and On/Off).
- Fresh scratch save, subtitles off (default): the first career NIS
  (Goto_m_1a_01_goto_slappys_outro_1, key 0x04517f19 at 0.0 s) captured 3 s in: no line.
- Fresh scratch save with the Options diagnostic: "SKATER.P holds true, the profile true"; New
  Career's intro movie then showed "Electronic Arts presents" over the movie (captured). Another
  fresh run captured the Video Settings page at tick 770: Subtitles / On on the fifth row.
- Next run on the toggled save without the diagnostic: "profile loaded with true" (load round
  trip); lines in order: intro_movie 0x6dde67a8, RedaCarRide 0x840ca572, the NIS 0x04517f19
  (captured 3 s in, the same frame as the off run, with the line and its band). A run on a copy
  of that save with Yes on "New to skate?" captured the in-career tutorial FMV (Tutorial_Skate01,
  0xa9a66ac3) with the line over the movie and no HUD.
- Career smoke with subtitles off (movie skip, Yes on "New to skate?"): the NIS, the tutorial FMV
  (drawn over the front end as before) and m_1a_01 gameplay to 2400 ticks with the HUD back.

## Open

- An "Accessibility" page would be a non-native addition (new row, page and labels with no HAL
  strings). The native setting is on Video Settings; a separate page would be a
  deliberate deviation.
- The other native Video Settings rows (Camera Angle, Grind-Vision, Video Calibration, and free
  skate's Minimap and HUD) are not ported.
- The European French/German override is not modelled (English NTSC data).
- The tutorial FMVs' HUD screens (movieplayer\tut*.apt) are still not drawn.
