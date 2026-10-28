# Scoring and original trick HUD — work in progress

This is an implementation checkpoint, **not a playable completion claim**.
The current release executable contains the completed marker UI, real fisheye
and marker-return noise, not scoring. The two parallel workstreams have
finished their effect work. Actual marker-return424/600-tick runs exited0 and
their screenshots verify noise appearing and clearing. Challenge-specific OOB
producers are still absent; there is no invented global boundary trigger.

Native evidence is captured in `scoring-ida-2026-09-30.txt` (module, holder,
sequence, scorables, database and meters), `scoring-collectors-ida-2026-09-30.txt`
(air/ground/grind/handplant/offboard collectors, metrics and rotation), and
`scoring-native-tables-2026-09-30.json` (typed layouts and all 269 catalog rows).
Original tuning and APT audits are in `artifacts/scoring-general-vault-20260930.txt`
and `artifacts/scoring-trickdisplay-audit-20260930.txt`.

Extend existing retained constructors and output owners; do not add a separate
input-driven score approximation. The current work lives in the existing
conditioners' `scoring/` directory. Shared Scorable records retain the distinction
between Sim zero initialization and PhysicsScoring constructor-unwritten fields.
Database lookup uses original sorted leaf collections and original tuning.
Meter thresholds, multipliers and duplicate penalties come from the Vault.
Signed-byte histograms, fused reward arithmetic, announcement/tap timing and
sequence ring-copy behavior must preserve the native operations.

Still required: complete collectors and their output/animation inputs, connect
ScoreModule::Update at the existing conditioner hook, original frontend trick
backend/manager, and the authored trickdisplay2 APT screen. Existing marker APT
assets/playback/font/localization/GPU infrastructure must be reused.

Core/app release checks now pass (artifacts/scoring-effects-*-check-20260930.log).
Implemented holder announcements, signed-byte repeat history, air reward and
sequence banking, retained sequence lifecycle and native ring publication,
and non-substituting sequence pattern matching (Sequence supplies false).
Four production ground scoring tests now PASS (manual release/finished length,
cancellation without banking or repeat penalty, normal revert finish, seventh
manual-revert interruption). Log: artifacts/scoring-ground-tests-20260930.log.
The full release test build took15m; do not repeat without a reason. Later core
test-target CHECK passed in artifacts/scoring-collectors-test-check-20260930.log.
No scoring runtime has run; the live hook remains unconnected, so these helpers
currently do not change gameplay scoring.

Collectors now extend the existing constructor-owned fields: ground/manual/
coffin/revert/powerslide, grind changes and continuous tracker, handplant height
and duration, offboard hippy jump, and aerial trick lifecycle/spins/body flips/
trajectory/gaps/entry/exit/threshold/output. Shared rotation and UID-context
helpers are reused; no separate input-driven system. The module finish helpers
preserve runout countdown, gestures, pending-air banking, sequence multiplier
ordering and line/session history clearing. All aerial lifecycle source passed
core/app release CHECK (scoring-air-lifecycle-check-20260930.log).

Typed external scoring input and entity-context population are being added;
the latest context batch still requires checking. Vehicle tests use the native
dampedCOM/local box; characters use native two endpoint cylinders; DMO tests
use existing probe hits and the original ten-unit ray/AABB check, not overlap
alone. Original entity classification follows RefSpec/scoring_entities. The
world producer must supply actual nearby owners/counts in native order, not
empty substitutes. ScoreModule selection/update dispatch, WipeoutScorer Update,
the world producer, backend manager and authored trickdisplay2 remain pending.

New raw evidence: scoring-{ground,grind,handplant,rotation,offboard,hippy,
air-tricks,air-metrics,air-gaps,module-finish}-raw-2026-09-30.txt;
scoring-air-lifecycle-keys-2026-09-30.txt; scoring-context-ida-2026-09-30.txt;
scoring-context-extra-2026-09-30.txt; scoring-cvars-2026-09-30.txt.
Twelve original scoring cvars appended at982..993, preserving existing indices.
Two pre-existing test compile errors were repaired solely for validation:
grind source Normal owner extraction and corpus DmoBankPending match.

Additional native traces: scoring-extra-ida-2026-09-30.txt. World producer
827af468 supplies sim/visual/replay ticks, challenge trick masks, player reset
flags, manuals-as-connectors and nearby entities. Reset827ab250 alone disables
all trick types; ChallengeScoringManager ctor82da2178 enables all14. Freeroam
IsScoringMultiplierActive827accc8 returns true with no main challenge. Do not
mistake reset output for finished external input. Full entity-context producers
still require integration with existing LivingWorld/simulation owners.

