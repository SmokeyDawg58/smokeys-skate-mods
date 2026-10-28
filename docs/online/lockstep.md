# Lockstep simulation (`Sk8::Net`)

Skate 2 online is deterministic lockstep. Every console simulates every skater and the whole
world; the only gameplay data that travels is each player's per-frame *commands* (normally just
the pad state). A frame is simulated only when every player's commands for it have arrived, and
each console checksums its world so a divergence is detected and the player is kicked.

Addresses are `sk82_na_m.xex`. The offline game runs through the same interface with a
pass-through network, which the port already has (`selected_sim_command_queue.rs`).

## 1. Pieces

| Native | Role |
| --- | --- |
| `Sk8::GameModule` (`Update` 0x8273edc8, `PreUpdate` 0x82729e40, `DoGameUpdate` 0x82730418, `DoInputUpdate` 0x82730668, `ResetGame` 0x8273a680) | the per-tick driver; owns the `SimController` and the player UI |
| `Sk8::SimController` (`Update1` 0x828a72c8, `Update2` 0x828a0af8, `GenerateSyncCommand` 0x828a89a0, `DoCommand` 0x828a6470, `Reset` 0x828a2c68, `ResetNetwork` 0x828a21d8, `IsSimulationStalled` 0x828a0a30, `NeedsUpdate` 0x828a09a8) | collects local commands, hands them to the network, runs the simulation on the commands the network releases |
| `Sk8::INetwork` | `Update`, `SendCommands`, `GetCommands`, `OnSimulate`, `HasUpdate`, `IsInSync`, `OnReset`, `ShouldCatchup`, `GetAvgLatency`, `OnEvent` (vtable slots 1, 3–10, 12; `IsInSync` is +0x1C) |
| `Sk8::Net::OfflineNetwork` (ctor 0x82793618) | single player, ghost and replay: one frame of commands, returned as-is |
| `Sk8::Net::OnlineNetwork` (ctor 0x827a9730, 0x5C8 bytes) | online: per-player queues, aggregation, pace control, sync checking |
| `Sk8::Net::CommandQueue` (ctor 0x827a0ff0, 0x28A50 bytes) | one player's frames, FIFO of at most 60 |
| `Sk8::Net::SyncChecker` (ctor 0x827a2758) | the send window and the checksum comparison |
| `Sk8::Net::MessageSystem` (ctor 0x827a24f8) with `InboundMessageQueue` and `OutboundMessageQueue` | batching, sending and receiving `Sk8::Net::Message`s |
| `Sk8::Net::Message_GameSync` (`Pack` 0x82a6c528, `Unpack` 0x82a6c620) | one player's commands for one frame |

## 2. Which network

`SimController::ResetNetwork` (run by every `SimController::Reset`) picks the network from
`CommonBackEnd::sGameInfo` (0x83040500):

- `isNetworkGame && !playbackGame` → `NT_MP`, a new `OnlineNetwork(sGameInfo.syncLevel)`;
- otherwise `NT_SP`, or `NT_Ghost` when the ghost system is enabled → `OfflineNetwork`;
- if the type did not change, the existing network gets `OnReset()` instead.

`StateOnlineGame::SetupInitalGameInfo` 0x8275db08 sets `isNetworkGame = 1`,
`sGhostSystemEnabled = 1`, and `syncLevel = kbDebugOnlineDeSyncs ? SL_High : SL_Low`. The cvar
(section "Online Debug", default false) means retail always runs `SL_Low`: checksums are on.
`GameModule::BackToSP` 0x8273da50 clears `isNetworkGame`, `isOnlineRace` and `syncLevel`.

## 3. Commands

`Sk8::Command` is 268 bytes: `mType` u32, `mReceiverID` u32, `mDataSize` u32, `mData[255]`.
A frame holds up to 10 inline (`fixed_vector<Command,10>`, overflow allowed). Producers call
`ICommandProxy::DoCommand`, which for the ordinary proxy is `SimController::DoCommand`
(copy into `mCommands`).

