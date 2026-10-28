# NIS FE overlays (FrontEndState_NIS) and the ICE freeze channel - evidence 2026-10-04

Source: debug XEX in IDA (read-only), FE movies from `data\fe\screens.big` dumped with
`tools/inspect_apt.py` (text fields read with the layout of `crates/skate2-formats/src/apt/movie.rs`
`read_character` type 2), ICE data `data/scene/ice_000.bin` + `scene_000.bin` (research parser).
Builds on `nis-playback-flow-2026-10-04.md` (load states, messages). All addresses are native.

## 0. Corrections to earlier notes

- Freeze command 5 is `PhotoShot_FixedFrame` (label table `gICEFreezeCmds` 0x82fe2b28), not "Killed It":
  it sends `cMsgNISFEEvent{85 = Take_FixedPhoto}` (0x826d1fa0) and the bloom pulse is a photo flash.
  Command 4 (`PhotoShot`) sends `{86 = Take_NonFixedPhoto}` (0x826d1dfc).
- Hex-Rays prints wrong enum values for ctor rows 38/53/57 and the sponsor id of row 61 (the ctor spills its
  constant registers with 64-bit std/ld, e.g. 0x82673fdc/0x82674194 and 0x82674040/0x826743dc). The values below
  come from emulating every store of the ctor 0x82673d30 from its raw words: row N always has `mNISState = N`.

## 1. eSk8HUDNISState and mNISStateTable

- Names: `gICEOverlayNames` 0x82fe2800 (87 string pointers, index = value; `ICEUserChannelOverlays::GetLabel`
  0x826c7740 returns NULL for value >= 87). Strings at 0x820b0024..0x820b0738 (prefix `eSk8HUDNISState_`).
- `eSk8HUDNISState_Max` = 87 (`cmpwi r31,0x57` 0x8268955c), `eSk8HUDNISState_Unknown` = 88 (ctor 0x82673d70,
  `ICEUserChannelOverlays::OnTakeStart` 0x826c7730 stores 88 in `mCurrentOverlayState`).
- `FrontEndState_NIS` (size 0x760): +0x38 mNISState, +0x3C mReadyToSetScreenState, +0x40 mSubtitleId,
  +0x44 `sNISStateTable mNISStateTable[87]` (20 bytes: +0 mNISState, +4 u8 mIsSponsor, +8 mSponsorId,
  +0xC m360Screen, +0x10 mPS3Screen), +0x710 mScreenMap (hash_map eSk8FEScreenState -> screen id),
  +0x730 vector mLoadedScreenIds, +0x740 vector mSponsorsToLoad, +0x750 mLastScreenId, +0x754 u8 mbSponsorReady,
  +0x758 mNumBiosShown, +0x75C u8 mICEOpened. `20*(state+4)` = 0x50 + 20*state = `&mNISStateTable[state].m360Screen`.
- Rows are filled only by the ctor 0x82673d30 (stores 0x82673dfc..0x82674708). mPS3Screen == m360Screen in every row.
- Enter/leave columns: `OnEnterState` 0x826675a0 jump table 0x826676f8 (index state-2, 85 entries);
  `OnLeaveState` 0x82667cc8 jump table 0x82667db4 (index state, 85 entries). "bios" = `++mNumBiosShown`
  (0x826678ac/0x826678cc). Values are the second argument of `FrontEndManager::SetScreenState(id, n)` 0x826063c0.

