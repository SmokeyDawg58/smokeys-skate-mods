# Online challenges

What a challenge round looks like online, the scripting hooks the challenge templates use, and
the online-only systems around them (voting, free-skate activities, ranked progression, spot
leaderboards). This is rules logic the port keeps 1:1. Addresses are `sk82_na_m.xex`.

## 1. Challenge types

Online modes are `Sk8::Net::ChallengeType`, mapped to the game's challenge types by
`Net::GetSk8ChallengeType` 0x827840b0 / `GetNetChallengeType` 0x827841d8:

| Net | Mode | `eChallengeTypes` |
| ---: | --- | ---: |
| 0 | Own the Spot | 9 |
| 1 | S.L.A.P. | 12 (Lap) |
| 2 | Best Trick | 8 |
| 3 | Jam | 15 |
| 4 | 1-Up | 32 |
| 5 | S.K.A.T.E. | 4 |
| 6 | Death Race | 19 |
| 7 | Hall of Meat | 31 |
| 8 | Online free skate | 22 |

Free-skate activities are type 42 (`OnlineFreeSkateActivity`). Race modes set
`sGameInfo.isOnlineRace` (`StateOnlineGame::UpdateGameInfo`, skill type race). Ranked player caps:
6 for Own the Spot and Hall of Meat, 4 for 1-Up and S.K.A.T.E., 8 otherwise
([matchmaking.md](matchmaking.md) §2).

**Pools.** `ChallengeOnlineManager::GetOnlineChallengesByType(type, keys, sortByName)` 0x825bf228:
every leaf of the challenges collection whose global data is flagged online, has that type and a
valid location. Ranked games without DLC permission drop DLC challenges
(`FilterDLCChallenges` 0x825bc420, `IsDLCChallenge` 0x825adae8 reads the global-data flag).
`StateOnlineGame::SetupChallengeKeys` keeps the remaining keys; the game leader's
`UpdateNextChallenge` removes the one just played and, in ranked, picks the next one at random.

## 2. A round, end to end

```
leader: GameRequest(StartGame) ─► RB
RB: GameReset(LoadChallenge) ─► all          clients load (ChallengeLoadState), sim paused
clients: GameChallengeLoaded ─► RB
RB: GameReset(Challenge, seed, activity) ─► all
                                             ResetGame(seed); StartMainChallenge(challenge key)
           ... challenge script runs; per round: Lua Online.OnlineChallengeRoundSyncPoint
               clients: GameSyncPoint ─► RB ; RB: GameSyncPoint ─► all (round resumes)
           ... Lua Online.OnlineChallengeComplete(result)
clients: GameComplete ─► RB ; RB: GameAllPlayersComplete ─► all
clients: GameResults ─► RB ; RB: GameFinalResults ─► all ; GameTimer(DisplayResults)
RB: GameExitPostChallenge ─► all ; leader: GameResetAttributes (next challenge) ─► RB
RB: GameAttributes ─► all ; then GameReset(LobbySkate) back to lobby skate
```

State-machine detail: [game-flow.md](game-flow.md) §6–7. The RB's side (when it counts everyone as
loaded or complete, timeouts) is not in the client; Arcadia's `reset-and-challenge-flow.md` and
`post-challenge-results.md` describe a working reconstruction.

## 3. The script interface (`Lua::ModuleOnline`)

The challenge templates are the same Lua state graphs as single player and Party Play; they
branch on `Online.IsOnline` and call:

| Function | Native | Effect |
| --- | --- | --- |
| `IsOnline` | 0x8288a468 | `Simulation::IsNetworkGame` |
| `IsConnectedReady` | 0x82882f30 | Plasma connection ready |
| `GetNumOnlineSkaters` | 0x8288a6c0 | slots with a peer id |
| `IsPlayerDisconnected(i)` | 0x8288a768 | `PlayersBackEnd::GetPlayerIsDisconnected` |
| `GetLocalPlayerID`, `GetSkaterName`, `GetLocalPlayerGamerTag` | 0x8288a570, 0x82882f88, 0x82882fe0 | |
| `SwitchCamera(i)` | 0x8288a4e0 | `SkaterManager::SetLocalIndex(i)`: which skater this console follows |
| `OnlineChallengeRoundSyncPoint(round, location)` | 0x8289adb8 | pause (`kPAUSE_STATE_ONLINE`, `Net::PauseGame`), find the round's NIS in the challenge data and the local player's spawn at the location, flow message 17 (`MESSAGE_ChallengeSyncPoint`) |
| `OnlineChallengeComplete(ranking, score, time, playersChoice, finishReason = 1)` | 0x8289abc0 | flow message 16 (`MESSAGE_ChallengeComplete`) with the result |
| `LeaveGame` | 0x8289aaa0 | flow message `MESSAGE_ChallengeLeaveGame` |
| `InitVoting`, `StartVoting(votee)`, `StopVoting`, `DeInitVoting` | 0x8288a800… | the voting manager, §4 |
| `UploadOTSScore(activity, score)`, `GetOTSScore`, `GetTopFriendScore`, `GetTopFriendName`, `RequestTopFriendScore` | 0x82883040… | spot leaderboards, §7 |

