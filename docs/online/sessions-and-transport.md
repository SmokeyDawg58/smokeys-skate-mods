# Sessions and transport

How a console reaches a game and what carries the bytes. Skate 2 uses EA's Plasma client SDK
(`Fesl::*`, linked into the executable) on top of DirtySock (`NetConn`, `ProtoTunnel`,
CommUDP, `NetGameLink`). Addresses are `sk82_na_m.xex` (Xbox 360); PS3 differences come from
Arcadia. Byte-level transport detail is in Arcadia's `docs/protocol/*.md`; this page is the
client's view and what was verified in the executable.

## 1. Three connections

| Connection | Transport | Used for |
| --- | --- | --- |
| FESL | TCP + SSL | login, PlayNow matchmaking, stats/ranking, records, friends (associations), presence, achievements, telemetry |
| Theater | TCP (+ UDP pings) | lobby/game lists, entering and leaving games, queues, host notices |
| game | UDP: ProtoTunnel → CommUDP → NetGameLink | the GameManager protocol and every `Sk8::Net::Message` |

## 2. Bring-up (`Sk8::Net::PlasmaSystem::OnInit` 0x82783040)

- `PlasmaSystemPlatform::InitDirtySock` 0x82781c90: `XNetStartup` (1 MB default receive buffer),
  `NetConnStartup`.
- `InitTitleParameters` 0x827867a0: `Fesl::TitleParameters::GetXb360(sku "0x4541087f",
  version "2.0", title id 0x4541087F, client string "skate2-360", FESL port 18820, environment)`
  plus the 46 offline achievement ids. The PS3 build is "skate2-ps3" on 18420 (Arcadia).
- Framework options 17; GameManager parameters: tunnelling not disabled
  (`SetDisableTunnelingOverride(false)`), unsafe game packet delivery on (packets reach the game
  from the network thread, hence `InboundMessageQueue`'s mutex), at most 8 game players. The
  game-network queue length is left at its default (32 per Arcadia).
- Services registered: presence, ranking, record, association, GameManager, telemetry; then
  `HubSingle::Init` and a connection listener.
- `Sk8::Net::GetProtocolVersion` 0x82a6ad18 = "Skate2-220" in this build (Arcadia saw
  "Skate2-211" on the retail PS3 build): PlayNow's game protocol version, so builds only match
  their own.

Sign-in (`ConnectionFSM` states, Xbox LIVE login) is in services.md.

## 3. Entering a game through Theater

The client never creates a game itself in Skate 2 ([README](README.md)); it joins one that PlayNow
or the game browser found, by lobby id + game id, user id (a friend) or invite
(`StateOnlineGame::JoinGame`, [game-flow.md](game-flow.md) §3). In the SDK
(`GameManagerImpl::JoinGame*` 0x82486e38…, `JoinGameInternalCommon` 0x82487640) that is
`TheaterService::EnterGameRequest`. Theater asks the game's host to admit the player (the host
side is `GameManagerHostedGameImpl::HandleEnterGameHostRequest` 0x82491360, which the RB ran);
on success Theater gives the joiner the host's address, the tunnel key (`EKEY`) and an entry
ticket. Game states on the way: `WAITING_FOR_ENTRY_RESPONSE`, `QUEUED`, `CONNECTING`, `ACTIVE`
(`Fesl::GameManagerGameState`). Join failures map to Skate errors in
`Sk8::Net::GameManagerListener::OnGameJoined` 0x827877b8: -612 game full, -605 rejected,
-603 not found, other = server error.

Other Theater operations the client has (`Fesl::TheaterServiceImpl`): `GetLobbyList`,
`GetGameList`, `GetGameData`, `LeaveGame`, `PlayerEntered/Exited`, `QueueUpdate`,
`DequeuePlayerForEntry`, `QueueKick`, `ReportAbuse`, `Ping`, `SignalHostLost`, `TransferHost`,
`RespondToHostTransferRequest/Acknowledge`. `CreateGame`, `UpdateGameData/Details`,
`ReservePlayers` and `RemoveGame` belong to the hosting side.

