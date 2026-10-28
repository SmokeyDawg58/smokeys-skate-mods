# Party Play — native map and port plan

Party Play is Skate 2's local multiplayer: two to four players take turns on one console in
Spot Battle, S.K.A.T.E. or Hall of Meat. Facts come from the debug executable `sk82_na_m.xex`
(IDA, read-only), the PS3 retail EBOOT where noted, and the Jan-2009 data (`data/db`,
`data/fe/screens.big`). Addresses are debug-xex virtual addresses.

## 1. Pieces

| Native | Role |
| --- | --- |
| `FrontEndState_PartyPlay` (FE state 35, screen 51 `main/partyplay`) | the set-up screen: players, controllers, skaters, game, location |
| `Sk8::BE::PartyPlayBackEnd` (back-end module 19) | the chosen game (`mPartyPlayChallenge`) and location list (`mPartyPlayLocations`, challenge keys) |
| `FELocationManager::GetPartyPlayLocations` 0x82635180 | a game's challenges (leaf `challenge_global_data` collections of a parent) |
| `FECharacterManager` with `isPartyPlay` | the FreeSkate character list without the created skater |
| `Character::PlayerLoader` party members | up to four loaded skaters (`PartySetMarqueeCharacter`, `LoadPartyMember` 0x8281ade8, `UnloadPartyMember` 0x828182f0) |
| `Sk8::PartyPlayUI` | `cMsgSetPlayer` (`SetPlayer` 0x828aa4c0): the player whose turn it is |
| screen 64 `PartyPlayPressA` | the hand-over screen between turns |
| `FrontEndState_PartyPlayChangeGame` (FE state 39, screen 65) | changing game and location mid-session |
| `StartHubState_PartyPlay` (`eStartHubMode_PartyPlay` 8) | the pause menu in party play |
| `Sk8::Flow::StateLoading::PartyPlayLoadState` 0x82770ea8 | `eLoadingType_StartNewPartyPlayChallenge`, `MESSAGE_StartNewPartyPlayChallenge` |
| challenge families | `ChallengeType` 9 (Own the Spot), 4 (S.K.A.T.E.), 31 (Hall of Meat) state graphs in turn-based mode |

Other enums: `ePartyPlayChallenge` SpotBattle 0, Skate 1, HOM 2; `GameMode` GM_PartyPlay 2;
`kGameLoadFlags_PartyPlay` 16; `eSk8BEEndMissionOptions` Rematch 2, Next 13, ExitPartyPlay 16,
AdjustSettings 17; `ePartyPlayControllerPullOptions_QuitGame`; cvars `kBanAmbientSkatersInPartyPlay`,
`kbUseDebugModelForPartyPlay`.

## 2. From the main menu

- `FrontEndState_MainMenu::OnMenuSelectGame` 0x82681338 row 3 (outside `kbDemoMarketing`):
  `RunNIS(eFENIS_FromMainMenuToPartyPlay)` and `PopState`. `UpdateFEState` pushes state 35 when
  the sequence completes.
- `FENISManager::mNISList` 0x82fdb240: entry 3 "From MainMenu To PartyPlay" (ICE take
  `ICELITE::StringHash` 0x25aa8f76) places the APT layer on the `fe_3d_screen` quad
  `0x40424a7d0c50aef8` (`hash64("partyplay")`); entry 5 "From PartyPlay To MainMenu"
  (0x8c14ddd6) on the main menu's quad 0x5453b391061ee548.

## 3. The set-up screen (`FrontEndState_PartyPlay`)

`ePartyPlayState`: NumberOfPlayers 0, ControllerSelect 1, PlayerJoin 2, PlayerSelectCharacter 3,
GameSelect 4, LocationSelect 5, ToGame 6.

- `OnEnter` 0x8264a870: ambient AI skaters off, `mPreviousGameMode` kept and `gameMode =
  GM_PartyPlay`, controller port unlocked, NumberOfPlayers with two players, the four skater and
  controller slots -1, `PartyPlayBackEnd::Reset(false)` (location 0, SpotBattle) and
  `RefreshLocations`, the FreeSkate character count with `isPartyPlay`, `OpenScreen(51)`.
