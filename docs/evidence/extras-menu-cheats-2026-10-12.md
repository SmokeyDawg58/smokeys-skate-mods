# PC Extras menu and cheat codes — 2026-10-12

## Follow-up: Intro Video InitAction and main-menu transition

The next run reached Extras, but Intro Video stopped at
`untranslated APT InitAction 87:1 0x260`. That run also showed Extras text
appearing over Main Menu during the transition.

### Movie class initialization

The original archive identifies this as the movieplayer class registration,
not a video codec error. The host already implements its constructor,
OnLoaded acknowledgement and intro/outro actions, but the separate
`FrameEffect::InitAction` dispatcher did not recognize the registration.
The same omission affected all five translated tutorial classes.

| Asset index / original movie | InitAction character | Script offset |
| --- | --- | --- |
| 87 / movieplayer | 1 | 0260 |
| 88 / tutbasreplay | 73 | fc08 |
| 89 / tutmap | 59 | 5e0c |
| 90 / tutcreatespot | 150 | af24 |
| 91 / tutadvreplay | 149 | 113f0 |
| 92 / tuthom | 48 | 64c0 |

These exact registrations now dispatch to the existing translated owners.
Other unknown InitActions still report an error. The asset fingerprints are
the six recorded in the Movies section below. Live IDA MoviePlayer::OnEnter
8265bb08 confirms screen 33 and optional tutorial HUDs 138..142; the existing
screen-ready generation acknowledgement and movie-start conditions remain.

### Screen lifecycle and ordering

- OnMenuSelectGame 82681338, Extras branch 82681624: normal PushState(24).
  It does not run the misleadingly named MainMenuToExtras NIS used by the
  online menu branch. PushState 82d69cf0 blocks input immediately and queues
  the state change with both keep-previous flags false.
- ExecutePushState 82d694e8 calls UpdateOnLeave 82d66300 for Main Menu,
  then enters Extras. MainMenu::OnLeave 8265a0b8 and Extras::OnLeave
  826531b0 request CloseScreen(false), which 82d66158 marks Closing.
- MainMenu::OnEnter 82659fd8 requests screen 8; Extras::OnEnter 82653150
  requests screen 70. StateManagerFrontEndBase::Update 82d68370 services
  RequestLoad after the APT update, through UpdateFEScreen 82657248 and
  ScreenManagerBase::OpenScreen 82d6a840.
- Raw gFEStateInfo 82fded80, 20-byte rows 22 and 24, has mbTransition=false
  at offset 2 for both. UpdateOnLeave therefore calls CloseFEScreen
  826572c0 directly. The unrelated multi-transition removal gate
  UpdateScreenTransition 82d660b0 is not used for this pair.
- Screen templates at 82fdb358 (16-byte rows) specify Main Menu as standard
  type 0 and Extras as TransitionAtEnd type 2. A standard main-menu screen
  does not become ScreenTransitionManager's current transition screen.
  The initial Extras transition uses InitialScreenReady, which shows the
  linked screen and plays intro. This is not an extra wait-for-outro rule.
- Original screenmanager.apt f8bd33f4: ReadyScreen assigns increasing display
  depths and queues loading; ProcessOpenScreens at 1048 counts down the
  eight-tick mBlockScreenOpen, then creates one screen. With no pending
  opens, it removes onEnterFrame and leaves that counter frozen. Destroy
  requests re-arm it; ProcessDestroyedScreens at fc0 removes one screen
  before any open processing that tick.
- Main Menu and Extras now use that existing shared queue, including its
  five-step asynchronous-load model. PreloadTextures 82610730 does not
  preload either screen. Both original screens have apt/const/geo/rx2
  members; CheckLoadingOperation 82a720d8 and
  CheckAndCompleteAsyncLoad 82a728b0 describe the staged load. Actual disk
  completion can extend the native timing beyond this cached-asset model.
- Their constructors, picker setup, shared letterbox title/buttons and intro
  now execute when the screen links, instead of at the input change. Authored
  timelines are retained: Main Menu outro frames 14..22, Extras 34..42
  (zero-based asset frame indices).
- screenmanager OutroComplete d38 calls the native callback and transition
  manager, then queues DestroyScreen. The host now removes these roots on
  the following APT tick instead of hiding them immediately in the callback.
  Close cancels an unlinked request; leaving the front end cancels its pending
  Main Menu/Extras loads. Input readiness still comes from IntroComplete.