## 4. The host connection (`GameManagerGameImpl::MakeHostConnection` 0x8248ada0)

`GameManagerConnectionFactoryDirtySock::CreateConnection(name "lobby-game-player", host user
id, demangler type, port, EKEY, unsafe deliveries)` with port **6001** for a game
(6002 for a play group), then `Connect` with the host address and the Xbox LIVE session. The
connection object (`GameManagerConnectionDirtySock`) runs the demangler (XNet security on 360),
ProtoTunnel (keyed by `EKEY`; Arcadia's default "RELAYKEY" is not the retail key) and a
NetGameLink over CommUDP; its callbacks are established / lost / warning / receive.

**Sending** (`GameManagerConnectionDirtySock::SendData` 0x8249a0a0): at most 1199 bytes
(0x4AF); NetGameLink packet kind **6** reliable or **7** unreliable, with a "system" byte that
separates GameManager packets from game data.

**Receiving** (`OnReceive` 0x8249a258 → `GameManagerGameImpl::HandleIncomingPacket` 0x8248a698):
system packets are queued for the SDK's idle thread; game data goes straight to the listeners
(Skate's `InboundMessageQueue::OnReceive`), with a negative sender ref mapped to the host player.

## 5. Routing: everything goes through the host

`GameManagerPlayerImpl::SendToInternal` 0x82496748: with a direct peer link, send to the
player; otherwise wrap the data in **PLAYER_RELAY_REQUEST(target ref, data)** to the host, which
re-sends it as **PLAYER_RELAY_MESSAGE(source ref, data)**
(`GameManagerHostedGameImpl::OnRelayRequestReceived` 0x82494670,
`GameManagerGameImpl::OnRelayMessageReceived` 0x8248e100, 1200-byte limit). In a client-server
game only the host player gets the host link as its peer link (`OnHostHelloReceived`); other
players have one only if something else made it (a VoIP mesh connection,
`GameManagerPlayerImpl::MakeVoipConnection`). So, normally:

- traffic for the RB (GameSyncs, requests, reports) goes to the host link
  (`SendToHostInternal` 0x8248e050; a host cannot send to itself);
- traffic for one player (a kick request, `Net::KickPlayer`) is relayed;
- traffic for everyone (`QueueMessage` destination -1) becomes one relayed copy per player.

The RB forwards every player's GameSyncs to the others (nothing in the client does), and a
client-server host is also free to drop or rewrite what it relays (Arcadia rewrites a relayed
kick into its own host-kick handling).

## 6. The GameManager protocol

`Fesl::GameManagerPacketType`; on the wire the type byte is biased by 0x80 and numeric fields
are sign-biased (Arcadia `gamemanager-handshake.md`).

| # | Packet | Direction | Client handling |
| ---: | --- | --- | --- |
| 0 | HELLO | C→host | `SendHelloMessage` 0x8248b248: version 20 (0x8014 biased) and the Theater entry ticket |
| 1 | GOODBYE | C→host | `GOODBYE_CLIENT_NORMAL` 5 on a normal leave |
| 2 | HOST_HELLO | host→C | `OnHostHelloReceived` 0x8248b3a0, fields below |
| 3 | HOST_ROSTER_ELEM | host→C | one per player already in the game |
| 4 | ROSTER_ACK | C→host | after the roster |
| 5 | PLAYER_JOIN | host→C | a player is joining |
| 6 / 7 | PLAYER_JOIN_QUERY / _RESULT | | peer-to-peer join checks |
| 8 | PLAYER_JOIN_MESH_ESTABLISHED | host→C | the joiner becomes active |
| 9 | PLAYER_JOIN_COMPLETE | host→C | `OnJoinCompleteReceived` 0x8248baa0: for the local ref, the game becomes `ACTIVE` and `OnGameJoined` fires (Skate: `OE_GameJoined`) |
| 10 | PLAYER_LEFT | host→C | `OnPlayerLeftReceived` 0x8248cc88: ref + reason; the local ref destroys the game; another participant fires `OE_PlayerLeave`; the host player leaving starts host migration |
| 11 / 12 | PLAYER_CONNECTION_ESTABLISHED / _LOST | | peer link state |
| 13 / 14 | PLAYER_RELAY_REQUEST / _MESSAGE | | §5 |
| 15 | HOST_PROPERTY_CHANGE | host→C | game properties (join mode, ...) |
| 16–18 | START_GAME_REQUEST / _READY / START_GAME | | `OnStartGameReceived` 0x8248bd20: Xbox LIVE session start → `OE_GameStart` |
| 19 | END_GAME | host→C | `OnEndGameReceived` 0x8248be40: session end |
| 20 / 21 | VOIP_ENABLED_CHANGE / VOIP_RECEIVER_CHANGE | | voice chat |
| 22 | HOST_MIGRATION_FINISH | | |
| 23 | PLAYER_PLAYGROUP_CHANGE | | play groups (parties) |
| 24–28 | PLAYGROUP_* | | play-group game change, attributes, chat, presence, create proxy |

**HOST_HELLO** fields in order: version s16; game name (string, 128); s16; network type
(`GameManagerNetworkType`: CLIENT_SERVER 0, PEER_PEER_PARTIAL 1, PEER_PEER_REQUIRED 2); VoIP
type (DISABLED 0, MESH 1, CLIENT_SERVER 2); join mode (CLOSED 0, OPEN 1, WAIT 2, AUTO 3,
CUSTOM 4); join flags u16 (FRIENDS 1, CLUB 2, EXTENDED_FRIENDS 4, EXTENDED_CLUB 8,
OBSERVERS 32); in progress u8; ranked u8; join in progress u8; join via presence u8; invite
status; host migration enabled u8; for participants and observers: capacity u16 + VoIP
enabled u8; 8-byte Xbox LIVE session nonce (`Xb360Session::ModifySession`); roster size u16;
"has host" u8 followed by the host's player record when 1.

## 7. Losing the connection

- **Warning** (`OnConnectionWarning` 0x8248cfb8): with host migration enabled, tell Theater
  `SignalHostLost(lobby, game, host ref)`.
- **Lost** (`DoConnectionLost` 0x8248d178): apply any queued PLAYER_LEFT packets, then fake a
  local disconnect so the game is destroyed → Skate gets `OE_GameLeft` with
  `LGR_Lost_Connection` and shows `SER_GAME_DISCONNECTED` ([game-flow.md](game-flow.md) §10).
- **Host migration** (`SwitchToHostMigrationMode` 0x8248d6b8, Theater
  `HostTransferStart/FinishNotice`): drop the peer links and the host link, clear the roster and
  wait for Theater to name a new host. This is SDK machinery for peer-hosted titles; with an RB
  host that does not migrate (HOST_HELLO's migration byte), losing the RB ends the game.

Separately, the lockstep layer has its own liveness: a 1 s heartbeat when no GameSync was sent,
and the "waiting for players" chyron after 1 s without a simulated frame
([lockstep.md](lockstep.md) §8).

## 8. Below the GameManager (summary)

From Arcadia's protocol docs, consistent with the client:

- **ProtoTunnel**: a 2-byte wire counter driving an RC4 stream, tuple headers
  `(length << 4) | tunnel index`, a 6-byte preamble on tunnel 7, CommUDP on tunnel 1. Skate 2
  (patched) encrypts only the preamble window; Skate 1 encrypts everything.
- **CommUDP**: CONNECT/CONNACK/DISCONNECT/NACK/POKE; 24-bit sequence numbers with cumulative
  acks in every packet; up to 16 sub-packets bundled per datagram; ~100 ms busy and 2.5 s idle
  retransmit/keep-alive; strictly in-order delivery to the layer above.
- **NetGameLink**: system flag, body, 10-byte sync trailer (tick echo, jitter), kind byte; the
  32-slot receive ring.

Strict in-order, exactly-once delivery is what the lockstep needs: a GameSync has no frame
number ([lockstep.md](lockstep.md) §5).

The port replaces all three with its own transport, every packet sealed
([implementation.md](implementation.md#builds-and-encryption)).