Two parallel workstreams ran despite older
SOLO notes: respawn_crackle handles original marker/OOB noise, fisheye handles
the actual scene distortion and combined final pass. Root retains scoring and
coordinates shared renderer files and all Cargo/runtime work.

Continuation: collector selection/dispatch/input/output now implemented, including
original animation-category smoothing, exit-before-enter, bounded line holds and
old-multiplier banking. Wipeout scorer now consumes original bone damage,
propagation, heavy-landing memory, body tweaks, rotations, nearby collision
scores and seven-area character damage. Original cvars994..1002 appended.
Core test-target release CHECK passed: artifacts/scoring-wipeout-check-20260930.log.
These helpers remain UNCONNECTED; no scoring/HUD runtime claim is justified.
The authored frontend and actual external world producer still need integration.
Native CollectNearbyVehicles/DMOs decompilation drops collection loops after
virtual out-parameter calls: use scoring-external-producer-raw-2026-09-30.txt.
PhysOut.Reset82e2f740 calls Scoring2.SetResetValues82e2a3f8 each Update5;
existing try_begin_update5_output already resets the complete output. Do not
reset it again at the later scoring hook or lose events published earlier.
Additional evidence: scoring-module-dispatch-raw, scoring-wipeout-{ida,extra,raw},
scoring-external-producer{-raw}, all dated2026-09-30.

Live connection continuation (validation in progress):
- ScoreModule.Update82dfcd18 coordinator and actual PhysOut sampler added;
  Animation -> Scoring -> GameCam hook now connected in source. New Simulation
  tick/visual-tick fields advance at SimUpdate1 before input, using actual mask.
  This connection is now release-built and runtime-tested (see latest below).
- Actual LivingWorld KD queries supply vehicle/pedestrian/DMO scoring context;
  DMO native32-candidate radius8 filter emits at most8. Full64 spline GUID,
  authored bounds and actual collision flags retained. Populated pedestrians
  remain explicitly unsupported by the existing constructor-empty owner.
- Distinct DMO scoring collision word+1108 now accumulates actor bits. World
  scoring clears vehicle/DMO scoring flags before later post-transform contacts.
- Existing player rows retain original manuals/reset flags and destructive Pops.
  Selected no-challenge input uses native all14-enabled constructor state,
  multiplier enabled, inactive patterns and empty replay DataTracker frame0.
  It reads actual game-mode and original HOM cvars1003..1005; SinglePlayer1 is
  NOT a free-skate game-mode0/3/4. No invented empty populated entities.
- Offline nearby-skater query uses actual SkaterManager order and body points.
  Network/ghost, populated opaque AI controller and spatial-debug branch are
  explicit unsupported boundaries; none are admitted by normal selected play.
- Native delayed landing reward tests, row flag tests and DMO distance test were
  added. Core test-target CHECK passed in scoring-world-contact-check-20260930.log.
  Full filtered scoring test build is running (scoring-production-tests-20260930.log);
  that compiler captured the PRE-live-hook source. Check the new connection next.
- Original TrickDisplayHUD change-controller added in ui/trick_display_hud.rs;
  not connected to rendered UI. Authored frontend/backend integration still
  required. Raw source evidence appended to scoring-frontend-ida-2026-09-30.txt.
- New world source evidence: scoring-live-producer-integration-2026-09-30.txt.
  Optional SKATE2_TRACE_SCORING=1 gives sparse events for runtime diagnostics;
  normal quiet play produces no added scoring logs.

Latest validation / frontend continuation:
- 22 filtered production scoring tests PASS (pre-hook helper snapshot),
 3 standalone production HUD-controller tests and3 backend tests PASS.
- Application release CHECK passes in40.64s; release BUILD passes6m19s.
 Original-asset scoring_entities class was added to startup's existing Vault
 selection; this enables real vehicle/DMO type classification.
- Scripted600-tick short ollie: tick10 sequence begin,23 ollie announced,
 47 trick end,52 sequence completion/banked10. No pending scoring error.
 Scripted600-tick manual: beginnings197/312, endings284/401, accumulated49,
 sequence completion411. Manual run exit0 (saved .exit); PowerShell wraps
 intentional stderr trace in NativeCommandError, which is NOT a game crash.
 Logs artifacts/scoring-live-{ollie,manual}-20260930.log.
- Core frontend bridge CHECK passes39.79s. Native backend constructor/reset,
 completion, wipeout, pure-spin and stance/name paths traced in IDA. New files
 ui/trick_backend.rs, ui/trick_display.rs and scoring/hud_frame.rs project
 completed PhysOut facts; they do not calculate new scores. App receives all
 completed tick packets between visual snapshots, avoiding lost events.