- Main Menu and Extras draw by their assigned screen depths, including a
  newly created Main Menu above the departing Extras on Back. Options keeps
  its existing keep-previous hide/show behavior and shared letterbox owner.

This corrects the immediate-open bypass. No invented fade or fixed additional
delay was added. In particular, the original queue can open an incoming screen
concurrently with an outro after its counter has drained; that behavior is
retained rather than declaring that every possible overlap is forbidden.

Source and executable backups, the scoped diff, decoded original scripts,
InitAction inventory, 25 live IDA/table records and build outputs are under
`.scratch/extras-transitions-fmv-20261012/`. No original binary assets were
copied. Seven PC presentation/fixture files changed; engine logic, save files
and Android are untouched. Two existing fixtures were adapted to the deferred
link interface; no tests were added or run. Confidence: **exact** for the
registration tuples, table values and authored queue/timeline instructions;
**high** for the integration. Visual timing, video playback and return paths
still require an in-game check; no frame-exact runtime claim is made.

### Release build

Normal `cargo build --release -p skate2-pc -j 1 --timings` passed, exit 0,
in 219.536 seconds, finishing 2026-10-12T17:37:16Z. The updated executable
is 42,835,968 bytes, SHA256
`EABAC3A328D9BBEA7B621F3F7CDA126C594772706D87A2C723201A01FACB2D83`.
Existing warnings remain (core 435, PC 18); scoped source whitespace review
reported no errors. No tests or game launch were run. This executable
supersedes both release checksums recorded below. The standard launcher is
`play-quiet.bat`.

## Follow-up: missing dummy0 on entering Extras

The first release build failed immediately on entering
Extras with `missing authored marker child dummy0`. This was a host menu
integration mistake, not an absent asset:

- Live IDA `FrontEndState_Extras::OnEnter` 82653150 confirms screen 70.
- Original `extras.apt` FNV32 7031d76f places import character 1 as `mMenu`.
  Its import names `source/controls/menu_picker`, `Menu_Picker_Component`.
- Original `menu_picker.apt` FNV32 f52d4e28 exports that component as
  character 55. Its only first-frame child is character 54, `mMenuPicker`.
  Character 54 owns `dummy0` through `dummy7`, `mItemGroup` and `mIndicator`.
- The authored picker constructor at 294c..2950 reads
  `mMenuPicker.dummy0._height`, confirming the intended nesting.
- `ExtrasMovie::update` incorrectly gave `map_movie::update_picker` the outer
  `mMenu`. It now passes `mMenu.mMenuPicker`, matching the original nesting
  and the existing main-menu integration. The helper's other callers already
  supply their inner picker. The shared row layout and behavior are retained.

The single-source-file change was rebuilt with the normal release command:
`cargo build --release -p skate2-pc -j 1 --timings`. Exit 0 in 142.444 seconds,
finished 2026-10-12 17:15:43 UTC. The updated executable is 42,698,240 bytes,
SHA256 `F9254162C2C47B280A6C92A2DFC50777304ABA3CEE0D91FC030372F6201C6933`.

`.scratch/extras-picker-fix-20261012/` contains the live IDA result, freshly
decoded original scripts and import/export/placement hierarchy, source and
previous executable backups, scoped diff, documentation backups, build log
and manifest. No original binary assets were copied into the repository.
No tests or gameplay were run; the in-game check remains pending.
Confidence is **exact** for the asset hierarchy and **high** for the diagnosed
failure path. This supersedes the executable checksum in the initial build
record below. Android and the real save were untouched.

## Requested release build completed

The subsequent normal PC release build passed on 2026-10-12:

- Command: `cargo build --release -p skate2-pc -j 1 --timings`.
- Finished at 17:09:18 UTC; 838.329 seconds wall time; exit code 0.
- Executable: `target/release/skate2-pc.exe`, 42,698,240 bytes.
- SHA256: `F2860AE0C818CBCE6B4FB564376ED296BB0657223CC9889A11D386E8F2911BCD`.
- Existing warnings: 435 from the engine, 18 from the PC application.
- No source or release-profile changes were needed. This executable includes
  Extras/cheat codes and the preceding PC edits. Android was untouched.
- Log, JSON manifest, prior executable and documentation backups are in
  `.scratch/extras-menu-20261012/pc-release-build/`.

This supersedes the initial unbuilt status recorded below. No tests, game
launch, visual comparison or actual-save round-trip were run during this build.
`play-quiet.bat` runs it for the hands-on check.

## Scope and result

