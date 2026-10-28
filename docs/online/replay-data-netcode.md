# Replay data as the online netcode (feasibility, 2026-10-15)

The question: instead of the original lockstep ([lockstep.md](lockstep.md)), could the port
send each skater's **replay data** — what the replay recorder already captures every tick:
pose, position, animation, audio — and add the few extra events the game modes need (a landed
trick and its score, a bail, a spot entered)?

Research only; no network code exists. Native addresses are the debug `sk82_na_m.xex`.

## Short answer

Yes, and it fits the port unusually well. The presentation layer that plays replays is already
a "drive a skater from received frames" engine: it blends between submitted frames, restores
the pose, the root, the opacity and the cloth, and the audio host plays a skater's sounds from
the recorded physics output. A remote player can be a replay-driven puppet. The parts replay
data does **not** carry are physics presence (bumping into another player) and the challenge
facts the templates arbitrate (tricks, scores, bails, volumes); those become messages. The cost
is bandwidth — about 1.4 KB per skater per tick before compression — which is manageable at
30 Hz with the native quantisation and ordinary compression.

## 1. What the replay recorder captures

### Native

`DataSysMan::DataSysMan` (0x8277f600) builds the `DataSystem::DataMan` with
`numproviders` 16, `numframes` 5400 (90 s at 60 Hz), `desiredsize` 12,582,912 bytes,
`compressionallowed` true, 10 cache frames in 100 KB, and a 15-frame visualisation buffer.
Every simulation tick each presentation's `DoSimSubmit` writes its data and
`DataMan::GatherAllData` (0x82997bf8) collects the frame; a replay or a restore reads them back
(`DataMan::RestoreData` 0x82996580). 90 s of every provider in 12 MB is about **2.3 KB per
frame after compression**, for the whole world.

The skater provider is `SkaterPresentation::WriteData` (0x827dbf98) → `WriteSkater`
(0x827d8ab8):

- a 208-byte `SkaterPresentation::Data` header: the root, the `peerId` (bytes 160..168) and
  player nibble (word 0xbc) that pair a skater across frames, the teleport bit (word 0xc0,
  0x1000) that stops a blend across a teleport, the render opacity (byte 0xbe), and the rest of
  the presentation state;
- the pose, quantised: the body parts `Torso`, `Head`, `Arms`, `Legs`, `Skateboard` and
  `Skateboard_Trucks` as 4-lane quaternions from the local matrices (8 bytes each), the hands
  (`QuatCompressQuantize3`) and the three face parts (`QuatTransCompressQ3`) as 3-lane
  quaternions (6 bytes), and the truck and face translations (`TransCompress`, 6 bytes).
  `SkaterPresentation::InitOffsets` (0x827dd548) sizes it (`mPoseBufferSize`).

Measured with the shipped hierarchies (`SKATE2_TRACE_PRESENTATION_SIZE=1`, the port's
`PresentationOffsets`): 31 matrix quaternions, 65 quantised quaternions, 37 translations —
**860 bytes of pose, 1,068 bytes with the header**, per skater per tick.

Beside it, `AudioPresentation::DoSimSubmit` records each skater's audio inputs
(`SkaterAudioPhysOut`: centre of mass and its velocity, foot and torso velocities, absorption,
contacts, surface, grind and wipeout state — about 60 fields), the camera, the reverb/ambience
keys, and the living world (pedestrians, vehicles, movable objects). `VDWriter` adds the bail and
tazer data (`_PrepareBailData` 0x827fcd40).

### In the port

