# Career: tutorial objective natives, unlockables, unlock overlay (2026-10-03)

Continues `docs/career/HANDOFF.md` (2026-10-02). All addresses are
`sk82_na_m.xex` VAs, read from the live IDA database (read-only).

## Why

The flip-trick tutorial `m_1a_03_tutorial_fliptricks` (the mission the sign-up starts) and
the next tutorials call objective natives the port did not bind, and completing `m_1a_03` runs
`Trigger_m_1a_03_tutorial_fliptricks`, which needs `Unlock.UnlockCharacterForFreeskate`, the
"Show Unlocks Screen" message (type 1) and "pSpawn Pros Arrive" (type 9).

A static census of every shipped chunk (`ChallengeCore.bin`, `progressioncore.bin`) and every
objective condition source found 432 natives the scripts reach that the port does not bind;
per-mission: `m_1a_03`/`m_1a_04` need only the six below; `m_2a_01_tutorial_slappy` needs
`ObjectiveGameInfo.SetFractionValue` and `ObjectiveInfo.GetTotalNumCompletedObjectives`;
`m_1a_05_photo_rail`, `m_1a_06_photo_puzzle` and `m_1a_07_follow_slappy` need 60–100 each (AI
skaters, highlight reel, photo selection, races, results screen).

## Objective natives (objectives' Lua state)

The objective natives reach the skater through `tObjectiveClosureData` → objective `+0x62C`
(`mManHolder`) → `SkaterMan` vtable `+0x18` (`FindByIndex(mPlayerID)`) → `+0x28` vtable `+0x14`
(`GetPhysOut`): `Sk8::Physics::PhysOut` (module pointers: `+0x30` `mTrick`, `+0x3C` `mScoring2`,
`+0x40` `mFiltered`).

| Native | Address | Reads |
| --- | --- | --- |
| `Wipeout.IsWipingOut` | `0x82d45b20` | `mFiltered->mFilteredPhysicsState == 4` |
| `Manual.IsManualing` | `0x82d452e8` | `mScoring2->mManualLength (+0x38) > 0` |
| `Manual.NoseManualTime` | `0x82d45668` | `mManualTime (+0x3C)` if `mTrick->mCurrentAnimData.mAnimTrickBits` bit 27, else 0 |
| `Manual.TailManualTime` | `0x82d45940` | same, bit 26 |
| `Tricks.DidSpecificTrick` | `0x82d4ae08` → `0x82d4a558` | objective `CompletedTricks` (+0xBE8) contains `StringHash64(name)`; `ld`/`cmpld` full 64-bit |
| `Tricks.DidSpecificTrickThisFrame` | `0x82d46dc8` | `mScoring2->mTricksEnded` (+0x590, elements +8, stride 0x130): `mIsValid` and not `mWasATapTrick` and `mTrickAttribKey` (`ld`/`cmpld`) == `StringToKey(name)` |

Facts checked:
- `eFilteredPhysicsState` from `PhysFilteredState` (`0x82c8c5e8`): invalid 0, ground 1, air 2,
  grind 3, wipeout 4, teleport 5, offboard 6, offboardair 7 — the port's enum order.
- `AnimTrickBits` allocates LSB first: switching 20, landing 21, ride idle 22, wipeout 23, trick
  24, grabbing 25, tail manual 26, nose manual 27, BS/FS revert 28/29, BS/FS powerslide 30/31.
  The port's motion callbacks already set `NoseManual = 0x0800_0000`, `TailManual = 0x0400_0000`.
- `Attrib::StringToKey` (`0x829768e0`) is a thunk to `StringHash64`.

Port: `MissionFrame`/`PlayerState` carry `manual_length`, `manual_time`, `anim_trick_bits`,
`wiping_out` (`physical_player_phys_out_mission.rs` now also requires the Trick component, which
is constructed with the others). Natives in `challenge/objective_natives.rs`.

## Unlockables (`Sk8::BE::UnlockablesBackEnd`, BE module 5) — `challenge/unlockables.rs`

- Layout: `UnlockableItem` (category, id, isNew) × 500 main + 500 DLC, counts, `mUnlockAll`.
- Constructor `0x82a739c8`: every slot (0, 0, new = 1). `Init` `0x8255cca0` → core `Init`
  `0x82a73c80`: count 0, then `AddDefaultUnlocks` `0x82566030`: skate shops
  `dist_financial_skateshop`, `dist_urbanres_skateshop` (cat 10); characters sammy, mike,
  slappy, shingo, cuz, seb, freeway (cat 4); every `fe_locations` RefSpec of
  `fe_location_group` `0x251427BAFAE99434` attribute `Location` (layout word 0 name, cat 2);
  `security_setting` (cat 13); 15 gestures (cat 14); `ResetAllNewUnlocks` (main list).
- `GetStringHash` `0x82a73430`: h = -1; h = tolower((signed char)c) + 33·h.
- `Unlock` `0x82a73a78`: `FindUnlockable` (`0x82a73370`, one list) → false if present (r7 = 0
  is returned; `FindUnlockable` leaves r7), else append (new = !fromDLC) and the virtual
  `UpdateUnlockStats` (vtable +0x1C = the shared empty function `0x825187c0`).
- `IsUnlocked` `0x82a73c98`: debug cheat (`kBEUnlockAll`, default 0 — `0x82f53778`; or
  `mUnlockAll`), else main list, then DLC when asked. `IsNewUnlock` `0x82a73d28`.
- Categories `kUnlockCategories` `0x82fd95d0`: CLOTHING, BOARDACCESSORIES, LOCATIONS, CHALLENGES,
  CHARACTERS, FMV, COVERAGE, SPONSORSHIP, OWN_THE_SPOT, OWN_THE_SPOT_OWNED, SKATE_SHOPS, REPLAY,
  EGGS, SECURITY_SETTING, GESTURES, CLOTHING_UNLOCK, CLOTHING_SPONSOR.
- Save `0x8255ce38`: chunk `UnlockablesBackEnd::Data` v1: UInt count, then 500 × (UInt cat, UInt
  id, Bool new). Load `0x8255cd08`: version 1 → `count` entries: `Unlock(cat, id, false)` then the
  item's `new` from the save. `LoadUnlocksFromFiles` (`data\unlocks\*.unlock`) is DLC-only and the
  data has no such folder.
- Savable order (`MemcardProfile::GenerateSaveData` `0x82569f48`): the version chunk named after
  the first savable, then savables in registration order; `UNLOCKABLES` is first.
  `SkaterProfile::set_leaf` with an empty `after` now inserts after the version chunk.

Natives (`unlockables::natives`, registered with the signpost natives in both Lua states):
`Unlock.UnlockCharacterForFreeskate` `0x8289b220` (`characters_marquee` layout word 0 → cat 4;
key pushed to `unlockedCharacters`), `Unlock.UnlockLocationForFreeskate` `0x8289b358` (cat 2,
`unlockedLocations`), `Progression.UnlockUnlockableByName` `0x825d5250` (cat 15, newly →
`unlockedCACItems`), `Progression.UnlockSponsorByName` `0x825d5108` (cat 16 →
`unlockedSponsorCACItems`), `Progression.UnlockGestureByName` `0x825c4270` (cat 14),
`Progression.IsUnlockUnlockedByName` `0x825c41e0` (cat 15, DLC included),
`ChallengeProgression.UnlockUnlockableByName` `0x8289bba0` / `UnlockSponsorByName` `0x8289bcd8`
(the native pushes the key into a *copy* of `tCompletionResults` that it destroys; mirrored: unlock
only).

## Completion results, unlock overlay, message types 1/9/14

- `tCompletionResults` (`0xA0` bytes) ctor `0x825cb070`: zeros, `Award` 15 (`eAwards_Total`),
  `MedalType` -1. Only the signpost ctor builds it.
- `cSignPostUtil::HandleCaughtMessage` `0x825de4e0`; `kEnableSignPostMedia` defaults to 1
  (`0x82f55b60`), so the shipped game takes the media branches. Media-off branches post
  `cMsgHUDContinue` (`503659400`) for types 1/5/7, `cMsgNISChallengeNotify(1)` for 2,
  `cMsgSpeechEnded` for 4, `cMsgFMVFinishedPlaying` for 15 (kinds verified with lookup2).
- Type 1 `ShowUnlocksScreen` `0x825cca80` by message id (layout word 2): 10 →
  `cMsgOpenUnlockOverlay` (FE `ReceiveOpenUnlockOverlayMsg` `0x826143f8` → PushState 10), 57 →
  `cMsgInitAutosave`, 180 → `NumPathsUnlocked++`, else `SponsorID` > 0 →
  `cMsgOpenSponsorOverlay` (FE state 11). "Show Unlocks Screen" is id 10.