| # | eSk8HUDNISState_ | sponsor (flag,id) | m360Screen = mPS3Screen | OnEnterState -> SetScreenState | OnLeaveState -> SetScreenState |
|---|---|---|---|---|---|
| 0 | Clear | 0 | -1 | - | ForceGestureOllieUnload (see 3.4) |
| 1 | Test1 | 0 | -1 | - | - |
| 2 | SpotFound_BottomLeft | 0 | 48 SpotFound | 4 | 5 |
| 3 | SpotFound_BottomRight | 0 | 48 SpotFound | 0 | 1 |
| 4 | SpotFound_TopLeft | 0 | 48 SpotFound | 6 | 7 |
| 5 | SpotFound_TopRight | 0 | 48 SpotFound | 2 | 3 |
| 6 | SpotOwned_BottomLeft | 0 | 49 SpotOwned | 4 | 5 |
| 7 | SpotOwned_BottomRight | 0 | 49 SpotOwned | 0 | 1 |
| 8 | SpotOwned_TopLeft | 0 | 49 SpotOwned | 6 | 7 |
| 9 | SpotOwned_TopRight | 0 | 49 SpotOwned | 2 | 3 |
| 10 | ProBioInfoOverlay_Left | 0 | 38 Pro_Bio_Info | 0 (+bios) | 1 |
| 11 | ProBioInfoOverlay_Right | 0 | 38 Pro_Bio_Info | 2 (+bios) | 3 |
| 12 | ProChallengeIntro_Left | 0 | 39 Pro_Challenge | 0 | 1 |
| 13 | ProChallengeIntro_Right | 0 | 39 Pro_Challenge | 2 | 3 |
| 14 | SkateAcceptDeclineOverlay | 0 | -1 | 0 (unreachable, no screen) | - |
| 15 | Imagesize_DoublePage_Large | 0 | 54 Imagesize_DoublePage_Large | 1 | - |
| 16 | Imagesize_FullPage_Tall | 0 | 55 Imagesize_FullPage_Tall | 1 | - |
| 17 | Imagesize_HalfPage_Med | 0 | 56 Imagesize_HalfPage_Med | 1 | - |
| 18 | Imagesize_QuarterPage_Small | 0 | 57 Imagesize_QuarterPage_Small | 1 | - |
| 19 | Imagesize_SBMCover | 0 | 58 Imagesize_SBMCover | 1 | - |
| 20 | Imagesize_ThrasherCover | 0 | 59 Imagesize_ThrasherCover | 1 | - |
| 21 | SponsorLogo_alien | 1,1 | 60 SponsorLogo_Setup | 1 | 50 |
| 22 | SponsorLogo_almost | 1,2 | 60 SponsorLogo_Setup | 2 | 50 |
| 23 | SponsorLogo_autobahn | 1,3 | 60 SponsorLogo_Setup | 3 | 50 |
| 24 | SponsorLogo_baker | 1,4 | 60 SponsorLogo_Setup | 4 | 50 |
| 25 | SponsorLogo_circa | 1,5 | 60 SponsorLogo_Setup | 5 | 50 |
| 26 | SponsorLogo_dcshoes | 1,6 | 60 SponsorLogo_Setup | 6 | 50 |
| 27 | SponsorLogo_destrocuto | 1,7 | 60 SponsorLogo_Setup | 7 | 50 |
| 28 | SponsorLogo_emerica | 1,8 | 60 SponsorLogo_Setup | 8 | 50 |
| 29 | SponsorLogo_enjoi | 1,9 | 60 SponsorLogo_Setup | 9 | 50 |
| 30 | SponsorLogo_es | 1,10 | 60 SponsorLogo_Setup | 10 | 50 |
| 31 | SponsorLogo_etnies | 1,11 | 60 SponsorLogo_Setup | 11 | 50 |
| 32 | SponsorLogo_fallen | 1,12 | 60 SponsorLogo_Setup | 12 | 50 |
| 33 | SponsorLogo_flip | 1,13 | 60 SponsorLogo_Setup | 13 | 50 |
| 34 | SponsorLogo_girl | 1,14 | 60 SponsorLogo_Setup | 14 | 50 |
| 35 | SponsorLogo_globe | 1,15 | 60 SponsorLogo_Setup | 15 | 50 |
| 36 | SponsorLogo_gold | 1,16 | 60 SponsorLogo_Setup | 16 | 50 |
| 37 | SponsorLogo_hubba | 1,17 | 60 SponsorLogo_Setup | 17 | 50 |
| 38 | SponsorLogo_independent | 1,18 | 60 SponsorLogo_Setup | 18 | 50 |
| 39 | SponsorLogo_ipath | 1,19 | 60 SponsorLogo_Setup | 19 | 50 |
| 40 | SponsorLogo_lakai | 1,20 | 60 SponsorLogo_Setup | 20 | 50 |
| 41 | SponsorLogo_momentum | 1,21 | 60 SponsorLogo_Setup | 21 | 50 |
| 42 | SponsorLogo_mystery | 1,22 | 60 SponsorLogo_Setup | 22 | 50 |
| 43 | SponsorLogo_nike | 1,23 | 60 SponsorLogo_Setup | 23 | 50 |
| 44 | SponsorLogo_osiris | 1,24 | 60 SponsorLogo_Setup | 24 | 50 |
| 45 | SponsorLogo_planb | 1,25 | 60 SponsorLogo_Setup | 25 | 50 |
| 46 | SponsorLogo_real | 1,26 | 60 SponsorLogo_Setup | 26 | 50 |
| 47 | SponsorLogo_royal | 1,27 | 60 SponsorLogo_Setup | 27 | 50 |
| 48 | SponsorLogo_silver | 1,28 | 60 SponsorLogo_Setup | 28 | 50 |
| 49 | SponsorLogo_spitfire | 1,29 | 60 SponsorLogo_Setup | 29 | 50 |
| 50 | SponsorLogo_thunder | 1,30 | 60 SponsorLogo_Setup | 30 | 50 |
| 51 | SponsorLogo_zero | 1,31 | 60 SponsorLogo_Setup | 31 | 50 |
| 52 | XGames_MegaRamp_Intro | 0 | 60 SponsorLogo_Setup | 1 | - |
| 53 | XGames_StreetSkating_Intro | 0 | 60 SponsorLogo_Setup | 1 | - |
| 54 | Take_Photo | 0 | 61 PhotoTreatment | 1 | - |
| 55 | SponsorLogo_bakerwheels | 1,32 | 60 SponsorLogo_Setup | 32 | 50 |
| 56 | SponsorLogo_flipwheels | 1,33 | 60 SponsorLogo_Setup | 33 | 50 |
| 57 | SponsorLogo_mysterywheels | 1,34 | 60 SponsorLogo_Setup | 34 | 50 |
| 58 | SponsorLogo_planbwheels | 1,35 | 60 SponsorLogo_Setup | 35 | 50 |
| 59 | SponsorLogo_zerowheels | 1,36 | 60 SponsorLogo_Setup | 36 | 50 |
| 60 | SponsorLogo_venture | 1,37 | 60 SponsorLogo_Setup | 37 | 50 |
| 61 | SponsorLogo_alienwheels | 1,38 | 60 SponsorLogo_Setup | 38 | 50 |
| 62 | SponsorLogo_icecream | 1,39 | 60 SponsorLogo_Setup | 39 | 50 |
| 63 | SponsorLogo_fillmore_wheels | 1,40 | 60 SponsorLogo_Setup | 40 | 50 |
| 64 | SponsorLogo_blacklabel | 1,41 | 60 SponsorLogo_Setup | 41 | 50 |
| 65 | SponsorLogo_chocolate | 1,42 | 60 SponsorLogo_Setup | 42 | 50 |
| 66 | SponsorLogo_krooked | 1,43 | 60 SponsorLogo_Setup | 43 | 50 |
| 67 | SponsorLogo_supra | 1,44 | 60 SponsorLogo_Setup | 44 | 50 |
| 68 | NisOverlay_Left | 0 | 67 NIS_Overlay | 0 | 2 |
| 69 | NisOverlay_Right | 0 | 67 NIS_Overlay | 1 | 3 |
| 70 | KilledIt | 0 | 50 SpotKilled | 1 (on mKilledItScreenId) | - |
| 71 | Highlight_FilmOverlay | 0 | 80 HighlightReelHUD | 0 | - |
| 72 | KilledIt_TopRight | 0 | 50 SpotKilled | 2 (on mKilledItScreenId) | - |
| 73 | KilledIt_BottomLeft | 0 | 50 SpotKilled | 3 (on mKilledItScreenId) | - |
| 74 | KilledIt_BottomRight | 0 | 50 SpotKilled | 4 (on mKilledItScreenId) | - |
| 75 | ProName_TopLeft | 0 | 130 ProName | 0 | -1 |
| 76 | ProName_TopRight | 0 | 130 ProName | 1 | -1 |
| 77 | ProName_BottomLeft | 0 | 130 ProName | 2 | -1 |
| 78 | ProName_BottomRight | 0 | 130 ProName | 3 | -1 |
| 79 | OpenResultsScreen | 0 | -1 | PushState(8) (0x826675ec..0x82667628) | - |
| 80 | TutorialGesture_OlliePt1 | 0 | 131 TutorialGesture_Ollie | 1 (on mGestureOllieScreenId) | - |
| 81 | TutorialGesture_OlliePt2 | 0 | -1 | 2 (on mGestureOllieScreenId) | - |
| 82 | GVR_Regular | 0 | 132 GVR_Regular | 1 | -1 |
| 83 | GVR_Goofy | 0 | 133 GVR_Goofy | 1 | -1 |
| 84 | SanVanTV | 0 | 136 NISSanVanTV | 1 | -1 |
| 85 | Take_FixedPhoto | 0 | 62 FixedPhotoTreatment | 1 | - (outside jump table) |
| 86 | Take_NonFixedPhoto | 0 | 63 NonFixedPhotoTreatment | 1 | - (outside jump table) |