Scope: the main-menu Extras flow, its original menu/content, a PC
in-game keyboard field, all original typed cheat codes, and persistent
character unlocks in `SKATER.P`. Implemented in this repository only.
Android is parked. The proposed retail 1.02 offboard comparison remains parked.

The implementation includes the Extras picker, code entry/success/save-error
popups, the five native movie selections with their unlock condition, the
credits roll and background movie, and the four tutorial HUD overlays used by
Extras (plus the map overlay in the shared movie player).

All three entries with actual code strings in the debug executable's table
are connected:

| Exact code | Original action | Persistence |
| --- | --- | --- |
| `letsdowork` | Unlock category 4, `big_black`, fromDLC=false | Saved in `UNLOCKABLES` within `SKATER.P`; consumed by the existing Free Skate character filter |
| `strangeloops` | Toggle stereoscopy | Runtime toggle, as in the reference; two geometry views and native red/cyan postfx composition |
| `streetsweeper` | Reset all dynamic objects | Cull existing objects, clear persistence, recreate streamed locators; save empty version-3 `DYNAMICOBJECTS` data with the retained clock |

Typing replaces the console keyboard. Enter submits; Escape cancels;
Backspace/Delete, arrows, Home/End edit the field. Original alphanumeric input
and the callback's 32-byte limit remain. Code comparison is case-sensitive.
The PC invalid-code exception means no error popup, network
promotion request or cheat action for unknown input. The field closes.

## Reference and evidence

Reference is the live symbol-rich PPC database for
`sk82_na_m.xex`.
Original APT/CONST/GEO/RX2, Vault and credits inputs are read from the external
Xbox game folder `<game folder>`.
Original assets and executables were not copied into the repository.

Scratch packet: `.scratch/extras-menu-20261012/`:

- `before/`: original versions of edited files, preserving earlier work.
- `native.json`, `stereo-native.json`, `reset-native.json`,
  `additional-reference.json`, `fmv-reference.json`: native functions,
  raw data/instructions and decoded shader evidence.
- `extras-apt.txt`, `credits-apt.txt`, `sk8popup-apt.txt`,
  `movieplayer-apt.txt`, `tut*-apt.txt`: decoded authored UI scripts/timelines.
- `tutorial-asset-inventory.txt`: asset fingerprints, fonts, frame counts,
  text, geometry and texture metadata. No tutorial places a standalone
  Bitmap character; the renderer's unimplemented standalone-bitmap branch
  is not reached by these timelines.
- `scoped-final.diff`: source changes against the task's starting snapshots,
  including new files.
- `cargo-check-audio.log`: final successful compile check;
  `cargo-check-complete.log`: preceding app-only check;
  `cargo-check-fmv.log`: preceding successful check after the FMV integration.
- `wgsl-validation.log`: Naga parse/validation of changed shaders.

`credits-layout-sample.txt` is an earlier sample of the obsolete
`data/fe/credits/credits.txt`; it is **not** the source used by this implementation.
The actual source is `data/credits/credits.txt` (36,010 bytes, 1,097 entries).
Early `native.json` requests at 82846900/8284c6b0 landed in adjacent functions;
the correctly addressed stereo functions are in the later packets.

## Extras and input

- `FrontEndState_MainMenu::OnMenuSelectGame` 82681338, row-6 branch 82681618:
  PushState(24), with no new NIS/camera transition.
- Extras OnEnter 82653150, OnLeave 826531b0, ReceiveInput 8265a480,
  OnMenuSelect 82669830, GetNumOptions 82643090.
- Screen template 70, `main/extras.swf`, fingerprint 7031d76f; original
  `menu_picker` and LIGHT_ON_DARK palette, 3D wall placement, letterbox title,
  button labels, wrapping selection, intro/outro and sound callbacks.
- `gExtraOptionsItems` 82fdf910: code, credits, videos 1,2,4,5,3. Six options
  normally, seven with category 11 `filmpackmovie` including DLC unlocks.
- `gScreenTemplate` 82fdb358 uses 16-byte records. Extras is transition 2,
  screen type 1; credits and movie HUDs use flat screen type 0.
- `gFEStateInfo` 82fded80 uses 20-byte records: FE24 does not pause, uses
  chyron and no dimmer; FE28/29 pause and hide the chyron; FE75 enables dimmer.
  These states now reach the shared audio/Chyron controller while covering
  the main menu, including the native chyron outro on entering video/keyboard.
  Raw rows are saved in `frontend-state-rows.txt`.