| `mType` | Data (`SetData<T>`) | Payload |
| ---: | --- | --- |
| 1 | `PlayerInputData` | packed pad state, below |
| 2 | `PlayerInputGainFocusData` | empty |
| 3 | `PlayerSpawnData` | player index, spawn matrix |
| 4 | `PlayerUnSpawnData` | player index, `mDisconnected` |
| 5 | `PlayerTeleportData` | player index, foot forward, matrix, on board, session-marker teleport |
| 6 | `PlayerWipeoutData` | player index |
| 7 | `PlayerPinData` | player index, pin |
| 8 | `PlayerSetTransformData` | player index, matrix |
| 9 | `GhostSpawnData` | sim player index, matrix |
| 10 / 11 | `AiSpawnData` / `AiUnSpawnData` | AI skater (recipe id, matrix, client type, profile) |
| 13 / 14 | `SkateboardContactsData` / `SkeletonContactsData` | contact sets |

A player's input command has `mReceiverID = player slot + 10`.

**Pad packing** (`PlayerInputData::Pack` 0x82a8c070, `WriteInput` 0x82a8b8b8). The 24-float
`InputDevices::CacheLine` becomes `CompressedInput`: digital buttons 12–15 and 6–9 → bits of
`B1`, buttons 0–5 → bits of `B2`, the four stick axes as signed bytes
(`(pos − neg) × 127`, pairs 16/17, 18/19, 20/21, 22/23 → LX, LY, RX, RY), triggers 10/11 as
`× 255` bytes. `WriteInput` writes a mask byte (bit per non-zero field: LX, LY, RX, RY, LT, RT,
B1, B2 = 0x01…0x80) followed only by the non-zero fields. An idle pad is 1 byte, a full one 9.
Without focus nothing is written; `Unpack` 0x82a8c330 then zeroes the pad and sets button 13
to 1.0.

## 4. The tick

The timer thread wakes the sim thread at `XBox2ExecutionMan::SetTimerHz` (60 Hz, adjusted
online, §8); an arriving message also wakes it (`InboundMessageQueue::OnReceive`). Each tick
`GameModule::PreUpdate` decides between an *input* tick and a *catch-up* tick (catch-up only
when `SimController::NeedsUpdate`, i.e. the network has a frame ready). `GameModule::Update`
then runs:

1. `DoInputUpdate` (input ticks): the desync reaction, §9.
2. `DoGameUpdate`: on input ticks the player UI reads the pad and queues commands
   (`IGameUI::Update`); unless the sim is paused, `SimController::GenerateSyncCommand` sends
   them. On catch-up ticks it re-tries the send only when the last one failed or
   `INetwork::ShouldCatchup`.
3. Only while `mInSync`: `SimController::Update1` — `INetwork::Update`, store
   `GetAvgLatency` into `Simulation::mAvgLatency`, `NISManager::Update`, then
   `GetCommands(mTick + 1)`. If commands come back, each is executed through
   `CommandManager::DoCommand` (one `Simulation::GetRandom` draw per command, RNG unlocked) and
   `SimUpdate1` runs.
4. `SimController::Update2`: `SimUpdate2`, re-lock the RNG, and after a real frame
   `INetwork::OnSimulate(mTick)` (checksum, §9).

`GenerateSyncCommand` sends `mCommands` as frame `mNextGenFrame` (starts at 1 on every reset);
only an accepted send advances the counter and clears the queue. So input generation runs
ahead of the simulation by up to the send window.

## 5. Sending a frame

`OnlineNetwork::SendCommands` 0x827a9a58:

1. If at most one *free checksum* remains (§9), the send is refused: `mMissedSendCommands++`,
   "Warning: Send Commands failed!", heartbeat check, return false. This is the back-pressure:
   a console can be at most `kMaxSyncCommands` = 45 frames (0.75 s) ahead of its own simulation,
   which itself waits for the slowest peer.
2. Otherwise build a `Message_GameSync`: `mFirstFrame = (frame == 1)`,
   `mCheckSum = GetFreeChecksum()`, `mPlayer` = the local queue's slot. The commands are packed
   only when other players exist (`PackGameCommands` 0x8278e108).
