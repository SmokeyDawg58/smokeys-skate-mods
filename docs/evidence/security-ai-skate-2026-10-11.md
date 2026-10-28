# Security and career AI S.K.A.T.E. continuation — 2026-10-11

Original Jan17 rich-symbol IDA is read-only. Work and
copied profiles are in `.scratch/security-ai-skate-20261011`. The final release
build and the scoped gameplay checks below passed. This is not a claim of
complete instruction-level parity.

## Career S.K.A.T.E.

The original `t_1b_01_skate_alphabet_shingo` handlers already implement the
leader/follower state machine. The previous run actually logged
`Random.GetRandomNumber: the Simulation RNG is not lent`, followed by
`wait_for_leader/stateenter: attempt to index a nil value`. It therefore did
not verify a working human turn. Its successful camera switches only verified
the earlier SessionMarker crash repair.

`try_update_career` returned the Simulation random stream before
`system.flush()`. Consequently AirCompleted and state-enter handlers could
refresh the ABD display, then abort when choosing an AI commenter, leaving
Lockout set before ChangeState/its timer was reached. The same mistake affected
AI path choices. The stream now remains lent through the frontend, challenge
queue, highlight and HUD updates; paused challenge queue delivery also borrows
and returns the same stream. Native `Simulation::PostUpdate` 0x8289fd88 flushes
queue7; `Random.GetRandomNumber` 0x8288adb8 unlocks the Simulation stream,
draws [1,n+1), converts to int, and locks it again. No fallback random source
or script edits were introduced.

Human challenge teleports were conditional on `player == GetLocalIndex`, so
the return from spectating Shingo could discard the human's spawn teleport.
Native `cTeleportManager::TeleportPlayer` 0x828a9888 checks for an AIController:
AI receives Actor::SetTransform; the offline human receives cMsgTeleport even
while the camera is on AI. Targeted teleports now share the ordered skater
request list with pins and camera changes.

Related identity audit:

| Reference | Native identity / correction |
| --- | --- |
| Simulation.GetLocalPlayerID 0x8289e458 | Zero outside a live network game. |
| Online.GetLocalPlayerID 0x8288a570 | SkaterManager camera index offline; kept. |
| TeleportPlayerNow 0x82da6a00 | Marker repositioning compares the network-query ID, not camera index. |
| TeleportLocalPlayerToLocationName 0x82db0640 | Network-query human ID. |
| MapBackEndManager.TeleportToChallenge 0x8256dc10 | Destination spawn index uses Simulation.GetLocalPlayerID. |
| cChallengeTeleportManager.OnChallengeSelect 0x82dadb20 | The +8 ISignupSubController receiver's +0x14 is mNetworkQuery (+0x1c), slot4; signup teleport uses the offline human ID. |
| StartHub.QuitPopupCallback 0x82671678 | Quitting orientation uses that same human ID. |
| TeleportLocalPlayer 0x828a92a8 / 0x828a9f40 | Challenge-end spawn index uses that same human ID. |

Original scripts retain turn timers, valid Air/Flip checks, spot-volume entry,
wipeout/physics-teleport checks, ABD repeat detection, spin count matching,
leader/follower messages, letters, gestures, highlights and cleanup. Native
StartSkatePath 0x8288b340, MatchedPattern 0x8288b010 and FollowTrickPattern
0x8288d508 were rechecked against the current implementations.

Repeated AI ABD attempts were also audited through StartAerialTrick
0x824d1690, RandomAerialTrickSwap 0x824c68f8, IsSafeToSwapAerialTrick
0x824c6708, GetSuitableTrick 0x824be428 and WeightGroup.GetRandomTrick
0x824ba5c0. The current path/profile selection retains the original CRT
draw, float cumulative weights, strict comparison and first-entry fallback.
No demonstrated mismatch was found in those routines; the diagnostics do
not change AI weights or fabricate trick/scoring events to shorten a match.

### Shipped results-button quirk (retained)

The complete-match check exposed a quirk in original handler
`8fd38f5b25c063d5` (`show_results/highlightreelcomplete`). Raw Lua bytecode,
checked independently of the readable pseudocode, writes
`vars.WinnerIndex = vars.SkatersList[j].id` inside the results-row loop:
PC65..68 end in `SETTABLE 2, K21("WinnerIndex"), r7`, word`0x8a81c089`.
PC82..86 then index `SkatersList[WinnerIndex].npc` and compare to false.
With the two original players0/1, the final row leaves WinnerIndex1, which
selects the human table entry. PC90 is `LOADBOOL r4,1,0`, followed by
OpenContinueRetryDialog(true,false,true) atPC93, even after the earlier
show_results handler correctly announces a loss and keeps ChallengeResult=Lose.

