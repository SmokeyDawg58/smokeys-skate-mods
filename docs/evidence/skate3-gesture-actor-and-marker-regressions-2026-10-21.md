# Gesture catalogs, Actor animation, and marker regressions

Scope: source changes following the frozen Test 6 release. The game EXE is not
rebuilt or launched for this work. Save-capable probes copy SKATER.P to individual
scratch directories and verify the original file hash afterward.

## Gesture ownership and evidence

Skate 2's CharacterGesture `82CC3198` keeps its original 35-entry catalog,
assets, channel lifecycle, and CAC gesture assignments. Skate 3's original
CharacterGesture `82BAA320`, instance `82BAADD0`, helpers
`82BAAFB0/82BAB080/82BAB258/82BAAED8`, and 37-entry table
`82F881D0..82F883C4` provide the other catalog. Ordinals are not interchangeable.

`Fill8259BF50` supplies the D-pad start/hold rules, direction precedence, and
bumper suppression. The port uses the original into/cycle/out transitions,
hand-busy selection, blocked-channel rules, and per-bone channel weights.
Animation curves and clips come from the existing executable-embedded S3 bank.
No separate installed S3 game is needed by this path.

The same Skate 2 Actor, character recipe, body ACS, hands/face writers, root,
physical bodies, and save owner remain authoritative. The S3 movement graph is
prepared once before Actor phase 3. Its movement base and the chosen gestures
are evaluated by that Actor before physical integration. The composed local and
hierarchical pose is then consumed by the selected physical player. A prepared
continuation finishes that same simulation tick once; there is no additional
simulation accumulator or second displayed skater.

Body ACS now admits retained SQT inputs through `ACSsetData_job82A24000` before
fetch/blend/matrix jobs. The body path previously lacked that execution stage;
its native hand/face SQT path already had it. SetData job indices are released
at body synchronization, together with the existing fetch/blend/matrix jobs.
Both ordinary and bounded-buffer reuse segments retain this ordering.

The PC save extension `SK82_PC::GestureSets` records the selected catalog and
four S3 assignments. The original CAC record keeps the four S2 assignments.
Editor save commits both in one replacement and validates the result. The
selected player invalidates its cached preference after that commit. A former
local Actor receives a release when Party Play changes the controlled Actor.

Confidence: **high** for the recovered gesture lifecycle and actual-Actor
admission, supported by the original assets and focused headless tests. This
does not establish console-wide animation or physical parity.

## Editor preview

Original-data inspection distinguishes the gameplay movement rig (36 bones)
from the original CAS rig (148 bones). Shared body bones bind by authored name
and compatible parent, with the explicit independent CAS Hips root. CAS board,
finger, face, twist and reparented IK channels keep their existing owners.
Preview uses the original CAS base and the S3 body gesture with the same packed
ACS blend coefficient and per-bone W weights. It does not reinterpret an S3
catalog index as an S2 gesture-library index.

The preview is a PC presentation adaptation, not a reconstruction claim for
Skate 3's entire 148-bone CAS animation or its finger/face pipeline.

## Marker evidence

The earlier offboard origin-jump correction is documented separately in
`skate3-marker-return-origin-jump-2026-10-21.md`; it remains in place.

A new fixture boots the actual S2 terrain at the marker recorded in Test 6
run `1792632568608230700-23484`, source tick 12862:
`[1192.4199, 208.65027, -1213.2676]`, retaining its recorded basis. It repeats
both natural stances, both requested feet, reversed headings, and board/offboard
returns through the actual Actor and source controller.

The first fixture completed all eight onboard returns with all four physical
wheels settled within 0.025 mm of the actual collision surface. It nevertheless
failed two offboard leading-foot assertions. An early host GivenStance call
consumed the saved request before the native teleport graph's own GivenStance
operation. The source correction removes that early consumption and logs the
native operation's requested and resulting relative stance. A proposed fakie
compensation did not fix the replay and was removed.

This fixture has **not reproduced** the user's intermittent visible wheel
clipping. Physical contact alone does not prove the rendered wheel mesh is
clear. New reset-time diagnostics log collision gaps, wheel contact flags,
rendered wheel centers, reset age, state and teleport generation without moving
or clamping bodies. Missing diagnostic line hits are recorded, not fatal.

## Offboard circle and arm observations

