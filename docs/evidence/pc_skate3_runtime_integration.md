# PC Skate 3 simulation integration

This is the user's explicit PC mod, separate from Skate 2 reconstruction
parity. Its accuracy target is the captured local Skate 3 Rust engine and local
changes. No console-parity claim is made.

## Agreed behavior

Implement full Skate 3 mode first, then reuse its dismount/offboard/remount
behavior for the hybrid. Selections, in order:

1. Skate 2 + Skate 3 Offboard: original Skate 2 onboard; Skate 3 owns the two
   board transitions and all offboard behavior.
2. Skate 3: the native captured Skate 3 player schedule owns all player/board
   physics in the Skate 2 world, including NPC/car/prop interactions.
3. Skate 2: original simulation.

Difficulty labels: Skate 2 Default, Easy, Normal, Hardcore, Motorized,
Easy Motorized, Skate 2 Motorized. RB holds the
native motor. Easy Motorized adds the native motor to Easy; Skate 2 Motorized
adds it to original Skate 2 tuning. The user clarified on 2026-10-20 that
Skate 2 Default means the regular, unchanged game. It is now exclusive to
stock Skate 2 mode; Skate 3 mode enters Normal. There is no Normal alias under
the Skate 2 Default name. Skate 2 Motorized is also unavailable until its actual
stock-plus-motor path is implemented.

The user initially prohibited game launches, then explicitly authorized loading
the game when it helps implementation. Rendered and normal-menu launches have
now been performed with a separate copied save; see the preview evidence below.

**Latest asset instruction supersedes the earlier assets-only restriction:**
required Skate 3 assets may be used if compiled inside the EXE. No separate
Skate 3 installation, asset directory, extraction or cache is required. The
configuration deliberately has no `skate3_asset_root`. Original Skate 2 ROM
data remains external as in the base port.

## Source and asset ownership

- Skate 2 baseline: `22063407933ce9e35c59b2fb62062e7d331ac513`.
- Captured local Skate 3 revision: `60efdef86600d8d8d4feb4b7c608fa0efd0643d7`.
- Sources: `vendor/skate3-engine`. `SOURCE-SNAPSHOT.json` records captured
  hashes and local modifications; embedding edits are distinct from that record.
- The original Skate 3 checkout has not been modified.
- Original asset inputs remain outside the repository. The local player pack
  has 18 files, 25,755,874 bytes; SHA-256:
  `e23b345188e5fd1863751468975693275e450166ca40a62b28de9545f8b5120d`.
  It contains the manifest, native action/motion/camera graphs, collections,
  physical skeleton, OnBoard/OffBoard banks, input configuration, gesture
  patterns and camera shakes. No Skate 3 world, character GLB or texture is used.
  Do not publish this asset pack to a remote repository.
- `SKATE3_EMBEDDED_PACK` supplies the local pack to `include_bytes!` at build
  time. It is not a runtime configuration or user installation requirement.

## Implemented boundaries

- Independent in-game selectors and strict persistent `gameplay.json`.
  The first test build offers full Skate 3 player simulation or stock Skate 2.
  Hybrid and Skate 2 tuning overlays are not selectable until implemented;
  the saved schema still represents the planned combinations.
- The embedded library invokes original input conversion, graphs and
  `physics::frame::advance`; no Bevy App, window, updater or OS polling starts.
- `skate-data::asset_io` validates immutable SK3PACK1 paths, sizes, SHA-256,
  duplicates, traversal and trailing bytes. The reserved virtual root reads
  embedded slices directly. Missing entries error without disk fallback.
- Native Easy/Normal/Hardcore/Motorized profiles; Easy Motorized derives from
  actual Easy collections plus original motor fields without editing assets.
  The temporary Normal-based compatibility aliases were removed following the
  user's stock-default clarification. Unsupported combinations are excluded
  from selectors and explicitly refused by the source-runtime boundary.
- Native scene interface accepts original triangles/features, packed surfaces,
  rails, moving SolidBody snapshots, authored grab bindings and drained one-shot
  solver reactions. Host identity is supplied, never guessed from mesh numbers.
- Skate 2 resident-ground export preserves arena/slot ownership, complete KD
  order and duplicates, and features from its native GP builder. It bypasses
  the trajectory result cap. The pure Skate 3 BoardWorld adapter preserves
  source identity, packed surfaces and feature flags/cosines across streaming.
- Occurrence-qualified external player ownership excludes those bodies from
  native integration, Joint/Drive preparation and skater collision publication.
- Raw controller capture observes Skate 2's existing hardware poll before
  conversion/remapping, without a second poll. The native input adapter can
  collect host packets separately from fixed-tick publication and reports
  controller availability through the original cache.
- `Skate3Player` constructs against actual Skate 2 board/skeleton allocations,
  native resident geometry and the pack inside its executable. It retains
  source owners, validates physical bone names before taking ownership, uses
  each host Part's actual mass frame, and publishes native board part poses.
- Pose retargeting uses names, the actual target hierarchy and inverse bind.
  Unmapped visual joints retain their authored local bind under their posed
  parent. The palette/material/cloth publication boundary is implemented and
  being tested against real Skate 2 data.
- `skate2-pc` enables the optional `skate3-runtime` library dependency. Cargo
  checking this combined graph passed. A captured Bevy-internal package patch
  excludes its unused audio backend to avoid native ALSA/CPAL version conflicts
  with Skate 2 audio. No source physics calculations were changed for that fix.

## Preview scope and remaining work

The full-mode selector, raw input, exclusion of local Skate 2 Actor/physical
stages, source player stepping, pose publication, observer state and camera
transport are connected and have passed the real-ROM queued-session corpus.
That corpus also covers walking, dismount, marker reset, live difficulty reload
and native Skate 2 rail import. The corrected populated main-bank fixture
also passed with three actual original DMOs and nonempty body export. A
rendered 180-frame preview and ordinary main-menu launch passed. The final
stock-default selection change is rebuilding before the package is published.

Remaining work:

- Preserve the tested single-solver schedule as the world population is added.
  Native Skate 2 stage completion is absent for the alternate player rather
  than fabricated from unrelated values.
- Scoring display, career observers and source-pose replay require integration.
  Source Skate 3 trick recognition/accounting already runs in its native tick;
  the Skate 2 HUD does not yet consume its publication.
- Validate the new actual NPC/car/prop body export and reaction path. Principal
  inertia, original primitive/mesh geometry, source ownership and occurrence
  IDs are retained. Skate 2's actual serialized assembly producer expands
  mapped arrays to the published leaves before export; unexpected nonleaf
  aggregate records are refused. Scripted contact events remain unconnected.
- Preserve authored grind/grab identities during streaming and connect moving
  object bindings. The current supplied grab registry binds static meshes.
- Complete native-pose replay submission; stale Skate 2 poses are deliberately
  not recorded as alternate-player frames.
- Add the requested native Skate 3 difficulty/motor behavior inside Skate 2
  simulation. Keep Skate 2 Default stock, as the user explicitly clarified.
- Implement hybrid handoff after full-mode ownership is complete.
- Finish the PC release and visual test, then package it. The user explicitly
  authorized launching the game on 2026-10-19 after originally prohibiting it.
  A preliminary release was launched on 2026-10-20 with a separate copy of the
  existing save. Its bounded diagnostic route overflowed the default Windows
  main-thread stack before the alternate player loaded. The PC link now gives
  that route the same 16 MiB reserve as the production simulation worker.
  The rebuilt executable passed a subsequent rendered test. The newest
  stock-default selector change still needs its final release check.

## Validation

Passed:

- Three gameplay selection/preference unit tests, including the strict current
  schema and invalid/unknown/duplicate fields.
  A fourth test now covers the first-build capability limits in the selectors.
- Both real main/pause menu integration tests across all 21 saved combinations.
- Three immutable pack-reader integrity/path/mount tests.
- Executable-embedded native-runtime test, with SKATE3_ASSET_ROOT absent: 60
  native ticks, raw Y, 90 further ticks reaching offboard, Easy Motorized reload
  and another tick. No asset-reader disk reads or extracted folder.
- Three earlier native reference tests: missing scene rejection, native ticks/
  profiles/poses/body reactions and raw Y offboard transition in a fresh owner.
- Occurrence-qualified external ownership test, including transactional foreign
  handle rejection and stale slot reuse.
- Complete native-GP static ground export feature test.
- Real Skate 2 terrain unload/reload corpus and native Skate 3 line query over
  that complete exported geometry.
- Combined native-library dependency `cargo check` (before subsequent new pose
  and input-boundary changes; those are under current test compilation).
- Two pose retargeting tests, including reordered source bones, authored bind
  inheritance, actual GPU row reconstruction and invalid hierarchy rejection.
- Initial combined real-ROM player/palette corpus passed both with 64 MiB and
  the actual PC simulation worker's 16 MiB stack. It proved mirror transforms,
  native ticks, palette publication and zero external Skate 3 asset reads.
- Original local Skate 3 checkout reverified against all 1,559 captured hashes:
  no changed or missing original files.

Subsequent checks and unresolved results:

- Extending that corpus to raw-Y dismount exposed an invalid fixture: the
  Waterfront player spawn was paired with the Rob's Park initial collection.
  The player fell without ground support; this is not a passed offboard test.
- Moving that partial-session fixture between collections reached a native
  waypoint deregistration refusal. The focused player test now uses the same
  authored-location-to-XSM initial collection lookup as the PC frontend and
  explicitly checks that original ground exists below the spawn.
- The selected-frame test passed with real captured pad polling, command
  admission, alternate stage exclusion, pose/PhysOut/camera publication and Y.
- The extended combined run passed all six selected tests: two pose tests,
  direct real-ROM player/rail/rig, queued real-ROM walking/dismount/camera/reset/
  difficulty, and two world-body identity/reaction tests. Runtime: 95.43 s.
  Teleport acknowledgements carry the actual Actor-requested matrix after the
  source engine consumes its original manual reset. No external Skate 3 asset
  reads occurred. The native 16 MiB worker stack is used.
- Rebuilding the 18-file payload with the portable Python tool reproduced the
  exact pack hash above. The build script now accepts an ignored local path
  pointer and reports missing build input explicitly.
- Three host body identity/reaction tests passed, including exact-zero source
  response rows preserving an untouched frozen body's state. Nonzero source
  corrections still awaken their actual bound body, and reused slots reject
  stale occurrences before any write.

## Regressions found with the copied skater save

The original no-save fixture did not instantiate the saved skater's cloth
effect. Testing with a copy of the actual save exposed non-finite skin/cloth
matrices: source XYZ frames have non-projective fourth lanes (root W=0), but
the target uses full 4x4 inverses. The pose boundary now supplies homogeneous
geometric lanes solely for palette/cloth math. Physical publications retain
the original lanes. A zero-W regression test passed, and both real-ROM direct
and queued saved-skater tests passed after the fix. The queued test checks all
actual skin matrices for finite values and exercises all seven full-mode
profile selections, walking, dismount and marker reset.

The main bank test loaded 93 models from 125 arenas but published no external
bodies at either Waterfront or Rob's Park. All census enable/ghost/network/
debug gates were checked against the live owners. Two observer defects were
found: skipped Actor stages left its external-reset bit pending, and the
bridge sent the animation-local centre to world census consumers. The reset
is now acknowledged only after native completion and only for its matching
Actor generation (a stale-generation rejection test passed). World centre now
comes from the original completed Physical.reckoning.vector_64, also used by
the source centre filter. The populated-session result after these changes
was still zero bodies: the combined run passed 9 tests and failed the populated
world assertion (178.21 s). Both world-space centre checks and the completed
generation-qualified reset acknowledgement passed. Do not report NPC/car/prop
interaction validation from a zero-body fixture or bank registration alone.

