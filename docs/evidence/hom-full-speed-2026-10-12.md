# Hall of Meat at normal speed (PC)

## Scope and status

All Hall of Meat slowdowns are removed, a deliberate parity difference for
smoother presentation at high frame rates. This is a
deliberate PC behavior change, not a correction to the original game's behavior.
Work is restricted to this repository; Android is on hold and was not edited.

Implemented in source; reviewed against the original control paths below.
No build, tests, or in-game validation were run for this change. The existing
PC executable predates this change and requires a rebuild.

## Reference evidence

Live IDA checks against the debug build with symbols:

| Reference | Native behavior |
| --- | --- |
| `VDWriter::UpdateHoMSlowMotion` `0x827fd448` | Requires the local actor, enabled HoM slowdown cvar, and bail/effect scoring flags. Evaluates normal bail, severe-impact minimum, and ending/recovery maximum curves. |
| `VDWriter::WriteData` `0x827ff8c8` | Calls the HoM speed evaluator at `0x827ffc68`; positive speed sends `SetSimRate` at `0x827ffcdc` with `(1/60)/speed` and one slow-motion tick. Visual/recording work is separate. |
| `Lua::ModuleHighlightReel::PlayHighlightReel` `0x82cd7ef8` | Reads the optional `useHOMEffect` argument, default false. |
| `cHighlightGroup::PlayHighlightGroup` `0x82cd7b68` | Stores `UseHOMEffect` at `0x82cd7b80` before starting the ordered highlights. |
| `cHighlightGroup::PlayHighlight` `0x82cd75e0` | Runs the pinned-frame replay through `RunReplayNIS`; ordinary playback overrides use speed 1.0. |
| `ICECameraMover::UpdateSimTime` `0x82a36d70` | Reads camera element 25, scales by 0.01, clamps to 0.01..1, and sets the scene multiplier. The camera mover also consumes this scale. |
| `ScenePlayer::GetDeltaTimeMult` `0x82861db8` | Multiplies override, stream, freeze, and scene rates. |
| `ICEUserChannelFreeze::FireEvent` `0x826d1b50` | Commands 1/3 freeze for a frame-count duration, 2 freezes indefinitely. Photo commands 4/5, killable notification 6, and time jump 7 have separate purposes. |

The original external `data/db/db.big` was read in memory. In its
`data\db\ChallengeCore.bin`, Lua chunk `0x189da4`, handler
`h_3F92184E06878553`, selects `HOMHighlightGreat`, `HOMHighlightGood`, or
`HOMHighlightAverage`, then calls `HighlightReel.PlayHighlightReel(group, true)`.
Instructions 56..60 explicitly load `HighlightReel.PlayHighlightReel`, the group,
and Boolean true before `CALL B=3`. Of the eleven direct PlayHighlightReel call
sites found across ChallengeCore/progressioncore, this is the only one passing
true; the others omit that argument or pass false. Classification therefore uses
the original group flag, not a scene name shared with another activity.

Confidence: **high** for the identified native paths and original script flag.
Full-speed playback is the requested host policy and has no parity claim.

## Changes

- Removed the dedicated live `update_hom_slow_motion` producer and its completed
  tick call. Normal bail, severe impact/bone break, and recovery no longer post
  HoM `SetSimRate` requests. This applies wherever HoM scoring is active.
- Kept native cvar indexing, scoring duration fields, Vault assets, and the
  shared simulation-rate controller intact. The old HoM cvar no longer enables
  a runtime speed request. No global normal-speed reset is posted each frame.
- Carried `HighlightGroup.use_hom_effect` through `Output::RunReplayNis`, the
  career consumer, and the queued NIS entry. Generic presentation replays pass
  false. Every highlight in a HoM reel carries its group's classification.
- HoM replay camera tracks return 1.0 for their time scale; both scene/replay
  progression and camera movement use that value. The scene rate is initialized
  to 1.0 on HoM entry, including the missing-track path. Replay audio reads the
  same effective scene rate as before.
- Skipped cinematic Timeout/Permanent/Owned freeze holds for HoM replays.
  Photo capture commands, skip/completion events, time jumps, scene loading,
  streaming gates, and user pause behavior retain their existing handling.
- The policy defaults off, is assigned for each replay, and clears for basic
  scenes and scene teardown. A fresh camera track defaults to authored timing.
  Film/photo replay time scaling and the editor's playback controls are unchanged.

The physics step remains 1/60 second; no force, collision, ragdoll, bone-scoring,
eligibility, recovery, recording, result-screen, or sound-effect logic was changed.
Removing time scaling changes wall-clock pacing, by design.

## Review and artifacts

Reviewed the scoped diff, both replay-output constructors, the sole production
NIS replay caller, camera rate consumers, and NIS entry/teardown state. Searches
found no remaining live call or implementation of `update_hom_slow_motion`.
The native cvar and PhysOut duration accessor intentionally remain as data/API
definitions. This source review is not a compiler or runtime result.

Local evidence and pre-edit backups: `.scratch/hom-full-speed-20261012/`:

- `before/`: the eight changed source files and three project status documents.
- `ida-reference.json`: eight live IDA function results.
- `original-lua.txt`: original call sites and the HoM argument bytecode.
- `source-review.diff`: this task's source diff against the backups.

After rebuilding, in-game checks still needed: voluntary and severe HoM bails,
automatic replay/skip/results/recovery, followed by a film or photo challenge.


## PC release build - October 12, 2026

Compilation followed. The normal command
`cargo build --release -p skate2-pc -j 1 --timings` succeeded with exit 0 at
`2026-10-12T09:19:27.0025899Z` (814.079 seconds wall time).
The new `target/release/skate2-pc.exe` is 42,463,744 bytes, SHA256
`C74ABAAA5C3CEEDE0B78CEF6E4DA45D3006A31D4588D52808D91E6A21CEA9175`. It includes both the postfx sampling correction and the
HoM full-speed changes. No source fixes were needed during the build. Existing
warnings remain (435 core, 18 PC). No tests or gameplay runs were performed;
the result awaits a hands-on check. This supersedes earlier uncompiled status,
while the visual/runtime verification limitations still apply.

Log and manifest: `.scratch/postfx-brightness-20261012/pc-release-20261012-120552.log`
and `.json`. Previous executable backed up as
`.scratch/postfx-brightness-20261012/skate2-pc-before-20261012-120552.exe`.
