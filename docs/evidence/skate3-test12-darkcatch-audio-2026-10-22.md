# Test 12: darkcatch audio and preserved features

## Observed failure

The user reports audible darkcatches in the supplied Test 10 executable,
and missing darkcatch sound in Test 11 while other skating sounds still work.
The supplied source archive and Test 11 have identical audio implementation;
no separate audio files were omitted from the Test 10 ZIP. This establishes
a current failure, not the historical reason for the binary/source difference.

The bounded `--live-controls-demo darkcatch-audio` uses the same raw flick
and RB timing as the existing original-data darkcatch corpus. It changes no
physics/animation state. A pre-fix release completed 720 ticks (12 simulated
seconds). The actual local audio records contained the darkcatch scorable,
inverted board and physical foot contacts, but no splice request for bank 16.

The actual Skate 2 `skater.vlt` / `skater.bin` in the user's `db.big` provide:

* class `aud_splicer`: `0x923ccb46ef5bf5ba`;
* collection `footcatch`: `0xf1647bfb782be97f`;
* minimum airborne time attribute `0x6bfb6a3f22aaf797`: **290 ms**;
* minimum foot speed `0xb63cc740d29ad118`: **0**;
* middle/high speed levels: **2** and **5**, retained unchanged.

`CheckForFootCatches` 0x8251c9d0 forwards physical false-to-true contact edges
to `PlayFootCatch` 0x8251c6c8. That function rejects intervals at or below the
authored minimum. The observed inverted right-foot contact was at audio frame
108 (local Y speed -1.1986678), and left-foot contact at frame 111 (2.2244465).
Both intervals were shorter than 290 ms. Earlier transient box contacts reset
the timers, so a valid S3 inverted catch need not pass S2's ordinary-flip gate.

## Narrow PC mod policy

`SkaterAudioPhysOut::is_inverted_dark_catch` derives eligibility from existing
recorded facts: airborne board, not wiping out, a named `_darkcatch` scorable,
and negative board-up / hips-up dot product. No new wire or replay fields are
introduced; the same classifier can inspect live and recorded audio records.

The app forwards this fact through `PhysOutView` and `SkaterData` to Contacts.
For this case only, the minimum off-deck interval is zero. Actual physical
contact edges, a nonzero off-deck interval, measured speed, native sample
selection, mixing, pitch and volume still own playback. Native S2 records and
ordinary flips retain their original Vault-supplied minimum.

This is an explicit PC mod audio adaptation, not recovered S3 audio parity.
Confidence: **high** for the observed suppression mechanism; the exception
is a design choice to support the integrated mod's shorter inverted catches.

## Checks

The existing real-data probes pass using a separate copy of the character save:

* darkcatch authored animation and named HUD, both leading feet;
* native/manual hold, release, completed score and HUD;
* grabs, landing, HUD and mission facts;
* air footplants and physical foot IK;
* ground boneless and fastplant authored takeoff;
* all six difficulties' air assists, native solver and animation.

Those use the Test 11 physics build: Test 12 adds an audio classifier and
audio-only policy; the tested movement code is unchanged. The initial
darkcatch probe with a test journal hit its existing process-singleton
installation guard on its second session. Running without that optional
journal passed both sessions; this was not a gameplay/animation failure.

Audio crate tests after the correction: **81 passed, 7 ignored**. The ignored
tests require separate original-data fixtures. Changed source lines were
audited against the supplied archive; all unchanged line endings are retained.

Final release build passed (11m11s after the module-reference correction).
EXE SHA-256: `07c7b6370ce999aaaa78c15d3fee719de223b43d06dc38b6ff8c71b0d665071d`.

The final 720-tick audio run requested bank 16 at frames 109 and 111,
with actual sample references, while the board was inverted. No further
catch was emitted during the held darkcatch. The pre-fix run requested none.
The mixed float PCM was finite and nonzero (peak 0.18768746). Background
audio varies between runs, so the full-mix RMS difference is not an isolated
foot-catch waveform comparison. A separate fresh 280-tick Kickflip run kept
the native catch timing policy; the combined demo had ended up offboard
after a bail and therefore was not used as that ordinary-flip control.

The exact final EXE also completed the rendered PlayerUI offboard-marker
sequence: 740 ticks, offboard return observed, 8.915422 metres of walking
after return, and zero watchdog stalls. This run did not request a screenshot.
Sign-in and lobby browsing succeeded with network build fadf63f8 / protocol 11;
there were no public lobbies to test actual multiplayer gameplay.

## Sharing and updates

The friends package must exclude the local profile, nickname, config game path,
logs, build inputs, network identity sidecars and unpacked game data. The user
keeps their latest profile in a separate local playable folder.

The user requires files-only distribution. No GitHub repository, release
service or scheduled job is created. The stock updater replaces the entire
custom executable; enabling it would discard this mod. Automatic compatibility
with every unknown future release cannot be validated or guaranteed. A future
mod update still requires a compatible build, or upstream mod support. Do not
label this package as automatically updating.