Native OpenContinueRetryDialog0x82896a58 directly stores its first boolean;
LeaderBoard.GetOption0x82657b40 resolves Continue_Retry/Retry_Quit using it.
Thus this original script supplies A/Continue and B/Retry after the observed
loss. B opens the rematch confirmation and restarts; A's `hudcontinue`
handler still calls LoseChallenge using the retained ChallengeResult. The
original handler bytes and native button selection were left intact.

## Capture fines: original evidence

This reference build does **not** deduct a monetary fine on guard capture.
The misleadingly retained `Statistics.GetJackedPenalty` binding at 0x8260b738
points to folded function 0x82604b28. Raw code is `li r3,0` followed by a branch
to AptValueFactory::CreateInteger. The symbol IDA names at that shared address
is MissionCmdTranslator.sMethod_GetSignupMissionType; the binding establishes
its use as GetJackedPenalty. The original `jacked_overlay.apt` has no penalty
read or money-changing call, and only adds A/Continue.

Full native capture/update and chase-end audit:
JackSkater 0x8270fd08, ChaseMan.Update 0x8270f678,
UnRegisterChaser 0x827186e8, ShowHUDChaseEndMessage 0x82713cc8,
SetCappingOnJacked 0x8289ca48, FE42.OnEnter 0x82642450 and
FE42.ReceiveInput 0x826598f0. These preserve the previous capture timer,
manual-respawn suppression, guard-owned nearest respawn, free-roam career
overlay, Photo/Pro StartLocation exception, recapping and chase suppression.

All xrefs to SignPost.SubtractCash 0x825dd7f8 lead to entry fees, property/shop
purchases, uncapping, draining or the paid security service. SetCash and
AddCash callers were checked as well. The original challenge/progression
handler corpus and English strings were scanned case-insensitively for cash,
chase, jacked, fine, caught, busted and penalty. The `fine` bytecode matches
are `AudioDefineChallengeWinLowTime`/other words, not capture fines. Scratch
`data-audit.txt` and `ida-audit.txt` retain the findings. Charging invented
cash here would contradict the requested reference.

## Missing guard rewards restored

- **Running Man:** UnRegisterChaser 0x82718a00..7c awards achievement34 with
  delay0 only after an uncaptured chase finishes, in career, while the local
  actor is still filtered state6 (offboard), and accumulated offboard chase
  time is strictly greater than kRunningManAchievementTime. The cvar is now
  registered/read live. Initializer 0x82f64c68 has default0, min0, max90;
  90 is not its default. Shipped loose startup INIs do not override it.
- **Good Samaritan:** PedestrianColliding.Begin 0x82716b58..6c60 already had
  its career/player-collision/guard-or-security_street01/nonlocal Actor chasee
  gates implemented. Its achievement callback returned a fatal placeholder
  error. It now grants achievement36 with delay0 through the existing shared
  achievement owner. Raw `li r3,0x24` at 0x82716c5c confirms the ID.
  The Lua-bound achievement enum labels 36 as TasteTheMongo and 37 as
  GoodSamaritan; the collision hook nevertheless explicitly passes 36 in
  this executable. The raw native call is retained, without renumbering it
  to match that separate enum's label.

The separately unported phone/Big Black bodyguard-service producer is not
represented as implemented by this guard capture/chase audit. The platform
achievement transport remains the port's existing local backend adapter.

## Guard audio, chase music and tazer presentation

The wider audit found three missing consumers: pedestrian speech hardcoded
chase flags to zero; MusicSystem recognized CHASE but never checked or updated
it; VisualFX had no tazer branch. They now consume the native state:

- ChaseMan.GetAudioState **0x8270fe90** uses the camera-local Actor's chasee
  count/end reason, Actor.GetPosition, WorldPainter layer6, chasers and warners.
  Raw PPC **0x8270ffb8/0x8270ffc8** stops the chaser scan once **either** guard
  kind has been found. The decompiler's misplaced `continue` statements must
  not be read as loop control; raw branches resolve to the flag stores.
  DoSimSubmit **0x827fb2f8** packs the count into two bits (`count & 3`).
- The replay frame now retains that count/flag byte and VD tazer bit.
  AudioSystem.UpdatePresentationData **0x82504848** latches chase audio after
  the state managers. Speech **0x82503848** receives the cached flags for
  pursuit/commentary and warning choice. Leaving an NSZ requests original
  cameraman event `T_762_LvNoSk8Zn`, with `lw_type=2` (raw **0x825049b0**).
