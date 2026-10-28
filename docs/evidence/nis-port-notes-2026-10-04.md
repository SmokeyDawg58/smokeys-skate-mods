# NIS port notes (verified in IDA, 2026-10-04 session)

Companion to section_flow.md (playback flow). Everything here was read directly in IDA (db 8ee0dbb0).

## Entry points

- `kEnableSignPostMedia` (cVarBoolRef, section "Challenge_Progression") default **1**: dynamic initializer
  0x82f55b60 (`li r7,1 # defv`). So the port's "media-off" branch is a stand-in, not native behaviour.
- `cSignPostUtil::HandleCaughtMessage` type 2 (0x825de5c4..0x825de6fc):
  - media off: `cMsgNISChallengeNotify{kind 0x95C091D3, size 0x14, fNotificationID 1}` to ChallengeSys+0x38.
  - media on, `cSignPostManager+0x40528` (in loading screen) set: `AddCachedMessage(key)` 0x825d6f78.
  - scene name == one of "04_Arrival_of_Mikey", "18_mag_cover", "17_Thrasher_Cover", "22_SlappyEnder"
    (rw::core::stdc::StringCompare 0x82bc3748 = case-sensitive strcmp): util+0xC = 12 (mNISWaitTimerTicks),
    util+0x10 = message key (mCachedNISMessageKey). NOTE: the shipped message's scene is "18_Mag_Cover",
    so the case-sensitive compare never matches it (native quirk: no delay for 18_Mag_Cover).
  - else `NISManager` vtable +0x10 (RunBasicNIS)(sceneName, (eTransitionType)"Transition Type",
    hold=0, skip=1, overrides=null, flags=0).
  - All 20 shipped type-2 messages have "Transition Type" 0 (NONE) (scratch nis_messages.py).
- `cChallengePresentationDirector::PlayNextPresentationEvent` 0x82e07f38, event NIS:
  `if StringnNoCaseCompare(arg2,"True",4)==0 -> IgnoreTeleportToBreadcrumbOnNextNIS()`; then
  `RunBasicNIS(arg1, TRANSITION_TYPE_NONE, 0, 1, nullptr, NIS_SPECIALFLAG_NONE)`; the group waits (no advance)
  until cMsgNISChallengeNotify(1) (OnNISChallengeNotify 0x82e08848).
  Replay event: `RunReplayNIS(arg2, last-60*arg1, last, 0, 0, 1, 0)`.
- m_1a_01 Lua uses no NIS natives (0 unported); its NIS come from presentation events / progression messages.
  Career data references 452 of the 561 scenes.

## Scene directory (`SceneBinDirectory`, vtable 0x82140f18)

- `FindSceneDataFile` 0x82873f98: highest N of data\scene\scene_* -> "scene/scene_%03d.bin" (scene_000.bin).
- `InitSceneDataBuffer` 0x82871b58: header 16 bytes (magic, version, scene count, char-info count),
  104-byte `SceneBinProperties_t` records, then 180-byte `SceneBinCharacterInfo_t`; hash map name-hash -> record.
- vtable: 0 dtor, 1 GetSceneCount, 2 GetSceneHashFromIndex, 3 ConvertStringToSceneHash (0x8287db48 =
  `ICELITE::StringHash` 0x8297e018: h=0xFFFFFFFF; h=33h+c, case-sensitive), 4 IsSceneHashValid, 5 IsSceneSkippable,
  6 IsSceneControllingReplay, 7 IsSceneNonVisIndicator, 8 IsSceneWithCharacters, 9 IsSceneWithDMOsEnabled,
  10 GetSceneName, 11 GetSceneType, 12 GetSceneTypeName, 13 GetSceneCameraTrack.
- IsSceneSkippable 0x8286eb40: found && mType != 6 (PHOTO_OUTRO).
- IsSceneWithCharacters 0x8286ece0: false if mType in {0,2,5,6,13}; else !mIsReplayOn (absent -> type 0 -> false).
- IsSceneNonVisIndicator 0x8286ec58: absent -> true; else mType != 2 (FLYTHROUGH).
- IsSceneWithDMOsEnabled 0x8286edc0: absent -> false; else (mBitField >> 2) & 1.
- mBitField & 1 = snap scene to ground; & 2 = wait for stream focus in load state 6 (section_flow).

## ICE data

- Contexts (`ICE::RegisterContext` 0x82a4e2d8): index i <-> id 0x80000003+i: 0 FMV, 1 Marketing, 2 Generic
  (+ group "Debug"), 3 NIS_FrontEnd ... 19 NIS_GoTo. Scene type -> context index:
  `User_ICESceneManager::GetSceneContext` 0x826cc968 (NONE/VIEW_MODE 15, FRONTEND 3, FLYTHROUGH 4,
  RACE_START/RACE_END 5, HIGHLIGHT 6, PHOTO_* 7, SKATE_* 8, SPOT_* 9, PRO_INTRO 10, PRO_CHALLENGE_* 11,
  RIGHT_OF_PASSAGE 12, STORY 13, TUTORIAL_* 14, FILM 16, 1UP 17, BEST_TRICK 18, GOTO 19, default 15).
- `ICEContextManager::GetCameraGroup` 0x82a59c68 (context index, scene hash).
- `ICEManager::LoadCameraSet` 0x82a3e600: per group, takes in file order; ElementCounts[35].nIntervals != 0
  -> AssemblyList, else TakeList.
- `ICEManager::SetPlaybackCamera` 0x82a3bd60(group, index): AssemblyList[index] if present, else TakeList[index];
  SetDataPointers; SetParameter(0, force=1, ignore=0); OnTakeSetup; if Length > 0: mPlaybackDataSet=1; with an
  active scene and CONTAINS_SUBTAKE: scene->SetTime(fSubParameter * subLength); SetupAnimElevation.
- `ChooseCameraPlaybackTrack` 0x82a3bf18: track = scene's camera track (`ScenePlayer::SelectTrack` 0x82863ed0:
  overrides AUTO -> directory mCameraTrack; SELECT -> min(max(high,0), takes-1); RANDOM -> Random()%(hi-lo)+lo).
- `ICESceneBridge::ConnectScene` 0x826cc268: if directory IsSceneControllingReplay -> take Length = scene length.

## ICE playback (per presentation frame)

- `SceneCamera::UpdateFrameBegin(dt)` 0x828645d0: ICEManager::Update(dt); then if mICECameraActive and an active
  scene handle: ICECameraMover::UpdateFrameBegin(GetDeltaTimeMult()*dt).
  `SceneCamera::UpdateFrameEnd(dt)` 0x828646f8: ICECameraMover::UpdateFrameEnd(GetDeltaTimeMult()*dt).
- `ICEManager::Update(dT)` 0x82a3b5f0 (with active ICE scene):
  - no assembly (Channels[35].nCurrentInterval == 0xFFFF, take+0x48C): p = elapsed/Length; SetParameter(p,0,0);
    if p < 1 return; else HandleCameraFinished.
  - assembly, CONTAINS_SUBTAKE(145) set: p = (elapsed/subLen - TAKE_START(143)) * (subLen/takeLen)
    + IntervalStart(35, cur); clamp 0..1; if SetParameter(p,0,0) returned changed: mover mSimTime=1,
    mHardCutInterval=-1, mLockToRenderObject=0, OnTakeChange; scene->SetTime(fSubParameter*subLen);
    ResetEffects. return.
  - assembly, no subtake: HandleCameraFinished.
- `ICETake::SetParameter(float p, bool force, bool ignoreAssembly)` 0x82a3b080 (args r5/r6; r4 = float slot):
  clamp; if !force && fParameter == p return 0; fParameter = p; if no assembly intervals or ignore:
  SetParameter(ch, p, force) for all 37, return 0. changed = SetParameter(35, p, force);
  if Values[145]: SetSubTake(Values[144], false); if sub: p = (p - IntervalStart35) * (takeLen/subLen)
  + Values[143]; fSubParameter = clamp(p); SetParameter(ch, fSubParameter, changed||force) for ch != 35;
  else Values[23] = Values[24] = 0. return changed.
  Layout: pTakeData +0, fParameter +4, fSubParameter +8, pSubTakeData +0xC, Values[i] +0x10+4i.
- `ICETake::SetParameter(int ch, float p, bool force)` 0x82a39360: ICEChannel::SetParameter -> changed, start,
  end; if changed||force: key elements' cubic from keys k=keyIndex(cur) (0 for interval 0;
  pKeyIndices[cur-1] if cur+1 < n; else nKeys-2) and slopes GetSlope 0x82a390e8 (end 0/1);
  a=-(2d-(S0+S1)), b=-(2S0-(3d-S1)), c=S0, d=P0; channels without intervals: default constant;
  interval elements = value at cur. Then u=(p-start)/(end-start) (assert > 1e-5), v=((a u+b)u+c)u+d;
  int data types (0..2) rounded: floor, +1 if frac >= 0.5.
