# PC Display Settings — October 2, 2026

Further PostFX appearance correction is deferred. This adds
independent PostFX/fisheye switches, saved in SKATER.P, reachable from both
startup Options and paused Game Settings. Main-menu Xbox LIVE becomes Online.
These switches and the English Display Settings label are intentional PC additions.

## Live IDA checks

- Options::OnEnter `8265c468` opens screen120 and clears its dirty/save flags.
- LoadMainMenu `826714d0` selects frontend versus in-game options.
- OnMenuSelect `826871a0` enters the native Video Settings subpage, selecting
  different modes for frontend, career, free skating and online play. The
  source has Video Settings but no PostFX/fisheye preferences.
- ReceiveInput `82686978`: Back from a child returns to the options root;
  Back from the root requests exit. Left/right toggle native boolean options
  and mark them dirty. A selects a category; it does not toggle selectors.
  The PC Start shortcut exits Options, returning to its parent (not directly
  resuming simulation).
- RequestExit `82681700` requests `eBeMemcardSave_Profile` when dirty and no
  challenge is running (or Party Play); OnComplete `82681930` ends the screen.
  EndScreen `8266b000` returns to the previous frontend state. Our current
  free-skating session has no active challenge; the PC extension commits
  changed preferences before closing. It does not fabricate career completion.
- ProfileBackEnd::SaveData `8256c2c0` calls SaveSettings, SaveProfileData,
  SaveMusicPlayer. SaveSettings `8255bc68` / LoadSettings `8255b950` use
  version3, with matching typed tuple order: SFX/dialogue/music/brightness
  floats; subtitles/minimap/HUD/autosave/rumble bools; units int; silent-signin
  bool; volume-mode/camera-angle ints; inverse-Y/inverse-X/auto-signin/grind-
  transparency bools. Neither new preference has an original field.
- SaveLoadManager::RequestSave `8257ee58` queues the profile save and calls
  TrySaveLoad `8257e5a8`. ConvertOperationToSaveType `82561798` maps it to
  SAVETYPE_PROFILE. SaveCommands `8257b988`, case0, obtains the active profile,
  calls InitFilename with **SKATER** (`8257bab8`), and sends the save flow.
  The existing MemcardProfile codec implements GenerateSaveData `82569f48`'s
  P buffer, native module wrappers, CRC and fixed size (see the linked
  edit-skater-save evidence). The PC filesystem is the host boundary in place
  of console storage selection/sign-in and memcard flow messages.

## Implementation and deliberate PC scope

Both menu destinations own the same `ui::display_settings::Controller` in
SelectedLiveSession. Parent menus remain retained, input stays in the original
PRES group, and no simulation runs behind Options. Back traverses the child
page and saves only when leaving Options; failed IO is not acknowledged as a
successful save. Defaults are On/On to preserve the previous presentation.

The first attempt reused StartHub and was dropped because its
presentation did not match wall Options. That shortcut has been removed.
Fresh live IDA/raw verification: MainMenu::OnMenuSelect826867d8 calls
OnMenuSelectGame82681338, case5. Disassembly826815e8..81610 sets state73,
keepPrevious=true, keepPreviousVisible=false (the decompiler misreconstructs
the by-value transition arguments). ExecutePushState82d694e8 retains/hides
the parent and enables state73's dimmer. gFEStateInfo82fded80, stride20,
row73 has pause=true, dimmer=true. OnEnter8265c468 opens screen120.
gScreenTemplate82fdb358, stride16, row120 is Options/options/options.swf,
type0 (flat); the main/main_options.swf row69 is NOT selected by this flow.