- MusicSystem.CheckChaseState **0x824f7088**, ChaseTrigger **0x824e99d8**,
  StartChaseMusic **0x82502a40**, StopChaseMusic **0x824e9ad8** and
  UpdateChaseState **0x82506cd8** now run through the existing Pathfinder
  stream owner. They retain the environment-mode, replay, FE, cached-data,
  environment/default-mode cooldown and Big Black gates; distinct street
  and NSZ captures, warnings, pursuit, escape, authored signed-byte intensity,
  14-second ending gate and ten-restored-frame suppression. Original game.mpf
  events are used. Native UpdateChaseState does not write back its selected
  intensity; that behavior is retained.
- The MusicSystem PDB incorrectly gives `long double` a 16-byte extent. Raw
  PPC reads eight-byte doubles; the fields after them are 0x10 earlier than
  the PDB names suggest. CheckChaseState's +0x60c/+0x610 are the environment
  and default-mode counters, not scene flags; +0x5d4/+0x5d8/+0x5dc are last
  restored frame, no-chase counter and street-guard latch.
- VDWriter._PrepareTazerData **0x827fc768** requires TazeIsOn AND the local
  scoring output's HoM_BailInProgress. VisualFX_Animation **0x82847ac0** uses
  inverted RGB matrices for both grades before HoM, challenge result and fade
  branches. This is wired into the existing post-fx renderer, preserving
  later ICE overrides and the recorded value during replay.

## Implementation map and confidence

| Area | Production owner |
| --- | --- |
| Simulation RNG loan and ordered actor requests | `session/selected_session_career.rs` |
| Offline human versus camera identity | `challenge/players.rs`, `challenge/natives.rs`, `challenge/teleport.rs`, `challenge/signup.rs`, `session/selected_session_career_start_hub.rs` |
| Running Man and Good Samaritan | `living_world/pedestrians/pedestrian_graph_frame_chase.rs`, `pedestrian_graph_frame_colliding.rs`, `input_cvar_runtime.rs` |
| Recorded security audio/tazer data | `session/selected_session_replay.rs` and its public facade |
| Audio presentation, speech, chase music | `skate2-audio/src/system.rs`, `speech/mod.rs`, `music/mod.rs`; app `game_audio.rs`/`live_skating_client.rs` |
| Tazer post-fx | app `render_postfx.rs`, `render_world_painter.rs`, `live_skating_view.rs`, `gpu_renderer/postfx.rs` |

Confidence is **high** for the reconstructed control flow and original-asset
integration above. The zero penalty and listed PPC constants/flag branches
are **exact** observations of this reference. Host simulation numerics,
pixel-level native rendering equivalence, and every mission combination are
not established by source review or these scoped runs. The diagnostic S.K.A.T.E.
pad/steer route is opt-in and changes no production rules or challenge data.

## Verification

Release build4 passed: `cargo build --release -p skate2-pc -j 1 --timings`,
12m39s, exit0. It includes the RNG/identity/teleport fixes, both achievement
hooks, guard audio, chase music and tazer rendering. Earlier build3 passed
before the audio/FX additions; shingo1/2/3 all exited0 and exercised repeated
AI leader turns, copy attempts, failure/letter handling and return teleports.
Final build5 passed the same release command in **9m04s**, exit0. It also
includes OnChallengeSelect's network-human ID correction and narrowed,
opt-in music logging under `SKATE2_TRACE_SECURITY_CAPTURE`. No compiler
errors; the application reports 18 warnings from the existing tree.
`shingo7` exited0 after 10,453 controlled ticks. It uses only controller input
for attempts; original physics, spot volumes, scoring and scripts decide the
result. Recorded transitions include:

- Shingo sets a trick; a different human trick produces Not Copied and a letter.
- At frontend tick4531 the human's valid Ollie reaches `leader_success` and
  `ID_SKATE_TRICK_SET` with `PLAYER 1`. At4652 `prepare_follower` queues human0
  to the waiting locator, AI1 to the spawn locator, then camera1.
- At4771 AI1 receives `FollowTrickPattern` for exacttrick106, with the matching
  authored path `[5,1,98,17,1,119,0,0,0,0,0,0,0,0,0,0]`. The AI attempt bails,
  reaches `follower_not_copy` at5360, then the letter/process-round timers.
- At5813 the next round queues AI1 to its waiting locator, human0 to the start,
  then camera0. At5932 human0 is playing at board position
  `[1441.2291,85.08646,-237.24602]`, corresponding to the authored spawn transform
  `[1441.235718,85.307449,-237.242889]` after physics placement. The spectator
  locator is elsewhere; the previous discarded-human-teleport bug is gone.