The real-data circle probe unpins the actual Actor, completes backend loading,
dismounts with ordinary Y input, and rotates only the left stick. The initial
teleport-only fixture was insufficient because it left input pinned; it is not
movement validation.

The corrected 420-tick fixture completes with finite bones and over 350 offboard
frames. Its requested camera-relative world direction and physical yaw remain
the same before and after the gesture integration. It therefore does not prove
that the reported turning hesitation is fixed. Arm diagnostics now compare
world-space animation bones to physical bone frames, rather than comparing a
bone origin to an offset collision volume. In the tested walking circle the
largest sampled hand target error was 0.1661 m (left) / 0.0838 m (right).
No drive strength or turn curve is changed on the basis of these observations.
The user's turning and arm-quality reports remain open.

## Validation record

Artifacts are under the task's `work/` folder, outside game/save directories.

- `gesture113-gesture-corpus`: all 37 original choices, three available-hand
  configurations, 360 ticks each; actual entry/hold/release exit, finite poses.
- `gesture114-actor-catalogs`: all four physics/catalog combinations on the
  actual Actor, 960 ticks; pass.
- `gesture114-marker-after-bail`: actual bail followed by offboard marker;
  preserved world position and completed Actor acknowledgement; pass.
- `gesture114-circle`: corrected 420-tick offboard input fixture; pass within
  the limited assertions described above.
- `gesture113-save-profile`: original copied profile, CAC and unknown-chunk
  preservation; pass. Two non-corpus gesture-format tests also pass.
- `gesture115-editor-save`: original editor filtering, separate choices, save
  and reload; pass.
- `body_set_data_reads_retained_pose_before_fetch_and_matrix_jobs`: retained
  source sampled by SetData, not command admission or later mutation; pass.
- D-pad direction order and bumper suppression unit test; pass.
- `gesture115-app-check.log`: PC app `cargo check`; pass with existing warnings.

No hands-on play test or full strict workspace gate is claimed.


## Follow-up validation and gameplay facts (revisions 116–123)

The banked marker's full 12-case replay, bail-to-offboard return, actual terrain
admission/defer/resume, both original CAS genders, and editor save/reload pass
in the 116/117 runs. Both CAS genders bind all 37 S3 preview selections to the
25 shared body bones; this does not validate finger/face parity.

An airborne grab regression in 116 was fixed in 117: native S2 BlendSpace Eval
ignores doEval, so feeding it a false evaluation to discard inactive movement
could leave AddSQT without a child result. Inactive S2 movement trees now update
their original history with an explicit history-only command sink. Only the
selected movement base and admitted gesture channels emit the actual body ACS
pose. The same eight grab/landing cases pass in 117 and in the pre-integration
48d baseline executable; the failed 116 log is retained.

S3 CharacterGesture now exposes its original start decision to the host. The
same Actor publishes a concrete gesture ID once, Invalid+active while playing,
and Invalid+inactive on completion through MotionGraph/GetPhysUpdateData.
S3 physical output explicitly consumes that same Actor payload before native
ScoreModule/mission conditioning. This fixes a missing mission event path; it
does not replace scoring or alter animation catalog indices.

The original S2 and S3 name tables share 28 semantic gesture names. These events
retain their S2 IDs. The nine S3-only names use the PC event namespace 256+S3 ID,
separate from animation/save catalog IDs. This prevents S3 Wings (35) from
aliasing S2 Invalid (35), and avoids inventing equivalences for e.g. GetAway.
Native mission facts, unique-gesture collection and online facts store integer
IDs; the tutorial name lookup bounds-checks unsupported native names. There is
no change to the online wire format or claim that unmodified peers animate the
S3-only catalog. The namespace's 37 IDs are unique and exclude Invalid.

`gesture122-actor-missions` passes all four physical-solver/catalog combinations
on the retained Actor, including the actual performed-gesture event at local
tick 90: S3 YardSale ->292, native S2 Axe ->34. The earlier 120 run also passes.
The incorrectly filtered `gesture120-actor-catalogs` ran zero tests and is not
validation. `gesture122-app-check.log` passes the PC compile check (18 existing
app warnings); the core test build has 927 existing warnings. A playable EXE
was not rebuilt.