- PC text uses the existing original `sk8popup` presentation. It does not
  enter skating input history, and Escape cannot simultaneously quit the app.
  Result/error dialogs also accept Enter/Escape. Reopening a popup during the
  previous popup's outro gives the new screen a fresh intro.

## Cheat execution and saves

- `kCheatCodes` 8208b438: five 16-byte rows, two with null code pointers
  (invalid and network promotion), then `letsdowork`, `strangeloops`,
  `streetsweeper`, with action IDs 1,4,5.
- ReceiveFEExtrasEnterCode 82681650 uses `StringCompare`.
  KeyboardAcceptCallback 8265a4d8 truncates input to 32 bytes.
- ExecuteCheat 8265a700, SaveProfile 82643168; success descriptions
  `ID_VALID_CODE_CHEAT_BIGBLACK`, `_STEREOSCOPY`, `_STREETSWEEPER`.
- Other switch cases (promotion response, all pros, all spots) have no typed
  code in this table; no invented codes were added. Unknown 15-character
  promotion input no longer contacts a server (a PC rule).
- Existing `EditSkaterProfileStore::save_memcard` merges other savables,
  encodes and checks read-back before committing, and retains its existing
  backup/replacement behavior. New profiles keep their `new_profile` state.
  Current audio settings are saved through the same existing profile path.
- The retained career unlock backend also receives Big Black after successful
  save, preventing a later career save from replacing the unlock with stale
  state. Free Skate's existing `FECharacterManager::IsCharacterUnlocked`
  reconstruction (82606a10) reads the same category/name.
- Save failures appear in a dismissible dialog; they do not fail the controlled
  frame loop. As in native execution-before-save, an already applied runtime
  action can remain applied if a subsequent save fails.
- No real save was edited or migrated during implementation.

## Dynamic-object reset

`DynamicObjectsBackEnd::ResetAllDynamicObjects` 825697e0 calls
RequestAllCull(false) 82d369c0, ClearPersistence 82d32268,
RemoveAllLocators 82d3c250, then RecreateStreamedLocators 82d3c2f0.

The session uses the existing cull/recipe/world lifecycle, then clears both
persistence owners and recreates each loaded chunk in the retained chunk
order, with original flags 0/true/true/false. No substituted physics reset or
changes to integration steps, contacts, forces, bail thresholds or animation.

Locator deletion follows the original map order: constructor 82d3ac40,
fixed-map constructor 82d35110, `gPrimeNumberArray` 822852f0 confirm
GetPrevBucketCountOnly(1025)=953 buckets. `DoInsertValue` 824da510 hashes the
wrapping sum of GUID halves and inserts at the bucket head. The host retains
this order alongside its keyed lookup; only new entries join it and erased
entries leave it.

Save 8256ea70 / Load 8256c538 confirm version 3, count, 112-byte records and
clock. BackEndManager::Init 825813c0 registers `DYNAMICOBJECTS`, with leaf
`DynamicObjectsBackEnd::Data`. An empty reset saves count 0 and the retained
clock, not a fabricated new clock.

## Credits and movies

Native movie keys were checked in raw PPC instructions to retain both halves;
the decompiler sometimes omits their high words.

| Extras row | Key | Shared movie HUD |
| --- | --- | --- |
| Video 1 | 44354e4fd31dae34 | None |
| Video 2 | 4ff5292a7dfb6081 | Basic Replay, 138 |
| Video 4 | 1970dd9e472a7126 | Create a Spot, 140 |
| Video 5 | 34049abce23c10ac | Hall of Meat, 142 |
| Video 3, Film Pack | fce8a97af24df012 | Advanced Replay, 141 |
| Credits | 0bdb6ab1cb803a1b | Credits screen 34 |

Credits FE29: OnEnter 8264a550, intro 826697f0, OnUpdate 8265a318,
ReceiveInput 8265a420, OnLeave 8265a240, outro 8265a3b8. The movie is drawn
under the roll with a forced black background, repeats until the roll ends,
and B follows the original CanBackOut flag. Audio uses the existing movie
pause/resume path. The Extras picker returns after leaving the movie state.

Credits APT fingerprint 1402d4b0: 30 text slots, x=960/y=750, 1.5 pixels per
main APT tick, reuse below y=-20, exact authored font sizes/line spacing and
intro/outro callbacks. ParseCreditEntries 82d6ade0 / ExtractCreditEntry
82d6ac28 and GetLine 826230e0 were checked. Original credits use Latin-1 bytes;
font index 2 resolves to Helvetica LT Std. Text is centered, unwrapped and
uses the original color 12046808. The existing PC Screen_EdgeOffset=0 policy
and display projection apply.