3. `Core::SendMessage(msg, -2, no flush)`: destination **-2 = the GameManager host** (the server).
4. Append the same commands to the local queue (frame numbers are implicit, §6), restart the
   heartbeat timer.

**Payload** (`PackGameCommands`): a single type-1 command for this player is written as its raw
data and the message is flagged *only input*; anything else is `count`, then per command
`receiver`, `type`, `size` (one byte each) and the data. At most 1 KB.

**Wire** (`Message_GameSync::Pack`), after the type byte 0x01:

```
flags     u8   bits 0-3 player slot, 0x20 only-input, 0x40 two-byte size, 0x80 first frame
checksum  u8   SyncChecker byte (§9)
size      u8 or u16 (0x40 set when size >= 0xFF)
data      size bytes
```

There is **no frame number**. Each player's GameSyncs are numbered by arrival order
(`CommandQueue::AddCommands` stamps `mNextFrame++`), so the transport must deliver every
GameSync exactly once and in order. One reordered GameSync permutes a player's input and
desyncs everyone (this is the root cause of Arcadia's 2026-07 relay-order bug).

## 6. Messages in and out

`MessageSystem` queues outgoing messages per destination in a reliable and an unreliable map
(`OutboundMessageQueue::QueueMessage` 0x827aaee8; destination -1 = every other player, -2 = the
host, otherwise a player). `OnIdle` 0x827ab120 flushes both maps every `mFlushRate` =
1 / `SendRate` seconds (`OnConfigLoaded` 0x82789028; default `SendRate` 30 → 33 ms), packing as
many messages as fit into one 1199-byte buffer per destination (`FlushQueue` 0x827aacb8). So at
30 Hz a GameSync packet normally carries two frames.

Inbound, `InboundMessageQueue::OnReceive` 0x827a4540 (network thread) decodes every message in
the datagram (`Message::Create` by type byte), queues them under a mutex and wakes the sim
thread. `Flush` 0x827a4258 (sim thread) resolves the sender to a player id and dispatches to the
listeners (`OnlineNetwork`, `StateOnlineGame`, `StateOnlineRecipe`, ...).

While **paused** (`MessageSystem::PauseGameSync` 0x82793598), arriving GameSyncs are kept aside
and a GameReset discards the kept ones. The queue starts paused, `OnlineNetwork`'s destructor
pauses it and its constructor un-pauses it with itself as the listener, which replays the kept
GameSyncs into the new network: frames that arrive while no lockstep network exists (before the
first reset, or while it is being rebuilt) are not lost.

`OnlineNetwork::OnMessage` 0x827a9c40:

- **GameSync (1):** while a reset is in progress it is parked in `mMissedSyncs`. Otherwise the
  slot in its flags picks the queue; the commands are unpacked (`UnpackGameCommands`
  0x82796428), appended to that queue, and the checksum byte goes to the SyncChecker for that
  peer.
- **GameReset (2):** destroy every queue, drop the local queue, set `mResetInProgress`.
- **GameRemovePlayer (19):** the leaver's queue gets `OnRemove(mLastFrame)` (keeps its frames up
  to and including the player's last frame, else clears) and the peer leaves the SyncChecker.

## 7. Advancing

`OnlineNetwork::Update` 0x827aab20 = `UpdateAggregate`, `UpdateHeartBeat`, `UpdatePing`,
`UpdateInputRate`.

`UpdateAggregate` 0x827a9f18 merges the per-player queues into the *aggregate* queue one frame
at a time: the next frame is aggregated only when **every** queue's head is that frame; their
commands are concatenated in slot order and every head is popped. A queue that is marked removed
and empty is dropped. This is the lockstep AND-gate: one silent player stops everyone.

`GetCommands(frame)` 0x827a3dc8 releases a frame to the simulation only if:

1. a local checksum is pending (this console sent its own frame), and
2. with other players present, the aggregate queue holds **more than** `mRecvPerUpdate` frames
   (= 60 / `SendRate` = 2): a small jitter buffer; then
3. the aggregate head is that frame. `DoSyncChecking(frame)` runs on release.

`HasUpdate` 0x827895a0 is true when every player queue has at least one frame. `ShouldCatchup`
0x827896a8 applies only without a local queue (spectating): catch up when the aggregate holds
`2 × mRecvPerUpdate` frames. `OnSimulate` 0x8279d2b0 pops the aggregate head after the frame
ran (it asserts the frame matches) and generates the checksum.

## 8. Pace control

- **Input rate** (`UpdateInputRate` 0x8278e4b8, read from the instructions; the decompile drops
  the `fsel` operands). Alone, the sim runs at exactly 60 Hz. With others, the free-checksum
  count (45 minus the frames this console has sent but not yet simulated) is compared with a
  target `t = min(max(45 × 0.333, pingMean × 60 / 1000 + 0.5), 44)`; the sample
  `clamp(free / t, 0.96, 1.04)` is smoothed (`rate = 0.9 rate + 0.1 sample`) and the timer is
  set to `(int)(rate × 60 + 0.5)` Hz. A console whose input runs far ahead of what its peers let
  it simulate slows by up to 4 %; one with headroom speeds up by up to 4 %.
- **Heartbeat** (`UpdateHeartBeat` 0x8278e308). With no GameSync sent for 1.0 s, send a
  `GameRequest` with request 7 to the host.
- **Ping** (`UpdatePing` 0x8278e400): exponential mean and variance (α 0.001) of the
  GameManager connection ping.
- **Late packets** (`UpdatePacketArrival` 0x82784590): mean and variance (α 0.004) of the time
  between aggregated frames when it exceeds the expected period (`mRecvPerUpdate × 16.67` ms)
  and is under 300 ms.
- **Latency** (`GetAvgLatency` 0x82789738), in frames:
  `ceil((pingMean × 0.25 + 2·sqrt(lateVar) + lateMean − expected) / 1000 × 60)`; 0 when alone.
  It feeds `Simulation::mAvgLatency`.
- **Online sim rate.** `Simulation::OnRequestSetSimRate` 0x8289e350 forces 1/60 s in a network
  game: no slow motion online.
- **Stall UI.** `SimController::IsSimulationStalled` is true when no frame ran for
  `kSimulationStallTime` (cvar section "Online", default 1.0 s, range 0.1–1000) and the sim is
  not paused. `StateOnlineGame::UpdateChyron` 0x82748858 then opens
  `eSk8FEPopup_Online_WaitForPlayersSmall` ("waiting for players") and closes it when frames
  flow again.

Tuning comes from the server's config (`Sk8::Xml::GamePlaySettings`; defaults in
`Net::GetDefaultGamePlaySettings` 0x8278d7a0): `SendRate` 30, `ScaleFactor` 1.0,
`SlowDownFactor` 1.15, `SpeedUpFactor` 0.85.