The next observer audit found unfilled physical foot positions. Host spawn
capsules read physical head/foot origins, not the final rendered bones. The
embedded boundary now exports all 24 original named physical parts, their
solved COM/rates, and the original inverse volume frame. The host publishes
head/hips/feet origins and axes from these solved parts. Mirror bodies also
use these physical bones and each part's own rates; rigid velocity is
transported to the host body's different authored COM with omega cross offset.
The saved-character corpus now checks each published part against its actual
source part and verifies world proximity. Its first rerun caught a new binding
mistake: the physical head volume is NECK1, not the visual Head bone, and the
host foot observers use LEFTTOEBASE/RIGHTTOEBASE volumes (15/19), rather than
LEFTFOOT/RIGHTFOOT (16/20). The authored 24-entry PHYS_TPOSE table was checked
directly; bindings are now named accordingly. That correction is under rerun.
The failed candidate was not launched or packaged for the user.

Five optimized PC builds completed during integration. The first real launch
exposed the diagnostic entry point's 2 MiB main stack limit; the executable
now reserves 16 MiB, matching its simulation worker. The subsequent real GPU
run completed and presented 180 frames with the copied saved skater, raw-Y
dismount and native camera. Visual inspection showed the saved clothing and
board rendered in the Waterfront world without a collapsed/exploded pose.
That was a preliminary build before the latest physical-part changes. The
next optimized build and real-ROM tests are running; no final test package
has been published.

## Rendered preview candidate, 2026-10-20

The corrected release build completed in 3m42s. Its executable SHA-256 is
`9544bbfaf951714749bbe24325700fd76d1ab3abb12f2131790b20682f9b1d0c`.
That exact executable completed/presented 180 real GPU frames, including the
raw-Y transition, with the copied saved skater. Window inspection confirmed
the saved skater/clothing and held board in the Waterfront scene. The world
schedule spawned 6 DMOs and 5 pedestrians by update 120 (97 resident locators),
without a frame error. This establishes population, not complete contact,
grab-action or scripted-reaction coverage. The normal no-argument launch also
reached the original main menu. Both diagnostic windows were closed afterward.

The old partial headless fixture had only 48 locators, the nearest 73.7 m
away, and no world bodies. All its other nine tests passed after fixing the
authored NECK1/toe-base bindings. The populated fixture now uses a real Actor
teleport to supported ground beside an actual enabled, model-backed locator
in its streamed tile; its original census and solve are still responsible for
creating all bodies. No synthetic prop, guessed shape or forced runtime spawn
was added. The combined rerun with that fixture is pending.

First source-runtime initialization still produces a cold-load pause: the
bounded 180-tick rendered run took 15.36 s including that initialization and
retained its host-clock backlog. This is not a measured steady-state frame
rate or a performance-completion claim.

Current test: the same queued session with Skate 2's original main DMO bank
registered and its ordinary LivingWorld schedule enabled. Consult work logs
for its outcome; compilation alone is not a passing runtime test.

The broad core run had 5,605 passes, 11 failures, 129 ignored. One failure was a
new test's incorrect expected surface override, corrected and retested passing.
The other ten are in untouched subsystems; no clean-baseline comparison has
established their origin. Full workspace formatting/testing/clippy gates have
not passed. Formatting reports differences in untouched files, and baseline
code emits hundreds of warnings. Preserve retained original line endings.

Confidence: **high** for invoking the captured production schedule and tested
source geometry/ownership boundaries; **hypothesis/unverified** for unfinished
live cross-engine integration. No playable-completion claim is made.

## Native character integration correction, 2026-10-20

The user rejected the embedded-player prototype as the final architecture.
The requirement is source integration in Skate 2's player systems, retaining
the Skate 2 character and gestures. The separate `EmbeddedRuntime`, duplicate
physical state and body mirroring remain unfinished migration work. Compiling
those into the EXE does not by itself satisfy this requirement.

The normal candidate-8 run exited after 12,441 completed frames with
`return cloth game header absent` in the Visual stage. This is an observed
failure, not a guessed physics panic. The live clothing restore now has its
own completed live header instead of relying on replay-pose admission.
The previously frozen diagnostic had completed its bounded run and retained
its final image because of `--live-hold`; that window was closed.

The queued scheduler now runs the actual Skate 2 action, motion, body, hand,
face, synchronization and animation-to-physics publication jobs in Skate 3
mode. The first character-job corpus passed (52.41 s). Subsequent work removes
the custom whole-rig Skate 3 palette publisher. The actual attached player
Parts now publish their 24 physical bones into Skate 2's `AnimOutPhysIn`
buffers using the native physical-parent table and reverse local-pose loop.
The normal Skate 2 presentation writer, decode/correction jobs, replay record,
materials and clothing consume those same buffers. Nonphysical authored
joints and the hand/face allocations are retained.

The first native-presentation run completed its initial frame but failed an
old test's bitwise root equality: native quaternion decoding differs by about
3e-8 and publishes geometric translation W=1 rather than source storage W=0.
The test now checks geometric XYZ with a 1e-5 bound and requires a real native
presentation/replay record. The combined native-presentation run passed all
seven cases (149.51 s), including forced clothing restoration after a fresh
clothing step. The subsequent native movement publication run also passed
all seven cases (148.42 s). No new game EXE or user package contains these
changes yet; optimized app test builds are separate from a delivered game.

The user confirmed Skate 3 movement animations on the Skate 2 character,
with Skate 2 gestures retained. Skate 2 Default is offered only in Skate 2
mode. The current movement path binds actual Skate 3 completed world bones
by name into the native Skate 2 AnimOutPhysIn buffers and preserves authored
Skate 2 character-specific joints under their animated parents. This replaces
the custom GPU palette publisher. It is not completion of physical-owner
migration or gesture blending before the physical input boundary.

The user also requested an Edit Skater selector for Skate 2/Skate 3 gesture
catalogs, remembering each catalog's Up/Down/Left/Right choices separately.
The native CACRecord retains the Skate 2 bank; SKATER.P's existing SK82_PC
extension now stores a versioned GestureSets leaf for the selected catalog
and independent Skate 3 bank. The selector uses the existing sixth Style
menu row and preserves the original console option ordinals. Skate 3 labels
follow the captured CharacterGesture catalog, not the flick-trick catalog.

Two codec tests passed, including malformed/future-version preservation.
The copied original-save in-memory test passed (1.10 s), retaining original
CAC and unknown chunks. The real-ROM Edit Skater/store test passed (4.84 s),
checking both catalogs, independent cycling, reopening, and merging stale
save writers without dropping the new leaf. A full PC cargo check passed.
The authored APT row test first could not compile because seven existing app
test fixtures had stale struct initializers/matches. Those test-only fixtures
are updated to their current production types. The next run reached assets
but found the existing fixture mounted only the four boot archives, while
Assets::load now reads credits from a later archive. Mounting the actual full
archive table corrected the fixture. The authored APT test then passed
(0.25 s), including repeated Skate 2/Skate 3 labels and visible arrows in the
existing sixth Style control. This was headless movie evaluation, not a
gameplay animation preview.
Runtime Skate 3 CAS previews and selected-bank gameplay playback/blending
are not yet connected. Saving and rendering menu rows do not establish that
these animations work in gameplay.

## Automatic test journal, 2026-10-20

The user explicitly requested detailed logs for testing. The ordinary quiet
game and bounded live runs now install a per-run rolling JSONL journal under
the save directory's test_logs folder. Public build/source fingerprints,
compiler/target/profile, save and executable paths are recorded; hashing the
actual EXE runs on a separate thread. The private network build secret is
not included. See docs/TEST_LOGGING.md for files, fields, limits and collection.

Instrumentation covers original raw pad polls, simulation stage boundaries,
native animation/physics publications, settings/frontend/editor/replay state,
native presentation, source-3 completed physics/ground/collision state, gesture
selection/assignments and verified save commit outcomes. The existing panic
hook and returned live errors save the recent journal independently from
console quiet mode. A watchdog observes active-stage breadcrumbs without
acquiring game/session locks, and saves history for work stalled for 10 s.
This can indicate a long load as well as a hang; no deadlock proof is inferred.

Six standalone journal tests passed (11.28 s): JSON string escaping, rolling
file retention, durable error/history and active-stage watchdog reporting,
nonblocking saturated queue behavior, memory record/byte bounds, and ordered
shutdown flushing after a full queue drains. A
full-PC check passed before the additional physics breadcrumbs; the first
optimized focused app test compile then caught a test-only world-body-count
accessor in the new diagnostics. The instrumentation now reads the retained
field directly. The optimized app test rerun passed and wrote its actual
build manifest and EXE hash. Collecting and summarizing that report also
passed; collection excludes an unrelated synthetic save fixture.

The headless populated-world run with real journal recording passed (59.59 s):
210 source-3 physics records, 840 raw pad records, 1,050 native presentation
stage records, and 840 physical schedule boundaries. Every JSON record parsed;
retained heartbeat counters reported zero dropped records, and session_end
was durable. The watchdog also caught the known first-time initialization as
10.4 s in the enclosing actor/animation stage. The run subsequently completed,
confirming why stalled-stage reports explicitly allow long loading work.
The original SKATER.P hash and all 1,559 captured original source files remain
unchanged. No gameplay window was opened for these checks. The final PC cargo check after the shutdown-queue improvement passed
(32.97 s), with the existing app warnings retained. No final-build claim is made.


## Native source and physical-owner migration, 2026-10-20

This supersedes the earlier EmbeddedRuntime prototype architecture above.
The captured player/controller/animation/physics calculations now compile
inside skate2-core under physics/modes/skate3. The real Skate 2
PhysicalPlayerAssemblyOwner owns the selected Skate3Player state. There is
no skate-game dependency, Bevy App, second game/window, device polling loop,
or EmbeddedRuntime in the executable dependency graph. Bevy math/transform/
logging utilities remain dependencies of the ported calculations.
SOURCE_PORT.json records the captured source hashes. Original standalone
host fixtures are excluded explicitly; native Skate 2 integration tests and
the captured pure-calculation tests are separate validation suites.

The selected solver's completed bodies now publish atomic read-only views
for native world, character and replay consumers. Publication validates the
complete occurrence set before replacing the prior completed frame. It does
not rewrite the dormant original Skate 2 physical body slots. Native 2
integration/drive/joint rows remain disabled for the selected mode. The
physical-owner test proves unchanged original slot state, current pose/rate
readers, rejection of incomplete frames, stale/reused occurrence rejection,
and clean view removal when returning to stock mode.

Validation before the release build:

- Native body ownership/publication proof: passed, 0.00 s.
- Populated original-ROM world with 210 selected frames and detailed logging:
  passed, 53.77 s; dismount/remount, teleport, 1,940 rail primitives, original
  world body snapshots, normal character/replay/clothing publication.
- All eight selected source/host integration cases in one process: passed,
  166.60 s. This includes actual ROM boot, embedded asset reads, live queued
  inputs/camera, the populated world, reaction identity and climbing approach.
- The first combined run with per-case logger installation failed two cases
  because the process-global journal was already installed, not because of
  simulation. Repeating the combined suite without per-case installation
  passed; detailed logging was tested separately in the populated run.
- The populated journal contained 210 physics records, 840 raw pad records,
  1,050 presentation stages, valid JSON throughout and zero dropped records.
  Its watchdog recorded a 10.4-second cold asset load and recovered normally.
  The native asset/graph load now has its own stage label in the game build.
- Cargo's normal PC dependency tree contains no skate-game, bevy application,
  bevy_render, bevy_winit or bevy_internal package.

The user requested a physics playtest before gestures on 2026-10-20.
The unfinished movement-base/gesture-before-physics work was preserved outside
the repository and removed from the candidate's execution path. Selected
catalog gameplay playback/CAS previews remain unfinished. The current native
3 movement output enters the normal native 2 character publication path;
this does not claim completion of the deferred gesture composition.

