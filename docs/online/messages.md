# Messages (`Sk8::Net::Message`)

Everything Skate 2 says over the game connection, once the GameManager handshake is done, is a
`Sk8::Net::Message`. This is the catalogue: type byte, layout, who sends it, what the receiver
does. Addresses are `sk82_na_m.xex`.

"RB" is EA's name for the dedicated game server ("Sending new attributes to RB",
`StateOnlineGame::UpdateNextChallenge` 0x82754578). Clients address it as destination **-2**
(the GameManager host). The RB's own code is lost; its side of each exchange is inferred from
what the client sends and expects, with Arcadia's reimplementation as the reference.

## 1. Framing

- `Message::Create(type)` 0x82a6d350 is the factory; types are 1–25 (`Sk8::Net::MessageType`
  `MT_GameSync`..`MT_GameRecipeData`), anything else asserts.
- On the wire each message is its type byte followed by its `Pack` output (`operator<<`
  0x82a6c4b0 / `operator>>` 0x82a6d768). Several messages are concatenated into one
  GameManager packet of at most 1199 bytes (`OutboundMessageQueue::FlushQueue` 0x827aacb8).
- Integers are written in native order: **big-endian** on Xbox 360 and PS3.
- Strings (`ByteStream::WriteString`/`ReadString`) are bounded to 64 bytes on read
  (`MAX_ATTRIBUTE_VALUE_SIZE`).
- vtable layout of every message: 0 destructor, 1 `GetType`, 2 `GetName`, 3 `Reliable`,
  4 `Pack`, 5 `Unpack`. `Reliable` is the same folded `return 1` for all 25 types, so every
  message goes on the reliable channel (NetGameLink kind 6).
- Destinations (`OutboundMessageQueue::QueueMessage` 0x827aaee8): **-2** the RB, **-1** every
  other player, otherwise one player's user id (sent directly, or relayed by the RB with
  `PLAYER_RELAY_REQUEST` when there is no direct link; see sessions-and-transport.md).
- Receivers: `OnlineNetwork::OnMessage` 0x827a9c40 (lockstep), `StateOnlineGame::OnMessage`
  0x8276b060 (session), `StateOnlineRecipe::OnMessage` 0x8276e0d0 (recipes),
  `FrontEndState_OnlineHubMenu::OnMessage` 0x82675cb8 (free-skate activities),
  `OnlineEventLog::OnMessage` 0x82619e80 (join notices).

## 2. Summary

C→RB: built by the client and sent to -2. RB→C: never built by the client, only received.
C→C: addressed to a player.

| # | Message | Dir | Layout after the type byte | Purpose |
| ---: | --- | --- | --- | --- |
| 1 | GameSync | C→RB→all | flags u8, checksum u8, size u8/u16, data | one player's commands for one frame ([lockstep.md](lockstep.md) §5) |
| 2 | GameReset | RB→C | peer ids u64[8], reset type u32, random seed u32, session id u32, activity u64 | (re)start the lockstep: lobby skate, challenge load, challenge |
| 3 | GameRequest | C→RB, RB→C, C→C | request u8, value u64 | small commands, table §3 |
| 4 | GameAttributes | RB→C | 9 strings | the game's attributes (§4) |
| 5 | GameAttributeUpdate | RB→C | attribute u32, string | one attribute changed |
| 6 | GameComplete | C→RB | — | I finished the challenge |
| 7 | GameResults | C→RB | event time f32, score s32, finish reason u32, ranking u16, players' choice u8 | my result |
| 8 | GameAllPlayersComplete | RB→C | — | everyone finished |
| 9 | GameFinalResults | RB→C | count u32, then per player (§5) | the results board |
| 10 | GameTimer | RB→C | timer type u32, value u64 | lobby countdowns (§6) |
| 11 | GameExitPostChallenge | RB→C | — | leave the results screen |
| 12 | GameLoadRequest | C→RB, RB→C | request number u32 | load handshake sequence number |
| 13 | GameChallengeLoaded | C→RB | — | my challenge assets are loaded |
| 14 | GameResetAttributes | C→RB | 9 strings | game host peer: attributes for the next challenge |
| 15 | GameRequestReset | — | reset type u32 | defined, never built by the client |
| 16 | GameRequestChange | C→RB | challenge type u32, challenge key u64 | game host peer: change challenge/location |
| 17 | GameSyncPoint | C→RB, RB→C | — | round barrier inside a challenge |
| 18 | GameChange | RB→C | challenge type u32, challenge key u64, change time u64 | a change is scheduled |
| 19 | GameRemovePlayer | RB→C | user id u64, last frame u32 | a player left |
| 20 | GamePlayerAttributes | C→RB | rank u32, challenge exp u32, total exp u32 | my ranked standing |
| 21 | GameWager | RB→C | wager type u32, amount u32, user u64 | wagers (only logged) |
| 22 | GameProposal | C→RB, RB→C | proposal type u32, activity u64, proposer u64, timeout u64, debug u8 | free-skate activity proposals (§7) |
| 23 | GameRecipeRequest | C→RB | peer id u64 | ask for a player's appearance recipe |
| 24 | GameRecipeHead | C→RB, RB→C | peer id u64, size u32, crc u32 | recipe header (size 0 = none) |
| 25 | GameRecipeData | C→RB, RB→C | peer id u64, size u32, chunk u32, data[size] | one recipe chunk |

