> Agent research (2026-10-04), read-only IDA. Most of it was re-verified while porting
> `crates/skate2-core/src/nis/`; re-verify anything else in IDA before porting it.

## 4. Playback flow: from `RunBasicNIS` to `cMsgNISChallengeNotify`

### 4.0 Threads, objects and queues involved

| Side | Object | Where it is updated |
|---|---|---|
| Sim thread | `Sk8::NISManager` (`Simulation::mWorkspace.mNISMan`, Simulation+0xA4+0x30) | `SimController::Update1` 0x828a72c8 dispatches `Simulation::mBroadcaster` (Simulation+0x38) messages, then calls `NISManager::Update` 0x828a67e8 **every sim update, even while paused** (only `Simulation::SimUpdate1` is skipped when `PauseMan` flag 1 is set). |
| Sim thread | `Sk8::VisualTransitionManager` (sim half of fades), `Sk8::TransitionManager`, `Challenge::Sk8ChallengeSystem` | sim update |
| Presentation (render) thread | `Sk8::Scene::SceneManagerImpl` (+ `ScenePlayer`, `Scene`, `SceneCamera`, `SceneLoadRequestManager`, ...) | `GameModule::Render(dT)` 0x827320f0: dispatches `SK8App->mPresThreadMsgMan` messages, `SceneManager::UpdateFrameBegin(dT)` 0x82878730 -> `SceneManagerImpl::UpdateFrameBegin` 0x82878428, then `Presentation::Update(dT)` 0x827f7540 (which first **dispatches `Presentation::mBroadcaster`** - the queue on which `SceneManagerImpl::AddEventHandlers` 0x82877160 registered all the `cMsgNIS*` handlers - then updates characters, living world, `TransitionPresentation`, particles, world, post-fx, VI, and copies `CameraPresentation::GetMatrix` into `mCameraMatrixForSim`), then `SceneManager::UpdateFrameEnd` 0x828770e8 -> `SceneManagerImpl::UpdateFrameEnd` 0x82875a90, then audio update and `RenderMan::Render`. |
| Presentation | `Sk8::PresVisualTransitionManager` (`Presentation::mPresVisualTransitionManager`, Presentation+0x7900) and `TransitionPresentation` (+0x791C) | `Presentation::Update` |
| FE | `FE::FrontEndManager`, `FE::FrontEndState_NIS` (FE state id 2) | FE update |

Message transport: all messages are `Hermes::Message` (16-byte header: `mKind` hash, `mPort`, `mSize`, `mID`) enqueued with `Hermes::Message::Enqueue(msg, port, 5)` by the generic `ThreadSafeSinglePortBroadcaster::SendMsg<...>` (the bodies are COMDAT-folded, so IDA names several of them after an unrelated message type, e.g. 0x828a2848 is shown as `SendMsg<cMsgNISUnloadAll>` but is the generic sender used for every sim->presentation NIS message; the actual type is the `mKind` written by the caller). Sim->presentation messages go to `Presentation::msInstance->mBroadcaster` (Presentation+0x8, queue `eEQGamePres`); presentation->sim messages go to `Simulation::mBroadcaster` (Simulation+0x38); challenge messages go to `Sk8ChallengeSystem::EventBroadcaster` (ChallengeSys+0x38).

Message kind hashes (read from the `ori` constant in each `Hermes::Handler::Create<cMsg...>` at the given address, and matched against the constants written by the senders):