Gates (both functions return early for state 88 or -1 and assert state <= 86, 0x826675b4..0x826675e8,
0x82667cf0..0x82667d18):
- OnEnterState runs the switch only if `mScreenMap.find(m360Screen)` hits with id != -1 (0x82667654..0x82667688),
  or mNISState is 70/72/73/74 (0x82667690..0x826676ac), or mNISState is 80/81 and `mGestureOllieScreenId != -1`
  (0x826676b0..0x826676c8). KilledIt cases additionally need `mKilledItScreenId != -1`.
- OnLeaveState runs the switch only on a map hit with id != -1, or `mNISState == 81 && mGestureOllieScreenId != -1`
  (0x82667d70..0x82667d88); entry 81 of its table is the default, so nothing is sent. It always ends with
  `mNISState = 88` (0x82667f80).
- `SetState` 0x8266fe00: `OnLeaveState(mNISState); mNISState = new; OnEnterState(new)`.
- `FrontEndManager::ReceiveNISFEEventMsg` 0x82628570 only forwards when FE state 2 is on the stack
  (`IsStateInStack(2)` 0x8262858c), sets `mControllerManager.mNISRunning = 1`, then `SetState(2, fID)` 0x826285d8.

## 2. FE screen templates used (gScreenTemplate 0x82fdb358, row = 0x82fdb358 + 16*value)

Row: +0 name, +4 swf (relative to `source/screens/`), +8 transition type, +0xC screen type.
All rows below: transition 0 (AS `OpenScreen` path), screen type 0 (registered as "0" by
`FrontEndManager::RegisterScreenTemplates` 0x82628098, mapping 0x826280e0..0x82628158).

| value | name | swf |
|---|---|---|
| 38 | Pro_Bio_Info | nis/pro_bioinfo_overlay.swf |
| 39 | Pro_Challenge | nis/pro_challengeinfo_overlay.swf |
| 48 | SpotFound | challenge/spotfound.swf |
| 49 | SpotOwned | challenge/ownthespot.swf |
| 50 | SpotKilled | challenge/killedit.swf |
| 54..59 | Imagesize_DoublePage_Large/FullPage_Tall/HalfPage_Med/QuarterPage_Small/SBMCover/ThrasherCover | nis/imagesize_<lowercase>.swf |
| 60 | SponsorLogo_Setup | nis/sponsorlogo_setup.swf |
| 61 | PhotoTreatment | hudComponents/phototreatment.swf |
| 62 | FixedPhotoTreatment | hudComponents/fixedphoto.swf |
| 63 | NonFixedPhotoTreatment | hudComponents/nonfixedphoto.swf |
| 67 | NIS_Overlay | nis/nis_overlay.swf |
| 80 | HighlightReelHUD | hud2/highlight_reel_overlay.swf |
| 130 | ProName | nis/proname.swf |
| 131 | TutorialGesture_Ollie | nis/gestollie.swf |
| 132 / 133 | GVR_Regular / GVR_Goofy | nis/gvrreg.swf / nis/gvrgoofy.swf |
| 136 | NISSanVanTV | nis/sanvantv.swf |

## 3. Preload and show sequence (basic career NIS)

### 3.1 Request (presentation thread)
1. `SceneManagerImpl::StartNIS` sends `cMsgNISFEPushState{0}`; `FrontEndManager::ReceiveNISFEPushStateMsg` 0x82633f08
   pushes FE state 2 (pops 81 first) if state 2 is not on the stack and `FENISManager::IsNISActive` is false
   (0x82633f98..0x82633fd0). `FrontEndState_NIS::OnEnter` 0x82658610 touches no screens (HUD hide, subtitles,
   `OnScreenReady(-1)`). `OnUpdate` (vtable 0x8209ab00 slot +0x10) is the shared empty function 0x825187c0.
2. `SceneManagerImpl::UpdateLoadState` 0x82875bc0, load state 1 (runs only when online or sim paused):
   `NotifyNISLoadStart`, `AudioQueueForStart` (0x82875ce8), then `ScenePlayer::FERequestPreload(gNISSceneHash)`
   (0x82875d04, IScenePlayer vtable 0x82140df0 +0x18 = 0x8286c618).
3. `FERequestPreload` 0x8286c618: if `kNISQueuesTheFE` (cVarBoolRef 0x830589fc, default 1 at 0x82f720bc) and
   `mSceneCamera->GetListOfOverlays(hash, &usage)` is true: send `cMsgNISFERequestPreload` (kind 0x77645B8F,
   size 0x20, +0x10 `SceneOverlayUsage_t{u8 numOverlays; u8 overlayIDs[15]}`) to `FrontEndManager::EventBroadcaster`
   (0x8286c6fc) and set `ScenePlayer+0xF2 mIsFEWaitForPreload = 1`; otherwise `mIsFEWaitForPreload = 0` (0x8286c714).
   This is the only sender (`SendMsg<cMsgNISFERequestPreload>` 0x8286b188 has this single caller).
4. `SceneCamera::GetListOfOverlays` 0x82864f10: clear usage; group = camera group of the scene, track =
   `GetSceneCameraTrack(hash)`. If `ICELITEGroup::GetAssembly(group, track)` 0x82a56120 (n-th of AssemblyList) is
   non-null, every regular take `0..NumTakes-1` is scanned (0x82865018..0x8286505c), else only `GetTake(group, track)`.
   `ICEManager::LoadCameraSet` puts a take in AssemblyList when `ElementCounts[35].nIntervals != 0`, else in TakeList
   (0x82a3e830..0x82a3e888). For scene type 6 (`SCENE_TYPE_PHOTO_OUTRO`, enum 0x82883430) with < 15 entries it
   appends `GetTimeAtPhotoShot(hash, 0) == 0.0 ? 85 : 86` (0x828650f0..0x82865150, no dedupe).