- `OnIntroDone` 0x8264b068: `SimCharacterViewer::Startup` with the party play viewer.
- `ReceiveInput` 0x82669b08 (keycodes A 2, B 3, left 14, right 15; sounds by key:
  A `0xEA167A71` core_a_button, B `0x4CC8B818` core_b_button, right `0x1C45EA28` nav right,
  left `0xB4997B49` nav left):
  - NumberOfPlayers: right/left `Increase/DecreaseNumberOfPlayers` (2..4, sound only when it
    changes) then `SetScreenState(2)`; A → ControllerSelect; B → `gameMode` restored,
    `RunNIS(eFENIS_FromPartyPlayToMainMenu)`, `PopState`.
  - ControllerSelect: `SetPartyPlayMode` sends state 3 and goes straight on to PlayerJoin.
  - PlayerJoin: A on any controller sets `mControllerIndex[active]` (a controller may join more
    than once), picks the first skater not taken by an earlier player, → PlayerSelectCharacter;
    B clears the slot, steps back a player (or to NumberOfPlayers) with state 3.
  - PlayerSelectCharacter (only the active player's controller): right/left
    `Next/PreviousCharacter` (skipping taken skaters, nav sound) and state 6; A keeps the skater,
    state 7, the next player's PlayerJoin or GameSelect; B → PlayerJoin.
  - GameSelect: right/left `PartyPlayBackEnd::Next/PreviousChallenge` (wraps) and state 9;
    A → LocationSelect; B → PlayerSelectCharacter with state 3.
  - LocationSelect: right/left `Next/PreviousLocation` and state 11 (the large image again after
    500 ms: `OnUpdate` 0x8264aac8 state 12); A → ToGame, input blocked, `EnableBackground` fade,
    `FadeComplete` 0x8265ae38 → `EnterGame`; B → GameSelect.
- `SetPartyPlayMode` 0x8265aca8: NumberOfPlayers state 1; PlayerJoin state 4 and the viewer
  hidden; PlayerSelectCharacter the active player's skater, state 5 and `ShowCharacter`;
  GameSelect state 8, viewer hidden; LocationSelect `RefreshLocations` and state 10.
- `EnterGame` 0x8264ad08: `sGameInfo.challengeKey` from `PartyPlayBackEnd::GetChallengeKey`,
  AI skaters unregistered, players reset, the challenge's location set (and its additional
  world unless BAM), each player's skater `PartySetMarqueeCharacter(i, or -1 for player 0)` and
  `LoadPartyMember(i)`, viewer shut down, `mWaitingForPartyPlaySkaters`, loading tag.
- `OnUpdate`: once every party member is loaded, each player `SetPlayerCharacter`,
  `CreatePlayer(i, PT_Human)`, `SetPlayerControllerIndex`, then `StartGameModule`.

### Movie `main/partyplay` (screen 51)

Imports `image_renderer` (`mImage`) and `highlight_category` (`mLocationTitle`, `mCharname`);
every nested frame script is a `Stop`. `SetState(n)` (the C++ `SetScreenState`):

| n | Mode | Script |
| --- | --- | --- |
| 1 | SelectPlayers | `mPartyplay` `numplayers` |
| 2 | RefreshSelectPlayers | `mNumplayers` `0`/`1`/`2` for 2/3/4 players |
| 3 | ControllerSelect | `setup`; players hidden, then the first N shown; from `GetPlayerIndex` on, `open` and no port |
| 4 | PlayerJoin | the active player's `join`, `mCharselect` `off` |
| 5 | PlayerSelectCharacter | `charselect`, `mBacker` `<player>`, `mCharselect` `on`, the controller's port, then 6 |
| 6 | RefreshSelectCharacter | `mCharname.SetTitle(GetSelectedCharacterName)` |
| 7 | AcceptSelectCharacter | the player's `set` and `mPlayername` text |
| 8 | SetupPartyPlayChallenge | standard B/A help, `mCharselect` `off`, `eventselect`, then 9 |
| 9 | RefreshPartyPlayChallenge | `UpdatePartyPlayChallengeInfo`: one of `item0..2` `on`, title and description, `mIcon` `0`/`1`/`3` |
| 10 | SetupSelectLocation | B Back / A Start help, `gotoAndStop("locationselect")`, then 11 |
| 11 | RefreshSelectLocation | `mLocationTitle.SetTitle(GetPartyPlayLocation)` |
| 12 | RefreshImage | `mImage.LoadRX2Texture(GetPartyPlayLargeImage)`, `locationselect` |

The constructor titles the letterbox `ID_PARTYPLAY_TITLE`, labels the four players `ID_INPUT_P1..4`
with `ID_PARTYPLAY_PLAYER_JOIN` (format argument 1..4), and shows the Xbox controller ports.

## 4. Games and locations (`PartyPlayBackEnd`)