- Authored trickdisplay2 movie and localization added to shared marker APT
 assets/renderer. Score movie implementation now IN SOURCE; latest application
 CHECK passes45.73s. Original-asset application test BUILD is running; no
 rendered score HUD runtime verification yet. Do not claim visible UI ready.
- Native HUDHooks.ActivateTrickScoring/UpdateStance point at old
 HUDHooks.TrickDisplay. Shipped hudbase only registers central messenger and
 does NOT assign that object. Across screens.big .const files, hudbase is the
 only movie containing HUDHooks+TrickDisplay. Preserve obsolete calls as
 no-ops; new TrickDisplayHUD SetState7/10/11 drives the actual scoring screen.
- Authored removed bigIcon_mc and absent mcSwitch intro label are native
 undefined/missing-label calls, not missing game functionality. _gotoAndX
 82a1cf48 ignores targets<=0 without changing playback. No invented icon.
- APT score movie's full bytecode/timelines are in
 artifacts/scoring-trickdisplay-audit-20260930.txt; HUDHooks audit in
 artifacts/scoring-hudbase-audit-20260930.txt. Both are UTF16 PowerShell output.
 Original score asset fingerprint58c4b20b; three shared movies retain phone0,
 button1,score2. All original assets stay external.
- Remaining: execute original asset test; fix any actual traversal issue;
 complete/verify shared landing jolt (new unregistered ui/apt_jolt.rs source);
 validate actual HUD captures, repeat penalties, multiplier/line behavior and
 bails. Context bonus display slots and frontend audio still need attention.

September30 original asset validation follow-up:
- App release test initially exposed APT1:8:4 score header vs marker1:7:4.
  CompleteLoad82a1cbe0 -> Resolve82a1c560 -> Fixup82a1beb0 uses identical
  layouts without version conversion; parser now admits both exact headers.
- Shipped symbol56 lacks x1Left/x1Right/TL2/BL/BR stale AS targets. Only
  authored TL/TR/glow clips receive multiplier commands; no invented controls.
- scoring-movie-asset-test3-20260930.log PASS; all four multipliers, score,
  line timer, names and bail fade traverse/flatten real external assets.
- Existing original marker traversal and new frontend nollie/cab name test
  PASS through same release application test executable.
- Shared landing jolt now registered with native speed/recovery cvars.
  Original initializers82f57b70/82f57c90; Update826014c0; FocusLeft826082b0.
  Frontend counters advance per completed game update, not uncapped draws.
- Headless sessions no longer queue unconsumed HUD packets: visual snapshot
  subscribes the frontend; scoring itself remains unconditional.
- New ui/trick_context.rs source is NOT registered/connected yet; seven-slot
  big/entity popup research saved scoring-context-hud-ida-2026-09-30.txt.
- Runnable HUD release build artifacts/scoring-hud-live-build-20260930.log
  currently compiling. No HUD live screenshot yet at this checkpoint.

September30 live HUD and context follow-up:
- scoring-hud-live-build-20260930.log PASS2m21s. Live105-tick high ollie/spin
  screenshot scoring-hud-ollie-20260930.png shows original Ollie180 HUD,38pts.
- Repeated original ollie pattern1185ticks exits0; capture
  scoring-hud-repeat-20260930.png. Each stationary sequence below50 points;
  no live multiplier progression claimed from this test.
- ui/trick_context.rs NOW registered. hud_frame.rs reuses native threshold
  output and existing ContextGroups.remove for airborne Over UID display.
  Seven slots/limits3big+3entity/2000ms/refresh ordering source verified.
- Original AS context movie translated; default Imperial preserved from
  ProfileBackEnd.tSettings.Default825644d8. ProcessDistance8255a2b8 intentionally
  multiplies3.0 after translator rounds original meters. No invented3.28084.
- scoring-context-asset-test-20260930.log PASS original movie with big/entity
  popups and expiry, multipliers, timer and bail. Core current-rate accessor
  was test-only; exposed existing accessor for actual HUD timers.
- scoring-hud-app-suite-20260930.log:110PASS,5ignored; all active app/GPU tests.
  Two direct production context slot tests and one landing-jolt test PASS.
- Native FrontEndManager82640130 order corrected: HUDManager before
  TrickDisplayManager.Update. Audio commands bounded per update; no frontend
  audio playback backend exists yet.
- The PC froze; a failed diagnostic file-write was verified intact and reapplied.
  Current release rebuild scoring-hud-context-live-build2-20260930.log active.
  New --live-ollie-pattern selects original two-point Kickflip/etc patterns
  via existing loaded matcher; only raw pad samples, no intent injection.


## Reported localization and HUD distortion, September30

