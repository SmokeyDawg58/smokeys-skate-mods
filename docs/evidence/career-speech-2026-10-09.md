# Career: cutscene and mission speech (2026-10-09)

Native: `sk82_na_m.xex` (Jan-17-2009 debug), read-only IDA. Every address below was read in the
IDB (decompile, and disassembly where Hex-Rays lost control flow, rotates or register halves).
Data facts come from the shipped files under the game root (`data\audio\english\*speech.big`,
`data\audio\context\*.big`, `nis.big`, `post.big`, `fepost.big`) and `skater.vlt`.

## Summary

- A scene's speech, post and music channels (ICE channels 32, 33, 31) queue their samples while
  the scene loads, play them on their interval, and the scene steps by the audio clock, so the
  dialogue stays with the picture.
- Every story cutscene carries its audio, dialogue included, in its post (`99_50xx`); the Goto
  Slappy outros add the cameraman's NIS lines (`40_5008` to `40_5012`).
- `Output::PlayNpcSpeech` now plays the line (`cMsgPlayMissionSpeech`) and the challenge hears
  `cMsgSpeechEnded` when the line ends, or at once when it cannot play.
- Every training-area line has an id below 5000, which the game plays only through the context
  systems (`Sk8ContextSysMan`). They are ported: the cameraman's, announcer's and main cast's
  contexts, their condition scripts, their triggers and their outputs into `spch`.

## 1. Scene speech, posts and music

`ICEUserChannelAudio*` channels on the scene's camera take (`skate2_formats::ice_camera`):

| Channel | Elements | Command 1 | Command 2 |
|---|---|---|---|
| 30 effect | 132 command, 133 id | `PlayNISSfx` | |
| 31 music | 134 command, 135 id | `QueueNISMusic` | `PlayNISMusic` |
| 32 speech | 136 command, 137 id, 138 character | `QueueNISSpeech` | `PlayNISSpeech` |
| 33 post | 139 command, 140 id | `QueueNISPost` | `PlayNISPost` |

- `UpdateFrameBegin` fires each interval crossed since the last frame once; the first interval
  fires unless the take just changed. `OnTakeSetup` 0x826d6c18 resets speech and post (previous
  -1, no take change), `OnTakeStart` 0x826cb400 effect and music, `OnTakeChange` 0x826d6e78 marks
  a take change.
- `IAudioInterface` (vtable 0x82022ac0): +0x50/+0x54 `QueueNISMusic`/`PlayNISMusic` are the empty
  `blr` at 0x825187c0, so the music channel does nothing; +0x58 `QueueNISPost`, +0x5C
  `PlayNISPost`, +0x60 `IsPostQueued`, +0x64 `QueueNISSpeech` 0x824f1080, +0x68 `PlayNISSpeech`
  0x8250c6a0, +0x6C `IsSpeechQueued` 0x824f10e8, +0x74/+0x78 `NotifyNISPlaybackStart`/`Finish`.
- Speech is gated on an offline game that is not party play; posts are not. `PlayNISSpeech` below
  id 5000 is `PlayMissionSpeech(character, id, fromNIS)`.
- `ScenePlayer::AudioQueueForStart` 0x82863cb8 (load state 1) queues the take's speech and post
  at parameter 0; `IsAudioReadyForStart` 0x82861b38 holds load state 5 until both are queued.
  `Scene::Init` calls `NotifyNISPlaybackStart`, `Scene::Purge` `NotifyNISPlaybackFinish`.