## 9. Checksums and desync

`SyncChecker` holds three things: `mFreeChecksums` (deque of bytes), `mLocalChecksums` (bytes
sent, waiting to be compared) and `mRemoteChecksums` (one deque per peer).

- `Reset` 0x827a10e0 fills `mFreeChecksums` with `mQueueSize` = `kMaxSyncCommands` = 45 zeros.
- `GenerateChecksum(frame)` 0x82799aa0, after each simulated frame: with a sync level, build a
  `SyncLog(level, SS_World, Sk8::GetCRC)`, let `Simulation::GetSyncLog` 0x8289ffe8 fill it, keep
  the log by frame, and push its 8-bit `mCRC` onto `mFreeChecksums`.
- `GetFreeChecksum` 0x827936c8, on each send: pop the oldest free checksum, remember it in
  `mLocalChecksums`, put it in the GameSync.
- `AddChecksum(peer, byte)` 0x82793770, on each received GameSync.
- `DoSyncChecking(frame)` 0x82799898, when a frame is released: pop the oldest local checksum
  and the oldest of every peer; any difference clears `mInSync` (debug builds also write
  `syncerror-frame<frame−45>-<user>.log`); the frame-45 log is discarded.

So the GameSync for frame N carries the checksum of simulated frame N − 45 (zero for the first
45 frames), and a divergence is noticed about 0.75 s later.