Folded serializers: 4 and 14 share layout (`ResetAttributes::Pack` 0x82a6aea8,
`Attributes::Unpack` 0x82a6aee0); 16 uses `GameTimer::Pack/Unpack` 0x82a6ce98/0x82a6cf28;
15 unpacks with `GameLoadRequest::Unpack` 0x82a6cf10; 6, 8, 11, 13 and 17 have the folded
empty `Pack`/`Unpack` (type byte only).

## 3. GameRequest (3)

`mRequest` is `Sk8::Net::RequestType`; `mValue` is 64-bit.

| Request | Value | Sent by | Effect |
| ---: | --- | --- | --- |
| 1 StartGame | — | game host peer, to the RB (`EventSystem::HandleStartGame` 0x8278de28, `OnlineHubMenu::GoToStartChallenge` 0x826617a0) | start the challenge in the attributes |
| 2 DebugReset | — | debug menu | — |
| 3 ChangePrivateStatus | new private flag | game host peer, to the RB (`GoToToggleSlotAccess` 0x8266c998) | the RB is expected to answer with `GameAttributeUpdate(IsPrivate)`, on which every client logs "session made private/public" |
| 4 KickPlayer | target user id | game host peer, **to the target** (`Net::KickPlayer` 0x8278dfc0, `mr r5,r30`) | the target shows the "lost connection" chyron and leaves; hub proposal state reset |
| 5 Pause | 1 pause, 0 resume | any client in `STATE_OnlineGame`, to the RB (`Net::PauseGame` 0x82791a88 / `ResumeGame` 0x82791ca8) | announces an expected stall (intros, sync points); the sender also sets PauseMan bits 1, 4, 0x10 locally |
| 6 DirtyDiscErrorKickMe | — | `XenonFileErrorCallback` 0x827386c0 (disc read error), to the RB | ask to be removed |
| 7 Heartbeat | — | `OnlineNetwork::UpdateHeartBeat`, to the RB, after 1 s without a GameSync | keep-alive |
| 8 EditReplay | — | — | defined |
| 9 SetSessionId | `(JuiceSessionID << 8) \| gameIndex++` | game host peer (`SendSessionId` 0x8274d420) after `GameResetAttributes` | telemetry session id; echoed in `GameReset.mSessionId` |

Arcadia names request 4 "LostConnection" and 6 "ConvertPrivateSession"; the symbols say
`Request_KickPlayer` and `Request_DirtyDiscErrorKickMe` (`KickReason`: `KR_KickedByGameHost` 4,
`KR_KickedForInactivity` 5, `KR_KickedDirtyDisc` 6).

## 4. Attributes (4, 5, 14)

Nine strings, index = `Sk8::Net::AttributeType`, read back with `Net::GetAttributeValue`
0x82a6a710 (`SetAttributeValue` 0x82a6a818):

| # | Attribute | Meaning |
| ---: | --- | --- |
| 0 | GameVersion | build/version tag |
| 1 | ChallengeType | `Sk8::Net::ChallengeType` name (`GetChallengeType(const char*)` 0x82a6aa78) |
| 2 | ChallengeKey | current challenge collection key (`AttribSysUtils::KeyToString`) |
| 3 | PingSite | matchmaking ping site |
| 4 | IsPrivate | "true"/"false" |
| 5 | MaxPlayers | |
| 6 | IsRanked | |
| 7 | OverallSkill | |
| 8 | ChallengeSkill | |

`Sk8::Net::ChallengeType`: OwnTheSpot 0, Slap 1, BestTrick 2, Jam 3, OneUp 4, SKATE 5,
DeathRace 6, HallOfMeat 7, OnlineFreeSkate 8.

- `GameAttributes` (RB→C) replaces all nine (`StateOnlineGame::OnMessage` keeps the message and
  sets every value).
- `GameAttributeUpdate` (RB→C) replaces one; IsPrivate also writes the event log.
- `GameResetAttributes` (C→RB) is the game host peer's proposal for the next challenge
  (`UpdateNextChallenge`: ranked picks a random key of the same challenge type, unranked keeps
  the current key).

## 5. Results (6, 7, 8, 9, 11)

`GameResults` carries `Sk8::Net` `GameResult`: `mEventTime` f32, `mScore` s32,
`mFinishReason` (`eFinishReason`: NotFinished 0, NormalCompletion 1, TimeUp 2,
PlayerDisconnect 3, KnockedOut 4, Inactive 5), `mRanking` u16, `mIsPlayersChoice` u8. The client
sends a cached copy (`ReportLocalPlayerResults` 0x8274ce30); it also grants achievements and
records the completion there.