5. `SceneCamera::GetOverlaysFromTake` 0x82861dd8: for every interval i of channel 7 (`nIntervals`),
   `v = (u8)ICETake::GetValueInt(107, i)`; append if not already present and count < 15 (value 0 is included).
   (ICELITE variant `SceneLiteCamera::GetOverlaysFromTake` 0x82879210 uses element 104 and has no photo append.)

### 3.2 Preload (FE)
6. `FrontEndManager::ReceiveNISFERequestPreloadStateMsg` 0x82634dc8 copies the ids into a vector and calls
   `FrontEndState_NIS::PreloadNISStates` 0x8268a948 on FE state object 2 (fetched even if not pushed).
7. `PreloadNISStates`:
   - `mNumLoaded = 0`, `mNumScreensToLoad = list size` (statics 0x8304a120 / 0x8304a124; duplicates of a screen and
     screen-less states still count).
   - temp set = every key currently in mScreenMap; `mSponsorsToLoad.clear()`, `mbSponsorReady = 1`.
   - for each state 0..86 with `mIsSponsor` and state != 79: push `mSponsorId` (0x8268aaa0..0x8268aae8).
   - if any sponsor: `mbSponsorReady = 0`; if SponsorLogo_Setup (60) is in mScreenMap: `SetScreenState(id, 100)`
     (0x8268aba8; AS `kReloadImages`).
   - `mNumBiosShown = 0`; for each state: `PreloadNISState(state)`, then remove its m360Screen from the temp set.
   - every key left in the temp set: `UnloadScreen(key)` (0x8268ad00).
8. `PreloadNISState` 0x82689520 (asserts state >= 0):
   - state >= 87, m360Screen == -1, or state 81: `++mNumLoaded; CheckLoadComplete()` (0x82689688).
   - 70/72/73/74: if `mKilledItScreenId` (0x82fdf3f0) == -1, `FrontEndManager::OpenScreen(0x32, cb, "screen")` with
     cb = {mScreenId=&mKilledItScreenId, mLoadedCallBack=KilledItReady 0x82658708} and return (0x8268962c..0x82689674);
     else count as loaded. Two KilledIt variants in one list would open template 0x32 twice (id still -1).
   - 80: same with template 0x83, `&mGestureOllieScreenId` (0x82fdf3f4), `TutorialOllieReady` 0x826587e8.
   - others: `OpenScreen(state, m360Screen)` 0x82684a40.
9. AS side (screen manager movie `data\fe\source\helper\screenmanager.apt`):
   - `ScreenManager_OpenScreen(template, type)` -> `OpenScreen` -> `ReadyScreen(template, type, InitialStandardScreenReady)`
     (AS 0x000b41/0x000b77): creates `mScreenArray["Screen_"+id] = {Name, Clip, screenType, Loaded=false, Instance,
     Id, mInputEnabled=false, Callback}`, queues it, returns id; native stores `atol` of the result
     (`ScreenManagerBase::OpenScreen` 0x82d6a9cc) and keeps the sScreenCallBack copy in `mPendingLoadedCallback[id]`.
   - `Update` (onEnterFrame, AS 0x000f64): `ProcessDestroyedScreens()` first; only if it removed nothing,
     `ProcessOpenScreens` (AS 0x001029): if `mBlockScreenOpen > 0` decrement and return; else if screens are pending
     and no destroy is pending, create ONE screen and set `mBlockScreenOpen = 8` (AS 0x001084). So queued screens are
     created at most one per 9 onEnterFrame calls; onEnterFrame is cleared when both queues are empty.
   - `CreateScreen` (AS 0x000884): type 'popup' -> gPopups, 'hud' -> HUD containers, otherwise template type '1'..'4'
     -> `_level1.._level4` 3D containers, else `mScreen.createEmptyMovieClip(clip, depth)` (AS 0x000ab9) followed by
     `loadMovie(url)`. NIS screens use layer `DEFAULT_SCREEN_LAYER` 0x83024bbc = "screen" (string 0x8233b360) and type
     "0", so they live directly under the screen manager root clip.
   - when the movie's class calls `ScreenManager.OnLoaded(clip, class)` (AS 0x0005b3): `Loaded = true`, `Class = class`,
     `class.Initialise()`, `Callback(Id)` (= `InitialStandardScreenReady` -> `PlayClip(id, 'intro')`, AS 0x000b11),
     `class.OnLoaded()`, `Game.OnScreenLoaded(Id)`.
10. `GameCmdTranslator::sMethod_OnScreenLoaded` 0x82636988 -> `ScreenManagerBase::OnScreenLoaded` 0x82d67f10:
    `*cb.mScreenId = id` (0x82d67f70), `cb.mLoadedCallBack(id, cb.mpObject)` (0x82d67f8c), erase the pending entry.
11. Loaded callbacks: `FrontEndState_NIS::OnLoaded` 0x8267c740 -> `MarkAsLoaded(*wrapper, id)` 0x82676d00
    (`mScreenMap[screen] = id`, `mLoadedScreenIds.push(id)`, `++mNumLoaded`, `CheckLoadComplete`) and frees the
    4-byte "NIS_screen_wrapper"; `KilledItReady` / `TutorialOllieReady` store the static id, `++mNumLoaded`,
    `CheckLoadComplete`.
12. `CheckLoadComplete` 0x82648538: if `mNumLoaded == mNumScreensToLoad && mbSponsorReady`:
    `NISManager::FEPreloaded` 0x828a3050 and `mNumScreensToLoad = 0` (0x826485a4). FEPreloaded sends
    `cMsgNISFEPreloaded` (kind 0xE2DA931F, size 0x10) to `Presentation::mBroadcaster` only if `mIsInNIS || mIsInICE`.
    Sponsor readiness comes from the sponsor movie calling `NIS.LogosReady()` -> `sMethod_LogosReady` 0x8260db40
    (`mbSponsorReady = 1; CheckLoadComplete`).