| Message | Kind | Size | Payload | Handler(s) |
|---|---|---|---|---|
| `cMsgNISEnterBasic` | 0x70779205 (Create 0x828e6210) | 0x110 | +0x10 `fSceneName` (NISMessageString, 64 chars), +0x50 `fCameraHoldAtEnd` (bool), +0x60 `fPlaybackOverrides` (NISPlaybackOverridesInternal, 0xA0), +0x100 `fSpecialFlags` | `SceneManagerImpl::ReceiveNISEnterBasicMsg` 0x8286bfc0 |
| `cMsgNISFEPushState` | 0xE93A2220 (0x828e6de0) | 0x14 | +0x10 `fLoadScreenRequired` | `FrontEndManager::ReceiveNISFEPushStateMsg` 0x82633f08 |
| `cMsgNISFERequestPreload` | 0x77645B8F (0x828e6e70) | 0x20 | +0x10 `SceneOverlayUsage_t` (count byte + overlay ids) | `FrontEndManager::ReceiveNISFERequestPreloadStateMsg` 0x82634dc8 |
| `cMsgNISFEPreloaded` | 0xE2DA931F (0x828e63c0) | 0x10 | - | `SceneManagerImpl::ReceiveNISFEPreloadedMsg` 0x82861590 |
| `cMsgNISWorldCollisionQuery` | 0x27A08C41 (0x828a1bb8) | 0x20 | +0x10 `fLocation` | `NISManager::ReceiveNISMgrWorldCollisionQueryMsg` 0x828a3588 |
| `cMsgNISWorldCollisionResponse` | 0xC1F1EC90 (0x828e69f0) | | | `SceneManagerImpl::ReceiveNISMgrWorldCollisionResponseMsg` 0x82861808 |
| `cMsgNISLocationListSnapQuery` / `...Response` | 0xFE94DD4B (0x828a1c48) / 0x20E14066 (0x828e64e0) | | | `NISManager::ReceiveNISMgrLocationListSnapQueryMsg` 0x828a3620 / `SceneLocationSnapper::ReceiveNISMgrLocationListSnapResponseMsg` 0x8286ae98 |
| `cMsgNISLoadingOverlay` | 0x086C5B92 (0x828a1f18) | 0x14 | +0x10 `fShow` | `NISManager::ReceiveNISLoadingOverlayMsg` 0x828a3550 -> `FrontEndManager::EnableLoadingTag(fShow, eLoadingTagFlag_FE)` |
| `cMsgNISLoadFinish` | 0xE22FAFF7 (0x828e6f00) | 0x10 | - | `FrontEndManager::ReceiveNISLoadFinnishedMsg` 0x826144b0 (only sent when a load screen was needed, never for a basic NIS) |
| `cMsgNISMgrPlayback` | 0xB0F2D44F (0x828a1a98) | 0x10 | - | `NISManager::ReceiveNISMgrPlaybackMsg` 0x8289ecf8: `mIsInPlayback = 1` |
| `cMsgNISSkip` | 0x72DAE530 (0x828e67b0) | 0x10 | - | `SceneManagerImpl::ReceiveNISSkipMsg` 0x828615f0 |
| `cMsgNISKill` | 0xF0653C9C (0x828e6450) | 0x10 | - | `SceneManagerImpl::ReceiveNISKillMsg` 0x82861660 |
| `cMsgNISReleaseCameraHold` | 0xC7C08F45 (0x828e6690) | 0x10 | - | `SceneManagerImpl::ReceiveNISReleaseCameraHoldMsg` 0x82863340 |
| `cMsgNISTransitionInBegin` | 0xF640BBEE (0x828a1e88) | 0x10 | - | `NISManager::ReceiveNISTransitionInBeginMsg` 0x828a3528 -> `HudManagerSetNISDisableHUD(1)` |
| `cMsgNISTransitionOutComplete` | 0x539A09BB (0x828a1df8) | 0x10 | - | `NISManager::ReceiveNISTransitionOutCompleteMsg` 0x828a3500 -> `HudManagerSetNISDisableHUD(0)` |
| `cMsgNISMgrComplete` | 0x62F1478B (0x828a1a08) | 0x10 | - | `NISManager::ReceiveNISMgrCompleteMsg` 0x828a3440 |
| `cMsgNISAllUnloaded` | 0x3EB31ACB (0x828a1d68) | 0x10 | - | `NISManager::ReceiveNISAllUnloadedMsg` 0x8289ed30 (clears preload hash) |
| `cMsgNISUnloadAll` | 0x6B7C67B9 (0x828e6960) | 0x10 | - | `SceneManagerImpl::ReceiveNISUnloadAllMsg` 0x8286bf38 (sent by `NISManager::UnloadAnyPreloadedNISs` 0x828a2e40, not by the basic path) |
| `cMsgNISPreloadList` / `cMsgNISPreloadFinish` | 0x2855C0E5 / 0x1CBEA291 | 0x214 / 0x14 | scene-name list / `fSceneHash` | `SceneManagerImpl::ReceiveNISPreloadListMsg` 0x82876820 / `NISManager::ReceiveNISPreloadFinishMsg` 0x8289ed08 |
| `cMsgNISFEEvent` | 0x4EB59F71 (0x828e6c30) | 0x14 | +0x10 `fID` (eSk8HUDNISState) | `FrontEndManager::ReceiveNISFEEventMsg` 0x82628570 |
| `cMsgNISChallengeNotify` | 0x95C091D3 (0x828e6b10) | 0x14 | +0x10 `fNotificationID` (eNISChallengeNotification) | `cChallengePresentationDirector::OnNISChallengeNotify` 0x82e08848, `cSignPostManager::ReceiveNISChallengeNotifyMessage` 0x825d2b60, Lua (`cMsgNISChallengeNotify_HandleMessage_LuaBinding` 0x828cafe0 registered in `BindAllMessageHandlersToLua`) |
| `cMsgNISComplete` | 0xC2D14440 (0x828e6ba0) | 0x10 | - | `cHighlightReelManager::OnNisCompleteMsg` 0x82cd7af8, Lua |
| `cMsgVisualTransitionBegin` / `...End` / `...Complete` | 0xC9CB602F (written at 0x828a3974) / 0x42DE7EBC (0x828a3a4c) / 0x16E328DD (0x827dce1c) | 0x14 / 0x10 / 0x10 | +0x10 `fTransitionType` (Begin only) | `PresVisualTransitionManager::ReceiveVisualTransitionBeginMsg` 0x827dffb0 / `...EndMsg` 0x827dffd8 / `VisualTransitionManager::ReceiveVisualTransitionCompleteMsg` 0x8289eda8 |

Enumerations used below (all from the Lua `BindEnum<>` tables, which list name and value explicitly):
* `eTransitionType` (`BindEnum<eTransitionType>` 0x82884128): NONE 0, ENTER_GAME 1, LEAVE_GAME 2, TELEPORT 3, LOADSCREEN 4, NISLOADSCREEN 5.
* `eTransitionEndParam` (0x82884270): NORMAL 0, EXTERNALRELEASE 1, ACTIVATEONLY 2.
* `eNISChallengeNotification` (0x828838e8): NIS_CHALLENGE_NOTIFY_NONE 0, NIS_CHALLENGE_NOTIFY_COMPLETED 1, NIS_FE_NOTIFY_ENTER_PHOTOVIEW 2, NIS_FE_NOTIFY_PLAYING_KILLABLE 3.
* `eNISKillType` (values from the stores): NIS_KILL_TYPE_NONE 0 (ctor 0x8286b568), NIS_KILL_TYPE_COMPLETED 2 (`UpdateNIS` 0x82876798), NIS_KILL_TYPE_INPUT 3 (`ReceiveNISSkipMsg` 0x8286163c); NIS_KILL_TYPE_MESSAGE / NIS_KILL_TYPE_ERROR also exist (values not needed: `StopNIS` does not branch on the kill type).
* `ePauseState` kPAUSE_STATE_NIS = 11 (`li r4,0xB` in `PauseSim` 0x828a10e8), kPAUSE_STATE_REPLAY = 7 (`EndNIS` 0x828a336c). `PauseMan::Push` 0x827381c8 sets `mFlags = kFlagsTable[state]`; `kFlagsTable` @0x820d8548: entry 11 = 0xFFFFFFF1 (bit0 = kPAUSE_FLAG_SIM set), entry 7 = 0xFFFFFFE1. `kPAUSE_FLAG_SIM` = 1 (`li r4,1` before `IsPaused` at 0x82875c60).

### 4.1 Who calls `RunBasicNIS` in the career

`cSignPostUtil::HandleCaughtMessage` 0x825de4e0, progression message type 2 (`GetMessageTypeIDFromMessageKey` == 2):
* reads string property "Scene Name" and int property "Transition Type" via `cSignPostManager::GetMessageStringProperty/GetMessageIntProperty`;
* if cvar `kEnableSignPostMedia` is false: no NIS, it immediately sends `cMsgNISChallengeNotify{fNotificationID=1}` to the challenge system (0x825de6d0..0x825de6fc);
* if the sign-post manager is `InLoadingScreen`: `AddCachedMessage` (replayed later);
* if the scene is one of "04_Arrival_of_Mikey", "18_mag_cover", "17_Thrasher_Cover", "22_SlappyEnder": it stores `mCachedNISMessageKey` and `mNISWaitTimerTicks = 12`; `cSignPostUtil::Update` 0x825c61d8 decrements the counter once per call and at 0 (and not in a loading screen, else it retries next update) calls `RunBasicNIS` (12-update delay);
* otherwise immediately: `NISManager->RunBasicNIS(sceneName, (eTransitionType)TransitionType, holdCameraAtEnd=false, skipEnabled=true, overrides=nullptr, NIS_SPECIALFLAG_NONE)` (0x825de678; same arguments at 0x825c627c).