`GameFinalResults` (`Pack` 0x82a6cca0) is `mPlayerCount` u32 followed by, per player, 39 bytes:
peer id u64, the five `GameResult` fields, then `cash` s32, `exp` f32, `wager` s32,
`winnings` s32. In memory each row is a 48-byte `Sk8::Net::PlayerStatsData` (peer id +0,
result +8, wager +0x18, winnings +0x1C, cash +0x20, exp +0x24, rank +0x28; `rank` is not
sent); the constructor 0x82a6b090 zeroes everything and sets wager -1. The array holds 8 rows
and the count is not range-checked. On receipt
(`StateOnlineGame::OnMessage` case 9) the client builds the results leaderboard (layout chosen
by challenge type: Death Race, free-skate accumulation/gap/tag/simultaneous-trick), maps each
peer id to a player index, sets progression points, credits the local player's cash
(`cSignPostManager::AddCash`), and in ranked grants "Skater's Choice" to the players'-choice
row.

The order is: each client `GameComplete` → RB `GameAllPlayersComplete` → each client
`GameResults` → RB `GameFinalResults` → RB `GameTimer(DisplayResults)` → RB
`GameExitPostChallenge`. The state machine side is in game-flow.md.

## 6. Timers and changes (10, 18)

`GameTimer` (`Sk8::Net::TimerType`):

| Type | Value | Client effect |
| ---: | --- | --- |
| 1 LockTime | time | `OnlineBackEnd::mGameLockTime`; online timer HUD mode 1 when ranked and non-zero, else 0 (ranked) / 4 |
| 2 StartTime | time | `mGameStartTime` |
| 3 HurryTime | — | timer HUD mode 2 |
| 4 DisplayResults | seconds | `OnlineBackEnd::StartPostEventTimer` |

`GameChange` (RB→C) after a `GameRequestChange`: timer HUD mode 3, an event-log location
change message, and the change time stored in the online back end (the countdown until the
world switches). The switch itself is the next `GameAttributes` + `GameReset`.

## 7. Free-skate activity proposals (22)

`Message_GameProposal::ProposalType`: Propose 0, Accept 1, Broadcast 2, Timeout 3, Missed 4.

1. A player picks an activity in the hub (`GoToStartFreeskateActivity` 0x8266c728; needs at
   least two participants, no open proposal, no challenge in progress, else popup
   `Online_NeedMorePlayers`): `Propose` with the activity key and proposer = local peer → RB.
2. RB → everyone `Broadcast`: the proposer sees the activity overlay; everyone else sees
   "ID_ONLINE_EVENT_FREESKATE_PROPOSED" and the sign-up overlay
   (`FrontEndState_FreeskateActivitySignUp`).
3. Players accept (`FreeskateActivitySignUp::ReceiveInput` 0x826a5648) → RB; RB → `Accept` to
   show "ID_ONLINE_EVENT_FREESKATE_ACCEPTED" with the name.
4. `Timeout` (no takers) or `Missed` (an activity already running): the proposal is cleared and
   logged. A challenge reset, a kick or a `GameChange` also clears it.

## 8. Load and sync points (12, 13, 17)

- `GameLoadRequest`: the client sends `mCurrentLoadSequenceNumber` (asserting it changed,
  `SendLoadRequest` 0x8274d298); a received one stores the number (`OnMessage` case 12).
- `GameChallengeLoaded` (C→RB) after a challenge-load reset finished loading
  (`ChallengeLoadState` 0x82771a28); the RB then sends the challenge reset (Arcadia does so once
  every participant has reported).
- `GameSyncPoint`: sent from `ChallengeSkate` 0x82776a78 when a challenge round reaches a
  barrier (Lua `Online.OnlineChallengeRoundSyncPoint` 0x8288adb8); the RB's `GameSyncPoint`
  means everyone arrived: unpause (PauseMan pull `kPAUSE_STATE_ONLINE`, `ResumeGame`), resend
  `cMsgSetPlayer`, unspawn AI, and post `cMsgPostChallengeDisplayExit` for the activity.

## 9. Recipes (23, 24, 25)

Appearance data exchange; detail in recipes.md. The client uploads its own recipe as a head plus
chunks, requests others by peer id, and accepts a remote recipe only as a head followed by
chunks numbered from 0 for the requested peer (`StateOnlineRecipe::OnMessage` asserts both).
A `GameRecipeRequest` arriving at `StateOnlineGame` sets its `mNewError` flag
(`OnMessage` case 23).

## 10. Not handled by the client

`GameComplete`, `GameResults`, `GameChallengeLoaded`, `GameResetAttributes`,
`GameRequestChange`, `GamePlayerAttributes` reach no client handler (they are for the RB).
`GameRequestReset` is never built. `GameWager` is only logged ("MT_GameWager!").