- `FrontEndState_Unlock::ReceiveInput` `0x826595c0`: A after the cash animation → (boot flow run)
  `SaveAndClose` `0x82659468`: coverage unlocked → FE state 61, else `cMsgHUDContinue` + pop;
  `OnLeave` `0x82652990`: `ResetCashAwards` `0x825c44c8` (seven ints) and every result cleared
  (`photoArticle` kept). Port: `ChallengeSystem::unlock_overlay_continue`, `Output::OpenUnlockOverlay`;
  the screen itself is described under "Unlock overlay" below.
- Type 9 `TeleportPlayerMoment` `0x825c5db0`: local player to "Location Name", no fade.
  "pSpawn Pros Arrive" is type 9.
- Type 14 `HandleSkaterEvents` `0x825ccc10`: name (layout word 0) "Pin Player"/"Unpin Player" →
  `cMsgPinPlayer`. The skater +0x214 short-circuit (clears a flag instead) is not ported.

## Challenge teleports (`cChallengeTeleportManager`) — `challenge/teleport.rs`

Why: `m_1a_03`'s `challenge_running` enter handler (bytecode, no source) runs
`if self.IgnoreInitialTutorialReposition == false or AI.SkaterDistanceFromLocation(...) > 10 then
AI.RepositionAI(LocalPlayerID, StartLocation, 0, false); vars.Teleported = true else ... SetEvaluationStatus(true)
end`; only its `teleporttransitionfinished` handler turns objective evaluation on after a teleport. The port
teleported at once and never sent `cMsgTeleportTransitionFinished` (kind `-2012647376`), so no objective was
ever evaluated.