The completed kickflip1185-tick run exited0 and reached native multiplier1.5,
but its screenshot exposed authored text74 displaying ID_TRICKDISPLAY_15.
Other multiplier graphics are not this localized text symbol. Fixed at the
shared Clip text construction boundary, not by replacing this one label.
SkateAptString82a70b28 / GetPackedString8297f2f0 hash non-# strings;
# supplies literal text. LanguageManager82d66a78 initializes missing-string
fallback from gEMPTY_STRING830248c4 ->8233afe8, one space. Authored placeholders
therefore do not show raw IDs. Dynamic strings already decoded are preserved.
Original score asset test now checks text74 == x1.5 AND four rendered glyphs;
original marker asset traversal also passes. Captured native evidence:
score-text-localization-2026-09-30.txt.

A comparison then exposed incorrect final composition. Earlier reasoning
stopped at RenderModeScene's FE.Render3D call and misidentified HUD ownership.
Actual chain: RenderModeScene82857a78 -> PostFX.Apply at82857dec ->
FrontEndManager.Render2D at82857ea4 -> AptIntegration.Render2D82633a18 ->
AptRenderingIntegration.Render2D82631a98. The latter selects APT_3D camera,
sets level5 at82631bc8 and calls AptRender(level5) at82631bd0.
Both PhoneListHUD and TrickDisplay templates are type5. Render3D8262a550
instead draws the separate mbEnable3dScreens world-menu texture quad.
Therefore HUD keeps native left-focus perspective but must draw AFTER scene
fisheye, vignette and marker noise. Corrected gpu_renderer ordering without
changing angle11.25/distance624 or weakening original world fisheye.
Full nested callees and raw call/layer evidence: score-render-order-2026-09-30.txt.
Prior effects documents claiming snow/warp over this HUD are superseded.
Release follow-up build and live visual comparison pending below.


Follow-up verification COMPLETE: release build passes (2m27s),
artifacts/scoring-hud-order-build-20260930.log/.exit. Original-asset score
localization/glyph regression and marker traversal both pass. Scripted
Kickflip/high ollie+spin105-tick run exits0, reaches1.5x and its actual image
scoring-hud-order-fixed-20260930.png displays x1.5, undistorted/undarkened HUD.
Marker-return424-tick run also exits0; scoring-hud-marker-order-fixed-20260930.png
shows scene snow behind the original clear phone popup, matching the recovered
render order. No changes to gameplay scoring, UI perspective angle/distance,
or original fisheye shader coefficients. Current release/play-quiet includes
both fixes. Screenshots are host validation, not a claim of full frame/pixel
parity against the reference build. Both scripted test sessions completed;
a separate skate2-pc process was observed afterward and left untouched.


## FPS font and bail gesture array follow-up (in progress)

The FPS counter uses the original trick-name font; a playtest hit scoring array
5717cfccca59bd47 (decimal6275713084239232327). The bail body-tweak consumer
incorrectly selected scalar storage; switched to existing typed POD array
reader after IDA82dec470 + original Vault24-byte/four-record verification.
Evidence: scoring-body-tweak-array-2026-09-30.txt.
FPS font is selected from actual mcTrickAnim0/mcTrickName/mcText/labelText,
using its existing font atlas and shared original glyph metrics. White24px
capital height,10px margin, screen-space overlay after HUD; FPS still counts
completed presentations over half-second windows. Geometry rebuilds only on
changed displayed FPS or dimensions; original bitmap fallback remains for
standalone inspection modes that do not load frontend assets.
Release build fps-font-bail-scoring-build-20260930 underway; live verification
will follow. No frontend font assets or second renderer duplicated.


Follow-up: a playtest hit RelocatedPayload(BIN800504). PDB confirms
BodyTweakAttribData+4 is const char* mHALID, not padding/POD. Final consumer
reads only numeric ranges0..4 and8..24 through bounded typed array field
projection. Original ID/gesture/15-frame/500-point behavior retained. No pointer
validation disabled. See extended body-tweak evidence.
The earlier build was interrupted/no final success record; do not claim final
FPS/bail changes tested. All final source changes ready for build-release.bat.
Also a slightly brighter world/sky: final scene RGB gain defaults1.08,
SKATE2_BRIGHTNESS=1 restores unadjusted output. This is a PC visual preference,
not a claimed original exposure/tonemap reconstruction. HUD/FPS and original
warp/vignette/noise constants remain outside this adjustment.

October1 brightness adjustment: default final-scene gain1.08 ->1.188,
exactly another10%. Existing world/sky-only final pass reused; HUD follows it.
SKATE2_BRIGHTNESS override remains absolute (1 restores unadjusted output).
This is an explicit presentation preference, not original-game tuning.
Source-only change; not recompiled.
