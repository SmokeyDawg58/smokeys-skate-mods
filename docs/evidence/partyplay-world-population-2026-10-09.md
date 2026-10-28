# Party Play world population: challenge props, no traffic (2026-10-09)

Party Play spawned the world's ordinary dynamic objects (DMOs) but none of the props a Party Play
location brings: the picnic tables, benches, rails, kickers, wedge ramps and news stacks its
challenge hulls hold (`cChallengeDMOManager::ChallengeDMOGroups`). The port recorded those groups
as the hulls streamed in but never placed their locators, never ran the challenges' DMO
operations on their trigger volumes, `DMO.ResetChallengeDMOs` did nothing, and the challenge
started before its hulls were in. This ports the challenge DMO manager's Party Play path, starts
the challenge once its assets are loaded, and, as a deliberate PC deviation, keeps traffic out of
Party Play the way the port already keeps pedestrians out.

Addresses are the debug `sk82_na_m.xex` in IDA (read-only).

## Native behaviour

### The challenge DMO manager

`Sk8::Challenge::cChallengeDMOManager` (PDB: `TriggerManager`, `AssetManager`,
`ChallengeBroadcaster`, `Activated`, `IssuedDMOOperations`, `WaitingForDMOSpawn`,
`WaitingForChallengeAssets`, `SerializedDMOBufferMap`, `DMOManager`, `DMOInstanceManager`,
`DMOModelMan`, `CensusMan`, `DMOProxyMap`, `NextDMOProxyID`, `GameTypeFlags`,
`ChallengeDMOGroups` (`fixed_vector<cAssetID, 64>`), `AddChallengeGroupsNextFrame`,
`TriggerBufferIDMap`, `NextSerializedBufferID`) is the 30th activity sub-controller
`Sk8ChallengeSystem::GameInit` (0x825aa948) registers; its `IActivitySubController` is at +4.

- `GameLoad` (0x825b03f8): game types 4 (career) and 16 (Party Play) cache the dynamic object
  manager, its locator manager and the census manager and set `Activated`; every game keeps its
  type flags. `GameUnload` (0x825ad480) clears `Activated`, the cached managers and the flags,
  not `ChallengeDMOGroups`.
- `AddChallengeDMOs` (0x825bdc48), from `Simulation::RegDMOArena` for a kind-31 (`DMOData`) asset
  registered with userdata bit 4 (a challenge hull's): an activated manager records the asset
  once. Nothing is placed yet.
- `RunMainActivity` (0x825c3a00), with the census cached: Party Play (`GameTypeFlags == 16`) runs
  `RunDMOOperations(activity, true)`; then `SetDynamicObjectsCensusRangeKey` (census vtable +0x38,
  0x8271c088) of the challenge's `DMOCensusRange` `RefSpec` collection (zero when absent).
- `RunDMOOperations` (0x825c3368), activated: for each element of the challenge's `tDMOResetInfo`
  array (attribute 0x915c7bf8b06d5a58, 24 bytes: `eDMOResetType Type` at 0, the volume's
  `tTriggerVolumeInstanceID` at +0x10) on `cTriggerVolumeManager::GetTriggerInstance(float)`
  (0x82e21cc0):
  - `eDMOResetType_Reset` (0): `RequestCullWithinTriggerVolume(trigger, false)`, then
    `ResetLocatorsInTriggerVolume`;
  - `eDMOResetType_Clear` (1): the cull, then `SetLocatorsInTriggerVolumeEnabled(trigger, false)`;
  - `eDMOResetType_Save` (2): first `SerializeDMOs(trigger, false)` (0x825c2210), the buffer kept
    under `TriggerBufferIDMap[trigger->m_uiGuid]` unless that trigger already holds one
    (`ReleaseSerializeBuffer` 0x825b73d0), then as Clear;
  - `eDMOResetType_DisableAll` (3) and higher: nothing.

  With `addChallengeDMOs`, `LWDynamicObjectLocatorMan::AddStreamedLocatorData(group, 0, false,
  false, DmoTreatAsStatic)` (0x82d3d548: flags, allowPersistence, allowPriorityTracking,
  treatAsStatic) of each `ChallengeDMOGroups` asset; the census then spawns from those locators.
  Last, activated or not, `cMsgDMOOperationsComplete` (0x7efd1f1d) on the challenge queue with
  the activity's `GetUniqueID` (0x82a8f568, its state graph's collection;
  `cMsgDMOOperationsComplete_BuildMessageTable` 0x828b8de8: "ActivityKey", the u64 as a float).