- `TeleportPlayer(player, location, posID, repositionMarker, transitionTime, setCamera)` `0x82dad8e0`
  (`setCamera` is r9; r8 is the float's shadow slot): transition > 0.001 or a fade running → queue
  `tTeleportInfo` (location hashed), `setCamera` → `mSetCameraPlayerID`; if no fade runs: half the transition,
  inline `StartEffect(FadeOut, half)`, `mFadeInProgress`. Transition < 0.001 → `TeleportPlayerNow` at once
  (no message).
- `TeleportPlayerNow` `0x82da6a00`: a skater that `FindByIndex` finds (or game type 8) →
  `ITeleportManager::TeleportPlayer`; with `repositionMarker`, the local player's session marker moves to the
  spawn point (`cMsgSetSessionMarker`).
- `OnEffectFinished` `0x82da6cd8` (Hermes handler, ctor `0x82dabd28`): FadeOut → teleport the queue, camera
  player, `mTeleportWaitTicks = 30`; FadeIn → `mFadeInProgress = 0`, teleport the queue, send
  `cMsgTeleportTransitionFinished`. `Update` `0x82da9a58`: the 30th tick starts FadeIn for the other half.
  `ClearTeleports` `0x82da9b10` (`AI.ClearTeleports`) ends a running transition at once.
- Sub-controller registration (`Sk8ChallengeSystem::GameInit` `0x825aa948`): audio, assets, pause, visual
  indicators, characters, sign-up, …, living world, **teleport**, AI, timers, …, objectives, …, effects; the
  port updates assets, sign-up, teleport, timers, objectives, effects in that order. Its handler is constructed
  before the progression manager's, so it receives Hermes messages first.
- Callers routed through it: `AI.RepositionAI` `0x8288d140` (posID defaults to the player, transition 1,
  setCamera arg 6), `Teleport.TeleportLocalPlayerToLocationName` `0x82db0640` (transition 1),
  sign-up `OnChallengeSelect` `0x82dadb20` (local player, transition 1; also clears `mIsPickAPro`), sign-post
  `TeleportPlayerMoment` `0x825c5db0` and `ExecuteChallenge` `0x825d27f0` (location challenges' `MapStartLocation`)
  (transition 0), presentation event 8 `TeleportLocalPlayer` (`PlayNextPresentationEvent` `0x82e07f38`,
  posID 0, transition 0.5) with `OnTeleportTransitionFinished` `0x82e08a00` resuming the group.
- The fade itself: `cChallengeEffectsManager::Update` `0x82daa1a0` sends `cMsgFadeAmount` only while a fade
  effect runs; `VDWriter::WriteData` `0x827ff8c8` passes it to the Visual Director once and resets it; the reader
  (`0x827fdd78`) keeps it per frame (`VDReaderData::sFadeInfo`). The renderer step from there to the screen was
  not traced; `RenderMan::FinalQuadFade` `0x8283fba0` draws an alpha-blended full-screen quad after the front
  end. The PC app draws black at that amount under its UI layers.

## Objective HUD (`ObjectiveHUD`, hud2/hudobjective2) — `challenge/objective_hud.rs`, `marker_ui/objective_movie.rs`

Core (the component and its data):
- `ObjectiveHUDData::Reset` `0x82d828e0`, `HUD2.SetupObjectiveHUDData` `0x82899810`, `GetDisplayedObjectives`
  `0x82d43658` (at most 4 rows), `MapObjectiveState` `0x826b2bd8`, objective `mTitle`
  (`cChallengeObjective` ctor `0x82d50d4c`: `StringHash(key)` → `FormatStringCustom(+0x10 args)` →
  `Snprintf(+0x508, 0x80)`; a `#` key is not special, so it falls back to the missing string).
- `HUDManager2::Update` `0x82cee4f0` / `FixupActiveComponents` `0x82cedff0` bind the component to the front active
  group with the component; `ObjectiveHUD::Show` `0x826bc780`, `Refresh` `0x826bc800` (state change →
  `SetScreenState(4+row)` before the row's title copy, then `SetScreenState(3)`), `Update` `0x826b2a88`,
  `RefreshScoringDisplay` (`SetScreenState(50)`). `SetScreenState` runs the screen's `SetState` at once, so each
  state carries the view the screen reads then (`HUDComponentsCmdTranslator::sMethod_Objective_*` `0x826b8248`…:
  title buffer `+0x2c`, rows `+0xcc + 0x60 i`, values `"#%d/%d"` `+0x24c + 0x20 i`, variant `mData+0x21`,
  `Objective_GetMaxNumObjectives` = 4).

Screen (`hudobjective2`, asset 29, FNV `c7731a08`, InitAction 81 `0x6fe8`, template entry `0x82fdb838`
placement 6 = `g3dHUDRight`, `SetApt3DFocusRightMatrix` `0x82608748`):
- The class methods are translated (`SetState`, `UpdateObjectivesX`, `InitTitleSection`, `ScaleTitleSection`,
  `UpdateObjectiveResultX`, `SetTypeState`, `ObjectiveScaleAdjust`, `ThreeSliceScale`). The movie is SWF 7
  (`Apt Data:1:7`; `AptLinker::Update` takes `nSwfVersion` from the header): `number + undefined` is undefined
  (`Add2` `0x82a03af0`), `toFloat(undefined)` is 0 (`0x829dcc48`), `Math.max` keeps arg1 unless
  `toFloat(arg1) <= toFloat(arg2)` (`0x829e8dd0`). `mBackground` has no `mLeft`/`mRight`/`mTop*` and `title100`
  no `labelTypeText`, so those authored calls act on undefined values.
- Text `_width` of an auto-sized, non-wrapping field is `textWidth + 4` (`objectMemberLookup` text members:
  `wordlist_6` `0x83001918` gives `0x14` = `_width`, `0x0f` = `textWidth`); `textWidth` is
  `cFont::TextExtent` `0x82849ba0` (`fmadds` of advances). autoSize right keeps the right edge
  (`EnsureStringAllocated` `0x82a12220`); the result/title fields are right-aligned single lines, so they draw
  unchanged.
- ActionScript `_x`/`_y`/`_xscale`… (`objectMemberSet` → `SetProceduralProperty(…, AS flag)` `0x829ea608`) set
  bit 31 of `AptCIH+0xc`; `placeObject` `0x82a17580` and `ReplaceDisplyListItem` `0x82a1a4c8` then skip timeline
  matrix/colour moves for that instance. The port's clip runtime now has this (`Clip::script_transform`); it keeps
  `mcTextTitle` where `ScaleTitleSection` put it when `mBackground` jumps to `1no`.
- Authored behaviour kept: the `fraction` label (frame 152) replaces the row's `label` with character 76, so a
  fraction row's first title only survives because `Refresh` runs `SetState(4+i)` before `SetState(3)`; a
  completed objective animates (and plays `FE_SOUND_TRICK_SUCCESSFUL`) only from `SetState(4+i)`.

Localizer (`marker_ui/localizer.rs`): `FormatStringCustom` `0x8297fee8` (comma split, 20×63, all types read as
text with `mAllArgumentsStrings`), `FormatStringExplicit` `0x8297f690`, `#` prefix (`LanguageManager::Initialize`
`0x82d66b08`), `FormatNumber` int `0x82980468` / float `0x82980a80` → `FormatNumberInternal` `0x82980610`
(double digits, `fnmsub`, round half up; float args take a GPR slot, so ProcessTime/Distance/Speed pass
`addPrefix = 0`, precision 1), custom types registered by `FEBackEnd::Init` `0x82570a40`: points `0x8255a468`,
distance `0x8255a2b8`, time `0x8255a358`, speed `0x8255a3b8` (imperial default, `tSettings::Default`
`0x825644d8`). The trick display's distance (`TrickDisplay_GetFormatDistance` `0x826b1f28`, precision 1) now
uses it: e.g. `36.9 ft`, where the port showed `37 ft`.

Three entry points format a label, and each call site uses the one its native caller uses (2026-10-07):
- typed `FormatString(label, …)` `0x8297fe70` from C++ (`format_string_int`, `format_hal`): a `d` token
  (`eFORMAT_ARG_INT` 1; `LocalizerManager::Initialize` `0x82980058` also registers `f` 2, `h` 3, `~` 4) shows
  `FormatNumber(int)`. `DPOHUDData::SetParams` `0x826b3338`'s count is `ID_COMMON_NUMBER` (`{0:d}`), so the
  DPO's best score groups (`28,212`); the port had read it as text (`28212`).
- `FormatStringCustom`, and scripts' `GetLocalizedFormatStringCustom` `0x82622e80` (`format_custom`): one
  comma-separated string; every non-custom token reads as text.
- scripts' `GetLocalizedFormatString(label, count, …)` `0x82622c00` (`localized_format_string`): an integer
  argument becomes `FormatNumber(value, false)` (grouped), a string passes as it is, and every token then reads
  as text. Party Play's join, player and round labels and both volume percentages use it; APT `Multiply` of two
  integers (`0x829e44c8`) and `Math.round` (`0x829e8c80`) stay integers, so the percentages take that path.

`original_dpo_values_format_as_the_native` and `original_script_format_arguments_as_the_native` assert the
shown text on the original assets.

## Tutorial callout HUD (`TutorialInstructionHUD`, hud2/hudtutorial) — `challenge/tutorial_hud.rs`, `marker_ui/tutorial_movie.rs`

Core:
- Presentation events 20/21 (`TutorialCalloutHUDEvent` `0x82e063c8`): the group's `TutorialInstructionHUDData`
  gets `SetInstruction(arg1, arg2)` (hide: `("none", "#")`), the presentation's own HUD group (`"p"` +
  `ConvertIToA(low 32 bits of the challenge key)[1..]`, `CheckPresentationGroupForHUD` `0x82e07d80`) is made
  active and its `tutorialinstruction` instance shown. `HUD2.TutorialInstructionSet` (defaults
  `"#FILL IN TUTORIAL HAL ID"`, `"nollie"`). Events 6/7 (pin), 9/10 (`HUDEvent` `0x82e06bc0`, component names by
  `GetHUDComponentName` `0x82e06ac0`), 15/16 (`MiniMapEvent` `0x82e062c0`) are handled too.
- `TutorialInstructionHUD` (ctor `0x826b5a50`): `Show` `0x826be510` (`SetScreenState` -10/-20), `Update`
  `0x826be7d0` (`RefreshTutorialHUD` `0x826be578` when the instruction changed: 11 none, 12 ollie, 13 nollie,
  14 kickflip, 15 `"popshuvit"` only (compared twice, so `tutorial_popshuvit` is an ordinary instruction),
  16 360popshuvit, else 10; `clear` hides; state 77 when `IsStanceTrickGesturesFlipped` changes).
  `GetTutorialHalText` `0x826bae30`; `GetTutorialGesturesButtons` `0x826bae70`: `[count, gesture, button,
  connector, …]` from `StringToLowerCaseKey(instruction)` in the trick class, else the tutorial class, slots
  until one has neither gesture nor button; `GetGestureString`/`GetButtonString`/`GetConnectorString`
  (`0x82644cf8`/`d20`/`d38`). Stance: `IsStanceTrickGesturesFlipped` = `(mCurrentStance != 0) ==
  IsPlayerStanceRegular`, `mCurrentStance` being `mIsRidingSwitch` (`PhysOutModule_Hud::UpdateStance`
  `0x827c4898` → tricks back end `+0x28`, read by `TrickDisplayManager::Update` `0x8261d258`); the mission frame
  now carries `is_riding_switch`.

Screen (`hudtutorial`, asset 30, FNV `5ca788c6`, InitAction 71 `0x9e6c`, template entry `0x82fdb868`
placement 6, same right-focus projection as the objective HUD) and controls/gesture_item (asset 31, FNV
`3577d433`, class constructor run as each `Gesture Item` is placed):
- `SetState`: -10/-20 visibility, -1 `ClearTricks`, 10 `ClearTricks` + `SethudtutorialItem` + `FillTrickData`,
  11 text + `ClearTricks`, 12–16 text + `ClearTricks` + `ShowAnimated*(true)`, 77
  `UpdateTrickGesturesForStance`. Constants `mMAX_GESTURES` 4, `buttonKerning` 28, `slashKerning` 17,
  `verticalOffset` 32; the constructor's `autoSave = "center"` is an ordinary member (not `autoSize`).
- Apt member lookup is case-insensitive (`AptNativeHash::HashFindKey` `0x829da970`: `EqualNoCaseHash`,
  `AptStricmp`; `EAStringC::CalculateHashValue` `0x829d8a30` lowercases A–Z; named children are registered in the
  parent's hash by `instantiateCharacter` `0x82a119a0`), so `mAnim360Popshuvit` reaches the authored
  `mAnim360popshuvit`. The port's `Clip::named` and `child` now match case-insensitively.
- `label.maxscroll` is text member 9 (`wordlist_6`), the text instance's `+0x10` filled by
  `EnsureStringAllocated` from `SkateAptString` (`0x82a70b28`) `MaxVisibleLines` = the wrapped line count. A tip
  the 30-point label wraps moves `mTextMC` to `smalltext` (never back) and the background grows 18 per further
  line; `_height` (`SetProceduralProperty` `0x829ea608`) rescales `d` by value / bounds height (bounds through the
  own matrix, `GetBoundingRect` `0x829e24e8`), at least 1.1322573 percent (`0x3f90edcf`).
- `attachMovie` / `createEmptyMovieClip` insert at depth + 0x4000 (`sMethod_attachMovie` `0x82a1d088`,
  `0x82a18d58`), replacing the clip at that depth (`InsertChild` `0x82a15e50`, forced instance), and
  `findCharacterInLibrary` `0x82a198e8` matches the control's exports case-insensitively. The gesture
  (`g_<gesture>`) therefore draws above the authored stick and `RS` label.
- `UpdateGestureForStance`/`UpdateStanceAnimated*`: `_xscale` -100 (gesture faces left) for a regular stance,
  i.e. `!IsStanceTrickGesturesFlipped()` with `mUseInGameStance` (always set before an item shows); the
  constructor's `PlayerStance_IsRegular` scale is replaced before any item is shown.
- Frame scripts' `gotoAndPlay` (`container_mc` idle loop at frame 125) are queued and run after the frame's
  display lists, as Apt runs queued actions.

## In-game HUD groups and presentation central messages

- `FrontEndManager::StartInGame` `0x82640450`: `main` (central, minimap, trickdisplay, homscoring,
  online_chyron) and, outside Party Play, `phonelist` are created and made active (phonelist in front);
  trickdisplay, minimap, online_chyron and phonelist shown, homscoring hidden; the `"Minimap Visible"` message
  setting is set to 1 (`GetMessageKey` `0x825cb150`, `MessageDataMap` value). `ChallengeSystem::start_in_game`
  ports it; the career session runs it after `LoadingReset`.
- Presentation events 22–25 (`PlayNextPresentationEvent` jump table `0x82e08008`): 23 central message
  (`CentralHUDData::DataShowCentralMessage` `0x82d82b30` with empty arguments on `main`'s central data, shown by
  `CentralHUD::Update` `0x826bc398` with audio hook 0, as a default `HUD2.ShowCentralMessage`), 22 modified,
  25 custom (arg2), 24 modified custom. Event 23 now reaches the central HUD; 22/24/25 stay unpresented.

## Sign-up prompt (`SignUpHUD`, hud2/actionoverlay2) — `challenge/signup_hud.rs`, `marker_ui/action_overlay_movie.rs`

Core:
- `cChallengeSignUpManager::GameLoad` `0x82daec00` creates the `cChallengeSignUpManager-DPadUp` group with the
  `signup` component (`SetSignUpType(eSignUpType_SignUp)`); `Reset` `0x82dabc80` deletes it.
  `ShowDPadUpHUD` `0x82da2cc0` makes it active and shows the instance; `HideDPadUpHUD` `0x82da2d10` hides the
  instance and removes the group from the active list (`RemoveGroupFromActive` `0x82cec2d0`).
- `HUDManager2::Update` `0x82cee4f0` runs whenever the front end is in game (no pause gate,
  `FrontEndManager::Update` `0x82640130`): `FixupActiveComponents` `0x82cedff0` binds each front-active group's
  components (`BindData`, then the data's `MakeDirty`) and unbinds the rest with `Show(false)`; then each bound
  component is shown/hidden to match its instance and updated.
- `SignUpHUD::Show` `0x826c0430` (refresh, then `SetScreenState(state)`, press delay 15; hidden:
  `SetScreenState(0)`), `RefreshFEScreen` `0x826bd2d0` (sign-up data: no challenges 0, a single Own the Spot 5,
  else 1; `SetScreenState` only for dirty data), `ShowChallengeTitle` `0x826bd1f0` (one challenge's `Title`
  through `FormatStringCustom` with no arguments, several: `ID_SIGNUP_MULTIPLIESIGNUP_SIGNUPHUD`, none: buffer
  kept), `Update` `0x826bd3e8`. `SignUp_GetChallengeTitle` `0x826b8210` and `GetTeleportTitle` `0x82638710`
  both read the component's buffer at `+0x34`.
- `UpdateDPadUp` `0x826bd450` with several challenges opens the popup (title
  `ID_SIGNUP_MULTIPLIESIGNUP_SIGNUPHUD`, at most 4 options, cancellable) and calls the component's own
  `Show(false)`; `MultipleSignupsCallback` `0x826b5430` signs up or, on cancel, calls `Show(true)` directly.
- `SignUpData::SortChallenges` `0x82d83618`: EASTL `quick_sort` with `SignUpChallengeSorter`, which ranks an
  entry before any other unless it is Create-a-Spot; up to 28 entries only `insertion_sort` `0x82d82d30` runs,
  so each other entry moves to the front in turn (the order of two overlapping sign-ups reverses). The port did
  not sort before.

Screen (asset 32, FNV `9bd9bc9c`, InitAction 48 `0x366c`, template entry `0x82fdb8f8` placement 0 — drawn 2D
like hudcentral):
- Constructor: the DMO/OTS/icon pieces' `_x += Screen_EdgeOffset` (zero; the AS sets still take them off their
  timelines), `Sign up` (`ID_COMMON_SIGNUP`) centre-auto-sized in both symbol fields, `mButtonX = -4 -
  signup._width / 2` (`_width` = `textWidth + 4` for an auto-sized single line), symbol 16 right.
- `SetState`: 0/-1 `Show(false)` (DMO, OTS or button outro); otherwise title hidden, `Show(true)`, then
  Challenge (D-pad up, title shown, `Sign up` symbol), Teleport (title as the symbol), PressStart, OTS (title in
  the OTS panel, `ots_intro`), `button_intro` otherwise. DMO grab (4, `InitDMOHUD`, `DMO_GetParams` `0x826b88c8`)
  is unreachable for the challenge sign-up data and returns an error.
- Apt text `_x` reads/writes on text fields are offset by the field's left bound (`objectMemberLookup`/
  `objectMemberSet` text `_x`: getter `+x0`, setter `+2`), so a zero edge offset leaves them unchanged.

## Progression dialog (`cMsgProgPopupOpener`) — session `CareerHud::dialog`, `marker_ui/progression_popup.rs`

`FrontEndManager::ReceiveProgPopupOpenerMsg` `0x82638748`: sk8popup with the message's title/description, options
`ID_COMMON_NO` (0, default) and `ID_COMMON_YES` (1), not cancellable. `OpenProgressionPopupCallback`
`0x82897720`: option 0 sends `cMsgHUDNo` (hash `0x4edc56c2`), option 1 `cMsgHUDYes` (`0xe4a8382f`). The session
showed nothing and answered Yes; it now shows the popup (same template as the pause quit popup: up/down step,
A confirms). The Start State Machine's `MsgBox` state: Yes plays the basics FMV, No executes
`m_1a_01_goto_slappys` directly.

## Sign-up poster (`FrontEndState_SignUpPoster`, challenge/challenge_signup) — `challenge/signup_poster.rs`, `marker_ui/signup_poster_movie.rs`

FE state and sign-up manager:
- `OpenPoster` `0x82658240` pushes FE state 32. `gFEStateInfo[32]` (`0x82fdf000`): HUD `eHUDState_Hide`
  (`UpdateOnEnter` `0x82d663f0` → `UpdateHUDVisibility(false, false)` `0x82656ac0`), music state 4, input to
  ActionScript. `OnEnter` `0x82647f68`: sound `0xB3585B93`, `mActionsBlocked = 1`, `OpenScreen(19)` (the
  render-effect/slow-down branch only runs on re-entry after the tutorial video).
- `OnIntroDone` `0x826581a0` (the screen's frame 56 `ScreenManager.IntroComplete`): `cMsgHUDIntroDone`
  (`0xBDE7CEA2`; no Lua handler) and `mActionsBlocked = 0`. `HUDIntroDoneMsg` `0x82da2c98`: while
  `WaitingForConfirmation`, `mIsSignUpSlowDown = 1`, rate 0.5. `Update` `0x82db44a0` ends every state with
  `cMsgSetSimRate(0.016666668 / rate, 1 tick)` while slowed, so the world runs at half speed (30 Hz timer)
  behind the poster. `PlayerAcceptChallenge`/`PlayerDeclineChallenge` (`0x82da66e0`/`0x82da6800`) clear it
  (rate 1.0) and `mShowSignUpRenderEffect` after the high-priority operations; `RunLowPriorityDPadUpOperations`
  sets the render effect before `OpenPoster` (nothing reads it in this build). Constructor rate 1.0;
  `Reset` `0x82dabc80` clears the flag.
- `ReceiveInput` `0x82657bf8` (`StateManagerFrontEnd::ReceiveInput` `0x82656af8` only passes input when the
  state allows it), once the intro is done. Accept (keycode 2): a betting challenge (`ChallengeType` 19 or a
  throwdown, `0x826481d8`) without `HasEnoughCashForMinBet` (`0x826482f8`: before the boot flow always, else
  cash ≥ `MinimumBet`) only plays `0x2F2CAED4`. Otherwise the entry fee: `Paid Entry Fee` save data,
  `Entry Fee` economy data plus `GetExtraEntryFee`, cash, `HasBoardSponsor`; a board sponsor marks it paid; a
  fee that is neither paid nor covered opens popup 203 after the boot flow (`ID_COMMON_WARNING_TITLE`,
  `ID_CHALLENGEINFO_FEE_FAILED`, `ID_COMMON_OK`; `CantPayFeeCallback` `0x82641ef8` clears `mActionsBlocked`);
  a covered one runs `ProcessEntryFee` `0x825de3f0` and is marked paid. Then `cMsgHUDYes`,
  `PlayerAcceptChallenge`, the audio's `NotifyChallengeLoad`, the announcer stopped, sound `0x7A4DDB99`,
  `PopState`. Decline (keycode 3): `cMsgHUDNo` (`0x4edc56c2`), `PlayerDeclineChallenge`, then for a throwdown
  or dynamic-competitor challenge `ResetDynamicallyChosenAICompetitorInfo`, otherwise `SetHUDState(1, -1)`
  (`0x82d6a520`), the announcer stopped, sound `0x9FD64A5E`, `PopState`. Keycode 5 plays the
  `TutorialVideoAsset` (FE state 28) when `GetHasTutorialVideo` (`0x826480d8`: `kbAllowSignUpVideoToBePlayed`,
  default 1, and a `TutorialNISHalTitle` other than `ID_COMMON_NONE`) and sends `cMsgHUDOverlay`
  (`0x4116DB66`).
- `FEBackEnd::Data.mBootFlowExecuted` is set by `StateMainMenu::MainFEState` (`0x82778448`); the career is
  always entered from the main menu.

Natives the screen reads (all of the poster's challenge):
- `GetSignUpTitle` `0x82626240`/`GetSignUpDescription` `0x826263e0`: `FormatStringCustom(key, null)`, or
  `#Missing Title...`/`#Missing SignUp Description...`. `GetTutorialNISHalName` `0x82626a60`:
  `TutorialNISHalTitle` (`0x59cd0b9449d19c3c`). `IsCompetitorChallenge` `0x82627008` reads
  `IsDynamicCompetitorChallenge` (`0xf3a508058ae5148d`).
- `Mission.IsDescriptionEnabled`/`IsObjectivesEnabled`/`IsCompetitorsEnabled(false)` (`0x82625b40`,
  `0x82625850`, `0x826259c8`): the activity's `challenge_local_data` bools `0x137f56d7c41456f1`,
  `0x858d73cd6085c8ec`, `0x0abdc707fc5ddf63`.
- `SignUp_GetNumObjectives` `0x826b7ca8`: 1 for `ObjectiveFlow` type 1, else the `Objectives` with
  `DisplayInHUD` set or absent; `SignUp_GetObjectiveTitle` `0x826b7f10`: the same walk, `FormatStringCustom`
  of the definition's HAL key (+12) with its arguments (+16), `#OBJECTIVE NOT VALID!` past the end.
- `EntryFee_GetFee` `0x82627658` (0 with a board sponsor; `Entry Fee`, 0 once paid, plus the extra fee) and
  `EntryFee_HasPlayerPaidFee` `0x82627390` (a sponsor pays; an extra fee is never paid).
- Signpost: `GetChallengeSaveData`/`SetChallengeSaveData` (`0x825cf418`/`0x825cf560`, by save type name,
  only existing entries), `CheckIfChallengeTypeHasSaveDataType` `0x825c9cf0`, `GetChallengeEconomyData`
  (`0x825d2ff0` → `0x825cf7f0`; tags `0xb88344f072a79d1f`, rows `0x22ddb3d8601ae9f5`), `GetCash`
  `0x825d2348`, `SubtractCash` `0x825dd7f8` (also for 0, which triggers the `Cash` pool's machines),
  `GetExtraEntryFee` `0x825d03e0` (200 while the local data's pool-drain challenge is incomplete, +100 per
  `Capped` uncap hull; the getters are folded with `0x82db40e0`/`0x8276cb50`), `HasBoardSponsor` `0x825d1290`
  (`GetMessageSetting("Board Sponsor", kMissingSponsorName)` `0x825cd7f0` among the `sponsor` names,
  `kMissingSponsorName` pointing at "Online"). `ProcessEntryFee` hands the converted key to the save-data
  calls, which convert it again.
- `Cash_GetPlayerCashTotal` `0x8260b008`; `GetLocalizedFormatCurrency` `0x82604418` → `FormatCurrency`
  `0x82981318` with `SetupCurrencyFormat` `0x82981418` over `ID_COMMON_CURRENCY_SIMPLE_FORMAT`
  (`$XXX,XXX.XX` when missing).

Screen (asset 33, FNV `711e9b3c`; template entry `0x82fdb488` `SignUp_Poster`, placement 0, drawn flat):
- InitAction (character 66, `0x5ff4`) defines the class; root action `0x5fc0` constructs it: `stop()`, the
  button backer's `tutorial`/`normal` frames by `HasTutorialNIS`, `mcChallengeInfo.SetMode(2)`, the zero edge
  offset added to `mcChallengeInfo._x`/`mBacker._x` and half of it taken from `mVignette._width`
  (`SetProceduralProperty` width: ratio of the bounds width, `(a·ratio)·100` as `_xscale`), `RegisterScreen`.
  Frame 1 `intro` (`0x6348`: `_visible`, `PlayIntroAudio`), frame 56 `IntroComplete`, frame 57 `outro`
  (`PlaySignupOut`), frame 69 `OutroComplete`.
- `challenge_info.SetMode(2)` (`0x482e`; sign-up branch `0x4bb8`..`0x4e98`): everything but `mHeader`
  disabled; title and description; the entry fee (`ID_CHALLENGEINFO_ENTRY_FEE_PAID`, or `#` + the currency,
  coloured 11221784 when the cash is short); the description per the local flag; the button menu
  (`SetJustify("left")`, `A` `ID_COMMON_ACCEPT` unless betting without the minimum bet, `B`
  `ID_COMMON_DECLINE`); the tutorial-video option; `mHeader` at `mTitleOffsetX/Y`, which the prototype
  (`0x557a`..) sets to 0; `InitObjectives` (`0x5399`); `InitCompetitors` (`0x5481`).
- `UpdatePositions` (`0x3ec4`): `mYOffset` = header `_y` + `_height` + 10; a title wrapping at 30pt takes
  `smalltext` (20pt) and, on one line there, `smalltextcenter`; description +5 then 16 per line + 15;
  objective rows 26 per line under their title; entry fee +10 then its height + 10; `mView` at 577 and
  `mNISOption` at 525 in this mode. Multiline auto-sized fields keep their authored bounds (`SkateAptString`
  `0x82a70b28` changes x1/y1 only for single lines); Apt bounds include hidden children and skip masks
  (`AptDisplayList::GetBoundingRect` `0x829da2c8`).

Not ported here: the tutorial video (no movie player or decoder), the fee-failed popup's screen (a log line),
the betting panel and the competitors list (log lines), the uncap-hull extra fee (needs dynamic hulls; a
script error), FE sounds, and `cChallengeOtsDiscoveryManager::CollidingSignupChallenges` (`0x825c09a8`, the
only colliding-challenges sub-controller).

## Unlock overlay (`FrontEndState_Unlock`, unlock/unlockoverlay) — `challenge/unlock_overlay.rs`, `marker_ui/unlock_overlay_movie.rs`

Completion-result writers ported for it:
- `ActivateChallenge` `0x825e02c8` calls `AddCompletedChallengeToResults(neutrinoKey)` `0x825d6fe0` outside
  the initialisation: the activity (`cActivityManager::GetChallenge`, vtable slot 5) with `ChallengeType`
  (`0x02b828ccdb4785e4`) 45 joins `unlockedProperty`, any other `unlockedChallenges`, once (full 64-bit
  compare).
- Message type 6 `HandleThrowdownEvents` `0x825d72e0`, by message id (layout word 2), over the leaf
  `throwdown_competitors` (`0x9c869217c5aee9b4`): `PhoneNumberMessageID` (+0x56) equal → `NumPhoneNumbersUnlocked++`
  and the competitor's key; `CompletedMessageID` (+0x5e) equal and `DemoPro` (+0x4c) → `sponsorMeBrand` = the
  `BoardSponsor` (+0x30) collection (0 when absent, `Instance::GetCollection` `0x82975d60`); id 180 →
  `sponsorMeCreated`.
- `Award.GrantAward` `0x82886078` → `cSignPostManager::GrantAward` `0x825c4618` (`AwardUnlocked`, `Award`). Its
  first call, `StatsBackEnd::GrantAward` `0x8255eda0` (module 14, vtable slot 11:
  `mAwardInfo[award].mAchieved`, asserting an award below 15), is not ported: the stats back end has no
  career savable in the port.
- `ChallengeProgression.UpdateCashAward(data, rank)` `0x828914c8` → `AwardPrizeCash(key, rank, saved "Highest
  Rank")` `0x825de2d0` (registers r4 key, r5 new rank, r6 old rank; the decompiler swaps them): rank ≤ 3 sets
  `MedalType = rank - 1`; with no old rank or a better one, `AddCash` (`0x825dd7a0`, credit 3 contest purse) of
  the difference of the places' economy data (`0x82fdac60`: "1st Place".."6th Place") and the rank saved
  through the converted key. All skipped while `sGameInfo.isReplayingChallenge` (never set by the port).
- `CheckChallengeResults(list)` `0x825d0968`: `GetListKeyFromName` `0x825c8c30` (`progression_lists` word 0) and
  `IsChallengeInList` `0x825c6068` (its `Challenges` refs, `0xec1c213bbdbf1a68`) over `unlockedChallenges`
  converted to signpost keys. The medal natives use it with "Mag Minute", "RobDyrdek Path", "Throwdown
  Locations", "DannyWay Path" and, for `Property_Unlocked`, "FE My Properties".
- `CASUnlockPartDB` (`casunlockpartdb.cpp`): `Load` `0x8259eb68` reads
  `data\content/recipe/createacharacter/cas_db_000.unlocks` (`CASDLC::Init` `0x8259e080`: `cas_db` + `_%03d` of
  the on-disk version); `Resolve` `0x82585b88` checks version 1 and the size; `GetUnlockFlags` `0x82585b08` binary
  searches the 12-byte entries (key, part types). 109 entries in the shipped file.

`FEUnlock` natives (`UnlockCmdTranslator::Initialize` `0x8262c108`), read once when the screen opens:
- Cash: `GetCashAwards(credit)` `0x825c44f0` (`mCashEarnedResults`): prize 2 (`0x8260baa0`), purse 3, wager 1,
  sponsor bonus 4, trick callout 5, bonus objective 6 (the `li r4` at +0x8c of each); `Cash_GetPlayerCashTotal`
  `0x8260b008`; `Cash_GetFormattedCashString` `0x826055a8` = `FormatCurrency` with the prefix flag (r5 = 1; the
  float takes r4's slot), so `#` + the currency (`RegisterFormatPrefix("#")` in `LanguageManager::Initialize`).
- Lists: `Phone_GetNumbersUnlockedList` `0x8260c128` (competitor `HALIDString` +0x48, else
  `ID_UNLOCK_PHONE_NUMBER`), `Character_GetUnlockedList` `0x8260cdb0` (`characters_marquee` word 1, else `#`).
- Flags: numbers (`NumPhoneNumbersUnlocked > 0`), location, `Trophy_IsTrophyUnlocked` (`AwardUnlocked`),
  coverage, the medal lists above, `Medal_IsSponsorTapeUnlocked` (`sponsorMeCreated`), character,
  `Medal_IsProChallengeUnlocked` (the constant-false `0x82605650`), `Path_NumPathUnlocked`,
  `Location_NumLocationsUnlocked`, `GetLocationIconText` (`fe_locations` word 2, else `ID_UNLOCK_LOCATION`),
  `IsCoverageThrasherCoverage` (the photo article's word 6 is 0), `GetSponsorMeTapeText`
  (`ID_UNLOCK_SPONSORTAPE`).
- Sponsor items: `Sponsors_GetNumSponsorsToCycle` (1 with CAC or sponsor CAC items),
  `SponsorX_GetUnlockHalText` (first `progression_asset_group_unlocks`, else `_sponsors`, word 1, else
  `ID_COMMON_UNKNOWN`), `SponsorX_IsNewInventory` (always true), `SponsorX_Is*Unlocked` → `CheckUnlockResults`
  `0x825c5ab0` with board 1, truck 2, wheel 4, shoe 8, clothing 16 over `GetUnlockFlags`.

FE state 10:
- `gFEStateInfo[10]` (`0x82fdee48`): `mbPause` 1, `mShowHUD` Hide, music 4, input to ActionScript.
  `UpdateOnEnter` `0x82d663f0` pushes the FRONTEND pause (`PauseMan::Push(3)`) and `UpdateOnLeave` `0x82d66300`
  pulls it after `OnLeave` and `CloseScreen`, so the world and challenge system stop while it is open.
- `OnEnter` `0x82659370`: `CentralHUD::ForceCentralHUDOutro` `0x826bc348`, `cMsgResultScreenEffect` (kind
  `0xBF6ABED7`, `fbEnable` 1, to the presentation broadcaster; delivered to Lua only — not ported), `OpenScreen(20)`.
  `OnUpdate` `0x82659408`: `SetScreenState(50)` each front-end update. `OnIntroDone` `0x826423c0` clears
  `mCashAnimDone` (+0x50) and `mAudioStopped` (+0x51); the constructor (inlined in
  `StateManagerFrontEnd::Initialize` `0x8268c330`) leaves both unset. `Cash_SetCashAnimDone` `0x82624038` sets
  `mCashAnimDone`; `ReceiveInput` takes A only then.
- Rates: `GameModule::Update` `0x8273edc8` → `FrontEndManager::Update(1666)` `0x82640130`: `AptIntegration::Update`
  with 16 ms (the spin, then one 16 ms Apt frame), then the state manager (`OnUpdate`). One movie frame and one
  `UpdateAnimation` per front-end update.

Screen (asset 34, FNV `07458651`; template `UnlockOverlay`, placement 4 = `g3dSpinContainer`):
- `SetVertexMatrix` `0x826095e8`: levels 4/7 use `SetApt3DSpinMatrix` `0x82608be0`, the focus views' matrix
  turned by `mCurrentSpinAngle` itself and without the HUD jolt. `ScreenManager`'s clip sequence (`0x0e86`) calls
  `Game.On3DIntroStart` (`0x8261af98`) for a type 4/7 screen's `intro` and `On3DIntroExit` (`0x8261b070`) for its
  `outro`; `CloseScreen` (`0x0da8`) plays `outro`. `AptIntegration::Update` `0x826014c0`: intro/outro step by
  `mCurrentSpinDelta` to the target, idle subtracts `XMVectorSin(mIdleSpinAngle) * kfSpinAmbience`. Defaults
  (cvar initialisers): start 90, `kfSpinDistance` 550, 20 frames, ambience 0.04, rotation delta 0.01;
  `kfDistanceOf3D` 624.
- Class (InitAction char 143 `0xa33c`, guarded by `!_global.unlockoverlay`; root `0xa320` constructs):
  `InitMainRewards` counts the flags (trophy and photo path forced false, path false) into the class static
  `numMainRewards` (capped 4), which only the definition sets, so it accumulates over a session's overlays;
  `mAwards.gotoAndPlay(String(n))`; `InitUnlockItems`; `InitCash` (prize = prize + purse + bonus objective; ≤ 0.01
  greys and hides a line; this movie has no `mCashTrick`/`mCalloutVal`; wallet from cash minus the awards, at
  least 0); `main_mc.gotoAndPlay("<r><i><c>-intro")`. The proceed button is `button_menu` with the prototype's
  `mJustify` "right" (`ID_PROCEED_FROM_UNLOCK_TO_BLOG` when coverage was unlocked, else `ID_COMMON_PROCEED`).
- `SetState(50)` → `UpdateAnimation`: 30 updates of delay, then a step when no reward animation runs.
  `ProcessTopDrawerNext` fills slot `phase` (one further once `phase ≥ numMainRewards`) with the first remaining
  reward (trophy, magmin, prochallenge, rob, throwdown — label `ID_UNLOCK_SHINGO ` with the tape — sponsor tape,
  dway, path, location, property, coverage, phone numbers, characters, photo stage); the property branch clears
  `bPath`, so a property repeats; numbers/characters take one step per entry plus a total. `ProcessSponsorNext`:
  the logo's `iconintro`, the five items `on`/`off`. `CashTick`: prize, wager, sponsor bonus and wallet each
  count `goal/30` per update, then `Cash_SetCashAnimDone`. Steps wait for `_global.UnlockAnimationStepDone` from
  `mMainComp` frames 26/86, the item clips' frame 21 and `icon_mc` frame 21.
- Apt numbers: `ActionAdd2` `0x82a03af0` adds integers as integers and otherwise as f32; `ActionDivide`
  `0x829e4658` always divides as f32.

Not ported here: `cMsgResultScreenEffect`, FE sounds, the stats award table, the trophy image (its flag is forced
false), the coverage screen (FE state 61, still answered at once), the sponsor logo image (the class never loads
it).

## Signpost message types 4/5/7/11/16, NPC speech, Skate 101, BLOG news — `challenge/signpost.rs`, `marker_ui/skate101_movie.rs`

`HandleCaughtMessage` `0x825de4e0` has cases 1–7, 9–12, 14–16 and 18; types 8, 13 and 17 fall through to
nothing. `HandleCaughtMessageScriptOverride` `0x825cc918` only answers while `Juice::JuiceRPCNG::sIsProgressionRunning`.
With `kEnableSignPostMedia` on:
- Type 4: `PlayNPCSpeech` (`0x82da1bb0` → `PlayMissionSpeech(character, id, 0)`) of the int properties "CharacterID"
  and "FileID" (media off: `cMsgSpeechEnded` kind `0x157EF735` at once). No progression machine handles
  `cMsgSpeechEnded`; the signpost's `ReceiveSpeechEndedMessage` `0x825d2e08` runs script type 6.
- Speech identity is `eSk8Characters`: the presentation director's `mSpeechCharacterMap` (constructor
  `0x82e08a88`) maps `GetHashValue32` of "Player".."SFXPost" to 0..99, the same list `ChallengeSpeech` binds;
  `OnSpeechEnded` `0x82e08870` compares the enum and id. Every shipped speech event (CameraMan, Slappy, Abita)
  is in the map; an unmapped name asserts (challengepresentationdirector.cpp:452).
  `ChallengeSpeech.PlayCameramanSpeech(id)` `0x82888f98` (and `PlayProgressionReminder` `0x82889040`, a branch
  to it) is `PlayMissionSpeech(eSk8Character_CameraMan = 40, id, 0)`.
- Type 5: `cMsgSkate101Show` (`0xF9141E2A`) to the front end. `FrontEndManager::ReceiveSkate101HUDShowMsg`
  `0x826285e8` asserts FE state 79 is not stacked (fe.cpp:1607), copies "TitleHALID"/"TextHALID" into the state
  (`SetTitleHAL` `0x82691ad0` +0x38, `SetDescriptionHAL` `0x82691a88` +0xB8, `Snprintf(0x80, "%s")`; the constructor
  writes `#NO TEXT`) and pushes it. `gFEStateInfo[79]` (`0x82fdf3ac`): `mbPause` 1, `mShowHUD` 3 =
  `eHUDState_Ignore` (`UpdateOnEnter` `0x82d663f0` leaves the HUD), music 9, input to ActionScript. `OnEnter`
  `0x8269fd58`: `cMsgPinPlayer(1)` (`0x7949DAC1`), `OpenScreen(135)`; `OnLeave` `0x8269fe08`: `cMsgPinPlayer(0)`,
  `CloseScreen`. `ReceiveInput` `0x8269feb0`: keycode 2 (A) sends `cMsgHUDContinue` and pops; it returns 0, so
  the input also reaches the screen, whose `UpdateInput` is empty. The 101_* messages have empty properties
  and no sender.
- Front-end input: `StateManagerFrontEnd::ReceiveInput` `0x82656af8` passes input only to the top state whose
  screen is ready (`IsScreenReady` `0x82d661a8`); `OnScreenReady` `0x82d65f98` (from the screen's `IntroComplete`
  through `OnScreenIntroCallBack` `0x82d66fe0`) sets it before `OnIntroDone`. The port now gates the unlock
  overlay's A on this too, so a second overlay ignores A during its intro even though `mCashAnimDone` is
  still set from the first.
- Screen 135 (template entry `0x82fdbbc8` "Skate101", `tutorials/skate101.swf`, placement 0, flat; asset 35,
  FNV `fbc3f712`): InitAction (character 32, `0x12d4`) defines `skate101`; root `0x1290` constructs it:
  `mScreen.stop()`, title `FELanguage.GetLocalizedFormatStringCustom(Mission.GetSkate101TitleString(), "x")`
  (`0x826250e8`), description text `Mission.GetSkate101DescriptionString()` (`0x82625118`), a `maxscroll`/
  `CalculateFontHeight(18)` height nothing reads, `mButton.SetButtonType("A")` (its placement's
  `mButtonImage` is "B"), `RegisterScreen`. Frame 1 `intro` (`Play101OverlayIn`; its `StartIntro` call targets
  a clip the movie lacks), fade frames 1–13, frame 14 `IntroComplete`, frame 15 `outro` (`Play101OverlayOut`),
  frame 29 `OutroComplete`.
- Type 7 `CellPhoneMoment` `0x825d0798`: the setting named by "Cell Phone ID" is set true
  (`SetMessageSetting<bool>` `0x825caa88` by `GetMessageSettingKeyFromName` `0x825c91f8`) unless
  `GetMessageSetting(name, 1)` `0x825cd6a8` already reads true; a missing setting reads true.
- Type 11 `ShowDebugText` `0x825c5e88`: `cChallengeDebugManager::DrawText2DPersistent` `0x82da37f8`, empty in
  this build.
- Type 16 `HandleBLOGNewsEvent` `0x825d9410` ("HALText", "Type", "RemoveEntry"; bool at layout byte 16,
  `GetMessageBoolProperty` `0x825c5360`): type 0 adds the message's own key (`GetMessageKeyFromID` `0x825c8fc8`
  of layout word 2) as a new `eBLOGType_Message` entry at `++intBLOGCount` unless it has one (during the
  initialisation the entry is allocated and never recorded); a removal finds the adding message
  (`GetAddBLOGMessage` `0x825ca1b8`: the first type-16 message of that type and text that is not a removal)
  and `RemoveBLOGEntry(3, id)` `0x825d0128` (`GetBLOGDataInOrder` `0x825c8108`, last in map order;
  `ShuffleBlogEntriesDown` `0x825c8420`; `--intBLOGCount`). Type 1 is user news: `AddUserNews` `0x825d9370` /
  `RemoveUserNews` `0x825cc740` keyed by the 64-bit message key (`CheckUserNews` `0x825cc6c8` compares all 64
  bits); `LoadProgressionData` inlines `AddUserNews`; `SaveData` writes the map size, so `intUserNewsCount` is
  not kept.

## Progression natives: dynamic hulls, pool draining, session marker, achievements — `challenge/dynamic_hull.rs`

A census of the progression machines' calls (comments stripped) against the port found these natives
unported; each raised a Lua error that ended its handler (e.g. `Trigger_m_1a_05_photo_rail`'s Final state never
reached `Terminate()`, `Trigger_m_1a_07_follow`'s Activation never reached `ChangeState("NIS")`):
`PlayProgressionReminder` (20 calls), `SetupDynamicHullState` (17), `GrantAchievement` (15), `DidPlayerBetMax`
(11), `DrainPoolViaProgression` (5), `TestForDynamicHullState`, `SetSessionMarker`,
`IgnoreTeleportToBreadcrumbOnNextNIS` (1 each). All but `DidPlayerBetMax` are ported now.
- `DynamicHull.SetupDynamicHullState(name, state)` `0x82db41f8`: `LookupValues[StringToKey(name)] =
  GetHashValue32(state)`; a loaded hull of that key with another state is unloaded first
  (`cPathedHullLoader::UnloadHull`). `TestForDynamicHullState` `0x82dab7b0`: `GetLookupValue(key) == hash`.
  `GetLookupValue` `0x82da6590` reads the activity's `tDynamicHullUnlock` (attribute `0x…58E806F8`) through
  `GetChallenge` (so the challenge must exist) and asks `Sk8ChallengeSystem::GetDynamicHullUnlockValue`
  `0x825a4560`, which answers only when the system's `GameModeFlags` (+0x78; the `IDynamicHullUnlock` base is
  at +0x34, `AllocateSubSystems` `0x825a9c7c`) is 2; in the career (4) the set state or `kInvalidHashValue32`.
  The port loads no dynamic hulls and reports each state as an output.
- Memcard: `Sk8ChallengeSystem::SaveData` `0x825a2ec0` → `cChallengeDynamicHullManager::SaveData` `0x82da97f8`:
  chunk `cChallengeDynamicHullManager::Data` version 3, `WriteUInt(size)` then `WriteData` (`0x8255fba0`: a
  length-prefixed tuple) of `PackHullStates(stream, false)` `0x82da4aa8`: version 3, game flags 0, count, each
  (u64 key, u32 value) in key order, then (−1, `0xFFFFFFFF`) up to 256 slots. `LoadData` `0x82db4330` /
  `UnpackHullStates` `0x82db3210` read the counted entries and `OverrideGameFlags`. `Reset` `0x82da63d8` keeps
  `LookupValues`. The savable follows "DYNAMICOBJECTS" and precedes "SignPost".
- `Service.DrainPoolViaProgression(name)` → `cServiceManager::DrainPoolViaProgression` `0x8289cc90`: an
  "Undrained" pool costs $200 without a board sponsor (`SubtractCash`, credit 0), becomes "Drained" and its
  challenge completes (`cSignPostManager::CompleteChallenge(name)` `0x825e3098` → `CompleteChallenge(key,
  false)` `0x825e1550`: signpost key, neutrino key, `WinChallenge`, `ProcessChallengeResults`). Its stats
  (24, 25, 28) and the `Pull_The_Plug` achievement (`GetNumDrainedPoolsInWorld` `0x8289c958` against
  `GetNumPoolsInWorld` `0x8289c890`) belong to the unported stats back end.
- `SessionMarker.SetSessionMarker()` `0x82da92d0` → `cChallengeTeleportManager::SetSessionMarker` `0x82da6e40`:
  `cMsgSetSessionMarker` (`0x9A96B5F7`) at the local actor's `CalcRespawnTransform` and
  `GetFootForwardWithFakie`. `Achievement.GrantAchievement(a, delay = 0)` `0x82885fe8` →
  `StatsBackEnd::GrantAchievement` `0x8255f580` (`Net::GrantAchievement` once, `mCompleted`).
  `NIS.IgnoreTeleportToBreadcrumbOnNextNIS` `0x8288bf80`. These three are outputs the session logs.
- `Progression.DidPlayerBetMax` `0x825c7320` reads `playerBetMax` (+0x404CC), written only by
  `cSignPostManager::SetPlayersBet` `0x825cfaf8` (the poster's `BetMore`/`BetLess` and Lua
  `ChallengeProgression.SetPlayersBet` `0x82890c60`): pot = (competitors + 1) × bet, `playerBetMax` when the bet
  reaches the "MaximumBet" setting. The constructor (`0x825d43d8`) zeroes +0x404C0..C8 but not +0x404CC. Left for
  the betting port.

## Presentation events 26/27/30/31/35 and `RevertChallengeByTitle` — `challenge/presentation.rs`, session `challenge_icons`

`PlayNextPresentationEvent` `0x82e07f38` continues at once after each of these:
- 26 `CompleteChallenge(arg1)` → `cSignPostManager::CompleteChallenge(name)` `0x825e3098` (see above).
- 27 `RevertChallenge(arg1)` → `cSignPostManager::RevertChallenge(name)` `0x825e4050`: `GetChallengeKeyFromName`,
  then `RevertCompletedChallengeMap` `0x825e2ef8`: only for a completed challenge; the order of the last
  completed entry (key order) whose `ChallengeID` (layout word 12) matches, 2049 when none; the entry erased,
  `--intCompletedChallengeCount`, every remaining entry at or above that order moved down, then
  `ActivateChallenge(key, isNeutrinoKey 0, evalActivities 1, force 0, isNew 1)` (`0x825e306c`..). The progression
  native `Progression.RevertChallengeByTitle` `0x825e40f0` is the same through `GetChallengeKeyFromTitle`; the
  port used to drop only the completed entry (no order shuffle, count or re-activation).
- 30 `AddChallengeMapIcon(arg1, arg2)`: `RemoveChallengeMapIcon` `0x82e06c38` (the director's `mMapIcons`, keyed by
  `GetHashValue32(arg1)`: `MapBackEndManager::RemoveChallengeObjectIcon` `0x82572fa0` → `MapIconManager::RemoveIcon`),
  then `MapBackEndManager::AddChallengeObjectIcon(icon, location)` `0x825781f8`: the last `fe_map_icons` collection
  (`0xf788504a8d922ced`, sorted keys) whose layout word 0 is the icon name, the location's position
  (`cLocationManager::GetLocationPosition`); x = z = 0 adds nothing. `MapIconManager::AddIcon` `0x82634548` without
  an activity: texture `RwHash64String(name + ".Texture")`, rotation 0, scale 1, no pinch point; the `MapIcon`
  constructor `0x82601a50` leaves `mChallengeMapVisible` (+0x44) off and `mMiniMapVisible` (+0x45), `mFadeAtEdge`
  and `mbVisible` on. Three collections are named `mini_teleport`; the last (`0x966ca3d2fbf9b293`) has range 500.
  m_1a_01's presentation adds `mini_teleport` at `m_1a_01_goto_slappys_challengelocator01`
  (`mini_teleport.Texture` is in `images/icons/minimap.rx2`). The session keeps the icons and the minimap draws
  them with its dynamic icons.
- 31 `RemoveChallengeMapIcon(arg1)`.
- 35 `IncrementPointsPoolPoints(arg1, atol(arg2))` → `cSignPostManager::IncrementPointsPoolPoints(name, value)`
  `0x825ddf68`: `GetPointPoolKeyFromName`, then the key form with credit 0.

## Checks

- `cargo check --release -p skate2-pc -j 2`: PASS (no errors; pre-existing warnings only).
- `cargo check -p skate2-core --lib --tests -j 2` and `cargo check -p skate2-pc -j 2` after the teleport
  manager: PASS.
- `cargo test -p skate2-core --lib challenge:: -- --include-ignored --test-threads 1` with the game data:
  27 passed, 0 failed (2026-10-03).
- After the tutorial callout HUD, StartInGame groups and presentation central messages: the same core run,
  27 passed, 0 failed; `cargo test -p skate2-pc -- --include-ignored marker_ui`: 13 passed, 0 failed
  (`original_tutorial_movie_shows_tips_animations_and_trick_guides` new; the score movie's distance
  assertion now expects `37.5 ft`: `FormatNumber` precision 1 (r6 = 1 at `0x826b1f9c`) before
  `ProcessDistance`'s ×3.0, where it expected the old port's rounded `39`).
- After the sign-up prompt screen, `SortChallenges` and the progression dialog: core 28 passed, 0 failed
  (`new_career_declining_the_basics_skips_the_fmv` new; `sign_up_starts_the_next_mission` checks the
  `SignUpHUD` screen states); PC `marker_ui` 14 passed, 0 failed
  (`original_action_overlay_signs_up_and_hides` new).
- After the sign-up poster: core 28 passed, 0 failed (`sign_up_starts_the_next_mission` checks the poster
  natives for `m_1a_03`; the shared sign-up helper checks the half-speed `cMsgSetSimRate` after the intro and
  none after accepting); PC `marker_ui` 15 passed, 0 failed (`original_signup_poster_lays_out_the_tutorial_challenge`:
  header at (0, 0) on its large frames, description at 55, the tutorial option at 525, Accept/Decline at 577,
  `IntroComplete` 55 frames after the intro starts, `OutroComplete` 12 frames after the outro).
- After message types 4/5/7/16 and the dynamic hulls: `progression_messages_speak_show_skate101_and_edit_the_blog`
  (NPC speech 40/5241, Skate 101 view, cell-phone setting, BLOG add/remove with the order shuffle, user news
  add/remove), `dynamic_hull` pack/unpack round trip; PC `original_skate101_shows_the_title_text_and_button`
  (title through `format_custom(…, "x")`, description, button A, `IntroComplete` after 13 updates, unload 14
  updates after the outro).
- After the unlock overlay: core `challenge::` 29 passed, 0 failed (`fliptrick_tutorial_completes_and_unlocks`
  checks what the overlay reads: Reda, the $300 wallet, no cash award, the three activated challenges in
  `unlockedChallenges`; `cas_unlock_parts` unit test); `ui::apt_projection` spin test; PC `marker_ui` 16 passed,
  0 failed (`original_unlock_overlay_counts_the_rewards_and_cash`: `101-intro` layout, the zero cash lines hidden,
  `$300` wallet, the right-justified Proceed, `IntroComplete` on frame 9, `Cash_SetCashAnimDone` on update 104
  (30-update delay, Reda's `icon_update` frames 37..86, two empty slots, the logo's `iconintro` 10..21, five items
  off, four cash lines), the spin's intro/idle/outro, `OutroComplete` 9 frames after the outro, and
  `numMainRewards` carried to the next overlay).
- `ObjectiveGameInfo.SetFractionValue`/`GetFractionValue` (`0x82d48960`/`0x82d489e8`: `mFractionNumerator`
  +0x504, `mDirty` +0x628) and `ObjectiveInfo.GetTotalNumCompletedObjectives` (`0x82d494c8` →
  `GetNumCompletedObjectives` `0x82d43e10`, the fraction denominator `0x2b245ec6d20a64aa`) are bound;
  `m_2a_01_tutorial_slappy` needed them.
- New tests: `fliptrick_tutorial_completes_and_unlocks` (sign-up → teleport fade → five tricks → win →
  trigger → overlay → NIS → autosave; Reda unlocked and new; next three challenges available),
  `unlockables_defaults_and_round_trip`, `unlockables::tests::string_hash_lowers_and_starts_from_minus_one`.

## Not done

Sponsor/coverage screens, NIS playback (media-off
stand-in), skater +0x214 pin branch, `UpdateUnlockStats` callers elsewhere, the unlockables'
consumers (CAC clothing locks still use the editor's unlock-all policy; map/freeskate lists).
Presentation events still unpresented: 17/18 HoM effect, 19 replay, 22/24/25 modified/custom central
messages, 28 achievements, 29 NIS preload, 32 breadcrumb teleport, 33 dynamic hull state, 34 DMO clear
(enum from `PlayNextPresentationEvent`'s switch); 6/7 pin is an output the simulation does not apply yet.
The teleport's session-marker move is logged, not applied.