- `ICEChannel::SetParameter` 0x82a35f18: q = ICELITEParameter::SetValue(p) 0x82a54488 (clamp 0..1, *65535.0,
  floor, +1 if frac >= .5, u16); keep cur if packed start <= q < packed end (last interval: q <= end), else
  binary search (upper bound on packed params); start/end returned = packed * 1.5259022e-5.
- `SetSubTake` 0x82a39810: TakeList index; if different, SetDataPointers(data, sub=1) 0x82a366f0 (channels
  != 35 read the sub-take; nCurrentInterval not reset).
- `ICECameraMover::UpdateFrameBegin(dT)` 0x82a39c40: UpdateSimTime 0x82a36d70: mSimTime =
  clamp(Values[25] TIME_SCALE * 0.01, 0.01, 1.0) -> User_ICECamera::SetSimTimeMultiplier 0x826cd140 ->
  ScenePlayer::SetSceneTimeMult (so TIME_SCALE 90 plays the scene at 0.9x); then
  ICELITEChannelManager::UpdateFrameBegin(take, dT) (user channels: overlays, freeze...).
- `ICECameraMover::UpdateFrameEnd(dT)` 0x82a3b2e0: UpdateForwardVector 0x82a36fb0, UpdateTransformationMatrix
  0x82a3a410, UpdateHardCuts 0x82a39ce0, UpdateScreenshots, UpdateLens 0x82a36be0, UpdateShake 0x82a39db0,
  UpdateConstraints 0x82a36e38, channel UpdateFrameEnd 0x82a37c20, SetCameraMatrix(m, mSimTime*dT+paused).
- UpdateTransformationMatrix: eye = Values[0..2], look = Values[3..5]; TransformToWorld(eye, SPACE_EYE 89,
  AVATAR_EYE 91, NOROLL 97, LAG 98, BUNGEE 99, fParameter); space 3 -> eye.z += GetAnimElevationFixup;
  same for look with 90/92; lockToRenderObject = (space_eye == 0) && Values[96]; LAG -> UpdateAccelOffset
  subtracted from both; CreateWorldCameraMatrix(eye, look, (u16)(dutch*65536), useSceneUp = space_eye == 3,
  usePlayerUp = lockToRenderObject).
- TransformToWorld 0x82a37448: space 1 world (unchanged); 2 = + (avatar > min ? avatar(avatar-1) position
  : anchor position); 3 = scene matrix (ICE scene position/orientation) * v; 0 = player space (anchor or avatar
  orientation, lag forward cubic, noroll, bungee 0.9*param+0.1).
- CreateWorldCameraMatrix 0x82a371b8: f = normalize(look-eye) (zero -> (1,0,0)); up (0,0,1), or scene
  orientation row 2 (useSceneUp && IsCameraUsingSceneUp), or anchor orientation row 2 (usePlayerUp);
  s = normalize(up x f) (zero -> (1,0,0)); u = f x s; RotateX(dutch); row 3 = eye.
- Space census over all NIS takes (scratch ice/census_space.py): mostly world (1); story uses scene (3) with
  avatars 4..9; photo/own-the-spot look uses 2 (anchor = player). Tutorial/GoTo/1Up/Film/BestTrick: world.

## Scene timing

- `Scene::Update(dt)` 0x82870bb8: only PLAYING; mTimeDelta = GetDeltaTimeMult()*dt; AddTime 0x82865700;
  SubTitler::Update((int64)(mTimeDelta*100000)); if mult > 0 and (!mCameraTrackFound || mCameraFinished):
  PHOTO_OUTRO -> SetFreezeTimeMult(0) + cMsgNISChallengeNotify{2}; "ViewScene" -> loop; else Stop.
- `ScenePlayer::Update(dt)` 0x82872558: if mFreezeCount > 0: -= dt; at <= 0 SetFreezeTimeMult(1), count 0;
  then every scene Update(dt).
- GetDeltaTimeMult = mOverrideTimeMult * mStreamTimeMult * mFreezeTimeMult * mSceneTimeMult (section_flow).
- `SceneManagerImpl::UpdateNIS(dt)` 0x82876608: deferred completion/unload messages; UpdateLoadState; wait-streamer
  path; if mDoKillNIS && !stopped && !inTransition && !loading -> StopNIS; if !stopped && !loading && handle &&
  !inTransition && !IsPlaying -> kill COMPLETED, StopNIS.
- StopNIS 0x8286e438: stopped=1; FRONTEND/VIEW_MODE -> StopDestroyUnloadNIS; else BeginTransition(NISLOADSCREEN,
  StopNISTransitionInCallback) + cMsgNISTransitionInBegin + Pause.
- StopNISTransitionInCallback 0x8286e300: overrides+29 (useStreamPositionOnReturn) -> frozen streamer +
  MoveStreamFocus(overrides+64), wait; else EndTransition(StopNISTransitionOutCallback) + StopDestroyUnloadNIS.
- StopDestroyUnloadNIS 0x8286dfe0: Stop+Destroy, handle 0, unload, VI restore, audio notify, stopped=0,
  SendCompletionMessages (cMsgNISMgrComplete) immediately.

## Transitions (presentation)

- PresVisualTransitionManager::BeginTransition 0x827dfde8 (asserts not in transition): type 1,2 BLACK_FADE,
  3 WHITE_FADE, else BLOOM_TO_WHITE; direction IN; StartTransition. EndTransition 0x827dfed8: same visual
  type, direction OUT; type NONE -> callback immediately.
- TransitionPresentation::StartTransition 0x827db108: BloomToWhite {time 1.0, duration 1.0, fadeUp = dir==IN}.
  Update 0x827d0ee8: effect Update(dT); if IsComplete: delete, call TransitionDoneCallback 0x827d0e28
  (clears inTransition/direction/callback, then user callback).
- BloomToWhiteTransitionEffect::Update 0x827d55c0: t -= dT; t > 0 ? (fadeUp ? (dur-t)/dur : t/dur) :
  (fadeUp ? 1 : 0) -> mGameGeneralPostfxData.bloomToWhiteValue, mICEGeneralPostfxData.bloomToWhiteValue,
  AudioSystem::mNISFadeAmount. (Runs in Presentation::Update after SceneManager UpdateFrameBegin, same frame.)

## Front end

- FrontEndState_NIS (FE state 2): OnEnter 0x82658610: HudManagerShow(false), FESubtitle autotimeout off,
  subtitle bottom off, OnScreenReady(-1) (input accepted at once). gFEStateInfo[2] 0x82fdeda8: mbPause 0,
  mShowHUD 3, mbInputToActionscript 0.
- ReceiveInput 0x826484a0: keycode 2 -> NISManager::SkipNIS(false). Gating: StateManagerFrontEnd::ReceiveInput
  0x82656af8 (top state's IsScreenReady 0x82d661a8).
- IsNISSkippable 0x828608f0: !loading, handle, IsSceneSkippable, not (time < 2.0 and type 12/15/18),
  not online.
- Subtitles: FESubtitle::SetSubtitleText 0x82615ec8 (text formatted only when the profile subtitle option
  (BE module 7 +11 byte 1) or override is on), ClearSubtitle 0x82615e68, RefreshSubtitle 0x82614c80 ->
  AptIntegration::CallFunction(eASCall_RefreshSubtitle).

## Subtitle line and movie (ported 2026-10-04)

- Sender: only SubTitler::Update 0x8269fbd0 posts cMsgSetSubtitle (fHalID; 0 for the NULL hash), to
  FrontEndManager::EventBroadcaster. SubTitler users: Scene::Init/Update (NIS) and FEMoviePlayer (FMV).
  No Lua or attribute script posts it (db.big and the script archives scanned).
- ReceiveSetSubtitleMsg 0x8261ba00 -> FESubtitle::SetSubtitleText 0x82615ec8 on mGenericSubtitles
  (FrontEndManager+0x1EC0). FESubtitle: mbAutoTimeout +0, mCounter +4 (ms), mSubtitleText[1024] +8,
  mbActiveSubtitle +0x408, mSpeechTimeout +0x40C (ms), mbSubtitleBottom +0x410, mbOverrideProfileSetting
  +0x411. FrontEndManager ctor 0x82635f18: autotimeout 0, active 0, counter 0, bottom 1, override 0.
- SetSubtitleText: halID && DoesStringExist (SearchForString != 0) -> counter 0, text "", active 1; if
  option or override: FormatString(text, halID) (no arguments); if autotimeout: timeout =
  kAutoTimeOutSubtitleLetter*strlen + kAutoTimeOutSubtitleMinTime, a leading '[' ... ']' sets the
  timeout by atoi and is stripped; then RefreshSubtitle. Else: active 0, counter 0, text "", Refresh.
- EnableAutoTimeOut 0x82616050: flag = b, mSpeechTimeout = 0, ClearSubtitle if !b. Callers:
  FrontEndState_NIS::OnEnter (false) and OnLeave 0x8266fd70 (true) only. SetSubtitleBottom 0x82606b38
  (OnEnter only): flag + XNotifyPositionUI.
- FrontEndManager::Update 0x82640130 (dT in 1e-5 s, /100 = ms): if autotimeout && active: counter += ms;
  counter >= timeout -> ClearSubtitle (0x826402c4). Not gated by FE state; runs before the event
  Dispatch and the state manager's ExecuteActions. PushState/PopState queue actions (PopState
  0x82d69e50). So a line still up when the NIS state pops goes on the next FE update.
