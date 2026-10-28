# Minimap, district display, and deferred location saving

Saved-location work is deferred until career missions exist. Do
not temporarily bypass the original completed-challenge requirement. No
SKATER.P serialization or startup location behavior was changed in this task.

## Location-save evidence retained for later

Fresh IDA `SaveProgressionOrientation825c7448` only records a valid position
when the game type flags permit it, there is no additional-world override,
the quitting override is inactive, and at least one challenge is completed.
It reads the local actor's **suggested safe breadcrumb respawn matrix**, not
the current raw position. Its version1 chunk contains a validity bool and
twelve XYZ floats (four matrix rows). `LoadProgressionOrientation825c4340`
reads that chunk; `cSignPostManager.SaveData825cdef8/Load825e53d8` own it.
`EnterGame826788b0` uses `SetLocationStartUpPosition82559eb8` when valid,
otherwise the authored default start location. This belongs to SignPost
progression, not the CAC recipe chunk currently written by the editor.

Evidence captures: `artifacts/progression-save-native-20261001.txt`,
`profile-location-native-20261001.txt`, `district-save-native-20261001.txt`.

## Current minimap / area work

Reuses the original `hud2/hudminimap.apt` (FNV8d92a68f), its RX2 backing,
the existing minimap icon atlas, APT clip traversal, and CentralHUD movie.
Native template84 has FocusRight projection, not score HUD's FocusLeft.
Original stage positions/timeline transforms remain authoritative. Root
intro/outro and `mMiniMap` normal/mongocorp states are translated from the
fingerprint-checked original bytecode. Frontend audio requests are retained;
audio playback remains a separate, existing unimplemented backend.

The mongocorp state is `MiniMap::EnableNoSkateZone`, driven only by
`FrontEndManager::ShowNoSkateZoneMessage` 0x826389c0. `ReceiveMsg(cMsgWorldPainter_KeyChange)`
0x8263de80 passes a security-layer (6) key change through unless free skate
(`IsFreeSkateMode` 0x8255b808: modes 0/3/4) runs with `securitySetting` off, and
`ShowNoSkateZoneMessage` never shows it online (4/5) or in Party Play (2);
`StateGameUnload::UnloadGameState` hides it. `selected_session_minimap.rs` applies
the same gate to the per-frame key.

`MiniMap826bcdd8`, `RenderIcons826b7650`, `MapIconManager8262ea20` establish
speed-dependent smoothed heading, presentation-hips-relative X/Z placement,
71.281097 ring radius, 32-pixel icons, fade60..350, disappear1000, and authored
maximumRange overrides. Original local-player icon is explicitly disabled
on the minimap by `Initialize82635c00`; do not add a guessed player arrow.
Map location catalog is reused. Career/activity/online icons require those
actual producers; this task does not synthesize career completion/unlocks.

Security spawn/cull and pedestrian chase start/end are existing queue4
messages. FE handlers826063e8/82606410/82606438/82606460 attach/remove icons;
MapBackEnd add/remove routines retain distinct capacity10 pools. Security
headings are relative to the local actor; pedestrian chase markers retain
rotation0. Update82565b78/82565f60 preserve last transform when lookup fails.

`StartWipeout827c7858` hides minimap independently of HoM effect visibility;
`ReactivateScoring827c7230` restores its saved visibility. Pause, map and CAC
use existing screen ownership. Current selected session has no career
SignPost visibility overrides; original `Minimap Visible` default is true.
Future mission integration must feed the actual setting, not replace it
with a proximity heuristic.

Area names originate in WorldPainter layer5, not camera districts(layer4)
or nearest FELocation. `ReceiveMsg8263de80 -> UpdateDistrictZoneName82638820`
reads worldpainter_fe_location_triggers layout0, then CentralHUD state12.
Raw instructions prove full64-bit default-key comparison d7edbd362d7d2152.

`FrontEndManager::mZoneKey` (+0x25E0, `marker_ui.rs` `zone_key`), from the raw
instructions (2026-10-07): the constructor sets the default key
(`StringHash64("default")`) and nothing resets it between games. `ReceiveMsg` ignores
keys 0 and -1 (`cmpdi`/`cmpldi`), otherwise stores the key and calls
`UpdateDistrictZoneName`. That returns on -1 or the default key and, while
`cActivityManager::GetMainChallenge` is set, stores -1 (`std`) and shows no name, so
challenges and all of Party Play never show one. `IsInStadiumZone` 0x82606528, which
picks the countdown's stadium sounds, compares `mZoneKey` in full with
`StringHash64("stadium")` = e1dde899e21b999b; the port had compared only a low word,
and that word was wrong (e21b5a9b for e21b999b).
The original `mcDistrict` at (332,619.15) owns text, intro, hold and outro.
No new hard-coded area boxes or text layout were introduced.

Raw/evidence captures are `artifacts/minimap-*-20261001.txt`,
`minimap-ctor-raw.txt`, `minimap-apt-audit-20261001.txt`, and
`central-apt-audit-20261001.txt`. Host floating-point/VMX estimate boundaries
remain the same as existing Reckoning helpers; do not claim Xenon bit parity.

## Verification and remaining boundaries

- `cargo test --release -p skate2-pc -j1 minimap_ -- --include-ignored
  --nocapture` passed both tests against the original assets. Covers cardinal
  headings (including equivalent +/-pi), ring placement, range/fade, authored
  intro/outro, normal/no-skate animation, and district timeline expiry.
  Log: `artifacts/minimap-final-tests-20261001.log`.
- Final release build passed in 2m30s after core/test compilation:
  `artifacts/minimap-final-build-20261001.log`.
- Focused live Boneyard run passed and screenshot confirms original
  `ID_ZONE_CLASSIC` resolves to **The Boneyard** at bottom center, plus the
  top-right minimap. `minimap-boneyard-final-20261001.{log,png}`.
- Scripted map travel returned to gameplay with minimap retained;
  `minimap-map-travel-20261001.{log,png}`. Bail screenshot at tick80 confirms
  hiding; final-build tick650 confirms restoration after automatic recovery:
  `minimap-bail-20261001.png`, `minimap-recovery-final-20261001.{log,png}`.
- Tests used isolated `artifacts/edit-skater-diagnostic-saves`; normal save
  data was not touched. Initial unscripted background-window captures correctly
  had no focused HUD and are not counted as visible-HUD verification.

The 37 installed location icons all have miniMapVisible=false in the original
group definitions. They remain available on the challenge map; do not force
them onto the minimap to fill otherwise empty space. Security/chaser queue
handlers and native per-kind ten-icon limits are connected, but these bounded
runs did not establish a visible security/chase encounter. Career objectives,
pinch-point producers, Big Black services, online/party modes and their profile
settings still require their real owners; no fake markers were added. The
current selected NoProfileFreeSkate session uses default minimap visibility.
District keys are sampled from completed visual snapshots, rather than a full
new FE queue9 broadcaster; intermediate region crossings between presented
snapshots are not separately replayed. Pixel parity across every aspect ratio
and unimplemented game mode is not claimed.
