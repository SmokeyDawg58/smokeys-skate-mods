# Replay controls and Trick Guide return audit — 2026-10-13

## Status

**Later panel-layout follow-up:** a temporary PC alignment adjustment was
subsequently reverted. The original wider lower
panel is retained again. See `replay-cac-followup-2026-10-13.md`; the input and
camera fixes below remain in source.

PC source changes only. The release build was stopped before completion.
The new full controller route has **not run**. Six focused ICELITE camera tests
passed before that instruction. The previous executable remains in place;
its earlier replay-entry test did not prove camera movement or Trick Guide input.

Work was solo in this repository. Evidence, original source backups and logs:
`.scratch/replay-trickguide-audit-20261013/`.

## Confirmed defects and changes

### Trick Guide left the returned pause menu waiting for an intro that never ran

`open_trick_guide` hides StartHub in the session. On return,
`PauseMenu::return_from_replay` reenters the menu with input blocked until its new
authored IntroComplete. The separate Trick Guide room renderer did not call
`MarkerUi::apply_pause`, so the old pause movie never received its hide request.
On return, the movie still thought it was requested visible and did not open
again or generate the new readiness callback.

The guide rendering path now passes the pause snapshot through
`live_skating_client` and `live_skating_view` to `MarkerUi::apply_pause`.
The hub visibility predicate also excludes the active/outgoing Trick Guide.
This uses the normal close/open screen queue and original readiness callback;
it does not force input ready. Native ownership: StartHub OnEnter **8265e4d0**,
TrickGuide OnLeave **8264e9e8**. Confidence: **high** source/control-flow match;
runtime return is pending a build.

### Camera buffer edits invalidated the native current-interval selection

`EditableTake::change_size` reset every current interval after inserting or
removing keys. A fresh lookup chooses the right side of an equal boundary.
Native editor lookup retains the current interval on both inclusive ends.
Consequently `Setup*Camera` changed the left interval's coordinate metadata
while the following edit selected a key on the right interval. The focused
Freecam commit test reproduced the edited pose being discarded by EndSetup.

Native **ChangeSize82a5b368** calls **SetDataPointers82a55d20** with a non-null
take. That branch rebinds the buffers without clearing `nCurrentInterval`;
only its null branch resets the cached indices. **Channel::SetParameter82a55678**
then preserves valid indices at inclusive editor boundaries.

The port now retains those indices through the splice. The existing forced
parameter evaluation relocates indices that were removed or no longer contain
the parameter. No artificial epsilon was added to hide the bug. This applies
to Follow, Tripod, Free and Game camera editing, hard clips and advanced keys.
Confidence: **high**; failure reproduced and six focused tests pass with the fix.

### Freecam height used the orbit-camera operation

Freecam called the orbit height operation, which adjusts only the look point
and limits the accumulated height to +/-1. Native **UpdateFreeCamHeight82a65020**
adjusts eye Z and look Z together, in that order, with a +/-10 accumulator.
Reaching a limit suppresses that step. It does not call ToBubble.

Added the separate free-height operation and wired Mode::Free to it. Orbit
retains its original look-only height operation **82a650d8**. Original frontend
height acceleration, +/-0.2 velocity cap and 0.6 decay are retained.
Confidence: **high**; height/aim and cap tests pass.

### Camera preparation omitted the collision-history reset

Native PrepareOrbit **8255da40**, PrepareTripod **8255db50**, and PrepareFree
**8255dd70** reset the ICELite world manager/collision state. PrepareGame
**8255dc88** does not. FE EnableFreeCamera **8266be00** prepares only for
`overwrite && !edit`.

The controller now signals that reset in the matching preparation branch;
the session consumes it before the next collision query. The collision
algorithm and its filters are unchanged. Confidence: **high** source match;
real geometry/camera-switch behavior remains a runtime check.

### Camera bubble endpoint and gesture placement details

ToBubble **82a606f0** and FromBubble **82a61a28** read the selected stored key.
The port's `key_point` used the current evaluation cache instead. It now reads
the selected stored endpoint. Incremental edits still accumulate in the
uncompressed value cache, matching SetKeyElementValueChange **82a60b40**.