- mbOverrideProfileSetting: written only by FEMoviePlayer::Play (0x82639170/78) and Stop (0x8263941c).
- kNISSubtitlesWithinICE (default 0) only picks Scene+0xA8 in Scene::Init.
- GetMovieSubtitleString 0x82636e50: mGenericSubtitles.mSubtitleText, "#" when empty.
- Movie: main.apt (main.swf, _level0) creates ScreenLayer clips in order gScreenManager,
  gScreenTransitionManager, gLetterBox (hidden), gChyron, gSubtitle (loadMovie
  source/screens/main/subtitle.swf, 0x12d5), gPopups, gFadeBackground; _level1..9 are main3d*,
  mainmisc, autosave (GetUrl). screenmanager.apt RefreshSubtitle -> _global.Subtitle.RefreshSubtitle().
- subtitle.apt: InitAction char 7 0x378 defines class subtitle; root action 0x34c `new subtitle(this)`:
  _global.Subtitle = this, subtitle.mScreen = movie, labelText.autoSize = "center", Clear()
  (EnableSubtitle(false): mcTextSubtitle and mDimmer _visible), Stop. RefreshSubtitle: s =
  GetMovieSubtitleString; s == "#" ? hide : show; labelText.text = s; labelText._y = (3 -
  maxscroll) * 12. Layout: mDimmer (char 3 -> 2 -> shape 1, 1280x96.05, texture
  subtitle_textures\1) at y 577.1, colour 0x7fffffff; mcTextSubtitle at (313.95, 584.45) with
  labelText at (-78, 2): HelveticaPresto 20, centred, multiline, word wrap, bounds -2..810.2 x
  -2..79.05.
- SkateAptString 0x82a70b28: text starting '#' is literal (rest), else a label looked up
  (GetPackedString(StringHash)). Multiline: break at the last space once the line's TextExtent exceeds
  the width, '^' forces a break, at most 24 lines; MaxVisibleLines = line count = maxscroll. A
  multiline field keeps its box with autoSize (x1/y1 = x0/y0 + old size), so
  EnsureStringAllocated 0x82a12220's centre shift is zero. SetProceduralProperty 0x829ea608 _y: ty =
  value, AS flag.