13. `SceneManagerImpl::ReceiveNISFEPreloadedMsg` 0x82861590 -> `ScenePlayer::SetFEPreloaded` 0x82861c50
    (`+0xF2 = 0`). `IsFEPreloaded` 0x82861c20 = `!kNISQueuesTheFE || !mIsFEWaitForPreload`; load state 5 requires
    `gNISSceneIsLoaded && IsAudioReadyForStart && IsFEPreloaded && (!mIsInTransition || direction == OUT)`
    (0x82876204..0x8287626c). There is no timeout.

### 3.3 Show (playback)
14. `ICEUserChannelOverlays::UpdateFrameBegin` 0x826d1a28 (channel type 7): requires active ICE scene `IsPlaying`, take
    != NULL and `Player()->GetDeltaTimeMult() > 0` (0x826d1ad8); `i = take+0x2CC` (Channels[7].nCurrentInterval);
    if i != 0xFFFF: `v = GetValueInt(107, i)`; if `v != mCurrentOverlayState && v < 87` (signed) send
    `cMsgNISFEEvent{fID=v}` (kind 0x4EB59F71, size 0x14) to FrontEndManager+0x5C0 and store v. Value-change
    detection; Init/OnTakeSetup/OnTakeStart (vtable 0x820b33c0 slots 1-3 = 0x826c7730) reset the state to 88,
    OnTakeChange is empty.
15. FE: `SetState` -> `OnLeaveState(old)` / `OnEnterState(new)` -> `ScreenManagerBase::SetScreenState` 0x82d65d50
    (AS `ScreenManager_SetScreenState("id","n")` with `%d` strings) -> AS `SetScreenState` (AS 0x00126b):
    `mScreenArray["Screen_"+id].Class.SetState(n)`.

### 3.4 Teardown
16. `FrontEndManager::PopNIS` 0x82640cb8 (outside FE mode): if state 2 is on the stack:
    `DeletePreloadedScreens` 0x82658678 (DestroyScreen every mLoadedScreenIds entry, `mNumLoaded = mNumScreensToLoad =
    mNumBiosShown = 0`, clear mScreenMap/mLoadedScreenIds/mSponsorsToLoad) then `PopState` -> `OnLeave` 0x8266fd70:
    `OnLeaveState(mNISState)` (map already empty -> sends nothing), destroy the KilledIt screen only if `mICEOpened`,
    always destroy the gesture screen (`mGestureOllieScreenId = -1`, 0x8266fdbc..0x8266fdd8), `mICEOpened = 0`,
    `FESubtitle::EnableAutoTimeOut(true)`. `mNISRunning = 0`.
17. `mICEOpened` (+0x75C): the only stores found are `= 0` in the ctor (0x8267470c) and OnLeave (0x8266fde0)
    (text search of `stb rX,0x75C(` over 0x82430000..0x82fc2000). So the KilledIt screen is not destroyed by OnLeave
    and is not in mLoadedScreenIds; it stays loaded (static id kept) until `ForceKilledItUnload` 0x82658798, whose only
    caller is AS native `KilledItHUDOutroComplete` 0x82624560 (registered 0x8262caec). No movie in screens.big
    references that name.
18. `OnLeaveState` case 0 (Clear) calls `ForceGestureOllieUnload` 0x82658830, but leaving state 0 can only reach the
    switch through a map hit for screen -1, which never exists, so this path is not taken in practice.
19. `ScreenManagerBase::DestroyScreen` 0x82d68138: AS `DestroyScreen(id)` (queued; `RemoveScreen` later calls
    `Class.Destroy()`, `removeMovieClip`, `Game.OnScreenUnLoaded`), drops a pending loaded callback, then
    `OnScreenIntroComplete(id,1)` / `OnScreenOutroComplete(id,1)`.

### 3.5 Career ICE lists (distinct element-107 values per context, union of all takes)
OwnTheSpot {2,3,4,5,7,54,70} (camera-track take of SPOT_INTRO 12 scenes: mostly 0 + a SpotFound corner 2-5 + 54
Take_Photo; SPOT_OUTRO 13: 0,7 with freeze cmd 3/30 frames in 36 scenes, 0,70 in 25); Photo {70} (PHOTO_OUTRO 6 adds
85/86 per 3.1 step 4; PHOTO_INTRO 5 scenes have no channel-7 data, so their freeze cmd 4 events reach
OnEnterState(86) with no screen loaded and do nothing);
ProIntro `NIS_AChalmers_ProIntro_S01_0` takes [0,10,0], [0,39,0] + an assembly take -> list {0,10,39};
ProChallenge `NIS_AChalmers_ProChallenge_S01_0` [80]; Tutorial `tutorial_m_1a_goto_slappys` [0,80,81,0] x3 + assembly
-> {0,80,81}.

## 4. Visibility after load (per movie, before any SetState)

`AptCIH::_gotoAndX` 0x82a1cf48: a missing label gives `labelToFrame` 0x829e6ee0 = -1, frame index stays < 0 and
nothing happens (0x82a1d00c). So `PlayClip(id,'intro')` is a no-op for movies without an `intro` label.

| movie | root frame 0 | `intro` label | visible after load |
|---|---|---|---|
| gestollie | ctor `mScreen.stop()`; mBacker placed without colour transform (opaque, text visible), mName alpha 0 | frame 0; frames 1-28 fade mName alpha 0x10..0xFF, frame 29 stop | YES: backer + text at once, stick icon fades in |
| proname | mName alpha 0 | frame 0, same 28-frame fade, stop 29 | YES ("TEMP Pro Name" at topLeft) |
| spotfound | all 3 clips alpha 0, `stop` | none | no (until SetState intro*) |
| ownthespot | 2 clips alpha 0, `stop` | none | no |
| killedit | one clip alpha 0, `stop` | none | no |
| phototreatment / fixedphoto / nonfixedphoto | snapshot_mc alpha 0, ctor `stop()` | none | no |
| pro_bioinfo_overlay | frame action `this._visible = false` | none | no (`_visible = true` at introLeft/introRight) |
| sponsorlogo_setup | `this._visible = false; stop` | none | no |
| pro_challengeinfo_overlay | `this._visible = false` | none | no |
| nis_overlay | content placed at x -676..-541 (left of the 1280 stage), ctor `stop()` | none | not on stage until introLeft/Right (exact bounds UNVERIFIED) |