Presentation now loads the actual options APT (FNV9506be8f), using its own
intro/outro and translated scripts. CreateOptions0008e0..ac4 reads dummy0's
authored position (436.85,199.15), attaches HighlightOptionComponent or
HighlightSelectorComponent and stacks by previous.GetHeight() without the
extra CAC six-unit gap. Original GetHeight constants: option36 (002272),
selector58 (001868). Reuses existing control translation/assets, fonts and
selected/unselected timelines. Selector On/Off values are a separate subtitle;
arrows only appear on the selected row and follow auto-sized value text.
SetState50/UpdateItemValue refresh values without restarting selection.
The original global LetterBox title/help and dimmer are rendered separately;
the already-open wall letterbox retains its entrance state. Button help uses
Back then Select for categories, Back only for selectors, matching
GetButtonHelpTextA82644128/B826441a0. Refresh starts at the retained original
button anchor, fixing drift across repeated page changes. Root title comes
from gGameOptionsScreensTitles82fdf97c (ID_GAMESETTINGS_TITLE).

This ports the original presentation for the PC pages, not all
audio/control/network destinations. No nonfunctional destinations are added.
Main wall and pause both have Options/Game Settings -> Display Settings ->
PostFX, Fisheye. The two settings and Display Settings title are PC additions.

The existing native memcard container/CRC/tuple codec and filesystem commit
owner are reused. New wrapper `SK82_PC` contains leaf
`SK82_PC::DisplaySettings`, version1, two one-byte bool tuples (PostFX, Fisheye).
Native Profile Settings v3 is preserved, never repurposed or extended in place.
Unknown module chunks, CAC and other PC wrapper children survive writes.
The fixed 0x400004-byte container and CRC remain unchanged. An absent extension
uses On/On; an unsupported extension version is an explicit error. A new
settings-only profile does not invent a skater recipe or career record. Original
console interoperability of this PC extension is not claimed.

Both clothing and display saves reopen the latest file before merging, so a
retained editor/settings snapshot cannot overwrite changes made by the other
owner. They share encode/parse validation, synced same-directory temporary
file, backup and rollback behavior. Default path remains `saves/SKATER.P`,
overridable by `SKATE2_SAVE_ROOT`.

PostFX Off bypasses exposure feedback, bloom, ghosts and colour-cube processing.
Scene material tone curves/lightmaps/irradiance remain intact. Exposure becomes
1 in the shared world/vehicle/prop globals and character/pedestrian uniforms,
including retained paused scenes. The original supplied exposure is retained
separately so toggling back restores it. Cached scalar writes avoid extra GPU
uploads when no value changed. Fisheye Off uses identity UV sampling and a
white lens mask; it is independent of PostFX. The existing final pass still
draws respawn static; the existing later passes still draw HUD/fades/FPS.
As a PC adjustment, PostFX Off multiplies final world/sky RGB by
1.22, once, before those HUD/fade/FPS passes. PostFX On remains at gain1.0.
On October 6 PostFX Off gained a further +15%: the gain is
now 1.22 * 1.15 (about 1.40), still once and before the HUD.
An explicit SKATE2_BRIGHTNESS override composes with this intentional PC
adjustment; no native lighting/material constants are changed.

## Validation

- Native-format unit tests: both flag combinations roundtrip alongside CAC and
  unknown career chunks; unsupported extension versions cannot be overwritten.
- Isolated production-source harness: controller Back/save boundary and
  independent toggles; concurrent-lifetime stale clothing/settings stores
  preserve each other's data through actual filesystem saves and reopening.
  All nine tests passed (including existing hash/pause tests).
- Initial release build passed. The interrupted first diagnostic verified
  wall-menu exit saved both preferences, but did not finish the pause check.
- Corrected Options original-asset test PASS: actual dummy0 placement,
  58-unit selector spacing, selected-only arrows, no selection replay on value
  changes, repeated page/help-anchor stability, flat/ultrawide geometry,
  intro/outro callbacks and saved-value loading while hidden.
- Gain/lens independence test PASS for all four combinations. Shared original
  CAC control regression PASS, including selectors, arrows and button layout.
  Logs: `artifacts/frontend-source/options-native-tests.log`,
  `options-shared-controls-test.log`. Fixed one stale map renderer test fixture
  to supply its neutral `FrameEnvironment.exposure=Some(1)`.