- The resident samples: `data\audio\english\nis.big|nisresident.big`,
  `data\audio\post.big|postresident.big`, `data\audio\fepost.big|fepostresident.big` (474 NIS and
  post samples, 8 front-end posts), each `CC_EEEE_*.snr` streaming its `.sns`. Two playing slots;
  posts (character 99) play on the post instance, speech on the voices instance (the characters
  instance while the voices' stream 0 plays). `Sk8SpeechSystem::QueueNISSpeech` 0x824ead68,
  `IsNISSpeechQueued` 0x824eaf20, `PlayNISSpeech` 0x824eafe0, `UpdatePlayingNISSpeech` 0x824eb170,
  `ShouldStopNISSpeech` 0x824fac80.

## 2. The scene clock

`SceneManagerImpl::UpdateFrameBegin` 0x82878428 with `kNISUsesTheAudioClock` (1) reads
`AudioCore::GetSystemTime` 0x82a7dae0 every frame (the first read only records it) and steps the
scene by the audio time since the last frame; a replay NIS steps by the frame time
(`kNISReplayUsesTheAudioClock` 0). `UpdateFrameEnd` uses the same step. Transitions and post
effects keep the frame time. The PC device renders many frames per callback, so the clock the
scene reads is the time handed to the device before its last callback plus the wall time since
(at most that callback's length); the console steps in 256-frame blocks, the port continuously.

## 3. Mission speech

- `AudioSystem::PlayMissionSpeech` (+0x3C) posts `cMsgPlayMissionSpeech`;
  `PlayMissionSpeechMsg` 0x8250d0a8 handles it in the audio update (`-2` rings the cell phone,
  1207 plays as 1211 while `mSketchyLand` counts).
- `PlayMissionSpeech` 0x8250b678: from id 5000, the character's NIS sample queued and held on the
  voices instance (`mMissionSpeechStatus` loading, playing, none in `UpdateMissionSpeech`
  0x8250bb18). Below 5000: the cameraman (or Dyrdek hosting), an announcer, an unknown speaker,
  a living-world character or the main cast, each through its mapping table
  (`cameraManTriggerMapping` 0x8201cd58, 215 rows; `maincastMapping` 0x8201c728, 111;
  `contestMapping` 0x8201c190, 46; `livingworldMapping` 0x8201c478, 61) to a speech event and a
  context system trigger.
- The line is watched (at most ten, newest first). `EventFinished` 0x825077f0 answers the
  watched line whose character has the finished event or id with `cMsgSpeechEnded`;
  `EventNotPlayedImpl` 0x8250a3d8 answers one that cannot play through `SendOutGoingMessages`
  0x82507680 (one message per update). A mapped line whose trigger picks no context stays
  watched; a challenge presentation speech event waiting on it ends on its 10 s watchdog.

## 4. The training lines need the context systems

The training missions' speech (challenge attributes `[character, id]`, `ChallengeSpeech` calls):

| Line | Character, id | Where | Below 5000 path |
|---|---|---|---|
| `AudioPhotoPickAShot` | 40, 920 | `m_1a_05`, `m_1a_06` | `T_920_pickphot` |
| `AudioPlayerBailed` | 40, 104 | same | `T_104_slam_cam` |
| `AudioBeginReplay` | 40, 712 | same | `T_712_replay` |
| `AudioReinforcePlayerOnObjectiveOnFail` | 38, 1003 | same | `T_1003_almost` |
| `AudioPlayerCompleteAnObjective` | 38, 1008 | same | not mapped |
| `AudioPlayerSucceedChallenge` | 38, 1011 | same | `T_1011_succ_PC` |
| `AudioPlayerFailedChallenge` | 38, 1015 | same | `T_1015_failure` |
| `PlayProgressionReminder(4004)` | 40, 4004 | `Trigger_m_1a_05_photo_rail` | `T_4004_Rmd_Atiba` |
| "SponsorMeStart" | 40, 2727 | `Trigger_m_1a_06_photo_puzzle` | `T_2727_SpnsrMeRmd` |

All are below 5000. The game reaches the cameraman, announcer and main-cast projects only through
`Csis::gT_*` trigger functions, whose clients are the context systems; the projects' events are
added only by the systems' output functions (`O_*`, `iSPCH_CsisCb`).

## 5. The context systems

### Loading

`AudioSystem::Initialize` 0x82509c18 creates "Sk8CameraMan" (sizes 225), "Sk8Announcer" (200)
and "Sk8MainCast" (200), all seeded with the same time base word, each followed by `kPostLoad`.
`Sk8ContextSysMan::LoadFiles` 0x824f9db0: `<group>context.big` entry 0 is the scripts
(`script.css`), 1 the trigger functions' `.csi`, 2 their `.evt` (`AddCsisEvents` 0x82ab3dd0
subscribes `CsisCallback` to every row); every `<group>xml.big` entry is a context file
(`AddData` 0x82ab6808 types it with `ContextFileTyper`; `AddContextXml` 0x82ab63c0).

| System | Contexts | Passive | Triggers subscribed |
|---|---|---|---|
| Sk8CameraMan | 217 | 2 (`Context835` absence, `Context837` return) | 215 |
| Sk8Announcer | 47 | 2 (`Context414`, `Context465Gvr`) | 47 |
| Sk8MainCast | 98 | 1 (`Context1014` bored) | 112 |

No file is a context group and no context has a follow event; nothing sends `kDoFollow`.
`AddContext` 0x82ab6330 replaces a context of the same id (`RemoveContextById` 0x82ab54f8 skips
the one after each removal) and inserts by descending priority. `kPostLoad` builds the id list,
the trigger table (each trigger's contexts in priority order) and the passive list.

The XML reader is `rw::core::xml::StateParser` (`ParseElement` 0x82bb69b8: a self-closing element
calls `StartElement` then `EndElement`; closing the outer element ends the document) and
`ContextBuilderXMLCallback` (`StartElement` 0x82ab72d0, `EndElement` 0x82ab1068).
`ParseParamRef` 0x82ab6f88 reads the trigger parameter `sourceIndex` names, else a local 0; a
`ParamGetter` (`ParseParamGetter` 0x82ab70f0) is a script whose return value is the parameter. Numbers go through
`ConvertAToI` 0x82bc3ce0; the Csis CRCs keep their low 16 bits.

### Triggers

`OnEvent` 0x82ab3b80: each context of the trigger copies the call's parameters into its trigger
parameters and joins the add list stamped `GetTime`; the current triggered list drops its expired
entries and merges with the add list (`Merge` 0x82ab3820: by descending priority, the add list
first on a tie). The port calls a group's trigger only when its `.evt` subscribed it. All 356
triggered contexts of the mapped triggers declare exactly the parameter words the port passes.

### Update

`AudioSystem::Update` 0x82507b88 runs `Sk8ContextSysMan::Update` (0x82507d6c) in group A right
after `Sk8SpeechSystem::Update`. `ContextSystemManager::Update` 0x82aaf8b8 walks its hash map: the
FNV-1 name hashes in five buckets give "Sk8CameraMan", "Sk8MainCast", "Sk8Announcer".

`ContextSystemImpl::Update` 0x82ab7dd8:

1. `mSpeakerPriority` decays by `elapsed ms * 0.001 * 1000` since `mLastTick` (only while above
   0); outside interrupts-only mode `mInterruptPriority` is 0; the threshold is the larger.
2. The evaluation list merges the triggered list and the passive list (passive first on a tie).
3. `Choose` 0x82ab55b8 walks it from the last stop down to the first entry below the best
   priority found (at least the threshold): in interrupts-only mode only interrupting contexts,
   triggered ones only within their expiry; a context whose conditions hold resets the chosen
   list on a new priority or joins it (at most 20).
4. One chosen context at a time is picked with `((EACrandom() >> 16) * count) >> 16`, takes both
   priorities and is emitted; a failed emit tries the next, then `Choose` continues.
5. The context that spoke leaves the triggered list with every other context of its trigger
   (`RemoveContext` 0x82ab36e8), counts a use and stamps its time; expired entries go.

`Context::Evaluate` 0x82ab4fb8 runs the conditions in order as the current context
(`mCurrentContextID`) and moves the first that fails to the front. A triggered context whose
conditions fail stays for its expiry (1000 ms for most) and is evaluated again each update.

`ContextRand` (`InitRandom` 0x82ab3ab8, `EACrandom` 0x82ab15e8) is one global state for the three
systems. `Sk8SpeechSystem::UpdateContextSystems` 0x824fb928 sends `kInterruptsOn` when the
announcer's or cameraman's voice starts actually playing (main cast: both character streams,
`UpdateContextSystem` 0x824fb7b0) and `kInterruptsOff` when it stops.

### Scripts

`GI::GIShell`: `BindHeaderData` 0x82ab1988 (the `CSS_` chunk; script ids to indices),
`BindScriptData` 0x82ab1b10, `ExecuteScript` 0x82ab46e0, `ExecuteLine` 0x82ab4080 (SET 5, IF 10,
ELSEIF 11, ELSE 12, RETURN 40; a line of module 5 stops the script, other non-zero modules are
skipped), `GetValue` 0x82ab1c48 (globals 5, arguments 10, locals 15, queries 25: 502 last time,
503 times used, 504 time since, the id's high half in the button value, `0xFEEDBABE` the current
context; group queries 512-514 answer 0), `SetValue` 0x82ab1d40, `ComputeSetFunc` 0x82ab1de8
(variable `func - 1000`, parameters `func - 2000`), `ComputeOperation` 0x82ab1ed0,
`ComputeCompare` 0x82ab1ff0, `ComputeCommand` 0x82ab3488. Arguments and locals live in the loaded
file and keep their values. RETURN moves to `mNumLines`, which nothing in the library writes; the
port takes 0, so no later line runs.

### Game interface

`AudioSystem::GetValue` 0x824f12f0 calls getter `n - 1` of `mFuncPointers`
(`RegisterGetValueFunctions` 0x824ece08): 1 `GetRandomBool`, 2 `GetRandomValue`
(`rw::math::Random`), 3 challenge type, 4 game mode, 5 online, 6 `Presentation::IsInReplay`,
7 `mbIsPlayerProvidingInput`, 8 in a challenge, 9 a getter folded to `return 1`
(`ICESceneBridge::IsCameraUsingSceneUp` 0x826c6d80), 10 `mAwayTime`, 11 `!mIsMale`,
12 `mFEMusicState`, 13 the scene type, 14 `mbIsPaused`, 15 `mTimeSinceUnPaused`,
16 `mSk8HUDState`, 17 `mbIsGVR`, 18 the challenge host (100 without), 19/20 seconds and minutes
remaining (only the constructor writes them: 0), 21 `mbIsDannyWayJam`, 22 `mbIsMegaChallenge`.
`GetTime` 0x82509ba8 is the audio core time in ms.

The fields: `UpdatePresentationData` 0x82504848 takes `GlobalData` bit 31 (the local skater's
`PhysOut_Intents::mIsPlayerProvidingInput`, `AudioPresentation::DoSimSubmit` 0x827fb2f8) and
bit 12 (player 1's `CACRecord::IsCharacterMale`); `mAwayTime` is 0 while the player gives input,
else adds the update's whole milliseconds while unpaused, not loading and in game;
`mTimeSinceUnPaused` always adds them. `SFXObj_Pause::UpdateParams` 0x82532f48 sets `mbIsPaused`
(simulation paused or loading) and zeroes `mTimeSinceUnPaused` as the pause ends. The GVR, Danny
Way jam and mega jam flags are the main challenge's name (`c_3a_01_street_gvr`,
`m_3a_03_besttrick_danny`, `c_3a_01_jam_mega`).

### Outputs

`EventSender::Emit` 0x82ab3080 computes the parameters (getters run their scripts), then
`Csis::FunctionHandle::Set` and `Csis::Function::Call`; a missing function or client fails the
emit. `SPCH_AddEventDB` 0x82ee57e8 runs `iSPCH_InitCsis` 0x82ee5938 on every project's
`Output_Events.evt`: its Csis table (offset at +4, 32-byte rows, names after the rows) subscribes
`iSPCH_CsisCb` 0x82ee58b0 to each event's function with the event's spec. The callback adds the
event with its `numParms` parameter words (`iSPCH_AddEvent`, no callback data); the speech
instances play it as any `spch` event.

## 6. Wiring

- `skate2-core`: the `nis::audio::AudioInterface` trait; the scene manager's channels, queueing,
  readiness wait, audio clock and playback notices; the career session calls
  `play_mission_speech` for `Output::PlayNpcSpeech` and posts `Event::SpeechEnded` from the audio
  system's messages; the session exposes the main challenge (type, host, large arena, names, game
  mode) and the player bits.
- `skate2-audio`: `AudioSystem` creates the context systems in `enter_world` (with `mAwayTime`
  1), runs them after the speech update, keeps `mbIsPaused`, `mTimeSinceUnPaused`, `mAwayTime`,
  the player bits.
- The 'Spch' stream pool is made in `enter_world` too, where `AudioSystem::Initialize` calls
  `SpeechSystem::Initialize` (0x82509fbc). It used to be made in `Sk8SpeechSystem::new`, before the
  host set its files, so the game had no pool: every speech request ended unplayed, and a queued
  scene line never buffered, which held the first career scene at load state 5.
- `skate2-pc`: `GameAudioInterface` over the audio system with the device clock; the client fills
  the challenge, name and player fields of `GameAudioState` each tick.

## 7. What plays in the training area

`photo_challenge_lines_pick_their_contexts` runs each line's trigger with `m_1a_05`/`m_1a_06`'s
values (`ChallengeType` 38, host Atiba 38):

| Trigger | Context that speaks |
|---|---|
| `T_920_pickphot` | `Context920` → `O_920_pickphot` |
| `T_104_slam_cam` | none: `Context104` needs challenge type 23, 31 or 52, `Context104_706` Dyrdek hosting, `Context830` `Context104` within 5 s |
| `T_712_replay` | `Context712` → `O_712_replay` (filmer) |
| `T_4004_Rmd_Atiba` | `Context4004` → `O_4004_Rmd_Atiba` |
| `T_2727_SpnsrMeRmd` | `Context2727` → `O_2727_SpnsrMeRmd` |
| `T_1003_almost` | `Context1003_1003` (priority 100, 50% and 75% draws) or `Context1003_1005` (priority 5, 75%) |
| `T_1011_succ_PC` | `Context1011` → `O_1011_succ_PC` |
| `T_1015_failure` | none: no main-cast context uses it |

`skater.vlt` tuning: the cameraman's `O_4004_Rmd_Atiba` (event 32913) and `O_2727_SpnsrMeRmd`
(32882) start 2.1 s after their request; Atiba's `O_1011_succ_PC` (193) is a voice-over (plays
without her pedestrian, 0.5 s delay); `O_1003_almost` (137), `O_1005_sketch` (213) and
`O_1014_bored` (141) take the main cast's default tuning, so they play only through an attached
pedestrian state of character 38 (`GetVoiceInfoForSample` 0x8250be58), else they are answered as
not played. Passive lines: the cameraman's absence after 95 s in game without input, outside
challenges (and 110 s since he last said it), his return on input within 30 s of that; Atiba's
bored line in a photo challenge after 13 s without input.