The first playtest exposes only Skate 3 with Easy, Normal, Hardcore,
Motorized and Easy Motorized, plus unchanged Skate 2 Default in Skate 2 mode.
Hybrid handoff, Skate 2 tuning overlays and Skate 2 Motorized remain unavailable.
NPC/car/object solid collision reactions are connected and component-tested;
semantic object-grab records are not yet bound to Skate 2 assets. Full
interaction parity and long-session stability still require dedicated tests.
No full-mod completion claim is made by a physics playtest package.


### First native game launch and inactive walking trajectory

The release-10 game compiled successfully (7m08s). Its bounded offboard-jump
launch failed after 173 completed frames with
Actor/Phase1/Locomotion/FutureNotReady(1). The journal preserved all 173
physical outputs. Native S2 animation had entered its walking-trajectory path
while the S2 physical player solver was inactive; its second-step future was
therefore not a valid movement dependency in S3 mode. The previous corpus
used a partial stick and ended at 210 frames; it did not cover this full-stick
case.

Movement authority is now explicit on the actual Actor animation owner.
While the native S3 physical owner is selected, S2 IsInLocomotion and
WasInLocomotion consumers do not schedule/read S2 walking trajectories.
Normal character, body, hands, face and pose-publication jobs remain active.
Changing authority clears the two old S2 locomotion-history flags, so stock
mode cannot inherit a pending alternate-mode trajectory. S3's own controller,
step queries, timing and physics calculations are unchanged. No fabricated
future readiness, ignored error or altered walking tuning is introduced.
The real-data corpus now runs 360 frames with a full left-stick command and
a raw offboard jump; the corrected game must also repeat the exact bounded
launch before delivery. Validation is pending at this entry's creation.


### Corrected native physics playtest delivery

Release 11 compiled successfully in 5m41s. All eight expanded integration
cases passed (170.96 s), now including full-stick running and offboard jumping
through 360 queued frames. The exact failing in-game sequence was repeated
with the new EXE and completed 1,200 ticks normally (20 simulated seconds).
The complete journal retained 1,200 physics outputs, 4,800 raw pad samples and
6,000 presentation stages, with zero dropped records, nonfinite pose reports,
panics or fatal errors. States 100, 103, 500 and 501 were reached. The original
world had 57 moving-body snapshots and 7,394 source rail primitives by the end.
The expected cold asset-load stall remained observable; the run recovered
and exited normally. No --live-hold was used.

A separate stock Skate 2 Default offboard run also completed 150 ticks
normally. Its character capture was compared with the native S3 capture:
the same original Skate 2 recipe/clothing/board and normal presentation were
retained, with the selected mode's own movement pose and camera.
The actual game's diagnostic log collector ZIP was also verified.

The first native physics playtest was packaged locally as
outputs/Skate2-Native-Physics-Test-1, with a separate copied SKATER.P,
Skate 3/Normal preselected, automatic logs, collector and matching PDB.
EXE SHA256: 5b796d61f347006b3ff452dccce8ba60559cf6c1eb5082c3601bba57c778633a
EXE size: 80,091,136 bytes. The build's runtime-computed hash and the packaged
EXE hash matched. Original save hash and all 1,559 captured original source
files remained unchanged. This is the first physics playtest, with the
deferred features and interaction limitations described above, not a claim
of complete mod parity or a completed gesture implementation.


### Upstream sync and revised Mods scope (2026-10-20)

The user superseded the three-mode selector with Options > Mods > Skate 3
Physics: All Skate 3 Physics, Darkcatch, Footplants, Offboard. The six
difficulties are Skate 2, Easy, Normal, Hardcore, Motorized, Easy Motorized.
Every difficulty must work with either physics; the earlier stock-only
restriction on Skate 2 Default is superseded. Gestures remain last.

Git and the official GitHub repositories were checked. The S2 v0.2.2 changes
(21e5226a1acefb6af71b943f8ea30801f6adf35e) were three-way merged into the
working tree, retaining local native changes. The updated upstream had
rewritten history; HEAD was not reset. New party/NIS/scene/career and
online/lobby-chat changes are retained. The merge originals, binary diff and
untracked source were backed up before editing.

S3 dependencies and ported production algorithms were updated to
2e16697797c8358d16f747a8259354ff2921041a. This includes the new broadphase,
water/contact/friction/bail fixes, low-camera selection and the source's
50-iteration solver setting. The user's local engine checkout remains
unchanged. Source host input adapters retain S2-owned normalized Xbox input;
standalone SDL/UI/render/audio hosting is excluded. S2 owns audio/assets.
UPSTREAM-SYNC.json and SOURCE_PORT.json distinguish the original captured
hashes from the latest upstream and actual in-core port hashes.

The first updated-source core integration suite passed all eight cases in
71.70s. This precedes the subsequent gameplay-consumer changes and is not a
validation claim for them. The PC compiled successfully after both upstream
merges, party index corrections, Mods menu and gameplay facts wiring.

Offline wall time is discarded while unfocused and capped to the existing
four-tick redraw budget after stalls. Completed frontend ticks reset host
wall time. Online cadence and native simulation periods are unchanged.
Four clock tests passed (long alt-tab, long stall, fractional jitter, online).
This removes the measured backlog mechanism; live alt-tab/resume validation
is still required.

The actual S2 profile Camera Angle selects the source S3 graph type 0/1.
The real-data corpus now checks High/Low/High stock shot branches. Party
integration no longer assumes index 0 for solve, pose, observers, camera or
result-array alignment. Ownership changes retire the prior source player;
spectated non-player AI retains AI control. Multi-controller/party and
story validation remain required.

Preferences now store explicit feature toggles and six difficulties. The
legacy first-test JSON migrates without changing its selected physics.
The master enables every feature; disabling an individual disables the
master while retaining the other selected replacements. Enabling all
individual features does not implicitly select the complete solver.
Nine standalone preference/config checks passed, including every feature
combination, every difficulty, migration and malformed/ambiguous input.
The original S2 options APT presents the Mods pages; a navigation/save
boundary test has been added. The independent replacement algorithms and
cross-solver difficulty overlays remain unfinished, and their runtime
errors have not been hidden by aliasing the complete solver.

Completed source facts now publish authored trick names/flags, stance,
physical feet, air/jump/footplant observations, solved board contacts,
audio surfaces/patterns, skeleton contact regions and per-bone reports to
the original S2 PhysOut. Native numeric scorable IDs are resolved by name
against S2's catalog. The existing S2 audio conditioner, trick history,
scoring module, HUD and statistics consumers now run from that publication.
This is pending the current real-data suite, including missing-field and
trick-name/score assertions. It does not claim all sounds, new S3-only
scorables, contexts, object grabs or career challenges are complete.

No updated user playtest package was made; the first delivered package is
preserved for comparison. Six obsolete preview executables/images (about
345 MB) and their three empty preview folders were permanently removed
after checking their paths and retaining the current package, sources,
merge evidence and original saves. No game was left running in this phase.


### Native gameplay facts and Party Play follow-up (2026-10-20)

The revised real-data suite passed 14/14 in 86.66 seconds
(`s2-native-gameplay-corpus10.log`). It verifies one S3 step for each admitted
S2 tick, original S2 body ownership/retarget/presentation/cloth, High/Low/High
camera selection, original-road contacts/audio surfaces, raw flick -> named
trick -> S2 HUD points, no repeated completed trick event, six difficulty
changes, manual offboard reset acknowledgement, walking/jumping and actual
populated-world collision/bail/audio. This is integration evidence, not a
complete story or every-trick reference trace.

Two concrete publication errors were corrected: SceneTriangle stores the
raw packed RenderWare surface (not the GP user tag with the surface shifted
16 bits), and Scoring2 event arrays reset before each native publication,
as in the original Update5 lifecycle. Source camera simulation-rate requests
now use the same S2 timer as the rest of the world. The original 3 clock
tests and 4 host clock tests cover timer expiry/order and suspended-wall-time
charging; live alt-tab and menu-resume tests are still pending.

Party coverage now creates a real second skater, switches the UI input and
SkaterManager local index 0 -> 1 -> 0 -> 1, checks physical ownership and the
selected presentation, and exercises handoff of an already-walking actor.
The fixture initially supplied the wrong PlayerLoader; that test setup was
fixed. It then found a real LockedForWrite dependency: S3 scoring queried
the other skater before that skater's Update5. Native gameplay consumers now
run after all native S2 post-solve publications and before their conditioners.
The collision export also follows the host's admitted skater-pair policy,
so Party Play's pass-through players are not exported as solid obstacles.
The latest suite passed 14/15; the remaining assertion checked the explicit
702 teleport output as if it were the following BipedGround output. It now
checks the next state and records the source actor's pre-handoff state.
This revised party assertion has not yet passed.

Immutable compiled S3 graphs, animation banks and decoded-pose data are now
shared across controller reconstruction. Mutable graph, camera, physics,
input and scoring state remain private to each player. Re-mounting the exact
same immutable embedded byte slice reuses its previously validated bundle;
different mappings still undergo all checksums and identity checks. This
removes redundant work but does not establish live frame-time improvement.

The S3-only scoring catalog extends existing S2 IDs by name, translates
consumed/base/variant references by name, retains S2's shared trick records,
and grows the repeat/sequence histograms for new IDs. New tricks use original
S3 points, timing and labels in the original S2 scoring/HUD path. A real-data
banking test is added and pending. The metadata/label integration test passed
in the last suite. The independent label probe decoded 299 of 300 authored
S3 records; the sole missing S3 label is the shared, unused slidelip entry,
which retains its existing S2 record. Additional S3-only labels are required.

The language histogram/category are embedded too, with no new runtime game
installation requirement. The next pack has 20 files, 26,293,405 bytes and
SHA-256 e22c488e959bef8f508783a8518207fef538ac37148451267547ecda65dfc4b3.
Original assets and generated packs remain outside this source repository.
The original-options movie test found an upstream boot regression: credits
were read from the not-yet-mounted Load group. Assets::load now reads the
real miscload archive when necessary; the original-movie test is pending.

Still unfinished: independent Offboard/Darkcatch/Footplants implementations,
all difficulty overlays on native S2, full interaction/HoM/audio semantic
coverage, career/story and controller-specific party probes, live performance
and resume validation, and actual gesture composition/CAS playback. No new
user playtest package has been produced and no game window was left running.


### Independent Offboard ownership and native return (2026-10-20)

`work/s2-independent-offboard-corpus21.log`: 17 passed, 0 failed, 156.50s.
The original S2 road and populated-world corpora still pass. The road case now
also exercises two independent Offboard dismount/walk/remount cycles, switches
All S3 back to independent Offboard during ordinary riding, and verifies that
ordinary riding returns to the native S2 solver. Two distinct controllers and
Party Play owner changes run through the actual queued simulation.

A real dismount edge is retained through source constructor reset and delivered
once to its input graph, without an extra physics tick. Actual completed S3
category/state controls the end of remount ownership; a timer does not substitute
for state completion. The S2 physical owner remains the same allocation.

Native return publishes the completed mass-frame poses once into the original
S2 bodies, clears external ownership, and resumes S2 integration. The matching
Actor reset generation carries per-body linear/angular rates. Its real S2 reset
applies these once after body resets and before Ground.Enter samples speed.
Velocity transport uses v + omega cross (new COM - old COM). The test observes
all seven board bodies before the native solver and checks the transported
linear rates. A newer Actor reset cancels the pending transfer.

The first continuity test caught a 13.77961m snap despite returning body poses:
the dormant Skeleton root/prediction still described its old location. The
one-time return now accepts the completed root, inverse, board frame and invalidates
that old prediction. Both tested remounts and the All-to-independent transition
report a 0m first-frame root delta. No per-frame native body mirroring was added.