FE28: OnEnter 8265bb08, FMVHUDScreenReady 826438b8, OnUpdate 8266a880,
PlayMovie 8265bf98 and OnLeave 8265bd78. Playback now waits for actual host
construction of movieplayer and its HUD, acknowledging OnLoaded, not waiting
for IntroComplete. A generation protects the acknowledgement against a
replacement screen. `SetState(1)` starts the tutorial animation.

| Asset | FNV32 | Initial one-based frame | Initial delay |
| --- | --- | --- | --- |
| movieplayer | f5e495b6 | intro label | — |
| tutbasreplay | 0d51f2a9 | 2 | 0 |
| tutmap | 87ffc45e | 10 | 0 |
| tutcreatespot | 40f669fd | 2 | 14 |
| tutadvreplay | 8e92ecf0 | 2 | 6 |
| tuthom | 15bda746 | 10 | 0 |

All five tutorial classes use kFRAMEDELAY=1 (every other main APT tick),
85% scale at x=98/y=0 in the non-safe-area branch. Their original timeline
placements, text, fonts and textures are retained. Create a Spot localizes
and auto-sizes OVER/RAIL with the authored five-pixel gap; HoM formats the
authored 1500 currency value. `jumpToFrame` 82a1b410 ignores an overrun target;
the overlay retains its last display until movie leave destroys the HUD.
Generic movieplayer plays its empty intro/outro callbacks. Subtitle drawing
is now also available over main-menu movies.

## Stereoscopy

- Begin 828223d8, field completion 828223a8, IsActive 82827988,
  Render 82858418 and SetPinch 82838e30.
- Native PINCH 82feffd0 is 1 degree; EYE_DIST 82feffd4 is bits 3d23d6ef.
  Projection 82840770 applies opposite eye yaw and horizontal displacement.
- RenderModeScene 82857b40 and PostFX::Apply 8284c7a0 suppress stereo in the
  front-end mode. Host main-menu scenes remain mono. The cheat takes effect
  in the playable world with PostFX enabled, through the normal postfx owner.
- Actual scene geometry is rendered for both eyes, including objects,
  pedestrians, map, dynamic objects, world UI and splines. The original camera
  near/far values are supplied by the view. The retained central projections
  are restored when disabling stereo. Screen HUD stays screen-aligned.
- ApplyVisual 82846728, ApplyFastUber 828469b8 and decoded
  `stereo_uberPostFX_PS` 8212e1a8: left VisualFX red plus right near-grade/cube
  green/blue and alpha. Original 1.41412354 vs 1.41 gains are retained in their
  respective passes; no invented channel matrix.
- GPU submissions separate the fields so later writes cannot replace the
  earlier field's uniforms. Presentation, FPS, simulation and temporal
  postfx state advance once per presented frame.

## Validation and remaining limits

- Final `cargo check -p skate2-pc --bin skate2-pc -j1 --offline` **PASS**;
  `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`. Final check 53.06s;
  preceding app recheck 7.51s, full check after FMV changes 48.09s.
  Existing warnings remain (core 435, PC 18).
  An initial ordinary check hit memory pressure; subsequent low-memory
  metadata checks succeeded. No release optimization settings were changed.
- Naga parse and validation **PASS** for postfx WGSL in standard and MSAA
  variants, marker UI WGSL and visual indicator WGSL.
- Scoped source/caller review, original-data decoding, raw constant/table
  review and lifecycle ordering review completed; scoped `git diff --check`
  passed. 32 source/fixture files (26 existing, 6 new). Only the existing PostFX
  fixture's call signature was adapted; no tests were added or executed.
- No release build or gameplay/visual/save round-trip run. The running
  executable and actual save have not been modified. Rebuild is required.
- Confidence: **high** for recovered menu branches, code strings/actions,
  save records, original content and lifecycle wiring. Constants/catalog
  entries are directly evidenced. Runtime visual parity, video/overlay
  synchronization and save behavior still need in-game verification.
- Stereo's host matrix math/trigonometry and GPU conversion are not claimed
  bit-identical to PPC/VMX. Stereoscopy currently uses the enabled PostFX path;
  the PC-only PostFX-off option suppresses its composition. The native shader
  itself, channels and constants are evidenced, not a claim of screenshot parity.
- Earlier intentional PC changes (all gestures, HoM full speed, removed
  lens reflections, PostFX-off brightness) are preserved. Android unchanged.
