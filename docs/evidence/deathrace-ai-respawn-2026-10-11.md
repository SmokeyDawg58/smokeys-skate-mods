# Deathrace AI recovery continuation — 2026-10-11

Report: a normal deathrace failed after 3,331 completed frames with
`Tick(Before(TeleportingActor(PendingAIController)))`. The exact race and
triggering actor were not identified. Work is solo; the Jan17 rich-symbol
IDA database is read-only. Scratch files and copied saves are under
`.scratch/deathrace-ai-respawn-20261011`.

## Cause and implementation

`Actor::TeleportSomewhereAppropriate` had an explicit fatal placeholder for
every non-ambient AI controller. A racer entering physics recovery could
therefore abort the whole simulation while the human continued skating.
This was an unimplemented dispatch, not a player-input failure.

The Actor now invokes the live session AI owner at the original recovery
call point. It is connected through both physical recovery callers
(Update0 trapped-in-collection and state702's Update3) and the security
capture caller. No precomputed destination or extra controller reset is
introduced. The query reads the current controller, paths, challenge respawn
configuration and other Actors' pending teleport destinations.

| Reference | Recovered behavior |
| --- | --- |
| Actor.TeleportSomewhereAppropriate `0x825f0e18` | AI query first; a successful query calls Teleport(matrix,true) and returns without changing spawn stance. A false query continues to ordinary breadcrumb selection and its stance/on-board choice. |
| Raw `0x825f0e38..74` | Ambient unspawn requires **both** controller client type0 and backend player type2. |
| AISkaterManager.UnspawnAISkater `0x824cbc10` | Queue destruction, unregister its recipe and remove the backend player at the reached call. The old Actor flag deferred this until the next manager update; that delay is removed. The Actor still takes the breadcrumb path afterward. |
| AIController.GetAppropriateRespawnTransform `0x824cc208` | Delegates to PathController. |
| PathController.GetAppropriateRespawnTransform `0x824ca430` | Use a configured challenge locator unless path respawn applies. Otherwise temporarily resolve mPathID when mPath is null, select a node, and restore the prior mPath. Missing path returns false; it is not an error. |
| Same `0x824ca4e0..548` | Reuse the cached respawn node when mRespawnNodeFound is set; otherwise find one. This query does not write a newly found node back, mark it found, change the current node or reset the controller. |
| IsInPathRespawnChallengeType `0x824bf108` | Tests whether ChallengeAIManager.mRespawnLocation is null. |
| ShouldRespawnAhead `0x824bf318` | Behind-always first, then behind-in-races; challenge types19/20 recover behind by default. |
| FindRespawnNode `0x824c7660` | Behind uses the approximated node; otherwise FindRespawnNodeAhead. |
| FindRespawnNodeAhead `0x824c75b0` | Current node+2 capped at the last node; the blocking-object advance uses a strictly positive difference strictly below its live limit. |
| GetSpawnLocation `0x824bee38` | One locator uses index0. Multiple locators begin at index1, choose the greatest nearest-skater distance, and keep the first on ties. |
| GetClosestObstacleDistanceSquared `0x824becb8` | Skip the current player; read each other Actor's path-relative position, replaced by its pending teleport position when the external-reset bit is set. |

The existing path-node selection is reused. `FindSafeNodeIndexBehind`
`0x824c1270` and `FindSafeNodeIndex` `0x824c1570` were checked for their
air/trick/trajectory margins, slope gates, scan direction, first accepted
node and fallback endpoints. No demonstrated control-flow mismatch was
found there.

## Related recovery audit

Checked `WipeoutBegin` `0x824c76e8`, `HandleWipeouts` `0x824cd780`,
`UpdateWipingOut` `0x824cd518`, `TeleportToSpawnLocator` `0x824cd4a0`,
`TeleportToNode` `0x824ca3a0` and Actor.Teleport `0x825ed1c8` against the
existing implementation. The normal AI wipeout path already handles the
duration, fade, path/locator choice and settling. Locator recovery already
gets30 settling frames from FullReset; path recovery explicitly sets8.
The newly connected emergency query must not perform those resets itself.

Four formerly fixed AI respawn settings now use the existing live cvar
registry, including the pre-existing normal-wipeout callers:

| Initializer | Setting | Native default |
| --- | --- | --- |
| `0x82f4cc28` | kbAlwaysRespawnOnPath | false |
| `0x82f4cd38` | kRespawnBehindInRaceChallenges | true |
| `0x82f4cd88` | kRespawnBehindAlways | false |
| `0x82f4de58` | kMaxNodeAdvanceForBlockingObject | 8, range0..256 |

Race progress/estimated times (`cChallengeRaceManager.Update`
`0x825c2500`) and race rubber banding (`0x824bf3a8`) were also checked.
Those paths are already implemented; the older source comment claiming
estimated race times are unported is stale. Existing race UI, gates and
win/loss/Retry/quit coverage is recorded in the earlier October11 evidence.
The original Vault scan in `race-data.txt` also confirms that the authored
deathraces share `run_heat`/lap path handlers (`eebae09794cefba7` and
`23ffaf72ebb4c8f0`). The recovery dispatch is shared by their AI Actors,
rather than being conditional on one mission name.

## Verification and limits

- Initial `cargo check --release -p skate2-pc -j 1` passed in1m27s after
  connecting dispatch, before the live-cvar follow-up.
- Final `cargo build --release -p skate2-pc -j 1 --timings` passed in13m53s
  after the live-cvar follow-up (`build1.log`).
- `race1`, copied latest profile, original `r_1_1_fabio_mem1`: loaded the three
  challenge racers and their original path package, entered `run_heat` at
  diagnostic tick3086, cleanup at5968, results at6089, ready at6125, B/Quit
  at6185 and free roam at6195. Continued past6300 controlled simulation
  frames with no reported failure. This exercised natural AI racing and
  results/quit cleanup, but **did not reach** the emergency respawn query:
  there are no `appropriate respawn` trace entries. Do not treat it as
  runtime proof of that branch or of the unidentified reported race.
- Closed the scratch game through CloseMainWindow; the process is gone, but
  its final exit status was not retained. The prepared `race2.json` was
  not launched. The exact race can be replayed with the final
  release executable; no compiler or diagnostic game remains running.
- No unit tests added or run. Diagnostics use copied profiles only.
- Real profile SHA256, confirmed unchanged after the diagnostic:
  `1F94A562F8B724CDD74C48226FBCD16B21462541AC107F96D6216A59E345BB1C`.

Confidence is **high** for the reconstructed branches and call ordering;
the cited raw conditions and initializer values are **exact** observations.
This does not establish numerical or instruction-level parity. The existing
path matrix helper uses affine translation W=1 where the original node
helper leaves a stack lane unspecified; that pre-existing host convention
and the project's VMX numerical limits remain explicit. Debug-only navmesh
controller swapping remains disabled by its reference default and is not
represented as implemented here.
