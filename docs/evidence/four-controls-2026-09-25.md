# Four live control scenarios, September 25

All four scoped live scenarios PASS. Repeated visible game runs cover
ground grabs, rolling manuals, a high ollie followed by right-stick left,
and Y dismount followed by at least three seconds offboard. Solo work.
Do not treat a compiled callback or an animation alone as a passing scenario.

## Reproduction

The release client's optional demos feed raw pad samples through the existing
selected session. They do not force graph states or physics results.

- `--live-ground-grab left|right|both`: hold triggers on frames 32..272, release.
- `--live-manual-demo`: A push32..140, coast, right Y=-18000 on160..400, release.
- `--live-ollie-demo --live-ollie-hold 30 --live-ollie-tweak-left`: high ollie,
  then full left right-stick only while the published physical state is airborne.
- `--live-y-demo`: Y at frame32, neutral thereafter.

Baseline logs: `artifacts/four-controls-*-before-20260925.log`.
Left/right grab stop at frame32, InCandidateSlidingState Begin82ca33d8
(indices2295/2273). Manual stops184 at Pre246. Tweak stops51 at update828f2668.
Offboard stops123 at Pre372, after physical500 and Motion752 are reached.

## Source verification and reuse

- InCandidateSlidingState Begin82ca33d8/End82ca34e0: cached/UCOM checked
  ISkaterMotionGraph virtual+40 resolves through vtable82069688 to
  GetSlidingStateReadOnly825fdfc8. SlidingState+10 changes bit40000000.
  PDB identifies that bit as mInCandidateSlidingState; use the existing
  combined_slide_bits, not general Motion flags. Update825187c0 is BLR.
- DisallowPumping Begin82c9f460: resolve IChannelAnimatable then motion graph,
  end the five existing PUMP channels with duration0.1 and current-frame=false,
  then virtual+80=SetAllowPumping825fe028(false). End82c9f608 restores true.
  Reuse channel dictionary and blend implementation; extend its existing
  BodySpin wrapper to support false and clear the native40000000 bit.
  EndChannelPrematurely82d7d648 only writes when blend-out time equals zero.
- Ground-grab update thunks828f40c0/4188/4238/42b0 expand Riding277 once,
  then turning/scoring, leaf callbacks and animation in packed-slot order.
  Idle1318/1321/1324 are aliases of their parent updates. Reuse existing
  turning, scoring, HandBusy, EndGesture and PlayAnimation implementations.
- Air986/987/988 reuse thunk828f2668, already connected for991: Down251
  update828efe38 then PlayAnimation slot24 and ControlAirLegExtension slot25.
  Their existing authored filter objects use the same FilterMotionGraphIntent
  callback implementation. No new tweak physics or animation substitute.
- Pre expressions recovered from raw table8290aa90, Sustain from8291e254.
  Cache and short-circuit ordering remain owned by the existing controller.
  Pre246=c353 (8291114c);247=c354;248=!c354. Sustain246=c353,247/248=true.
  Pre372=c54||(c53&&!c48)||(c53&&c48) at82917464, Sustain=true.
  Pre233=c340;353=c344||c250;238/354=!c287&&!c225;
  230=!c288;236=!c343;240=!c287. Sustain231=c339,235=c342,
  353=c344||c250; verified native default cases remain true without reads.
  Pre241=c349 (82910f40),407=c259&&c260&&!c258 (82919ed0),
  22=c46 (8290b9d4),23=c52 (8290ba34); their Sustain cases are native defaults.

## Validation so far

Round1 build passed. Replays reached further boundaries: manual Pre241 at184,
tweak Motion986/KnownAir201 then Pre407 at52, offboard Pre22 at123.
Those conditions were traced in the original switch and added with their
native semantics. Round2 also includes DisallowPumping and ground grab exits.

The following rounds record subsequent fixes and successful completion.
No claim of complete game-wide reference parity.

Round2: left and right grabs each PASS330 ticks, hold33..272 then release to
Motion1170. Left reaches FSGrabInto1260 then Idle1321; right BSGrabInto1255 then
Idle1318. Tweak PASS160 ticks: Motion986/Action156/KnownAir201 on52..97,
then landing263 and neutral1170. Both-trigger hold reaches1324 then Pre135.
Offboard reaches752 then Pre25. Both predicates were recovered from the raw
switch:135=c224 at8290e460;25=c56 at8290baf0, native true Sustain for both.