No native code on this path calls ShowScreen/SetVisible; visibility comes only from each movie's timeline and its
SetState handler. So the gesture tip is on screen from the moment its movie loads (during NIS load state 1-5) until
`FrontEndState_NIS::OnLeave` destroys it; the other career overlays stay invisible until their intro SetState.
Whether the FE layer is drawn above the 1 s bloom-to-white NIS load transition is UNVERIFIED (render order not traced).

## 5. OpenScreen / UnloadScreen

`FrontEndState_NIS::OpenScreen(nisState, screen)` 0x82684a40 (nisState unused):
```
if (mScreenMap.find(screen) != end) { ++mNumLoaded; CheckLoadComplete(); return; }   // 0x82684a84..0x82684a9c
mScreenMap[screen] = -1;                                                              // 0x82684ac0
cb = {0}; cb.mScreenId = &mLastScreenId (+0x750); cb.mLoadedCallBack = OnLoaded 0x8267c740;
cb.mpObject = new int(screen) ("NIS_screen_wrapper", 4 bytes);
FrontEndManager::OpenScreen(screen, &cb, DEFAULT_SCREEN_LAYER)                        // 0x82684b30
```
(sScreenCallBack 0x14: +0 mScreenId, +4 mpObject, +8 mLoadedCallBack, +0xC mIntroCallBack, +0x10 mOutroCallBack;
helpers 0x82d65c00 zero-init, 0x82607be0 SetIdReference, 0x82a7f658 store +8, 0x82e4cbf0 SetCallBackObject; IDA shows
COMDAT-folded names for the first and third.) No screen-state number is sent.

`FrontEndState_NIS::UnloadScreen(screen)` 0x8266fe50: if mScreenMap has `screen`: id = value, erase the node,
`FrontEndManager::DestroyScreen(id)` 0x826063b0, remove the first `id` from mLoadedScreenIds (memmove). No state number.
Only caller: PreloadNISStates 0x8268ad00. Within FrontEndState_NIS the only screen-state sends outside
OnEnter/OnLeaveState are `100` to SponsorLogo_Setup (0x8268aba8) and `5` to the KilledIt screen from
`ForceKilledItOutro` 0x82658750 (only caller `ReceiveHighlightReelCompleteMsg` 0x826287c0).

## 6. Movies (career contexts first)

### gestollie (131, TutorialGesture_Ollie; states 80/81)
- Class `gestollie` (AS 0x00370b): ctor `mScreen._x -= _global.Screen_EdgeOffset`; `mBacker.label.autoSize =
  mBacker.labelGlow.autoSize = 'center'`; if `label.maxscroll > 2`: `mBackerbottom._y` and `mBackerreflect._y` += 
  (maxscroll-2)*22, `mBackercore._height = mBackerbottom._y - mBackercore._y`; `mScreen.stop()`; RegisterScreen.
- SetState (AS 0x003891): `1` (kScreenState_OlliePt1) -> `mName.gotoAndPlay('olliedown')`; `2` (OlliePt2) ->
  `mName.gotoAndPlay('ollieup')`; constants Clear -1, OlliePt1 1, OlliePt2 2 (AS 0x003954..0x003961). No outro call.
- Root: `intro` 0, `outro` 30 (mName fades out 32-58, stop 59) - never triggered by native.
- mName sprite (char 30): `idle` 0 (stop 3), `olliedown` 4 (stop 48), `ollieup` 49 (stop 154).
- No native data read. Destroyed only by OnLeave (3.4).

### spotfound (48, SpotFound; states 2-5)
- SetState (AS 0x0062bd): first refreshes texts: if `!Mission.GetIsOnlineMission() && !Mission.GetIsPartyPlayMission()`
  (AS 0x0062f8/0x00630b): spot name = `Mission.GetTitle()`, header = `ID_EVENT_SPOT_FOUND_CAPS` ("Own the Spot");
  else header = `ID_MISSION_TEMPLATE_ONLINE_SPOTBATTLE` ("Spot Battle"); autoSize right (mcOwned) / left
  (mcTextLeftJustified); mcPoints/mcABDPoints writes target clips that do not exist in the movie.
- States: 0 IntroBottomRight, 1 OutroBottomRight, 2 IntroTopRight, 3 OutroTopRight, 4 IntroBottomLeft,
  5 OutroBottomLeft, 6 IntroTopLeft, 7 OutroTopLeft (AS 0x0066d2..0x006702). Intro states shift `mScreen._x` by
  -EdgeOffset (right) / +EdgeOffset (left) once (when `_x == mOriginalX`).
- Labels: introBottomRight 1 (`_visible = true`, `Audio.PlaySound(FE_SOUND_SPOT_FOUND)`), holdBottomRight 24 (stop),
  outroBottomRight 60, end 71 (`_visible = false; stop`); TopRight 73/96/127/138; BottomLeft 140/163/194/205;
  TopLeft 207/232/263/274.

### ownthespot (49, SpotOwned; states 6-9)
- Same state numbers 0..7 as spotfound (AS 0x0071e3..0x007213). Static text `ID_COMMON_OWNED` ("owned").
- Labels: introBottomRight 1 (`PlaySound(FE_SOUND_CHALLENGE_OTS)`, `_visible = true`), hold 106 (stop), outro 114,
  end 126 (`_visible = false; stop`); BottomLeft 127/231/243/254; TopLeft 255/351/361/370; TopRight 371/474/485/498.
- `EnableRenderEffect` (`Render.SetEffect`) and `playAllAnimations` exist but are not called by SetState.

### killedit (50, SpotKilled; states 70, 72-74)
- SetState (AS 0x01fcf4): 1 -> `topleft_intro`, 2 -> `topright_intro`, 3 -> `bottomleft_intro`, 4 ->
  `bottomright_intro` (each also `logoAnimation.gotoAndPlay('intro')`), 5 kOutro -> `logoAnimation.gotoAndPlay('outro')`.
  The movie has no named instances, so `logoAnimation` calls do nothing.
- Each variant plays intro -> flourish -> outro unattended: bottomleft 1/28/105, end stop 115; bottomright
  116/143/220/230; topleft 231/258/335/345; topright 346/373/450/460. Sounds FE_SOUND_KILLEDIT_INTRO / _FLOURISH /
  _OUTRO at those labels. Static text `ID_CHALLENGE_COMMON_KILLED_IT` ("Killed it.").

