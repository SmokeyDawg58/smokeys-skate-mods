# Mission sign-up lockout after a Hall of Meat replay

Date: 2026-10-14. PC source only, solo.

## Report and finding

The reproduction sequence: enter a mission giver's D-pad sign-up area,
bail out of that area, watch the Hall of Meat replay, respawn, and return to a
mission giver. Sign-up prompts then remain unavailable for every mission giver.

The shared Lua native `Challenge.CompleteChallenge` omitted its final,
unconditional `cActivityManager::ReevaluatePrereqActivities` call. This is a
confirmed source mismatch, and it provides a concrete global lockout path:

1. HoM results Continue sends `cMsgHUDContinue`. The original flow-manager
   handler calls `Challenge.CompleteChallenge(data)` with default result
   processing enabled.
2. Completing a main activity sets `ChallengeAIManager`'s release state to 0.
   This applies to the HoM flow manager even though it has no AI competitors.
3. The release update advances states 1, 2 and 3 to 4. It does nothing in 0;
   `ReevaluateActivityPrereqs` is the transition from 0 to 1.
4. Result processing sometimes reevaluates activities as a side effect. With
   no results, it sets `mStateMachineTriggered` and skips that evaluation. The
   HoM completion hook also processes no rewards when its completed list is
   empty. A personal-best replay can take this path.
5. Without the omitted unconditional call, the AI release state can remain 0.
   `CanStartMainChallenge` requires 4, and every normal sign-up checks it.
   Returning to another volume cannot release that gate.

The previous [sign-up return audit](signup-return-2026-10-12.md) corrected HUD
visibility/input lifecycle issues, but did not inspect this missing completion
call. Those fixes remain in place.

## Live reference evidence

Live IDA was healthy, with auto-analysis and Hex-Rays ready. Database:
`sk82_na_m.xex.i64`.
Input executable SHA-256:
`3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.

Full decompilations and raw instruction bytes are retained in
`.scratch/signup-bail-lockout-20261014/native.json`.

| Reference | Address | Relevant behavior |
| --- | --- | --- |
| `Lua::ModuleChallenge::CompleteChallenge` | `0x82894af0` | Complete activity, optionally process results, then always reevaluate prerequisites. |
| `cActivityManager::CompleteChallenge` | `0x82cd9d80` | Notify subcontrollers while `MainActivity` still identifies the activity; clear it and resume globals afterward. |
| `ChallengeAIManager::CompleteChallenge` | `0x825b1e68` | Main completion stores release state 0 at adjusted `this + 0xc` (`0x825b1e98`). |
| `ChallengeAIManager::ReevaluateActivityPrereqs` | `0x825bb010` | State 0 becomes 1 (`0x825bb01c..30`). |
| `ChallengeAIManager::Update` | `0x825b1d60` | Only states 1, 2 and 3 advance; the existing six-update unspawn wait remains. |
| `ChallengeAIManager::CanStartMainChallenge` | `0x825addf8` | True only for release state 4. |
| `cActivityManager::CanStartMainChallenge` | `0x82cd8798` | Main-activity and subcontroller gates. |
| `cChallengeSignUpManager::IsAllowedToSignup` | `0x82da2d60` | Includes the shared main-challenge gate. |
| `cChallengeSignUpManager::Update` | `0x82db44a0` | Hides an unavailable sign-up and reevaluates existing collisions while waiting. |
| `ChallengeHOMManager::IsSignupAvailable` | `0x825b2fc8` | The two scoring bail flags also suppress sign-up; these gates remain. |
| `cActivityManager::ReevaluatePrereqActivities` | `0x82cdbaa8` | Loaded/not-quitting and game-mode gates; Career invokes AI reevaluation. |
| `cSignPostManager::ProcessChallengeResults` | `0x825e10c0` | Empty results skip its conditional evaluation. |
| `ChallengeHOMManager::CompleteChallenge` | `0x825bfec8` | HoM flow completion processes rewards only for a nonempty completed list. |
| `FrontEndState_HOMResults::OnMenuSelect` | `0x826591e8` | Continue posts `cMsgHUDContinue`, pops results, and triggers the reward/tutorial progression. |

The decisive raw branch in `CompleteChallenge` is `0x82894c84`:
`beq cr6, 0x82894cf8`. Skipping result processing jumps to the final evaluation,
not past it. The processed-results branch calls `ProcessChallengeResults` at
`0x82894cf4` and falls through to that same tail. Instructions
`0x82894d60..70` dispatch activity-controller vtable slot `+0x2c`, identified as
`ReevaluatePrereqActivities` by the debug type information.

## Authored flow

Read the original external `data\db\db.big` with the repository's existing
Vault and Lua bytecode readers. Archive SHA-256:
`6332b45232c4fe42257aa1082522fa8b9d594e2885767e6bba4fd74141273fa3`.

`hall_of_meat_sp_flow_manager` hashes to `0x3ed00d2efa759fe5`. Its `complete`
state has the following resolved handlers in `ChallengeCore.bin`:

- State-enter `h_C278D35C418DF912`, chunk `0x1850c8`: `FrontEnd.OpenHOMResults()`.
- `cMsgHUDContinue` (`0x1e053b88`), `h_77286B03F12B5A13`, chunk `0x186a6c`:
  `Challenge.CompleteChallenge(data)`. Bytecode supplies one argument; it does
  not pass a false result-processing flag.

The inspection script and decoded instructions are
`.scratch/signup-bail-lockout-20261014/audit-authored.py` and
`authored-flow-final.json`. This reads data only; it does not execute a synthetic
gameplay scenario. One unrelated unresolved handler reference is preserved in
the packet rather than invented. The initial packet is superseded by the final
packet, which avoids unsupported custom text-field decoding.

## Change and scope

Restored the unconditional call in
`crates/skate2-core/src/challenge/natives.rs`, after optional result processing,
matching native order. All callers of this Lua native use the correction,
including results-disabled and empty-results completions. The existing
reevaluation method still owns game-mode/loading/quit checks.

No sign-up proximity reset, forced AI state, arbitrary timer, HUD reset, scoring,
physics, save-data or Android change was added. The original release sequence
and ordinary bail/chase/service gates are preserved.

## Validation and remaining limits

- Live IDA decompilation checked against the decisive raw branch and stores.
- Read-only original-asset inspection confirmed the HoM Continue caller.
- Rustfmt syntax parsing passed for the changed Rust file, with child-module
  traversal disabled and formatted output discarded. Source was not reformatted.
- Source diff reviewed against the per-task backup: one behavior call and its
  explanatory comment. No tests, compilation or game launch were performed.

Confidence: **high** for the source mismatch and resulting blocked-state path.
The hands-on bail sequence has not been reproduced against this change;
in-game confirmation requires a rebuilt executable. The important follow-up is
the reported leave-volume/replay/return sequence, including a personal-best
replay without new HoM challenge completions.
