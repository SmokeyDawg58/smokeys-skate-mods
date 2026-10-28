# New career start position (2026-10-09)

A new career used to show the skater at the training facility (`Z_TrainingStart_Fixed`, where the
front end's world parks the first player) while the progression's go-to NIS and the "New to
skate?" popup ran; the first mission then repositioned him to `J_Gamestart`. The native game
spawns him at `J_Gamestart` when the career's game loads, before the progression runs. Every game
start's spawn also gives the skater a new breadcrumb manager and, outside Party Play, puts the
session marker at the spawn, and the go-to NIS re-places him on his suggested breadcrumb when the
area is loaded. All addresses are `sk82_na_m.xex` (Jan 17 2009) VAs read from the live IDA
database, read-only.

## Native flow

| Step | Native | What it does |
| --- | --- | --- |
| Main menu, New Career | `FrontEndState_MainMenu::StartSinglePlayer` 0x82678c20 | `GM_SinglePlayer`; with `ProfileBackEnd` `mNewProfile` set and no `kbDemoMarketing`: `FELocationManager::GetLocations(eFELocationType_NewCareer)` (asserts a location, festate_mainmenu.cpp:536), the first one's `fe_locations` layout text 1 to `CommonBackEnd::SetLocationName` 0x82559d00 (copies `sGameInfo.locationName`, stores `cLocationManager::GetLocationPosition` at `sGameInfo`+432), `EnableLetterBox(0)`, then `EnterGame`. |
| Location list | `gFELocationTypes` 0x82fdb2d0, `GetLocations` 0x826354e8, `AddLocationsFromGroup` 0x82635018, `IsValidLocation` 0x82615998 | `[eFELocationType_NewCareer]` is `fe_location_group` 0x17006FCF96F17E1F (`[FreeSkate]` 0xF17B6509256EE58B). The group's locations (the unlock test only for the FreeSkate type), then its leaf descendants', each once and only with a Sim stream (`<world>_Sim.xsm`). The NewCareer group holds only `gamestart` (0x2E33DBE1D6B33472), whose layout text 1 is `J_Gamestart`. |
| Intro | `EnterGame` 0x826788b0, `StateMainMenu::NewCareerIntro` 0x82763790, `RedaIntro` 0x8275f5f8, `NewCAC` 0x82757ba8 | Flow message 28; movie 0x5D7E000030F27698 (`god_intro1`), movie 0x1F9000001F90D8BA (`RedaCarRide`, `data\movies\RedaCarRide_english_ntsc.vp6`, `MoviePlayer::Init` 0x826a3c24), then the editor (FE state 19, `eNewCareerSubState`). The car ride is this FMV: the in-engine `RedaCarRide_01`..`08` scenes of `scene_000.bin` (type 18, `SCENE_TYPE_STORY`; `_08` at (1501.5, 112.9, -733.8), 14 m from `J_Gamestart`) have no caller in the executable or the shipped vaults. |
| Editor closes | `FrontEndState_CaC::OnOutroDone` 0x82649348, `FrontEndManager::StartGameModule` 0x82633da0 | `SetEnableProgression(gameMode == GM_SinglePlayer)`, flow `MESSAGE_EnterGame`: the game module loads. |
| Game load | `GameModule::GameLoad` 0x8273be50 | Without `locationStartUpSet` (no saved orientation): `cLocationManager::GetSpawnLocation(locationName)` (0x82a8d200 by name, 0x82a8dce8 by hash), `MoveStreamFocus` there (0x8273c724, flags 0xFF); with it, the focus goes to the saved transform, and Party Play's to its location's spawn. The load then waits until `IsStreamFocusLoaded(7, kFocusLoadType_FELoad)` (0x8273c8a4) and makes the function's only `ResetGame` call, `ResetGame(newGame 1, useLocationSpawner 1, seed 0)` (0x8273c8c4), for a new career, a continued one, free skate and Party Play alike. |
| Reset | `GameModule::ResetGame` 0x8273a680, `PlayerUI::OnReset` 0x828aadd0, `PlayerUI::OnResetPos` 0x828aa428 | A new game gets a new `SessionMarker` from `CreateSessionMarker` 0x82729c28 (none for `GM_PartyPlay`, nor for a `GM_OnlineChallenge` whose net type is Own the Spot, S.K.A.T.E. or Death Race); the `SessionMarker` constructor 0x828adab8 leaves it not valid. An offline game's `PlayerUI` is player 0 with that marker; Party Play's `PartyPlayUI` 0x828aeae0 builds its `PlayerUI` with none. Single player with `locationStartUpSet` (a continued career's saved orientation) calls `OnResetPos(transform)` (`mLastLocation`, key -1); otherwise `OnReset(&GetHashValue32(locationName), -1)` keeps the key. Both set `mResetPlayer`. |
| Spawn | `PlayerUI::Update` 0x828af9f8, `PlayerUI::ResetPlayer` 0x828ad280, `SkaterManager::OnCommand` 0x82600920 case 3 | The game's first `PlayerUI::Update` (after its input command, once the spawner is ready) sends Command 3 `PlayerSpawnData` for `GetSpawnLocation(mLocationKey, 0)` (or `mLastLocation`); the simulation runs it in that frame's update, before `Sk8ChallengeSystem::Update`: `AdjustSpawnPosition` 0x825f1a20 (line test from +0.75 to -500, hit + 0.15), `DestroyPlayer`, `SpawnPlayer` 0x82600630 (a new `Actor` at the adjusted transform). |
| Spawn marker | `PlayerUI::ResetPlayer` 0x828ad280, `SessionMarker::SetSessionMarker` 0x828acf30 | After the spawn command and `ReloadAttributeData` 0x828aa470 (`mFootForward` from `IsPlayerStanceRegular`): with a marker that is not valid (a new game's never is), `SetSessionMarker(spawn, mFootForward, force 1, on board 1)` at the unadjusted spawn transform (forced, it always succeeds and sets `mValid`), then `cMsgSetMarker` (kind 0xF90E4922) with `mPlayerIndex` on the Simulation's broadcaster. |
| Progression | `Sk8ChallengeSystem::Update`, `cSignPostManager::StartGameSession`/`Update` (docs/career/runtime.md) | The toggle starts the signpost the next frame; `Start State Machine`'s `NIS` state sends `Goto_m_1a_01_goto_slappys_outro_1` (with `Hide Set/Use Session Marker`, the debug-flow locks, `Show MiniMap`, `PreloadChallengeAssetsByName("m_1a_01_goto_slappys")`). |
| NIS | `cSignPostUtil::HandleCaughtMessage` 0x825de4e0 type 2, `NISManager::RunBasicNIS` 0x828a8d70 | `RunBasicNIS(scene, transition, 0, 1)`: `TeleportToLastBreadCrumb`, the HUD and NIS load-screen transition, `PauseSim`. The scene is `SCENE_TYPE_GOTO` (26), 5 s, no characters. Its `cMsgNISChallengeNotify` moves the machine to `MsgBox`, which sends `Dialog - New To Skate`. |
| First mission | `m_1a_01_goto_slappys` start handler (ChallengeCore) | Unless `IgnoreInitialTutorialReposition`, `AI.SkaterDistanceFromLocation(LocalPlayerID, StartLocation) > 10` repositions the skater to `StartLocation` (`J_Gamestart`); otherwise it prints "No teleport was required". |

`J_Gamestart` is a global locator (`data\content\global_locators\DIST_Project_[0xf685aefdb7dc4187].rx2`,
loaded at boot by `cGlobalFileLocationManager::LoadGlobalLocations`, so `GetSpawnLocation` finds it
before anything streams), at (1491.81, 109.96, -743.31) with x axis (0.766, 0, -0.643) and z axis
(0.643, 0, 0.766), next to the alley behind Slappy's that the first mission starts in. The training
facility start is about 4.2 km away (`Z_TrainingStart`, FE_Main locator, (-2646.87, 129.72, -101.56)).

### The go-to NIS's breadcrumb teleport

`TeleportToLastBreadCrumb` 0x828a0db8 (story and go-to scenes, unless ignored) calls
`TeleportLocalToLastBreadcrumb` 0x828a93a8 only when `TeleportLocalToLastBreadcrumbRequiresLoadScreen`
0x828a9350 is false: `TeleportToLastBreadcrumbRequiresLoadScreen` 0x828a9800 takes the local Actor's
`GetSuggestedRespawnPosition` 0x82ce2698 and `TeleportRequiresLoadScreen` 0x828a9420 answers. Without
a load screen, `TeleportToLastBreadcrumb` 0x828a9158 asks for the same crumb again and
`TeleportPlayer` 0x828a9888 (no AI controller on the local Actor, not a network game, the same
load-screen test) sends `cMsgTeleport` on the board unless the crumb is off it. `RunBasicNIS` pauses
the simulation right after; `SimController::Update1` 0x828a72c8 still dispatches the message, and
`PlayerUI::ReceiveTeleportMsg` 0x828ad120 queues Command 5 (`PlayerTeleportData` with the UI's
`mFootForward`), but commands run only unpaused, so the skater moves as the NIS ends.

`TeleportRequiresLoadScreen` 0x828a9420 (no large load radius here): true when
`GameBackEnd::RequiresAdditionalWorldOperation` 0x8255b898 (an additional world location to load or
unload), `sGameInfo.startChallengeDuringLoad`, or `cGameAssetManager::IsAreaLoaded` 0x824e0768 fails
for the simulation focus within `kMinSimRadiusForNoLoadScreenOnTelelport` (flags 2; cvar default 20,
range 1..400, initializer 0x82f73848) or the presentation focus within
`kMinPresRadiusForNoLoadScreenOnTelelport` (flags 1; default 30, initializer 0x82f738a8). A missing
focus passes. `cAssetStreamSystem::IsAreaLoaded` 0x824e0340 (focus, point, radius):

- `GetAreaTestPoints` 0x824d7460: eight points, the square's corners and edge midpoints at the
  radius on x and z ((r, r), (r, 0), (r, -r), (0, r), (0, -r), (-r, r), (-r, 0), (-r, -r)); not the
  centre.
- `GetCollectionId` 0x824d73b0 per point: `RwHash64String("c<Sim|Pres>_<x>_<z>_high")` of the 200 m
  cell (truncated x and z, `200 * (v / 200) ± 100`); each id once (64-bit compare).
- `IsCollectionLoaded(id, true)` 0x824daa58: an id the stream system does not know counts as loaded
  (it knows only level-0 collections of its stream files: `cStreamFile::Update` 0x824e0ba8 makes no
  octree entry for the others); a known one is loaded when `tOctreeInfo::mActivation` is 2 (both
  `cCollectionLoadedCallback`s of `LoadComplete` 0x824dc208 done).
- For the first id that is not loaded, `FindCollections` 0x824dfea8 runs once on a copy of the
  focus's info moved to the point (the stream files' `cStreamFile::FindCollections`, sorted, capped by
  `kPresMaxFocusCollections` or, without stream XML, `kSimMaxFocusCollections`); the area fails when
  an unloaded id is one of them.

Right after a spawn, the new Actor's `BreadCrumbManager::Implementation` 0x82ce33b8 drops one crumb at
the (adjusted) spawn through `DropBreadCrumb` 0x82ce1fd0 (with `GetFootForwardWithFakie`), leaves
the override unset (RIGHT, on board), and starts `mLastKnownOrientation` as identity axes (w 0) with
the spawn position at +0x120. `Implementation::Update` 0x82ce2230 copies the hips there every update
but drops crumbs and refreshes the orientation (`_TestForPhysicsConstraints` 0x82ce1880, states
100/104) only from `kMinTicksAfterResetForBreadCrumb` (default 120) ticks after a reset, away from
the newest crumb. `GetSuggestedRespawnPosition` 0x82ce2698 tries `GetWipeoutSpotBreadCrumb`
0x82ce1cb8 first (`kEnableWipeoutSpotBreadCrumb` default on: `mLastKnownOrientation`'s axes at the
last known position, so identity axes at the hips until the orientation refreshes), which must pass
`_TestForCollisionConstraints` 0x82ce19b0, `_TestForNavMeshConstraints` 0x82ce14d0 (a nav-mesh cell
score under 1000) and `TestForLWRespawnContraints` 0x82ce1320 (no vehicle within 2 m, no pedestrian
within 1 m, no dynamic object across the capsule). The collision test runs a 10 m line down from 0.1
m above the point, whose hit must have a normal y of at least `skater_breadcrumbs`' 0.75 and lie at
most its `CollisionMaxVerticalDistance` (1.0 m) below the line's start, on an allowed surface, with
the line test of the capsule above it clear (`CapsuleRadiusForCollisionTest` 0.7,
`CapsuleHeightForCollisionTest` 0.4). Next comes `GetBreadCrumb` 0x82ce2428, the newest crumb that
passes `TestForLWRespawnContraints`, else `GetLastResortBreadCrumb` 0x82ce2598, the newest crumb (or
the spawn without one).

At `J_Gamestart` the wipeout spot fails the vertical distance. When the go-to NIS asks, the port's
skater, standing on his board, has his hips at (1491.69, 110.46305, -743.35944), 0.902 m above the
flat ground there (y 109.561005, normal y 1, surface class 2), so the line starts 1.0020447 m above
its hit. `GetBreadCrumb` then returns the ring's only crumb, the spawn's, in the empty alley: the
teleport keeps the skater where he spawned (the adjusted spawn, ground + 0.15) with `J_Gamestart`'s
heading, z axis (0.643, 0, 0.766). The margin is 2 mm: hips 2 mm lower when the NIS asks would pass
this test (the nav-mesh and living-world tests were not traced for the wipeout spot) and give
identity axes at the hips, facing world +Z, about 40 degrees from `J_Gamestart`'s heading, as the
first version of this note assumed. Both skaters spawn 0.15 m above the ground (`AdjustSpawnPosition`);
how far the native one has settled when its NIS asks was not measured.

## Port

| File | Change |
| --- | --- |
| `crates/skate2-core/src/ui/freeskate_option.rs` | `GetLocations` per location type (`type_locations`); `new_career_locations` for `eFELocationType_NewCareer` (`NEW_CAREER`), without the unlock test. The FreeSkate list is unchanged. |
| `crates/skate2-core/src/session/selected_session_career.rs` | `start_career` places the career's skater by the map's travel (`travel_to_career_start`: on the board, the fade covered while the simulation and presentation stream the spawn in, `career_spawn_pending` set): a new career at the first NewCareer location's `GetSpawnLocation(hash, 0)` (`place_new_career`), a continued one at the saved orientation or `J_Gamestart` (`place_continued_career`, which used to teleport while revealing). `NisEvent::TeleportToLastBreadcrumb` now asks for `try_nis_breadcrumb_teleport` (the crumb, `teleport_requires_load_screen`, `send_teleport`); `local_suggested_breadcrumb` returns the whole crumb. Traced under `SKATE2_TRACE_CAREER` ("new career starts at"; "NIS breadcrumb" with the hips and the wipeout spot's ground line, `wipeout_ground_line`). |
| `crates/skate2-core/src/session/selected_session_public_facade.rs` | `career_spawn_pending`: the completed ticks skip the career update and the replay frame until the travel has landed. A game start's travel (`map_travel_on_board`) puts the session marker at the spawn when the mode has one and sends `cMsgSetMarker` at its dispatch (traced under `SKATE2_TRACE_SESSION_MARKER`), and resets the kept Actor's breadcrumbs when it lands. The NIS breadcrumb teleport runs after the career update, paused or not. The main menu's Continue no longer reveals before the travel. `set_presentation_stream_area` keeps the renderer's presentation stream. |
| `crates/skate2-core/src/session/selected_session_edit_skater.rs` | `finish_edit_skater` reveals the front end only without a travel; the travel's completion reveals the new career. |
| `crates/skate2-core/src/streaming/area_loaded.rs` (new), `streaming/streaming.rs` | `area_test_points`, `high_collection_id`, the generic `is_area_loaded`, and `PresentationStreamArea` (the renderer's presentation XSS, stream XML, radius and registered collections) with its `is_area_loaded`. |
| `crates/skate2-core/src/session/selected_session_terrain.rs`, `selected_session_terrain_input_extension.rs`, `streaming/selected_terrain_startup.rs` | `try_area_loaded` for the simulation focus (level-0 XSS collections; loaded once registered; `FindCollections` is the focus's own selection, now `focus_selection`); `collection_registered`. |
| `crates/skate2-core/src/input/tuning/input_cvar_runtime.rs` | Cvars 1247/1248: `kMinSimRadiusForNoLoadScreenOnTelelport`, `kMinPresRadiusForNoLoadScreenOnTelelport`. |
| `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs` | `respawn`: the state the constructor gives a new Actor's manager. |
| `apps/skate2-pc/src/city_streaming.rs`, `render_inspection.rs`, `live_skating_view.rs`, `live_skating_client.rs` | The presentation stream (XML now shared) and its registered high collections go to the session after each redraw; under `SKATE2_DIAGNOSTIC_CAREER_YES`, the popup's opening tick and the skater's (x, z) are printed. |

`RequiresAdditionalWorldOperation` is false in the port, which loads no additional world location.
The port keeps its Actor through a game start (no `DestroyPlayer`/`SpawnPlayer`), so the spawn's
other new-Actor state stays as the teleport leaves it; only the breadcrumbs are reset. A game
start's session marker is the one `try_reset_game` made for the mode when the game was chosen.

## Verification

- `cargo build --release -p skate2-pc` builds.
- New career (`career_run.sh` with `SKATE2_DIAGNOSTIC_MOVIE_SKIP=120 SKATE2_DIAGNOSTIC_CAREER_YES=1`,
  fresh scratch saves):
  - First fix (`nc1`): `career: new career starts at J_Gamestart [1491.8145, 109.96031, -743.30505]`
    (the global locator's spawn 0, the same transform `BAM_Sim.loc` lists); `diagnostic: popup open
    at 2148, skater (x, z) Some([1491.8147, -743.3056])`; after Yes, `m_1a_01` starts at the same
    spot with no `TeleportPlayer`. Before the change (the same demo's `cash1` run earlier today) the
    skater stood at the training facility through the NIS and the popup, and `m_1a_01` sent
    `TeleportPlayer { location: "J_Gamestart" }` after `challenge_running`.
  - With the breadcrumb teleport (`nc2`, captures every second): black while the travel streams
    `J_Gamestart`, the reveal into the alley as the go-to NIS starts, its white transition, its
    shots, the white transition out, then the popup over the skater in the alley facing down it, and
    after Yes the mission start at the same spot and heading. The trace: `career: NIS breadcrumb
    [1491.8145, 109.711006, -743.30505] facing [0.6427878, 0.0, 0.76604426] on board true` (no load
    screen; the spawn crumb, `J_Gamestart`'s heading, on the ground `AdjustSpawnPosition` found) and
    `popup open at 2262, skater (x, z) Some([1491.8147, -743.3056])`.
  - With the spawn marker (`nc3`): `session marker: set at the spawn [1491.8145, 109.96031,
    -743.30505]`, then the same breadcrumb and popup lines.
  - With the hips and the ground line in the trace (`nc5`): `career: NIS breadcrumb [1491.8145,
    109.711006, -743.30505] facing [0.6427878, 0.0, 0.76604426] on board true (hips
    Some([1491.69, 110.46305, -743.35944]); ground line from y 110.56305 hits y 109.561005 (drop
    1.0020447, normal y 1, surface 2))`, then `popup open at 2248, skater (x, z)
    Some([1491.8147, -743.3056])`.
- Continue Career on scratch copies of a save with three completed challenges: `career: continue at
  [1670.4205, 102.41446, -521.4711] (saved orientation true)`; the screen stays black while the
  travel streams Slappy's, then shows the skater there; `session marker: set at the spawn
  [1670.4205, 102.41446, -521.4711]`.
- Free skate (`main-menu-freeskate`, `SKATE2_DIAGNOSTIC_FREESKATE_LOCATION=slappy`, starting at
  `ID_LOCATION_SLAPPYS`): `session marker: set at the spawn [1682.4763, 104.88679, -535.1605]`.
- Party Play (`main-menu-freeskate`, `SKATE2_DIAGNOSTIC_PARTY_PLAY=3200`, S.K.A.T.E. at the snakes
  spot): the game starts and its players spawn; no session marker is set (the port's
  `try_reset_game` makes none for `GM_PartyPlay`, as `CreateSessionMarker` does).
- `parity.sh`: PARITY IDENTICAL (6000 lines) after the first fix, after the breadcrumb teleport,
  after the spawn marker and with the final traces; the baseline was not touched.
- Unit tests in `streaming/area_loaded.rs` (test points; an unloaded cell failing only when the
  focus there wants it) were written but not run (no test build).

## Open

- The rest of a new Actor's state at a game start (the port keeps its skater) beyond the
  breadcrumbs.
- The go-to NIS's heading at `J_Gamestart` rests on a 2 mm margin (above); the native skater's hips
  at the tick its NIS asks were not measured.
- The presentation side of `IsAreaLoaded` reads the renderer's stream as of its last redraw.