- Shipped English NIS lines: 719; 547 one line, 145 two, 22 three, 1 four; none has a {n:type}
  token; 4 HAL ids have no string (cleared by SetSubtitleText's else branch).

## Scene characters (ported 2026-10-04)

Port: `crates/skate2-core/src/nis/characters.rs` (records, clips, pose), `session/
selected_session_scene_characters.rs` (renderables), `SelectedPresentationRuntime::publish_borrowed`
(current_player), PC `live_skating_view.rs` (draw and visible types).

- Records: `Scene::AddEntities` 0x82877838 reads location +0x94, orientation +0xA0, anim offset +0xAC.
  A record whose category is exactly "dynamic" (case-sensitive) is skipped outside the editor with no
  overrides. Non-dynamic records count toward `kNISCharacterLimit` (32); past it every record is
  skipped.
- Clip DB: `SceneCharacterAnimLoader::Load` 0x8286f5e8 reads `data\` + the record's DB; for a female
  create-a-skater local player its first "_Player" reads "_Female".
- `SceneCharacterAnim::Init` 0x82873088: `NewACS` (MaxTotalSQT 13, mode 0, fetch mode 1, category 1,
  truncation 0.5, codec RAW) with `hasTrajectoryNode` false. `FetchSys::BatchFetch` 0x82d737d0 then
  takes no trajectory delta (it needs `mHasTrajectoryBone` and a part starting at bone 0), so a scene
  clip fetches the trajectory absolute: the root motion in world coordinates, hips near zero. Bind
  pose = the pose named like the hierarchy record. Bones: Trajectory/Vehicle_Root, Hips/Chassis,
  Skateboard_Root, the eight twist bones.
- Timing: `Clip::Eval` 0x82d6ef78 (start/end clamp, `oldTime = time`, loops 0), `AddTime` 0x82d6d238
  (looping wraps), `GetTime` 0x828070c0, `FrameToTime` 0x82805ed0 (`f/((base*fps)*speed)`).
- `EvalCurrAnim` 0x82866cd0: AddSQT(clip, bind); the hips SQT is taken out as a rigid matrix H and
  reset; twist bones only when Skateboard_Root exists (`ComputeExpressions` 0x82866760, the CAS keys
  and `animation_twistbones`; scene `TwistThatBone` 0x82866258 divides the swing by the length once,
  cos before sin); `PoseWithMatMult` with the render entity's inverse bind
  (`GetInvBindPoseBuffer` 0x82866068); `R = Tr·S`, object matrix `H·R` to `SetObjectTransform`
  0x82865fd0 (the entity's `SetLocalToWorldMatrix`), `mHipsPos = t·R`, `pose[traj] = H`. Bit-exact
  against the VMX emulator.
- `SceneCharacter` ctor 0x82875100: `stfs 0` over orientation.y (0x82875208), (0,0,-1) when x == z ==
  0, `ComputeTransform` basis with up (0,1,0) stored transposed (columns), location in row 3.
  `SetSceneTransform` 0x82862158: `mTransform = U × T`.
- `InitFace` 0x82873918 (current_player): the SkaterFace pose named by the schema, else
  "CAC_male"/"CAC_female", else "rig_Tpose", applied with `OverrideWith`. `IsCaC` 0x82866108 is the
  render recipe's category == CreateACharacter (3); `IsMale` 0x828661b8 is
  `CharacterEntity::IsMale` 0x82cfe138 (no customizations reads as male).
- Renderables: `CreateCharacterRenderable` 0x82871150 takes the recipe the loader read
  (`data\content\recipe\<category>\<schema>.recipe`, `EverythingButBlendTargetsClone`). LivingWorld and
  Vehicle recipes get a `ColorizedSkaterPresEntity` (not ported; skipped with a log line), others a
  `SkaterPresEntity` of type `eEntTypNISChar` (0x822), cloth disabled (`kNISCharacterClothOn` off).
  `SceneRenderView::UpdateLODs` 0x82829828 holds every visible 0x822 entity at LOD 0.
- current_player: `ControlCharacterRenderable` 0x82870fb0 borrows the local skater's
  `SkaterPresEntity`: type 0x822, `mIsGhosted` cleared, `i_params[0].x` = 1, the presentation callbacks
  (`GetSkaterBoneBuffer` 0x82865d60, `GetHipsPos` 0x82865d10) answer the scene character.
  `kPAUSE_STATE_NIS` sets `kPAUSE_FLAG_DATASYSRESTORE`, so `GameModule::Render` 0x827320f0 skips
  `RestoreFrame` and `DoSimRestore` does not run; the entity's `Update` (from `SceneRenderView::Cull`)
  takes the local-player `StartJobs` 0x827dbb90 / `EndJobs` path with the scene's bones and the
  `m_MatLtoW` `EvalCurrAnim` set. `GetPosition` 0x827d79c0 (the light probe) is `GetHipsPos`.
  `RemoveCharacterRenderable` 0x8286d478 restores the saved type, index and callbacks; opacity and
  ghosting are not restored (the next `DoSimRestore` recomputes them).
- Technique: `CreateCharacterRenderable` (SkaterPresEntity only) and `ControlCharacterRenderable` call
  `SetTechnique("nis")`, which only sets `m_TechniqueHash[lod]` (+0x148). `SkaterPresEntity::Update`
  0x827e8568, run by `SceneRenderView::Cull` 0x828500a8 (vtable +0x10) for every visible entity before
  it is drawn, sets `mIsGhosted ? ghost : m_CurrentTechnique` (+0x154, "default" from
  `BindConstants` 0x827fb168, which the `SkaterPresEntity` ctor runs at 0x827e5c50). So the `nis`
  technique (`nisPS`: no world shadow, a subsurface term `m_params[7]`/`m_params[6].w`, an
  `i_environment` cube reflection, alpha from the tone map) is never drawn; scene characters draw
  with `default`. Do not port `nisPS`.
- Visible types: `RenderMan::UpdateVisibleTypes` 0x82839be0 (flythrough 0x286F; character NIS 0x282F,
  plus 0x42 online / DMOs / cvar; else all); `DetermineVisibilityFromGameState` 0x82829698 draws an
  entity only when `mask & type == type`. Hidden entities keep updating.
- Entities: `AddEntities` adds only what `InstanceCharacter` returns. A non-current_player record is
  dropped when its `.abin` is missing or `CreateCharacterRenderable` fails (0x82877070..0x828770b8:
  the character is destroyed), e.g. for want of its recipe: `Character::GetRecipeFileName`
  0x82cfdfc0 reads `data\content/recipe/<gCategoryMapping[category]>/<schema>.recipe` from
  `data\content/<category>.big` (vehicle recipes from `livingworld.big`); categories 0 marquee,
  1 livingworld, 2 vehicle, 3 createacharacter, 4 undefined, 5 test. Shipped data has such records
  (`minivan_01` filed as livingworld in the RedaCarRide scenes; its recipe is not under livingworld).
  Later entities move down one index.

## Camera anchors on scene avatars (ported 2026-10-04)

- `ICECameraMover::TransformToWorld` 0x82a37448, space 2 (anchor offset): an avatar up to the AVATAR
  element's minimum (0) adds `mAnchor->GetPosition()`; any other adds
  `TheICESceneManager->GetAvatarPosition(avatar - 1, mLockToRenderObject)` (0x82a375d8..0x82a37664).
  Spaces 1 and 3 take no avatar.
- `User_ICESceneManager::GetAvatarPosition` 0x826d1608 by index: 0 `User_ICECameraAnchor::GetPosition`;
  1 the replay camera matrix's position; 2 a point off the replay camera, then case 3's code runs
  (switch fall-through) and overwrites it when there is a local skater; 3 the local skater's
  `SkaterPresentation::GetHipsPos` 0x827e3ef0 (restored data, frozen while the NIS pauses
  `DataSysMan::RestoreFrame`; not the borrowed entity's scene hips); 4 scene entity 0 with Skate y
  stored as 0.0 (`f31`); from 5 `ICESceneBridge::GetEntityLocation(index - 5)` 0x826cc178. Skate
  (x, y, z) leaves as ICE (z, x, y) in the static `iceVect`.
- `ScenePlayer::GetEntityLocation` 0x8286ce10 -> `Scene::GetEntityLocation` 0x82865490: the entity's
  vtable +0x20 (`SceneCharacter::GetObjectTransform` 0x82865f08, vftable 0x82140ee8), translation row;
  the zero vector for an index past the last entity. `GetObjectTransform` is the render entity's
  `GetLocalToWorldMatrix` (`m_MatLtoW`), which `EvalCurrAnim` sets through `SetObjectTransform`.
- Shipped use (space 2 with an avatar): NIS_Story RedaCarRide_01 (avatar 6: entity 0, Reda's car),
  RedaCarRide_05 (9: entity 3, Reda's car), RedaCarRide_07 (7: entity 1, Haslam, the whole take);
  NIS_Highlight (4), NIS_Extras ViewScene (5, 6). The camera mover runs in
  `SceneCamera::UpdateFrameEnd`, after the scene player's `EvalCurrAnim` of the same frame.
- The port keeps each character's render entity inverse bind at the trajectory bone and
  recomputes the object matrix on every `SetTime`/`AddTime`; a character without a render entity
  (LivingWorld/Vehicle, not ported) leaves its avatar unresolved and the camera falls back.

## ColorizedSkaterPresEntity (LivingWorld ported 2026-10-04; Vehicle 2026-10-05)

- Ctor 0x827e5e20: a `SkaterPresEntity` with `mCSColours[2]` = (0.5, 0.5, 0.5, 1) each; then every
  leaf collection of the living world models class 0x4e740a6670f241a7 (`GetLeafCollections`,
  sorted) whose layout-offset-0 string equals the recipe's schema (`StringCompare`) sets red to
  element 0 of 0xdf76d7d773857edb and blue to element 0 of 0x12026e2eed18cc8d, lane by lane, a
  missing attribute reading `DefaultDataArea` zeros (0x827e5e9c..0x827e6170); the last match wins.
  The same arrays pedestrians index by their colour indices.
- `BindConstants` 0x827dd1a0 adds `i_colorize_red`/`i_colorize_blue` (`ki_colorize_redPed`/`bluePed`
  hold those strings) as instance pixel params to the skater binds (`i_params`, decals, `i_shmult`[9],
  `i_vtx_params`, modulate colours). It binds no `i_material_lod`.
- LivingWorld recipes use the `livingworld_stamp`/`defaultlivingworld` effects (as pedestrians; the
  PC reuses the pedestrian draws with opacity 1, the scene character's skater SH and its own shadow
  bounds). `cScope::DispatchParamsHelper` 0x82e14aa0 uploads a param only when its data pointer is
  set, so the unbound `i_material_lod` (stamp PS c31, the far-colour blend) keeps whatever the
  previous draw left in the register: undefined in the native. The port uses 0 (no blend).
- `CreateCharacterRenderable` 0x82871150 builds a `ColorizedSkaterPresEntity` for both
  `Category_LivingWorld` and `Category_Vehicle` (`SetTechnique("nis")` only on the
  `SkaterPresEntity` branch). `reda_car` (8 records, RedaCarRide) is the only Vehicle recipe; its
  meshes take `vehicle` and `vehicle_glass` only, and no living world models collection is named
  after it, so `mCSColours` keep the ctor's (0.5, 0.5, 0.5, 1).
- `SkaterPresEntity::InitSkinning` 0x827e4b48 remaps with the default bone "Hips", but
  `RemapBones` 0x827f8138 and `FillInvBPBuffer` 0x827f8338 look it up only for a model bone the
  hierarchy lacks (FailAssert when that lookup misses too). The car's clip hierarchy has no Hips
  and every car model bone, so it never asks; the port's `build_skinning_state` now looks the
  default bone up the same way. The scene animation's hips are Chassis (`Hips` else `Chassis`).
- The vehicle effect's constants for the car: `i_colorize_red`/`i_colorize_blue` (the same names
  `cPresVehicle::BindConstants` 0x8280b8f8 binds) from `mCSColours`, and `i_shmult` (the same
  name) and `i_params` from `SkaterPresEntity::BindConstants` 0x827dcf50: its own SH lit at its
  hips and full opacity. Only `cPresVehicle` binds `i_isLightOn` (instance level), so the car's
  register keeps the previous draw's value: undefined; the port uses 0 (lights off).
  `kf_lightSelector` is an effect-level param pointing at the static `cPresVehicle::i_lightSelector`.
- PC: dynamic-object vehicle draws (the traffic shaders) with the entity's `m_MatLtoW`, the
  palette's world rows, culled by `m_localBB` plus the translation (`SceneRenderView::Cull`);
  `ShadowView::Cull` 0x82850c30 measures its distance from `GetPosition`, which for a
  `SkaterPresEntity` (0x827d79c0) is the callbacks' `GetHipsPos`. Appended after the DMO and
  traffic lists each frame; an `eEntTypNISChar` entity, never hidden by the NIS's visible types.
- With the car's render entity, RedaCarRide_01 and _05 (cameras anchored on the car) resolve
  every frame.

## Ground snapping and the stream focus (ported 2026-10-04)

- `UpdateLoadState` 0x82875bc0, state 1 (sim paused or online): the frame from the overrides'
  location name or the scene properties (location w 0, orientation w 0, normal (0, 1, 0, 0));
  `mSnapSceneToGround = mBitField & 1`; a scene that does not snap fills the location list
  (`PreSnapFillLocationList` 0x82874ad8: every record, dynamic ones too, at its location with
  `mLocationNeedsSnapList[i] = mSnapToGround != 0`; returns whether any snaps). Either snap ->
  state 2 with `MoveStreamFocus(mSceneLocation, flags 0xFF)` and a `FrozenStreamerController`;
  else `LoadAndCallback`, state 5.
- State 2 waits for `IsStreamFocusLoaded(3, kFocusLoadType_GameLoad)` (or online). A scene snap
  sends `cMsgNISWorldCollisionQuery{mSceneLocation}` (kind 0x27A08C41, size 32), clears
  `mSnapSceneToGround`, state 3; else asserts the list is not empty, `SnapLocationsToGround`,
  state 4. State 3 on `mGroundLocationRecieved`: clear it, `PreSnapFillLocationList`,
  `SnapLocationsToGround`, state 4. State 4 on the snapper's `mSnappedLocationsRecieved`:
  `UpdateLocationListWithSnappedLocations` 0x82868a00 (the snapped vectors into the flagged slots
  in order), `LoadAndCallback`, state 5. `Create` and `Scene::Init` take `&mCharacterLocationList`;
  `AddEntities` places record i at entry i.
- Simulation: `ReceiveNISMgrWorldCollisionQueryMsg` 0x828a3588 and
  `ReceiveNISMgrLocationListSnapQueryMsg` 0x828a3620 (each location replaced by its ground position)
  both use `GetGroundPositionAndOrientationAtLocation` 0x828a1298: start = location + (0, 1, 0, 0),
  end = start - (0, 500, 0, 0), `LineCollisionTest` without fatness; a miss answers (0, 0, 0, 0)
  and normal (0, 0, -1, 0).
- `ReceiveNISMgrWorldCollisionResponseMsg` 0x82861808 acts only in state 3: `mSceneLocation` and
  `mSceneNormal` copied whole from the message (the location keeps the hit's w; `ComputeTransform`
  writes row 3's w as 1), `mSceneOrientation` with w zeroed, then orientation = N x (O x N)
  (`vmulfp128`, `vnmsubfp`; emulated from the raw words), received = 1.
- `SceneLocationSnapper::SnapLocationsToGround` 0x8286d6d8 (called by
  `SceneManagerImpl::SnapLocationsToGround` 0x8286e1d0 with `ComputeTransform` of the frame and the
  flagged locations in order): clears received, keeps `rw::math::vpu::Inverse` 0x82600f38 of the
  transform, maps each (x, y, z, 1) through the transform (`vmulfp128` row 0 by x, then `vmaddfp`
  rows 1, 2, 3 by y, z, 1; unrolled by four with the same arithmetic) and sends it as (x, y, z, x)
  (`vperm <0,5,0,0>`, `vrlimi` z) in `cMsgNISLocationListSnapQuery` (kind 0xFE94DD4B, size 0x220).
  `ReceiveNISMgrLocationListSnapResponseMsg` 0x8286ae98 copies the message's list, maps each
  (x, y, z, 1) through the inverse the same way, writes xyz over it and sets received with no
  state check.
- `Scene::ComputeTransform` 0x828652f0 and `ICEMatrixFromSkatePointAndNormal` 0x826cc6a8 build Z
  from 0 - orientation (`vsubfp128`), so its zero lanes are +0, and negate their crosses
  (`vnmsubfp`), so an exact cancellation is -0; the port's `vmx_cross` and `compute_transform`
  keep both (checked against the emulated `ComputeTransform`).
- Stream focus: `cGameAssetManager::MoveStreamFocus` 0x824d8250 moves the presentation (flag 1),
  simulation (2) and texture (4) focuses unless `kDisableStreamFocusMovement`, and rejects NaNs.
  `IsStreamFocusLoaded` 0x824d83d0 asks each flagged focus `cAssetStreamSystem::IsStreamFocusLoaded`
  0x824d7a00: for `kFocusLoadType_GameLoad`, not dirty and `mCount == mLoaded`, so a focus with
  nothing to load is loaded. `Scene::Init` (0x82878124..0x828781e0), with `kNISFreezesStreamer`,
  moves every focus to `CameraPresentation::GetMatrix(true)`'s position and freezes the streamer
  (else a `CameraStreamerController`).
  `Sim_LocalPlayerStreamerController::UpdateStreamer` 0x8277d4d0 (offline, manual camera off) moves
  every focus to `Actor::GetPosition` of the local skater. `Scene::Purge` 0x8286a908 returns
  `CreateGameStreamerController` unless a streamer-controlling scene holds its camera at the end.
  `StopDestroyUnloadNIS` 0x8286dfe0 purges only through `ScenePlayer::Destroy`, so a NIS killed
  while loading keeps state 1's frozen controller.
- State 6 waits for `IsStreamFocusLoaded(3, GameLoad)` when `mBitField & 2` (offline) before it
  ends the transition: 05b_DWay_VertChallenge, five best-trick scenes and
  photo_in_m_2a_01_security_jerks.
- Census: the scene snap bit is set on 01_Welcome_Back, 02_Pros_Arrival and RedaCarRide_05 (story),
  15 race starts, 14 race ends and 10 NONE scenes; records snap in 6 story and 11 race-end scenes
  (344 of 818 records).
- Port: `NisSystem::stream_focus` is the frozen focus (the scene from state 1, then
  `Scene::Init`'s); while the NIS pauses the simulation the session moves the terrain's focus there,
  else to the Actor's `PhysOut` skeleton root as `Sim_LocalPlayerStreamerController` does, and
  reports the terrain's `focus_loaded` (every focus collection registered, none in flight) as
  `IsStreamFocusLoaded`. The renderer streams at the same frozen focus through the snapshot's
  `stream_focus_position`, also during the load. Divergences: the presentation focus's load state
  is the renderer's and is not waited for; after a kill during the load the terrain follows the
  Actor again once the simulation resumes.

## FE NIS overlays (FrontEndState_NIS; ported 2026-10-04, PC movie: gestollie)

Agent research `nis-fe-overlays-2026-10-04.md`, re-verified in IDA before porting:

- Request: load state 1 calls `ScenePlayer::FERequestPreload` 0x8286c618: with `kNISQueuesTheFE`
  (dynamic initializer `li r7, 1`) and `SceneCamera::GetListOfOverlays` 0x82864f10 true, it sends
  `cMsgNISFERequestPreload` (kind 0x77645B8F, size 32) to the front end and sets
  `mIsFEWaitForPreload`, else clears it. `GetListOfOverlays`: `GetAssembly(group, track)` (n-th of
  `AssemblyList`) non-null -> every `TakeList` take, else `GetTake(group, track)` (n-th of
  `TakeList`); `GetOverlaysFromTake` 0x82861dd8 appends each channel 7 interval's
  `(u8)GetValueInt(107, i)` not yet listed while fewer than 15 (0 included). A photo outro appends
  85/86 by `GetTimeAtPhotoShot` (not ported). Load state 5 waits for `IsFEPreloaded` 0x82861c20
  (`!kNISQueuesTheFE || !mIsFEWaitForPreload`).
- Front end: `ReceiveNISFERequestPreloadStateMsg` 0x82634dc8 -> `PreloadNISStates` 0x8268a948
  (`mNumLoaded` 0, `mNumScreensToLoad` = count; sponsor ids of `mIsSponsor` rows except 79, then
  `mbSponsorReady` 0 and a `SetScreenState(SponsorLogo_Setup, 100)` when that screen is mapped;
  `mNumBiosShown` 0; `PreloadNISState` per state; unmapped leftovers `UnloadScreen`ed).
  `PreloadNISState` 0x82689520: no screen or state 81 counts as loaded; KilledIt (70, 72-74) and
  OlliePt1 (80) open templates 0x32 / 0x83 into static ids (`KilledItReady` 0x82658708,
  `TutorialOllieReady` 0x826587e8) unless already open; others `OpenScreen` 0x82684a40 (a mapped
  screen counts as loaded, else mapped to -1 with the `OnLoaded` wrapper -> `MarkAsLoaded`
  0x82676d00). `CheckLoadComplete` 0x82648538: all loaded and `mbSponsorReady` ->
  `NISManager::FEPreloaded` 0x828a3050 (`cMsgNISFEPreloaded` 0xE2DA931F when in a NIS) and
  `mNumScreensToLoad` 0; `ReceiveNISFEPreloadedMsg` 0x82861590 -> `SetFEPreloaded`.
- State table: constructor 0x82673d30 emulated (rows 0, 80, 81 also read by hand: r29 = -1 from
  0x82673d80, r6 = 0x83 from 0x826745b4, the sponsor byte r30 reloaded from a spill of
  `li r30, 0`). Initial `mNISState` 88, `mLastScreenId` -1, `mICEOpened` 0.
- Playback: `SceneCamera::UpdateFrameBegin` 0x828645d0 runs `ICEManager::Update`, then with the ICE
  camera active and a scene `ICECameraMover::UpdateFrameBegin` 0x82a39c40 (`UpdateSimTime`, the
  user channels). `ICEUserChannelOverlays::UpdateFrameBegin` 0x826d1a28: active playing scene,
  take, `GetDeltaTimeMult() > 0`, channel 7's current interval -> `GetValueInt(107, i)` (the
  playback's element data, the sub-take's for channel 7); a value other than
  `mCurrentOverlayState` and below 87 sends `cMsgNISFEEvent` (0x4EB59F71) and is kept. Vtable
  0x820b33c0: `Init`, `OnTakeSetup`, `OnTakeStart` all 0x826c7730 (88); `OnTakeChange` (slot 4,
  the slot of the freeze channel's 0x826d6e78) is the empty 0x825187c0. `ReceiveNISFEEventMsg`
  0x82628570 needs FE state 2 on the stack -> `SetState` 0x8266fe00 (`OnLeaveState(mNISState)`,
  store, `OnEnterState`). `OnEnterState` 0x826675a0 / `OnLeaveState` 0x82667cc8: the jump tables
  and gates as the agent tables them (a screen still at -1 sends nothing; 79 pushes FE state 8;
  leaving always ends at 88).
- Teardown: `EndNIS` -> `PopNIS` 0x82640cb8 with FE state 2 on the stack:
  `DeletePreloadedScreens` 0x82658678, then `OnLeave` 0x8266fd70 (KilledIt only with
  `mICEOpened`, the gesture screen always, `mICEOpened` 0).
- Screen manager (`helper/screenmanager.apt`, AS dumped): `ReadyScreen` ids from `mScreenId++`
  (constructor 0), queued, `onEnterFrame = Update`; `Update` runs `ProcessOpenScreens` only when
  `ProcessDestroyedScreens` removed nothing; `ProcessOpenScreens` counts `mBlockScreenOpen` down,
  else creates one screen and blocks eight frames, and drops `onEnterFrame` with both queues
  empty (holding the count); `SetScreenState` reaches a registered class only; `DestroyScreen`
  queues (its duplicate test reads the slot past the queue); `RemoveScreen` calls `Destroy` and
  `removeMovieClip`; `OnLoaded` -> `Initialise`, the `InitialStandardScreenReady` callback's
  `PlayClip(id, 'intro')` (`gotoAndPlay`; 3D calls only for transition types 4/7), the class's
  `OnLoaded`, `Game.OnScreenLoaded`. NIS screens are template type 0: the screen manager's flat
  layer, depth `mScreenArrayDepth` (500 up).
- Cadence: `FrontEndManager::Update` 0x82640130 gives `AptIntegration::Update` `dT / 100` ms (16 per
  60 Hz update), the overlay movies' frame time, then dispatches its events: one Apt frame per
  front end update. `loadMovie` loads through `AptLoadingIntegration` asynchronously (disc I/O);
  the port completes a movie on the Apt frame after `CreateScreen`.
- Sponsor logos (`sponsorlogo_setup.apt` AS): the constructor registers, then `LoadImages` loads
  `NIS.GetLogosToLoad()` (`mSponsorsToLoad`); each logo's `OnComponentLoaded` counts and the last
  calls `NIS.LogosReady()` (`sMethod_LogosReady` 0x8260db40: `mbSponsorReady`, `CheckLoadComplete`).
  The port readies them a frame after the screen registers. `NIS_AChalmers_ProIntro_S01_0` lists
  {0, 10, 39}: 39 is `SponsorLogo_ipath`.
- gestollie (asset 37, AS translated on PC): InitAction character 31 `0x36f0` defines the class,
  root action `0x36c4` constructs it (`_x -= Screen_EdgeOffset`, centred labels, the backer grown
  22 per label line past two, `stop()`, `RegisterScreen`); `SetState` 1 `olliedown`, 2 `ollieup` on
  `mName`. The English tip is two lines.
- Census (overlay values per context): OwnTheSpot {2, 3, 4, 5, 7, 54, 70}, Photo {70}, Film {70},
  Highlight {7, 8, 70}, ProIntro {10, 39}, ProChallenge {80}, Tutorial {80, 81}, RaceStart {79, 82,
  83}, BestTrick {84} (and 0 everywhere).
- ownthespot (49, asset 38) and spotfound (48, asset 39): APT 1:6:4. `AptLoader::CompleteLoad`
  0x82a1cbe0 resolves every movie through the same layouts and nothing reads the version (the
  XEX's only "Apt Data" bytes are an unreferenced embedded movie at 0x82ff5500), so the parser
  takes 1:6:4 as well. ownthespot: root action 0x6e90 `new ownthespot(this); stop()`, InitAction
  30 0x6ea4; `SetState` 0..7 = intro/outro of BottomRight, TopRight, BottomLeft, TopLeft; intros
  show the root and play FE_SOUND_CHALLENGE_OTS, frame 105 drops the banner to alpha 0 before the
  hold stop, ends hide and stop. spotfound: root action 0x6110, InitActions 27 0x614c (empty
  `iscreenmodule` test) and 28 0x6160; `SetState` writes `Mission.GetTitle()` into both
  `mcSpotName.labelText`s and `ID_EVENT_SPOT_FOUND_CAPS` into both `mcSpotFound` labels (offline:
  `GetIsOnlineMission` 0x826058b0 reads `sGameInfo.isNetworkGame`, `GetIsPartyPlayMission`
  0x82605920 `gameMode == GM_PartyPlay`), auto-sizes `mcOwned` right and `mcTextLeftJustified`
  left, hides `mcPoints`/`mcABDPoints` (absent), moves `_x` by `Screen_EdgeOffset` on an intro and
  plays the corner (same labels); intros show and play FE_SOUND_SPOT_FOUND.
- killedit (50, APT 1:6:4, fingerprint c36cd0da): root action 0x1fb48 `new killedit(this);
  stop()`, InitAction 67 0x1fb5c; `SetState` 1..4 play topleft/topright/bottomleft/bottomright
  intros, each running its flourish and outro by itself (sounds FE_SOUND_KILLEDIT_INTRO,
  _FLOURISH, _OUTRO) to a stop; its `logoAnimation` calls and 5 (kOutro) address no child (the
  movie names no instances). Its two masks (sprite 32: shapes 17, 22..31 over the ornaments at
  depths 2 and 5; sprite 62: shapes 37, 41..61 over depth 3) draw as the native occluders
  (`apt-mask-layers-2026-10-04.md`): a rectangle from vertices 0 and 3 of each mask shape's first
  unit plus the accumulated translation hides the content inside it.
- phototreatment (template 61 `PhotoTreatment`, entry 0x82fdb728, type 0; overlay state 54, the
  OwnTheSpot scenes' snapshot; APT 1:7:4, fingerprint 56133c80): root action 0x82c `new
  phototreatment(this)` stops the root and plays `snapshot_mc` from `widescreen` (`Screen_EdgeOffset`
  0; `standard` otherwise), InitActions 8 0x84c (empty `iscreenmodule` test) and 9 0x884. `snapshot_mc`
  waits at alpha 0; `SetState(1)` plays the root's `snapshot` (frame 1, FE_SOUND_PHOTO_FLYTHROUGH),
  opaque to frame 16, fading over 17..25, stopped transparent on 26; `SetState(0)` does nothing.
- fixedphoto (62 `FixedPhotoTreatment` 0x82fdb738, state 85 `Take_FixedPhoto`; ba88d09d) and nonfixedphoto
  (63 `NonFixedPhotoTreatment` 0x82fdb748, state 86 `Take_NonFixedPhoto`; 6218005f): phototreatment's class
  (fixedphoto's offsets 4 lower), the snapshot sounds FE_SOUND_PHOTO_SNAPPING and FE_SOUND_PHOTO_SKATE1.
  `OnEnterState` gives both 1. They load only through the preload: `SceneCamera::GetListOfOverlays`
  0x82864f10 appends, for a PHOTO_OUTRO (6) scene with fewer than 15 entries, 85 when
  `GetTimeAtPhotoShot(hash, 0)` 0x82864d30 is 0 else 86, and then always reports overlays
  (0x828650f0..0x82865150). `GetTimeAtPhotoShot`: the scene's assembly or take at parameter 0, the
  `index`-th freeze interval whose command is 4 (`PhotoShot`), take length times its start; 0 without
  one (fixed-frame photos shoot with command 5 only: photo_killed_m_1a_05_photo_rail preloads 85,
  he_u2_photo01 86).
- pro_bioinfo_overlay (template 38 `Pro_Bio_Info`, entry 0x82fdb5b8, type 0; overlay states 10/11 =
  left/right card; APT 1:7:4, fingerprint b9a276d2): root action 0x29830 `new pro_bioinfo_overlay(this)`
  (stop, register) then `_visible = false`; InitAction 64 0x2989c. `SetState` 0 introLeft / 1 outroLeft /
  2 introRight / 3 outroRight, the intros first `GetInfo`: `_global.NIS.GetProInfo()` [name, hometown,
  stance] into `mcTextHeader.labelText`, `mcTextBody2.labelText` under `ID_NIS_HOMETOWN`,
  `mcTextBody3.labelText` under `ID_NIS_STANCE`. Intros show the root and call
  `Audio.PlayInterfaceOverlayIn`, outros `...Out`, ends hide and stop (frames 10/31/32/65, 66/87/88/121).
- `NISCmdTranslator::sMethod_GetProInfo` 0x8260d840: no main challenge, or a nonzero
  `challenge_global_data.ChallengeType` (0x02B828CCDB4785E4 on the activity's instance) -> "#Bad Pro",
  "#Brooklyn, NY", "#Goofy"; else the main activity's name (`cStateGraph::GetName` 0x82ce3730,
  `challenges.Name`) picks the pro: fp_u3_tutorial03 Way (7), 04 Haslam (8), 01 Dill (4), 02 Rodriguez (1),
  any other Carroll (5); `NISBackEnd` (`Init` 0x8255c558, 20 entries indexed by `eProChallengePros`)
  gives `ID_CHEAT_PROSKATERnn`, `ID_CHEAT_PRODETAIL_LOCATIONn` and ID_CHEAT_PRODETAIL_GOOFY/REGULAR.
  Shipped data: no fp_u3_tutorial activity exists, every pro mission is ChallengeType 23 or 40 (so the
  placeholders), the strings carry only ID_NIS_STANCE of the card's labels (the others show the lookup
  default), and only NIS_AChalmers_ProIntro_S01_0 raises states 10 and 39 (no Chalmers mission ships).
- gvrreg / gvrgoofy (templates 132 `GVR_Regular` 0x82fdbb98, 133 `GVR_Goofy` 0x82fdbba8, type 0; race
  start states 82/83; APT 1:7:4, e58a8c6e / 9fa95bcf): one class, the constructor moving `_x` by
  `Screen_EdgeOffset` (minus for regular, plus for goofy; zero) and registering without `stop()`, so the
  root runs to frame 3's stop with the badge (character 4 at (852, 92)) at alpha 0. `SetState(1)` plays
  `gvrin` (4..22: scale 1.76 to 1, opaque), -1 `gvrout` (23..42: fade). InitActions 5 0xdb0 / 4 0xd98.
- sanvantv (template 136 `NISSanVanTV` 0x82fdbbd8, type 0; best trick state 84; 45497183): the
  constructor stops the root; `PlayClip('intro')` runs it over frames 1..29 (no placements); `mLogo`
  (character 9 at (1102.4, 622.9)) stops on its empty first frame. `SetState` knows 0 (`mLogo` `show`)
  and -1 (`hide`, 121..129), but `OnEnterState` 0x826675a0 sends 1 for SanVanTV (0x82667cbc, with the
  GVR badges and the photo states), so natively the logo only ever plays its hide.
- `cFontManager::GetFontByAptName` 0x82851790: the mapping whose Flash name matches ignoring case, else the
  first whose file is not `debug` (the default group's first, "Helvetica LT Std"); the card's "TradeBld
  Stroke" is in no group. The PC's Apt text paths resolve fonts this way (`Assets::apt_font`).
- Projection: every NIS overlay template is `eFEScreenType_2D` (type 0; the 16-byte template entries
  name, path, transition, type at 0x82fdb458/0x82fdb468, 0x82fdb658..0x82fdb678, gestollie
  0x82fdbb88), drawn on camera 8's flat stage, as are SignUpHUD 0x82fdb8f8, SignUp_Poster 0x82fdb488
  and Skate101 0x82fdbbc8 and main.swf's subtitle layer. The PC `flatten` had projected them through
  the type 5 focus-left HUD camera; it now draws them flat.
- `Mission.GetTitle()` (`MissionCmdTranslator::sMethod_GetTitle` 0x8260e208): the main challenge's
  `challenges` title (0xE2A50788B6C20C92) through `FormatStringCustom`, "#Missing Instance
  Title..." without one; with no main challenge, `GetChallenge(sGameInfo.challengeKey)` decides
  between ID_FREESKATE_LOBBY and ID_FREESKATE_CHALLENGE_LOBBY. `SkateAptString` 0x82a70b28 shows a
  `#` string as is and otherwise looks the text up (`GetPackedString(hash, true)` 0x8297f2f0:
  the default string on a miss); `FormatStringCustom`'s results carry the `#` format prefix.