`RefreshLocations` 0x82566ef0 fills `mPartyPlayLocations` from `GetPartyPlayLocations(game)`:
leaf `challenge_global_data` (`0xa4f35b159d7432b5`) collections under a parent.

| Game | Parent(s) | Filter |
| --- | --- | --- |
| Spot Battle | `spot_battle` 0xe107b17103ff61c1 (`f_1a_*_ots_*` Own the Spot) | unlocked |
| S.K.A.T.E. | `skate2_single_player` 0x29bb63acb748a562 then `skate2_online` 0x844fde45e50c82de | a bool attribute set, then unlocked |
| Hall of Meat | `hall_of_meat` 0x7680df0642c4a9be (`ol_hom_*`) | unlocked |

"Unlocked" (`GetPartyPlayLocations` 0x82635180 with `onlineonly` false): every location while
`kAllLocationsUnlockID` is unlocked; otherwise a challenge whose `b2c86f650bdefce5` RefSpec names
a career challenge counts only once that challenge's "ABD Points Earned" save value reaches its
threshold. A location's title is its activity's `Title`; its image `fe\source\images\locations\`
plus `FELargeImage` (`GetPartyPlayLargeImage` 0x82566bc0).

## 5. The games

Each location is an ordinary challenge activity run in turn-based form (`TurnBasedStartVolume`,
`TurnBasedWaitingLocation`, `TurnBasedAttemptSpawnPoint`, `CentralMessageHALIDYourTurn`,
`NumRounds` 3, per-turn `TimeLimit`). Own the Spot and Hall of Meat share one online template
(setup → countdown → challenge_running → end_turn → wait_for_players → leaderboard /
highlight_reel); S.K.A.T.E. has its own leader/follower round graph. Their natives are mostly
unported in the career runtime (HUD2, Contest, Trick, Skate, HOM, HighlightReel, Online).

## 6. Port plan

1. The set-up screen: main menu row 3, the NIS, state 35 and its movie, the viewer, the
   location list and image.
2. Game start: party members, players and controllers, `GM_PartyPlay`, the chosen challenge.
3. Turns: `PartyPlayUI`, the hand-over screen, the party-play pause menu, Change Game, end
   options.
4. The three families' state graphs and natives, one at a time.

## 7. Port status

The set-up screen, the game start, S.K.A.T.E.'s turns and Spot Battle from start to results
are in (2026-10-06).

- Set-up screen: `skate2-core` `ui/party_play.rs` (`PartyPlayBackEnd`'s lists, with the career
  unlock rule read through `session/selected_session_career.rs` `profile_challenge_system`, and
  the `FrontEndState_PartyPlay` controller); `ui/main_menu.rs` row 3, its NIS takes and wall
  quad; `session/selected_session_party_play.rs` (the viewer, `take_party_play_start`); app
  `marker_ui/party_play_movie.rs` (asset 51).
- Game start: `session/selected_session_freeskate.rs` `start_party_play` (`GM_PartyPlay`, the
  challenge's `Location`, a skater per player) and §8's multi-skater engine.
- Challenge hulls: `challenge/assets.rs` (`cChallengeAssetManager`) and
  `session/streaming/selected_hull_loader.rs` (`cHullLoader`; Sim collections only).
- S.K.A.T.E.'s natives and turns (`challenge/skate.rs`, `players.rs`, `teleport.rs`), the
  hand-over screen (`FrontEndState_PressA`, app `party_play_press_movie.rs`), the turn HUD
  components (`challenge/turn_hud.rs`: `TimerHUD`, `ThrowDownMessageHUD`, `ABDHUD`,
  `LeaderboardHUD`, each screen state snapshotted with its data as `TurnHudView`) and their
  movies (app `hud_timer_movie.rs`, `hud_throwmes_movie.rs`, `hud_abd_movie.rs`,
  `hud_leaderboard_movie.rs`).
- Spot Battle and Hall of Meat share the Own the Spot template. Offline (`Online.IsOnline` is
  `Simulation::IsNetworkGame`) its `setup` group is `CountdownHUD`, `TimerHUD`, `CentralHUD`,
  `LeaderboardHUD`, `TrickDisplayHUD` and `MiniMap` for Hall of Meat, and the same plus
  `HOMScoringHUD` and `DPOHUD` for Spot Battle (`EQ 0` on the challenge type skips the jump to
  the longer list only for Hall of Meat). Voting needs an online game of three or more.
  `CountdownHUD` is in `challenge/turn_hud.rs` with app `hud_countdown_movie.rs`;
  `ChallengeVotingManager` is `challenge/voting.rs` with app `hud_voting_movie.rs` (online
  only, so unseen offline).
- The DPO HUD (`challenge/dpo_hud.rs`, `turn_hud.rs` `Kind::Dpo`, app `hud_dpo_movie.rs`): 14
  of the 31 Spot Battle locations score cumulatively (`UseCumulativePoints`). Their
  `challenge_running` sets `DPOSetParams(0, leader's score)` and the Challenge icon and shows the
  group's DPO HUD; each completed sequence sets (turn score, leader's score); `end_turn` hides
  it. It is the "Score current / best" panel: `hud2/dpo` (template entry `DPOHUD` 0x82fdb888,
  placement 6, g3dHUDRight), raised (`high`) over a leaderboard of more than four players. Its
  icon is the main challenge's `fe_map_icons` name (`GetActiveChallengeIconName` 0x8260b158)
  from `screens.big`'s `dpoicons`.