The real body-pool boundary test rejects nonfinite and foreign rates before any
stores; an incomplete completed-pose set refuses to change ownership. It also
checks the one-time pose/rate return and rigid velocity transport at a changed COM.
These checks do not yet cover every air-entry, refused-dismount, streaming or
career-driven reset case. Those remain required before a final user build.

Earlier supporting results now supersede the pending items in the prior entry:
original Options movie/Mods rendering passes (s2-mods-original-movie-test2.log);
52 original conditioner tests pass with 1 ignored; 23 scoring tests pass; three
embedded AssetIO tests pass; the featureless core check passes. Existing baseline
warnings remain. No GUI game was launched during this work.

Native S2 difficulty providers are being connected to the embedded S3 authored
profiles with per-player contexts, retaining the S2 solver and original Vault.
Jump limits, source speed-dependent push limits, shared pumping values, automatic
spin limit, ground wipeout acceleration, grind lock distance and hold-RB motor
settings have been wired. The combined native S2 profile corpus is still under
validation (work/s2-native-difficulties-corpus24.log); this is not a claim that all
S3-only difficulty assists or the independent Darkcatch/Footplants are finished.

Cleanup permanently removed three obsolete standalone test executables and two
superseded probe helpers from this task's work directory after explicit path
verification. Current test logs, source, upstream merge backups, embedded packs
and the previous user playtest package were retained for recovery and comparison.


### Six native S2 difficulties and reset/stance follow-up (2026-10-20)

`work/s2-native-difficulties-corpus24.log`: 17 passed, 0 failed, 221.82s.
`work/s2-native-character-reset-corpus25.log`: 18 passed, 0 failed, 217.40s.
The real queued road case now contains 1,680 additional handoff/profile frames:
two Offboard cycles, All-to-independent riding, and six native S2 difficulty
segments. Every S2 segment asserts that no S3 solver was selected; Skate 2
Default asserts an absent tuning context. Real RB input measured motorized
speeds of 5.7779107 and 5.7665386 m/s, versus 0.0167..0.0212 m/s for the four
nonmotorized profiles, on the same original flat road. Native trick scoring
also receives the scripted original-S2 flick inputs.

Profiles use immutable embedded S3 values and per-human physical-owner contexts.
Original S2 Vault/global cvars and NPC tuning are not modified. The S2 speed
model's original motor branch consumes its actual GrabWorld/RB processed flag;
its cheat enable/top-speed reads are scoped to the selected profile. Stock
Skate 2 retains the original captures. The S3 speed-dependent push-cap port is
also compared bit-for-bit with the actual source push kernel at finite positive,
zero and negative speeds for all five S3-derived profiles. S3-only difficulty
assists without corresponding S2 consumers still need a behavior audit.

A newer Actor reset with the same matrix now has an actual corpus case: its
generation supersedes the pending return, and old momentum is not restored.
The existing velocity-transport assertions still run for the other real remounts.
Regular stance is selected on the real S2 Actor before source acquisition and
asserted in the completed source movement output. S2 RIGHT0/LEFT1 maps to S3's
checkpoint enum; S3 natural GOOFY1/REGULAR0 uses its own customization method.

The source constructor now accepts the actual host collision scene directly.
Previously it built a collision-map world/BVH and immediately replaced it with
the same scene's native-feature world. The intermediate map/world is removed;
source physical settings, spawn numerics, actual terrain, rails and solver
iterations are retained. This avoids redundant construction; it is not a claim
that all live lag spikes are fixed. Scene replacement continues to preserve the
existing host grind provider (inspection confirmed it does not reset that data).

Eleven original speed-model tests pass (s2-native-speed-provider-regression-tests.log).
The featureless core check passes again (s2-stock-configuration-check-after-difficulties.log,
17.25s, 439 existing baseline warnings). The original SKATER.P hash remains
AD1593DDBC5CC0F9E22A2DC8CC9F39B9B7565FC36B8EEF0756C46C50F72B9C44.
There is still no new user release build or GUI game session.


### Native scripted Actor policy, LivingWorld pushes and S2 0.2.3 (2026-10-20)

Official heads were checked again. Skate 3 remains
2e16697797c8358d16f747a8259354ff2921041a. Skate 2 advanced to
f72f2c1302bf9cf176f842ace7b441d1824feba4 (0.2.3). Its five changed files were
three-way merged from the previous 21e5226 base without resetting the working
tree. `work/upstream2-0.2.3-merge/REPORT.json` records the conflict-free merge
and retains each outgoing/base/upstream file. The upstream IsLoaded fix applies
character attributes during registration as well as after registration.

The original S2 Actor now supplies its actual scripted policies to the native
S3 player. Capture precedes GenerateActionGraphIntents825FDA28 (which consumes
the pending wipeout request) and the SetUpPhysics/Actor.Update one-shot clears.
S2 pin 04000000, pending wipeout 80000000 and prevent-manual-respawn 10000000
are mapped by meaning; they are not aliased onto S3's unrelated Actor1908
filter word. A pin filters the final S3 ActionGraph input after riding, grind
and offboard producers, retaining only the explicitly permitted D-pad emotes
and ForceBrake. S3's emotes use Start/Held pairs rather than S2's four names.

The first real scripted-pin test caught offboard inputs appended after the
initial filter: corpus33 passed 21 cases but failed the new pin case. Moving
the policy to the final action map fixes the actual bypass. Corpus34's equipment
assertion assumed constructor truck defaults; the real loaded skater uses its
saved value. The corrected probe compares the actual backend row and changes
it live, rather than claiming that the constructor value is its saved setting.

The actual S2 SetUpPhysics suffix supplies live truck tightness, wheel hardness,
force braking, prevent-manual-respawn and ignore-reset-button policy to S3.
Actor.Update825F10F0 still runs for the S2 character when S3 owns physics, so
audio/chase timers and one-shot requests retain their original lifecycle.

S2 Actor.LivingWorldWipeOut825ED130/ForceWipeOut825EDAD0 now supplies the actual
normalized push vector and one-shot flag to S3's existing Actor packet transport.
S3 AnimOut10496 maps to Processed2468 bit2 and vector10480 to vector1520;
ProcessAnimAttributes also publishes its native request flag. Wipeout entry
uses the original S3 push force and applies displacement to spine part12.
No fabricated force, duplicate command or repeated post-solve body write is used.

Corpus35 passed the 580-tick real-world scripted-policy scenario: pin, gesture
exception, unpin, one-shot reset lock, live equipment edits, pinned independent
Offboard admission, ordinary riding return, and exactly one actual LivingWorld
push followed by source ragdoll. The source vector matches the S2 publication
word for word. Embedded-source disk reads remain zero. Corpus32 separately
passed airborne independent dismount/remount; source offboard-air state501 was
observed before landing, followed by native S2 riding after remount.

Corpus36 passed all 22 native integration cases in 316.81s on S2 0.2.3,
including both darkcatch stances with inverted-board contact and named S2 HUD,
party ownership/controllers, repeated independent transitions, camera selection,
original audio/scoring/surfaces, populated world, return momentum and all six
difficulties on each solver. A featureless core check passed (17.21s), and the
PC target check passed (1m25s). Existing warning baselines remain.

After that broad run, the input-history audit also preserves S2's skipped
Derived.Fill while pinned without the gesture exception. Recognizers/physical
ticks retain their own schedules; the first unpinned edge is not consumed by a
blocked Fill. The focused journal run and controller-history test are recorded
below once complete. New change-only journal events identify scripted policy,
live equipment changes and actual LivingWorld pushes. This is evidence for
these contracts, not a claim that the whole story or every object action is
already verified. Independent Darkcatch/Footplants, object grabbing, gestures
and performance/live gameplay validation remain unfinished.


#### Scripted-policy journal and input-history verification

Corpus37 passed the same real 580-tick scenario with logging enabled and the
Derived-history fix (41.76s). The audited journal contains 8,104 valid JSON
records, policy transitions at source ticks60/100/130/160/161, the actual saved
equipment (.6/.7) and live edit (.25/.85), and exactly one skate3_world_impulse.
The run ends with session_end and reports no panic/nonfinite-animation record.
The journal is retained under work/actor-policy-test-logs; the machine-readable
audit is work/actor-policy-journal-audit.json. Four focused units also pass:
the new upstream IsLoaded registration boundary, authorized gesture filtering,
semantic one-shot mapping, and actual controller history through pin/unpin
and the gesture exception. No GUI game was opened for these headless checks.


### Native footplant routines and original-world baseline (2026-10-20)

The executable-embedded S3 player completes all four one-foot-air combinations
over the actual S2 streamed road: LT/RT grabs crossed with A/X free-foot inputs.
Corpus38 passed 1,200 admitted native ticks in 56.78s. Each combination observes
a physical footplant and the authored footplant scoring name; no separate S3
assets are read. Ollie takeoff, raw hardware samples, actual one-foot poses,
prediction, contact, IK, absorb/release and ordinary riding all execute through
the existing source graphs and physical path. This establishes the full-S3
baseline; it does not establish the independent Footplants toggle.

The original FootPlantManager computations now accept completed typed pose
observations and return foot IK/collision and ground-path outputs. They own no
second controller, body pool, window or simulation loop. Source math and authored
settings remain in the same manager. Candidate selection preserves toe15/19
priority and the velocity-prevision COM calculations. IK preserves its target
lock, blend and two-frame collision gates. Query consumption remains separate
from submission and next-tick result visibility. Actual deck position remains
an explicit input to the edge-visibility calculation. The extraction guard
caught that remaining full-owner dependency before the query edit was applied;
corpus40 therefore verifies only the IK extraction, and corpus41 verifies the
subsequently completed query/contact and ground-path extraction as well.

Ground start/path/release and the existing heavy-landing/too-fast requests
(6 and23) also remain the source algorithms. IK writes commute with the
manager-only stores moved into the calculation: those stores read no IK or
collision fields, and the native writes have no fallible side effects.

Corpus39 passed56.53s, corpus40 passed56.45s, and corpus41 passed56.72s. All29
sampled per-variant trace records, including manager state, timers, contacts,
foot bounds, action/motion states and final four-way results, match corpus38
exactly after each extraction. This is a bounded real-world regression proof,
not a claim of complete game/reference-machine parity. Native S2 manager,
predictor, IK, selector and state bindings for the independent toggle remain
the next work; selecting the full S3 solver for that checkbox is not the design.


### Independent Footplants on the S2 character and solver (2026-10-20)

The original S2 footplant path was exercised first: corpus42 passed15.92s,
corpus43 after ownership/reset/Fill integration passed16.03s, and corpus44
after the readonly KnownAir/selected-trajectory input adapter passed15.95s.
No S3 player or second solver was permitted. The adapter samples the actual
completed winning COM trajectory at KnownAir's trajectory index using S3's
sampling functions. Unknown native S2 W lanes remain unknown in their owner;
only new geometric inputs use a documented W convention, with XYZ driving the
source footplant geometry.

The real S2 authored Motion graph contains no FootPlanting marker. Corpus44
observed false at all five sampled stages; this was not treated as an encoder
failure. With the mod enabled, its real Footplant subtree supplies the semantic
request instead, using actual generated active ancestor IDs, including each
branch's child states. S3's authored prepare window(0.1..0.5s) is used only for
this replacement. BodyAdjustX/Z uses the actual S2 ActionGraph object-movement
intentions, following S3 OnBoardGeneral's OB_OBJECTMVX/Z copy rule. The OneFootAir
and prior-body-flipping gates are mapped by meaning, not by reusing S3 flag bits.
The 0.1s source gate preserves its <= comparison, including unordered behavior.

The independent replacement owns only a source FootPlantManager and query
state inside the actual S2 manager. Candidate/launch/contact, locked foot IK,
temporary collision gates, authored ground path, steering, release velocity,
and footplant-specific speed requests run source routines. Real S2 body/pose
observations feed them; returned targets go to the existing S2 Skeleton IK.
Queries use the actual owner-bound World predictor and resident grind geometry.
Only the preceding completed request is accepted; query submission cannot mark
a result ready. The resulting manager observations feed the existing S2 Fill,
physics state selection, native Skeleton update and trajectory launch. The
shared S2 collision-bail detector still runs; its S3 counterpart is a remaining
audit boundary. The S2 animation graph remains the partial replacement's host.

