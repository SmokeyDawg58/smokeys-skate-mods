# Multiplier ring: deferred timeline actions

Date: 2026-10-14. PC only, solo. **Source correction; not built or run.**

Playtesting showed an intermittent dark/black ring at x2, although that colour
belongs to x3. The colour constants and multiplier notifications were correct;
the retained score movie executed timeline actions too early.

## Reference and original assets

Used live IDA, with Hex-Rays and auto-analysis ready:

- Database: `sk82_na_m.xex.i64`.
- Input SHA-256:
  `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.
- Decompiled functions and raw instructions are retained under
  `.scratch/multiplier-ring-20261014/native*.json`.

Decoded `data/fe/source/screens/hud2/trickdisplay2` directly from the external
Xbox `data/fe/screens.big`. The original payloads were kept out of the repo.
The complete timeline/ActionScript audit is in the scratch directory.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| .apt | 146460 | `912a4df07c75a6dc8f34d7c291a5668fd0e0ed57437fe326352ca89eeed97143` |
| .const | 10723 | `6368b625edf23adcbe6cdf2d807241d1acffd09e0b08042e835527345ca6dca0` |
| .geo | 3832 | `40f6f2435f09facfa9a0ad5fb7ff6bac5f11020f459e1d95a4828eae09c441b8` |

These three files are byte-identical to the PS3 version's files in
`PS3_GAME\USRDIR`.

Original UpdateMultiplier (`.apt` `0x21430..0x21562`) calls gotoAndPlay on the
background, TL, TR, glow and score backers, in that order. Removed instance
names in the script still resolve to undefined and have no visual effect.

The retained timer's TR sprite (character49) has these zero-based frames:

| Label | Frame | Stop action | RGB bias |
| --- | ---: | --- | --- |
| x1 | 0 | `0x232bc` | `cbcbcb` |
| x1.5 | 14 | `0x232c4` | `02bf38` |
| x2 | 29 | `0x232d0` | `f7ee0d` |
| x3 | 44 | `0x232d8` | `202020` |

The independent TL sprite (character55) uses `02bf38`, `f7ee0d`, and black
for its authored states. Its stops follow the label frames. The glow has its
own stop frames. No uniform ring tint or substitute colour table was added.

## Confirmed dispatch mismatch

1. `FrontEndManager::Update` (`0x82640130`) runs
   `AptIntegration::Update` at `0x82640220`, then HUDManager2 at `0x82640240`,
   then TrickDisplayManager at `0x82640294`.
2. `TrickDisplayHUD::Update` (`0x826bda40`) sends state10 when the cached
   multiplier changes. Its notification order and the float translator
   `0x826b8df0` match the port.
3. The state reaches ActionScript synchronously through
   `FrontEndManager::SetScreenState` (`0x826063c0`),
   `ScreenManagerBase::SetScreenState` (`0x82d65d50`),
   `ScreenManager::CallASFunction` (`0x82696688`), AptIntegration, and
   `AptCallMemberFunctionInternal` (`0x82a12ef8`). This call does not drain
   the queued timeline actions before returning.
4. `_gotoAndX` (`0x82a1cf48`) jumps and sets playback. `jumpToFrame`
   (`0x82a1b3d0`) immediately changes placements but queues target actions
   through `queueFrameActions` (`0x829e0518`). Same-frame jumps queue nothing,
   but gotoAndPlay still resumes the clip. That existing behavior is correct.
5. `_tick` (`0x82a21c10`) advances the display list and THEN invokes
   `RunActions` (`0x82a1c828`), once per authored interval. Thus a stop queued
   by an external HUD call survives to that boundary; it is not executed
   inside gotoAndPlay.

Previously `Clip::apply` ran the stop immediately, including for external
goto calls. A subsequent same-frame gotoAndPlay could resume the clip with
no remaining stop action. Its ring colour could then advance independently
of the multiplier background.

### Source-derived bail/reset sequence

`PhysOutModule_Hud::ReactivateScoring` (`0x827c7230`) calls SetReadyToScore.
`TricksBackEnd::SetReadyToScore` (`0x82565048`) calls
`TrickDisplayManager::Reset` (`0x82607c90`), which calls
`TricksBackEndCore::ClearData` (`0x82a735f8`) and sets the backend multiplier
to zero. The next ordinary HUD physics update (`0x827c79b8`, write at
`0x827c7cd0`) publishes the scoring multiplier again. Both zero and one map
to the authored x1 label.

For a ring previously at a higher multiplier:

- Old port: zero seeks frame0 and immediately stops there. One seeks that
  same frame and resumes. TR can run to the x1.5 stop at frame14; a later
  x1.5 request can resume it to x2/frame29, and x2 can resume it to the dark
  x3/frame44. The background uses different stop frames, so it can still
  display the correct x2 glyph.
- Native ordering: the frame0 stop remains queued. Whether the one update
  arrives before or after the next authored tick, the queued stop is retained
  or a fresh one is queued by seeking back from frame1. This prevents that
  chain of prematurely consumed stops.

This is a control-flow diagnosis, not a captured reproduction of the reported
particular gameplay incident. Same-frame gotoAndPlay remains native; it was
not replaced with gotoAndStop or a forced colour refresh.

## Changes

- Added an optional FIFO for retained clip frame actions and enabled it for
  ScoreMovie after its existing construction. Placement/matrix/colour changes
  remain immediate; subsequent frame actions run after the authored tick.
- Queue entries identify an instance, so removing a clip cannot redirect its
  pending action into a new clip at the same depth or name.
- Preserved `RunActions`' frame guard: normal tick actions have a negative
  source-frame tag and are skipped if that frame has changed. Explicit goto
  actions use nonnegative tags and survive subsequent seeks. Frame0 is
  unguarded. Raw stores: tick `0x82a1b70c`; jump `0x82a1b568`; guard
  `0x82a1c8c0..0x82a1c8e0`; queue copy `0x829d9b68`.
- ScoreMovie processes callbacks between queued bodies, so line-timer expiry
  can append the original parent `clear` actions to the same FIFO.
- Restored the authored background/TL/TR/glow call order. Existing movie
  initialization and other screens' dispatch remain outside this change.

## Review and limits

- Two modified Rust files syntax-parsed with Rustfmt `--emit stdout`,
  `skip_children=true`; it did not rewrite them.
- Reviewed up/down multiplier seeks, repeated x1 requests, retained timer
  rewinds, natural stops, stale-frame suppression, removed/replaced owners,
  callback-generated actions, and fresh HUD ownership on game unload.
- The UI owner stays on the presentation thread; its shared queue is not
  passed to the SIM worker. Queue entries do not retain clips, so dropping
  the old score movie releases its queue.
- No new tests, Cargo check, build or game launch. Runtime confirmation of
  the ring after a bail/reset and subsequent x1.5/x2/x3 transitions remains
  pending. The earlier October1 tests did not assert the TR ring state.
- No scoring thresholds, points, decay, multiplier calculation, physics,
  save data or Android changes. This does not claim a complete general APT
  scheduler reconstruction.

Confidence: **high** for the reference timing mismatch and source repair;
the exact reported occurrence has not been reproduced in-game.
