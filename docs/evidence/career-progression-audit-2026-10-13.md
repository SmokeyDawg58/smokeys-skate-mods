# Career unlock audit, 2026-10-13

PC only, solo. Requested review of unlock propagation and persistence, using
the live Xbox debug IDA database and original external Vault/Lua corpus.

## Results

- Inventoried 717 progression challenge records, 233 machines, 765 states,
  910 handlers, 38 groups, 39 exits, 47 point pools and 26 rankings.
- All literal challenge title/name, group, message, ranking and point-pool
  references examined in the progression handlers resolve. All 56 links
  between the four sets of 15 Hall of Meat challenges resolve.
- The 37 progression native names used by those handlers are registered,
  except `LockCostByName` in the unused test machine (`e9018dc0cc9d92c3`).
  `Service.DrainPoolViaProgression` is registered through `dynamic_hull`.
- Reviewed new/continued-career feature gates, Slappy -> HoM tutorial ->
  four HoM paths and rewards, sponsor acceptance, magazine covers and six
  path-ending messages. The original scripts remain the source of rules.
- A standalone diagnostic linked to the existing release core traversed
  255 available entries and ran all 60 HoM stages without a ScriptError.
  Available/completed maps, points, rankings and message settings then
  round-tripped through the actual save/load routines **in memory**.
  No player profile was read or written by this probe.

This probe injects wins and immediately acknowledges presentation callbacks.
It also visits initially available global activities. It is a propagation
and serialization check, **not a legal-order campaign playthrough**, objective
test, rendering test, or proof that every challenge can be completed.

## Native comparison

Live decompilations checked against the Rust consumers:

| Owner | Address |
| --- | --- |
| ActivateChallenge | `825e02c8` |
| ChallengeComplete | `825e00f8` |
| ProcessChallengeResults | `825e10c0` |
| QueueTriggeredStateMachine | `825d6c20` |
| GroupExitLogic / CheckGroupExit | `825ddcc8` / `825dd850` |
| IncrementPointsPoolPoints / CheckRanking | `825d78d0` / `825c7bc0` |
| SaveData | `825cdef8` |
| ProcessEntryFee / AwardPrizeCash | `825de3f0` / `825de2d0` |
| Get / SetChallengeSaveData | `825cf418` / `825cf560` |
| EvaluateActivitiesForPrereqs | `82cd9e88` |
| ReceiveReevaluatePrereqActivitiesMsg | `8289ee50` |
| UpdateEnviroInstanceLocks | `828afe60` |

No new missing activation link was established. In particular, the unusual
group-exit serialization really is present in native SaveData (lookup of a
point pool using the exit number). It was not silently changed or presented
as a port regression. The pre-existing empty environment-lock context producer
is an evidence limitation; its missing notification alone does not establish
an affected career unlock.

## Confirmed playability limitation

The late team-ender route unlocks the training facility and its teleporters,
but `selected_session_career.rs` discards `TeleportLocalToWorldLocation` for
world files other than BAM. The facility needs the original additional-world
load/unload lifecycle; teleporting coordinates within BAM cannot implement it.
This is an existing host gap, also documented in `resume-three-2026-10-11.md`.
It remains unimplemented in this audit. Therefore **the entire career is not
certified playable** by this work.

The challenge-handler scan also finds unbound Create-a-Spot/DMO recording
calls. Some are in shared handler branches. A static occurrence does not
prove they are reached by an ordinary career challenge; no such branch was
changed without establishing its execution context.

## Artifacts and confidence

`.scratch/career-progression-audit-20261013/`: `catalog.json` (correct catalog;
the earlier `survey.json` is superseded), `corpus_links.json`, `live-ida.json`,
the read-only corpus scripts, and `progression_probe.rs` / `probe-2.log`.
Probe exit 0; all assertions pass using the October 13 release core.

Confidence **high** for the audited propagation and serialization paths;
no claim of a complete playthrough or instruction-level parity. No production
progression changes were warranted by the evidence gathered here.