- Scoring: `Physics::IsScoringMultiplierActive` (0x827accc8) turns the multiplier off while the
  main challenge's `ChallengeType` is Photo (6), BestTrick (8), OwnTheSpot (9: Spot Battle),
  MegaRamp (26), CreateASpot (35) or CreateASpot_Setup (36), so a Spot Battle sequence's
  `SequenceScore` is its raw points (`ChallengeScoring::multiplier_active`, read into the
  skaters' scoring input).
- Which free-skate HUD a mode keeps: `HUDManager2::FixupActiveComponents` (0x82cedff0) binds
  each component to the front-most active group listing it, and nothing while a visibility
  flag hides the HUD (`challenge/hud.rs` `HudManager::binding`, sent to the app as
  `CareerHud::binding`). The "main" group of `FrontEndManager::StartInGame` lists central,
  minimap, trickdisplay, homscoring and online_chyron.
  * S.K.A.T.E. (group: countdown, timer, central, leaderboard, abd, throwdownmessage) hides
    "main"'s trick display (`HUD2.Show("main", TrickDisplayHUD, false)`, shown again at
    `complete`) and calls `HOM.EnableHOMVisuals(false)`: no score or multiplier, no Hall of
    Meat panel on a bail; the minimap stays ("main"'s). Hidden, it is not updated either:
    `HUDManager2::Update` (0x82cee4f0) shows a bound component to its instance's visibility and
    `TrickDisplayHUD::Update` (0x826bda40) runs only while visible, so no multiplier sting
    (`FE_SOUND_MULTIPLYER_*`) plays (`HudBinding::trick_display`, `ScoreMovie::update`).
  * Spot Battle's `reset` shows its group's trick display and hides its minimap and Hall of
    Meat instances, and disables the Hall of Meat visuals.
  * Hall of Meat's `reset` hides its group's trick display and minimap; its group has no
    homscoring, so "main"'s Hall of Meat panel shows on each bail (`StartWipeout` 0x827c7858).
  * `Physics::IsHallOfMeatEffectEnabled` (0x827acbd0), which gates that panel and the bail
    scoring: the cvar, outside the front end, forced on by its cvar, else off in free skate
    (modes 0, 3, 4), off when the HOM manager's visuals are off, and in single player (1, 7)
    the "Hom Available" signpost setting (`ChallengeScoring::hom_visuals`/`hom_available`).
- The area name at the bottom centre never shows in Party Play: `FrontEndManager::
  UpdateDistrictZoneName` (0x82638820) only sets `mZoneKey` to -1 while a main challenge runs,
  and each Party Play game is one from its first frame at the location
  (`minimap::Frame::main_challenge`, `marker_ui.rs` `zone_key`).
- Dynamic shadows follow the turn: `PostFX::RenderShadows_CSM` (0x82855888) fits the cascades'
  depth (`m_BB.m_Max.y + 2` over `nis_depth_precision`) to
  `GetSkaterPresEntityFromIndex(GetLocalIndex())`, the skater whose turn it is
  (`SelectedVisualSnapshot::local_skater_shadow`); each skater still culls by its own hips and
  box. Skaters more than 2 m above the active one cast no shadow, as in the original.
- Skater audio: `AudioPresentation::DoSimSubmit` (0x827fb2f8) submits every SkaterManager
  Actor, local when its player ID is `GetLocalSimIndex`, which the turns move; the two
  `CSTATEMGR_Player` states follow the local skater and any other within 30 m of the camera
  (`SelectedLiveSession::audio_skaters`). The bail, multiplier and WorldPainter audio layers
  read the local skater.
- The end of a game: `HighlightReel.PlayHighlightReelWithSpecifiedOrder` (an empty reel
  completes at once), the results natives (`challenge/results.rs`: rows, column titles,
  layout, end options) and FE state 8 (`FrontEndState_EndChallengeOptions`, with Rematch and
  Exit popups) in `session/selected_session_career.rs`. `OnEnter` (`0x82658880`) picks the
  screen (`results::end_screen`): a throwdown's 47 (app `throwdown_results_movie.rs`; the
  `skate` section authors four rows, the one-up section eight) or, for a win with several
  players, 41 (app `end_challenge_movie.rs`: `endchallengeoptions` with the `results_lb`
  list, `dynamic_selectable_list` and its `LIST_TEXTBOX`/`LIST_NUMBERBOX` cells). Both are
  `g3dSpinContainer` screens turned by the front end's one spin, which `MarkerUi` owns. FE
  state 8, like the unlock state, pauses and hides the HUD (`gFEStateInfo` 0x82fded80).