`gesture120-marker-push-away` passes all 12 marker cases with unpinned input;
eight onboard cases settle and then push away. Actual wheel gaps stay near the
real collision surface. `gesture122-marker-held-input` also moves away without
bailing or sticking, but its final stance assertion fails two cases because it
checks the original foot after 100 ticks of explicit right-push input. The
first reset frames retain the requested stance. The new assertion checks that
stance at reset completion, before treating ordinary push animation stance as
a spawn result. The failed log remains available and is not counted as a pass.
A separate relocation fixture alternates the original marker with a nearby
resident surface at least five metres away, samples actual terrain heights, and
exercises pending queries during return. It runs neutral input, independently
of the settled/held-push fixtures. This is fixture placement, not runtime
terrain clearance or body tuning. Revision127 passes all12 cases (63.56s).
The earlier126 fixture drove off a real platform edge; its missing wheel ray
at tick99 was not evidence of marker penetration and its failed log is retained.

`gesture120-small-circle-held` and `gesture120-small-circle-dropped` each run
980 ticks, covering raw radial magnitudes 9000,10000,12000,16000,26000. Y performs
the dismount; LT performs an ordinary drop and the fixture verifies the actual
board-held flag is clear. Bones remain finite. The 122 carried replay adds
physical drive targets, drive mode/strength, and collision recovery weight.
At step 516 the left hand is 0.341 m from its IK-adjusted drive target, while
the right hand error is 0.0235 m. Mode3 (collision-softened arms), partial ragdoll
and repeated recovery-weight resets are observed. This is a physical/target
tracking discrepancy, not evidence that the gesture system fixes arm motion.
Rendered-vs-physical comparison and opt-in contact-report diagnostics are added
for the next replay; no drive strengths or clamps are changed.

The smallest-radius fixture's initial zero-turn interval coincides with the
original IntoStand stop animation; subsequent scalar turn reversals can occur
when the moving camera-relative desired direction crosses the back axis. These
observations do not resolve the user's small-stick complaint. Direct original
TU3 inspection of 82D31578..82D315FC confirms that both movement and turning
multiply by stick magnitude and processed scales. Removing that multiplication
would be a tuning change, not a correction of this native producer.

The user clarified that the wheels stick/jolt when moving, small stick movements
cause the offboard hesitation, and arms are affected both carrying and without
the board. Those physical quality reports remain open until a repeatable cause
and appropriate verification exist. The frozen Test 6 EXE and original save
are preserved; all save-capable runs reported unchanged original-save hashes.


## Follow-up results and arm cause (revisions124–128)

The user withdrew the turning complaint; no turn-curve or stick-response change
is made. They described the remaining arms as floppy, both with and without the
board. No further classification is required from the user.

`gesture124-marker-held-input` passes all12 banked reset cases (88.92s), including
requested stance at native reset completion and subsequent held-push departure.
`gesture126-actor-missions` passes the actual Actor/catalog/fact replay (53.02s).
`gesture126-gesture-reset` passes the original-bank reset/held-D-pad regression
(3.69s). ResetSkaterAnim ends S3 gesture channels and clears its publication while
retaining controller history: an already-held D-pad must not restart on reset.
The126 PC `cargo check` also passes. The127 gesture attribute inventory returns
empty merged physics attributes for all37 gesture channels; it does not prove
hand/finger/face animation stream parity (a different subsystem).

The125 source correction to the side-plane override uses Processed2480 bit0,
as read by original82BD8328..8334. The previous host used bit1. This is a proven
bit mismatch, but the arm-circle fixture has neither bit and is unchanged by it.
The125/127 traces show rendered bone matrices agree with the same actual physical
bones (maximum sampled component difference approximately0.000122m). The hand
error is therefore in physical tracking rather than a detached render root.

The127 DangerZone observation is empty throughout ordinary walking. Investigation
found another native producer of Processed2472 bit0x8000: SkeletonMotion
82BD8CB0..8D2C sets it when the carried animated board is sideways. It is not safe
to infer that this flag necessarily came from a DangerZone graph attribute.
With that flag set, ordinary foot-floor reports were admitted by the original
SkeletonCollision filter, repeatedly resetting recovery weight to zero and
putting the arms into collision-soft mode3.

The missing upstream gate is earlier: GroundPipeline8277BD40..BD7C reads the
assembly/part group table, before the separate group/surface gate8277BD80..BDC4.
The host had implemented the latter (water exclusions) but not the former for
static-world skeleton volumes. The original flags3 table already recovered for
SkaterSkaterCollisionPipeline8277A6C0/8277AF90 excludes biped legs17 and torso18
from world group0. Hands/head5 and walking controller capsules20 remain eligible.
Thus footsteps were reaching the shared solve and softening arms even though
those pairs should have been culled before geometric dispatch.

