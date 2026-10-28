# Career: challenge icons on the minimap and the map (2026-10-09)

Agent B of the 2026-10-09 career port. Addresses are `sk82_na_m.xex` (Jan-17-2009 debug) VAs
read from the live IDA database (read-only); data facts are from the career vaults through
`tools/career` and the `challengemap.apt` / `challenge_info.apt` / `menu_picker.apt` scripts in
`data\fe\screens.big`. Nothing here was built or run: the code type-checks
(`cargo check --release -p skate2-core --tests`, `cargo check --release -p skate2-pc`), and the
tests are for the orchestrator to run.

## What the player sees between missions (vault data)

| After | Icons (`ChallengeIcon` → `fe_map_icons.minimap_texture`, at the challenge's `Location`) |
| --- | --- |
| `m_1a_01` | `m_1a_03` `mini_tutorial` at `m_1a_03_tutorial_fliptricks_challengelocator01` |
| `m_1a_03` | `m_1a_04` `mini_tutorial`, `m_1a_05` `mini_smag` (pinch point, pulsing), `m_2a_01_tutorial_slappy` `mini_tutorial` |
| `m_1a_05` | `m_1a_06` `mini_smag` (pinch point, pulsing) |
| `m_1a_06` | `m_1a_07` `mini_follow` (pinch point, pulsing; `RunMainActivity` shows the AI skaters' icons while it runs) |
| `m_1a_07` (its accept set the `slappys` hull to `Unlock`) | also the four `fe_location_group` icon sets (`key_map_general`, `_plazas`, `_skatepark`, `_spawnpoints`) |

Every one of these icons has `DisplayOnMiniMap` and `DisplayOnFEMap` set; no `tLocationID`
overrides the `Location`. The pinch points are `progression_pinchpoints` collections holding the
one challenge with threshold 1 and the non-pulsing byte clear. Completed type-0 missions get no
completed icon. While a main challenge runs, `RunMainActivity` removes every challenge icon and
its presentation adds its own (presentation event 30, e.g. `m_1a_01`'s `mini_teleport`).

## Minimap and back-end icons (`challenge/map_icons.rs`)

| Native | Address | Port |
| --- | --- | --- |
| `MapIcon::MapIcon` | `0x82601a50` | scale 1, fade at edge, not a pinch point, non-pulsing, visible, `kMapIconScaleDir` 0.02, not map-visible, minimap-visible, map scale 1 |
| `MapIconManager::AddIcon(type, pos, activity, completed)` | `0x82634548` | texture `minimap_texture` + `.Texture`; an activity's icon carries its key and is minimap-visible from `DisplayOnMiniMap` unless completed |
| `MapIconManager::AddIcon(type, location, pos)` | `0x82634400` | location icons, minimap visibility from the icon's own `DisplayOnMiniMap` |
| `MapIconManager::RemoveIcon` | `0x82631290` | |
| `MapIconManager::UpdateMiniMap` / `UpdateMap` | `0x826187b8` / `0x82618728` | pinch-point pulse between `kFEMiniMapIconPinchPointMinScale` 0.5 and `...MaxScale` 1.0 |
| `MapIconManager::ActivateBigBlackIcon` / `EnableBigBlackIcon` | `0x82635d48` / `0x82634720` | from the type-12 progression messages 256/257 |
| `MapBackEndManager::AddChallengeIcon(activity, completed)` | `0x82577aa8` | `tLocationID` `0x0717c5a85c96fd90`, else `Location`; none at the origin; type 34/45/9/40 second or third icons |
| `MapBackEndManager::RemoveChallengeIcon` | `0x82572ec0` | 64-bit key compare (`cmpld`) |
| `MapBackEndManager::AddChallengeObjectIcon(name, location / trigger)` | `0x825781f8` / `0x82577f80` | the last `fe_map_icons` of the name in `GetCollections` order |
| `MapBackEndManager::AddLocationIcon` / `AddLocationGroup` | `0x82577850` / `0x8257cf50` | |
| `MapBackEndManager::RemoveAllIcons`, `RemoveChallengeObjectIcon`, `RemoveVisualIndicatorIcon` | `0x8256c300`, `0x82572fa0`, `0x82573050` | |
| `MapBackEndManager::ShowAIPlayers` / `UpdatePlayerIcons` | `0x82565578` / `0x825656e0` | `mini_skater` icons of the non-local, non-human skaters |
| `MapBackEndManager::AddVisualIndicatorIcon` | `0x82578470` | `mini_teleport` `0x6ae8ebac95cc3108` |
| `MapBackEndManager::StuffMenuFilter` / `EnlargeIcon` | `0x825654d8` / `0x82565478` | 64-bit key compares |
| `MapIconManager::RenderMiniMapIcons` / `RenderIconOnScreenMiniMap` | `0x826311f0` / `0x8262ea20` | `minimap_icons()`, fed to the session's minimap `dynamic_icons` |
| `cChallengeMiniMapManager::RunMainActivity`, `CompleteChallenge`, `ReevaluateActivityPrereqs` | `0x825a2aa8`, `0x825a2b40`, `0x825a5bd0` | wired into `system.rs` / `activity.rs` |
| `cSignPostPinchMan::CheckPinchPoints`, `CheckPinchPoint`, `ActivatePinchPoint` | `0x825dff58`, `0x825dd4b0`, `0x825d8750` | from `cSignPostManager::Init`, `EvaluateActivities` and `LoadData` |
| `IsChallengePinchPoint` / `IsChallengeNonPulsing` | `0x825d19a8` / `0x825d1a18` | |
| `cChallengePresentationDirector::RemoveChallengeMapIcon` / `RemoveAllChallengeMapIcons` | `0x82e06c38` / `0x82e06cb8` | events 30/31 and `RemovePresentationGroup` `0x82e07af8` |
| Lua `Map.AddChallengeIcon` (= `HideChallengeIcon`), `ShowChallengeIcon`, `RemoveAllChallengeIcons`, `AddMapIcon`, `RemoveMapIcon`, `AddObjectiveVisualIndicatorToMap`, `RemoveObjectiveVisualIndicatorFromMap` | `0x828839e0`, `0x82883a40`, `0x82883aa0`, `0x8288c288`, `0x8288c348`, `0x8288c3e0`, `0x8288c570` | `natives()` |
| `Attrib::Class::GetFirstCollection` / `GetNextCollection`, `VecHashMap` `InternalAdd`, `RebuildTable`, `CopyFromOldTable`, `Add` | `0x8297a508` / `0x8297a518`, `0x82977ae0`, `0x829778f8`, `0x829770f8`, `0x8297a690` | `AttribDb::table_order` (the `GetCollections` order) |

`cSignPostManager::ProcessChallengeResults` (`0x825e139c`) runs `EvaluateActivities` after the
autosave; the port did not, so the icons of the next missions never appeared. It does now.

## The career map (`challenge/stuff_menu.rs`)

The start hub's Map option opens `FrontEndState_ChallengeMap` (FE state 21) over the pause in
the career; the session routes the map's input there and the PC draws it on the existing
`challengemap.apt` movie in place of the FELocation browser.

| Native | Address | Port |
| --- | --- | --- |
| `FrontEndState_ChallengeMap::OnEnter` / `OnLoaded` | `0x826539e8` / `0x826879b8` | `ChallengeMap::open`: `Initialize`, rows (state 10) in select mode, `SetHighlightRegion(Stuff)` (`FilterIcons()`, active mode), letterbox 53/50/54 |
| `ReceiveInput` | `0x8266b420` | Stuff region: X toggles the views, B goes back a level or closes at the root |
| `OnMenuHilight` / `OnMenuSelect` | `0x82644870` / `0x826828f0` | a level of categories filters the highlighted one's icons; the item is highlighted; level changes run states 80, 10 and the letterbox |
| `SetHighlightRegion`, `TriggerInfoComponent`, `TriggerInfoComponent_FELocation`, `DisableInfoComponent`, `DoTeleport`, `OnLeave` | `0x8265ef80`, `0x8265f070`, `0x8265f180`, `0x8265f208`, `0x8265f280`, `0x8265ec98` | |
| `GetChallengeTitle`, `GetChallengeComponentOption`, `GetChallengeComponentNumOptions` (always 1) | `0x8264d1b0`, `0x8264d938`, `0x82623a78` | |
| `FEStuffMenu` constructor, `Initialize`, `PopulateMenus`, `PopulateChallenges`, `PopulateMenuItemsVector_Challenges`, `PopulateMenuItemsVector`, `PopulateMenuItemsFromLocationGroup`, `PopulateMenuItemsVector_DlcSpawnPoints`, `PopulateMenuItemsVector_Property` | `0x82635720`, `0x82640d88`, `0x8263e170`, `0x82634270`, `0x82632418`, `0x826332a8`, `0x82632cc0`, `0x82632e90`, `0x826328a8` | list ids 96, 90, 105, 89, 97, 93, 91, 87, 88, 62, 86, 85 (tutorials, available view only), 61, 59, 57, 74, 70 |
| `GetNumSubCategories`, `GetNumSubMenuItems`, `FilterCategory`, `GetNumMenuItems`, `GetCategoryHAL`, `GetChallengeHAL`, `GetStuffMenuItems` | `0x82607308`, `0x8261c8c0`, `0x82629430`, `0x826294f0`, `0x8261cb48`, `0x82639770`, `0x8263c8b8` | `gSubCategory` `0x82079860`, `gStuffCategoryHAL` `0x82fdb2f0` |
| `FilterIcons(index, mapped)`, `FilterIcons()`, `FilterActiveChallenge`, `HighliteMenuItem` (+ `_GenericFEStuffMenuItem`, `HighlightFELocationItem`), `SelectMenuItem`, `PreviousMenuLevel`, `ToggleListActiveOrComplete`, `ResetMenu` | `0x82607388`, `0x82618718`, `0x826321c0`, `0x82629560` (`0x8261cc28`, `0x8261ceb8`), `0x826357f0`, `0x826296b8`, `0x8263e468`, `0x82635bc0` | |
| `cSignPostManager::GetAllChallengesInList`, `cSignPostUtil::GetListKeyFromListID` | `0x825d7c10` / `0x825d69e0`, `0x825c8d70` | progression byte 40 skips; signpost keys converted to `challenges` keys |
| `FELocationManager::IsLocationUnlocked` | `0x82615868` | |
| `MapBackEndManager::TeleportToChallenge` | `0x8256dc10` | `GetSpawnLocation(Location, GetLocalPlayerID())`, through the session's existing map travel |
| `MapIconManager::RenderMapIcons` / `RenderIconOnScreenMap` | `0x82631138` / `0x8262e350` | the `mChallengeMapVisible` icons, `kFEMapEventIconSize` (`0x83049b08`, 32) `* mScale_map * mScale` wide, after the local skater's |
| `challengemap.apt` `SetState` (`0x6249`), `UpdateLetterBox` (`0x5e64`), `UpdateStuffMenuItems` (`0x611d`); `menu_picker.apt` `Update` (`0x3211`) | | the screen states the native sends; `kLBConfig_Stuff`'s X shows `ID_CHALLENGE_MAP_COMPLETED_CHALLENGES` in the available view, `..._ACTIVE_CHALLENGES` in the completed one |

After `m_1a_01` the root lists `ID_STUFFMENU_CATEGORY_TUTORIAL` (1) (and whatever other list
holds an available challenge); its highlight shows `m_1a_03`'s icon on the map, the tutorials
list it, its highlight draws it 64 wide, and its info component offers `ID_COMMON_TELEPORT`.

## Wiring

- `challenge/system.rs`: `World::map_icons` (`MapIcons`, was a counter), `World::pinch_points`;
  `RunMainActivity`, `CompleteChallenge`, the Lua `Map` natives, `UpdateFrontend`'s pulse.
- `challenge/activity.rs`, `signpost.rs`, `signpost_progress.rs`, `presentation.rs`, `natives.rs`:
  the hooks above. `Output::AddChallengeObjectIcon` / `RemoveChallengeObjectIcon` are gone (the
  core keeps the icons); `Output::ChallengeMapIcon` stays for observers. The race natives push a
  null `lightuserdata` when no icon was added, as the native does.
- `session/selected_session_career.rs`: `challenge_icons()` for the minimap; the open map
  (`open/close/update_challenge_map`, `challenge_map_view`, `challenge_teleport_transform`).
- `session/selected_session_public_facade.rs`: the Map option opens it with the FELocation
  browser's `map_menu`; its update runs the career map; `map_menu_snapshot()` carries its view.
- `ui/map_menu.rs`: `MapMenuSnapshot::career` (the snapshot is `Clone`, no longer `Copy`).
- `apps/skate2-pc/src/marker_ui.rs`, `marker_ui/map_movie.rs`: rows, title, info header and
  option, letterbox buttons, icons. `marker_ui/minimap_movie.rs` skips an unregistered texture
  (the native draws `missingtexture`, `0x8262ea90`, which no loaded arena holds).

## Not ported, and why

- Map and News regions: pan and zoom (`PanMap`, `ZoomMap`), the district title, the news panel
  (`UpdateNewsPanel`); the map cursor's pan onto a highlighted item (`FEMapCursor::
  SetCursorPosition` `0x826183b8`, `UpdateSetPositionPan` `0x82607090`): the PC map draws the whole
  map as the existing FELocation browser does.
- The info component's description, objectives and location image (`IsDescriptionEnabled`
  `0x82625b40`, `IsObjectivesEnabled` `0x82625850`, `IsLocationImageEnabled` `0x826256d8` read the
  challenge's local data): the panel shows the header and the option only.
- Retry (completed view) and Own the Spot start the challenge during a load (`OnChallengeRetry`,
  `sGameInfo.startChallengeDuringLoad`): replay is out of scope; A does nothing there.
- Hall of Meat panel, property purchase, Create-a-Spots, downloaded content (no data in the port),
  the Big Black dismissal popup (no Big Black service), the chase-disabled option labels (the map
  does not read the living world's chase), FE sounds.
- The Big Black icon follows `ILWManager::GetBigBlackEntity`; the port's living world has no Big
  Black entity, so an activated icon stays at the origin.
- `cChallengeMiniMapManager::GameUnload` (`0x825a2b68`): the port drops the career session on quit.
- `kFEChallengeMapIconSensitivity` (default 10) is never read by the native.

## Tests

- `challenge/map_icons.rs`: the pinch-point pulse; `StuffMenuFilter` / `EnlargeIcon` /
  `RemoveAllIcons` touch only the back end's icons.
- `challenge/stuff_menu.rs`: the `gSubCategory` table; `PreviousMenuLevel` at the root; the map
  draws only map-visible icons at their map scale.
- `challenge/tests/career_map.rs` (game data, `#[ignore]`): `m_1a_03` on the minimap after
  `m_1a_01`; the career map's root, filter, highlight, info and Teleport after `m_1a_01`; the
  presentation's `mini_teleport` is the table's last of its name and is removed with the
  presentation; `m_1a_05` is a pulsing pinch point after `m_1a_03`.