### phototreatment / fixedphoto / nonfixedphoto (61 / 62 / 63; states 54 / 85 / 86)
- ctor: `mScreen.stop()`; `snapshot_mc.gotoAndPlay(Screen_EdgeOffset == 0 ? 'widescreen' : 'standard')`.
- SetState: 0 nothing, 1 -> `gotoAndPlay('snapshot')`: frame 1 alpha 1 + sound, fade 17-25, frame 26 alpha 0 + stop.
- Sound: FE_SOUND_PHOTO_FLYTHROUGH / FE_SOUND_PHOTO_SNAPPING / FE_SOUND_PHOTO_SKATE1. No leave case (85/86 are outside
  the OnLeaveState table; 54 is the default).

### pro_bioinfo_overlay (38, Pro_Bio_Info; states 10/11)
- SetState (AS 0x029a11): 0 IntroLeft (`GetInfo()` then `introLeft`), 1 OutroLeft, 2 IntroRight (`GetInfo()`,
  `introRight`), 3 OutroRight. Labels: everything 0, introLeft 10 (`PlayInterfaceOverlayIn`, `_visible = true`),
  stop 31, outroLeft 32 (`PlayInterfaceOverlayOut`), 65 (`_visible = false; stop`); introRight 66, stop 87,
  outroRight 88, 121 (`_visible = false; stop`).
- `GetInfo` (AS 0x029909): `info = NIS.GetProInfo()`; mcTextHeader.labelText = info[0]; mcTextBody2.labelText = info[1],
  labelTitle = `ID_NIS_HOMETOWN`; mcTextBody3.labelText = info[2], labelTitle = `ID_NIS_STANCE` ("STANCE:").
- `NISCmdTranslator::sMethod_GetProInfo` 0x8260d840: main challenge activity; if it is NULL or its attribute
  0x02B828CCDB4785E4 (u32, 0x8260d8f0..0x8260d928) is nonzero -> ["#Bad Pro", "#Brooklyn, NY", "#Goofy"]; else pro by
  activity name: fp_u3_tutorial03 -> 7, fp_u3_tutorial04 -> 8, fp_u3_tutorial01 -> 4, fp_u3_tutorial02 -> 1, any other
  -> 5 (0x8260d950..0x8260d9c0); returns [NISBackEnd::GetProName, GetHometown, GetIsGoofy ? ID_CHEAT_PRODETAIL_GOOFY :
  ID_CHEAT_PRODETAIL_REGULAR]. ID_NIS_HOMETOWN and both ID_CHEAT_PRODETAIL_* are absent from the English Global/
  FrontEnd language BINs in miscboot.big.

### sponsorlogo_setup (60, SponsorLogo_Setup; sponsor states)
- ctor: RegisterScreen, `LoadImages()`, `mcObject._x += Screen_EdgeOffset`. `LoadImages`: `mLogosToLoad =
  NIS.GetLogosToLoad()` (`sMethod_GetLogosToLoad` 0x8260da50 = array of mSponsorsToLoad), `mNumLogos = 0`,
  `LoadImage(id)` each: create `LoadArea.mcArea<Depth>` once per id and `loadMovie('source/images/sponsor_logos/' +
  sponsorlogo_<name>.swf)`; on load `_visible = false`; when `mNumLogos == length` -> `NIS.LogosReady()`.
- SetState (AS 0x01c024): 100 -> LoadImages; 50 -> `outroLeft`; 51 -> `outroRight`; any other n -> `introLeft`,
  hide logo `state_<old>`, show `state_<n>`, `mOldState = n`. Ids: adidas 0, alien 1 ... supra 44 (same as mSponsorId).
- Labels: introLeft 10 (`_visible = true`, `PlaySignupBack`), stop 51, outroLeft 52 (`PlaySignupError`), 77
  (`_visible = false; stop`); introRight 83, stop 124, outroRight 125.

### Not in the career contexts
- nis_overlay (67): states 0 introLeft, 1 introRight, 2 outroLeft, 3 outroRight; ctor fills texts from
  `Mission.GetChallengeTypeName/GetTitle/GetDescription`, icon from `GetChallengeIconString(GetSignupMissionType,
  GetSignupMissionPath)`. outroLeft/outroRight ends call `ScreenManager.OutroComplete(screen.Id)`, which destroys the
  screen on the AS side (mScreenMap keeps the stale id); introRight end calls `IntroComplete`.
- proname (130): states 0 topLeft, 1 topRight, 2 bottomLeft, 3 bottomRight move `mName`; -1 (native leave) has no handler.
  Text is the literal "#TEMP Pro Name".
- pro_challengeinfo_overlay (39): 0 IntroLeft, 1 OutroLeft, 2 IntroRight, 3 OutroRight; placeholder texts only.

## 7. gestollie texts (exact)

| field | char | initial text | resolves to (English Global) | font / size / align / colour |
|---|---|---|---|---|
| mBacker.labelGlow | 14 | `ID_CHALLENGE_TIPS_OLLIE` | `Pull right stick back^Push right stick forward` | HelveticaPresto (char 13) / 20 / 2 / 0xFFFFDB57, multiline+wrap |
| mBacker.label | 15 | `ID_CHALLENGE_TIPS_OLLIE` | same | HelveticaPresto / 20 / 2 / 0xFFFFFEEC, multiline+wrap |
| mName.stickName_txt | 25 (frame 0), 29 (re-placed frame 154) | `ID_CAC_RIGHTANALOG_ABBREVIATION` | `RS` | Helvetica LT Std (char 24) / 18 / 2 / 0xFFFAFCDB |

`SkateAptString::SkateAptString` 0x82a70b28: text starting with '#' is literal, otherwise
`LocalizerManager::GetPackedString(StringHash(text))`; in multiline fields '^' (94) is a forced line break
(0x82a70df8), so the tip renders as two lines. The AS never writes these fields.

## 8. ICEUserChannelFreeze (channel type 34, ctor 0x826cf100)

Object: +0xC u16 mPreviousInterval (ctor -1), +0xE u8 mIsTakeChange (0), +0xF u8 mInstanceFrozenSkater (0).
Vtable 0x820b3c30: Init/OnTakeSetup empty, OnTakeStart 0x826cb400 (`mPreviousInterval = 0xFFFF`), OnTakeChange
0x826d6e78 (`mPreviousInterval = 0xFFFF; mIsTakeChange = 1`), UpdateFrameBegin 0x826d2498, UpdateFrameEnd 0x826cf1a8,
GetLabel 0x826cb508 (141 -> `gICEFreezeCmds[v]` for v < 9: None, Timeout, Permanent, Owned, PhotoShot,
PhotoShot_FixedFrame, Killable, Jump, LoadScreenAnimChange; 142 -> "%d").

