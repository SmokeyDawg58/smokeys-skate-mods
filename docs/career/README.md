# Skate 2 career, missions and challenges — reconstruction plan

Status (2026-10-02): **implemented through the second mission's start** — New Career, the first
mission, saving on completion, Continue Career and the challenge sign-up run in the port. The
current state, verification and next steps are in [HANDOFF.md](HANDOFF.md). Facts come from the debug executable `sk82_na_m.xex` in IDA (read-only), its
PDB class layouts, and the Jan-17-2009 data (`data/db/*.vlt|bin`, `data/fe/screens.big`,
`data/movies`). Addresses are debug-xex virtual addresses.

| Document | Contents |
| --- | --- |
| [HANDOFF.md](HANDOFF.md) | **Start here**: port status, code map, native facts, next steps |
| [README.md](README.md) | Architecture, new-career flow, first mission, implementation plan |
| [data.md](data.md) | Vault classes and records, Lua chunk format, research tools |

## 1. Architecture

Career progression, missions and challenges are **Lua 5.1 state machines** whose definitions
and compiled chunks are Vault records. Native code runs the Lua VM, routes messages, and
implements about 50 binding modules the scripts call.

```
 FrontEnd / Flow (C++)            StateMainMenu, FrontEndState_MainMenu, FrontEndState_CaC
   │ new profile → movies → CAS → StartGameModule with progression enabled
   ▼
 Sk8::Challenge::Sk8ChallengeSystem (C++, ~2,900 functions)
   cActivityManager · cSignPostManager · managers for HUD, AI, DMO, living world, HoM,
   teleport, presentation, objectives, timers, races, contests, film, photo, online …
   │ messages (cMsgStateEnter, cMsgHUDYes, cMsgTimerExpired, …) → Lua handlers
   ▼
 ChallengeRuntime (C++)           cLuaStateManager, cStateGraph/cState, cActivity,
   │                              cLuaAttributes (Vault → Lua metatables), PersistentDataSerializer
   ▼
 Lua 5.1 (stock, embedded)        lua_Number = 4-byte float; chunks little-endian
   progression: 910 chunks (progressioncore.bin)   challenges: 627 chunks (ChallengeCore.bin)
```

Embedded Lua is stock 5.1 (`luaopen_base/table/string/math/os/io/debug`, coroutines present).
The shipped chunks only use: MOVE LOADK LOADBOOL LOADNIL GETGLOBAL GETTABLE SETGLOBAL SETTABLE
NEWTABLE ADD SUB MUL DIV NOT CONCAT JMP EQ LT LE TEST CALL RETURN FORPREP FORLOOP SETLIST CLOSURE.
No upvalues, method calls, varargs, tail calls or generic `for`.

## 2. New-career flow (native)

1. **Main menu label.** `mainmenu.apt` asks `ProfileCmdTranslator::sMethod_IsNewProfile`
   (`0x82604118`) and shows `ID_MAINMENU_NEWCAREER` for a new profile, otherwise
   `ID_MAINMENU_CONTINUECAREER`. The flag is `ProfileBackEnd::mProfileData.mNewProfile`
   (`ProfileBackEnd+0x68`).
2. **Row 0.** `FrontEndState_MainMenu::OnMenuSelectGame` (`0x82681338`) → `StartSinglePlayer`
   (`0x82678c20`): sets `gameMode = GM_SinglePlayer`. With `mNewProfile` set it selects the
   `eFELocationType_NewCareer` FE location as the start location and disables the letterbox;
   then `EnterGame` (`0x826788b0`) sends flow message `0x1C` instead of loading the world.
   Without it, `EnterGame` uses `SignPostManager.ProgressionOrientation` when valid, else the FE
   location, loads the CAS character and starts the game.
3. **Flow message `0x1C`** sets `StateMainMenu.mDoEnterCACFromMainMenu`; `InFEState`
   (`0x82777c50`) transitions to `NewCareerIntro`.
4. **`NewCareerIntro`** (`0x82763790`): plays FE movie key `0x5D7E000030F27698` (FE state 28,
   forced black on close), then → `RedaIntro`.
5. **`RedaIntro`** (`0x8275f5f8`): plays movie key `0x1F9000001F90D8BA`
   (`RedaCarRide_english_ntsc.vp6` expected), then → `NewCAC`.
6. **`NewCAC`** (`0x82757ba8`): Create-A-Character in `eNewCareerSubState` (FE state 19).
7. **CAS exit** `FrontEndState_CaC::Shutdown` (`0x82659d28`): for new-career sub-states 1/2 in
   single player, `Sk8ChallengeSystem::SetEnableProgression(1)` and `mNewProfile = 0`.
   `OnOutroDone` (`0x82649348`) → `FrontEndManager::StartGameModule`.