## 4. Voting (players' choice)

`Sk8::Challenge::ChallengeVotingManager`, HUD `OnlineVotingHUD`:

- `StartVoting(votee)` 0x825bc4b8: enter the voting state; everyone except the votee sees the
  voting HUD.
- `Update` 0x825b1a18: for every player other than the votee whose input has focus, read that
  player's pad from the game input manager: input 139 → Bad, 136 → OK, 137 → Good,
  138 → Sick (`eChallengeVotingScore`). The local player's choice is mirrored to the HUD.
- `StopVoting` 0x825b8200: average the scores of the connected players other than the votee,
  post the result for the votee as a challenge event (`eEQChallenge`), hide the HUD, reset all
  votes to OK.

There is **no voting message**: remote players' pads reach the input manager through the
lockstep commands, so every console computes the same average on the same frame. A state-sync
design must send votes explicitly. Offline the template never votes (Party Play: voting needs
online and at least three players).

## 5. Free-skate activities

Online free skate lets a player propose an activity (accumulation, gap, group trick list,
simultaneous trick, survival, tag) that others accept; this is the `GameProposal` exchange
([messages.md](messages.md) §7, [matchmaking.md](matchmaking.md) §3). The activities are challenges
of type 42 for the location (`ChallengeOnlineManager::GetOnlineFreeSkateActivities` 0x825bf3d0);
how an accepted proposal starts its challenge on the RB side was not traced.

## 6. Ranked progression

- Before matchmaking, an expired skill is refreshed (`StateOnlineGetProfile`:
  `GetAndSaveMyExps`, `GetAndSaveMyOverallRanks`, `GetUserSkillLevel`); skills feed the PlayNow
  skill preferences.
- On joining, `GamePlayerAttributes(overall rank, challenge skill, overall skill)` goes to the
  RB.
- After a challenge, the client asks the profile web service for its new badge and rank
  (`HTTPRequest_SkateProfileUpdateAndGetNewBadgeAndRank(peer, challenge type)` in
  `DisplayFinalResults`), grants rank achievements (`StateOnlineGame::GrantOverallRankAchievements`)
  and saves. Ranked stats live in FESL ranking (Arcadia: `rank/GetStatsForOwners` with
  `EXP_*`/`GAMES_*` keys).
- `GameFinalResults` credits cash to the local player and progression points per row.
- Achievements tied to results: a ranked win with a positive score → `AT_Pwnsomen00bs`
  (`ReportLocalPlayerResults`); the players'-choice row in a ranked game → `AT_SkatersChoice`
  (`OnMessage` on `GameFinalResults`).

## 7. Spot leaderboards (single player, online)

`Sk8::Challenge::cOnlineLeaderboardManager` shows friends' best scores on spots while
free-skating signed in: it fetches the friend list at most every 120 s (`RequestOTSInfo`
0x825c03f8), asks FESL ranking for their best scores on the spot
(`Transaction_GetBestScoreForUsers`, `RequestScores` 0x825b9558), and `UploadOTSScore` 0x82883040
reports a new best (`Net::UpdateChallengeBestScore`). This is not part of an online session.

## 8. For the port

Everything here except the transport-dependent parts (who pauses whom, sync-point barriers,
votes travelling as pad input) can be kept as is. In a state-sync design:

- the RB's barriers (loaded, sync point, complete) become server-side barriers over explicit
  messages, as in the original;
- votes and any other input-derived decision need their own messages;
- `SwitchCamera` and the per-console local index stay client-side.
