# Completed challenge map Retry — 2026-10-11

## Request and status

Completed challenges appeared in the map, but selecting Retry did nothing.
Implemented the connected map action, teleport/load/start sequence and native
replay guards. Also restored the active Own the Spot map action, which used the
same missing start-during-load path.

**Confidence: high (source reconstruction).** Live Jan17 rich-symbol IDA,
including raw PPC where Hex-Rays loses the 64-bit argument convention, was
the reference. No compilation, tests or game launches this turn.
Runtime behavior is not yet verified.

## Reference path

| Native owner / address | Recovered behavior |
| --- | --- |
| `FrontEndState_ChallengeMap::DoTeleport` `0x8265f280` | Calls the backend with `!mStuffMenu.mPopulateWithActiveChallenge`, then pops state 7. The completed list requests replay; the active list does not. |
| `MapBackEndManager::TeleportToChallenge` `0x8256dc10` | Requires a challenge and its `Location` attribute before setting flags. Retry sets `isReplayingChallenge`, calls `OnChallengeRetry`, then sets `startChallengeDuringLoad`. Type 9 (Own the Spot) also starts during loading, without setting replay for an active challenge. |
| Same function, `0x8256de1c` | Uses `Simulation::GetLocalPlayerID` to resolve the authored spawn. This is offline player 0 even if the game camera is following an AI. |
| Same function, `0x8256de68` / `0x8256deb4` / `0x8256df00` | Queries loading, teleports to the authored world/location offboard with the loading option, then sends ClearSessionMarker. |
| `cSignPostManager::OnChallengeRetry` `0x825d7f90` | Calls `CheckStateTriggers` with result 7. |
| `DoTeleport` `0x827767e8` | The start-during-load flag requires the loading path. |
| `StateLoading::TeleportLoadState` `0x8275fdb8` | Waits for the stream focuses and loading screen, sends the teleport, and releases its teleport pause on exit. |
| `PostLoading` `0x82765d08`, `CharacterPrepare` `0x82760058`, `PrepareExitToWorld` `0x827589e8` | Lead through world/character preparation to StartChallenge. Existing host streaming/teleport/presentation acknowledgments remain the owners of these resources. |
| `StateLoading::StartChallengeState` `0x8274f800` | For a career challenge with the start flag: select AI with `false`, check AI assets, request challenge assets, update their loader, then check hull readiness. Four ready updates are required; an intervening not-ready update does not reset the counter. |
| Same function, raw `0x8274f950..0x8274f964` | `LoadChallengeAssets(key, true, false, 0)`. Hex-Rays prints incorrect arguments here; raw r4/r5/r6/r7 establish these values. |
| Same function, `0x8274f9d0..0x8274fa14` | Run DMO operations with `true`, clear the request key/start flag, start the main challenge, then enter WorldPrepare. |
| `StateLoading::WorldPrepareState` `0x8274f668` | Wait until the render tick is strictly greater than entry tick + 5; finish loading and call `LoadingReset`. |
| `cActivityManager::LoadingReset` `0x82cd8728` | Calls the registered sub-controller loading resets. Reuses the existing port entrypoint. |

The `Location` key is `0xf1c6d5fa4a23951e`; `ChallengeType` is
`0x02b828ccdb4785e4`. Raw `li -1; std` establishes the cleared 64-bit request
sentinel; the port represents absence by its loading-state enum.

### Replay lifecycle and progression

| Native owner / address | Replay rule |
| --- | --- |
| `cSignPostManager::RunMainActivity` `0x825d7e08` | Skip result-3 progression triggers during replay. The map already sent result 7. |
| `WinChallenge` `0x825e0ea0` | Skip recording a career win. |
| `LoseChallenge` `0x825e0f90`, `AbandonChallenge` `0x825e1028` | Existing guards retained. |
| `ProcessChallengeResults` `0x825e10c0` | Return before changing pending career results, triggers, endpoint teleport or save behavior. |
| `CompleteChallenge` `0x825e1550` | The name-based port entrypoint now reaches the same guarded key-based operation. |
| `AwardPrizeCash` `0x825de2d0` | Skip contest rank/prize progression during replay. |
| `cChallengeBEManager::CompleteChallenge` `0x825b7920` | Clear replay at `0x825b794c` for every activity completion; this is outside the main-only branch. |
| `cChallengeBEManager::GameUnload` `0x825b7858` | Clear replay at `0x825b7894`. |

The existing `Challenge.IsReplayingChallenge` Lua binding reads this same
world flag. Existing photo-selection replay handling is retained, including
its guard against replacing the career's saved photo.

## Production integration

- `challenge/stuff_menu.rs`: distinct `RetryChallenge` action for the completed
  list; active challenges, including Own the Spot, retain `TeleportChallenge`.
- `challenge/system.rs`: map preparation and loading steps reuse AI, assets,
  DMO operations and `activity::start_main_challenge`; replay completion and
  game-unload cleanup are connected to their existing owners.
- `challenge/signpost.rs`, `signpost_progress.rs`: missing replay guards.
- `session/selected_session_career.rs`: explicit Teleport → StartChallenge →
  WorldPrepare states. Resolve Location before changing replay state. Preserve
  Simulation RNG ownership through AI selection. Hold progression-machine
  dispatch while loading. Existing frame pumps deliver hull and AI requests.
- `session/selected_session_public_facade.rs`: start the map travel, retain
  the fade cover after teleport completion, block frontend/D-pad/PlayerUI
  focus while preparing, then release the cover after WorldPrepare.
- `session/selected_session_edit_skater.rs`: count actual presented frames,
  independent of simulation/NIS pause. No timer approximation.

The loading request is consumed exactly once. A missing challenge/Location
does not put the session into replay. Normal active-challenge travel still
teleports without forcing a challenge start. Replay completion uses the
existing activity reset, result screen, globals-resume and unload paths.

### Adjacent source checks and limits

`NISManager::FEGo` `0x828a2fe8`, scene receive `0x828633f0` and load-state
update `0x82875bc0` were checked. FEGo's wait latch belongs to scenes that
requested their own loading screen. The existing host NIS loader remains
responsible for that flow; no unconditional synthetic NIS event was added.

`GetChallengeComponentOption` `0x8264d938` changes option labels while chased.
`OnMenuSelect` `0x826828f0` does not establish an equivalent generic input
rejection. No behavioral gate was inferred solely from a disabled label.
Existing Big Black service and additional-world/training-facility limitations
are outside this change; this does not implement a new world-switch system.

The port drives native state semantics from its existing session/frame and
streaming callbacks. This is not proof of instruction-identical timing or of
all challenge types running correctly. The new sequence needs a build
and gameplay coverage: completed film/photo, race/contest, AI S.K.A.T.E., Own
the Spot, win/loss/quit and another ordinary challenge afterward.

## Inspection performed

- Live IDA pseudocode and raw PPC inspections listed above.
- Static review of all MapAction consumers, loading/teleport acknowledgments,
  native PlayerUI focus propagation, replay-flag users and result cleanup.
- Compared changed hunks against pre-turn backups; unrelated dirty work retained.
- Rustfmt parser only (`--emit stdout`, output discarded, children skipped).
  No cargo check/build/test, shader validation or game launch.
- Scratch/backups: `.scratch/challenge-map-retry-20261011`.