Corpus46 passed1,200 admitted ticks in49.77s: all LT/RT xA/X combinations observed
source contact/plant flags and toe candidates without acquiring an S3 player.
Corpus47 repeated after lifecycle hardening and passed49.66s. All four variants
also entered the actual S2 FootPlant601 state, released into actual native S2
air, published authored footplant names, and produced valid footplant labels
through the existing S2 scoring/HUD. No separate S3 asset reads occurred.
Compiler-only45 failed from an incorrect module path and was corrected before46;
it is not counted as a runtime validation.

Motion context writeback is retained even if optional signal publication fails;
constructor-only Motion updates with no published physical owner have no mod
manager to signal. Disabling the toggle retires prediction/manager ownership
while allowing an already-contacting plant to finish its authored release.
That toggle boundary and source steering still need dedicated live-corpus checks.
Change-only owner/contact/request/IK facts and explicit start/release diagnostics
are in the automatic journal; no per-tick footplant wall of text is added.

This is component/corpus evidence. It does not claim a final build, console
parity, full career coverage, independent Darkcatch, gestures or object grabbing.


### Source plant collision checks, live toggles and steering (2026-10-20)

The previously recorded shared-S2-bail boundary for independent FootPlant601
is now replaced with the S3 check. The audit found real differences: S2's
compliant-collision exception and squared-displacement comparison must not be
substituted for S3's length comparison, and S3 closing contact can append an
additional regional force request. A consumed-observation input now feeds the
same S3 function from the complete S3 wrapper or the actual S2 character.
Squash/pose-error, regional forces, contact count, WorldToAnimation and closing
velocity come from completed S2 skeleton/body observations. A noncontacting
board requires no invented closing velocity or transform. All thresholds come
from embedded original S3 data. Trigger order and duplicate force requests are
retained when writing the actual S2 wipeout ledger. Stock S2/Boneless behavior
continues through its previous branch; the Boneless/fastplant602 port is still
outstanding and is part of finishing the Footplants scope.

Existing source wipeout tests passed6/6 after extraction(corpus49). The new
boundary regression adds a real rotated closing contact and verifies the
ordered force, impact, force requests and native count3, plus no-contact
behavior. Together the seven source wipeout tests passed in corpus52.

Corpus50 compiled the native core and passed four original-road cases in
147.07s: unchanged native S2 baseline, complete-S3 four-variation baseline,
independent S3 FootPlant601 four-variation contact/release/names/HUD, and a new
600tick live-toggle/steering case. Switching off after actual601 entry defers
retirement until its source release; the manager, pending source request and
enabled flag then retire. Enabling at native KnownAir201 tick130 subsequently
performs/releases without acquiring a complete S3 player. Left-stick steering
reaches BodyAdjust and the release through the actual S2 ActionGraph. Both
variations retain valid authored trick/HUD labels. All29 complete-S3 sampled
trace records remain byte-identical to corpus38 after the collision extraction.
The prior stock/full-S3 regression48 also passed2 cases in72.20s and retained
all29 records. The PC cargo check51 passed33.30s with its18 existing warnings.

Journal corpus53 repeated the 600tick toggle/steering scenario and passed29.30s.
All7894 JSON records parse, with10 change-only footplant contact records, two
start and two release records, two ownership records and a normal session_end.
The steering release records BodyAdjust[0.35433072,0]. The original SKATER.P
hash remainsAD1593DDBC5CC0F9E22A2DC8CC9F39B9B7565FC36B8EEF0756C46C50F72B9C44.

The journal also exposes a cold-data hitch: first independent FootPlant manager
construction blocks physics_pre_solve for about2.9s in the unoptimized harness.
This needs preloading/caching at the game loading boundary, with release-build
timing measured, before claiming seamless activation. It must not be hidden
by loosening physics or timing. No GUI game, release or new user package was made.

The requested permanent cleanup of20 applied footplant edit scripts was
attempted after successful verification. Automatic approval review rejected
both the checked batch and a single verified literal Remove-Item command with
the reason blocked by policy. None of those scripts were deleted and no
alternate deletion mechanism was used. Sources, traces, saves and backups stay
intact. Prior successful cleanup remains separate historical evidence.


### Ground plant reference and moving cold assets out of simulation (2026-10-20)

The original S3 ground-grab Action/Motion XML supplies the boneless/fastplant
fixture: LT/RT held before the A/X push inputs, without an airborne ollie.
Corpus54 passed1,200 ticks56.82s. All four combinations entered actual602,
launched into actual air, and published the authored boneless/fastplant names.
Corpus55 repeated after immutable data caching and passed56.46s; all33 sampled
boneless trace records match54 byte-for-byte. These are the full-S3 baselines;
the independent602 binding is still outstanding. Source launch calculations
and begin now have a body-owner-independent interface, pending corpus58.

Only data from the immutable executable pack is shared. Per-owner custom
profiles clone that pristine Collections source before any override. External
asset-root reads retain their normal reload behavior. Difficulty, native
scoring and player profile readers use the shared embedded source. FootPlant
manager construction clones a private pristine recipe; only read-only wipeout
settings use Arc. The ownership regression in corpus57 passed3.15s: changing
one manager's contact/request/toe and pending query cannot affect another.

The first preload placement in the PC facade did not cover the native session
harness. Corpus56 passed600 ticks29.21s, but its journal still showed the cold
load inside physics_pre_solve. This finding was retained instead of declaring
the hitch fixed. Preload now belongs to the shared SelectedSession boot prefix,
with an explicit mod-data error before any world/player physics is admitted.
The PC facade uses that same path; its duplicate preload was removed.

Corpus57 passed the same600tick scenario29.53s. All7896 JSON records parse and
end normally. Pack verification took2898.713ms and Collections loading264.085ms
in the unoptimized harness, both before the first tick_stage record. This
separates actual pack-verification cost from JSON loading rather than assigning
the entire old stall to parsing. First private manager construction occurs
1.169ms after the physics_pre_solve stage record. No sampled nonzero physics
pre-solve heartbeat remains. Every sampled partial-physics/HUD/input record is
byte-identical to corpus53. These are unoptimized harness timings, not measured
release-game performance or a claim that every lag spike is resolved.

No game window or user package was launched. Automatic policy still blocks
permanent script cleanup; no alternate deletion mechanism was attempted.


### Independent Footplants complete through corpus 63 (2026-10-20)

The independent Footplants replacement now includes source S3 FootPlant601
and Boneless/Fastplant602 on the actual S2 character, state selector, world,
predictor, skeleton and solver. No S3 player or second solver is constructed
for these partial replacements. Private episode state is cloned from
immutable preloaded source recipes. The source full-player path calls the
same extracted calculation kernels. Its 29 FootPlant and 33 Boneless sampled
trace records remained byte-identical to the corresponding pre-extraction
baselines (38 and 54).

Boneless uses source launch curves and operation order, authored FootJump,
the actual pushing foot/toe anchor, processed prepared velocity and ground
forward. FootPlant and Boneless configure the actual retained animation
drive with source HookDriveState parameters. FootPlant uses the shared
source collision-check kernel and forwards ordered duplicate requests.
Correction to earlier research notes: the current source dispatcher has no
Boneless602 post-physics wipeout branch. It is intentionally not assigned
the FootPlant601 collision check. Stock S2's own Boneless check is unchanged.

The live 900-tick Boneless ownership test62 passed in 39.95s: disabling a
source episode finishes it before retirement; enabling during a stock
episode keeps its current owner until release; the next episode uses S3.
The corresponding 600-tick FootPlant ownership/steering test also passes.
The recipe isolation unit covers both managers and passed in 2.85s.

Run61 journals contain 15,743 valid Boneless JSON records (four starts and
four launches with consumed input/output diagnostics) and 7,896 FootPlant
records (ten contact changes, two starts/releases/owners); both ended
normally. Run60's four full/partial plant cases passed in 203.46s. The PC
check60 passed in 28.25s with its 18 pre-existing warnings. Core build62
includes the later diagnostic and ownership tests.

Broad run63 passed all 29 integration cases in 572.42s, including these
paths plus camera, actor policies, party ownership, difficulty, offboard,
audio/surfaces and scoring/HUD regression coverage. The featureless stock
core check63 passed in 15.52s with 439 baseline warnings. These automated
corpus results do not establish full career, object manipulation or live
release performance. Independent Darkcatch, remaining difficulty assists,
full-game contexts and final gesture playback are still unfinished. No new
release, user package or GUI game was launched in this phase.

Permanent deletion of the verified obsolete scripts remains blocked by
automatic approval review ("blocked by policy"). Both the batch removal and
a fully literal single-file removal were rejected before execution. No
alternate deletion mechanism was attempted and those files remain.


### S3 difficulty air assists on the native S2 owner (2026-10-20)

Native2 difficulty tuning now reads EasyBodySpins, PerfectBodyFlips and
AutoPushEnabled from the embedded source collections. Easy and Easy Motorized
enable both air assists; Motorized enables assisted spins only. Normal and
Hardcore leave these flags false. Skate 2 retains its original air path.
AutoPushEnabled is read for provenance but is not yet connected to native2
automatic push input; this section does not claim that assist complete.

Easy/Motorized air spin calls the actual TU3 BodySpin82D8BF58 kernel and its
seven authored curves. A transient typed source view consumes and writes
back the same retained S2 history, cursor, smoothing and speeds. There is no
second history or solver. Ground and non-assisted updates remain original
S2. Perfect flips call the source UpdateBodyFlip82D8EC70 kernel on the actual
S2 axes/angle/speed. The source KnownAir perfect-target calculation was
extracted without changing its operation order and is shared by both paths.
The native2 graph and flip-start gate still control admission.

The two ownership units passed in runs67 and69: actual source curves/history
including preserved unused scalar and all consumed state words; and actual
S2 flip axes/zero-angle output with positive/negative turns, finite output
and untouched input W lanes. The PC check67 passed in36.29s with18 baseline
warnings; featureless core passed in18.63s with439 baseline warnings.

Runs67/69 flat-ground corpus trials exercised all six spin profiles but
failed the requested flip assertion. Actual graph maps showed the recognized
BackFlip and grab intent; S2's real minimum collision-time gate is0.8s.
Run70 uses an actual Actor teleport to an8m drop at tick65, without forcing
states, intents, trajectories or a source player. All six profiles then
entered air, spun and flipped through the original S2 animation and solver.
The1800-tick corpus passed in71.80s. One actual observation was collision
time1.1555641, elapsed0.4166668 and perfect target-6.524799. Late attempts can
bail naturally; this is not a claim that every attempt lands a complete flip.

Remaining independent Darkcatch is still explicitly unconnected. Source3
automatic-push preferences also need their real output producer connected;
do not guess Animation156 or treat the mode boolean as a complete input
path. Existing authored DMO/vehicle grab providers were located in
living_world/interaction/living_world_grab_provider.rs. Filling the source
registry alone will not complete interaction: dynamic validation lines,
current assemblies and manipulation reactions must also be connected.


### Air-assist journal and regression checkpoint72 (2026-10-20)

Run71 repeated the six-profile1800-tick actual drop corpus with automatic
logging: passed in72.26s. Its24,073 JSON records have complete, unique sequence
IDs and a normal session end, including7,200 raw-pad records,7,200 tick
stages,9,000 presentation stages,564 diagnostics and five changed difficulty
profiles. The initial stock profile is unchanged None, so it does not emit a
profile-change event. Heartbeat concurrency caused12 physical-file
sequence reversals; sorting by the unique sequence ID reconstructs order.
Audit: work/native-air-audit72.json (local work folder).
The source3 air-module unit selection passed all42 cases.