8. **Gameplay starts** with progression enabled: `GameModule::GameLoad` (`0x8273be50`) streams in
   the start location (`gamestart`'s `J_Gamestart`) and the game's first `PlayerUI::ResetPlayer`
   (`0x828ad280`) spawns the skater there, before the challenge system runs (see
   `docs/evidence/new-career-start-position-2026-10-09.md`); the only `Starting` progression state
   machine, **"Start State Machine"**, runs.

Movies are EA VP6 (`MVhd` container, `vp60`, 1280×720). The port plays them: see
`docs/evidence/career-fmv-playback-2026-10-09.md`.

## 3. Start State Machine and the first mission

`Start State Machine` (progressioncore, 6 states), initial state `Initial`:

| State | Message | Handler (traced) |
| --- | --- | --- |
| Initial | StateEnter | if `m_1a_01_goto_slappys` active → deactivate; → `Activation` |
| Activation | StateEnter | → `NIS` |
| NIS | StateEnter | restores unlock/lock state from completed challenges (existing career) or preloads `m_1a_01_goto_slappys` (new) |
| NIS | NISChallengeNotify | `m_1a_01_goto_slappys` complete → `Final`, else → `MsgBox` |
| MsgBox | StateEnter | `SendMessage('Dialog - New To Skate')` |
| MsgBox | HUDYes / HUDNo | Yes → `FMV_Basics`; No → activate + execute `m_1a_01_goto_slappys` → `Final` |
| FMV_Basics | StateEnter / FMVFinishedPlaying | `SendMessage('FMV - Tutorial Skate 01')`, then activate + execute the mission → `Final` |
| Final | StateEnter | `TerminateWithoutSave()` |

The traced handler lines are linear call lists; branch structure must be taken from a real
decompile before porting.

First mission **`m_1a_01_goto_slappys`** (`challenges`, key = `StringHash64(name)`): objectives
`…_push`, `…_minimap`, `…_duck`, …; state graph:

```
initial → make_hud ─hudloaded→ make_objectives → setup → asset_load ─assetloadcomplete→
challenge_running ─allobjectivecomplete→ show_results ─timerexpired(3s)→ WinChallenge →
play_pres_event ─presentationeventgroupfinished→ challenge_cleanup → complete (CompleteChallenge)
global: abandonchallenge → challenge_cleanup; restartchallenge → restarted → setup
```

Natives used by this mission: `Online.GetLocalPlayerID`; `Objectives.MakeObjectives /
SetEvaluationStatus / ResetObjectives / ResetObjective / DeleteObjectiveGroup`;
`HUD2.CreateGroup / GetGroupHash / SetupObjectiveHUDData / SetActiveGroup / Show / DeleteGroup /
ShowCentralMessage`; `Pause.Disable/EnablePauseMenuAndBackpack`; `VisualIndicators.CreateGroup /
AddIndicator / RemoveIndicator / DeleteGroup`; `ChallengePresentation.AddPresentationGroup /
AddPresentationEvent / StartPresentation / RemoveAllPresentationGroups`;
`HOM.EnableHOMVisuals`; `AI.SkaterDistanceFromLocation / RepositionAI`; `Effects.StartEffect`;
`ChallengeAsset.IsChallengeAssetLoadComplete / Activate/DeactivateChallengeAssetLoadScreen`;
`ChallengeTimer.AddCountdownTimer`; `Challenge.WinChallenge / CompleteChallenge`;
`table.getn`, `string.format`, `print`.

## 4. Scale

- Progression scripts read 14 globals (`Progression` 1,085 uses, `ChangeState`, `Terminate`,
  `TerminateWithoutSave`, `Unlock`, `Achievement`, `Award`, `ChallengeSpeech`, `DynamicHull`,
  `Service`, `Teleport`, `SessionMarker`, `ChallengeAsset`, `NIS`) and 63 fields.
- Challenge scripts read 61 globals (`HUD2` 816 uses, `ChangeState`, `FrontEnd`, `Challenge`,
  `ChallengeTimer`, `AI`, `Objectives`, `Online`, `VisualIndicators`, `ChallengeSpeech`,
  `HighlightReel`, `Pause`, `Trick`, `Race`, `ChallengeManager`, `Contest`, `NIS`, `Map`, …) and
  1,323 distinct fields.
- Vault: 233 progression state machines, 765 progression states, 910 progression handlers,
  396 progression messages, 717 progression challenge entries; 804 challenges, 246 challenge
  states, 627 challenge handlers, 1,475 objectives.

## 5. Implementation plan

Steps 1–5 are implemented (presentation pieces aside); step 6 is in progress. See
[HANDOFF.md](HANDOFF.md).

1. **Lua 5.1 VM** in Rust (bytecode loader + interpreter for the used opcodes, float numbers,
   Lua 5.1 table semantics, stdlib subset), plus a compiler for the progression handler
   sources (`challenge/lua/compiler.rs`).
2. **Vault database**: skater.vlt classes plus ChallengeCore/progressioncore collections, Lua
   metatable bindings for activity data (`cLuaAttributes`).
3. **Progression runtime**: state machines, message routing, `Progression`/`Unlock`/…
   natives, `progression_messages` (dialogs, FMVs, unlock screens), save/load.
4. **New-career front end**: `IsNewProfile` label, new-career CAS mode, NewCareer start
   location, `mNewProfile` persistence.
5. **Challenge runtime** for `m_1a_01_goto_slappys`: activities, state graphs, objectives,
   HUD2 objective HUD, presentation, completion → progression message → save.
6. Remaining challenge types and modules, then Party Play (`PartyPlayBackEnd`).

Presentation pieces that need their own subsystems: VP6 movie playback, NIS cutscenes, speech
and music (audio is a separate, paused workstream).
