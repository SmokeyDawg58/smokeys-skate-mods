# Hall of Meat unlock follow-up

Date: 2026-10-14. PC release built afterwards.

## Finding

Finishing Slappy's opening tutorials alone does not put the Hall of Meat stages
on the challenge map. After Follow Slappy, the original `hom_tutorial` runs as
a background global activity. A qualifying bone-breaking freeroam bail, outside
a main mission or security chase, completes it; continuing the HoM results then
processes its progression messages and exposes the HoM challenges.

That bail had not been performed. No forced unlock or new
prerequisite was added. The original four 15-stage HoM progressions remain.

## Reference and source correction

IDA 9.3 batch mode read the separate saved debug database
`world_probe_temp\waypoint_replay_20261014.i64`, input SHA-256
`3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.
Live IDA was unavailable. Output and disassembly are retained in
`.scratch/hom-unlock-followup-20261014/native.json`.

| Native symbol | Address | Inspected contract |
| --- | --- | --- |
| `ChallengeHOMManager::CanEnableHOM` | `0x825ae488` | Original freeroam/mode/sign-up/chase gates. |
| `ChallengeHOMManager::Update` | `0x825b30a8` | Enable/update the background HoM flow. |
| `ChallengeHOMManager::CompleteChallenge` | `0x825bfec8` | Collect completions until results processing. |
| `cChallengeObjectiveGroup::Update` | `0x82d4f608` | `FindByIndex(ActivePlayer)` for continuous trigger state (`0x82d4f638..0x82d4f6b8`). |
| `MakeObjectives_helper` | `0x82d52de0` | Activity/objective group/player ownership. |
| `ObjectiveHOM.GetNumBonesGTOEDamageLevel` | `0x82d47dd8` | Bone predicate supplied by the assigned skater's real HoM state. |
| `ResumeSuspendedGlobalChallenges` | `0x82cd9080` | Resume original global activity graph. |
| `SuspendAllGlobalChallenges` | `0x82cd9c40` | Suspend graph evaluation while a main challenge owns gameplay. |

One shared source mismatch was found: continuous objective updates used the
local `world.player` frame regardless of the group's assigned player. Changed
`challenge/objectives.rs` to find the group's `player` in `world.skaters` and use
that frame's HoM state. This follows the original lookup and also applies to
AI/Party Play objective groups. It does not demonstrate that this mismatch was
responsible for the reported freeroam case.

Added an opt-in `SKATE2_TRACE_CAREER` HoM-start diagnostic in `challenge/hom.rs`
showing enable state, current main challenge/chase and relevant objective groups.
It changes no unlock, score, bone threshold or slowdown behavior.

## Inspection and limitations

Original data and a read-only snapshot of a playtest save were inspected using
an already-built library. Snapshot SHA-256:
`11770b5f3c610d398dabff9644677b0889a5fcf783b9ec9e972cda1d012ce88b`.
The scratch helper supplied synthetic wipeout/bone events, suspended/resumed a
main challenge, and continued results in memory. It showed the background
tutorial returning after the mission and its completion/progression messages
after the bail. Logs: `inspect-hom-2.log` through `inspect-hom-4.log`.

This was an inspection of an older compiled library with injected events, not a
legal gameplay run or a test of the changed source. The real save was not written.
At the initial source handoff the changed files were syntax-parsed only.
The subsequent normal PC release build passed; see
[the Contacts release record](career-contacts-security-2026-10-14.md#release-build-follow-up).
No regression test or game launch was run. Confidence is **high** for the native
source and authored-data contracts, with physical-bail/UI/runtime confirmation pending.
