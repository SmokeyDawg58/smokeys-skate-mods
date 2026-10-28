# Remake notes

What the findings in this folder mean for the port's own online play. Decision (2026-10-07):
keep the online *rules* 1:1, replace the *netcode*. What was built from these notes is in
[implementation.md](implementation.md).

## 1. What the original is, in one paragraph

A dedicated server per game (EA's "RB") hosts the GameManager session, runs the lobby state
machine (resets, timers, barriers, results) and relays every player's per-frame pad input. Every
console simulates every skater and the whole world in lockstep at 60 Hz from those inputs and a
shared seed, checks an 8-bit world checksum, and kicks itself on a mismatch. Players never host;
a "host" player is only the lobby leader who picks challenges.

## 2. Keep 1:1

| Area | Where it is described |
| --- | --- |
| Hub menus, quick/custom/ranked/unranked matchmaking choices, max players per mode, private sessions | [matchmaking.md](matchmaking.md) |
| The lobby state machine: lobby skate → challenge load → challenge → results → next challenge; join barrier; leader role; kicks, privacy, location change | [game-flow.md](game-flow.md), [messages.md](messages.md) |
| Challenge pools, rotation, the Lua `Online.*` hooks, sync points, completion and results, voting rules, free-skate activities | [online-challenges.md](online-challenges.md) |
| Recipes (appearance exchange, defaults, six cached) | [recipes.md](recipes.md) |
| Results board, cash and progression, ranked badges/ranks, achievements | [online-challenges.md](online-challenges.md) §6 |

The server keeps the RB's role: it is the authority for the session state and the barriers.

## 3. Replace

The lockstep: `OnlineNetwork`, `CommandQueue` aggregation, the 45-frame send window, the input
rate controller, `SyncChecker` and the desync kick ([lockstep.md](lockstep.md)). In its place:

- each client simulates only its own skater and sends snapshots (root, board, compact pose or
  animation-graph state, contact flags) at a fixed rate;
- remote skaters are interpolated puppets, rendered a little in the past;
- discrete events travel as reliable messages.

The replay recorder's per-tick skater data is a ready-made snapshot format; see
[replay-data-netcode.md](replay-data-netcode.md).

## 4. Things lockstep gave for free that now need messages

| In the original | Why it worked | In a state-sync design |
| --- | --- | --- |
| Spawn, unspawn, teleport, pin, wipeout, set-transform commands (`Sk8::Command` types 3–8) | applied at the same aggregated frame everywhere | the owner applies them to its skater; the result arrives as state; session-level ones (spawn positions, teleports between rounds) come from the server |
| Voting (`ChallengeVotingManager` reads every player's pad) | every pad is in the lockstep input | explicit vote messages, averaged by the server or the leader |
| Leaving players (`HandlePlayerLeave` injects an unspawn command) | the command lands on the same frame everywhere | the server's "player left" removes the puppet |
| Challenge evaluation (every console runs every player's challenge logic) | deterministic replay | each client evaluates its own skater (score, landing in spot volumes, bails) and reports events; the server or leader runs the arbitration the template does (S.K.A.T.E. letters, Spot Battle best scores, Hall of Meat damage) |
| Shared world (traffic, pedestrians, movable objects) | simulated identically from the seed | local cosmetic only, or switched off online; movable objects owned by whoever touched them last |
| Skater-to-skater contact | deterministic physics | resolve on the owner against the other skaters' puppets |
| Timing (the server's `GameReset` restarts everyone at frame 1) | a shared frame counter | a server clock for countdowns and race timing (Death Race needs a synced start and server-timed finish) |

The challenge templates already separate "my skater" from "everyone" in places that Party Play
exercises (turns, `SwitchCamera`, per-player HUD binding); that work carries over.

## 5. Reusable from the port

- The offline command path (`selected_sim_command_queue.rs`): the command producers are the
  event vocabulary.
- Party Play: several skaters in one session, turn hand-over, HUD binding per player, results
  screens.
- `RecipeData` (`skate2-formats`): recipes travel as-is.

## 6. Servers

- **Arcadia** already speaks FESL/Theater/PlayNow and runs an RB equivalent for real consoles.
  The port could reuse its accounts, matchmaking and lobby orchestration if the port's client
  spoke the Plasma handshake. The game channel would still be new (snapshots, not GameSync), so
  Arcadia would need a port-mode lobby, and consoles and the port cannot share a game.
- **A new server** for the port can follow the RB's message model ([messages.md](messages.md)): the
  same phase resets, barriers, timers and results, with snapshots in place of GameSyncs.

## 7. Not researched yet

- The RB's own timeouts and policies (EA's server code is lost; Arcadia's are reconstructions).
- Voice chat (`VoipConnection*`, GameManager VoIP packets).
- Host migration (present in the SDK, apparently unused with RB hosts).
- How an accepted free-skate activity proposal starts its challenge on the server.
- The exact entry lists of the host/client hub layouts.
- PS3 specifics (PSN login, ticket, platform id) beyond Arcadia's notes.