- `CompleteChallenge` (0x825bbd70), activated: the main challenge's completion calls
  `ResetDynamicObjectsCensusRangeKey` (census vtable +0x48, 0x8271c138: "dynamicobjects"), then
  `UnwindDMOOperations` (0x825ba848, activated only): Clear `SetLocatorsInTriggerVolumeEnabled(
  trigger, true)`; Save `RestoreFromSavedState(-1, buffer)` (0x825b5c98,
  `RestoreLocatorsInSerializedList(null, 0, saved, n)`) when the trigger holds a buffer, which is
  then released and erased. `ShutdownActivity` (0x825bbd68) unwinds too. `RestartChallenge`
  (0x825b5c18) redoes the operations only with `DmoRedoOperationsOnRestart`, which no Party Play
  challenge sets.
- `ResetChallengeDMOs` (0x825b06a0), Lua `DMO.ResetChallengeDMOs()` (0x82892ba8, no activation
  check): `ResetStreamedLocatorData(group)` (0x82d2de58) of each group: per record
  `ResetLocator(id, false)` (0x82d2a208 / 0x82d3c8d0: `RequestExplicitCullOfUserObject`
  0x82d2a278, then `ResetLocatorTransform(id, initialStreamTransform, false)` 0x82d3c740, which
  moves the passive tree entry only when asked). Spot Battle and Hall of Meat call it as each
  turn's `countdown` state starts (seen live below).
- `RemoveChallengeDMOs` (0x825bbde8), from `Simulation::DRegDMOArena` (0x828a0828) for userdata
  bit 4: an activated manager calls `RemoveStreamedLocatorData(asset, userdata, true, false)`
  (forceObjectCull, no persistence; 0x82d3d618 / `RemoveLocatorsForStreamedLocatorChunk`
  0x82d3c0c8) and erases the record. A deactivated one does nothing.
- The career runs the operations when a sign-up is accepted (`OnChallengeAccept` 0x825c3728,
  `IsChallengeAcceptOperationDone` 0x825c37a8) and redoes them on a restart; see the follow-up.

### The locator and object calls