- Port: `nis::fe_overlays` (core) runs the state machine and the screen manager queue and hands
  `ScreenCommand`s to the PC through `CareerHud::nis_screens` (with `mission_title`); the PC
  draws gestollie, ownthespot and spotfound in the placement-0 group (FE sounds are collected, not
  played, as elsewhere). Not ported: the other overlay movies (logged per template), FE state 8 for
  79 (logged), the photo outro's 85/86 append, the sponsor screen's `kReloadImages` path (PopNIS
  empties the map between NIS), and the native's screen id count (the port's ids start at 0 for
  NIS screens).

## ICE freeze channel (34; ported 2026-10-04 but commands 5 and 8)

- Channels: `Sk8::ICE::RegisterChannels` 0x8286a1d0 registers lens/post-fx channels, the audio
  ones, Overlays, then Freeze; `RegisterChannel` 0x82a56f08 appends, so `UpdateFrameBegin` runs the
  overlay channel before the freeze channel each frame.
- `ICEUserChannelFreeze::UpdateFrameBegin` 0x826d2498: an active playing scene and a take;
  channel 34's current interval (none: nothing); with `mPreviousInterval` 0xFFFF the current
  interval fires unless `mIsTakeChange` (then cleared, nothing fires); else every interval from
  previous + 1 through the current fires in order (u16; a backward jump fires nothing); the current
  interval is kept. No delta-multiplier gate.