`selected_session_replay.rs` keeps, per tick, a `ReplayFrame`: the `PresentationSubmission`
(camera, `SkaterFrame`s with the header, the decoded pose and the twist ratios, the living
world), the `AudioFrame` (one `AudioSkater` per skater: player index, local flag,
`SkaterAudioPhysOut`, `can_see_player`, an AI's `AIOut_Audio` words) and the `RecordedWorld`.
`PresentationRestorer` (`rendering/presentation_restore.rs`) is the restore side: it keeps the
three newest submissions, picks the moment to show from a clock, and per skater blends the root
(`UpdateSkater` 0x827d9e10), the pose (`UpdateBody` 0x827de868, `AnimCommandSystem::Blend`), moves
the root and computes the expressions (`UpdateBodies` 0x827de988), then the entity matrix,
bounds and opacity (`DoSimRestore` 0x827e6438). The career's replays, the replay editor and the
highlight reels all run through it.

## 2. Why it fits

- **The restore is already the interpolator.** A network puppet needs exactly what
  `PresentationRestorer` does for a replay: hold a few received frames, show a moment between
  two of them, never blend across a teleport. Its clock already works from per-frame due times;
  a network clock is the same with a jitter delay.
- **Audio comes with it.** The audio host plays the replay's skaters from the recorded
  `AudioSkater`s — wheels, grinds, landings, bails — with no simulation. A remote skater sounds
  right from the same record, attenuated from its centre of mass.
- **No determinism needed.** Each machine simulates only its own skater. Mods, float
  differences and frame-rate differences no longer desync anyone ([lockstep.md](lockstep.md)
  §9's kick disappears).
- **Spectating works.** `Online.SwitchCamera` (0x8288a4e0) is `SkaterManager::SetLocalIndex`;
  the presentation's local flag and everything that follows it (the dynamic shadow, the HUD
  binding) already work on non-simulated skaters, as Party Play and the S.K.A.T.E. AI show.

## 3. Bandwidth

| Stream | Bytes per tick | At 60 Hz | At 30 Hz |
| --- | ---: | ---: | ---: |
| Header + pose (native packing) | 1,068 | 64 KB/s | 32 KB/s |
| `SkaterAudioPhysOut` (≈ 60 floats, packed) | ≈ 300 | 18 KB/s | 9 KB/s |
| **Total, one skater** | ≈ 1,370 | 82 KB/s | 41 KB/s |

Reductions, in order of value:

1. **30 Hz with interpolation.** The restore blends between frames anyway; replays already show
   sub-tick moments.
2. **Compression.** The native buffer is compressed (`kDataManCompressionAllowed_true`); pose
   streams compress well (most bones change slowly). Delta against the previous sent frame plus
   a general-purpose compressor should give 3-4x: **about 10-15 KB/s per remote skater**.
3. **Detail by distance.** The hands and face (the 65 three-lane quaternions and every
   translation but the trucks') matter only close up; the body alone is the 31 matrix
   quaternions plus the truck translations, under 300 bytes.

An 8-player lobby is then roughly 70-100 KB/s down per client and the same up per client to the
relay — ordinary for broadband.

## 4. What replay data does not carry

| Need | Lockstep had it because | With replay data |
| --- | --- | --- |
| Bumping into another skater | every console simulated every body | the puppet has no body; give it a kinematic proxy (capsule from the header root and bounds, velocity from consecutive frames) on the receiving side, and accept that contact is decided by the owner of the moving skater |
| Tricks, scores, bails for the challenge logic | every console ran every player's scoring | the owner's scoring produces the challenge messages and sends them; receivers post them for that `PlayerID` |
| Trigger and spot volumes | deterministic positions | the owner reports enter/exit of its own skater |
| Spawns, teleports, pins between rounds | `Sk8::Command` types 3-8 | server messages; the owner applies them to its skater, the result arrives as frames |
| Timers and race starts | shared frame counter | a server clock ([remake-notes.md](remake-notes.md) §4) |
| Pedestrians, traffic, movable objects | same seed | local cosmetic only |

### The challenge messages to forward

The challenge templates already consume per-player messages; in the port they are the
`cMsg*` events the physics and scoring post with a `PlayerID`. The owner sends the ones its own
skater produces:

- tricks and lines: `cMsgTrickComplete`, `cMsgSequenceBegin`, `cMsgSequenceComplete`
  (`SequenceScore`, trick counts), `cMsgGrindCompleted` (length, spline), `cMsgManualCompleted`,
  `cMsgEnterManualTime`, `cMsgLineTimerStarted` / `cMsgLineTimerExpired`;
- bails: `cMsgPlayerWipeout`, `cMsgWipeoutOver`, `cMsgWipeoutAlmostOver`,
  `cMsgWipeoutBodyTweakPerformed` (Hall of Meat damage);
- places: trigger volume enter/exit, `cMsgPlayerRadiusEnter` / `cMsgPlayerRadiusExit`,
  `cMsgLeftObjectiveScoringBoundary`.

Per mode ([online-challenges.md](online-challenges.md) lists them):

| Mode | Extra facts beyond the frames |
| --- | --- |
| S.K.A.T.E. | the setter's landed trick (scorable id and name) and whether the follower's landed trick matches; letters decided by the server or the leader |
| Own the Spot / Spot Battle, Best Trick | each trick's score in the spot volume |
| Jam | per-trick scores (first to the target) |
| Death Race | checkpoints and the finish, timed by the server |
| Hall of Meat | bail scores and body tweaks |
| 1-Up | the trick to beat and each player's attempt |
| Online Free Skate | activity proposals and votes (already messages in the original) |

Voting reads every player's pad in the original (`ChallengeVotingManager`); it becomes vote
messages.

## 5. Plugging a remote skater into the port

1. **Recipe first.** The original exchanges appearance before a game (`StateOnlineRecipe`,
   [recipes.md](recipes.md)); the port builds the palette and materials from the recipe as it
   does for AI and Party Play skaters, and the rig (`SkaterRig`) from the shared hierarchies.
2. **A remote track.** Received frames (header, packed pose, audio record, tick) go into a
   per-remote ring. Each frame the puppet's `SkaterRestore` blends the two frames around
   `now - delay` (delay ≈ 100 ms, adapted to jitter); a missing frame extrapolates the root only.
3. **Presentation and audio.** The restored skater joins the visual snapshot like a Party Play
   skater; its `AudioSkater` (marked non-local) joins the audio frame.
4. **Challenge players.** Remote players are challenge players with their own `PlayerID`; their
   forwarded messages enter the challenge system as if their skaters had produced them locally.
5. **Packing.** The port's `WriteData` path already produces the native quantised bytes, so the
   wire format can be the replay format itself, and a recorded replay of a network game is just
   the received frames.

## 6. Risks and open questions

- **Header fields.** Every word of the 208-byte header must be checked for process-local values
  (pointers, local indices) before it crosses the network; pairing uses `peerId` and the player
  nibble, which must be assigned by the session.
- **Skater contact.** Without a shared simulation, bumping is approximate and owner-decided.
- **Trust.** Scores are owner-reported; a server can bound them (score per second, trick
  catalogue) but not prove them.
- **Lobby size.** Jam and Free Skate lobbies of 8 multiply the stream by 7 per client; the relay
  pays N × N.
- **Native ghosts.** The original has a ghost path (`GhostSpawnData`, `NT_Ghost`) that may
  already replay a skater from recorded data; it is worth tracing before writing a new puppet.

## 7. A first prototype without a network

Record the local skater's frames, feed them 100 ms later into a second skater offset by two
metres, through the same restore. That proves the puppet, the audio and the clock on one
machine; the transport (sequence-numbered UDP at 30 Hz) and the event messages come after.