- `SetLocatorsInTriggerVolumeEnabled` (0x82d32380): a 0.1 m sphere
  (`PhysicsUtility::CreateCollisionSphere` 0x82e4af08; `SphereVolume::Initialize` 0x82ad9438:
  identity rows, group and surface 0, flags 1) at each locator's transform, then at each streamed
  record's transform for its locator; `IsColldingWithTriggerVolume(sphere, transform, trigger,
  false)` (0x82e213b8: `PrimitivePairIntersect` with the instance's `m_pCModel` bounding volume
  at `m_TransformMatrix`, false without a model) sets or clears the locator's enabled bit.
- `ResetLocatorsInTriggerVolume` (0x82d3c928): the same two walks; the first
  `ResetLocator(id, false)`, the second culls the user object and `ResetLocatorTransform(id,
  record transform, false)`; after any reset the passive tree is rebuilt (`MemStore::Reset`,
  `UpdateSpatialLookups` 0x82d32188: every free locator).
- `SerializeLocatorsInTriggerVolume` (0x82d35ba0): each locator whose sphere collides, as
  `tLocatorData` (transform, initial stream transform, locator ID, model ID, attachment time).
- `RestoreLocatorsInSerializedList` (0x82d3b650) with no serialized list: a saved locator still
  present is enabled again; a missing one with persistence data or a streamed record
  (`GetStreamedLocator`) is created from the saved data.
- `LWComponentMan<DynamicObject>::RequestCullWithinTriggerVolume` (0x82d36b18): every entity
  (census managed or not) for which `LWBasicEntity::IsIntersectingTriggerVolume` (0x82e4c138: the
  model's `GetBBoxVolume` at `GetBBoxTransform` of `LWEntityCoreData::mTargetTransform`) holds
  gets `RequestExplicitCull` (0x82d36390).

### Census range

`CensusMan`'s constructor (0x827208d0) keys +0x68/+0x70/+0x78/+0x80 with "pedestrians",
"vehicles", "props" and "dynamicobjects"; `EvaluateDynamicObjectCensus` (0x82726930) reads its
`livingworld_census_ranges` collection from +0x80. All 804 challenges have `DMOCensusRange`; only
`m_4i_01_spotfilm_barbee` names another collection than "dynamicobjects".

### Party Play loads the challenge's hulls before starting it

`StateLoading::PartyPlayLoadState` (0x82770ea8) calls `LoadChallengeAssets(challengeKey, true,
false, 0)` on entering and moves on to `PartyPlayLoadingState` only once
`IsAssetLoadComplete(challengeKey)` (0x82e0f990) holds (and the players and load screen are
ready); `StartChallengeState` (0x8274f800) then runs `StartMainChallenge`. A location's groups are
therefore recorded before `RunMainActivity` places them.

### Leaving a game mid-way

`GameModule::GameUnload` (0x8273d1b0) runs `Sk8ChallengeSystem::GameUnload` (0x825a4330: every
sub-controller's `GameUnload`, the DMO manager's clearing `Activated`, then
`cChallengeAssetManager::UnloadAll`), and only then pumps `UnloadUpdate` until the hulls are gone.
Their `DRegDMOArena` reaches a deactivated manager, so a quit Party Play game's group locators
stay in the locator manager (`ResetGame`'s `LivingWorld::Manager::Reset` 0x8271d3d8 culls the
objects but keeps locators; `RemoveAllLocators` 0x82d3c250 runs only from
`DynamicObjectsBackEnd::DefaultData`/`ResetAllDynamicObjects`).

### Data

- 25 hulls of Party Play locations hold `DMOData` records: 16 Spot Battle `_props` hulls (e.g.
  `f_1a_30_ots_svm09_snakebowl_pr`: a wooden picnic table, two low corner-bend rails, a rainbow
  rail), 8 Hall of Meat hulls (kickers, wedge ramps, a loading ramp) and
  `ol_skate_04_DT_bigstreetdrop` (news stacks, a wedge ramp): 86 locators over 28 models. 25 of
  the models are main DMO bank (`cPres_Global.xsf`) models; three `OT_BarcelonaBench_DMO`
  variants of the cathedral spot (0x0001b48003e38707, 0x0001b48103e38707, 0x0001b48203e38707)
  are in no bank and the hull's Pres asset is only a material (0x00eb0005), so the census cannot
  spawn those three benches.
- `tDMOResetInfo`s: `t_5b_01_skate_snakes` Save on volume 0x000721d203e38706;
  `ol_hom_16_wf_01_boat` (0x0007aaa103e38706) and `ol_hom_17_svm03_underbridge`
  (0x0007aaa203e38706) Clear. `DmoTreatAsStatic`: `f_1a_36_ots_wf08_stairset_ol`,
  `f_1a_07_ots_wf02_lilypads_ol`.
- `LivingWorldManagement` (bits 0 to 5: spawner, pedestrians, prop pedestrians, vehicles, DMOs,
  props): Spot Battle 0x31 (no pedestrians, no vehicles); S.K.A.T.E. 0x3f (vehicles); Hall of
  Meat 0x3f, 0x39 (`ol_hom_11`, vehicles), 0x37, 0x35, 0x31 or 0x11. So the original drives
  traffic through S.K.A.T.E. and most Hall of Meat locations.

## Port

| Where | What |
| --- | --- |
| `challenge/dmo.rs` | The manager's `GameLoad`, `RunMainActivity`, `RunDMOOperations`, `CompleteChallenge`, `ShutdownActivity`, `UnwindDMOOperations` (with `GameTypeFlags` and the Save buffers per trigger GUID); `DMO.ResetChallengeDMOs`. They queue `LivingWorldOp`s and send `cMsgDMOOperationsComplete`. |
| `challenge/characters.rs` | `LivingWorldOp::{DmoReset, AddChallengeDmos, UnwindDmoReset, ResetChallengeDmos, DynamicObjectsCensusRange}`. |
| `challenge/system.rs`, `challenge/activity.rs` | The DMO manager's slots in `RunMainActivity`, `CompleteChallenge`, `ShutdownActivity`, `GameLoad`; `ChallengeSystem::load_challenge_assets` / `is_asset_load_complete`. |
| `session/selected_session_career.rs` | Party Play loads the challenge's assets first and starts it once they are in (`PartyState::loading`, `start_loaded_party_challenge`). |
| `session/selected_session_career_living_world.rs` | Applies the new operations (the Save buffer goes to the challenge system's DMO manager). |
| `session/living_world/selected_living_world_challenge_dmos.rs` (new) | Trigger instance lookup and the sphere / bounding-box trigger tests; the trigger-volume cull; Reset/Clear/Save and their unwinding; the groups' placement and reset. |
| `living_world/dmos/living_world_dmo_locators.rs` | `SetLocatorsInTriggerVolumeEnabled`, `ResetLocatorsInTriggerVolume`, `SerializeLocatorsInTriggerVolume`, `RestoreLocatorsInSerializedList`, `ResetStreamedLocatorData`, `ResetLocatorTransform`'s spatial flag; `RemoveStreamedLocatorData`'s forceObjectCull and allowPersistence. |
| `session/streaming/selected_terrain_dmo.rs`, `selected_terrain_startup.rs` | The recorded groups with their records for the session (`ChallengeDmoChunks`); `RemoveChallengeDMOs` now removes the group's locators (force cull, no persistence). |
| `living_world/population/living_world_census.rs`, `living_world/dmos/living_world_dmo_objects.rs` | `CensusMan`+0x80 as `dynamic_objects_range_key`, read by the DMO census. |
| `living_world/population/living_world_census_vehicle.rs`, `session/living_world/selected_living_world_dmo.rs` | Port change: `vehicles_banned` in `GM_PartyPlay` (2), like `pedestrians_banned`: no vehicle spawns, every cullable vehicle culled. `SKATE2_TRACE_LIVING_WORLD_COUNTS=<n>` prints the population every n updates. |
| `apps/skate2-pc/src/live_skating_client.rs` | Party Play demo: `SKATE2_DIAGNOSTIC_PARTY_PLAY_DISTINCT` (players 2 and 3 on different skaters), `SKATE2_DIAGNOSTIC_PARTY_PLAY_LOCATION=<text>` (right through the locations until the title ID holds the text). |
| `challenge/dmo_tests.rs` (new), `docs/partyplay/README.md` | Tests (game data); README sections 7 and 9. |

Native locator map iteration (eastl hash order) is replaced by the port's block order, both for
the trigger-volume walks and the passive tree rebuild.

## Verification

- `cargo build --release -p skate2-pc` builds (21:17, every change). The runs below used the 19:57
  build, which had every behaviour change; the two later edits are a trace (locator counts in
  `career DMOs: ... placed`) and `restore_saved_locators` asking `is_streamed_locator`. A Spot
  Battle run on the final build prints `challenge DMO groups [dc9dc93d272980bd] placed (static
  false): locators 81 -> 85`.
- `cargo test --release -p skate2-core --lib challenge::dmo:: -- --include-ignored` (with
  `SKATE2_GAME_ROOT`): 2 passed (`party_play_runs_the_dmo_operations_with_the_groups`,
  `career_start_sets_only_the_census_range`). The same binary's `dmo_locators` (12), `census` (8)
  and `dmo_objects` (7) tests pass.
- `PARITY IDENTICAL (6000 lines)`.
- Live, Party Play from the main menu on fresh scratch saves (`SKATE2_TRACE_LIVING_WORLD_COUNTS`,
  `SKATE2_TRACE_CAREER`, `SKATE2_DIAGNOSTIC_PARTY_PLAY_DISTINCT=1`, captures at 1280x720):

| Run | Result |
| --- | --- |
| Spot Battle, Snake Session (`f_1a_30_ots_svm09_snakebowl_ol`) | Both hulls (`f_1a_30_ots_svm09_snakebowl`, `_pr`) load before the challenge enters `initial`; `challenge DMO groups [dc9dc93d272980bd] placed`, locators 81 -> 85 (the four `_pr` records); every turn's `countdown` resets the group (9 turns over 460 s, no error). The captures show the wooden picnic table and the red rails at the bowl, the skater grinding one. `enable=0xc6`, `banned=true/true`, no vehicle or pedestrian, 15 to 17 DMOs. |
| S.K.A.T.E. the Gap (`t_5b_01_skate_snakes`) | `reset type 2 on volume 0x721d203e38706 (trigger a7b03be68c707e4f)`; no group. `enable=0xfe` (its 0x3f lets vehicles spawn) with `banned=true/true`: no vehicle in 290 s; about 45 DMOs. |
| Hall of Meat, Bridge to Somewhere (`ol_hom_17_svm03_underbridge`) | `reset type 1 on volume 0x7aaa203e38706`; group `[b34f90b312fdbd5e]` placed, locators 44 -> 46 (its two `KickerRampLarge_DMO`); reset at `countdown`; no vehicle. |
| Spot Battle, Quit at tick 2700, then the main menu's free skate | No error. Free skate has `banned=false/false` but `enable=0xc6`, as the original's would (see the census after a quit). |
| Main-menu free skate | `enable=0xfe`, `banned=false/false`: pedestrians 3 -> 12, three vehicles by 20 s, DMOs. |
| New Career, Yes (`SKATE2_DIAGNOSTIC_MOVIE_SKIP=120 SKATE2_DIAGNOSTIC_CAREER_YES=1`) | `m_1a_01_goto_slappys` reaches `challenge_running`; `enable=0xfe`, `banned=false/false`, DMO range "dynamicobjects", 9 DMOs (7 within 30 m), 229 locators. No pedestrian or vehicle spawns while the skater stands in the `J_Gamestart` alley for 40 s; outside `GM_PartyPlay` both bans are false, so the census runs as before. |

## Follow-up: shared skaters, the census after a quit, career operations, completion

### Two players on one skater

- The original's set-up screen never gives two players one skater: a joining player starts on
  skater 0 and steps past every skater an earlier player holds, and `NextCharacter` (0x8265ae68) /
  `PreviousCharacter` (0x8265af80) skip them too (`IsSkaterAlreadySelected` 0x826433e8, called from
  `ReceiveInput` 0x82669b08 at 0x82669db8). The port lets players share one, a PC change
  (`career-replay-photo-film-2026-10-09.md`, section 7).
- The character asset manager itself shares a loaded schema. `CharacterAssetManager::UpdateRequest`
  (0x82d14ac0), `RequestState_LoadSchemaFromFile`: a schema the presentation registry holds
  (`GetRegAsset` 0x82d14b90) becomes the request's (`++mRefCount` 0x82d14bb4) and the request goes
  on from `RequestState_SchemaLoaded` without the file; `RequestState_AssetLoadRequested` finds the
  entity's assets loaded (`mState`, 0x82d15274) and goes to `FinalizeStructure`
  (`RebindToMaterials`, `BuildTInstanceArray` 0x82d15318). `RequestState_UnloadRequested`
  (0x82d15554) drops one reference; only the last one's `Unload` removes the registry entry
  (`RemPresRegistryAsset` 0x82d155d8, `TrackUnloadedRecipe`). Requests run one at a time, so a
  second load of a schema always finds it.
- A created skater loads under a fresh `schar_%d` name (`SimCharacterManager::Load` 0x8281a358,
  `LoadSchemaFromByteStream` 0x82d13788 asserting on a registered name), so the original never
  shares one.
- Port: `MountedRecipeRegistry::try_share` (`player_mounted_recipe_load.rs`) shares a registered
  skater schema (recipe and scene, one more reference, finalized again); `unload_registered` drops a
  reference and removes the entry with the last. The port keys a created skater's entry by its
  recipe's schema name, so two players on the created skater share one entry instead of loading
  two `schar` entities with the same parts. A LivingWorld entry is still not shared with a skater.

### The census after a quit

- Only challenges and the career start set the census spawners: `cChallengeLivingWorldManager::
  RunMainActivity` (0x825c0da0, the challenge's `LivingWorldManagement`, no activation check),
  its `CompleteChallenge` (0x825b7640: a main challenge's completion sets all six, 0x3f),
  `ReevaluateActivityPrereqs` (0x825ad840: `EnableCensusSpawner(true)`; only the career runs it,
  `cActivityManager::ReevaluatePrereqActivities` 0x82cdbaa8) and
  `FrontEndState_MainMenu::StartSinglePlayer` (0x82678c20: `EnablePedestrianSpawn(true)`
  0x82678c90, `EnableVehicleSpawn(true)` 0x82678ca8). `OnMenuSelectGame` (0x82681338) clears them
  only in its marketing demo. `CensusMan::Init` (0x82715908) and `LivingWorld::Manager::Reset`
  (0x8271d3d8, from `ResetGame`) set only the world bit; the LW manager's `GameLoad`/`GameUnload`
  (0x825ad758/0x825b6080) leave the census alone. The setters' bits: 0x8271bfe0 to 0x8271c058
  (`insrwi` 25 to 30: 0x40 spawner, 0x20 pedestrians, 0x10 prop-driven pedestrians, 0x08 vehicles,
  0x04 DMOs, 0x02 props).
- A quit does not complete the challenge (`ReceiveAbandonChallengeMessage` 0x825a2de0 ignores
  `fIsQuitting`; `cActivityManager::GameUnload` 0x82cda420 terminates running graphs). So after a
  quit Spot Battle (0x31) the original's free skate keeps pedestrians and traffic off, as the port
  did; a career start turns both back on (prop-driven pedestrians stay off until a challenge sets
  them).
- Port: `start_single_player_census` (`selected_session_career.rs`) at the main menu's career row
  (`selected_session_public_facade.rs`). Free skate is unchanged. The demo's
  `SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT_ROW=0` takes the career row after the quit.

### Career DMO operations

- The DMO manager is the 9th low-priority sign-up sub-controller (`Sk8ChallengeSystem::GameInit`
  0x825aa948 registers its `ISignupSubController`, +0x10, at 0x825ab308; low-priority order:
  telemetry, service, HOM, teleport, audio, asset, DMO, living world, OTS discovery, effects, front
  end, challenge system). `cChallengeSignUpManager::Update` (0x82db44a0) calls every low-priority
  `OnChallengeAccept` once the high-priority ones are done, then asks every low-priority
  `IsChallengeAcceptOperationDone` each update until all are done, then starts the challenge.
- `OnChallengeAccept` (0x825c3728): activated, `IssuedDMOOperations` cleared; with the challenge
  hulls in (`AreChallengeHullsLoaded`), `RunDMOOperations(activity, true)`, issued, and
  `WaitingForDMOSpawn` 3. `IsChallengeAcceptOperationDone` (0x825c37a8): done when not activated;
  issued: done once the wait is 0, else one less; not issued: run them as soon as the hulls are in.
- `RestartChallenge` (0x825b5c18, from `cActivityManager::RestartMainChallenge` 0x82cd8880 after the
  living world manager's): with `DmoRedoOperationsOnRestart`, `RedoDMOOperationsOnRestart`
  (0x825b04f0): Reset again; Clear and Save as Clear (nothing saved again); then, with recorded
  groups, `AddChallengeGroupsNextFrame`, which `Update` (0x825b0700) places on the next update
  (`AddStreamedLocatorData(group, 0, false, false, DmoTreatAsStatic)`).
- Not the port's: a mission the SignPost starts (`cSignPostManager::ExecuteChallenge` 0x825d27f0,
  `StartMainChallenge` directly) runs no operations; the career's start during a load
  (`StateLoading::StartChallengeState` 0x8274f800: `RunDMOOperations(key, true)` 0x8274f9d0 before
  `StartMainChallenge`) and global activities (`RunGlobalActivity` 0x825c3aa8:
  `RunDMOOperations(activity, false)`) have no port.
- Challenges with `tDMOResetInfo`s: Party Play `t_5b_01_skate_snakes` (Save),
  `ol_hom_16_wf_01_boat`, `ol_hom_17_svm03_underbridge` (Clear); career Reset:
  `m_1a_01_goto_slappys` (redo), `m_6a_01_follow_spillway`, `m_7c_03_photo_cover_sm_3` (redo),
  `m_4d_02_spotfilm_dill_dyrdek` (redo), `w_3b_01`; Clear: `c_1a_01_bowl_series`,
  `c_1a_01/02/03/05_besttrick_street` (three volumes each), `c_1a_03/05_bowl_series`,
  `c_2a_01_jam_branded`, `c_2a_02_jam_branded_pure`, `m_4k_01_spotfilm_haslam`; Save:
  `m_4b_01_pro_navarrette_1` (redo), `m_4a_01_skate_carroll_koston` (redo), `m_4m_01_pro_rattray_cole_2`,
  `m_6p_01_demo_first_job`, `m_6p_07/08_photo_pool_sm_2/3`, `m_7a_01_demo_dyrdek`,
  `m_1a_06_photo_puzzle`, `r_1_1_fabio_mem1`, `sponsor_me_01/02/05`; Clear and Save:
  `m_4h_01_pro_appleyard` (redo); `c_3a_01_street_gvr` lists type 3 (`DisableAll`, nothing). 132 of
  the 557 mission hulls hold `DMOData` records (962 locators), the career's among them
  (`m_1a_01_goto_slappys` 2, `m_4b_01_pro_navarrette_1` 27, `m_1a_06_photo_puzzle` 8,
  `c_1a_03_besttrick_street` 16, ...).
- Port (`challenge/dmo.rs`, `signup.rs`, `system.rs`): `on_challenge_accept` before the effects
  manager's, `is_challenge_accept_operation_done` among the low-priority answers (each asked every
  update), `restart_challenge` after the living world's, `update` before the dynamic hull
  manager's (the 31st). The port's terrain keeps the recorded groups, so a redo always sets
  `AddChallengeGroupsNextFrame`; with no group its placement does nothing.

### Party Play completion

- In the original the next Party Play challenge loads through `StateLoading` with the same
  challenge system, whose asset manager unloads the completed challenge's hulls meanwhile (their
  `DRegDMOArena` reaching an activated DMO manager, `RemoveChallengeDMOs`). The port builds a new
  system for the next game and dropped the old one in the frame the challenge completed, before its
  asset manager issued the unloads: the hulls stayed, and the next game placed the old location's
  group again (`challenge DMO groups [dc9dc93d272980bd] placed ... locators 131 -> 131` at the
  snakes). `CareerSession::take_party_switch` now waits for `IsAllUnloadComplete` (0x82e0f400,
  `assets::is_all_unload_complete`), as `GameModule::GameLoad` (0x8273be50) does before a new game.
- `SKATE2_DIAGNOSTIC_PARTY_PLAY_TWO=1` plays with two players (six turns of three rounds) so a
  Spot Battle or Hall of Meat ends within the demo; the native end path runs (end of the last turn,
  `challenge_finished`, highlight reel, leaderboard, end options, Change Game). Under
  `SKATE2_TRACE_CAREER` the terrain prints `challenge DMO group <id> removed`.

### Follow-up verification

- `cargo build --release -p skate2-pc` (22:21). `PARITY IDENTICAL (6000 lines)`.
- `cargo test --release -p skate2-core --lib` (with `SKATE2_GAME_ROOT`): `challenge::dmo::` 4 passed
  (`career_acceptance_runs_the_operations_then_waits_three_updates`,
  `restart_redoes_the_operations_of_challenges_that_ask` added), every `challenge::` test with
  `--include-ignored` 96 passed, `mounted_recipe` 8 passed (with
  `registered_skater_schema_is_shared_with_a_reference`), `dmo_locators` 12 passed.
- Live (scratch saves):

| Run | Result |
| --- | --- |
| Spot Battle at the megapark, players 2 and 3 on Cuz (no `_DISTINCT`) | Both spawn with recipe 0x8c0d2ae0c53b259c; the game runs. Before: `RecipeCacheHitPending`. |
| Spot Battle at the snake bowl, two players, Change Game to S.K.A.T.E. (`_TWO`, `_CHANGE=switch`) | Six turns, `challenge_finished`, highlight reel, leaderboard, end options; Change Game's A: `challenge_cleanup`, `complete`, `UnloadHull` of the `_pr` hull, `challenge DMO group dc9dc93d272980bd removed`, `UnloadHull` of the location hull; `enable=0xfe` again (the completion's 0x3f); then S.K.A.T.E. at the snakes places no group (`[]`) and runs its Save. |
| Hall of Meat at the underbridge, two players, then Change Game | Six turns and the end flow; `complete`: `reset type 1 on volume 0x7aaa203e38706 unwound` twice (`CompleteChallenge` and `ShutdownActivity` both unwind, as in the original), `UnloadHull`, `challenge DMO group b34f90b312fdbd5e removed`; the next game (Spot Battle at the megapark) ran to its own end options. |
| Career: New Career, Yes, then a sign-up of `c_1a_01_bowl_series` (`SKATE2_DIAGNOSTIC_CHALLENGE`) | Its hulls load, the poster opens; on acceptance `reset type 1 on volume 0x64fe703e38706` and `challenge DMO groups [f098ccc38348dda8, 998737d843ff50eb] placed (static true): locators 90 -> 93` (the bowl props and `m_1a_01_goto_slappys`'s two, which the native manager records alike); the challenge starts (`enable=0xc4`, its 0x11). |
| Spot Battle quit at tick 2700, then the career row (a scratch copy of a career save) | `enable=0xc6` before, `0xee` in the continued career; pedestrians spawn. |

## Open

1. **A quit leaves the location's props.** The port does not unload a challenge's assets at a quit
   (no `cChallengeAssetManager::UnloadAll` in the game unload), so its group stays recorded and
   placed. Natively the hull unloads but `RemoveChallengeDMOs` meets a deactivated manager, so the
   locators stay as well and `ChallengeDMOGroups` keeps the ID; the props stay in both.
2. **Cathedral benches:** three of `f_1a_06_ots_ot17_cathedral_pro`'s four benches have no model in
   the DMO bank (see Data), so they never spawn.
3. **Not ported:** the career's start during a load (`StartChallengeState`'s operations) and global
   activities' operations, which have no port at all.
4. The trigger-volume cull tests the object's target transform (`mTargetTransform`, as
   `IsIntersectingTriggerVolume` does); the existing census-managed cull of pedestrians and
   vehicles (`census_managed_cull`) tests `core.transform`.
5. The set-up screen's shared skaters remain a PC change; the original skips skaters
   another player holds (`IsSkaterAlreadySelected`).