Other entry points: `NISManager::RunBasicNISDefinition` 0x828a8a70 (builds a `NISPlaybackOverrides` from a `tNISPlaybackDefinition`: location override, `useStreamPositionOnReturn` from `GetRestorePosition`, actor ids, `IgnoreTeleportToLastBreadcrumb`), `RunReplayNIS` 0x828a3c70 (replay-controlled scenes), Lua `ModuleNIS`.

### 4.2 Sim side: `NISManager::RunBasicNIS` 0x828a8d70

1. If the online-game flow state is active, hides the popup FE state (`FrontEndState_Popup::HideForNIS`).
2. If flow state `STATE_Loading` is active: `tBasicDelayNISData::StartHold` 0x828a8cf0 copies all arguments into `mStateLoadingDelayNIS` and sets `mActiveHold`; `NISManager::Update` 0x828a67e8 re-issues `RunBasicNIS` (through the vtable, slot 4) on the first update where `STATE_Loading` is no longer active, then clears `mActiveHold`. Nothing else happens.
3. Asserts `!mIsInTrickGuide`. **If `mIsInNIS` is already set the call is silently dropped** (no queue).
4. `TeleportToLastBreadCrumb(sceneName)` 0x828a0db8: unless `mIgnoreBreadcrumbTeleportOnNextNIS` (then it is just cleared), for scene types `SCENE_TYPE_STORY` (18) and `SCENE_TYPE_GOTO` (26) only (type looked up through `NISManager::GetSceneType` 0x828a0c88 = directory `ConvertStringToSceneHash` + `GetSceneType`): if `mTeleportManager->TeleportLocalToLastBreadcrumbRequiresLoadScreen()` (`cTeleportManager` 0x828a9350 -> 0x828a9800 -> `TeleportRequiresLoadScreen(GetSuggestedRespawnPosition())`) is false, calls `TeleportLocalToLastBreadcrumb()` (`cTeleportManager` 0x828a93a8 -> `TeleportToLastBreadcrumb(localPlayerID)` 0x828a9158 -> `BreadCrumbManager::GetSuggestedRespawnPosition` + `TeleportPlayer`). The teleport manager is `Simulation::mWorkspace.mTeleportManager` (passed to `NISManager::Init` from `Simulation::Init` 0x828a6d88).
5. hash = `Directory()->ConvertStringToSceneHash(sceneName)`; if `GetSceneType(hash)` is neither `SCENE_TYPE_FRONTEND` (1) nor `SCENE_TYPE_VIEW_MODE` (22):
   * `FrontEndManager::HudManagerSetNISDisableHUD(true)` 0x82606898 = `HUDManager2::Show(false, eHUDVisibilityFlag_NIS)`;
   * if the sim `VisualTransitionManager` is not already in a transition: `VisualTransitionManager::BeginTransition(TRANSITION_TYPE_NISLOADSCREEN, nullptr, nullptr)` 0x828a38c0 -> sets direction IN and sends `cMsgVisualTransitionBegin{fTransitionType=5}` to the presentation (see 4.6).
6. State: `mIsInPlayback=0`, `mIsInNIS=1`, `mNISSceneNameString=sceneName`, `mExitNISTransition=completionTransitionType`, `mHoldCameraAtEnd`, `mAllowSkip`, `mNISPlaybackOverridesInternal` = `NISPlaybackOverridesInternal(overrides ? *overrides : msDefaultPlaybackOverrides)` (0x82862b00), `mNISSpecialFags`, `mPreloadedSceneHash=0`, `mPreloadedSceneCompleted=0`.
7. `PauseSim()` 0x828a1070: `PauseMan::Push(kPAUSE_STATE_NIS)` (flags 0xFFFFFFF1 => sim paused), `Net::PauseGame()`, and in an online game `cGameAssetManager::LockStreamFocus(true, 7)`; prints "NIS Pause Sim on Tick %u". `mAddedReplayPauseState = 0`.
8. If a preload is still resolving characters (`mResolvingStatus == eResolvingStatus_ForPreload`) -> `eResolvingStatus_ForPreloadWithPendingPlay` (Update finishes the preload resolve, then restarts the resolver for this scene). Otherwise `NISCharacterResolver::Start(sceneName, overrides)` 0x828a5458; if `mIsCharacterResolverFinished` immediately -> `StartPendingNIS()`, else `mResolvingStatus = eResolvingStatus_ForPlay` and `NISManager::Update` polls `NISCharacterResolver::Update` 0x828a49a0 every sim update until finished, then calls `StartPendingNIS` (section 3 documents the resolver).

`StartPendingNIS` 0x828a2d30 builds `cMsgNISEnterBasic(mNISSceneNameString, mHoldCameraAtEnd (this+0x38), copy of mNISPlaybackOverridesInternal, mNISSpecialFags)` (ctor 0x828a0bf0) and sends it to the presentation broadcaster. `mAllowSkip` is *not* sent; skip permission stays on the sim side (4.8).

### 4.3 Presentation side: entering and loading (`SceneManagerImpl`)

`ReceiveNISEnterBasicMsg` 0x8286bfc0:
* `ScenePlayer::SetCameraHoldAtEnd(fCameraHoldAtEnd)`, `SetPlaybackOverrides(fPlaybackOverrides)` (0x82861d90), `SetSpecialFlags(fSpecialFlags)` (0x82861d98);
* sceneHash = `SceneChooser::VerifyScene(name)` 0x82863750 = `ConvertStringToSceneHash(name)`, or the hash of "TestFlythrough1" if `IsSceneHashValid` fails;
* `StartNIS(sceneHash)` 0x8286b8b8.

`StartNIS` 0x8286b8b8: `Directory->DumpScene(hash)` (debug); asserts no active `mNISHandle`; `mIsInOnlineGame` = flow state OnlineGame; `mNISNeedsLoadingScreen = 0`; if `mNISLoadState == 0` and the ICE editor is off: `mDoKillNIS = 0`, sends `cMsgNISFEPushState{fLoadScreenRequired = 0}` to the sim-thread broadcaster (`Sk8AppMan::GetSimThreadBroadcaster`), `mNISLoadState = 1`, `mNISLoadTimer = 0`, if `IsSceneNonVisIndicator(hash)` -> `VIPresentation::SetRenderTypes(0)` (hide visual indicators), `TheICEWorldManager->Reset()`, global `gNISSceneHash = hash` (0x83058834).