- `FireEvent` 0x826d1b50 (frames * 0.033333335 s): 1 Timeout and 3 Owned freeze
  (`SetFreezeTimeMult(0)`, `SetFreezeResetCounter`) when frames is nonzero; 2 Permanent freezes;
  4 PhotoShot, with `GetDeltaTimeMult() > 0`, the same freeze and `cMsgNISFEEvent{86}`; 5
  PhotoShot_FixedFrame, with time running and the local skater's sphere in `GfxMan->mCameras[5]`'s
  frustum (`TestSphere_VMX` 0x8282c208), the frozen skater flag, the freeze, the bloom pulse
  (base = threshold, -0.6, up 0.15 s, back 0.1 s) and `{85}`; 6 Killable
  `cMsgNISChallengeNotify{3}` (0x95C091D3) outside the editor; 7 Jump `ICEScene::SetTime`; 8
  `PlayNextSceneCharacterViewAnim`. `ScenePlayer::SetFreezeTimeMult` 0x82861da8 and
  `SetFreezeResetCounter` 0x82861db0 store their value.
- Resets: `OnTakeStart` (0x826cb400, `mPreviousInterval` 0xFFFF) from `Scene::BindToGame` 0x82865c18
  only while the camera take's `fParameter` is 0 (no take: the scene's time 0), after
  `ClearFreezeTimeControl`; `OnTakeChange` (0x826d6e78, also `mIsTakeChange`) from
  `ICEManager::Update` (0x82a3b7f0) when `SetParameter` reports an assembly cut. The overlay
  channel's `OnTakeSetup` (`SetPlaybackCamera` 0x82a3be44) and `OnTakeStart` both reset it to 88;
  its `OnTakeChange` is empty.