Manual's InStateForTime349 is authored `greater=0.5,state=OnGround`.
Activation82c94920 resolves GetSubState relative to the tagged parent and reads
the controller's retained timer. GetTimeInState8292bbb0/82931c40 returns -1 for
missing/inactive states. Reuse existing name lookup, timer storage, and shared
comparison helper; do not duplicate timers or add elapsed time before selection.

Round3: manual active on184..330 with PhysOut is_manual=true and
is_kickturning=false, while rolling. Then Bumped276 and missing NoPush1187 update
at397. NoPush828f3bf0 aliases Switch1031/828f2fc0: Riding277 then turning,
score augmentation, AttachIntent, PlayAnimation slots28..31. Added that existing
callback sequence and its exit Pre128=c214 (8290e1c4, WillExpire).
Both-trigger next reached Pre197=c225 (8290fd64, Coffin target).
Offboard idle next reached Sustain279, verified default true at829221c8.

Visual captures inspected: `artifacts/four-controls-ground-left-held.png`,
`four-controls-manual-held.png`, `four-controls-tweak-apex.png`.
Manual260 shows a pitched board and lowered board-focused camera; tweak75
shows airborne board adjustment. Camera conditions already use published manual
flag && !kickturn flag, so no camera tuning was invented for these cases.

Original-asset regression mounted_original_body_shapes_mass_caches_and_hat_variants
PASS, including the extended real-channel check for replacing current-frame=true
with false and the source blend duration. Motion tests initially76 PASS/1 stale
test expecting an unimplemented offboard child; its actual result is now the
correct neutral Standing/Still752. That expectation was updated and the suite rerun.

## Completed scenario evidence

- Left/right grabs: round2 logs,330 ticks each, held33..272 then clean release.
- Both triggers: `four-controls-ground-both-round4-20260925.log`,330 ticks PASS.
- Rolling manual: `four-controls-manual-round4-20260925.log`,450 ticks PASS.
  Manual1328 is active184..330 (147ticks,2.45s) with is_manual=true and
  kickturn=false. The long route then bumps, reaches the source stance switch,
  and returns to1170 by450. This is not a claim of an uninterrupted four-second
  manual through the encountered terrain. The mid-manual capture confirms
  board pitch and lower camera, not merely a selected animation ID.
- High ollie/left tweak: round2 log160 ticks PASS; captured at75; physical
  KnownAir201 and Motion986 for52..97, landing afterwards.
- Y offboard: `four-controls-offboard-round5-20260925.log`,360 ticks PASS.
  Physical500 begins60; neutral Standing/Still752 lasts123..360,238ticks
  (3.9667s), exceeding the requested three seconds. Capture inspected.

Final offboard continuation: Sustain368 at829215a4..82921810 is
`!(c38 && (c39 || c11 || c12 || c42 || (c40 && c41)))`;
shared82920774 returns true. This preserves the original locomotion/dismount/
landing/runout/quit ordering, instead of forcing idle or suppressing errors.
The standing state's parent chain is752 ->308 ->105 ->33 ->8 ->1 ->0;
its remaining Sustain cases are connected.

Updated Motion test suite:77 PASS,0failed in
`artifacts/four-controls-motion-regression-final-20260925.log`.
Original-asset channel regression:1 PASS in
`artifacts/four-controls-channel-regression-20260925.log`.
The test suite binary predates the final Sustain368 addition; the actual
360-frame offboard replay validates that addition. Two redundant Sustain arms
231/235 were removed in the final duplicate audit; their existing implementations
remain unchanged. No duplicate expression arms remain in the touched dispatches.
Final release cleanup build: PASS,4m03s, `artifacts/four-controls-final-build-20260925.log`.
No compiler or live test process was left running. The release executable is ready.

## Controller launch

From PowerShell (no scripted demo, no automatic frame cutoff):

```powershell
$env:SKATE2_GAME_ROOT='<game folder>'
& 'target\release\skate2-pc.exe' --live-skating
```

XInput uses the ordinary pad path. Diagnostic state logging is optional and
disabled unless SKATE2_TRACE_SELECTED_STATES=1. The four specific input sequences
above are verified; arbitrary combinations and other known project boundaries
remain outside this milestone.