Regression72 passed34 of35 selected tests in202.79s, including the full
source Darkcatch input/pose/HUD test, the main selected-frame/camera/party/
offboard/six-difficulty corpus, retained spin histories and failure prefixes,
and the two new air-assist units. One legacy corpus failed before entering
physics: attribute_runtime_corpus requires skater.vlt851,568 bytes with FNV
f7db0ed527db57b4, while this user's original ROM has851,632 bytes with FNV
9ad2cf9a7bcb71dc.
The frozen identity check and its numerical oracle were left unchanged.
This is a corpus compatibility limitation, not a passed test; the actual
user-ROM runtime corpora above use the real mounted Vault successfully.

Both official Git HEADs were rechecked and remain S2f72f2c1302bf9cf176f842ace7b441d1824feba4
and S32e16697797c8358d16f747a8259354ff2921041a. The original save SHA256 is
unchanged. All test processes are closed; no GUI game or new user package
was launched. The goal remains unfinished, including independent Darkcatch,
automatic-push input, object manipulation, full-game contexts and gestures.


### Automatic pushing connected on both solvers, through75 (2026-10-20)

The earlier unconnected AutoPushEnabled limitation is superseded. The
source3 riding-intention auto gate now has a consumed-observation helper;
the complete source producer calls that same helper. The actual source
angle, magnitude,0.6/0.8 comparisons, manual-push and actor gates retain
their order and values. There is no extra controller history.

Native2's selected Easy/Easy Motorized profile applies it after its real
listener Fill to the same Actor's ActionGraph. It consumes the actual
Derived stick, Ground100, signed deck speed and source physics_push threshold
8.0. Existing manual/script push, AI, pins and ForceBrake are respected.
The full source player reads its preceding actual Ground output's selected
mode/speed gate. Both paths choose the active character's regular(non-mongo)
foot using the established S2 Update5 regular/switch classification. This
is explicit host integration policy; no equivalence to an unrecovered
numeric S3 Animation156 storage slot is asserted.

Run74 passed16 cases/2400ticks in101.38s: both solvers, all six difficulties,
and the opposite leading foot for both automatic profiles. Only Easy and
Easy Motorized automatically pushed. No case used the wrong physical foot.
The source helper unit also passed its forward, manual, lock, disabled,
nonfinite and both-foot cases. The featureless check74 passed in17.88s with
439 baseline warnings; PC check74 passed in27.50s with18 baseline warnings.

Run75 repeated all16 cases with actual Actor.try_set_pinned110..125. Both
input paths contained no pushing intents while pinned, retained ForceBrake,
and resumed after unpinning. All assertions passed in101.97s. Its33,259
valid JSON records have complete, unique IDs and a normal end, including
220 native2 automatic-push events and1200 solved source frames. Actual
native2 deck-speed peaks were4.30..4.32m/s; source solved horizontal speed
peaks were4.43..4.79m/s in automatic cases. This validates physical motion,
in addition to intent and pose flags. Audit: local work/auto-push-audit75.json.

No GUI game, new release or user playtest package was launched. The main
integration remains unfinished; independent Darkcatch is still an explicit
runtime error. Full-game contexts, object manipulation, common animation
composition, offboard edge cases, live performance and final gesture
playback still require work.


### Full source physics priority, host grab scene through85 (2026-10-20)

The user explicitly prioritized All Skate3 Physics and deferred gestures.
Independent feature expansion is also deferred while completing that path.

Regression76 passed32/33 cases in612.03s; the remaining party walking-acquisition
assertion also failed in isolated79 in41.84s. Its preceding six-profile scenario
now genuinely automatically pushes in EasyMotorized, ending at8.232m/s. The
primary actor was then left to its native solver for80 inactive ticks, hit
scenery, dismounted/bounded into offboard air and recovered before the fixed
140-tick assertion. Thus that assertion's statement that Y never dismounted
was false. Run80 establishes the separate walking acquisition with a real
Actor marker reset at80, requires native Ground100/non-wipeout at84 before
raw Y85, and retains the original140/141 walking continuity assertions.
The whole selected-frame case passed in176.47s. This repairs a fixture
precondition; it does not certify the user's original Party Play symptom fixed.

The full source player now supplies its original grab manager with actual
Skate2 LivingWorld DMO/vehicle spline IDs, points, approach vectors, flags,
transforms, velocity and attached Part inertia. Geometry owners are retained
by Arc and cached by actual bank/record occurrence. Assembly identity retains
the actual allocation handle; entity, mesh and body slot numbers are not cast
to assembly IDs. DMO and vehicle provider feed cvars remain host-owned.

Grab validation/interactable lines now use immutable admitted-tick snapshots
of native2 ClusteredMesh slots. The live native World and snapshot path call
the same existing ordered line kernel, preserving pool bits1/2/4, conditional
vehicle island3, nearest-hit comparison and actual physical owner lookup.
No generic solid approximation replaces that authored geometry. Source
query/validation/reset timing is unchanged; host snapshots are installed
after terrain replacement and before the source input/physical tick.

Source Processed object2464 and its PostInput2480 bit20 validity now reach the
original2 State.interactable_object_id and Offboard.has_valid_interactable_object.
Actual BipedGround State36/40 descriptors and Offboard304 grabbing also reach
the original2 prop-HUD consumers. These observations are refreshed each tick.

PC check82 passed in44.52s with18 existing warnings; stock85 passed in32.97s
with439 existing warnings. Core83 compiled with926 warnings in2m37s but its
new audit compared a pre-solve grab snapshot with post-solve/reaction-updated
native object targets and failed.84 failed to compile one test-only module
alias; corrected85 uses the actual nested geometry module.85 passed the
populated real-ROM case in36.69s. It verifies source resolution of original
spline IDs/geometry, endpoints through the native2 matrix math, source-native
assembly identity and actual eligible mesh hits, plus five interaction observer
fields each frame. At tick0 there were3 objects,16 records,3 eligible mesh hits;
at179 there was1 object,8 records,1 hit after actual census changes. The6409
JSON records have complete unique sequence IDs and a normal scope exit.
Audit is local work/grab-native-audit85.json.

This validates authored scene/query/observer integration. It does not yet
certify user-controlled object manipulation or skitching; source Skitching104
and FollowPath105 are unsupported in the imported state registry. Main full
movement regression87 and internal release86 are in progress. No new user
package or GUI game was launched through85. Live performance, menu/focus,
full-game camera/context and interaction behavior still need verification.

Regression87 completed:27/27 passed in218.09s, including full source selected-frame
movement/camera/HUD/offboard/party handoff, full source darkcatch, native2
ClusteredMesh line kernels and host occurrence-bound reaction identity.
Internal optimized release86 is still compiling; no GUI test yet.


### Internal release86 and rendered full-physics checks (2026-10-20)

Optimized release86 completed in12m55s with18 PC warnings. EXE81,053,184 bytes,
SHA25679f147045189029be9d6752896b8b92ec47d8e84cbae4817ad866b46e4175281,
source fingerprint1acb5c122662af11728d161af8dfd96e14d2d0c0a50ad7caebe73cf7d89d89f3.
The existing diagnostic CLI supplied raw controller samples through the actual
input path; no Windows input automation or separate3runtime was used.

The first bounded rendered run completed900 source ticks in29.72s wall time,
with851 BipedGround500 ticks, real dismount/ground/air transitions and roughly
66m of movement on original2 terrain. The captured running character retains
original2 clothes/mesh and minimap. Its19,559 journal records have complete
unique IDs and normal end. Typical admitted tick median5.85ms,p95=7.30ms,
p99=9.13ms;8ticks exceeded33.33ms. Four spikes aftertick60 were222.76,482.637,
414.116 and444.357ms. Journal stage ordering locates the large stalls inside
physics_solve, with one heartbeat explicitly naming skate3_player_solve.
This reproduces the reported stutters and does not claim them fixed.

The second run went through the actual main menu, two-player join, S.K.A.T.E.
location load, pushes/flick/ledge landing and turn change. It completed3500
frontend ticks/1365 source gameplay ticks in85.46s. The source physical owner
changed to local index1; the final captured native2 Party HUD highlights Player2.
It exited0 and produced97,580 complete unique journal records with normal end.
This covers one actual rendered party handoff, not every Party Play condition.

Both game processes closed, and both original-save hash comparisons passed:
ad1593ddbc5cc0f9e22a2dc8cc9f39b9b7565fc36b8eef0756c46c50f72b9c44.

After the user offered to test, a fresh outputs/Skate2-All-Skate3-Physics-Test-2
package was made without overwriting Test1. Its ZIP is80,603,511 bytes and its
contents/EXE hash were checked. All source3 features/Normal are preselected;
the package uses a copied save and automatic detailed logs. The README labels
it unfinished, identifies the lag/object/skitching/career limits and directs
testing to the full replacement, keeping independent toggles/gestures deferred.
It is a feedback candidate, not completion of the active goal.


### Streaming performance work89..92 (2026-10-20; validation in progress)

The optimized86 spikes align with changes to resident rail counts, while the
selected-frame stage diagnostics put 168..478ms inside the native physics
owner.89 adds scoped timings for ground decode/world construction, grind
conversion/provider construction, grab refresh and the actual source tick.
Its existing populated450-tick corpus passed in35.41s; that fixture retains
one terrain set, so it does not reproduce the rendered streaming spikes.
Its initial1940-segment grind conversion took5.987ms and provider14.063ms in
the explicitly unoptimized test binary. These are not release FPS claims.

The working adapter now retains decoded ground meshes only while both their
actual arena Arc and complete native slot identity match. Current slot order
and native KD triangle/features are retained; departing sources are discarded.
Grind conversion caches immutable payloads by the existing source identity,
uses the serialized144-byte stride through the checked format resolver instead
of repeatedly searching the segment array, and issues header IDs in the same
first-encounter order. Source loose-octree construction remains unchanged.
A test-only copy of the old conversion is the bitwise oracle for actual source
reversal, removal, re-admission and duplicates. The actual terrain focus test
compares the cache against fresh exports through unload/move/revisit.

90's formatter stdout included a filename prefix and91 needed an explicit
closure error type; both compile failures were corrected.92 is compiling.
Test2 remains frozen and the user's launched copy is not closed or replaced.
This work does not yet establish the rendered lag spikes fixed.


92 validation completed: core compiled in2m22s with926 existing warnings.
The populated450-tick case passed in39.63s, including all seven old-vs-cache
source sequences. Coefficients/bounds are compared by f32 bits, plus exact
header IDs/GUIDs/local indices. Reordered retained1940-segment conversion was
0.190ms versus6.197ms for the uncached oracle in the same unoptimized process.
The actual native terrain unload/move/revisit case passed in16.73s; unchanged
mesh allocations are retained and current exports equal fresh original-data
triangle and feature arrays. The checked spline offset parser test passed.
PC check passed47.47s/18warnings and stock check29.09s/439warnings.

All450 completed physical-output diagnostic strings are identical between89
and92, SHA256d734643adec0588c5acf70bdca36aa5bfe46110e53312aa0a72ea00f0a11ad31.
Their6859/6871 journal records have complete unique sequence IDs and normal
scope exits. This is a trajectory regression for that fixed-terrain fixture;
rendered release performance still needs measuring. Audit is in local
work/stream-cache92-audit.json.

While the user tested the frozen package, its logs revealed two explicit
unsupported independent-Darkcatch selections and a separate full-mode fatal
at frame8687 in Party Play, Easy Motorized, Low camera. The latter followed
travel completion, a physical owner change and zero resident ground at source
construction.93 defers acquisition until the replacement world publishes real
collision, retaining the native frontend preview while waiting and applying
full source physics when admitted. Its new real-ROM boundary test attempts
three empty-scene admissions, then the actual world, checking the same native
Skate2 Actor owner.93 is compiling; no result or rendered fix is claimed yet.