- Current-source save/controller harness rerun: all nine PASS.
  `artifacts/frontend-source/display-settings-harness-tests.log`.
- Release build PASS (2m29s), `options-native-build.log`. Existing repository
  warnings remain; no workspace-wide lint/test success is claimed.
- Full raw-pad diagnostic PASS, exit0: new isolated profile defaults On/On;
  wall Options opens, both switches go Off, Back saves, Continue Career runs
  113 gameplay ticks, StartHub Settings opens, independent switches toggle
  on/off while paused. Final capture `options-native-pause.bmp` inspected.
- Second executable launch PASS, exit0: profile loads Off/Off, returns to wall
  Options, 0 gameplay ticks, `options-native-wall.bmp` inspected. Corresponding
  `.log` files record loaded preferences and all completion assertions.
- Only `artifacts/frontend-source/diagnostic-saves/options-native/SKATER.P`
  was used by these runs. No other save was touched. No compiler/game
  process remains; current release is ready via `play-quiet.bat`.

Confidence: exact native destination/control constants and shipped APT
selection; high for integrated presentation/input/save path after the corpus
and live checks. This is not a pixel-for-pixel original-console capture claim
or a claim that the deferred PostFX appearance problem is solved.

Further overbright-sky/wet-looking-clothing investigation is deferred.
The preceding confirmed PostFX clock correction is retained: repeated
redraws must not reconsume one presentation frame's exposure/LUT blend dt.

## Video Settings and VSync — October 6, 2026

The Frame Rate Cap (October 8, below) replaced VSync.

The page takes the original's name: the row and the page title use
`ID_GAMESETTINGS_VIDEO_SETTINGS` (`gGameOptionsItems` 0x82fdf9a8, `gGameOptionsScreensTitles`
0x82fdf97c), the native Video Settings destination this page stands in for. Its rows stay PC
options. A third selector, VSync, steps Off/Half/On (wrapping, `core_nav_left|right`), shown as
`ID_COMMON_OFF`, "Half" and `ID_COMMON_ON`.

- Off presents uncapped: Immediate, else Mailbox, else Fifo. On is Fifo. Mailbox is not
  uncapped after a reconfigure: on the NVIDIA Vulkan driver a Fifo surface
  switched to Mailbox stayed at 165 FPS on a 165 Hz screen, and even a new surface ran at two
  frames per refresh (330). Immediate measured 539-638 in free skate, against 464 for the old
  launch-time Mailbox.
- Half is Fifo with each frame presented twice. The finished frame is copied to a retained
  texture before its present, and a second surface image receives the copy, so every frame shows
  for two refreshes without being drawn twice. It needs COPY_SRC and COPY_DST surface usage;
  without them Half behaves as On. The FPS counter counts drawn frames.
- The renderer reconfigures the surface on the next frame when the mode changes, so a choice
  applies at once; the simulation keeps its fixed 1/60 step in every mode.
- `SKATE2_NO_VSYNC` still forces Off (now Immediate), for measuring renderer headroom.
- Verified in free skate from the pause menu: Off 638 FPS, Half 82, On 165, and Off back to
  On or Half re-caps at once.
- Save: `SK82_PC::DisplaySettings` version 2 appends a one-byte tuple (0 Off, 1 Half, 2 On).
  Version 1 still reads, with VSync On; writes are version 2.

## Frame Rate Cap — October 8, 2026

The VSync row became Frame Rate Cap (a PC change). Its values are Unlimited,
"60 (Vanilla)", 120 and Refresh Rate, stepped in that order and wrapping
(`core_nav_left|right`). The values are literal text.

### The original's rate

`RenderMan::Render` 0x82858360 sets the present interval before every frame:

- Interval 1, `m_iFPS` 60: the 60 Hz display shows every frame.
- Interval 2, `m_iFPS` 30: when `PostFX::m_b30HzMode` is set.

