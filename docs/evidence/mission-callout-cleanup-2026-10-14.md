# Mission instruction callout left visible after completion

Date: 2026-10-14. PC source only; no build or game run.

## Report and cause

A playtest screenshot shows the right-side instruction, "Reach the end of the
run for your score to count", remaining in freeroam after a mission.

Presentation events 20/21 drive `TutorialInstructionHUD` through a named HUD
group owned by the challenge presentation director. The port's `remove_groups`
discarded the presentation records and queue entries but did not delete their
HUD groups. The old `tutorialinstruction` instance therefore remained on the
active HUD stack. Hiding the HUD for results/NIS only concealed it temporarily;
it could become visible again when ordinary gameplay returned.

## IDA reference

Live IDA was offline. IDA 9.3 batch mode read the separate saved debug reference
`world_probe_temp\waypoint_replay_20261014.i64`. Reference input
SHA-256: `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.
Decompiler output and raw PPC are retained in
`.scratch/mission-callout-cleanup-20261014/{native,removal-native}.json`.

| Native symbol | Address | Relevant behavior |
| --- | --- | --- |
| `cChallengePresentationDirector::TutorialCalloutHUDEvent` | `0x82e063c8` | Sets the instruction and HAL text, activates the presentation HUD group and shows/hides the tutorial component. |
| `cChallengePresentationDirector::CheckPresentationGroupForHUD` | `0x82e07d80` | Creates/adds the components required by the group's show events. |
| `cChallengePresentationDirector::RemovePresentationGroup` | `0x82e07af8` | Calls `GroupRemovalHelper`, then removes presentation map icons. |
| `cChallengePresentationDirector::ShutdownActivity` | `0x82e074f0` | Uses the same removal helper for the activity key, then removes presentation map icons. |
| `cChallengePresentationDirector::GroupRemovalHelper` | `0x82e07348` | Removes matching queued entries, deletes each group's HUD, destroys its records; clears active wait state if the active group was removed. |
| `HUDManager2::DeleteGroup(const char*)` | `0x82ceeb08` | Looks up the named HUD group and deletes it if present. |
| `HUDManager2::DeleteGroup(HUDGroup*)` | `0x82cee6e0` | Removes the group from the active stack before unloading its components. |
| `HUDManager2::RemoveGroupFromActive` | `0x82cec2d0` | Removes an active group and sets `mActiveGroupsChanged`. |
| `HUDManager2::FixupActiveComponents` | `0x82cedff0` | Rebinds to remaining active instances; components with no remaining binding receive `BindData(nullptr)` and `Show(false)`. |
| `TutorialInstructionHUD::Show` | `0x826be510` | A visible-to-hidden transition sends screen state `-20`. |

Critical PPC: `0x82e0741c` takes the presentation group's name at `+0x60`,
`0x82e07424` calls `HUDManager2::DeleteGroup`, before group destruction at
`0x82e0745c`. `0x82e074cc..0x82e074e4` clears the removed active group's speech,
timer/watchdog, teleport and location wait state. Native group removal clears
timer IDs; timer deletion belongs to the timer manager.

## Change and callers

Only `crates/skate2-core/src/challenge/presentation.rs` changed:

- Remove each matching group's queued entries and its named HUD group before
  dropping the presentation records.
- If the active group was removed, clear its speech, timer/watchdog and
  teleport wait state, matching the represented native fields.
- Keep other activities' presentation groups and active wait state intact.
- Reuse the existing HUD deletion/fixup path. It emits the authored hide state
  or binds another active group's instruction instead of forcibly clearing the
  entire tutorial HUD.

Production callers are `shutdown_activity_sub_controllers` and the existing
`ChallengePresentation.RemoveAllPresentationGroups` Lua binding. Pending-reset
processing and game unload already call activity shutdown. Thus the correction
is shared by missions using these cleanup paths, including reset/retry flows
that tear down their activity; a continuing objective is not hidden merely
because a results screen appeared.

The presentation-created HUD groups are separate from Lua-created groups
tracked by `cChallengeHudTracker`/`hud_owners`. That existing cleanup did not
cover them. Repeated group names within one activity are safe: native and port
`DeleteGroup` both ignore a name already removed.

## Validation and limits

Confidence: **high** for this native cleanup mismatch. No runtime reproduction
or claim that every callout bug is resolved.

- Reviewed native decompilation and the decisive PPC calls/stores.
- Traced both cleanup callers and the existing HUD fixup, screen-state drain
  and PC `TutorialMovie::set_state(-20)` consumer.
- Rustfmt syntax parsing passed (`--emit stdout`, `skip_children=true`); no
  formatting was applied. Scoped diff reviewed against the pre-edit backup.
- No tests added/run, compilation, game launch, save edits or Android changes.
- In-game confirmation remains: finish/quit the affected mission, check the
  callout is gone in freeroam, then retry and confirm its instruction returns.

Backups, the scoped source patch and review manifest are retained under
`.scratch/mission-callout-cleanup-20261014/`.