```
UpdateFrameBegin(t):                                    // 0x826d2498
  s = TheICESceneManager->GetActiveScene(); if !s || !s->IsPlaying() || !t: return
  cur = t+0x47C (Channels[34].nCurrentInterval); if cur == 0xFFFF: return
  if mPreviousInterval == 0xFFFF:
    if mIsTakeChange: mPreviousInterval = cur; mIsTakeChange = 0          // first interval after a take change: no event
    else: FireEvent(GetValueInt(141,cur), GetValueInt(142,cur)); mPreviousInterval = cur
  else:
    for i = (u16)(prev+1); i <= cur; i = (u16)(i+1): FireEvent(GetValueInt(141,i), GetValueInt(142,i))
    mPreviousInterval = cur                                               // backward jumps fire nothing
```
Detection is by interval index advance (every crossed interval fires once, in order), not by value change, and it
does not test GetDeltaTimeMult (commands 4 and 5 test it themselves).

```
FireEvent(cmd, frames):                                 // 0x826d1b50, f = (float)frames * 0.033333335f (0x3d088889)
  1 Timeout:   if frames: P->SetFreezeTimeMult(0); P->SetFreezeResetCounter(f)          // 0x826d1bd0..0x826d1c1c
  2 Permanent: P->SetFreezeTimeMult(0)                                                  // 0x826d1ca4
  3 Owned:     same as 1                                                               // 0x826d1cc4..0x826d1d44
  4 PhotoShot: if P->GetDeltaTimeMult() > 0:
                 if frames: SetFreezeTimeMult(0); SetFreezeResetCounter(f)
                 send cMsgNISFEEvent{86}  -> FrontEndManager+0x5C0                     // 0x826d1d64..0x826d1e1c
  5 PhotoShot_FixedFrame: if P->GetDeltaTimeMult() > 0:
                 e = local skater pres entity; if TestSphere_VMX(GfxMan->mCameras[5].mFrustum, e->GetBoundsCenter(),
                    e->m_BR, e->GetMinPixelSize()) != 0x40 (Outside):
                   mInstanceFrozenSkater = 1
                   if frames: SetFreezeTimeMult(0); SetFreezeResetCounter(f)
                   ice = Presentation->mPostfxPresentation.mICEPostfxData (Presentation+0x7A70)
                   ice.bloomAnimBaseValue = ice.bloomThreshold; ice.bloomAnimValue = s_fBloom (-0.6)
                   ice.bloomAnimCurrentTime = 0; ice.bloomAnimUpTime = s_fBloomUpTime (0.15)
                   ice.bloomAnimDownTime = s_fBloomBackTime (0.1)
                   send cMsgNISFEEvent{85}                                              // 0x826d1e3c..0x826d1fb0
  6 Killable:  if !ICEManager::IsEditorOn(): send cMsgNISChallengeNotify{fNotificationID=3 NIS_FE_NOTIFY_PLAYING_KILLABLE}
               (kind 0x95C091D3, size 0x14) -> Sk8ChallengeSystem+0x38 broadcaster       // 0x826d1fd0..0x826d2020
  7 Jump:      s = GetActiveScene(); if s: s->SetTime(f)                                // 0x826d2040..0x826d208c
  8 LoadScreenAnimChange: SceneManagerImpl::PlayNextSceneCharacterViewAnim()          // 0x826d20b8
  P = SceneManager::Player(); IScenePlayer vtable +0xC4 SetFreezeTimeMult, +0xC8 SetFreezeResetCounter,
  +0xD0 GetDeltaTimeMult. cMsgNISFEEvent kind 0x4EB59F71 size 0x14.
```
Bloom constants: s_fBloom 0x82fe2d8c = 0xBF19999A (-0.6), s_fBloomUpTime 0x82fe2d88 = 0x3E19999A (0.15),
s_fBloomBackTime 0x82fe2d84 = 0x3DCCCCCD (0.1); only read by FireEvent. Consumer `PostfxPresentation::Update(dT)`
0x827efe58 (called from `Presentation::Update` 0x827f7608), with t = bloomAnimCurrentTime:
`t < 0`: idle; `t < up`: threshold = base + (value-base)*(t/up), t += dT; `t < up+down`: threshold =
value + (base-value)*((t-up)/down), t += dT; else t = -1, threshold = base.

```
UpdateFrameEnd():                                       // 0x826cf1a8
  if !mInstanceFrozenSkater: return; mInstanceFrozenSkater = 0
  ps = SceneManagerImpl->mCharacterPoseSaver (+0x38); pose = ps->GetMostRecentPoseBuffer()  // 0x82862648
  if pose: ent = CharacterManager()->InstanceFrozenSkater(&ps->mLocalToWorld (+0x10), pose)  // 0x82874f90
           if ent: Player->AddEntity(Player->GetActiveSceneHandle(0) (vtable +0x60), ent)    // 0x8286d1d0
```
Shipped usage (union over takes): OwnTheSpot cmd 3/30 frames (38 intervals), cmd 1/20 (1); Photo cmd 4/10 (447),
cmd 4/0 (1), cmd 5/10 (88); RaceStart cmd 3/30 and 7 (jump); Highlight cmd 3/30; Extras cmd 7/0 and 8/0. Cmd 6 has no
shipped intervals.

## 9. UNVERIFIED

- Draw order of the FE "screen" layer vs the NIS bloom-to-white transition (whether gestollie is seen during load).
- Apt playback rate for FE movies (frame_ms 16 in every overlay movie; screen manager movie 83) and how often
  `onEnterFrame` runs, i.e. the real time of the 8-frame open throttle and the 28-frame gestollie fade.
- `_global.Screen_EdgeOffset` value (set outside these movies).
- Exact on-stage bounds of nis_overlay content at frame 0.
- Name of attribute 0x02B828CCDB4785E4 read by GetProInfo; pro enum names other than via the decompiler
  (Way 7, Haslam 8, Dill 4, Rodriguez 1, Caroll 5).
- Callers of the ICE channel OnTakeStart/OnTakeChange (ICE channel manager) were not traced.