- Repeated Ollies correctly reach `ID_SKATE_TRICK_ABD` at6333,7581 and8973;
  Shingo's repeated trick also reaches ABD at6985. Further rounds continue,
  including successful AI-set tricks at9622 and11260. No RNG/nil/ForeignActor
  failure appears.

Screenshots `shingo7_018.png` (human Trick set), `shingo7_019.png` and
`shingo7_020.png` (Shingo Copy the trick) were visually inspected. Original
script end/retry/cleanup handlers were audited; this run ends during play,
so it does not establish a complete match win/loss or retry runtime result.

Earlier diagnostic acceptance during the still-running signup teleport fade
was too early for a useful initial-camera check. The native
OnEffectFinished0x82da6cd8 applies pending camera changes in FadeOut, not
FadeIn. That quirk was retained. `shingo7` accepts after the authored signup
transition and verifies the correct initial AI camera.

`security1` was closed normally, exit0, before capture because full audio
logging made the run very slow. It reached original NSZ/warning audio flags
0x80/0xa0.

Final-executable `security2` exited0 after **5,147 controlled ticks**. Natural
Mongocorp warning/pursuit reached counts1/2/3 and flags0xf0, capture0xf4/0xd6,
then0x82,0x80 and0x00 during recovery. Capture retained the original 4.5-second
timer and guard owner's authored respawn at `[-70.08412,-33.187897,668.60516]`.
`security2_007.png`/`_008.png` show JACKED and A/Continue; `_010.png` shows
offboard respawn, and `_021.png` shows remount after Y. All were inspected.

This copied profile has MusicPlayer version2's fourth tuple set to2 (Music
Player mode), so the native environment-mode gate correctly suppresses chase
music. `world_music_profile.py` creates a separate scratch copy with that
existing setting set to1 (World Music), preserving every other byte except
the outer native CRC. LoadMusicPlayer0x825645a8 was rechecked in IDA before
creating it. `security3` exercises the World Music branch; it and the final
match-end run use the final executable. `security3` exited0 after **5,410
controlled ticks** and logged the original music transitions `8 -> 0`
(warning), `0 -> 2` (pursuit, active node831), `2 -> 3` (capture, node894).
It also exercised capture cancellation: filtered state5 ended the first bail
with1.5833292 seconds left on the timer, correctly cancelling the pending
JACKED flow. A subsequent capture reached JACKED and the authored respawn.
The four-chaser case produces packed count0 with chase flags still set,
matching the native two-bit storage rather than saturating the count.

Tazer grading, both new achievement branches, Photo/Pro capture exceptions,
and replay/Big Black music gates have source coverage; these diagnostics do
not establish runtime coverage for them.

Final-executable `shingo8` exited0 after **24,522 controlled ticks**. The human
sets an Ollie at2889; Shingo successfully copies it (`follower_copy`) at3586.
Later incorrect human copies accumulate all five letters. At16796 the
original script announces Challenge Failed, starts the recorded highlight
at16977, and opens the results dialog at17283 (ready17289). B at17349 opens
the native rematch confirmation; Yes at17410 reaches `restarted`, resets ABD,
repositions both actors and resumes AI turns at17500. More human/AI rounds
continue before the diagnostic ends. The retained script quirk above explains
why B is Retry on this loss. `shingo9` exited0 after 25,070 controlled ticks
but ended with four human letters and repeated ABD attempts. It did not
reach the Continue/cleanup branch, so it is not counted as coverage of that
branch.

Final-executable `shingo10` uses a delayed pop on the controller route,
letting the original valid-area rules reject the human attempts. The fifth
failed copy reaches match loss at20747, the highlight starts at20928,
and results are ready at21240. A/Continue at21300 invokes the original loss
handler, enters `complete`, sets camera0 and unpins the human. At21311 the
main challenge is absent; AI slot1 is unspawned, ExitAiBasedChallenge runs,
ambient AI resumes, and the isolated profile autosaves. The run continued
in free roam beyond 22,500 controlled ticks before a normal window close,
exit0. `shingo10_099.png` was visually inspected: the challenge/ABD HUD is
gone and the human is back in the world. No RNG/nil/ForeignActor/script or
controlled-tick failure was found in these final S.K.A.T.E. runs. A full
match win has source coverage but was not exercised by these routes.

No unit tests added or run.
Real profile baseline SHA256:
`0272B44B1AFE815168A019642364DC320719D92A726CA2DC11D3F69B1111ACF0`.
The final hash matches that baseline. No game or compiler remains running.