**What is checksummed** (`Simulation::GetSyncLog`, hash `crc = crc × 33 + crc(field)`): the sim
tick, the RNG seed and index, `IsNetworkGame`, the `CommandManager`'s log (the commands
themselves only from `SL_Med` up, `CommandManager::DoCommand` 0x825e5610), every entity's
`GetSyncLog` (skaters: `Actor::GetSyncLog`), `Physics::World::GetSyncLog`, and
`Dmo::SimManager::GetSyncLog` (movable objects).

**Reaction** (`GameModule::DoInputUpdate` 0x82730668): on the first input tick where
`IsInSync()` turns false, Juice status "DESYNC",
`Sk8::Net::ShowOnlineError(eONLINE_ERROR_KICKED_UNKNOWN)`, and a SYNC/GAME log event with the
sim tick. In game, `GameModule::Update` runs `SimController::Update1/Update2` only while
`mInSync`, so the local simulation freezes behind the error. This is the "kicked for unknown
reason" a modded or divergent player gets.

## 10. Resets

The server's `GameReset` restarts the lockstep for everyone:

1. `OnlineNetwork::OnMessage` (type 2): queues destroyed, `mResetInProgress` set; GameSyncs that
   arrive from now on are parked in `mMissedSyncs`.
2. `StateOnlineGame` takes the reset (`HandleLobbySkateReset` 0x82776f90): rebuild the players
   from the reset's slot table (`UpdateGameInfo` 0x827538c8), then by type — lobby skate 0 and
   challenge 2 call `GameModule::ResetGame(..., mRandomSeed)`; challenge load 1 only loads
   (`ChallengeLoadState`).
3. `ResetGame` → `SimController::Reset(seed)`: commands cleared, `mNextGenFrame = 1`,
   `Simulation::Reset(seed)` (shared RNG), `ResetNetwork` → `OnlineNetwork::OnReset` 0x827a7438.
4. `OnReset`: `CleanUp` (SyncChecker reset to 45 zero checksums, queues dropped), then one
   `CommandQueue(peerId, slot)` per human slot 0–7 of `PlayersBackEnd` (asserting a valid peer
   id), the local one becoming `mLocalQueue`, the others `SyncChecker::AddPeer`; a fresh
   aggregate queue (id -1, index 8); finally, if a reset was in progress, the parked
   `mMissedSyncs` are replayed through `OnMessage` and cleared.

Every console therefore restarts at frame 1 with the same seed, the same players in the same
slots, and empty queues; the first GameSync of each player carries the first-frame flag. Any
GameSync that belongs to the previous epoch but arrives after the rebuild lands in the new queues
and wedges the lockstep, which is why a relay must fence the reset boundary (Arcadia's
FirstFrame barrier, `arcadia docs/skate/lockstep-and-relay.md`).

## 11. Leaving

The server announces a departure with `GameRemovePlayer` carrying the leaver's last frame. Peers
keep simulating that player up to that frame, then drop the queue
(`CommandQueue::OnRemove` 0x82799d20, `UpdateAggregate`); the SyncChecker stops expecting its
checksums. The game state side (`StateOnlineGame::HandlePlayerLeave` 0x8276aa70) is covered in
game-flow.md.

## 12. What this means for the port

- The pad stream is tiny (1–9 bytes a frame) and deterministic replay of it is all the original
  relies on; any non-determinism (mods, float differences, frame-rate-dependent code) breaks it.
  The port cannot be bit-identical with Xenon, so cross-play with consoles is impossible either
  way.
- The command vocabulary (§3) is still the right *event* vocabulary: spawn, teleport, pin,
  wipeout and set-transform commands are exactly the discrete events a state-sync design must
  also carry. Teleports and pins issued by challenges are commands here; in a state-sync design
  the owner applies them and the result travels as state.
- The checksum (§9) is a ready-made consistency probe for testing a port of the deterministic
  path, even if the shipped design no longer uses lockstep.