FE reaction (`FrontEndManager::ReceiveNISFEPushStateMsg` 0x82633f08): if FE state 2 (`FrontEndState_NIS`) is not on the stack and no FE NIS is active: pops state 81 if present and pushes state 2. `FrontEndState_NIS::OnEnter` 0x82658610 calls `HudManagerShow(false)` (HUD hidden for the whole NIS), `FESubtitle::EnableAutoTimeOut(false)`, `SetSubtitleBottom(false)`. (If state 2 is already on the stack and no load screen is required it calls `TransitionManager::EndTransition()` instead.)

`UpdateLoadState(dt)` 0x82875bc0, run every presentation frame from `UpdateNIS` (see 4.5). `mNISLoadTimer += dt` while `mNISLoadState != 0`, and when it crosses 1.5 s a `cMsgNISLoadingOverlay{fShow=1}` is sent (0x828765c4..0x828765e8) -> FE loading tag shown.

| state | waits for | then does |
|---|---|---|
| 1 | sim paused (`PauseMan::IsPaused(kPAUSE_FLAG_SIM)`) or online game | `AudioInterface::NotifyNISLoadStart(sceneType)`; `ScenePlayer::AudioQueueForStart(hash)` 0x82863cb8 (queues NIS speech/post from the camera take's params 136..140); `ScenePlayer::FERequestPreload(hash)` 0x8286c618 (sends `cMsgNISFERequestPreload` with the take's overlay list if cvar `kNISQueuesTheFE` (default 1, 0x82f72090)); computes the scene frame: if `overrides.useLocationValues` -> `mSceneLocation = overrides.locationPosition`, `mSceneOrientation = overrides.locationDirection`, `mSceneNormal = (0,1,0)`; else if `overrides.locationName[0]` -> position = `cLocationManager::GetLocationPosition(name)`, orientation = `M * (0,0,-1)`, normal = `M * (0,1,0)` with M = `GetLocationOrientation(name)` (vmaddfp sequence 0x82875e9c..0x82875ef0); else from the directory record: `mSceneLocation = (mLocationX,Y,Z)`, `mSceneOrientation = (mOrientationX,Y,Z)` (a forward direction vector, e.g. (0,0,-1)), `mSceneNormal = (0,1,0)`. `mSnapSceneToGround = mBitField & 1`; if not set, `PreSnapFillLocationList()` decides whether individual characters need snapping. If anything needs snapping -> `MoveStreamFocus(sceneLocation)`, frozen streamer controller, state 2. Otherwise `SceneLoader::LoadAndCallback(hash, overrides, NISSceneCallbackFromLoad)` 0x82874d40, state 5. |
| 2 | stream focus loaded (`IsStreamFocusLoaded(3, kFocusLoadType_GameLoad)`) or online | if `mSnapSceneToGround`: send `cMsgNISWorldCollisionQuery{fLocation=mSceneLocation}` to the sim, state 3; else `SnapLocationsToGround()` 0x8286e1d0, state 4 |
| 3 | `mGroundLocationRecieved` (set by `ReceiveNISMgrWorldCollisionResponseMsg`) | `PreSnapFillLocationList`, `SnapLocationsToGround`, state 4 |
| 4 | `SceneLocationSnapper::mSnappedLocationsRecieved` | `UpdateLocationListWithSnappedLocations`, `LoadAndCallback(...)`, state 5 |
| 5 | `gNISSceneIsLoaded` (set by `NISSceneCallbackFromLoad` 0x828608e0 when the `SceneLoadRequest` finishes), `ScenePlayer::IsAudioReadyForStart()` 0x82861b38 (speech/post queued, cvar `kNISQueuesTheAudio`), `ScenePlayer::IsFEPreloaded()` 0x82861c20, and the presentation transition is finished or fading OUT | `gNISSceneIsLoaded = 0`; track = `ScenePlayer::SelectTrack(hash)` 0x82863ed0; speed = `ScenePlayer::SelectSpeed(hash)` 0x82861c60; `mNISHandle = ScenePlayer::Create(hash, track, speed, &mSceneLocation, &mSceneOrientation, &mSceneNormal, &mCharacterLocationList)` 0x82878590; state 6 |
| 6 | if record bit 1 (`mBitField & 2`) and not online: stream focus loaded | if the presentation is not in a transition: `PresVisualTransitionManager::EndTransition(StopNISTransitionOutCallback)` (starts the fade from white back to the picture, 4.6); if `ScenePlayer::IsFreezingStreamer()` -> frozen streamer controller; (`mNISNeedsLoadingScreen` path: `cMsgNISLoadFinish` + wait for FE "go" - never taken for a basic NIS because `StartNIS` cleared it); state 7 |
| 7 | (`mIsFEWaitForGo` only with a loading screen) | `mHideAISkatersForPhotoOutro` handling for PHOTO_OUTRO; sends `cMsgNISMgrPlayback` to the sim (`NISManager::mIsInPlayback = 1`); `ScenePlayer::Play(mNISHandle)` (vtable slot 15 -> 0x82872348 -> `Scene::UnPause` 0x82870a18 -> `Scene::BindToGame` 0x82865c18 + `PLAY_STATE_PLAYING`); on error `StopDestroyUnloadNIS` with `NIS_KILL_TYPE_ERROR`; if the load timer had passed 1.5 s sends `cMsgNISLoadingOverlay{fShow=0}`; state 0 |

Loading itself (`SceneLoadRequestManager`): `LoadAndCallback` cancels other pending/completed requests unless this scene is already complete/active, then `AddLoadRequest(hash, cb)` 0x828741c8 (re-uses an existing request; if already complete the callback fires immediately). `SceneLoadRequestManager::Update` 0x828742e0 (called every frame from `UpdateDeltaFrameBegin`) activates one pending request at a time (acquiring the scene anim allocator unless PHOTO_OUTRO), `SceneLoadRequest::Load/Update` 0x82863bc8 / 0x82868f80 run the scene's asset loaders strictly in order (`Load`, poll `IsLoaded`/`Update`, next), then set `ELoadRequestState_Loaded` and call the callback. The loaders are created per scene by `SceneBinDirectory::AddLoaders` (section 1/3).

### 4.4 `ScenePlayer::Create` and `Scene::Init`: what is set up for playback

`ScenePlayer::Create` 0x82878590: `mOverrideTimeMult = speed`; returns 0 unless the load request status is Complete; allocates a `Scene` (0xC0 bytes) with `mSceneHash`, `mCameraTrack = track`, `PLAY_STATE_STOPPED`, all times 0, flags `mIsControllingSimulation/FrontEnd/Streamer/Camera/PostFX/Audio/Subtitler = 1`, `mIsControllingReplay/WorldRender = 0`; registers it in the instance manager (handle = generation<<8 | index+1) and calls `Scene::Init`.

