# Audio game layer (`Sk8::Audio`)

<a id="lifecycle"></a>
## 1. Singletons and lifecycle — verified

Singletons (map `sInstance` symbols): AudioCore, AudioSystem, AudioAssetManager, MicrophoneMan,
Sk8SpeechSystem, MusicSystem, GlobalEffects, Sk8ContextSysMan. Construction order is in
[README §2.1](README.md#21-boot--sk8sk8appmaninitapp-0x827349c8).

`AudioSystem` (3600 bytes, PDB) derives from `IAudioInterface`, `Nicotine::IDynamicMixer` (+0x04) and
`Contexts::GameInterface` (+0x28). Notable members: 16 `xcore::BigFile*` (+0x34..+0x70),
`mpDynamicMixerFile`, `mCrowdsLoadStatus`/`mVehiclesLoadStatus`, `mContestKey`,
`CrowdPodInfo mpCrowdPodInfos[30]`, `cAudioEventBroadcaster AudioEventBroadcaster` (+0x460),
`pMasterSFXObj` / `pPauseSFXObj` / `pChallengeSFXObj`, `aud_fehud` attribute,
`CSTATEMGR_Base* mStateManagers[13]` (+0x4C8, indexed by `eMAINMAPSTATES`), the eight Csis
event-system blobs, `mpEmitterSystem`, `mpUtility (Csis::C_foley_utility*)`, challenge/score/scene
state, and Hermes message handles. The full layout is in the PDB (`Sk8::Audio::AudioSystem`).

### 1.1 `AudioSystem::Initialize` 0x82509c18
1. `mAwayTime = 1`.
2. Mount BIGs (`xcore::BigFile(path, bUseBigHeader)`), in this order: splicerskateboard (0),
   splicercollision (1), splicerdmo (1), splicerfe (0), skateboard (0), vehicle (0), crowds (0),
   grains (0), pedestrian (0), emitters (1), emitters2 (0), rolling (0), ambience (1),
   dynamicobject (0), moments (0), loadscreen (0).
3. `LoadCsisEventSystem` 0x824edc20: sync-load and `Csis::System::Subscribe` `SK8_AEMS_skateboard.csi`,
   `SK8_AEMS_Foley.csi`, `Sk8_Emitters_Project.csi`, `SK8_AEMS_rolling.csi`, `AEMS_TRAFFIC.csi`,
   `SK8_AEMS_Crowds.csi`, `Sk8_AEMS_MoveableObjects.csi`, `Sk8_moments.csi`, then cache handles
   (`CacheHandles*`, `ClassHandle::Set` for rolling, traffic car/horn/skid, arena start, crowd
   ambience/behaviour, killedit bloom/logo, fade-to-white, ownedit logo).
4. `RegisterSFX`, `RegisterStates`, `RegisterStateManagers` (class lists, §2).
5. `SpeechSystem::Initialize`; `RegisterGetValueFunctions`.
6. Context systems (seed = `mftb`): Sk8CameraMan (cameramanxml.big / cameramancontext.big, 225),
   Sk8Announcer (announcerxml / announcercontext, 200), Sk8MainCast (maincastxml / maincastcontext,
   200); each `CreateContextSystem` then `DoCommand(kPostLoad)`. Then
   `Csis::CacheHandlesCMTrigger_Events/ANTrigger_Events/MCTrigger_Events`.
7. `AudioAssetManager::SyncLoad("emitter_utility.abk", 0, 6, eAssetType_AEMSBank, 0, 1)`.
8. Emitter system: `EnterGamePlay(world AABB)`, `LoadGameFiles`, `EnableEmitterSwap(1)`.
9. `MicrophoneMan` instance; `AddDelayedMessageHandlers`; `mbInitialized = 1`.

### 1.2 `EnterWorld` 0x82504450 = `Initialize` → `LoadInGameData` → `InitializeMixer`

`LoadInGameData` 0x824fdfe8:
1. `SyncLoad("Sk82_FE.bnk", 0, 0, eAssetType_SplicerData, slot 25, 0)`,
   `SyncLoad("sk8_menu.bnk", 0, 1, SplicerData, slot 26, 1)`; `CreateMenuFlangeVoice`.
2. `LoadBigFileHeaders`, `LoadSkateboardData`, `LoadCollisionData`.
3. `SyncLoad("Main_Ambience_Crossfade.abk", 7, 0, AEMSBank, 0, 1)`; `LoadFoleyData`.
4. `MusicSystem->MusicTrigger(ALL_INIT)`; `Sk8SpeechSystem::LoadEarlyResidentData`;
   `InitLoadScreenData`; atomic `mInGameDataLoaded = 1`.

`InitializeMixer` 0x824ed830:
1. `mStateManagers[i] = CSTATEMGR_Base::CreateStateMan(i)` for `eMM_MAIN .. eMM_NISCHARACTER`, then
   `EnterGamePlay()` on each.
2. `IDynamicMixer::CreateMixer({mPrintSelect 0, NumMixStates 13, audio resource allocator})`.
3. `SyncLoad("data\\audio/MixMapSK8.mxb")` → `IDynamicMixer::InitMap`.

### 1.3 `SecondaryLoad` 0x824fe178 (`GameModule::SecondaryLoad02`)
Language = `LanguageManager::GetCurrentLanguage` (French / German / otherwise English).
`SpeechInstance::Load`: instance 0 ← ("maincast", slot 0), ("livingworld", slot 1); instance 1 ←
("cameraman", slot 2), ("announcer", slot 3). Then `LoadResidentData(lang)`, `LoadVehiclesData(1)`,
`LoadMomentsData`.

### 1.4 Other entry points
`GameLoad`: `ResetAdditionalLocationKey`, `SetAdditionalLocationKey(u64)`, `AdditionalDataLoaded`
(location-specific emitter/DLC data, consumed in `Update`). `GameUnload`: `ExitGamePlay`,
`ResetAdditionalLocationKey`. FE: `StateManagerFrontEnd::ChangeFEAudioState` /
`ChangeFEMusicState` send `cMsgFEMusicStateChange` / `cMsgFEAudioStateChange`.
`FrontEndState_StartHub::OnOutroDone`, credits, leaderboard and challenge FE states also touch
AudioSystem (R-G2).

<a id="update"></a>
## 2. Per-frame update — `AudioSystem::Update(dt, b30Hz)` 0x82507b88 — verified

Called from `GameModule::Render` after `Presentation::Update` and `SceneManager::UpdateFrameEnd`,
with `b30Hz = RenderMan.m_PostFX.m_b30HzMode`.

```
if !mbInitialized: return
dt1 = mfPrev_delta_time_1 + dt ; dt2 = mfPrev_delta_time_2 + dt
if kForceSingleAudioUpdate || (kCompensateForSlowFrameRate && dt > 0.02) || b30Hz:
     toggle = 0; A = B = true; prev1 = prev2 = 0
elif toggle != 0: prev1 = dt; prev2 = 0; B = true; toggle = 0
else:             prev2 = dt; prev1 = 0; A = true; toggle = 1
if A:
    AudioAssetManager::Update; AudioEventBroadcaster.Dispatch
    MusicSystem->Update(dt1); Sk8SpeechSystem->Update(dt1); Sk8ContextSysMan->Update
    AudioCore::UpdateSpliceManager(dt1); UpdateLoadScreenLoads; UpdateUnloadLoadScreenData
    emitters: UnloadDLCData if mAdditionalDataKeyToUnload; AsyncLoadDLCData if ToLoad; UpdateDLCAsyncLoad
if DataSys tick._frame == 0x1FFFFFF:            # no running simulation (frontend)
    if A: mStateManagers[MAIN]->UpdateParams(dt1)
    if B: ProcessMixMap(dt2); every manager ProcessUpdate; UpdateFEHudSplices(dt2)
else:
    if A: MicrophoneMan::UpdateCameras(dt1); every manager UpdateParams(dt1)
    if B: ProcessMixMap(dt2); every manager ProcessUpdate
    if A:
        Sk8EmitterSystem->Update(dt1)            (with the MicrophoneMan listener matrix)
        LoadCrowdsData / LoadVehiclesData(0) when WaitingToLoad
        UpdatePresentationData(dt1); UpdateFEHudSplices(dt1)
        if mbPlayArenaOnLoad && crowds Loaded: CSTATEMGR_Crowd::StartCrowds(contest); clear flag
        UpdateHOMBankStatus; mNextHOMTime -= dt1 (if > 0); mb321GoCount countdown → mb321Go = 0
        emitters.mbAllowNewEmitterVolumes = !(P2P/DeathRace/SpotRace) && MusicSystem.mTargetState.mode != 5
```

So "params" run on one frame and "mix + process" on the next; each sees the summed dt of two frames.
`kCompensateForSlowFrameRate` defaults to 1, so any frame slower than 20 ms runs both phases.

## 3. Object model — verified

Framework classes (`audiocore` library):

* `SndBase` (28 bytes): `m_pNextSFX`, `m_bIsEnabled`, `Nicotine::DMixIO* m_pDMixIO`, `m_pStateBase`,
  `m_refCount`, `m_allocatorIndex`, `objectID`. Virtuals include `SetupSFX`, `InitSFX`,
  `GetController(i)` (−1 terminates), `AttachController`, `UpdateParams`, `ProcessUpdate`,
  `UpdateMixerOutputs`, `Detach`, `Destroy`. `SFXObjBase` / `SFXCTL` derive from it.
  `SFXObjBase::GetVolumeFromMixer / GetFrequencyFromMixer / GetDepthFromMixer / GetAzimuthFromMixer /
  GetPitchFromMixer` read the Nicotine outputs.
* `CSTATE_Base` (64 bytes): state list links, `m_pStateMgr`, `m_InstNum`, `m_eStateType`,
  `m_StateInstType`, `m_pAttachment`, `m_pHeadSFXCTL`, `m_pHeadSFXObj`, `m_SFXFlags`, loaded counts,
  `bIsAttached`, times. `Setup(flags)` 0x82a7f2e0 stores the flags, clears the attachment, then
  `CreateSFXObjs` (bit *i* of the flags → `NewSFXObj(i)` → `CSTATEMGR_Base::CreateSFX(instNum, i)`,
  `SetupSFX`, appended to the list) and `CreateSFXCtrls` (for every SFX object and then every
  controller, `GetController(k)` until −1 → `NewSFXCtrl(id)` → `AttachController`; then `SortSFXCtl`).
* `CSTATEMGR_Base` (28 bytes): `m_CurTime`, `m_DeltaTime`, `m_eStateType`, state list, count,
  `bIsInitialized`. `CreateState(instType)` 0x82a7eae0 chooses the most derived registered state class
  whose `StateID` byte 1 equals the manager type and whose low 16 bits equal `instType`, calls
  `createState(HIBYTE(StateID) & 0xF)`, sets `m_pStateMgr / m_InstNum / m_eStateType /
  m_StateInstType`, and appends it.

### 3.1 IDs

`eMAINMAPSTATES`: MAIN 0, PLAYER 1, ENVIRONMENT 2, COLLISION 3, TRAFFICCAR 4, PEDESTRIAN 5, EMITTER 6,
CROWD 7, DYNAMICOBJECT 8, SPEAKER 9, WHOOSH 10, OBJECTINSTANCE 11, NISCHARACTER 12 (13 = max).
Object IDs = `(eMAINMAPSTATES << 16) | (index << 4)`. The index is the flag bit used by `Setup`.

| State | SFX objects (index) | Controllers (index) |
| --- | --- | --- |
| MAIN 0 | Announcer 0, Music 1, Master 2, CameraMan 3, *(4 unused)*, Reverb 5, NIS 6, Pause 7, Speech 8, Bloom 9, VU 10, Challenge 11, HOM 12, Menu 13, Jitter 14 | MusicPos 0 |
| PLAYER 1 | SkateBoard 0, Contacts 1, Wheels 2, Rail 3, Cracks 4, Tricks 5, Clothing 6, PlayerSpeech 7, Treatments 8, SenseOfSpeed 9, OffBoard 10, HandGrabs 11 | PlayerPhysics 0, 3DObjPos/3DPlayerPos 1, 3DBoardPos 2 |
| ENVIRONMENT 2 | Ambience 0 | — |
| COLLISION 3 | Collision 0 | 3DColPos 0 |
| TRAFFICCAR 4 | TrafficEngine 0, TrafficSkids 1, TrafficHorn 2, TrafficWoosh 3 | TrafficCarPhysics 0, 3DTrafficCarPos 1, 3DTrafficEngPos 2, 3DTrafficExhPos 3 |
| PEDESTRIAN 5 | PedestrianSpeech 0, PedestrianSFX 1 | PedestrianPhysics 0, 3DPedestrianPos 1 |
| EMITTER 6 | Emitter 0 | 3DEmitterPos 0 |
| CROWD 7 | Crowd 0 | 3DCrowdPos 0 |
| DYNAMICOBJECT 8 | Dynamic 0, Moving 1 | 3DDynamicObjectPos 0, DynamicObjectPhysics 1 |
| SPEAKER 9 | Speaker 0 | 3DSpeakerPos 0, SpeakerPhysics 1 |
| WHOOSH 10 | Whoosh 0 | 3DWhooshPos 0 |
| OBJECTINSTANCE 11 | ObjectInstance 0 | 3DObjectInstancePos 0 |
| NISCHARACTER 12 | NISCharacter 0 | NISCharacterPhysics 0, 3DNISCharacterPos 1 |

### 3.2 Known state-manager behaviour
* `CSTATEMGR_Main::Initialize` 0x8254fca8: one state, `Setup(0x7FFF)`, attached immediately.
* `CSTATEMGR_Player::EnterGamePlay` 0x8254fde0: `mPreviousLocal = −1`, `mIdCount = 1`, **two** states,
  each `Setup(4095)` (all 12 player SFX objects).
* `CSTATEMGR_Player::UpdateParams` 0x82556c60: walks `Presentation.mAudioPresentation.mInterpolatedSkater`
  (≤ 16, assert). If the local skater index changed, every attached state detaches. Then for each
  skater that is not `ghosted` and is either `local` or `CSTATE_Player::IsInRange(skaterPos)`:
  * if a state is already attached with the same `playerIndex`, refresh its array index and
    local flag;
  * otherwise take `GetFreeState` and `Attach(SkaterAudioStateData)`, which holds the skater ID and
    index, local flag, `characterType = character` bits, and the speech characteristics read from
    Vault `aud_pedestrian` (collection key from `Sk8SpeechSystem::GetSpeechCharacteristicsKey`, high
    word `0x6154DF15`). It also holds the grunt index (`GetPlayerGruntIndex` for the local skater)
    and near/far distance, volume factor and stream.
* Every other manager's create/attach rules: R-G3.

## 4. Nicotine dynamic mixer — architecture verified, data format open

`AudioSystem` implements `Nicotine::IDynamicMixer` (`miCamState`, `NFSMixMaster* mMixMaster`,
`NFSLiveLink*`, `MAP_CREATE_PARAMS`, `SnapshotMixer* mpSnapshot`, `mpRefreshedMapData`).
Callbacks: `ConnectDMixIO(DMixIO*)` 0x824eda60 resolves `(stateID, instance, SFX/SFXCTL id)` to
an object through `mStateManagers[stateID]->GetStateObj(instance)`, then
`GetSFXObject/GetSFXCTLObject(id)`, and binds the `DMixIO` (else `TurnOffMixOutput`).
`GetStateCount(state)` and `DMixPrintf` are the other callbacks. `ProcessMixMap(dt2)` runs in phase B.

The Nicotine library (~100 functions: `NFSMixMaster`, `NFSMixMap`, `NFSMixMapState`, `NFSMixShape`,
`SnapshotMixer`, `DMixIO`) implements master and sub mix channels and mix controls of three kinds:
event (ADSR/ASR/AR envelopes), 3D, and processing. It also provides curve shapes
(`GetCurveOutput`, `GetFloatCurveOutput`, `GetAzimShapeOutput`, cents↔pitch-mult) and snapshots.
The file layout, per-state counts and constant tables are in [nicotine.md](nicotine.md). The processing algorithms are research item R-N1. **This is the layer that turns
controller inputs (speed, impact, distance) into final volume, pitch, filter and azimuth.**

## 5. Presentation data (game → audio) — layout verified

`Sk8::Presentation` holds `PresentationObj<AudioPresentation> mAudioPresentation` (+0x10A10). Built
on the sim side (`AudioPresentation::DoSimSubmit` 0x827fb2f8, `FillSkaterData` 0x827f4c48), restored
and interpolated on the presentation side (`DoVizRestore`, `DoSimRestore`, `BlendSkaterData`
0x827ef828, `GetInterpolatedSkater`). `AudioPresentation` contains `SkaterData* mLocal`,
`VehicleData* mVehicle`, `PedestrianData* mPedestrian`, `GlobalData mGlobalData`,
`DynamicData* mDynamicObject`, `SpacialInstanceData* mSpacialInstance`,
`SkaterData mInterpolatedSkater[9]`, `mGhostData`, `mGhostGlobalData`.

`SkaterData` (512 bytes) fields, in order: skaterPos, skaterLook, skaterVel, boardPos, boardLook,
boardVel (Vector3 each); grindType; turn, boardSpeed, playerSpeed, grindImpact, wheelSlide,
timeInAir, timeToLand, jumpHeight, jumpFraction, deckTilt, playerAngularVel, limbSpeed. Then three
bitfield words:
* `+0x94`: inAir, isPushing, isGenerallyPushing, footBreaking, manualBreaking, isManual, isRailGrind,
  isPerformingTrick, isTrickWithScrape, fl/fr/rl/rrLanded, fl/fr/rl/rrLevel (2 bits each),
  front/backTruckInContact, deckInContact, left/rightFootOnDeck, isWipingOut, isWipeOutOver,
  wheelHardness, isGrabbingRight/Left, isReverting.
* `+0x98`: local, ghosted, offBoard, isRight/LeftFootTouchingGround, terrain (3), isJump,
  isHippyJump, numWheelsOnGround (3), playerIndex (4), character (7), isSkitching, isGrabbing,
  isHandPlant, isPushMongo, canSeePlayer, isCelebrating, wipeoutSeen, respawnSeen.
* `+0x9C..+0xA8`: big-trick / wipeout / respawn "seen" bookkeeping, avoidance, foot-planting,
  boardSlide (2), jumpLanded (3), hippystrength (2), announcer/score/line/wipedout scores,
  air-dismount, holding-board, dismounting.

Then trickType, trickFinishType, the off-board foot surface IDs, wheelPos[4] and
deckLocalXYZAngularSpeed. After those come the foot deck/slide/angular speeds and
`SkeletonCollisionInfoAudio` (8 × contact force, 8 × slide speed, 8 × `tAudioSurfaceType`,
groin/face contact, vehicle/skater/pedestrian impact). The block ends with wheelMaterial[4],
surfacePattern[4], front/back truck and deck materials, deck slide speed and contact force, boardUp,
grindMaterial, absorptionCompression and handPlantSurface.

`GlobalData` (40 bytes): emitterKey, ambienceKey, reverbKey, challengeKey (u64 each), then bits
isPlayerProvidingInput, isInOnlineGame, gameMode (3), localActorIndex (4), isHallOfMeat,
numberOfChasers (2), playerInNoSkateZone, playerInChase, NSZGuardWarned, NSZGuardChasing,
StreetGuardChasing, … (rest in PDB).

The port already produces the physics sources for most of these (see README §4). Mapping each field to
its port producer is research item R-P1.

## 6. Messages (Hermes) handled by AudioSystem

Message types defined in `audio_lib`: PlayMenuSound, StopMenuSound, PlayMissionSpeech, PlayMissionSFX,
TriggerMissionMusic, StopSpeechByChannel, FEMusicStateChange, FEAudioStateChange,
IsCameramanSpeaking, PlayAnnouncerCsisEvent, PlayCameraManCsisEvent, PlayMainCastCsisEvent,
PlayLivingWorldSEDEvent, StartThrowdownChallengeNotification, StartTurnNotification,
OneUpSuccess/FailureNotification, LeaderBoardOverallScreen, ContestPointsEarned, AudioPlayWinSpeech,
TrickComplete, CalloutTriggered, CalloutComplete, ChallengeFailed, AirCompleted, AbandonChallenge,
AudioSequenceComplete, plus MissionKeyChange and RestartChallenge handles. Registration:
`AddMessageHandlers` 0x8250d538 / `AddDelayedMessageHandlers` 0x82508858 (R-G4).

Front-end sounds: `Sk8::FE::GlobalFEPlaySound(u64 id, bool)` 0x826150b0 and `GlobalFEStopSound`
0x826151a0 → `AudioSystem::PlayMenuSound/StopMenuSound` → `PlayMenuSoundImp` 0x8250c738 (Splicer
menu banks, slots 25/26, menu flange voice). APT `FE_SOUND_*` callbacks from the port's UI should
route here (R-UI1).

## 7. Callers outside audio that matter for integration

* Physics/presentation: `AudioPresentation::FillSkaterData`, `DoSimSubmit`.
* Rumble: `Input::RumbleDataMapper::GetAudioSurfaceRumble / GetGrindSurfaceRumble` read AudioSystem
  surface mapping (shared surface tables).
* FE: `FrontEndManager::Render2D`, `FEMoviePlayer::Play`, StartHub/Credits/Leaderboard/Challenge101/
  PhotoChallenge/MoviePlayer states, `TimerHUD::UpdateTime`, `BloomToWhiteTransitionEffect::Update`,
  `RenderMan::UpdateLighting`.
* Scenes/NIS: ScenePlayer / SceneLite (`IsAudioReadyForStart`, `AudioQueueForStart`), ICE user
  channels (AudioMusic / AudioPost / AudioEffect / AudioSpeech `FireEvent`).
* Challenges: `ChallengeAudioManager` (holds `IAudioInterface&`), Lua `ModuleChallengeSpeech` /
  `ModuleChallenge`.
* Flow: `StateLoading::Load/UnloadAdditionalWorldStream`, `StateReplay`, `StateOnlineMovie`.