Trick Guide APT SetState10 **004aee/004b20** places gesture items at
`48 + 130*i`, and button items at `47 + 130*i`. Both previously used 47.
Corrected the gesture base; connector bases and kerning remain authored.

## Other paths audited

- Native FE ReceiveInput **82682b68**, OnUpdate **826799f8**, BE camera updates
  and input-map entries 204..207 confirm the existing stick mapping. The
  cvar names LS/RS are misleading; they were not used to swap the controls.
- Help preserves an active camera edit; returning to the effects menu reloads
  committed effects as in the native mode setter. Camera commit and effects
  commit are separate original actions (effects commit on B).
- Speed remains 10..100 by tens, shake wraps four values and watermark wraps
  three. Original frame-step size, jump behavior, trim limits, clip/key delete
  and overwrite branches were reviewed against the reference packets.
- CheckStreamStalled **8265f590** only updates the overlay state, not the
  input gate. No invented input freeze was added for streaming.
- Six focused tests cover all basic camera kinds, Freecam translation/yaw,
  vertical movement/cap, commit/zoom, advanced-key edit/delete, hard-clip reset,
  and serialized camera/effect data. These are native-derived invariants, not
  numerical traces captured from the original runtime.

## Trick Guide panel-width finding and limits

Inspected the supplied `trickguide.png` again.
The wider bottom panel is present in the original asset geometry and texture
coverage; it was **not resized**. The original root places the bottom panel at
(748,370), inner (-3,-3), and the top at (809,77), inner (-59,4). Neither panel
hierarchy has a clipping mask or a script that equalizes their widths. The
constructor subtracts the same Screen_EdgeOffset from both inner X values.

The previous evidence packet
`.scratch/replay-name-trickguide-20261013/reference/trickguide-camera-layout-proof.json`
records identical Xbox/PS3 APT, CONST and GEO payloads and identical decoded
BC3 payloads for all four panel textures. At native UI coordinates, the lower
art has roughly 384 pixels of coverage and the upper art roughly 358–364.
This is source/asset evidence, **not a claim of complete screenshot parity**.
No matching original runtime screenshot is available for this page.

The earlier camera ClassRefSpec fix remains: FS Grab uses track3 rather than
the accidental track0. It was already in the executable at the start of this
audit; no new claim of an in-game camera comparison is made here.

## Validation and continuation

Passed before the build was stopped:

```text
cargo test -p skate2-formats --lib replay_camera_tests -j 1 --offline
6 passed; 0 failed; 278 filtered out
```

Log: `formats-camera-tests-3.log`. The earlier failing commit probe is retained
in `formats-commit-failure.log`.

The expanded `replay_entry_probe.rs` now drives Career -> Trick Guide leaf
navigation -> returned pause menu -> Replay Editor using controller input.
It checks Freecam height/yaw/strafe/zoom and committed pose, help return,
advanced key insertion, speed/shake/watermark persistence, both trim heads,
playback, visible pause return and 120 resumed simulation ticks. It does not
fabricate readiness or recorder frames. **Written, not run in this audit.**
After a build, the existing isolated-profile runner is:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-career-replay.ps1
```

Release build1 was stopped after 364.40 seconds, exit 4294967295. This was an intentional
termination, not a reported compiler diagnostic. No subsequent build/check
or game launch was performed. Existing EXE SHA256 remains
`446B21C7920DADDDD6D6471E259AA7E8315F32997A606A7D3F618BE020810503`.

No game-save writes were performed by this work. The game was running meanwhile;
the observed real-profile hash changed during the session from
`27320921DEC653166759E99232BC1DE525E072B08111A2137910C74F21522F09`
to `F8A8D2A2CF512772B488F7871F34361379C906F4CFF35EA06B9CC102C4493E56`.
It was not restored or overwritten. Android, simulation/physics, the always
available Film Pack, and the earlier requested presentation overrides were
not edited.

Do not present the existing EXE or the previous limited entry-script PASS as
validation of this new source. Full controller/visual parity is still unverified.