`Scene::Init` 0x82877ac8:
1. `ClearFreezeTimeControl`; props = `GetSceneProperties(hash)`; `mSceneType = props->mType`; for "ViewScene" no FE/streamer control.
2. `mLocation/mOrientation/mNormal` = the three vectors; `mTransform = Scene::ComputeTransform(location, orientation, normal)` 0x828652f0 (section 1/3 gives the math).
3. `mTimeElapsed = mTimeDelta = 0`, **`mTimeLength = props->mLength`** (seconds).
4. `mIsControllingReplay = (props->mIsReplayOn != 0)`; replay-controlled or FRONTEND scenes do not control the streamer. For replay scenes: length = (replayEnd - replayStart) * 0.016666668 (60 Hz replay frames), clamped to >= 1.5 s, and a `SceneReplayTimeController` drives the DataSystem time.
5. `SceneBinDirectory::AddEntities(hash, this, overrides, characterLocations)` 0x82877838 creates the scene characters (section 3); every entity gets `SetSceneTransform(mTransform)`.
6. `SetTimeElapsed(0)` -> every entity `SetTime(0)`.
7. Camera: if controlling camera, `mCameraTrackFound = mNotify->CameraControlConnect(instanceID)` (`User_ICESceneManager::CameraControlConnect` 0x826ccdf8 -> `gICESceneBridge.mIsICESceneActive = 1; ICESceneBridge::ConnectScene(handle)` 0x826cc268 -> `ICEManager::ChooseCameraPlaybackTrack()`; for replay-controlled scenes the chosen take's `pTakeData->Length` is overwritten with the scene length). Then the first camera matrix: `SceneCamera::GetCameraComponentsAtTimeZero(hash, pos, ?, dir, fov)` -> `ComputeTransform(pos, dir, sceneNormal)` (falls back to the current game camera matrix) -> `CameraPresentation::SetICEMatrix` 0x82587130, and (editor off) `mICECameraActive = 1`, `mICEFirstFrame = 1`. From now on `CameraPresentation::GetMatrix` 0x827f6308 returns `mICEMatrix` (priority: challenge camera, ICE camera, NIS-sequence camera, game camera) and `GetFOVY` 0x827f64b0 returns `mICEFOV`.
8. Streamer: `CameraStreamerController` (or frozen + `MoveStreamFocus(camera pos)` if `IsFreezingStreamer`).
9. PostFX: `mPostfxPresentation.mICEPostfxActive = 1`, `RenderMan::m_isNIS = 1`, `mLightRigPresentation.mICELightRigActive = 1` (ICE post-fx/light-rig channels now drive rendering).
10. Audio: `NotifyNISPlaybackStart(sceneType, specialFlags, isControllingReplay, GetDeltaTimeMult() * mTimeLength)`.
11. Subtitles: unless online, `FE::SubTitler(sceneName)` 0x826913b8 is created and `Start`ed (text keyed by the scene name).

`Scene::BindToGame` 0x82865c18 (on Play): if the camera take's `fParameter` is 0 (or no take and time 0): `ICELITEChannelManager::OnTakeStart(TheICEManager.mChannelManager)` and `TheICEWorldManager->Reset()`.

### 4.5 Per-frame update while playing

Every rendered frame (`GameModule::Render`), on the presentation thread:

1. `SceneManagerImpl::UpdateFrameBegin(dT)` 0x82878428: if cvar `kNISUsesTheAudioClock` (default **1**, initializer 0x82f71e58) the delta actually used is `gFrameAudioBeginDelta = AudioCore::GetSystemTime() - gLastAudioTime` (audio mixer clock, seconds, double); replay NIS use it only if `kNISReplayUsesTheAudioClock` (default 0, 0x82f71ea8). The chosen delta is stored in `gFrameBeginDelta` and passed to `UpdateDeltaFrameBegin` 0x828770f0 (only if `mAllowUpdates`, default 1):
   1. `UpdateNIS(dt)` 0x82876608: deferred `SendCompletionMessages` / `SendUnloadDoneMessages`; `UpdateLoadState(dt)`; the "wait for streamer before fade out" path (4.7); if `mDoKillNIS` and not stopped, not in a transition and not loading -> `StopNIS()`; **if not stopped, not loading, ICE editor off, a handle exists, not in a transition and `!ScenePlayer::IsPlaying(handle)` -> `mKillNISType = COMPLETED; StopNIS()`**.
   2. `UpdateSceneCharacterViewMode()` (trickbook view mode, unrelated).
   3. `ScenePlayer::Update(dt)` 0x82872558: `mFreezeCount -= dt` and when it reaches 0 `SetFreezeTimeMult(1)`; then `Scene::Update(dt)` 0x82870bb8 for every instance:
      * only in PLAYING state; `mTimeDelta = GetDeltaTimeMult() * dt` where `GetDeltaTimeMult` 0x82861db8 = `mOverrideTimeMult * mStreamTimeMult * mFreezeTimeMult * mSceneTimeMult`;
      * `Scene::AddTime(mTimeDelta)` 0x82865700: `mTimeElapsed += delta` and every entity `AddTime(delta)` (character clips advance; section 3);
      * `SubTitler::Update((int64)(mTimeDelta * 100000))`;
      * **end test**: if `GetDeltaTimeMult() > 0` and (`!mCameraTrackFound` or `mCameraFinished`): PHOTO_OUTRO -> freeze (`SetFreezeTimeMult(0)`) and `cMsgNISChallengeNotify{2 = ENTER_PHOTOVIEW}`; "ViewScene" -> loops the ICE scene; every other scene -> `Scene::Stop()` 0x82870b38 (`PLAY_STATE_STOPPED`, `UnBindToGame`), so `IsPlaying` becomes false and the next `UpdateNIS` stops the NIS.
   4. `SceneCamera::UpdateFrameBegin(dt)` 0x828645d0 (ICE camera, section 2).
   5. `SceneLoadRequestManager::Update()`, `AnimPoolLoader::Update()`.
2. `Presentation::Update(dT)` 0x827f7540: dispatches the queued sim->presentation NIS messages (so e.g. a `cMsgNISSkip` sent by the sim is acted on in the *next* frame's `UpdateNIS`), updates presentation characters (`PresCharacterManager::Update`), runs `TransitionPresentation::Update(dT)` (fade effects advance with the render `dT`, not the audio clock), post-fx, and publishes the camera matrix for the sim.
3. `SceneManagerImpl::UpdateFrameEnd()` 0x82875a90: `SceneCamera::UpdateFrameEnd(gFrameBeginDelta)` 0x828646f8 (camera matrix/FOV/post-fx written to `CameraPresentation`/`PostfxPresentation`), plus the scene/ICE data reload state machine.

`mCameraFinished` is set only by `ScenePlayer::HandleCameraFinished(handle)` 0x8286c8c0 (vtable slot 19), whose only caller is `ICESceneBridge::HandleCameraFinished` 0x826cbcc8 (ICE scene interface vtable 0x820b3290 slot 3) - i.e. the ICE library reports the end of the camera take (section 2).

The sim does not advance during playback: `NISManager::Update` keeps running (resolver), but `Simulation::SimUpdate1` is skipped while `PauseMan` flag 1 is set (`SimController::Update1` 0x828a72c8).

### 4.6 Visual transitions, HUD, overlays, letterbox, fades

Two-phase "NIS load screen" transition (bloom to white):
* Sim `VisualTransitionManager::BeginTransition(type, cb, data)` 0x828a38c0: `type != NONE` -> direction IN, send `cMsgVisualTransitionBegin{fTransitionType}`; completion arrives as `cMsgVisualTransitionComplete` (`ReceiveVisualTransitionCompleteMsg` 0x8289eda8 clears the in-transition state and calls the callback). `EndTransition` 0x828a3998 sends `cMsgVisualTransitionEnd` with direction OUT.
* Presentation `PresVisualTransitionManager::ReceiveVisualTransitionBeginMsg` 0x827dffb0 -> `BeginTransition(type, SimulationBasedTransitionEndCallback)` 0x827dfde8 (if already in a transition it just answers Complete). Mapping `eTransitionType -> eVisualTransitionType` (disasm 0x827dfe48..0x827dfe6c): 1,2 -> BLACK_FADE (1); 3 -> WHITE_FADE (2); anything else (0, 4 LOADSCREEN, **5 NISLOADSCREEN**) -> **BLOOM_TO_WHITE (4)**. Direction IN = 1. `EndTransition` 0x827dfed8 replays the stored visual type with direction OUT (2).
* `TransitionPresentation::StartTransition` 0x827db108 creates the effect; `TransitionPresentation::Update(dT)` 0x827d0ee8 runs it and on `IsComplete` (`mVisualTransitionTime <= 0`, 0x827d0f98) calls `PresVisualTransitionManager::TransitionDoneCallback` 0x827d0e28 (clears in-transition, calls the user callback). `SimulationBasedTransitionEndCallback` 0x827dffa8 forwards to `SendVisualTransitionCompleteMsg` 0x827dcdf0.
* `BloomToWhiteTransitionEffect` (0x10 bytes: time, duration, fadeUp) is constructed with time = duration = **1.0 s**, `mFadeUp = (direction == IN)`. `Update(dT)` 0x827d55c0: `t -= dT`; value = fadeUp ? (dur - t)/dur : t/dur while t > 0, and 1 (in) / 0 (out) afterwards; the value is written to `mPostfxPresentation.mGameGeneralPostfxData.bloomToWhiteValue` and `mICEGeneralPostfxData.bloomToWhiteValue` and to `AudioSystem::mNISFadeAmount`. `ColorFadeTransitionEffect` (BLACK/WHITE fades) has the same 1.0 s timing and writes `screenFadeColour` of the PostFE post-fx blocks (0x827d5500).

Sequence for a basic NIS:
1. `RunBasicNIS` (sim): HUD NIS-flag off, bloom-to-white **in** (1 s) starts on the presentation.
2. While the screen is white the scene loads (`UpdateLoadState` state 5 explicitly waits until the presentation transition is no longer running, or is fading out).
3. State 6: `PresVisualTransitionManager::EndTransition(StopNISTransitionOutCallback)` - bloom **out** (1 s) reveals the NIS; when done `StopNISTransitionOutCallback` 0x8286bc48 (tail call) -> `SendNISTransitionOutCompleteMessage` 0x8286bb48 -> sim `HudManagerSetNISDisableHUD(false)` (the HUD itself stays hidden by `FrontEndState_NIS::OnEnter`).
4. End of NIS: `StopNIS` 0x8286e438 starts bloom-to-white **in** again with `StopNISTransitionInCallback` and sends `cMsgNISTransitionInBegin` (sim: HUD NIS-flag on) and pauses the scene (`ScenePlayer::Pause`).
5. When white: `StopNISTransitionInCallback` 0x8286e300: unless `overrides.useStreamPositionOnReturn` (4.7) -> `EndTransition(StopNISTransitionOutCallback)` (bloom **out**, 1 s, back to gameplay picture) and immediately `StopDestroyUnloadNIS()`; at the end of the fade `cMsgNISTransitionOutComplete` again (HUD NIS-flag off).
6. FRONTEND / VIEW_MODE scenes skip all of this (no begin transition in `RunBasicNIS`, `StopNIS` calls `StopDestroyUnloadNIS` directly).

HUD / FE:
* `FrontEndState_NIS::OnEnter` 0x82658610 hides the HUD; `OnLeave` 0x8266fd70 re-enables subtitle auto-timeout; the state is popped by `FrontEndManager::PopNIS` 0x82640cb8 from `EndNIS` (also `DeletePreloadedScreens`).
* Overlays: the ICE take's overlay channel (`ICEUserChannelOverlays::UpdateFrameBegin` 0x826d1a28: channel index 7, int parameter 107, values < 87) sends `cMsgNISFEEvent{fID}` whenever the value changes -> `FrontEndManager::ReceiveNISFEEventMsg` 0x82628570 -> `SetState(2, fID)` -> `FrontEndState_NIS::SetState` 0x8266fe00 -> `OnLeaveState/OnEnterState` 0x826675a0 opens the matching Flash screen (`eSk8HUDNISState_*`: SpotFound/SpotOwned corners, ProChallengeIntro, NisOverlay_Left/Right, ProName, ProBioInfoOverlay, magazine Imagesize_* covers, ~40 SponsorLogo_* states, XGames intros, Take_Photo, GVR, SanVanTV, KilledIt, TutorialGesture_Ollie...). These screens were preloaded during load via `cMsgNISFERequestPreload` -> `FrontEndState_NIS::PreloadNISStates` 0x8268a948; when every requested screen is loaded and the sponsor logo is ready, `FrontEndState_NIS::CheckLoadComplete` 0x82648538 calls `NISManager::FEPreloaded` 0x828a3050, which sends `cMsgNISFEPreloaded`; `SceneManagerImpl::ReceiveNISFEPreloadedMsg` -> `ScenePlayer::SetFEPreloaded` clears `mIsFEWaitForPreload`, so `ScenePlayer::IsFEPreloaded` 0x82861c20 (`!kNISQueuesTheFE || !mIsFEWaitForPreload`) lets load state 5 continue. If the camera take lists no overlays, `FERequestPreload` sets `mIsFEWaitForPreload = 0` immediately.
* Letterbox: ICE user channel `ICEUserChannelLetterBox` (type id 8, `mLetterBoxHeight = 0.125`, ctor 0x826cdb90). `UpdateFrameEnd` 0x826c84b8: `h = 0.125 * clamp(GetValueFloat(26) * 0.01, 0, 1)` -> `User_ICECamera::SetLetterBox(h)` 0x826cd198 -> `CameraPresentation::mICELetterBoxHeight` (Presentation+0x13028). The renderer-side consumer of this field was not located (unverified how the bars are drawn; the value is a fraction of the screen per bar, most likely).
* Fade-to-colour channel: `ICEUserChannelFade::UpdateFrameEnd` 0x826c80f8: amount = clamp(param 27 * 0.01, 0, 1), colour from int param 108: 0 black, 1 white (1,1,1), 2 red (1,0,0), 3 green (0,1,0), 4 blue (0,0,1) -> `User_ICECamera::SetFadeColor(r,g,b,a)` 0x826d34f8 -> `mPostfxPresentation.mICEGeneralPostfxData.screenFadeColour`.
* Other ICE user channels drive post-fx while `mICEPostfxActive` (DOF focus/raw focus/lens, tonemap, motion blur, bloom, noise, brightness, fisheye, near/far, contrast, saturate, tint, character effect mask, FX mode, screen wipe, shadows, scanline, radial blur, sun direction, light rig, audio effect/music/speech/post, freeze) - list from the `ICEUserChannel*` constructors 0x826cd540..0x826cf100.
* Freeze channel (`ICEUserChannelFreeze::FireEvent(cmd, frames)` 0x826d1b50, events fired from `ICEUserChannelFreeze::UpdateFrameBegin` 0x826d2498): cmd 1 and 3 = `SetFreezeTimeMult(0)` + `SetFreezeResetCounter(frames * 0.033333335)` (freeze scene time for `frames` 30 Hz frames; `ScenePlayer::Update` counts `mFreezeCount` down with the frame delta and restores `mFreezeTimeMult = 1`); cmd 2 = freeze indefinitely; cmd 4 = freeze + `cMsgNISFEEvent{86}`; cmd 5 = if the local skater is inside the view frustum: freeze, ICE bloom pulse (`s_fBloom`, `s_fBloomUpTime`, `s_fBloomBackTime`) and `cMsgNISFEEvent{85}` (the "Killed It" moment); cmd 6 = `cMsgNISChallengeNotify{3 = NIS_FE_NOTIFY_PLAYING_KILLABLE}` to the challenge system; cmd 7 = `ICEScene::SetTime(frames * 0.033333335)` (jump); cmd 8 = `PlayNextSceneCharacterViewAnim`. This is why `GetDeltaTimeMult` contains `mFreezeTimeMult`; `User_ICECamera::SetSimTimeMultiplier` 0x826cd140 drives `mSceneTimeMult`. Note the **1/30 s frame unit** used by the ICE channel data (`0.033333335`).

Subtitles: `FE::SubTitler` (created in `Scene::Init`, `Start` 0x826a7508 -> `Load` 0x826a7228) loads `data\fe\subtitles\SubTitles_<Language>\XBOX2_<SceneName>.sub`, falling back to `SubTitles_English\XBOX2_<SceneName>.sub`, `SubTitles_<Language>\<SceneName>.sub`, `SubTitles_English\<SceneName>.sub` (`SubtitleManager::GetSubtitleData` 0x82699c10). The shipped files live in the nested archive `data\fe\subtitles\subtitles.big` inside `miscboot.big` (288 entries; only the last naming form exists, e.g. `SubTitles_English\02_Pros_Arrival.sub`, 296 bytes). Format = array of 8-byte `SubtitleInfo_t {u16 startTime; u16 pad; u32 stringHash}` stored **little-endian** (byte-swapped in place on first load), terminated by `startTime == 0xFFFF`; startTime is in 0.1 s. `SubTitler::Update(dt)` 0x8269fbd0 is called from `Scene::Update` with `(int)(mTimeDelta * 100000)` (1e-5 s units): if `10000 * data[k].startTime <= mTimeElapsed` it sends `cMsgSetSubtitle{fHalID = stringHash, or 0 when stringHash == ICELITE::StringHash("NULL") = 0x001A20BA}` (kind 0x61363201) to the FE and advances `k` by one (at most one key per frame), then adds dt. Example `02_Pros_Arrival.sub`: (2.6 s, 0x5B1A2C7F), (5.3 s, NULL), (5.4 s, 0x5B1A2C80), ... (43.5 s, NULL). `ICELITE::StringHash` 0x8297e018 is `h = 0xFFFFFFFF; for c in s: h = h*33 + c` (it also reproduces the scene hashes in scene_000.bin, e.g. "02_Pros_Arrival" -> 0xA5FED0F4).

Audio: `AudioQueueForStart` reads take params 139/140 (post mode/id, mode 1 or 2 -> `QueueNISPost`) and 136/137/138 (speech mode/id/character -> `QueueNISSpeech`); loading waits for `IsSpeechQueued`/`IsPostQueued`. `NotifyNISLoadStart`, `NotifyNISPlaybackStart`, `NotifyNISPlaybackFinish` (in `Scene::Purge`), `NotifyNISUnloadFinish` (in `StopDestroyUnloadNIS`) bracket the NIS.

### 4.7 Stopping, unloading and completion

`StopNIS` 0x8286e438 (editor off, handle present): `mNISStopped = 1`; FRONTEND/VIEW_MODE -> `StopDestroyUnloadNIS` immediately; otherwise bloom-in + `cMsgNISTransitionInBegin` + `ScenePlayer::Pause(handle)` (4.6 step 4).

`StopNISTransitionInCallback` 0x8286e300: if `overrides.useStreamPositionOnReturn` (+0x1D): set static `mWaitStreamerBeforeTransitionOut`, freeze the streamer and `MoveStreamFocus(overrides.streamPosition)`; `UpdateNIS` then waits for `IsStreamFocusLoaded(3, GameLoad)` before `EndTransition` + `StopDestroyUnloadNIS`. Otherwise `EndTransition(StopNISTransitionOutCallback)` + `StopDestroyUnloadNIS` right away.

`StopDestroyUnloadNIS` 0x8286dfe0: PHOTO_OUTRO -> `CharacterPoseSaver::EndSaving`; `ScenePlayer::Stop(handle)` + `ScenePlayer::Destroy(handle)` 0x82872fc0 (-> `Scene::Purge` 0x8286a908 + instance destroy); `mNISHandle = 0`; `SceneLoader::Unload`; restore visual-indicator render types if non-vis-indicator; `AudioInterface::NotifyNISUnloadFinish()`; `mNISStopped = 0`; `SendCompletionMessages()` 0x8286ba48 = **`cMsgNISMgrComplete` to the sim**.

`Scene::Purge` 0x8286a908: purges and deletes all entities; deletes the subtitler; re-enables world render if it was disabled; `NotifyNISPlaybackFinish`; post-fx/light rig off and `RenderMan::m_isNIS = 0`; `SunLightMan.mManualModeNIS = 0`; streamer back to `GameStreamerController` **unless camera hold**; `CameraControlDisconnect`; **unless `GetCameraHoldAtEnd()`: `mICECameraActive = 0` and `mNISSequenceCameraMatrix = mICEMatrix`**; replay scenes return to real time (`Presentation::EnterRealTime(false)`) unless hold. With hold-camera-at-end the ICE camera keeps rendering the last NIS frame until `NISManager::ReleaseNISCameraHold` 0x828a2ec0 sends `cMsgNISReleaseCameraHold` (`ReceiveNISReleaseCameraHoldMsg` 0x82863340: hold off, `mICECameraActive = 0`, copy matrix).

Sim side completion:
* `NISManager::ReceiveNISMgrCompleteMsg` 0x828a3440: `NISCharacterResolver::ReloadMarquees()` 0x828a2578; if `mIsCharacterReloadingFinished` -> `EndNIS()`, else `mResolvingStatus = eResolvingStatus_Reloading` and `NISManager::Update` calls `EndNIS` once the resolver reports reloading finished.
* `EndNIS` 0x828a3200, in this order:
  1. if the popup FE state was hidden for the NIS, `UnhideForNIS`;
  2. `mIsInNIS = 0`, `mIsInPlayback = 0`;
  3. `FrontEndManager::PopNIS()` (pops FE state 2);
  4. **`cMsgNISChallengeNotify{mKind 0x95C091D3, mSize 0x14, fNotificationID = 1 (NIS_CHALLENGE_NOTIFY_COMPLETED)}`** to `mChallengeMan->EventBroadcaster` (0x828a3244..0x828a32c8);
  5. `cMsgNISComplete{0xC2D14440, size 0x10}` to the same broadcaster;
  6. if `mAddedReplayPauseState`: `PauseMan::Pull(kPAUSE_STATE_REPLAY)`;
  7. `TransitionMan->EndTransition()`; `TransitionMan->BeginTransition(mExitNISTransition, TRANSITION_END_PARAM_ACTIVATEONLY)` (the career passes the "Transition Type" property here);
  8. `ResumeSim()` 0x828a1170: prints "NIS Resume Sim on Tick %u", `PauseMan::Pull(kPAUSE_STATE_NIS)`, `Net::ResumeGame()`, `cGameAssetManager::LockStreamFocus(false, online ? 5 : 7)`.

Consumers of the notify:
* `cChallengePresentationDirector::OnNISChallengeNotify` 0x82e08848: if a presentation group is active and `fNotificationID == 1` -> `PlayNextPresentationEvent(group)` (the challenge intro/outro sequence continues).
* `cSignPostManager::ReceiveNISChallengeNotifyMessage` 0x825d2b60: for every running sign-post state machine, `nextState = currentState` if unset and `cSignPostStateMachine::Execute(eSignPostScriptType_NISChallengeNotify)` (career/progression scripts advance).
* Lua scripts through the message bindings.
* `cMsgNISComplete`: `cHighlightReelManager::OnNisCompleteMsg` 0x82cd7af8.
* `Scene::Update` additionally sends `cMsgNISChallengeNotify{2}` (ENTER_PHOTOVIEW) to the challenge system when a PHOTO_OUTRO camera finishes (0x82870d54..0x82870d84).

### 4.8 Skip and kill

* Input: `FrontEndState_NIS::ReceiveInput` 0x826484a0: keycode 2 -> `NISManager::SkipNIS(false)` (also `FrontEndState_EndChallengeOptions::OnLeave` uses it). The FE keycode-to-button mapping was not traced (keycode 2 is presumably the confirm/A button; unverified).
* `SkipNIS(force)` 0x828a2f10: if `(mAllowSkip || force) && mIsInNIS` send `cMsgNISSkip`.
* `ReceiveNISSkipMsg` 0x828615f0: ICE editor off, `!mDoKillNIS`, and `IsNISSkippable()` 0x828608f0 -> `mKillNISType = INPUT`, `mDoKillNIS = 1`. `IsNISSkippable`: false while loading (`mNISLoadState != 0`) or without a handle; requires `IsSceneSkippable(hash)` (directory record flag, section 1); **not skippable during the first 2.0 s of scene time for types 12 SPOT_INTRO, 15 PRO_CHALLENGE_INTRO and 18 STORY**; never in an online game.
* `UpdateNIS` turns `mDoKillNIS` into `StopNIS()` as soon as no transition is running and loading is done, i.e. a skip goes through exactly the same white-out / unload / `cMsgNISMgrComplete` / `EndNIS` path as natural completion (so the challenge notify is identical).
* `NISManager::KillNIS` 0x828a2f88 (sent when exiting to the main menu, accepting an invite, Lua `KillNIS`) sends `cMsgNISKill` -> `mKillNISType = MESSAGE`, `mDoKillNIS = 1` (no skippable check).

### 4.9 Preloading (not used by the career `RunBasicNIS` path, documented for completeness)

`NISManager::PreloadNISList` 0x828a6998 resolves characters for the first scene and `StartPendingPreload` 0x828a2dc8 sends `cMsgNISPreloadList`; the presentation (`ReceiveNISPreloadListMsg` 0x82876820) queues a load request with `ProximityBasedSceneLoader::SceneLoadRequestCallback` 0x8286c2a0, which reports `cMsgNISPreloadFinish{fSceneHash}`. `GetNISPreloadStatus` 0x828a0d18 returns 0 (none/other scene), 1 (loading), 2 (complete). A later `LoadAndCallback` for the same hash completes immediately.