- Census: OwnTheSpot outros 3/30 (38 takes, e.g. Fp_u1_ownspot_01_outro), Photo 4/10 (447),
  4/0 (1), 5/10 (88), RaceStart 3/30 and 7 jumps, Highlight 3/30, Extras 7 and 8.
- Port: the presentation runs the channel after the overlays in `scene_camera_frame_begin` with
  the scene player's freeze mults. Not ported: command 5 (needs the view frustum test, the pose
  saver's frozen skater instance in `UpdateFrameEnd` 0x826cf1a8 and the bloom animation of
  `PostfxPresentation::Update` 0x827efe58) and command 8; both send `FreezeCommandNotPorted`.

## ICE player space (0; ported 2026-10-04)

- One `ICECameraMover`: `Sk8::ICE::Init` 0x8286a6d8 builds it from `SceneCamera::Init` 0x82874528, which
  the `Presentation` constructor calls (0x827f9808, `Camera()->Init()`). The constructor 0x82a3d508 zeroes
  `mForward` (`Cubic3D`, three `Cubic1D` of 0x2c bytes, `mDuration` 1) and copies the anchor into
  `mBungeePlayerPos`/`mBungeePlayerFwd` (0x82a3d768..0x82a3d7b4); with no local skater yet
  (`SkaterPresentation::GetLocalIndex` 0x827d0090: -1 while `mCount` is 0) that is the origin and
  (1, 0, 0). Nothing resets them later: `OnTakeChange` 0x82a39c10 only clears the hard cut, the lock
  and `mSimTime`.
- `SceneCamera::UpdateFrameEnd` 0x828646f8 (ICE camera active, active scene) -> `ICECameraMover::UpdateFrameEnd`
  0x82a3b2e0 (`dT` = delta mult * dt): `UpdateForwardVector` 0x82a36fb0 first, then
  `UpdateTransformationMatrix`. `UpdateForwardVector` reads `User_ICECameraAnchor::GetVelocity` 0x826c7648,
  always zero: no reflection, blend clamp(((|v| - 8) * 0.07692308), 0, 1) = 0, direction (1, 0, 0); each
  lane's `mValDesired` = fmadds(row0, 1 - 0, 0 * dir) (a -0 arrives as +0), `mState` 2 when it differs
  from `mVal`, then `Cubic1D::Update` 0x82a340c0 by `mSimTime * dT` (fmuls). `UpdateSimTime` 0x82a36d70:
  TIME_SCALE (an int element) * 0.01 clamped to [0.01, 1].
- `Cubic1D::Update`: state 1 eases, 2 restarts (time 0; state 1 unless `mFlags`; c2 = dval, c3 = val,
  c0 = -(fma(delta, 2, -(dval + dvalDesired))), c1 = -(fma(dval, 2, -fma(delta, 3, -dvalDesired)))), 0
  returns. Time += seconds / duration (1 when duration <= 0.001); below 1 val = ((c0 t + c1) t + c2) t + c3
  and dval = (round(c0 t) * 3 + round(c1 * 2)) t + c2, fused; else val/dval take the desired ones, state 0.
  The forward target moves with the skater, so the curve restarts on most frames.
- `UpdateTransformationMatrix` 0x82a3a410 passes `fParameter` (`lfs f30, 4(r5)`) to both `TransformToWorld`
  calls (`GetValueInt` 0x82a36650 never touches f1), so a bungee eye and look ease twice a frame;
  `mLockToRenderObject` = eye space 0 && element 96 (no shipped take: the port refuses it).