93 compiled in2m08s/926warnings. Its actual-ROM terrain-admission case passed
in12.19s: three pending attempts do not construct a source solver, actual
resident terrain admits it, and the original Skate2 Actor Arc is identical.
The journal contains the explicit terrain_admission_audit result. This proves
the boundary guard; an actual Party Play travel/return replay is still needed.
Optimized internal release94 is building with cache/timings and admission93.
It does not modify the frozen Test2 EXE or close the user's game.


### Release94, Party return, real focus timing and geometry95 (2026-10-20)

Release94 compiled in10m24s with18 PC warnings. EXE80,932,352 bytes,
SHA256461d34ad8ca24aa8c4068cbc4343ec01a05ccf145110711b9e145e5b6b282a9c.
The900-tick streamed walk passed in30.13s, preserving the same physical state
counts and visibly the original2 skater/minimap. Its19,436 journal records are
complete/unique with no fatal events. Later spikes fell from222.76/482.637/
414.116/444.357ms to54.517/271.626/95.904/144.069ms. Median6.31ms,p958.09ms;
four late spikes still exceed33ms. This run shared the machine with the user's
frozen Test2 instance; it is not an isolated whole-system FPS comparison.
New stage timings identify decode up to180.868ms and world construction up to
67.789ms. Actual retained terrain reaches326,273 triangles; this remains a
measured, unfinished streaming issue.

A first Party-return fixture was invalid: PowerShell coerced a null string
environment value to an empty value, so presence-based BACK and other optional
diagnostic switches were inadvertently enabled. No Party gameplay acquisition
was certified from it. Explicit `[NullString]::Value` removes those variables;
the corrected Easy Motorized case passed133.48s/3607 source ticks, then the
Normal case deliberately waited for Player2's active turn before quitting.
It passed134.32s/3610 source ticks, observes selected1 then selected0 and the
actual free-skate return. Its168,963 records are complete/unique with no fatal
events; the first corrected case has172,591. The new-owner route passes, but
no empty-scene wait was encountered in that run;93's separate boundary corpus
covers that condition. All internal processes exited; the user's instance
was retained. Original save hashes stayed unchanged.

The computer-use skill controlled a separate bounded GUI test through Sky.
A menu-only attempt did not reach player physics and is not focus proof.
The direct gameplay attempt supplied the existing scripted Ollie input to
skip frontend startup, with no controls_demo/completion_limit clock bypass.
Actual Windows activation of the project Explorer window and return caused
60.644158 seconds of lost focus: zero source ticks occurred during that gap.
The same completed-frame count1529 appears at loss and recovery. Every full
second measured after the first recovery second contains60 source ticks;
there was no catch-up burst. It exited normally after167.46s. The actual menu
pause and online timing paths remain outside this check. Audit is local
work/full-skate3-focus94-direct-20261020-145441/focus-audit94.json.

95 removes discarded native2 contact-GP calculations from host geometry
export. The exact points/edge cosine/flag topology projection shares the old
serialized dispatch, transform and edge kernels; build_gp_record copies these
fields unchanged, while its face frames, edge directions and dimensions are
not consumed by this export. The original native2 contact GP path is unchanged.
A test-only full-GP oracle compares all emitted feature fields by float bits
on actual terrain. The unload/move/revisit case passed20.09s and populated
450-tick case36.95s. All450 physical outputs still match89 bit for bit in the
logged representation, SHA256d734643adec0588c5acf70bdca36aa5bfe46110e53312aa0a72ea00f0a11ad31.
Core95 compiled3m56s/926warnings. PC/stock checks are running; no optimized95
performance result or new user package exists yet.


95 PC/stock checks passed in1m10s/52.61s with18/439 existing warnings.
96 adds diagnostic-only world-construction phase measurements: vertex/feature
materialization, mesh metadata, validation, canonical triangle data and query
index. The profiled constructor preserves the exact ordinary constructor
ordering and all query/contact arithmetic. PC check96 passed1m06s/18warnings.
Optimized release96 is building; no optimized95/96 performance claim yet.
The current test EXE remains95, the internal rendered EXE94 until this build
finishes, and frozen user Test2 remains86.


### Internal release96 and bounded career entry (2026-10-20)

Release96 passed in9m58s with18 PC warnings,81,116,160 bytes. SHA256
595376487054db826652f8f8a7029354e27a825d9035e1bcf6986849b703ae83.
Its900-source-tick rendered walk exited normally in27.126s with a complete
unique journal and no fatal events. Post-startup median5.907ms,p957.219ms;
four later tick spikes remain49.749/188.363/82.184/114.445ms. This is still
a streaming overhead issue, not a smoothness certification. With326,273
triangles, preparation32.333ms, metadata14.561ms, validation4.210ms, triangle
data11.661ms, index1.760ms. A mesh removal alone rebuilds all remaining
triangles, confirming that unchanged derived data should be reused.

Two bounded career-entry cases used the frozen internal94 executable,
despite their helper's96 directory prefix. They exited normally in102.597s
and100.944s, retaining3940/4175 source ticks. The native Skate2 character
and Meet Slappy quest HUD were observed. The raw-A input variant retained
109,206 complete unique journal records with no fatal events; later tick
maximum10.741ms. Neither case proves natural objective completion or full
story compatibility. Original saves and frozen user Test2 are unchanged.

97 keeps host moving-query ownership across static-world replacement. The
actual selected tick publishes host NPC/car/prop snapshots before terrain
refresh; assigning a new BoardWorld cleared its external query field for
that tick. The new static-only replacement transfers that already-admitted
query owner, preserving fresh snapshots and permitting an explicit None.
The source contact/reaction solve is unchanged. A regression exercises
actual MovingQueries box contacts, moved-body publication, departed terrain
and removal of the query owner in the production schedule order. Validation
is pending. The same batch retains prepared triangles by actual arena Arc
and full slot identity, seeding from the initial native world. Material bits
are part of cache eligibility. Current native order, global64-triangle leaf
boundaries and full metadata validation are retained. Real-ROM comparison
checks all31 float fields, flags, surface tags and metadata bounds bitwise
through source movement/removal/revisit and contact material changes.

The user's latest clarification narrows immediate work to native Skate3
skater physics on the actual Skate2 character with its existing gameplay
connections. Gestures and extra mod options remain deferred.


97 checks passed: PC1m18s/18warnings; stock54.69s/439warnings; core test
build4m13s/926warnings. Moving-query regression passed. Actual terrain
unload/move/revisit, prepared-geometry/material/metadata float-bit oracle and
full native2 GP-feature oracle passed28.32s. Actual450-frame populated world
passed35.26s with no fatal events; all450 physical outputs match89 exactly,
SHA256d734643adec0588c5acf70bdca36aa5bfe46110e53312aa0a72ea00f0a11ad31.
The new cache module and newly extracted function/test were formatted after
these checks; no behavioral edits were made. Optimized97 is building for a
bounded rendered streaming test. No smoothness or final readiness claim.


### Full-physics-first test stage99 and Skate2 0.2.4 (2026-10-20)

Release97 completed in10m37s with18 PC warnings. Its81,097,728-byte EXE is
SHA256bf19ee211921edeead81b538bf37a85ae92f286e94499101ef316a36e7c11296.
The actual900-source-tick rendered walk exited normally in27.639s with19,622
complete unique journal records and no fatal events. Original save unchanged.
After frame60, median6.119ms,p957.279ms,p997.979ms; four later spikes remain
33.822/182.819/49.484/82.353ms. This is not a smoothness certification.
The prepared cache retains unchanged geometry; the full new-tile decode still
costs time. The capture uses the original Skate2 skater, clothes and minimap.

The human explicitly excluded skitching if absent upstream, then asked for
full Skate3 physics confirmation before independent mixed-physics toggles.
Skitching104 remains unsupported exactly as upstream. An unreferenced scalar
research file in vendor/player is not declared or compiled and is not an
implemented feature. No further skitching work is in this test stage.
The native Mods physics page exposes only All Skate3 Physics for now; future
feature fields remain saved without being offered as completed switches.
Gestures, mixed physics and independent Darkcatch remain deferred pending the
human's full-physics test. Career entry is tested; full story is not certified.

Skate2 upstream0.2.4 f2f953243e8308fd3a09283580ae8804b2e1a66b was fetched and
its13-file delta three-way merged from f72f2c1 with no conflicts, retaining all
native host changes. The merge includes missing character UV defaults for
hair/glasses and CAS, actual network respawn validation and lobby fixes.
UPSTREAM-SYNC.json records0.2.4; original source HEAD was not reset.
The new original-asset test helper was adapted to this integration's existing
shared Arc<Vault> return type (no second Arc wrapper). PC check passes1m26s
with18 warnings. Core/test and release99 validation are pending below.


99 validation completed: core test build2m23s/927warnings; PC test build
3m17s/13warnings, including the existing MSVCRTD linker warning. Stock core
check1m38s/440warnings. Core42 and PC8 targeted tests pass (50 total).
The real selected-player corpus passed200.74s, including source movement,
original HUD/scoring/audio/camera/party/profile owners. Actual populated
world passed36.07s. Six character upload tests, original tutorial character
admission0.41s and native Options APT lifecycle5.49s pass.

Release99 passed in601.18s with18 PC warnings; EXE
81,112,064bytes SHA2568423e319811558dd915d1813151d08f051dd086f560b6a6258827639f16f8b4c.
Four bounded rendered tests all used that exact executable:
- walk: 28.423s, 900 source ticks, 19,530 journal records.
- skate: 27.492s, 900 source ticks, 19,540 journal records.
- marker: 27.370s, 900 source ticks, 19,515 journal records.
- party-return: 134.692s, 3619 source ticks, 169,619 journal records.

Every run exited normally with complete unique journals, scope exit, no fatal
events and unchanged original save. Party Play explicitly quit during the
second player's turn, observed native owner changes ['selected=1 changed=true', 'selected=0 changed=true'],
and returned to the actual free-skate game. All internal game processes exited.
The walking and onboard captures were visually inspected: original Skate2
striped-shirt character and board in the original world/minimap, with source
movement and low camera. No second rendered avatar or external3 runtime.

An initial test driver treated an unset LASTEXITCODE as failure after the
successful first walk. The orchestration was corrected and reused/audited
that same passing result; no physics failure or rerun was hidden.

Streaming spikes remain. After frame60, walking median6.116ms/p958.455ms,
max193.078ms with four spikes; onboard median5.728ms/p956.836ms,max182.008ms;
marker median4.980ms/p956.091ms,max191.553ms. These checks do not certify
every move, mission, surface or long-session interaction. Full story remains
unproven. Prior94 real Windows focus test remains the focus timing evidence;
these scripted tests bypass real-focus pacing and do not replace it.

The Test3 package is the full-physics confirmation stage requested by the
human. Keep it frozen after delivery. Wait for that feedback before exposing
or developing mixed-physics toggles/gestures; skitching is skipped. The broader
goal remains active and is not marked complete on a short playtest.

Party99 timing after frame60: median12.949ms,p9516.744ms,p9919.259ms,
max623.856ms with8 ticks above33ms. Menu/owner/world transitions are included;
this is an unresolved hitch, not a certified60fps result.

Test3 ZIP80,732,369bytes; integrity/executable hash verified. EXE81112064bytes
SHA2568423e319811558dd915d1813151d08f051dd086f560b6a6258827639f16f8b4c.
No internal game remains running. Package is frozen for human feedback.


### Native collision topology reuse100/101 (2026-10-20)

The latest human scope still gates mixed physics and gestures on full-physics
Test3 confirmation. Test3 stays frozen. Work here only targets existing native
physics hitches; no original streaming schedule, resident selection or query
capacity is changed. Previous99 gameplay logs measured source terrain decode
up to105.493ms and complete terrain refresh179.534ms in the streamed walk.

