# Career challenge characters and living world management (2026-10-09)

Native: `sk82_na_m.xex` (Jan-17-2009 debug), read-only IDA. Every address below was read in the
IDB (decompile, and disassembly wherever Hex-Rays dropped asserts, 64-bit arguments or vector
math). Data facts come from the shipped vaults (`skater.vlt`, `ChallengeCore.vlt`).

## Summary

- `Sk8::Challenge::cChallengeCharacterManager` (sub-controller 4) and
  `Sk8::Challenge::cChallengeLivingWorldManager` (sub-controller 13) are ported in
  `crates/skate2-core/src/challenge/characters.rs` and `challenge/living_world.rs`, wired into the
  activity manager's `GameLoad`/`ReevaluateActivityPrereqs`/`Update`/`RunMainActivity`/
  `CompleteChallenge`/`RestartChallenge` in native registration order.
- The managers' calls into the living world (`LW::Man`, `ILWCensusMan`, `RecipeMan`) are queued
  as `LivingWorldOp`s and applied by the career session after the challenge update
  (`session/selected_session_career_living_world.rs`), in call order. Reads go through
  `LivingWorldFacts` (waypoint groups, `IsLivingWorldModelTracked`, `ChaseIsOn`).
- `RecipeMan` can now load Marquee models (the `MarqueeLoader` name round trip), unload models
  (`UnloadLivingWorldModel` → DReg → `SimCharacterManager::Unload` → `OnUnloaded`), and the
  challenge manager receives the `cMsgAssetSimDRegLWSchema`.
- Every training-area character (Slappy, Giovanni Reda, Atiba Jefferson) is a marquee entity whose
  AI graph is `marquee.xml` or `Photographer.xml` (motion graphs `motiongraph_marquee.xml`,
  `Motiongraph_Photographer.xml`). Those four graphs are ported
  (`career-marquee-graphs-2026-10-09.md`), so the session's marquee load, spawn and release paths
  above run.

## 1. cChallengeCharacterManager (0x7430 bytes)