`PostFX::Apply` 0x8284c7a0 clears `m_b30HzMode` each frame. It sets it again when any of these
hold:

- The presentation's ICE postfx is active (`mPostfxPresentation.mICEPostfxActive`).
- A replay effect is running (`mReplayFXState` >= 0).
- The replay editor is on (`ICELITEManager::IsEditorOn`).
- A screenshot is pending (`mbScreenShotRequired`).
- `kbForceNISMode` is on.
- `m_bLock30FPS` is set. Challenges send it with `cMsgAdjustFPS`, and
  `Presentation::ReceiveSetAdjustedFPS` 0x827eefc0 stores it.
- The effects include `eFE` (0x10): the front end.

Create-A-Skater keeps 60: the mode is only set while `CASPresManager::mState` is 0.

Free skating therefore runs at 60, and the front end, cutscenes and replays at 30. The port's
60 cap does not reproduce the 30 Hz mode.

### The port

- **Present mode.** Refresh Rate is Fifo, as VSync On was. Unlimited, 60 and 120 present
  at once: Immediate, else Mailbox, else Fifo. This is the mode VSync Off used.
- **The 60 and 120 wait.** `GpuRenderer::wait_for_frame_cap` runs at the start of every
  redraw, before the client reads its clock. A frame starts one interval (1/60 or 1/120 s)
  after the last presented frame started, or at once if it is already later. The wait sleeps
  to within 1 ms of the target, then spins.
  - Because the wait comes before the clock read, the moments the presentation restore shows
    are as evenly spaced as the frames.
  - A redraw that presents nothing does not move the schedule.
- **Half removed.** The repeat texture and the surface's COPY_DST usage are gone.
- **`SKATE2_NO_VSYNC`** forces Unlimited, for measuring renderer headroom.
- **Save.** `SK82_PC::DisplaySettings` version 3 stores a one-byte cap: 0 Unlimited, 1 60,
  2 120, 3 Refresh Rate.
  - Version 2 still reads: VSync Off, Half and On become Unlimited, 60 and Refresh Rate.
  - Version 1 reads as Refresh Rate.
  - Writes are version 3. The default stays Refresh Rate.
- **Scripted demo.** The `display-settings` controls demo now steps the new row to 60 before
  its main-menu save. It checks the saved value at tick 900 and again at the end.

### Verified, October 8

Cruise route: `perf-skate` from `Q_SmokeTest_1300_N900`, 2400 ticks, 1280×720, 4× MSAA
on a 165 Hz display, with a scratch SKATER.P for each cap. Frame times come from the restore trace,
which skips the first 120 frames.

| Cap | Frames/s | Frame time mean / sd / max (ms) |
|---|---|---|
| 60 (Vanilla), runs 2 and 3 | 60.0 | 16.667 / 0.004–0.006 / 16.78 |
| 120 | 120.0 | 8.333 / 0.002 / 8.40 |
| Refresh Rate | 165.0 | 6.06 / 0.40 / 12.0 |
| Unlimited | 610 | 1.64 / 0.43 / 7.6 |

- **60 (Vanilla), first run:** one 160 ms swapchain acquire at 18.8 s. The two reruns and the
  other caps had none.
- **Unlimited against the previous build:** three alternating pairs gave 583–612 FPS against
  557–580.
- **`parity.sh`:** IDENTICAL (6000 lines), uncapped and with the 60 cap.
- **Tests:**
  - `skate2-pc` release: 128 pass.
  - The ignored Video Settings layout test passes. It checks every cap value's glyphs in the
    selector font.
  - The options and profile display tests pass.
- **`display-settings` demo, wall variant:** shows Frame Rate Cap with "60 (Vanilla)" between the
  arrows.
- **`display-settings` demo, full run:** saves and reads back the 60 cap at tick 900. Its in-game
  half fails as it does on the previous build.
- **A version 2 save** (VSync Off) reads as Unlimited.