Revision128 factors that same21x21 table into `physics/collision_groups.rs` and
applies it in the existing skeleton world-volume selection. Actor/board pair
queries continue using the same table; disabled world volumes remain available
for their independent actor/board queries. No drive stiffness, collision
threshold, animation clip, turning input or root displacement is tuned. The
fixed world assembly in this host path is group0. The raw original gate listing
is retained in `work/offboard128-native-world-group-gate.txt` outside the repo;
original source image SHA256 is
`f4aa113eb541bfba03dbc108cf5ab43f58c965b20fa3b82f9c40938a0ad841c4`.
Confidence: high for the restored gate; visual/console parity still requires
runtime comparison. Automated post-change results are recorded below.


### Post-filter validation128

Both980-tick actual-Actor walking replays pass, with carried and dropped board
respectively (55.16s /57.95s). After fixture settling, all1758 sampled arm drive
observations per replay are normal mode0 and recovery weight stays1.0. Carried
hand-to-IK-drive-target maximum is0.01404m, versus0.34064m before this gate;
dropped-board maximum is0.000898m. These are observed regression measurements,
not new runtime thresholds or a console-parity claim. The comparison JSON is
`work/gesture128-arm-comparison.json` and an explicit check confirms both maxima
stay below2cm. Original group/role and table-symmetry tests pass. PC cargo check
passes (26.97s,18 existing app warnings). The changed collider/group files pass
git diff --check. Banked held-input returns and pending-query relocations each
pass all12 cases after the filter (81.80s /61.80s), including both offboard and
onboard requests. Original save and Test6 executable hashes are unchanged.

## Authored finger and expression cues129–130

Body playback alone was not complete gesture integration: each original S3
movement clip also carries LeftHand, RightHand and Face blend-pose streams.
Those are separate from physics attributes; the empty127 attribute inventory
was not evidence that these cues were absent. For example, native Shaka enters
hand pose48 and face cue6, then cycles between authored expression cues.

S3 SkaterLeftHand/RightHand each have62 fixed RAW poses. Both17-bone names and
parents match their actual S2 hand owners exactly. The first61 right-hand frames
are byte-identical. Eleven left-hand frames differ; silently reusing the S2
palette would therefore be incorrect. The original S3 hand banks are now build
inputs embedded in the EXE, with no separate runtime S3 folder.

The original S2 fixed hand poses0..60 remain at their original IDs. The S3
62-frame palettes are appended to those SAME retained ACS owners as61..122;
source hand cue IDs are relocated into that range, within the native7-bit token
lane. S3 body commands retain their original clip sample times and nested
blend/mirror/AddBindPose structure. The shared native clip key/token evaluator
feeds the existing hand/face ACS before extraction and sync. Finger poses add
the actual S2 hand bind pose, preserving the S2 character. Face cues address the
existing S2 character expression palette, rather than replacing its head/face.
Reset clears the queued cue channels along with gesture body state.

The129 actual Actor replay passes all four solver/catalog combinations,
including fixed-palette registration and hands/face ACS sync (51.59s). The first
expanded37x3 component corpus exposed a loader omission: native selection-space
children include G_GSTR_FISTS... as well as B_GSTR... clips. Revision130 indexes
all authored _GSTR_ clip names. The failed129 corpus is retained and not counted
as a pass. Post-fix validation follows below. Editor CAS preview is still the
shared-body retarget described above; it is not a full S3 CAS pipeline.


Post-fix130: all37 gestures x3 available-hand configurations x360 ticks pass
with actual original body poses and cue tokens (14.75s). Both source solver and
native solver/catalog combinations pass through the retained Actor and native
hand/face ACS (51.94s). Six native blend-pose token/clock/pool regression tests
pass. Reset including cue release passes (3.78s); editor separate catalogs,
selection/save/reload passes (5.31s); both original CAS characters accept all37
body previews (8.69s). These automated runs use isolated save copies and preserve
the original save. No console trace or hands-on visual parity is claimed.

The user now requests a new EXE after completion, superseding the earlier
no-rebuild preference. Test7 release packaging includes the source changes and
two embedded original hand banks. No second game is launched while their
existing game is running. The CAS preview remains a shared-body retarget;
gameplay hand/expression cues run on the actual character's existing writers.