- Change Game (results option X, `ePartyPlayOption` Adjust Settings): `OptionSelect` keeps the
  leaderboard and pushes FE state 39 (`FrontEndState_PartyPlayChangeGame`, `ui/party_play.rs`
  `ChangeGame` over the kept `BackEnd`), which leaves FE state 8. Its screen 65 is app
  `party_play_change_movie.rs` (asset 68, over the shared dimmer). Up and down switch between
  game and location, left and right step them (a game change restarts at its first location);
  B pops back and FE state 8 opens its results again; A on the running challenge sends
  `RestartMainChallenge(0, true)` and `cMsgHUDRetry`, on another `cMsgHUDContinue` (IDA labels
  both sends with folded template names: kinds 0xb17fdd0c and 0x1e053b88) and flow message 6.
  The port starts the new challenge once the old one completed: the first player travels to
  its location and the others are teleported there (native `SpawnPlayer`, command 3, destroys
  and recreates each skater), then a new challenge session starts it.
- The pause menu: `ui/pause_menu.rs` `StartHub::PartyPlay` (`StartHubState_PartyPlay`'s
  options; Restart Challenge's popup 204 restarts through `ChallengeSystem::
  restart_main_challenge`, whose sub-controllers are not ported).
- Check: `SKATE2_DIAGNOSTIC_PARTY_PLAY=<end tick>` on the `main-menu-freeskate` demo (three
  players, S.K.A.T.E. at the snakes spot, start at 1760; the first player rolls through the
  spot volumes and pops by position, the others let their turns run out);
  `SKATE2_DIAGNOSTIC_PARTY_PLAY_GAME=spot` (Spot Battle at Danny's Megaramp; its results open
  near tick 15600) or `hom` picks the game; `SKATE2_DIAGNOSTIC_PARTY_PLAY_CHANGE=full|view|switch`
  drives Change Game from the results; `SKATE2_DIAGNOSTIC_PARTY_PLAY_BACK=1` backs out at
  1000. The scripted player cannot land a set trick, so a S.K.A.T.E. game never ends in the
  check; app test `marker_ui::tests::original_party_play_end_screens_build` (ignored, needs
  `SKATE2_GAME_ROOT`) builds screen 47 for three S.K.A.T.E. players, screen 41 and screen 65
  instead (`gotoAndStop` on a label a clip lacks does nothing, as `gotoAndPlay`: only
  `mcStrip0` of throwdownresults has `none`/`highlight`).

Hull requests (`cPathedHullLoader::LoadHull`/`UnloadHull`) are queued wherever the asset
manager makes them: in its update, and when a challenge starts from a front-end answer or a
sign-up (`selected_session_career.rs` `queue_hull_requests`); the loader starts them on its
next update. A hull's challenge DMO assets are recorded as `cChallengeDMOManager::
AddChallengeDMOs` (0x825bdc48) does (`ChallengeDMOGroups`; `RemoveChallengeDMOs` 0x825bbde8
removes their locators, culling their objects, when the hull unloads).

A location's props are those groups. Its challenge starts only once its assets are in
(`PartyPlayLoadState` 0x82770ea8: `LoadChallengeAssets`, then `IsAssetLoadComplete`;
`CareerSession::start_loaded_party_challenge`), and the DMO manager's `RunMainActivity`
(0x825c3a00) then runs `RunDMOOperations(activity, true)`: the challenge's `tDMOResetInfo`s
(the snakes' S.K.A.T.E. saves and clears its volume's dynamic objects, two Hall of Meat
locations clear theirs) and `AddStreamedLocatorData` of each group, whose locators the census
then spawns from; the challenge's `DMOCensusRange` becomes the dynamic object census range. Each
turn's `countdown` resets them (`DMO.ResetChallengeDMOs`); the challenge's completion unwinds the
operations and puts the range back. `challenge/dmo.rs`, `selected_living_world_challenge_dmos.rs`;
evidence in `docs/evidence/partyplay-world-population-2026-10-09.md`.

The PC pad is controller 0, which drives every player's turn. Not yet ported: replays and
highlight reels, `RestartMainChallenge`'s sub-controllers, the hulls' Pres collections, the
leaderboard's time layout (`LeaderboardSetTime`), end screens 42 to 45 (wins without players and
losses), `OnEnter`'s `cMsgResultScreenEffect`, additional world locations, and the vote input
(online only).

Quitting a game to the main menu (`StateGameUnload::UnloadGameState` 0x82757cc8,
`selected_session_freeskate.rs` `game_unload`) runs `ResetGame` 0x8273a680's parts that matter
here: the offline local index 0 and a fresh `PlayerUI` for player 0 (whose turn the input then
follows again), the challenges' end, and `SimController::Reset`'s `SkaterManager::Reset`
0x82600410, which destroys every later skater (`DestroyPlayer` 0x825fff80: the `EntityManager`
and skater list, `UpdateLocalPlayer`, the observers' Destroyed notification; then the Actor's
destructor 0x825f9668 releases its input listener and its physical player's controller drives,
skeleton, hook and board, and its entry in the World's physical collection, `Simulation::Remove`
0x828a63e0). `UnloadPartyMember` 0x828182f0 then unloads each member's character
(`selected_session_party_skaters.rs` `try_destroy_party_skaters`). The next Party Play game spawns
its later skaters afresh. The first player's skater stays, as it does in every port mode, without
the pin another player's turn gave it (a fresh native Actor has none).

The unload resets the player rows before its `ResetGame`, as `UnloadGameState` does:
`AISkaterManager::UnregisterAllSkaters` 0x824cb868 (each AI row's recipe unregistered, then the
row removed), `PlayersBackEnd::ResetPlayers` 0x82564338, then `CreatePlayer(0, PT_Human, -1)`
0x82563d50 on the controller player 0 had. `UpdatePlayerName` 0x82563a88 names that player by the
game mode still in force: Party Play's "Player 1". The single player game started next
(`StateMainGameFlow::DoStartSinglePlayerGame` 0x8277b070, outside Party Play: the career, a free
skate, an online game) resets the rows again, creates player 0 under the profile's name, applies
the loaded skater's attributes (`PlayerLoaderImpl::ApplyPlayerAttributes` 0x828158f0) and gives it
the front end's controller (`selected_session_freeskate.rs` `unload_players`,
`start_single_player_players`). Before this, the career after a Party Play game kept "Player 1" as
the player's name. The attributes apply once `PlayerLoader::IsLoaded` 0x828141c8 holds: the
character id `OnSimCharacterLoaded` 0x82817fb4 stores before the attributes and the registration,
so a skater still registering counts (`PlayerLoaderSlot::is_loaded`). Waiting for the
registration instead left a game started soon after boot on the reset's gestures, truck
tightness and wheel hardness until Edit Skater applied the record again. `cMsgSetPlayer` reaches only a `PartyPlayUI`, the UI `ResetGame` makes in
`GM_PartyPlay` (its constructor 0x828aeae0 subscribes to it; `PlayerUI` does not).

A switched challenge has no tips loading screen, on purpose: the port's loads finish behind the
fade. Natively `PartyPlayLoadState` waits for the assets and for `IsLoadingScreenReady`
0x82758928, which for `eLoadScreenType_Tips` is `FrontEndState_TipLoader::IsReady` 0x82d6ec48
(`mbReady`, set by `OnIntroDone` 0x8265a178): FE state 81's screen 143 always plays its intro,
then shows a new tip every 360 updates (`OnUpdate` 0x8265a1e0) while the load continues.

## 8. Several skaters at once (stage 2's foundation)

Native Party Play simulates every player's skater. `StateLoading::PartyPlayLoadingState`
0x8276e6c8, once the stream, boundary and `kPartyPlayHUDGroupName` group are loaded, sends for
each player `cMsgSetPlayer(i)` (PartyPlayUI switches `mPlayerIndex`/`mControllerIndex`) and
`cMsgSpawnPlayer(hash32(location))`, whose `PlayerUI::ReceiveSpawnMsg` 0x828ac8b0 issues command 3
for that player; then `cMsgSetPlayer(0)` and `SkaterManager::SetLocalIndex(0)`.
`SkaterManager::OnCommand` 0x82600920: 3 `SpawnPlayer` (Actor, skater list, entity manager,
sort by index, `InitPlayer`: graphs, Created, `UpdateLocalPlayer`), 5 teleport, 7 pin
(Actor+600 bit 0x4000000; Lua `AI.PinAI`/`UnpinAI` 0x82895d78/0x82895f38 through `cMsgPinPlayer`),
8 set transform. `SetLocalIndex` 0x82600380 / `UpdateLocalPlayer` 0x825ffef8 re-find the local
skater and notify LocalChanged (camera). Turns use `ChallengeManager.SetPlayer`,
`AI.RepositionAI` (`cChallengeTeleportManager::TeleportPlayer`), `AI.PinAI` and
`Online.SwitchCamera` (= `SetLocalIndex`).

Every frame `Simulation::SimUpdate1` 0x8289f410 / `SimUpdate2` 0x8289f980 run each phase over
the whole skater list before the next: `UpdateAnimation1` (Phase1), LivingWorld Update2,
`UpdateAnimation2`, `EndDMOUpdate`, `UpdateAnimation3` (Phase3, then every skater's
`DispatchHandsFaceAnim` inside one left/right hand ACS command set), `UpdatePhysics` (per skater:
ACS sync, `SetUpPhysics`, `Actor::Update`), `World::Update1`, ..., `UpdateLocomotion`,
`World::Update2`; then `World::Update3`, ..., camera, predictor, `PostUpdate`.

The port's session was built for one skater (`SelectedPlayerTick`, `SelectedSessionFrame`,
the boot's player/pair/presentation owners). The plan:

1. The tick takes the skater list (index order) and the local skater; per-skater stages loop,
   world stages (LivingWorld with the local skater, grab messages, the solver, predictor) run
   once. One skater must trace identically to before (`SKATE2_TRACE_SELECTED_STATES`).
2. A spawned skater owns its loader, recipe, formation, Actor, animation inputs, physical pair,
   presentation and presentation runtime; it shares the primary's input manager, animation
   banks, face manager, recipe registry, graphs and back-end rows.
3. Roster admission for later skaters (append, entity list, sort, `InitPlayer`), `SetLocalIndex`
   and the pin flag.
4. Presentation and drawing for every skater; PartyPlayUI input routing.

### Game start (native, for stage 2)

- `EnterGame` 0x8264ad08: `sGameInfo.challengeKey`, `AISkaterManager::UnregisterAllSkaters`,
  `PlayersBackEnd::ResetPlayers` 0x82564338, the challenge's `Location` (`SetLocationName`) and
  additional world (unless BAM), then per player `PlayerLoader::PartySetMarqueeCharacter(i,
  marquee, peer)` with peer -1 for player 0 and `i` otherwise, `LoadPartyMember(i)`. Each
  member's `OnSimCharacterLoaded` 0x82817f28 runs `ApplyPlayerAttributes(i)` 0x828158f0:
  `SetPlayerCharacter`, `SetPlayerStance(IsSkaterRegular)` (layout byte 16 of
  `characters_marquee`) and `SetHatType`. The port writes them in `create_party_players`
  (`PlayerAttributeServices::apply_marquee`; Freeskate's start does the same for its pro): its
  first player's loader can finish only after the travel to the location, whose spawn foot
  forward reads the stance byte. Reading `ResetPlayers`' 0 there made a regular pro spawn
  goofy-footed and ride switch (the mongo brake).
- `OnUpdate` 0x8264aac8, once every member is loaded: per player `SetPlayerCharacter`,
  `CreatePlayer(i, PT_Human, peer)` 0x82563d50 (its `UpdatePlayerName` 0x82563a88 names a party
  player `FormatString(ID_PARTY_PLAYER_PLAYER, i + 1)`), `SetPlayerControllerIndex`, then
  `StartGameModule`.
- `PlayerLoaderImpl::Load` 0x8281aa80 passes `isLocalPlayer` true for every requested marquee
  (false only for the invalid-CAS default marquee): party members load as local players.
- `StateLoading::DoNextTransition` 0x82775fd0: `kLOADING_STATE_STARTNEWPARTYPLAYCHALLENGE` (10,
  pushed by `StateMainGameFlow::FilterImpl` on its flow message) goes to `PartyPlayLoadState`
  0x82770ea8 (`LoadChallengeAssets`, tips load screen, `kPAUSE_STATE_TELEPORT`), then
  `PartyPlayLoadingState` 0x8276e6c8 (spawns, `SetLocalIndex(0)`), then `PostLoadingState`.
- `cChallengeBEManager::GameLoad` 0x825b7800: game type 16 (party play) sets
  `sGameInfo.ignoreWipeoutResetButton`.
- `cLocationManager::GetSpawnLocation` 0x82a8d250 (`tLocationDesc`, index): a sub-location's
  matrix when the index is in range; otherwise sub-location 0's (or, without sub-locations, the
  location's) rows with translation + (0, 0, -5 * index).
- Multi-skater natives ported for stage 2: `UpdateCollisionStatus` 0x82e016a0's nearest
  same-flags skater (`GetSkateboard()->GetTransform()`), `AddSkaterObstacles` 0x82e3a198
  (`kActorSearchRadius`, three 0.75 m spheres), `SpawnPlayer`/`SetLocalIndex`/`UpdateLocalPlayer`,
  `kEnableHatCollision`.

## 9. Port changes (not in the original)

Community requests change Party Play on purpose (2026-10-07, 2026-10-09 and 2026-10-19):

- **No pedestrians.** The original populates Party Play like every mode but free skate
  (`CensusMan::Update` 0x82702780 hands the density sliders to free skate, mode 3, alone). While
  the game mode is `GM_PartyPlay` (2) the port's census spawns no pedestrian, from the census or
  from props, and culls every one it may, so none stay from an earlier game
  (`CensusState::pedestrians_banned`, `living_world_census_pedestrian.rs` `evaluate`).
- **No traffic.** The same for vehicles: in `GM_PartyPlay` the vehicle census spawns none and
  culls every one it may (`CensusState::vehicles_banned`, `living_world_census_vehicle.rs`
  `evaluate`). The original spawns cars wherever the game's `LivingWorldManagement` allows them
  (S.K.A.T.E. 0x3f, some Hall of Meat locations; Spot Battle's 0x31 never does).
- **The created skater.** The original's set-up screen lists the FreeSkate skaters without the
  created skater (`FECharacterManager` with `isPartyPlay`). The port lists it first, as
  FreeSkate's screen does (under the profile's name), whenever the profile has saved its recipe:
  unlike FreeSkate's row it ignores the new-profile flag (`ProfileBackEnd`+0x68), which the port
  clears only at a career autosave where the original clears it on leaving Create-a-Skater
  (`FrontEndState_CaC::Shutdown` 0x82659d28) and at `StateGameRootFlow::EnterImpl`. The original
  lets one player at most take a skater (`IsSkaterAlreadySelected` 0x826433e8); as a PC change the
  port lets players share one, which loads once and is shared as the character asset manager
  shares a registered schema (`UpdateRequest` 0x82d14ac0, `++mRefCount`). The
  game's start writes its stance, hat, style and tuning from the record into the player's row
  (`create_party_players`, as `Sk8CACLoader::ApplyAttributesImpl` 0x82819cb8). The first
  player's skater loads it as FreeSkate's start does; a later player's `PlayerLoader` member
  loads it through `PlayerLoaderImpl::Load` 0x8281aa80's CAS branch, which reads the profile's
  parts record whichever member asks (`PartyCharacter::Created`, with the member's peer kept for
  its registration). The later players spawn only once the first player's skater is the chosen
  one, so a character the first player gave up is free to load.
- **Skaters pass through each other.** The original collides the skaters with each other in
  `Step_Collision1`'s skater-skater pipeline (`Island::InitWorld` 0x827b4bf4: the skater pool on
  both sides, two members a job). In `GM_PartyPlay` the port runs that pipeline over each
  skater's own board and Skeleton (the pool's adjacent members) alone, so the skaters pass
  through each other as an online game's remote skaters do, which have no simulation
  (`SkaterSkaterPairs::Own` in `skateboard_ground_frame.rs`, picked from the game mode in
  `selected_player_tick.rs`).
- Checks: `SKATE2_DIAGNOSTIC_PARTY_PLAY_CAREER=1` keeps the first player on the first skater, the
  created one with a profile that has it; without it a profile with a created skater gives it to
  the second player. `SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT=<tick>` presses Start at the tick, quits
  and starts the main menu's free skate with its first skater, who pushes off.
