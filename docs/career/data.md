# Career data: Vault classes, records and Lua chunks

## Vault layout

`data/db/db.big` holds three vaults:

| Vault | Classes | Collections | Notes |
| --- | --- | --- | --- |
| `skater.vlt/.bin` | 304 | 4,596 | all class definitions, including challenge and progression classes |
| `ChallengeCore.vlt/.bin` | 0 | 5,771 | challenge records; BIN embeds 627 Lua chunks |
| `progressioncore.vlt/.bin` | 0 | 6,260 | progression records; BIN embeds 910 Lua chunks |

Collections in ChallengeCore/progressioncore use the class definitions from `skater.vlt`.
A collection key is `Attrib::StringHash64(name)` (for example
`StringHash64("m_1a_01_goto_slappys") = 0xee4594d4bf57f08c`).

### Challenge classes

| Class | Fields | Role |
| --- | --- | --- |
| `challenges` | 279 | challenge definition: `GlobalData`, local data, `Objectives`, states, `ObjectiveFlow`, `StartLocation`, presentation, audio, … |
| `challenge_global_data` | 29 | type, icons, `Location`, `WorldLocation`, AI lockout |
| `challenge_local_data` | 44 | per-challenge local settings |
| `challenge_states` | — | state records (name, message handlers) |
| `challenge_messagehandlers` | 4 | `Attrib::Blob` (compiled Lua chunk), `Name`, a text, `UInt32` message hash |
| `challenge_objective` | 22 | objective definition, triggers, scoring boundary, `tCompiledLua` expressions |

### Progression classes

| Class | Key fields |
| --- | --- |
| `progression_statemachines` | `Name`, `States` (RefSpec[]), `InitialState`, `Starting`, `Description` |
| `progression_states` | `Name`, `StateMachine`, handler list (RefSpec[] into the handler class) |
| handler class `0x1b08ce405acdd040` | `StateMachine`, `Attrib::Blob` chunk, `Name`, `LuaScript`, message hash (`UInt32`) |
| `progression_messages` / `messagegroups` / `messagetypes` | named messages sent by `Progression.SendMessage` |
| `progression_challenges` / `groups` / `groupexits` / `lists` | challenge activation structure |
| `progression_pointpools`, `rankings`, `rankinggroups`, `pinchpoints` | economy and ranking |
| `progression_asset_group_sponsors` / `_unlocks`, `challengeeconomytags`, `photoarticles` | rewards |

Message types are identified by hash in the handler's `UInt32` field, e.g. StateEnter
`4040627377`, HUDYes `3836229679`, HUDNo `1323062978`, FMVFinishedPlaying `2897133262`,
NISChallengeNotify `2512425427`.

## Lua chunks

Header `1b 4c 75 61 51 00 01 04 04 04 04 00`: Lua 5.1, official format, **little-endian**,
4-byte int/size_t/Instruction, **4-byte float `lua_Number`**. Each handler chunk's main
function is `CLOSURE; SETGLOBAL h_<16 hex hash>; RETURN`; the closure is the handler.

## Research tools (local Python scripts)

`biglist.py` (BIGF archives), `vault.py` (VLT/BIN reader + `StringHash64`), `vault_dump.py`
(records), `luadump.py` (Lua 5.1 bytecode reader), `luacalls.py` (call tracer),
`progression_graph.py`, `challenge_graph.py`. Attribute names are recovered by hashing
candidate strings (PDB identifiers with MSVC `V`/`U` prefixes stripped, Lua constants, BIN
strings).
