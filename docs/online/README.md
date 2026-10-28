# Online — how the real Skate 2 does it

Reference for the day the port gets online play. It describes the original game's online
stack end to end: signing in, finding and joining a game, how the consoles stay in step,
how challenges run, and how a session ends. Facts come from the debug executable
`sk82_na_m.xex` (IDA, read-only; addresses are its virtual addresses) unless a section says
otherwise. The Arcadia server (a Plasma emulator, `arcadia-main/docs/`) is cited for what
only the wire shows; EA's own server software is lost, so its behaviour is reconstructed from
what the client sends and expects.

## Scope for the port

Online is the one place the port will not be 1:1 (decided 2026-10-07):

- **Kept 1:1:** lobby kinds, matchmaking choices, the online hub and its menus, challenge
  selection and voting, challenge rules, scoring, results, ranks.
- **Replaced:** the lockstep netcode. Skate 2 sends only pad input and simulates every skater on
  every console, so any difference between consoles (a physics mod, a float difference between
  PC and Xenon) desyncs the whole lobby and the game kicks the player
  (`GameModule::DoInputUpdate` 0x82730668). The port will send skater state instead
  ([remake-notes.md](remake-notes.md)).

These documents describe the original faithfully, including the parts that will be replaced,
so the rules layer can be ported 1:1 on top of a new transport.

## The architecture in one picture

```
 console A                      EA "RB" game server                      console B
 ─────────                      ───────────────────                      ─────────
 GameModule / SimController     GameManager host (ClientServer)         same as A
 Sk8::Net::OnlineNetwork        relays every GameSync to all peers
   GameSync (pad input) ──────► orchestrates the lobby:          ──────► OnlineNetwork
   requests, reports   ──────►   GameReset, GameAttributes,      ──────► StateOnlineGame
                       ◄──────    GameTimer, FinalResults, ...    ◄──────
 Fesl (Plasma SDK) over          registered with Theater                Fesl
 DirtySock: ProtoTunnel /        (lobby/game listing)
 CommUDP / NetGameLink           allocated by PlayNow "reset server"
        │                                   │
        └──── FESL (TCP/SSL): login, PlayNow, stats, friends ───────────┘
              Theater (TCP+UDP): game listing, join, player events
              HTTP: config, profiles, skate.Reel, Create-a-Spot
```

The key facts, each verified in the client:

1. **No console ever hosts a game.** PlayNow "create game" is
   `SESSION_TYPE_RESET_SERVER` with a pool of one (`PlayNowManager::SetupOptions` 0x8278d518):
   EA's service hands the player an idle dedicated server ("RB"; failure is
   `SER_PLAYNOW_NO_RB`, "No RB found", `StateOnlinePlayNow::CreateNewServer` 0x82767cf8).
2. **The server is the GameManager host and relays the lockstep.** Every GameSync is sent to
   destination `-2`, the host (`OnlineNetwork::SendCommands` 0x827a9a58 `li r5,-2`;
   `OutboundMessageQueue::FlushQueue` 0x827aacb8 → `GameManagerGame::SendToHost`). Nothing in
   the client re-broadcasts another player's GameSync, so the server must.
3. **The server runs the lobby.** Clients only ever build requests and reports
   (StartGame, change requests, proposals, results, recipes, ...). The messages that drive
   everyone — `GameReset`, `GameAttributes`, `GameTimer`, `GameAllPlayersComplete`,
   `GameFinalResults`, `GameExitPostChallenge`, `GameChange`, `GameRemovePlayer` — are never
   constructed by the client; they come from the server.
4. **The simulation is deterministic lockstep at 60 Hz** with an 8-bit world checksum per frame;
   see [lockstep.md](lockstep.md).

"Host" therefore means two different things in these docs: the **server** (GameManager host,
destination -2) and the **game host peer** — the player whose `GetGameHostPeerID` matches, who
gets the host menus and sends `StartGame` (`EventSystem::HandleStartGame` 0x8278de28).

## Documents

| Document | Contents |
| --- | --- |
| [lockstep.md](lockstep.md) | `Sk8::Net` simulation sync: commands, GameSync, queues, the 45-frame window, pace control, checksums and desync, resets, removal |
| [messages.md](messages.md) | every `Sk8::Net::Message` (types 1–25): who sends it, layout, what the receiver does |
| [sessions-and-transport.md](sessions-and-transport.md) | Plasma bring-up, joining through Theater, the host connection, relaying, the GameManager protocol, connection loss, the DirtySock layers |
| [matchmaking.md](matchmaking.md) | ways into a game, PlayNow (filters, preferences, thresholds, wire keys, results), the online hub |
| [game-flow.md](game-flow.md) | `StateOnlineGame`: join → recipe → lobby skate → challenge load → challenge → results → next challenge; joins, leaves, errors |
| [recipes.md](recipes.md) | skater appearance exchange (`StateOnlineRecipe`) |
| [online-challenges.md](online-challenges.md) | challenge types and pools, a round end to end, the Lua `Online` module, voting, free-skate activities, ranked progression, spot leaderboards |
| [services.md](services.md) | the `Sk8::Net` façade, sign-in, Plasma services, remote config, web services (Skate.Reel, profiles, Create-a-Spot) |
| [remake-notes.md](remake-notes.md) | what to keep, what to replace, what lockstep gave for free, reusable port pieces, server options, open questions |
| [implementation.md](implementation.md) | the port's online play as built: the dedicated server, matchmaking, the Lobby Browser, the replay-frame codec, the challenge facts lockstep, the screens that follow the original, and what is not implemented |
| [replay-data-netcode.md](replay-data-netcode.md) | feasibility of sending each skater's replay data (pose, root, audio record) plus challenge events instead of lockstep: sizes, bandwidth, what needs messages, how a remote puppet plugs into the port |

## Related material

- Arcadia's docs (`arcadia-main/docs`): byte-level transport
  (`protocol/prototunnel.md`, `commudp.md`, `netgamelink.md`, `gamemanager-handshake.md`) and the
  server's orchestration (`skate/*.md`). Server-side view; the docs here are the client's view.
- Port: `crates/skate2-core/src/session/selected_sim_command_queue.rs` ports the offline half of
  the same command pipeline (`OfflineNetwork`, `GenerateSyncCommand`).
- [Party Play](../partyplay/README.md): the local multiplayer, which shares the challenge
  templates (`Online.IsOnline` gates voting and other online-only branches).