100 adds separate per-mesh triangle and topology preparation timings to the
existing native-player diagnostics. Its core test build passed1m51s/927warnings.
Two headless source-admission runs cover the same72,405 original triangles.
Triangle expansion55.911/57.004ms, topology273.554/294.999ms. These are
unoptimized-test timings, not rendered-game FPS measurements.

101 borrows the cluster/unit decode already validated by the same immutable
mesh's PreparedMeshTriangles. The old feature export parsed all clusters and
units again after the successful triangle export. A read-only accessor exposes
those same serialized-order slices. Only the mod export consumes it; native2
query arithmetic/order/error handling is unchanged. The per-unit cache now
uses a single HashMap entry lookup. No iteration over that map determines
output; the original KD-ordered triangle list still determines every output,
duplicate visit, sibling prefix and error. Physics/contact math is unchanged.

The test-only full-GP oracle independently reparses the original bytes, so
the retained metadata cannot validate itself. It compares all point/edge
float bits and flags against native2's full GP constructor. The original-data
terrain unload/move/revisit plus97 prepared-world31float/material/metadata
oracle passed27.38s.38 native pool tests passed. Populated450-frame simulation
passed39.69s, with6,872 complete unique records, no fatal events and every
physical output exactly equal to baseline89. Joined-output SHA256
d734643adec0588c5acf70bdca36aa5bfe46110e53312aa0a72ea00f0a11ad31.

The same two72,405-triangle headless samples now show topology176.798/147.906ms;
mean284.277 to162.352ms,42.89percent lower. Triangle expansion62.239/52.872ms
remains in the same range. This establishes the targeted cost reduction only;
an optimized rendered comparison is still needed before a gameplay claim.
Core101 test build2m18s/927warnings. The changed export function was formatted
after this build with no behavioral edits, then its binary tests passed.
Formatted source PC check29.90s/18warnings and stock21.99s/440warnings pass.
An internal optimized101 build/test is next; no new user package is planned.


Internal optimized101 passed in489.40s with18 PC warnings.
EXE80,941,568bytes, SHA256c7782ee499ab52998bc638c4cc4b12eb6b9212bd94bf3e3873e2d5e0f2e0869d.
- walk:900 source ticks,27.228s,19,630 complete unique journal records.
- skate:900 source ticks,26.682s,19,614 complete unique journal records.

Both actual rendered runs exited normally with no fatal events and unchanged
original saves. No user game was running alongside either test. Source state
counts match99 for both routes. Walking steady median6.116 to6.190ms, p95
8.455 to7.373ms, maximum193.078 to151.953ms; late>33ms ticks4 to3. Onboard
median5.728 to5.543ms,p956.836 to6.548ms,maximum182.008 to150.467ms; late
ticks4 to3. Walking source terrain decode max105.493 to72.123ms; complete
refresh179.534 to139.326ms. Onboard decode102.772 to70.757ms and refresh
170.839 to139.357ms. These are bounded route measurements, not a whole-game
performance certification. Significant hitches remain, and world rebuilding
still reaches roughly66-67ms. Test3's EXE hash was reverified unchanged.

All internal game/build/test sessions ended. No new user package was made.
Mixed physics/gestures remain gated on human Test3 confirmation, skitching
remains excluded. The broader goal is unfinished and remains active.


### Immutable triangle bounds cache102 (2026-10-20)

101 still rebuilt all per-triangle bounds and global64-leaf metadata after
every resident change, including unchanged meshes.102 caches matching bounds
with immutable canonical triangles in a private-field PreparedTriangles type.
Chunks concatenate in actual resident order; initial cache seeding copies the
already admitted world's bounds. No caller can pair unrelated triangle/bounds
vectors through this API. Native2 streaming/selection and all collision math
remain unchanged. The ordinary BoardWorld constructor uses the exact previous
bound calculation and the shared unchanged aggregate/contact initialization.
The new constructor retains full query metadata validation. Maximum fatness,
margin, water indexes and contact-buffer setup keep the same reductions/order.

Global query leaves remain64 triangles across mesh boundaries. Their boxes
can combine cached per-triangle bounds. Any signed-zero endpoint or invalid
sentinel takes the original vertex-order bounds scan, preserving min/max
zero-bit behavior and original invalid-geometry errors. Current resident
identity, full material-bit invalidation and moving-query retention are intact.

Three targeted host tests pass, including the new130-triangle chunk/reorder,
global-leaf, signed-zero, rounded-line, missing-surface, NaN/Infinity and empty
world comparison, plus existing moving-body retention and surface-tag tests.
Real terrain unload/move/revisit/material cases now additionally compare every
cached triangle bound by float bits, alongside the97 canonical31float and
metadata oracles and original-byte full-GP oracle. They pass31.55s. Actual
populated450-frame test passes38.83s with6,871 complete unique records, no
fatal events and all physical outputs exactly equal to89, joined SHA256
d734643adec0588c5acf70bdca36aa5bfe46110e53312aa0a72ea00f0a11ad31.
The first core compile found a missing local Vector3 import; it was fixed.
The final core test build passes5m48s/927warnings; PC check2m01s/18warnings;
stock check3m35s/440warnings (including waiting for the test target lock).
An internal optimized timing build is next; no performance claim yet.

The human's frozen Test3 is running as PID16712 on the last process read.
It is not overwritten or closed. A new rendered performance test will wait
if a user game is still running, to avoid loading their session or contaminating
the timing comparison. Human physics confirmation still gates mixed features
and gestures. Skitching remains skipped. No new user package is being made.


### Ramp airtime clock capture and optional camera effect103 (2026-10-20)

The human reported apparent speed loss off a ramp, then marked a reproduction
with “just happened”. A read-only snapshot retained the active Test3 journal at
17:17:59 PDT in work/user-air103-repro-20261020-171759. Source camera requests
include tick845 rate0.028517757: the native reciprocal rounds to35Hz. In the
peak450ms window,16 physical publications have median28.004ms wall spacing,
while median computation is5.431ms and maximum6.133ms. This is positive evidence
of camera-requested whole-simulation slow motion, distinct from a costly tick.
Six local3 ground/air modules compare identically after namespace/type/test
module renames. This establishes the diagnosed clock effect; it does not prove
every ramp contact or velocity change correct.

The human explicitly requested normal airtime speed and then a Mods option.
Mods > Airtime Slow Motion is a separate on/off selector, default off. The
selection persists in gameplay.json version3; version2 and legacy mode saves
remain readable with this optional effect off. Physics and difficulty switches
do not alter the effect preference. It controls S3 camera time requests only.
The unchanged source graph/camera and physical equations still run. Disabled
requests, including normal-rate End messages, are observed but not enqueued.
Already enqueued one-tick source requests expire through the normal native
countdown. Releasing the player no longer sends an unconditional normal-rate
reset that could cancel native story/script timing. Actual host timer changes
now have a simulation_timer_rate journal breadcrumb.

Validation: PC --tests check passes1m11s/12warnings. Core test build passes
3m02s/927warnings after correcting the diagnostic build to the existing
debug=0 cache.19 targeted tests pass:2 actual embedded-camera-curve/host-clock
tests (on/off, mid-jump disable, native script preservation),6 preferences,
7 menu/navigation and4 native timer tests. Actual original options.apt movie
layout/lifecycle test passes, including the new row and both localized values.
Original-ROM populated450-frame execution passes42.66s, with
6,874 complete unique journal records, no fatal events, and every
physical publication exactly equal to baseline89; joined SHA256 d734643adec0588c5acf70bdca36aa5bfe46110e53312aa0a72ea00f0a11ad31.

The independent102 cache release succeeds12m56s/81,105,408bytes, SHA256
2851413D1A4BE9AD6AD20959B0AA5A8CE5DD8C8AA9A5CE12A3A093D2A45CEEF4.
Rendered102 timing was deferred because the human's Test3 PID16712 was live.
No new performance claim follows from compilation. Their Test3 is unchanged;
neither102 caches nor103 option are in that currently running EXE. The same
user capture also contains real collision-refresh hitches and ~500ms player
admission/Party handoff stalls. Those are separate and remain unfinished.
Human full-physics confirmation still gates mixed features and gestures;
skitching remains skipped. Full career and broader workspace gates remain
unproven. Do not describe the integration or performance work as complete.


### Authorized online compatibility and Test4 (2026-10-20)

The human reprioritized a current online playtest and explicitly paused manual
investigation. They want existing games in the upstream Lobby Browser. After
learning that builds have separate network identities, the human stated that
Wispp, the creator, granted permission for compatibility. That authorization
supersedes the earlier stop at the build-authentication boundary.

Official 0.2.4 Windows release binary was downloaded from its GitHub release.
Only its distributed NETWORK identity is used, verified by public build ID
3548cb37 against the official executable's own loopback server startup. The
private release/update signing key was neither present nor used. The optional
SKATE2_NETWORK_BUILD_SECRET_FILE build input embeds the authorized network
identity into the EXE; handshake proof validation, encryption, game version,
message codec and all server permission checks are unchanged. There is no
runtime sidecar, proxy, injected DLL or separate physics process. This network
configuration change does not affect the source physical equations.

The initial optimized current-source build passed576.31s. Rebuilding the
application with the authorized identity passed127.36s. Final EXE81,130,496bytes,
SHA256 EBC891F3C5A013C253620F80CB08A9600F564B55AC5F70493F5EA312A0A1F401.
Both upstream HEAD checks still match the pinned sources: S2f2f9532, S32e16697.
Nineteen camera-clock, preference, menu and timer checks pass. Two rendered
original-ROM tests (walking and onboard skating) each complete900source ticks,
exit normally, retain complete journals and preserve the original save.

The same EXE then signs in to an unmodified OFFICIAL0.2.4 server on loopback,
joins a freeskate lobby, runs900source physics ticks and publishes897network
pose frames. Max actual root speed14.372095m/s, no fatal events, normal exit,
34.63s including startup. This proves actual selected-player source physics
and the native online pose publication run together; it does not certify all
online challenges or remote interactions. Public normal sign-in also succeeds
and the actual Online > Lobby Browser lists two of two existing games. The
final public UI check/package report records its own exact run and result.

Early harness attempts were corrected rather than treated as passing runtime
checks: a live-capture invocation initially omitted its required completion
limit; a generic main-menu probe wrongly required ending in the frontend after
an online join; and a public-menu button script initially selected Freeskate,
then sent an extra Up after return had already restored Continue Career. The
final probes use the appropriate gameplay lifecycle and real raw UI controls.
No production checks were relaxed to accommodate these harness mistakes.

The previously added original-data manual diagnostic completed just before
its pause: native/source cases with free-skate/Party mode flags bank positive
finished scores. This is not an installed Spot Battle challenge or proof that
the human's HUD issue is fixed. Only score-owner/final-total diagnostics were
added; manual score semantics are unchanged. Stutters, Fun Track high-detail
loading, disappearing scoring HUD, full career completion, gestures and mixed
features remain outside this current-build delivery's validated claims.


## 106: repeated spawns, grab awards and HUD recovery (2026-10-21)

See [spawn and grab scoring evidence](skate3-spawn-grab-scoring-2026-10-21.md).
Repeated native Actor resets now carry their requested leading foot independently
of the actual character profile's natural stance. Actual source reset callbacks
reset the retained native ScoreModule and notify the existing camera anchors.
Continuous source grab descriptors now publish the native scoring admission bit.
The shipped scoring movie retains completed points through its authored name
outro and processes scoring recovery while temporarily hidden/unbound.

Original-road tests passed 12 repeat-spawn cases and eight grab landing cases
(native/source x left/right x released/held), including native HUD and mission
award agreement. Twenty focused core checks and the real original APT movie
test passed. Optimized build and two-client online validation are recorded in
the test package's VALIDATION.json when complete; this source checkpoint alone
does not certify all public challenges or full career completion.