| Native | Address | Port (`challenge/characters.rs`) |
|---|---|---|
| `Init` | 0x825a9180 | `init` (with `cActivityManager::CreateActivity`'s `AddActivity` 0x825a80f8, debug-only activities skipped) |
| `Reset` | 0x825a8258 | `reset` |
| `GameLoad` | 0x825a2f58 | `game_load` (game types 4 and 2) |
| `GameUnload` | 0x825a9270 | `game_unload` |
| `ReevaluateActivityPrereqs` | 0x825acc48 | `reevaluate_activity_prereqs` (`IgnorePrereqForLivingWorld`) |
| `AddActivityEntries` | 0x825ac948 | `add_activity_entries` |
| `AddLivingWorldSpawnDataToTree` | 0x825ac378 | `add_spawn_data_to_tree` |
| `UpdatePendingBBoxCreation` | 0x825ac5b0 | `update_pending_bbox_creation` |
| `GenerateLivingWorldSpawnActivityBBox` | 0x825a46a0 | `spawn_bounds` |
| `Update` | 0x825ac8f8 | `update` |
| `SpotTestCharactersAgainstLocalPlayer` | 0x825ac7e8 | `spot_test_characters_against_local_player` |
| `SpotTestCharactersAgainstPoint` | 0x825ac8a8 | `spot_test_characters_against_point` |
| `TestBBoxAgainstTree` | 0x825ab590 | `test_bbox_against_tree` / `place_entry` |
| `IsAllowedToSpawnWithCurrentChallengeState` | 0x825a3160 | `is_allowed_to_spawn` |
| `AddEntityToLineTestBatch` | 0x825a2ff0 | `add_entity_to_line_test_batch` |
| `ReleaseSpawnData` / `ReleaseMapData` / `ReleaseModelData` | 0x825a7930 / 0x825a6bb8 / 0x825a4800 | same names |
| `RunMainActivity` | 0x825a58c8 | `run_main_activity` |
| `ForceCullChallengeSpawnedLivingWorld` | 0x825a85f0 | `force_cull_challenge_spawned_living_world` |
| `CompleteChallenge` / `ResetAlternateLocationCharacters` | 0x825a92d8 / 0x825a8a30 | same names |
| `ReceiveWaypointGroupAddedMessage` | 0x825a5798 | `receive_waypoint_group_added` |
| `ReceiveLocationAddedMessage` | 0x825a3150 | `receive_location_added` |
| `ReceiveDRegLWSchemaMsg` | 0x825a8470 | `receive_dreg_lw_schema` |
| `IsChallengeCharacter` | 0x825a6760 | `is_challenge_character` |
| `cManagedOctTree<tLivingWorldSpawnData,256>` `Insert`/`HasEntry`/`GetEntryIndex`/`Remove`/`GetCollisions`/`Reset` | 0x825a49b8 / 0x825a3238 / 0x825a32a0 / 0x825a7d00 / 0x825a7eb0 / 0x825a59b8 | `SpawnTree` |

Facts the port relies on:

- Character arrays: `challenges` 0x555c505a36c3353a `tGlobalSpawnCharacterInfo[10]` (entity
  RefSpec, `Location` text +24, `OnStartType` +28, `AlternateLocation` text +32; default area 0x28)
  and 0x0cbe4b5ee9580e8d `tSpawnLivingWorldOnWaypointInfo[10]` (entity, `WaypointID` +32,
  `OnStartType` +40; default area 0x30). `ActivityCharacterIndex` runs over the location entries,
  then the waypoint entries. 0x16e7e79b28332502 (Bool) hides other activities' characters while
  the challenge runs.
- Box: the `livingworld_census_ranges.pedestrians` minimum circle's cull radius when its speed is
  not below the maximum's, else the larger cull radius (shipped data: 70/45 and 90/80 → 90 m), plus
  10 m for a marquee model (`livingworld_models` 0xf6897cc7b637850c == 1).
- Tester box: `TriggerVolumeUtils::GenerateTesterBBox` 0x82e20ee0 (0.58 m wide, 1.74 m high).
- Ground test: `AcquireLineBatch("cChallengeCharacterManager", BF_GroundDmo)`, from 1.0 above to 2.5
  below the position, fatness 0.33 (`kChallengeSpawnLineTestOffsetsUp/Down/Fatness`, 0x82f55278 /
  0x82f552d8 / 0x82f55218); at most eight per batch; a hit owned by an assembly resets its dynamic
  object (`CanResetDynamicObject` 0x82d3a908, `ResetDynamicObject` 0x82d39e10) and counts as none.
- Spawn: `ILWCensusMan::SpawnPedestrian` (vtable +4) with initial false, allowCull false, force
  true, allowWorldCollisions true, ignoreNavMeshInGroundTest true, allowHandProp true; `RecipeID` is
  the pedestrian's `mRecipeID`; a waypoint character is put on the group's first waypoint
  (`Pedestrian::SetCurrentWaypoint` 0x82e6e600); with no main challenge `UseAlternateLocation`
  clears; with it set or the activity running, `tPedestrianStateStorage` +0xca9 |= 0x40, +0xca8 |=
  0x40, +0xca8 &= ~0x20. A marquee model loads first (`RecipeMan::LoadLivingWorldModel`); while
  `NISManager::mPreloadedSceneHash` is set, an untracked marquee model is not spawned.
- `kEnableChallengeSpawn` true, `kKillPrereqFailedCharacters` true (0x82f55128, 0x82f55178).

## 2. cChallengeLivingWorldManager (0xb8 bytes)

| Native | Address | Port (`challenge/living_world.rs`) |
|---|---|---|
| `GameLoad` | 0x825ad758 | `game_load` (activated for 4 and 16) |
| `GameUnload` | 0x825b6080 | `game_unload` |
| `SetLWManagementValues` | 0x825ad780 | `set_management_values` (census byte +0xa8 bits 0x40/0x20/0x10/0x08/0x04/0x02 from value bits 0..5) |
| `ReevaluateActivityPrereqs` | 0x825ad840 | `EnableCensusSpawner(true)` |
| `IsSignupAvailable` | 0x825ad858 | `is_signup_available` (no chase), used by `signup::is_allowed_to_sign_up` |
| `RunMainActivity` | 0x825c0da0 | `run_main_activity` |
| `CompleteChallenge` | 0x825b7640 | `complete_challenge` (value 0x3f, avoid list cleared) |
| `RestartChallenge` | 0x825bf220 | `restart_challenge` (`SkipInitialSpawn`) |
| `ExecuteLivingWorldOperations` / `ExecuteLivingWorldOperation` | 0x825bf148 / 0x825bde28 | same names |
| Lua `LivingWorld.ForceCullLivingWorld` | 0x82893ec0 | module userdata is the manager (`PerformLuaCBinding` 0x8289b498); `+0x14 LWManager->RequestCensusManagedCull()` |
| Lua `LivingWorld.GetIfPlayerBeingChased` | 0x82883f90 | `ChaseMan::ChaseIsOn` |

`ExecuteLivingWorldOperation`: type 0 culls every census-managed pedestrian and vehicle; types 1
and 2 look up `cTriggerVolumeManager::GetTriggerInstance(float(VolumeID))` and cull those
intersecting it only when it exists; type 2 also appends the 64-bit `VolumeID` to `AvoidVolumeList`
(no duplicate). Shipped examples: `c_1a_01_bowl_series` value 0x11 (census spawner and dynamic
objects only), `m_3a_01_pro_duffy` cull-all, `sponsor_me_03` avoid `0x000574fc03e38706`.

## 3. Living world side (session)

| Native | Address | Port |
|---|---|---|
| `Pedestrian::RequestExplicitCull` (INavEntity +0x24) | 0x82e90dd0 | `SetAllowCull(true)` then `LWEntityCensusMan<Pedestrian>::RequestCull` 0x8271a5e8 (`request_pedestrian_cull`) |
| `Pedestrian::SetAllowCull` (INavEntity +0x30) | 0x82e946b0 | core byte 240 bit 7 (`allow_culling`) |
| `Pedestrian::IsCensusManaged` | 0x82e6d408 | not `IsPedestrianInvolvedInChallenges` (the op carries `CharacterTreeMap` at the call) |
| `LW::Manager::RequestCensusManagedCull()` / `(volume)` | 0x826f5158 / 0x826f51b0 | census-managed pedestrians (0x82e993e0 / 0x82e99538) and vehicles (0x82d286b0 / 0x82d28818: core bit 0x80, vehicle census `RequestCull`) |
| `LWBasicEntity::IsIntersectingTriggerVolume` | 0x82e4c138 | model bbox volume at `LWModel::GetBBoxTransform` 0x82e4cd60, `IsColldingWithTriggerVolume` 0x82e213b8 (no model: false) |
| `LW::Manager::GetWaypointByGUID` | 0x82715b18 | `PropMan.waypoint_guids` → first waypoint's location |
| `ChaseMan::ChaseIsOn` | 0x826f3b48 | `ChaseMan::chase_is_on` |
| `NISManager::RunBasicNIS` | 0x828a8d70 | now clears the preloaded scene (`mPreloadedSceneHash = 0`) before `PauseSim` |

## 4. RecipeMan, the marquee loader and unloads

- `LoadLivingWorldModel(modelKey)` 0x82726210 → 0x827239e0 is `RecipeMan::load_key`;
  `IsLivingWorldModelTracked` 0x82712aa8 / 0x8270f610 is `RecipeMan::is_tracked` (new).
- Marquee requests: `PresCharacterManager::ReceiveSimCharacterLoadRequest` 0x8281b6f0 passes
  `MarqueeDefs::GetIDForName` 0x82cfe828 of the model name to `RecipeFactory::CreateMarqueeLoader`
  0x82814998, which takes `GetDefaultMarquee` 0x82cfdbe8 (`tutorial_male`) unless the
  `characters_marquee` layout byte 8 is set; `MarqueeLoader` 0x82d0b110 loads `GetSkaterName`
  0x82cfe920 of it with `CloneType_NoClone` (0) for a non-local request; `LoadImpl` 0x82d09ef0 is
  `LoadSchemaFromFile` in `gCategoryMapping[0]` ("marquee"). Ported: `begin_ambient` category 0 and
  `marquee_schema_name`. All shipped challenge characters map to themselves.
- `UnloadLivingWorldModel(name)` 0x82715278 / `(iterator)` 0x82712918: `cMsgAssetSimDRegLWSchema`
  (ModelMan `RemoveRecipe` 0x82ce8cd0 → `Manager::NotifyOfAssetRemoval` 0x826f5508: pedestrians,
  vehicles and props ignore it (LWBasicEntity slot 0x6c is the empty 0x825187c0),
  `DynamicObject::NotifyOfAssetRemoval` 0x82d2a400 releases a matching model; and
  `cChallengeCharacterManager::ReceiveDRegLWSchemaMsg`), `cMsgAssetPresDRegLWSchema`
  (`cLivingWorldPresModelManager::ReceiveDRegLWSchemaMsg` 0x8280dfe0 → presentation entities of the
  recipe destroyed, 0x8280cbe8), then `SimCharacterManager::Unload` 0x82817910; `OnUnloaded`
  0x828154a0 later. Ported in the session's recipe bridge.
- `SkaterAssetManager::IsRecipeUsedByLivingWorld` 0x824bb120 (`IsLivingWorldModelTracked` of the
  marquee name) now feeds the AI catalog's recipes in use, so ambient AI skaters avoid a marquee the
  living world holds.

## 5. Deviations and limits (port)

1. The managers call the living world synchronously; here the calls are queued and applied after the
   challenge update and flush, in call order, and a spawn's pedestrian is `Pending(token)` until the
   session answers. Line tests run at application time and are read by the next
   `TestBBoxAgainstTree`; a second test in the same update reads them as unanswered (native `Sync`).
2. Locations come from the `BAM_Sim.loc` table the challenge system already holds (as the sign-up
   and teleport managers read it), so location entries never wait. Waypoint entries wait for their
   group: the constructor (0x825a8f80..0x825a90c8) registers `ReceiveWaypointGroupAddedMessage` on
   the simulation broadcaster and the empty 0x825187c0 for `cMsgWaypointGroupRemoved`; the port's
   GameSim route keeps the added groups while a career subscribes and the career takes them before
   its update.
3. The `NISManager` preloaded-scene flag is read once per update after the outputs are applied.
4. An unload completes on the next RecipeMan update (selected schedule, like the loads' fixed
   delay); the native asset manager's timing is not deterministic.
5. The session registry keeps one entry per schema: a living world marquee load whose schema a
   skater (player, party, AI) holds is refused (`skater_holds_marquee`); natively they share it.
   A live pedestrian still built from a recipe whose presentation is removed is an error (its
   presentation destruction is not ported); the release order culls first.
6. `CharacterFaceManager::LoadFaceData` for marquee recipes (`OnComplete` 0x82d0a140) is not wired
   (the face manager is not live for any recipe yet).
7. `StateLoading::CharacterPrepareState`'s `SpotTestCharactersAgainstPoint` and its
   `SkipInitialSpawn` read are not run by the session (no loading state between missions); the
   first update's tester test places the characters.
8. `GameUnload` restores no census bits (native neither); the session applies the managers'
   `GameUnload` before the career goes.

## 6. Not ported / blocked

### Marquee and photographer graphs

Ported; see `career-marquee-graphs-2026-10-09.md`.

### Spectators

`LivingWorld.ActivateSpectatorWaypoint` 0x82893da8 / `DeactivateSpectatorWaypoint` 0x8288e488 →
`WaypointMan::ActivateSpectatorWaypoint` 0x82ce9180, `DeactivateSpectatorWaypoint` 0x82ce8ed8,
`UpdateSpectatorActivationInfo` 0x82ce71b8 (`kMaxNumForceSpectators`, category-group forced spawns
locked to waypoints). Used by the contest scripts (`ContestRounds`). Ported:
`living_world/pedestrians/pedestrian_spectators.rs` (the `WaypointMan` queue and the group's
activation), `session/living_world/selected_living_world_spectators.rs` (`WaypointMan::Update` in
`Manager::Update1` between `ModelMan` and RAIN) and the career's living world operations.

### ChallengeAIManager::GetChallengeCharacters

0x825bdff0 (vtable slot 128 of 0x823dea50): the active AI skaters' character recipes. Not used by
the character manager; not ported.

## 7. Tests

`challenge/characters_tests.rs` (game data via `SKATE2_GAME_ROOT`, `#[ignore]`d like the other
career tests): Slappy's box, ground test, marquee load, spawn and release; a release before the
spawn's answer; the DReg release; Atiba's alternate location through `ForceCull` and completion;
`dt_2`'s waypoint guards waiting for their group; `c_1a_01_bowl_series` management values;
`m_3a_01_pro_duffy` and `sponsor_me_03` operations with restart. Data-free: the oct tree's free
slots and the tester box.