## 8. Tests

`skate2-audio` (`cargo test --release -p skate2-audio -j 1`; game data with `SKATE2_GAME_ROOT`):

- `speech::context::builder`: a context file's trigger, conditions, output and getter; `ConvertAToI`.
- `speech::context::tests`: merge order; the random words' carries; the three systems load
  (counts above, every context's trigger subscribed); the mapped triggers' parameter counts; the
  training contexts hold offline in single player and fail online; the photo challenge lines
  (table above); `the_cameramans_reminder_plays_and_ends`: `PlayMissionSpeech(40, 4004)` →
  `Context4004` → the cameraman plays `O_4004_Rmd_Atiba` (4.24 s) on the voices instance and
  `cMsgSpeechEnded(40, 4004)` follows its end.
- `speech::nis::tests`: resident samples and channels; a story post buffers then plays.

`skate2-core` NIS tests (`manager_tests.rs`) compile (`cargo check --tests`); they are not run.

## 9. Not ported / left

- `FindCharacterToSpeak` (unknown speaker): `CSTATEMGR_NISCharacter` and `SFXObj_PlayerSpeech`
  states attach to nothing in the port.
- `AirCompletedHandler` 0x82505b40 (`cMsgAirCompleted`) is not received, so 1207 never plays as
  1211; `CheckForRoundStart` needs `mContestId`; `mChaseAudioData` is not fed (no
  `T_762_LvNoSk8Zn`, chase warnings).
- The music override has no user music (`XMPOverrideBackgroundMusic`); the front end does not
  consume `cMsgCallSpeechStarted`/`Ended`; `mInterrupting` and `mCachedEvents` are never set in
  this build.
- `mAwayTime` returns to 1 in `EnterFE` 0x824fd320, `ExitFE` 0x825087c0, `ExitWorld` 0x825087b0
  and `ExitGamePlay` 0x824ed990, which have no counterpart in the port's audio system.
- The context seed is the clock standing in for the time base, as for `rw::math::Random`.