- `TransformToWorld` 0x82a37448, space 0 (0x82a376b0..0x82a37a74): AVATAR above its minimum (0) ->
  `GetAvatarPosition`/`GetAvatarOrientation(avatar - 1, lock)` (vtable +0x14/+0x10) through `ICELITE::Copy`
  0x82a33210 (rows 0..2 as given, row 3 = position, w 1); else LAG -> row 0 = `ICE::Normalize` 0x82a55310
  (fused length, `fdivs`, (1, 0, 0) at zero) of the three `mVal`s, position = anchor, row 1 = up x f,
  row 2 = f x row 1 with up = the anchor orientation's row 2 (fmsubs crosses, first product exact);
  else Copy(anchor orientation, anchor position). NOROLL: row 0's z = 0, normalised, row 1 =
  (round(z*0) - y, x - round(z*0), fmsubs(y, 0, round(x*0))) = (0,0,1) x f in the same pattern, row 2 =
  f x row 1. BUNGEE: k = fmadds(fParameter, 0.9, 0.1); pos += round((frame pos - pos) * k) and fwd
  likewise toward row 0 (separate fsubs/fmuls/fadds), fwd normalised in place; row 1 = old row 2 x fwd,
  row 2 = fwd x row 1, position = the bungee position. Result: x = round(fma(y, r1.x, fma(x, r0.x,
  round(z r2.x)))) + p.x, y = fma(z, r2.y, fma(x, r0.y, round(y r1.y))) + p.y, z likewise with r2.z/r1.z.
- `User_ICECameraAnchor::GetOrientation` 0x826cd228: `MakeICEFromSkate` 0x826d71c8 of the skater root (rows 0..2
  = the Skate Z, X, Y axes, each as (z, x, y); position zeroed, w 1); the identity without a local skater.
  `GetPosition` 0x826cfc78: the root's translation as (z, x, y).
- `User_ICESceneManager::GetAvatarOrientation` 0x826cf5f0: index 0 the skater root (position row zeroed,
  w 1), 1/2 `CameraPresentation::GetReplayMatrix`, 3 `GetHipsMatrix`, 4 entity 0 and from 5 entity i - 5
  (`ICESceneBridge::GetEntityOrientation` 0x826cc1f0); then for every index (0x826cf97c..0x826cfab8; IDA
  prints some VMX128 operands 32 too high) forward = (0, 0, 1, 0) through the rows (`vmulfp128`, then
  `vmaddfp`), look = up x (forward x up) with up (0, 1, 0) (VMX crosses), `Math::Matrix44FromLookAtUpEyePos`
  0x829b0938 at the origin (Z = normalize(look - eye), X = normalize(up x Z), Y = Z x X, VMX rsqrt
  normalisation), then `MakeICEFromSkate`: the avatar's heading stood upright. The port fills index 0.
- `SceneCamera::GetCameraComponentsAtTimeZero` 0x828647f0 calls `TransformToWorld(space, avatar 0, no
  NOROLL/LAG/BUNGEE, 0.0)`: a player-space point is anchor-relative there.
- Shipped use (census over all takes): replay_c_1a_05_besttrick_street_01 (look, LAG, BUNGEE),
  highlight_generic takes 1-2 (eye and look on avatar 1, BUNGEE), Replay_Generic_Wide (look, LAG),
  VideoCapture, TestFlythrough1 and one shake take. Takes without space elements (the FE transitions,
  assembly-only takes) carry no camera of their own.
- Port: `ice_camera::{Cubic1D, Mover, avatar_orientation, make_ice_from_skate}`; the NIS manager keeps the
  `Mover` for the session and hands it to `CameraTrack::camera`; the session publishes the whole
  `GetSkaterRoot` (`published_root`). Tests: formats `every_player_space_sample_resolves_against_the_anchor`
  (every shipped player-space sample), core `the_best_trick_replay_looks_at_the_skater_through_lag_and_bungee`.

## Freeze command 5 (PhotoShot_FixedFrame): waits for replay-controlled NIS playback

- Research: `docs/evidence/nis-frozen-skater-2026-10-05.md` (the camera-5 frustum from the previous
  frame's ICE matrix, `TestSphere_VMX` 0x8282c208 with `GetPixelSize`, the hips sphere with
  `GetMinPixelSize` 19, `CharacterPoseSaver`, the frozen `SceneCharacter`).
- Re-verified here: the pose saver records at the end of `SkaterPresentation::UpdateBodies` (virtual
  `SavePoseBuffer`, 0x827dedac), the restore path that replays the skater during a replay NIS; all 64
  PHOTO_OUTRO scenes in `scene_000.bin` set `mIsReplayOn`. The port plays replay-controlled scenes
  without replay frames (the 1.5 s clamp), so the sphere `FireEvent` tests and the pose it freezes
  do not exist yet: command 5 stays a logged gap until replay NIS playback is ported.

## Photo flash (bloom pulse; ported 2026-10-05, its trigger with freeze command 5 pending)

- `PostfxPresentation::Update` 0x827efe58 (from `Presentation::Update` 0x827f7608, after
  `TransitionPresentation::Update`; `GameModule::Render` runs `SceneManager::UpdateFrameBegin`, then
  `Presentation::Update`, then `SceneManager::UpdateFrameEnd`): with t = `bloomAnimCurrentTime` (+0x1DC),
  t < 0 idles; t < up: t += dT, threshold = fma(value - base, t_old / up, base); t < down + up:
  t += dT, threshold = fma(base - value, (t_old - up) / down, value); else t = -1, threshold =
  fma(base - base, 0, base). `SetDefaults` 0x827eff00: base 0, value 0, up/down/time -1.
- `FireEvent` command 5 starts it (0x826d1f44..0x826d1f8c): base = the current threshold, value
  `s_fBloom` -0.6, up `s_fBloomUpTime` 0.15, down `s_fBloomBackTime` 0.1, t 0.
- `ICEUserChannelBloom::UpdateFrameEnd` 0x826c88e0: BLOOM_OVERRIDE (112) sets threshold (37) and
  intensity (36); else the default intensity, and the default threshold only while t < 0, so a pulse
  shows unless the take overrides the bloom.

## ICE post-fx channels (ported 2026-10-05 but DOF)

- Research and re-verification: `docs/evidence/nis-ice-postfx-2026-10-05.md` (its "Port" section).
- Tonemap, FishEye, NearFar, Brightness, Contrast, Saturate, Tint (`ICE::XYZ2HSV`, `ICE::HSV2RGB`),
  Scanline and RadialBlur write the take under their override, else `SetDefaults`' values, each
  frame the take plays (`scene_camera_frame_end`).
- `PostFX::SetParameters` in ICE mode: the near and far grades (`ApplyFX_NEW`, the far one with the
  near saturation) replace the vault matrix and blend by linear depth (fully near below
  `nisFarEnd` 40, fully far beyond `nisFarStart` 150); `m_RadialBlur` turns the bloom blurs into zoom
  blurs, drops the medium level and the lens ghosts; the scanline offset rolls the graded screen
  with wrap; `fisheyeWarpMultiplier` w scales the final warp (`warp * w + (0, 1 - w, 0, 0)`); a
  positive tonemap value replaces the exposure.
- Not ported: DOF (RawFocus in 445 takes: far-only, `GenerateDOFMotionBlurBuffers`), the light rig,
  shadow and sun channels' readers, the stencil character mask (never on in shipped takes).

## Breadcrumb teleport (TeleportToLastBreadCrumb; blocked on IsAreaLoaded)

- `NISManager::TeleportToLastBreadCrumb` 0x828a0db8 (from `RunBasicNIS`, `RunBasicNISDefinition`,
  `RunReplayNIS`): story and go-to scenes, unless ignored, call
  `TeleportLocalToLastBreadcrumb` only when `TeleportLocalToLastBreadcrumbRequiresLoadScreen` is false.
  Both go through the local player's const `GetSuggestedRespawnPosition` (ported 2026-10-05 for the save
  orientation); `cTeleportManager::TeleportPlayer` 0x828a9888 without a load screen sends `cMsgTeleport`
  (onboard = !offboard, not a session-marker teleport) on the sim broadcaster.
- `TeleportRequiresLoadScreen` 0x828a9420: true when `RequiresAdditionalWorldOperation`,
  `startChallengeDuringLoad`, or `cGameAssetManager::IsAreaLoaded` 0x824e0768 fails for the sim focus
  (flags 2, `kMinSimRadiusForNoLoadScreenOnTelelport`) or the presentation focus (flags 1, the Pres
  radius). `cAssetStreamSystem::IsAreaLoaded` 0x824e0340 tests `GetAreaTestPoints` around the point:
  each point's 200 m cell (`GetCollectionId` 0x824d73b0: `c<suffix>_<x>_<z>_high`, centres at +-100)
  that is not loaded fails only if a focus at the point would want it (`FindCollections`). The port
  has the sim collision state in core but the presentation load state only in the PC renderer, so the
  event stays logged.
- Ported 2026-10-09 (`IsAreaLoaded` for both foci, the spawn's breadcrumbs, the teleport as the NIS
  ends): `docs/evidence/new-career-start-position-2026-10-09.md`.

## Open research

- Full ICE element table with cubic/tangent ids (agent -> ice/elements_full.py).
- ICE user channels used by career NIS: overlay (7) -> cMsgNISFEEvent screens, letterbox (8), fade (9),
  freeze (34), audio speech/post (32/33); UpdateLens/UpdateShake/UpdateHardCuts details for NIS takes.
- bloomToWhiteValue use in the post-fx shader; letterbox draw.
- NISManager sim side exact code: RunBasicNIS 0x828a8d70, Update 0x828a67e8, EndNIS 0x828a3200,
  TransitionManager::BeginTransition(NONE, ACTIVATEONLY).
