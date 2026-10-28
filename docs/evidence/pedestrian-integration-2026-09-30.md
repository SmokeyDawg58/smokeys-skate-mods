# Pedestrian and security integration — in progress

The active objective remains live pedestrians and security: population, movement,
animation, collision, guard warning/chasing/capture and despawning. It is not yet
ready for a playtest. Compilation/runtime testing remain pending.

Existing owners retained: LivingWorldGrindServices EntityMan<40>, shared entity-ID
allocator, ModelMan/RecipeMan, common entity core, streamed RAIN topology and
WorldPainter, shared Vault and Andale tree machinery. No parallel NPC roster.

Root confirmed Pedestrian constructor82e97748 against raw PPC. Its entity class
is livingworld_entities; core entity_type2. Selected initial navigation is
PhysicsNavigation, not an invented unconditional mesh prerequisite. RefSpec keys:

| Resource | Attribute key | Source |
| --- | --- | --- |
| Animation | 00367b8f33e79c43 | 82e97ef4..82e97f14 |
| Chase | fbc42525e74d946f | 82e981d0..82e981ec |
| Locomotion | 8a973d5659a65f1b | 82e9825c..82e98278 |
| TakeDown | 76ac331fbd5803ff | 82e982d0..82e982ec |

living_world_pedestrian_resources binds these through the same retained Vault;
required missing references remain explicit. Shared stored-reference reader now
reads each eight-byte key without interpreting adjacent pointer-cache bytes as
plain data. New try_ref_spec_instance resolves both full-width class and collection
keys. The immutable retained host Vault substitutes for guest pointer caching.
Shared TextField projection resolves a bounded pointer field through existing
relocation/text handling for the original pedestrian animation remaps.

Memory manager source82e6b070 provides ten40-slot stores, eight guard extras and
one BigBlack extra. living_world_pedestrian_memory retains native LIFO allocator
and explicit factory classification; this is allocation infrastructure, not a
completed live entity constructor.

Native generated Pedestrian AI graph tables exported directly from IDA with
tools/ida/export_pedestrian_graph.py into pedestrian-native-graph-2026-09-30.json:
134 states,59 transitions,201 behaviors,89 conditions,142 attributes,59 indices.
PDB field layouts and ArraySection count/index order verified through tinfo.
Generated Update82956488 orders lists, exits, transition, entries, behaviors.
Declarations are evidence for implementing callbacks, not proof of execution.

Three agents own census/navigation, chase/security and animation.
Security/chase modules and the animation attribute child are registered. Full
concrete entity publication, generated AI execution, motion graph callbacks,
animation pose loading/evaluation, collision and session scheduling are still
required. Never publish stationary or bind-pose stand-ins as working pedestrians.

Syntax parsing passed for root resource/memory and attribute changes plus current
security/chase sources. No compilation, unit tests or game runs performed.

## Continued integration

LivingWorldGrindServices now retains PedestrianMemory beside its existing
pedestrian EntityMan, initializes it before that entity manager, and captures
the original initial pedestrian cvar allowance plus `pedestrians` range key in
the same CensusState that already owns vehicle counts. No population scheduling
or successful spawn is claimed by this constructor change.

Native declarations are also retained as pedestrian_graph_data.rs using the
existing generated declaration types. Read-only range validation of parents,
children, behaviors, transition-index sections and attribute sections passed.
Registered pedestrian_navigation and pedestrian_animation source modules.

Additional source inspection: generated UpdateLists8294fab8 uses nine-state
sets; active behaviors use twenty slots. UpdateEnter82953518 establishes timers
and instances before Begin; UpdateBehaviours82933468 invokes the generated
state-update function. Existing skater-specific runtime cannot simply be used
unchanged, nor can its eight-state limit be silently applied to pedestrians.

Remaining immediate root work: concrete entity construction and shared graph
execution/context. Navigation agent continues native path/behavior owners;
security agent owns typed pedestrian AI state/wants; animation agent continues
the actual pedestrian playback startup and original PRES/SIM hierarchies.

## Generated controller integration

Moved the existing generated Action template selection (not its expressions)
to animation/graphs/generated_graph_selection.rs. Action keeps an alias with
capacity8; pedestrian uses9. Predicate order and selection operations are
unchanged. Pedestrian equivalents independently inspected: GetNextState82934be0,
Evaluate82934390, EvaluateState829344d8, SearchTransitions82934b50 and
FindCommonAncestors82933e20. Existing Action selection tests still target the
same shared implementation but have not been run. Error envelope remains the
existing ActionUpdateError; this does not bind pedestrian predicates to Actor.

pedestrian_graph.rs now owns native current/last state and timer lifecycle plus
sorted20-occurrence lifecycle bookkeeping. It calls required backend operations
for predicates, Create/Begin, End/remove, transition and generated state update.
UpdateExit82934950 was independently checked for descending occurrence order;
UpdateEnter82953518 for numeric state order and timing. The concrete backend
and generated expressions still need implementing before any entity admission.

Existing shared playback loader methods were made crate-visible for the
animation agent's pedestrian loader; no duplicate playback registry is added.
Syntax-only parsing passed for the shared selector, Action facade and pedestrian
controller. No compilation or behavioral parity test has been performed.

## Playback wiring and expression dispatch

SelectedPlayerStartupOwners retains PedestrianPlaybackStartup. After existing
skater presentation initialization/restoration it initializes the pedestrian
SIM/PRES playback against those same database/playback owners; failures have a
distinct PresentationStartupError::Pedestrian variant. Source startup loader
is registered. This is playback admission, not live pedestrian publication.

Pedestrian graph now calls pedestrian_graph_expressions::evaluate itself, then
the backend's individual condition callback. All native switch cases from
EvaluateExpression<1>828fc2d8 and <2>828fd248 translated; separate entry/sustain
tables preserve native short-circuit order. Source decompilations retained in
pedestrian-expression-pre-2026-09-30.txt and sustain counterpart. Native default
true is intentional; missing condition callbacks still return errors. In
particular sustain61/62 adds NOT timer21, sustain64/65/69 uses different distance
conditions, sustain25/39 checks monitored intents, and sustain70 drops IsChasing.

Syntax parsing passed for controller/expressions and selected startup changes.
Concrete entity, individual condition dispatch, behavior callbacks, physics,
navigation and evaluated pose publication remain outstanding. No runtime claim.

## Concrete condition constructors

pedestrian_graph::conditions binds authored HasSpecificWantThatNeedsToBeAddressed,
SimpleTimerExpired and HasMonitoredIntent. Source constructors826f1920/826f1b28/
826f1a28 verified; absent timerName uses original TimedRangeRandom, want and
intentName remain required. MoodMan82ce6d58 provides the sole case-insensitive
15-name want mapper. Direct callbacks826f2968/826f29a8/826f28d8 read active wants,
timer<=0, and actual monitored packet InQueue respectively. Merely having a
monitor map entry is not treated as an active intent. Bind returns None only
to delegate another class to its owner; not as a successful condition result.
Registered living_world_guard_graph, whose concrete service dependencies still
need wiring. Source syntax parsed only; no build/tests/runtime.

## Manager and lifecycle review

LivingWorldGrindServices now retains the original embedded ChaseMan constructor
state after RAIN, matching Manager82721f00. ChaseMan.Init827185f8 registers
AllowLocalChase/DisallowLocalChase messages; those subscriptions are still
outstanding, not claimed by constructing the owner.

Native UpdateTransition82933c30 dispatches only listed hook indices. All59
authored pedestrian transitions have hook count0. Controller now directly
honors that empty-hook case. Entry callbacks split CreateInstance from Begin
so instance creation precedes sorted occurrence insertion and Begin follows it,
matching82953518. Source syntax parsed, no compile/runtime.

Animation agent has source-backed shared PedestrianAnimation<K,W> wrapper;
delegated concrete entity constructor to that agent to compose its exact owner
without guessed generic fields. Root still owns factory/registry/frame wiring.

## Latest integration checkpoint
Registered retained PedestrianConstruction with the crate; this remains partial,
not a published live entity. Verified SuggestVelocity SetSeekSpeed slot168 at
8255df18: direct float assignment, no clamp. Added IsBeingHonkedAt to shared
pedestrian condition binding/evaluation: raw826f1138 reads entity+1a18 AI state,
then+c80, and compares against -1 via addi/cntlzw. Reuses honking_vehicle_id.
NeedsToBeginColliding826eadf8 and IsPedestrianColliding82707690 instead read
bytes8/10 of the actual collision interface result; do not replace these with
AI collision_flags. HasPlugin826ffb08 queries IBehaviourSink virtual result.
Three parallel workstreams stopped early; preserve their current edits. Animation/entity,
navigation adapter and chase iterator work require review before live wiring.
Syntax-only rustfmt parsed conditions/entity; no builds or runtime tests.
Entity navigation owner now retains initial Physics selection and SwitchNavigationMode82e6d7d8: 0/1 select Mesh, 2 Physics, other values preserve selection. Original switches only owner pointer; no copying or resetting settings. Source reviewed and syntax parsed only; no live entity publication yet.

## Intent packet source recovery
No existing pedestrian IntentGen owner found in core. Added pedestrian_intent_packet
using existing FastString30, no duplicate packed-name encoding. Native constructors
82e86eb8/82e8a558, AddAnimIntent82714580 (4), AddPacketStage826eac98 (3),
SetActiveStage826ea5b8; PDB enum states0..5 recovered. Native stage equality
82e873f8 excludes target vector and ground blend; packet equality82e87530
compares stages only. IsPacketComplete82e6dfd8 uses terminal states3..5 or
Active plus actual graph MajorIntentComplete. EvaluateSuggestType82e8ae18
rejects identical tail/full queue before priority handling; Collision blocks all,
DoTakeDown blocks noncollision; Collision/Avoid/ChaseJump or empty queue replace.
Source decision implemented; actual queue ownership and SuggestNextIntent82e93c70,
ClearAllIntents82e8b028, Update82e937b0 remain to connect. Clear preserves
CompleteNormal states, sets other removed packets4/5 and InQueuefalse, destroys
managed packets only, resets base to Idle and optional AI input. Do not skip
these side effects when connecting. Syntax parsed; no compilation/playtest.

## Retained intent queue
Entity construction now initializes IntentQueue after native Intent allocation.
Three embedded unmanaged packets and Idle base follow82e8e3f0. Queue identities
are monotonic host u64 IDs (existing monitor representation), no pointer reuse.
Implemented SuggestNextIntent82e93c70, clear82e8b028 and removal82e86e38;
queue/stage completion prefix82e937b0 retains stage increment/decrement order,
normal-completion pruning and base turn decrement. Managed packets are removed
from the owner map; unmanaged packets retain terminal state and cleared membership.
AI feedback reset independently verified raw82e8b124..140: bools1,0,1,0 then
stage0, with PDB stage enum0/1/2. No motion completion values fabricated.
Regression source cases cover collision interruption/uninterruptibility and full
queue rejection before priority clearing. Syntax parsed only; tests NOT executed.
Remaining: actual UpdatePacket/UpdateBaseLocomotion and motion graph consumers,
nav smoothing/speed mapping fields, factory publication/session integration.

## Motion input / turn selection
PedestrianAnimation now owns actual AI feedback initialized by cPedAnimData
82e8d268 and packed motion intent prefix. SetMotionGraphFromIntentPacket82e87740
copies names/count only; IntentsValid remains separate. Existing schedule retains
the only motion time accumulator. Added DetermineOverrideTurningIntents82e8b168:
priority RightReverse/LeftReverse/Right135/Left135/Right90/Left90/Right45/Left45,
thresholds160/115/70/25, exact radians conversion bits42652ee1. Raw PPC blt/bgt
fallthrough preserves unordered comparisons; success appends secondary name,
disables rotate-to-target, zeros minimum speed, clears locomotion-turn-complete.
UpdatePacket82e8e550 still needs actual nav bearing/modifier-mask producers;
no fabricated defaults or premature live publication. Syntax parsed only.

## Queued packet update and next locomotion boundary
Implemented UpdatePacket82e8e550 in IntentQueue: get explicit/nav direction,
copy active stage, gate on locomotion-turn-complete/use-loco/count1, read turn
mask with active modifiers, call actual rotate-to-target, append/increment on
turn override, then write direction and fixed4.25 max rotation to active stage.
PacketNavigation interface must be implemented from actual nav/modifier owners;
no default service implementation. Syntax parsed only.
GetNavSpeed82e6b950 returns Navigation.GetSeekSpeed iff NavRig.GetMaxRange exceeds
mNavRigDistanceRazor or target mode ExplicitNoSeekNoAstar, else0. GetSeekSpeed
82e6bbb0 matches existing Navigation.speed() (ramped iff ramping enabled).
UpdateBaseLocomotion82e8e7e0 is still REQUIRED, not implemented by packet update.
It gates on turn/transition completion, computes distance razor from jump tuning
or actual velocity length, applies the FIRST active speed-recommending modifier
while accumulating preceding turn masks, then strafe or authored speed mapping.
Full mapping key raw82e8ecc8..ecdc is9c9a4f2cb1cc45de (not decompiler32-bit).
PDB tLocomotionSpeedIntentMapping size32: Velocity+0, MajorIntent text+4,
MinorIntent text+8, five AllowableTurnIntentValues text pointers+12..28.
Select first mapping whose Velocity is >= nav speed; preserve original order,
previous mapping and invalid-name termination. Reuse Vault relocation projection;
do not plain-read pointer-bearing records or hardcode walk/run thresholds.

## Authored speed mapping reader
Read-only original skater.vlt/bin from TEMP/skate-ped-vault confirms class
53e332f67b8124b2 definition9c9a4f2cb1cc45de typefdff00a501b1e0f7,
size32,max8,flags5. Added typed mapping reader to existing pedestrian animation
attribute module, preserving authored element order and seven relocated strings.
Extended shared typed array projection with TextField rather than duplicate
array parser; existing numeric array_range path remains unchanged. Decoder
returns actual velocity/major/minor/five previous-intent values. No hardcoded
thresholds. Syntax parsed only; source not compiled, reader corpus runtime not
executed. Locomotion selection and original invalid-name termination still need
connection; null pointers currently explicit errors, not invented names.

## Speed selection connected to base packet
Read-only full mapping corpus audit found five distinct arrays; all seven text
pointers per record resolve, including legitimate empty MinorIntent and turn-list
terminators. Native invalid-name82395367 is NUL (not the adjacent string shown
misleadingly by decompiler). Initial previous mapping8239d170 MajorIntent points
to FromPacket8239d164. IntentQueue now preserves previous major, selects first
native ordered threshold, writes major AND empty-or-nonempty minor, scans five
allowed-previous entries with empty terminator, updates previous major only when
mapping selected. No match preserves existing packet. Raw fcmpu/ble82e8ed3c..40
branches on not-greater, including unordered; preserved rather than Rust <=.
Original arrays have differing Walk/Brisk/ChaseRun thresholds; none hardcoded.
Syntax parsed only. Full base locomotion/modifier/navrig and live frame remain.

## Full intent orchestration
Added UpdateBaseLocomotion82e8e7e0 control flow: completion gates, source-ordered
navigation queries, first active speed recommendation stops modifier scan,
strafe branch with copied pre-rotation stage and maskcc, authored mapping,
optional turn stage, final ForceStop. SetBaseMinLinearVelocity82e6ac30 writes
active base stage only. BaseNavigation remains required concrete owner interface,
not a permissive fake implementation. IntentQueue.update now connects retained
PedestrianAnimation AI feedback -> queue advance -> packet/base update ->
SetMotionGraphFromIntentPacket, and sets IntentsValid at original start position.
Still not invoked by live frame until actual NavRig/modifier implementation and
motion graph owner are ready. Syntax parsed only; no compilation or tests run.

## Concrete NavRig distance operations
No existing Pathogen NavRig distance owner found (vehicle validator is different).
Added distance child under pedestrian_navigation: original seven volume/physics
rays, max range, direct physics write, closest-volume write and GetNavSpeed gate.
GetMaxRange82e4d610 raw fsubs/fsel ordering preserved rather than Rust min/max;
path distance is NOT included. HasToJump82e4d480 retains caller outputs: enabled
test always writes height, strict forward/up/down bounds, ledge only on success.
Raw82e4d494..520 independently checked. Cvar jump thresholds are required inputs,
not host constants. No Default or synthetic clear rays; actual query producer,
GetBestNavRay and transforms remain to implement. Syntax parsed only.

## Closest ray selection
Added GetClosestNavRay82e4d8b8 using existing ReckoningNumeric.dot3. Preserves
forward index0 start, lazy evaluated seven-ray dot cache, predecessor-before-
successor updates, strict greater comparisons/tie retention and native neighboring
ray direction (CounterClockwise=-1, Clockwise=1 verified PDB). Adjacent ray
matches GetNavRigDistance closest-direction wrap. No global-max shortcut.
Source shows GetNavRigDistance82e518d0 uses existing CustomCollision Line_Line_Test
between clearance endpoints and normalized requested direction; that connection
remains missing. TransformRig82e51f38 resets volume distances from actual cvar,
caches physics based on dot>.98, displacement threshold and counter>40, then
transforms original initialized rotation matrices. Do not rebuild with arbitrary
ray angles or assume vehicle four-ray validator is the same owner.
Syntax parsed only, no compilation/runtime tests.

## Heading clearance interpolation
Found existing Line_Line_Test arithmetic inside Pathogen NavPath; extracted
unchanged parameter calculation into shared line_line_parameters and reused in
NavRig.GetNavRigDistance82e518d0. No second intersection implementation. Builds
fused ray endpoints, NormalizeXZSafe edge (Y/W explicitly0 verified82de2290),
flattens first endpoint to positionY, normalizes requested direction, consumes
second line parameter and returns0 unless positive. Original ignores boolean
parallel classification; preserve IEEE parameter arithmetic. Closest index and
adjacent direction are returned for GetBestNavRay. Syntax parsed only; no builds.

## Ray direction initialization
InitializeRig82e52278 source establishes circular angles[0,-11,-55,-125,125,55,11]
and front-group membership[true,true,false,false,false,false,true]. Added matching
rotation pairs through existing XMVectorSinCos polynomial (no libc substitute).
TransformRig direction portion reuses NormalizeSafe and original fused matrix
multiply order; forward uses full heading, remaining six use original heading
then flattenY/W before normalization. Raw82e524d8/dc verifies pCos/pSin outputs;
82e52520..34 verifies negative-sine X-axis Z and positive-sine Z-axis X.
Full TransformRig cache/test lifecycle and original physics query producers are
still required; these functions alone do not publish a ready live NavRig.
Syntax parsed only; no build/runtime test executed.

## NavRig volume batch result flow
PDB tNavRigTest128-byte layout recovered: starts0/48, headings16/64,
magnitudes32/80, result distances96/100, parallel score104, owner108, ray112.
ProcessBatch82e4dbf8 reuses Line_Line_Test arithmetic including denominator minus
bits3089705f. Shared helper now exposes this score without duplicate intersection
math. Added request processing and UpdateNavRigWithBatchResults82e52560 reduction:
four inclusive segment tests via fsel (sum vs3.5), outside candidate1000,
closest distance combine, parallel-score selection, only-shortening volume write.
Raw82e5263c..6ac verified operands/constants1,0,3.5,1000. Actual shared batch
owner/range submission and physics cache remain to connect. Syntax parsed only.

## Shared validator ownership
Replaced MeshMan's three constructor-only validator placeholder fields with one
Mutex<NavRigBatch>; no parallel second validator. Constructor82ce7590 remains
unallocated count/capacity0, existing constructor test updated. Native ordered
AddNavRigTest82e4dac0, ProcessBatch82e4dbf8 and DeInitializeBatch82e4dd60
represented; applying ranges requires completed results and uses original reduce.
Host initialize requires caller-supplied capacity (no guessed production capacity);
original frame allocation call and scheduling remain to trace/connect. Existing
empty-world behavior unchanged, no pedestrian publication enabled. Syntax parsed
only; no compilation/runtime/tests executed.
# Navigation boundary batch integration follow-up

Root collision construction integration: new pedestrian_collision.rs retains
the native82e720c0 owner in PedestrianConstruction; create_collision reserves
existing MemStore2 as82e78718. InitBodies82e99df0 reuses CharacterCollisionAssembly
and CharacterAssemblyCandidate: group11 world body first, group13 skater body
second, shared authored definitions, world mass then main/arm/arm masses from
FillMasses82e97260/72e8. Material callbacks vtable823a4268+120/+124 both noop
825187c0, so native initialized zero material retained. Constructor collection
full key5b6a0051dd6a295c verified insrdi82e720fc. Candidates remain owned on
partial failure; this is not Island admission or entity publication.
Registered the agents' new pedestrian_ground_dmo_batches and
pedestrian_motion_delegate modules; concrete source callbacks and scheduling
remain to wire. Syntax parsed only, no compilation or runtime tests.

DMO avoidance now reads the existing DmoObjectMan roster, samples original
physics_skeleton/default small-object mass threshold once before traversal,
filters hand props and mass, and reuses model.bbox_transform/dimensions*.5.
Caller supplies the common masked-volume owner identity. DMO transform mode
and hand-prop cull timer are now retained at construction, with Physics/zero
defaults from82d3de14/34. FillPriorityIndex now includes actual hand-prop mode.
No hand-prop setter was added without its collision side effects.

CORRECTION to earlier decompiler-layout note: IsSetAsHandProp82d2ad08 receives
IDynamicObject at complete-object+140(8c); its relative+1080 is complete+110c,
PDB mTransformType, NOT mMonitoringEntities. Enum values recovered from PDB:
Physics0,HandProp1,Invalid-1. Ctor writes0 at complete+110c and0float at+10f8.
Set/Unset use the same interface adjustment. This resolves the apparent layout
conflict. Rustfmt edition2024 syntax parsing passes; no compilation/runtime.

Parallel work resumed successfully: ped_execution_audit now implements the
missing generated pedestrian motion controller from82922408/829562e8;
ped_ground_dmo owns new ground-DMO batch source; ped_adjust_owners owns
PostAdjust/HeadTracking/SpatialAdjust constructor owners. Root integrates
shared files.

Pedestrian avoidance bounds now use model dimensions with Y*.5; non-wipeout
also scales X/Z*.6, measures horizontal velocity, caps speed*scalar, adds half
the capped shift to Z extent and shifts centre along forward, with Y lifted by
half height. Original cvars82fa9c48/9ca8 default.1 and1, registered with original
labels. IsWipedOut is ITargetable vtable823a4d50+24 ->82e6eba8; explicit caller
input remains until the live producer is connected. Raw82e8522c writes the
wiped-out lifted-centre calculation to sp+120 temporary, not the submitted
transform atsp+d0 (translation atsp+100); implementation preserves that binary
behavior instead of silently fixing it. Existing host min/length policies reused;
exceptional VMX payload parity is not claimed. Syntax parsed only.

Hand-prop investigation: raw IsSetAsHandProp82d2ad08 compares object+1080 to1.
SetAsHandProp82d2ad20 also sets collision byte+12 and body collision group15;
Unset82d2ad88 clears state/byte, chooses group14 when mass>=5.5 otherwise16,
and resets kHandPropCullTimer. Cannot implement as an isolated boolean setter.
PDB mislabels the accessed storage as mMonitoringEntities, so raw/native layout
must win. Existing skeleton evidence identifies avoidance mass key as full
9ab57b38d5bb9449; actual DMO owner/hand-prop publication remains to connect.

Avoidance population follow-up: vehicle AddEntityVolumes82d22c08 now reads the
existing live EntityMan in its retained native order. It uses actual core
transform (GetTransform82d1cef8 reads core+16), actual model dimensions*.5,
canonical affine W lanes, rotated local half-height offset, and NULL mask
(not vehicle ID). PedestrianVolumeBatches exposes this same service, with no
new vehicle roster. Source syntax parsed, not compiled/tested.

Traced remaining population sources: pedestrian82e84f20, DMO82d312f8.
DMO eligibility vtable IDynamicObject8232cfa8+96 is IsSetAsHandProp82d2ad08,
NOT IsStatic(+80); +124 is GetMass82d2d3b0. It rejects hand props then tests
authored mass against physics_skeleton_default attribute low wordd5bb9449
(decompiler signed-709127095; full key still to recover), then uses existing
model bbox_transform and dimensions*.5 with object owner mask. Hand-prop mode
storage is not yet represented on DynamicObject and must be traced before
publishing that source; no fake false property added. Pedestrian source includes
state-dependent extent/velocity expansion; its source is traced but not wired.

Pedestrian volume buffers now retain two concrete VolumeLineBatch instances and
their request associations. Raw ctor82e8a6d4/d8 and82e8a720/724 confirms null
mask0 and capacity80 for both. Create82e868d8 and Process82e72940 reused the
existing NavRig request/hit-distance code; entity wrapper reads current core
position and does not apply the ground cache gate. Skater volume helper uses
original half extents(.5,1,.5) and world Y offset.9. Native AddNavRigTest82e6db10
stores GetID() separately from the entity pointer owner mask: the adapter
requires an explicit common owner token rather than conflating pedestrian ID
with collision identity. Registry adds original volume fatness.45, Y clearance
.25, and enabled=true from initializers82fa95d8/9698/9998, retaining shared
start offset. The execution/session still must populate entity/world volumes,
call these buffers in native frame order, and distribute to the sole roster.
Syntax parsing passed; no compilation/tests/runtime performed.

Volume collision continuation: collision_volume_line now implements827c05d8's
owner/ignore filtering, degenerate-segment gate, initial-overlap escape policy,
strict nearest-hit selection and early zero-parameter exit. Original thresholds
read at821043a8/ac are37800000 and34000000. Reverse-hit positions use the same
inverse-transform operation ordering already present in grab geometry. Same-side
local XYZ products accept escaping; the fallback tests horizontal centre/motion
dot strictly negative. Result owner/surface are NOT replaced by volumeMask.

Added collision_box_line for82ad7170's six-region-transition traversal, shared
sphere corner helper, cylindrical edges and face planes. This is the specialized
identity-local BoxVolume constructed by VolumeLineBatch with external transform;
it does not claim to replace all formats collision primitives. Raw VMX operand
decoding confirmed edge dot products at76a4/76dc/76f0 are offset²,
offset·travel,travel². Raw scalar7700..7744 retains two fused discriminant
operations;78e8..78f4 retains cylinder/boundary ordering;7b18/7ca4 retain
fused total fraction accumulation. Raw78b0..78c4 and7aa0..7ab4 copy an actual
fraction pair, correcting nonsensical Hex-Rays COERCE_FLOAT(1) output.
World hit position subtracts normal*queryFatness at7e58..7e60. Existing numeric
estimate/refinement backends reused; no claim of Xenon estimate-bit identity.
VolumeLineBatch::end now invokes this implementation. Regression cases added
for overlap escape/crossing, masks/ties, horizontal fallback, fat face contact,
rounded-corner slab false positives and initial interior normal selection.
Syntax parsing passed; compilation/tests were NOT run. Pedestrian double-buffer
volume population and live entity/session integration remain to connect.

Shared sphere intersection follow-up: traced rwcSphereLineSegIntersect82acdef0
in IDA pseudocode and raw instructions. The existing DMO grab provider already
contained its boolean decision graph. Extracted that graph to
collision_sphere_line for reuse by rounded-box corner probes, preserving the
existing numeric dot backend and cross_full4 operation ordering. Native -1
(not approaching) remains distinct from0 (miss); accepted hits retain the
numerator/denominator fraction, with hardware-style single-precision sqrt
deferred for boolean-only consumers. Raw branch words40980024/4098000c test
LT clear, and40990010 tests GT clear: retained strict comparisons rather than
rewriting them to >=/<= with different unordered behavior. Grab queries now
use this one shared implementation. Rounded-box traversal and VolumeLineBatch
processing still remain incomplete. Rustfmt syntax parsing only; no compilation
or runtime tests performed.

Started actual VolumeLineBatch data admission under shared collision module:
ctor827c0ac8, AddVolume82d16060 hard capacity512 with direct transform/dimensions,
AddTest827bdaf0 capacity, null-mask comparisons, result defaults. Owner and
ignore masks are equality exclusions in native order827c0694..06c8. Not a
terrain batch alias. Query827c05d8 uses BoxVolume::LineSegIntersect82ad7170,
reverse test for near-zero forward parameter, local sign/escape policy, nearest
strict parameter selection, early exit nearzero. Full box/escape math remains
unimplemented; no End/process or result API pretends these are completed misses.
Syntax-only verification; no compile/runtime tests.

Added AddTestsToVolumeBatch82e79478 request generation: native enabled bit only,
all7 rays, fixed distance10, distinct volume fatness/Y clearance, shared forward
start offset and request initialization. No ground cache/group gate. PDB enum
confirms ground0/down1/volume2/jump3/positioning4/taze5. Original volume snapshot
order82e868d8 is pedestrians, vehicles, DMOs, skaters, world-avoid volumes;
skater half-extents[0.5,1,0.5], position offsetY0.9; ignored owners passed through
AddTest. This needs genuine VolumeLineBatch backend, not terrain tests with
different flags. ProcessVolume82e72940 shares the existing hit-copy/length flow.
Request generation syntax parsed; backend/session integration remains unfinished.

PedestrianGroundBatches now retains both ground buffers and ten actual World
line-group leases (five BF_Ground each), with per-entity request construction,
submission, End, swap, Sync, and ground hit publication. This uses the existing
static collision query backend. Ctor82e7c1e0 send=B/receive=A; UpdateCurrentFrame
82e96ef8 swaps before Process/Receive and only then evaluates modifiers/intents.
Native CreateGround82e72320 stops visiting later entities after AddTest rejection.
Result distribution remains for the actual EntityMan roster integration; optional
spatial-instance processing is still absent (native default disabled). No session
instantiation yet. Syntax parsed only; not compiled or runtime tested.

Shared WorldLineGroup added around existing SimulationWorldLineQueries leases,
no new geometry/query backend. Verified AddTest827bc248: on full child End it,
advance, Begin next, carry cumulative count, retry same input; GetResult827bc370
selects first cumulative offset > index and subtracts prior offset. Begin82e9d878
starts only first child; End82e9d8d8 ends only current; Sync827b84e8 all children.
Owner retains partial acquisitions on error. Not yet instantiated by execution.
SpatialInstanceManager LineTest82dfde58 selects nearest intersection in the
FIRST intersecting spatial map, not nearest across maps. Original initializer
82fa99e8 disables kPhysicsDoSpatialInstanceTests by default. Optional branch
still needs real spatial-map query implementation; do not force-enable it.
Syntax-only verification; no compilation/runtime tests.

ProcessGroundLineTestBatch82e72478 synchronizes receiving batch, copies actual
GetResult hit position/normal/surface/owner, then recomputes native length from
start to hit (not line fraction). Added this publication prefix and retained
normal/owner fields; miss defaults remain endpoint/up-normal/zero IDs. Existing
collision_query_batches and live grind batch bridge provide reusable backend
machinery. Remaining spatial-instance branch applies only type0 and requires
actual SpatialInstanceManager query; closer hits use XMVectorPow(squared,0.5)
and clear surface/owner. Not implemented as a silent miss or sqrt substitute.
No backend publication/scheduling claimed yet. Syntax-only verification.

Probe cvars registered in shared registry1042..1049 (total1050), verified
initializers82fa3af8/9458/94b8/9518/9578/9638/98a8/9a38. Defaults: distance10,
fatness0.3, Y clearance0.3, forward offset0.05, down offset1, down start height0,
down height2, physics tests enabled. Typed probe capture and entity wrapper now
use existing cvar/core/id owners. Ignore-volume and enabled flag remain explicit
until the entity flag producers are connected. No physics backend substituted.
Syntax-only checks; compilation/runtime not performed.

Added retained front/back test-group state and ground request construction
from AddTestsToGroundBatch82e78818 and helpers82e6db10/82e6dd58. Enabled and
cache gates precede all work; forward rays then downward rays, authored group
membership, shared capacity checked per entry, group toggles even when full.
Forward start=adjusted+ray*startOffset, end=adjusted+ray*physicsDistance;
down start=adjusted+ray*downOffset+Y*downStartHeight, end=start-Y*downHeight.
Both retain original FMA order. Initialized miss position=end and distance=
native length(end-start). Submitted starts false until backend AddTest accepts.
Physics distance comes from kNavRigPhysicsDistance82e4d718, distinct from path
range. Probe tuning capture and actual physics batch backend remain unfinished.
Syntax parsed only, no compile/runtime tests.

Physics receive projection added for ground82e72850 and volume82e72a60.
PDB LWPedestrianLineTestBatchEntry offsets: start16,end32,hitPosition48,
normal64,distance80,surface84,hitOwner88,fatness92,type96,pedId100,rig104,
ray108,owner112,ignoreVolume116,submitted120. Ground type0 assigns distance;
type1 forces zero if surface&0xf80==0x300 or any abs(end-hitPosition) exceeds
EPSILON_192 bits34000000 (823953d0). Other types ignored. Volume shortens only.
Submitted is NOT a hit flag: request initialization copies end to hitPosition
and line length to hitDistance (AddNavRigDownTest82e6dd58). Misses must retain
these defaults. Old synchronous ResolveNavRigWithPhysicsTests82e58d48 starts
with unconditional FailAssert, so do not connect it as a substitute for the
live double-buffered query flow. Real batch creation/submission remains open.

PedestrianConstruction now retains one optional NavRig and reserves native
store3 through the existing allocation ledger. Added actual entity transform,
path validation, batch distribution, topology wrappers. Transform normalizes
core Z before TransformRig, matching82e6d848. Validation brackets batch indices
even if gated off; active MeshNavigation dispatches immediate/batch source
paths. PhysicsNavigation vtable823a4390 slots104/108 both resolve825187c0, raw
single `blr`, so those path methods correctly do nothing. Enabled bit6724/80
remains an explicit caller input until its live producer is integrated.
No extra roster/update loop. These methods are not yet live scheduled and
initialize_nav_rig must be called at the original post-adjust constructor stage.
Syntax parsed only; no compilation or runtime verification.

Registered seven rig controls in the existing InputCvarReferences registry
(1035..1041, count1042) and typed try_pedestrian_rig_tuning capture. Verified
all native constructors: range10 (82fa3a98), cache distance0.5 (82fa3b58),
caching true (82fa3e18), path validation true (82fa9948), topology angle45
(82fa9b88), topology distance2.5 (82fa9be8), batching false (82fa9798).
Uses existing cvar override flow; no parallel tuning owner. Syntax parsed only.
Pedestrian entity rig field/allocation and update wrappers still need connection.

Completed topology slope direction pass82e589a0..8b28 in NavRig. Raw instructions
verify maxAngle degrees*bits3c8efa35; horizontal component abs gate uses
bits34000000; zero horizontal vector leaves prior ray unchanged. Normalize
endpoint-position, length(XZ), native ACos, then accept if limit>=angle or
clamp to XZ*(cos(limit)/length)+Y*(sign(normalizedY)*sin(limit)). Reuses existing
numeric helpers and reciprocal refinement, preserves FMA final combination.
All seven endpoint samples precede direction writes. Syntax-only validation;
live ownership, tuning publication and execution scheduling remain unfinished.

Topology82e58720 endpoint sampling now uses the existing MeshStore/cell owner:
seven endpoints first, min(clearance,maxDistance) through native fsel order,
GetMeshCell tolerance f32::MAX, then triangle plane height when valid. Shared
ComputeBarycentricY82e54878 translation normalizes cross(p1-p0,p2-p0), tests
abs(dot(normal,Y)) > EPSILON_190 bits34000000 (read8238b0c0), uses existing
reciprocal refinement and FMA plane evaluation. Missing/degenerate cells leave
endpoint Y untouched. No duplicate asset parser/query owner. The following
vertical-angle clamp pass still requires raw VMX review and is NOT connected
yet. Syntax parsed only, no build/runtime tests.

Retained NavRig transform state added from ctor82e51870 and TransformRig82e51f38.
Ctor explicitly zeros volume/physics arrays; remaining initial fields come
from cleared MemStore storage. Transform resets only volume distances to the
supplied actual cvar range, retains physics clearance, gates caching on old
forward dot new heading >0.98 and squared displacement minus squared cache
distance <0, and invalidates every stale counter >40. It resets batch indices
and reuses existing ray transformation math. Native does NOT write mPosition
here; its topology/query publication still needs integration. PDB offsets:
position16, directions32, volume144, physics172, path200, jump228, mesh240,
group304, batch308/312, stale316, reuse320, jump-enable321. This partial rig
owner is not yet allocated/published by the pedestrian constructor.

Important default-path correction: initializer82fa9798 sets
Pedestrian_Pathogen_NavRig/kBatchNavRigLineTests FALSE. Pedestrian82e6d8d8
checks enabled flag at byte6724 bit80, then selects immediate virtual104 or
batch virtual108. Do not force batching or invent its allocation capacity.
MeshNavigation::ValidateNavRig82e6c1a0 skips when using off-mesh navigation;
its batch counterpart82e6c2a8 does not. Both wrappers now exist.
Immediate ResolveNavRigWithinPathBounds82e5b780 walks all seven rays, generates
the endpoint from current clearance, and uses LineIntersectsEdgeList82e58f00.
Refactored the existing shared intersection function to expose its first-hit
distance while preserving boolean callers. Native returns the FIRST qualifying
authored edge (not global nearest); only lower volume clearance is written.
Implemented this default path without duplicating intersection math. Syntax
parsed only, no build or playtest. Live rig ownership/scheduling remains open.

FrameBegin82ce6c28 loads validator at MeshMan+12 and tail-calls824a6c20.
The folded symbol misleadingly names Fesl HeapLinkedList Dealloc; raw code is
`li r11,0; stw r11,8(r3); blr`. This resets mNumBatches only, retaining capacity
and storage. Added the corresponding shared batch and MeshMan frame methods;
live scheduling still needs connection. No deallocation/reallocation per frame.

IDA `TargetProvider::AddNavRigPathBoundsLineTestsToBatch` (82e52a88)
submits seven rays outermost, path edges innermost. Each test uses the original
edge first point, authored unit direction and length, and the rig's combined
volume/physics clearance. Unlike the separate path intersection query it does
not flatten edge Y. `NavPath::add_nav_rig_tests` now implements this through
the existing weak tile handles and shared `NavRigBatch`; expired tiles fail
explicitly. It returns the submitted range for later result distribution.

Execution `Init` (82e9dc50) initializes double-buffered physics batches (five
ground batches on each side, one DMO batch on each side, volume batch pair),
but does NOT allocate the Pathogen validator batch. Allocation remains to trace.
PreFrame82e96fc0 orders positional data, graphs, pre/seek/AStar/navigation/post,
rig transform, validation, validator processing, distribution, rig orientation,
then ground/volume/DMO physics batch creation. Live scheduling remains unfinished.
Source syntax parsed only; no compilation or runtime verification.

Root integration follow-up: registered pedestrian_adjust_owners; PedestrianConstruction now retains PostAdjust/HeadTracking/SpatialAdjust and allocates stores7/8/9 in order through initialize_adjust_owners, preserving PostAdjust before fallible authored bone reads. Registered NavRig mesh child and retained mesh_info initialized by its recovered constructor. These remain construction pieces; live factory/scheduler publication is incomplete. Rustfmt edition2024 syntax parsing passed for four changed/integrated files; no compilation or runtime checks.

Ground-result integration: registered entity child pedestrian_ground_dmo_receive, applying jump distances, SpatialAdjust ground-plane results and corrected taze clear-line flag. Root independently read Task_UpdatePositionalData82e72fd8: mesh lookup/SetPathogenCell precedes shared positional callback. Also decompiled common collision ctor82e571e8 and retained explicit required-to-begin, forced-recovery, colliding, colliding-with-skater and recovery-count fields on PedestrianCollision. Actual admission and flag updates remain under implementation; no fabricated collisions. Syntax parsing passed for receiver/entity/collision; no compile/test/runtime.

Execution-order audit: root directly decompiled UpdateCurrentFrame82e96ef8 and verified ground/volume/default buffers ALL swap before any processing or receive. Split previous combined swap_and_process APIs into swap_buffers/process_received for all three owners; no existing callers required migration (repository search). This permits exact native ordering including retained state on processing errors. Native then fills current targets, evaluates modifiers and updates intents. Syntax parsing passed; scheduler integration and live execution remain incomplete.

IMPORTANT pool architecture correction (root direct IDA audit): Island.InitWorld827b42c0 constructs pedestrian EA::Physics::AssemblyPool, NOT Sk8::Physics::DynamicPool. PDB Params fields maxAssemblies+0/createSerialisedAssemblies+4/numAssembliesPerJob+8/updateInParallel+12; ctor82e0ad90 defaults0/true/1/false. Raw827b4378 writes148 to pedestrian params base stack+90 for flags&1, else827b4388 writes16; decompiler falsely labels this assembliesPerJob. Parallel comes from actual kPedestrianPoolUpdateInParallel. AddPedestrianBody827b68e8 inserts into mPedestrianPool, shares Island IDs/lists. Therefore do NOT clone vehicle DynamicPool or expose pedestrian world-line mesh storage: reuse shared serialized assembly attach/detach/update via a distinct base-pool owner. Shared handle/admission API remains unfinished. Ground/skater-ped/ped-ped/DMO-ped/ped-vehicle pipelines are distinct native constructor routes and still need integration.

Base pool implementation: character::pedestrian_pool::PedestrianAssemblyPool now owns distinct148/16-capacity serialized character membership. Shared CharacterAssemblyCandidate.attach_serialised extracted from existing vehicle insertion, preserving shared world-index allocation, Parts attach and CharacterCollisionPublication creation. New pool has no DynamicPool line collection or mesh slots; candidate handle remains existing opaque stable character identity. Constructor validates manager/backend and refuses unsupported parallel true; insert/remove retain checked partial failure semantics. Island admission, update scheduling, publications refresh and collision pipelines remain incomplete. Rustfmt syntax parse passed; no compilation/runtime verification.

Session ownership: SimulationAssemblyConstraintSession now retains optional distinct pedestrian base pool and explicit initializer consuming actual island flags/live parallel setting. Caller wiring remains outstanding. Motion integration: directly verified CompleteLocomotionTransition82e6e030 and CompleteLocomotionTurn82e6e048; PedestrianAnimation now exposes exact AI flag/count mutations, including turn packet decrement and count-only retain-first preserving names. Agent callbacks can reuse these existing owners. Three edited files syntax parsed; no compilation/tests/runtime.

Base refresh: root directly decompiled AssemblyPool.BeginUpdate82e0dc98 and EndUpdate82e0bb10. Added pedestrian try_refresh using shared CharacterCollisionPublication Update/bounds primitives in all-update/all-bounds/all-header/live-state-active-list order. Shared publication now exposes fresh all-Part frozen scan (not cached serialized frozen state). Pool retains active ordinals/frame and invalidates failed refresh/admission/removal. Host refresh executes synchronously, no background update handle; unsupported parallel remains refused. Island integration delegated bounded shared files to ground agent. Source parsed only, live caller and full collision pipeline still outstanding.

Live startup integration: selected_session_world_startup::try_new (production caller selected_session_boot_owner.rs) now initializes the separate pedestrian pool before the skater pool using actual registered try_pedestrian_pool_update_in_parallel (native defaultfalse), preserving explicit init errors. Root verified LockBaseLocomotion82e75068 against PDB cIntentGenPacket stages+16 / stage RotateToTarget+100 / MinLinearVelocity+136; existing IntentQueue now mutates precisely the active base stage. Syntax parsing passed; no compile/runtime test. Pool frame scheduling, spawn publication and complete graph backend still pending.

Frame publication integration: directly decompiled Step_UpdatePrePhysics2 (entry827b28d0/body28f0): DMO EndUpdate/jobs precede pedestrian EndUpdate827b298c then vehicle EndUpdate827b29a0. Existing serial host authored-ground frame now refreshes retained pedestrian pool at the same preparation seam before vehicle refresh, with explicit PedestrianUpdate errors. Actual NPC ground/pair collision jobs are still missing; this change publishes only the correct retained collision volumes/bounds/active state. Syntax parsing passed; no compilation/runtime.

Motion ownership integration: PedestrianAnimation now retains the recovered pedestrian delegate and declaration objects, with explicit load_pedestrian_motion after authored selection; load binds objects once and uses native call_behaviours=true. Does not silently select this graph for arbitrary authored XML. Entity jump child registered, using actual chase MaxChaseSpeed getter. Concrete motion Services/live factory load and frame invocation still pending. Syntax parse only; no compile/runtime verification.

Root Island review: inspected common borrowed pool adapter, candidate admission, shared ID/list/spy registration, remove path and contact-owner dispatch. Independently decompiled CleanUpPedestrianBody827b6c88; ascending Part spy removals -> contact erase -> base AssemblyPool.Remove matches new pedestrian arm. Added base pool publications/len/active getters for collision jobs, without exposing a duplicate roster. Syntax parsed seven Island/pool/caller files. No type-check or runtime claim. Collision job integration delegated next; root proceeds entity execution/factory work.

Collision settings wired: root decompiled initializers82f69fd8,82f6a120,82f6a260,82f6a300,82f6a3a0,82f6a3f0. Added exact section/label/title/default booleans (drawfalse/runtrue), appended1070..1075 without changing earlier indexes, and live getters. SelectedLivingWorldDmo now samples all six into DmoFrameSettings for job integration. Cvar array1076. Syntax parsed three files; no compilation/runtime verification.

Ground-job provenance integration: added sealed output.offset_assembly_ordinals so native pedestrian chunks of five retain full-pool assembly ordinals without exposing/rebuilding contact records. Collision agent consumes it at live job boundary. Roster audit still finds sole EntityMan40 payload() and populated-roster pending paths in vehicle horn/spawn and RAIN invalidation; these must be migrated to actual completed pedestrian payload and shared spatial lookup before publication. Do not add a second pedestrian collection or bypass these consumers. Syntax parse passed for output helper; no compilation/runtime.

Shared animation reuse: root directly decompiled Animatable.Transitioning82d720f0 and extracted existing skater expiry transition-list scan into core.try_is_transitioning. Existing skater adapter now calls the same primitive preserving missing-owner/foreign-list diagnostics; pedestrian live adapter can reuse it without duplicating animation logic. Native empty-list early return before base lookup and nonempty membership assertion retained. Syntax parse passed; no compile/tests/runtime.

Pedestrian retains rain_data Option<living_world_rain::spatial::RainItem> for existing network ownership and pending roster migration, initialized absent until producer. Root independently read UpdateAnimationData82e9a078: clears actual graph intent map and copies current MotionGraphInput prefix to FastString36 values1; BuildAnimTree is not the producer. Notified live-adapter agent to retain snapshot at native phase and reuse existing ActorGraphIntents fixed map instead of reading mutable input on demand. Entity source syntax parsed only. Spawn factory and schedule still incomplete.

Registered NavigationModifiers and retained it on entity using existing IntentQueue packet allocation. NavRig initialization now initializes modifier records with actual mesh-navigation owner and resets enable flags from actual active navigation parameters; no second navigation owner. Registered populated roster child after peer file creation. Source syntax parsed; full modifier evaluation and factory publication still pending.

Continuation: retained native motion intent snapshot and probe predicate.
Direct IDA82e9a078 confirms UpdateAnimationData clears the shared Intents map
and copies current packed names with value1. PedestrianAnimation now retains
that existing map at graph load and exposes the phase capture; scheduler caller
still required. Constructor82e867a0 confirms CycleSwappingPercentage starts0;
retained on motion input for live adapter, no separate value owner.
Ground receive raw82e728f0..2910 extracts CR6.EQ after vector comparison:
all-false, not any-true. Corrected inverted condition so water OR no endpoint
separation zeros probe clearance. Existing input/payload/owner flow unchanged.
Animation syntax parsed only; no build/runtime. NPC execution still incomplete.

Query delivery and native channel cancellation continuation:
Added pedestrian_execution_queries.rs over the three existing buffer types and
sole typed EntityMan40. Direct UpdateCurrentFrame82e96ef8 verifies all three
swaps precede ground Process/Receive, then volume, then ground/DMO. Missing
raw-rig owners remain errors; positioning/taze absent-ID lookup skips match the
native receiver. Module registered; live session scheduling still required.
Shared core EndAllChannelsPrematurely82d74770 now traverses existing native
channel dictionary, reusing the named overload zero-time gate/length/flags.
Channels remain until their ordinary expiry; no immediate removal. Pedestrian
animation retains the agent channel-request owner. Syntax parsing only, no
compile/runtime. This is ongoing integration, not NPC readiness.

Shared animation lookup and security construction integration:
GetTreeAttr82d79170 checked directly: ordered cached rows, all five packed name
words, return first matching copy. Existing skater helper now delegates to the
shared frame-phase lookup; pedestrian frame producer uses core.try_tree_attribute.
No independent pedestrian attribute algorithm. Animation now retains concrete
frame metadata initialized from the source constructor via agent FrameMetadata.
SecurityGuard82e98778 and PostInitialize82e70330 checked directly. Existing
SecurityGuardData now retained on PedestrianConstruction, using its same store10
reservation and same identity; navigation rig bit80 is enabled on guard extension.
Post placement calls existing zone helper and sets AI flag8. No separate guard
roster or invented chase state. Full factory/placement caller remains pending.
Shared files syntax parsed only; no build/runtime performed.

Spawn/model source continuation:
Direct Spawn82e9a510 establishes mesh-cell validity/cost gate, barycentricY,
existing pedestrian density query, DMO planar overlap gate, factory create,
motion-pool increment, InitModels, collision-model null teardown, mesh-cell
assignment, target placement, world bodies, culling flag, contained props,
PostInitialize. No alternate minimal spawn has been published.
InitModels82e78580 raw PPC confirms identical colour-array keys/draw order to
vehicle InitModels: df76d7d773857edb then12026e2eed18cc8d, global rw RNG each,
modulo length stored as byte. Extracted existing vehicle implementation into
CommonEntityCore.initialize_model_colours and reused for pedestrian model
identity prefix, preserving errors and first-store ordering. This is not the
complete InitModels/AI loading flow. Shared execution module registered after
agent file existed. Syntax parsed only; compilation/runtime pending.
Retained pedestrian_execution attached to LivingWorldGrindServices and initialized from actual cvars before vehicle Init. Direct82e9ef00 verified rate capture order. No frame execution call or NPC publication claimed by this initialization.

Frame-order continuation:
Direct82e96fc0 confirms preframe order: collect/sort, positional, state graphs,
pre/seek/AStar/update/post navigation, transform/validate rigs, validator process,
distribute/orient, ground/volume/groundDMO creation. Current frame receives all
queries before modifiers/intents. Added entity positional task using existing
rig cell history and shared player-distance stack; mesh iterator supplied by
actual retained target owner, null threshold0. Direct82e72fd8 confirms ordering.
Shared MatrixProgram constructor now exposes existing retained output allocation
for pedestrian pose producer; old skater call uses same constructor. No extra
pose implementation. Shared ACS currently forbids multiple programs and mixed
fetch/set dependency graphs; agent auditing actual allocation/dispatch next,
not bypassing checks. Syntax parsed only, no compile/runtime.

Retained navigation/frame state and entity predicates:
Entity now owns NavigationIntentState and native PedStateOut; animation retains
optional real SimLodPose separately from completed global bones. No fake pose.
conditions::entity_condition directly consumes existing collision state for
NeedsToBeginColliding826eadf8 and IsPedestrianColliding82707690. IsOnRoad
82701268 virtual dispatch checked through Pedestrian82e6e378: rejects current
cell score<=0 before base RAIN road/intersection check82e4c048. IsOnIntersection
82701160 uses base82e4c0d8 without cell gate. Missing collision/rig errors remain.
These predicates await complete concrete graph backend; no success fallback.
Syntax parsed only. Shared pose dispatcher audit found further RAW/VBR,
external matrix SetData and multistream allocation dependencies; agent tracing
native commandset allocation instead of removing restrictions blindly.

AI direct callbacks continuation:
Registered factory/model-init children after peer source review; they retain
construction prefixes and still require real graph/animation/suffix bindings.
Added simple_behaviours constructor/callbacks: SetSimpleTimer826ec700/7c0 uses
actual timerName default and length0; existing timer storage reused. Raw PPC
verifies waiting/reacting stores use AI state1a18+CAA, flags[2] bits20/10.
AllowPedestrianJumping826ee898/8b0 writes NavRig1a28+141, actual do_jump_test.
Unhandled callback phases explicitly return unhandled, not successful noops;
full vtable registry/generation still required. No live graph completion claimed.
Syntax parsed only, no builds/runtime. Pose agent now tracing original fixed
matrix registration and shared ACS allocation; registry changes stay source-led.

AI lifecycle follow-up (source only): static comparison of all134 state
rows found83 unique update_address values; every value is present in the
new ordered update dispatcher (83/83, no extras). This is callback sequence
coverage, NOT completed callback implementations or live AI readiness.
Captured80 unique behaviour class lookups in pedestrian-ai-lifecycle-2026-09-30.json.
77 exact vtables resolve; ResumeChase, SpawnConversationArea, SuggestMusicState
need factory alias/registration investigation. Slots44/48/52/56 represent
CreateInstance/Begin/Update/End. Folded CreateInstance82bb3da8 returns null;
do not allocate fictitious instance state for these occurrences.
SuggestVelocity End826eb470 calls Navigation+52 unconditionally; base vtable
823a3eb8+52 resolves ClearDebugTuningVelocityOverride82e6bbe8. Added missing
end callback over existing velocity_override, retaining seek/ramp speeds.
This prevents lifecycle binding from treating that End as no-op. The full
AI backend still needs to dispatch it. Registered all89 condition binding
child and retained one manager PedestrianAnimConstructInfo on existing
LivingWorldGrindServices (native initial pool0). No duplicate roster/backend.
Only syntax parsing performed; no builds or runtime tests.

Resolved remaining three AI factory aliases: registration82f60768 ResumeChase,
82f618a8 SuggestMusicState,82f61d28 SpawnConversationArea all select folded
factory82c90c88. Factory allocates base Behaviour28 bytes, runs base ctor,
installs PrintText2D vtable82300930. Its CreateInstance is native null82bb3da8;
Begin/Update/End all825187c0. Added these exact no-op leaves (no gameplay
invention based on their names) to simple behaviour binding. All80 behaviour
names now have verified lifecycle metadata; unknown names remain absent,
real callbacks remain explicit. Full concrete backend still in progress.
Registered lifecycle module and syntax parsed; no compilation/runtime.

Additional AI leaves: ClearExhaustionTimer Begin8270a9d0 calls IChaser
ClearExhaustion82e6fd28. Raw receiver lwz1958 (IChaser-adjusted+192 gives
entity1a18), stfs stateC74. PDB tChaserData atC64 has mExhaustion+10;
reused existing state.exhaustion, not a duplicate chaser timer.
StartInvestigateTimer Begin826ed9b0 reads GetInvestigateTimerDefault82e6f180,
sets timer1. Raw key assembly is1ae29113978be5aa, adjusted IPedestrian
receiver18b0+176 resolves entity1960 mAttribulatorData. End826eda00 resets
same timer1 to0. SetTargetUnreachableTimer Begin8270bd68 reads getter82e6f690
from actual chase collection1970 key5a3c4a7700dda13c, sets timer40.
All getters sample resources at callback time; missing collection errors,
missing scalar preserves native zero DefaultDataArea. Source syntax parsed;
no compile/test/runtime. Lifecycle backend still must route these leaves.
Fixed manager construct-info new to const to match existing const services ctor.

Additional end leaves: UnsetWantOnEnd ctor826edec0 requires authored want,
resolved by existing want_type; End8270ad08 reuses SetWantActive(false).
TakedownTargetableClearOnEnd8270a5b8 calls SetTakeDownSinkId82e7aa50 with
(-1,-1). Raw setter writes state864/868, PDB confirms mTakeDownSinkId;
reuses sole state.take_down_sink. No unrelated flags or targets cleared.
Shared PedestrianChannels now exposes source PlayChannelAnim remap entry:
82e91e08 -> existing RemapAnim82e8d8e0 -> TransitionChannelAnim, including
when current status is Finished. Short wrapper82e6e830 raw f1=f2=f3=.2,
r8=false. Registered clips, selected data, and original random source remain
caller supplied, not duplicated. Intended consumer is concrete guard channel
service; that connection still pending. Syntax parsed only; no builds/tests.

RestFromChase concrete Begin/End added and bound through simple behaviours.
Begin827172c0: creates shared monitored chasertired packet (initializer82f60078),
adds Tired then Tired+StopTired stages with RotateToTarget false, suggests
through existing queue, pushes NoHeadTracking, clears exhaustion, sets resting
flag stateCA9 bit2, reads entity rest timer key59604f998608081e at callback
and writes timer4. End826eb218 advances same monitored packet stage, releases
monitor (active packet remains queue-owned), pops headtracking, clears resting.
No duplicate queue, state, animation, or timing owner. Native GetMonitoredIntent
is direct monitor lookup; completion does not destroy unmanaged packet.
Also bound EndTaze826ef018 in GuardBehaviour: channel TazerOut before state3.
Registered agent collision lifecycle child. Syntax-only parsing passed; no
build/check/test/runtime. Full live NPC session wiring remains incomplete.

Prop/waypoint registry integration prerequisite: admitted EntityMan<256,E>
through existing shared roster and LocatorKdTree, not a second spatial tree.
Native Prop EntityMan ctor82e67710 -> fixed_hash_map ctor82e66f20 calls
GetPrevBucketCountOnly(257). Read actual source prime table822852f0 entries
241,257,277 confirms effective257 buckets. Init82e67788 allocates80-byte
KD blocks x256. Existing generic try_initialize already uses N; no alternate
query or ordering implementation added. Syntax parsed only, no compilation.
Prop/waypoint agent is integrating this with real asset/group reservations;
full pedestrian/guard session still not complete.

Retained local behaviour bindings added: constructor data is parsed per authored
occurrence once; Begin/Update/End dispatch consults verified native lifecycle.
Only native empty825187c0 skips work. Existing SimpleBehaviour (including Rest)
and SuggestVelocity callbacks mutate actual entity owners; nonlocal callbacks
return their real source address for required backend dispatch, never success.
This connects callback implementations to one dispatch contract but full
PedestrianGraphBackend/session scheduling is still unfinished. Syntax parsed.
Registered sole services::waypoints module and retained props Option owner;
agent is migrating actual chunk/Guid references, not creating a parallel roster.
No compilation/check/test/runtime performed.

ChangeNavModifierSetting is now bound in retained local dispatch. Constructor
826ee5f8 parses set_to defaultfalse and case-insensitive modifier names:
ChaseGlue0/Pedestrian1/SkaterAvoidance2/VehicleAvoidance3/BigBlackAvoidance5.
DynamicObject4 is intentionally not an accepted constructor name. Native
CreateInstance827126f8 is folded12-byte instance with saved boolfalse.
Begin826ff568 reads current modifier IsEnabled virtual48, saves it per
occurrence, then SetEnabled virtual52. End826ff608 restores the saved value;
no reset-to-default policy. Reused sole NavigationModifiers fields/set_enabled,
added read-only enabled getter. Local dispatch now mutably retains the captured
value; every native Begin overwrites it before any End. Native Update is empty.
Syntax parsed only; full graph backend/live session still pending.

Guard reaction leaves: KnowAboutWantTarget ctor826eda80 requires valid want;
Begin826edb30 calls shared KnowAboutEntity(target,30) without active-want gate.
UnsetWantAndSuppressMoodForATime ctor826edd48 defaults timeout1 and requires
valid want. Begin826ede20 gates active, copies target, calls existing
SetWantActive(false), then SuppressMoodForATime82e98718. Shared suppression
helper reuses same knowledge record and native fsel-max timer semantics,
including unordered case. Existing SuppressMoodAboutChasee now reuses this
same helper; no second mood store/algorithm. This preserves SetWantActive
reaction-count/mood-reset effects BEFORE suppression. Syntax parsed only;
no compilation/runtime. Full concrete AI backend still pending.

Ordinary Wander/NoRoadWander callbacks now call existing retained targeting
owners. Wander Begin826eb108 resets honking id at stateC80, selects mesh1,
sets navigation+94 Auto, SwitchTargetingMode2, then sets entity1a44bit80.
NoRoadWander Begin826eccb0 sets rig-enabled first, selects mesh1, then mode6.
Raw store order checked; shared targeting retains native Seek and RNG behavior.
Wander Update826eb198 checks actual optional navigation then SetSeekSpeed2;
connected in local phase dispatch. Begin still requires full targeting frame
from live AI backend. Exposed optional current-navigation accessor shared by
strict existing accessor; no second navigation owner. Syntax parsed only.

Target timeout follow-up: IDA raw826ef5c8 loads timer41 and branches out on
fcmpu/bgt (>0), so unordered also invokes entity vtable+0x80 with argument4.
PDB identifies that slot as OnPathingFailure, not SwitchTargetingMode.
Native82e6e4a0 obtains current iterator, calls its failure callback and seeks
with force=true only if requested. Added this over the same retained iterator
adapters; unlike internal MeshNavigation failure it does not reset feedback or
A* counters. Shared adapter selection now accepts active physics/mesh iterator
kind without duplicating iterator state. Both navigation owners are constructed
together by existing entity initialization. Local occurrence dispatch with a
targeting frame routes Wander Begin, NoRoadWander Begin and timeout Update to
these implementations. Syntax parsed all three modified Rust files; no compile
or runtime. Concrete PedestrianGraphBackend remains absent, so these are NOT
claimed live or NPC-playtest-ready.

Agent interruption checkpoint: all three worker agents returned terminal usage
limit errors. Animation worker planned PedestrianSessionAnimation + actual
census/factory caller, but search finds no adapter by that name yet. Prop worker
identified synchronous waypoint registration needing real RecipeMan access;
do not replace publication with successful queued placeholders. Contact worker
still needs existing ChaseeState attached through source constructor/reset.
Preserve partial edits and resume those tasks after inspecting current files.

InvalidateNavTargetsOnEnd826eed40 now dispatches through retained targeting.
Raw instructions and PDB identify entity1920 as mNoRoadWanderTargetIterator,
its vslot10 as IncrementTarget, entity vslot9c as GetNavigation, and navigation
vslot4 as TargetExplicit. Native first increments no-road at current position
with float bits3e2aaa7e (existing iterator ignores dt), then TargetExplicit at
the same position with mode1. It does not switch the current iterator or reset
all nav state. TargetExplicit82e6be90 stores mode before virtual destination,
then forces Seek unless mode2. Existing MeshNavigation.set_destination and
Navigation.set_seek_destination82e7db70 reused, preserving Euler/turn/seek
direction calculations and active mesh versus physics dispatch. Source and
syntax verification only; live concrete AI backend remains pending.

Patrol occurrence binding now retains a separate PatrolExecution for every
authored occurrence and routes Begin/Update/End to the existing concrete owner.
Begin uses actual TargetingWorld; End restores that occurrence's captured
arrival radius. Rechecked folded CreateInstance82712148: allocated12-byte
instance initializes float+8 to0, so retained constructor now reflects that
instead of absent payload. End826fe478 reads this same float and calls active
Navigation.SetArrivalRadius (vslot144). No duplicated patrol movement logic.
Worker agents resumed successfully after earlier usage failures; animation
factory, Prop streaming and contact integration still owned by those workers.

Live contact caller added: selected_living_world_dmo.try_update_post_transforms
now invokes shared pedestrian_resolve_contacts with the real world/local
identity/cvars, after existing vehicle contacts and before DMO scoring contacts.
Dedicated PedestrianContacts(String) propagates failure. EntityMan40 is already
allocated by initialize_entity_owners before this registered-world path. No
second roster or physics backend. Agent contact dispatcher owns classification
and body responses; exact cross-entity native Island pointer-map traversal
ordering remains a documented limitation. Caller syntax parsed, not compiled.
Spawn still awaits real collision fills, full AI controller and admission suffix;
empty roster contact traversal is not evidence that live NPCs are working.

Initial AI graph load added to entity model_init: load_graph(graphs,path).
Native InitModels82e78580 calls LWBasicEntity.LoadAIGraph82e4c340, which owns
AIGraph then HierarchicalController.Load82cef180. Load calls Create with
allow=true/require=false/call_behaviours=true; generated factory82951770 uses
exact Pedestrian.xml basename and delegate5 enabled flag. Reuses shared
select_onboard_controller and actual living_world_enablement; no second path
parser. Other routes and reload remain explicit errors. Retains ai_bindings
alongside existing graph/active occurrence roster. Constructor82932b08 sets
current/lastNULL,dt0,emptyactive; Load only hashes root, no initial Update.
Condition/local bindings now retained by entity; full concrete callback backend,
shared native behavior statics semantics and root-hash consumer remain unfinished.
Syntax parsing passed, no build/runtime. Animation factory agent notified of API.

Local behaviour bindings now retain PedestrianColliding's authored behaviour
and separate instance for each occurrence, dispatching Begin/Update/End to
existing colliding owner with real external services/world. Added CreateInstance
dispatch for Patrol, modifier and collision; re-entry resets payload rather
than retaining previous recovery state. IDA rechecked82711d20:32byte allocation,
float+8=1, words12/16/20/24=0,byte28=0. Folded827126f8 initializes byte8=0;
82712148 initializes float8=0. Native null-instance factories remain empty;
unimplemented factories return their actual source address, never success.
Syntax parsed binding file only. Concrete live backend remains pending.

Behaviour enablement now bound per occurrence using shared authored boolean
reader, defaulttrue. Verified generated ctor82932c90 passes null parent to
factory; Behaviour82cf70f0 calls Base82cf6f88, whose mEnabled is parent gate
AND GetBool(enabled,true). Thus generated parent state's active flag must not
be substituted for behaviour enablement. Added retained enabled accessor for
backend. Syntax only. GameSim event integration delegated to Prop agent;
animation worker continues real census/factory and then motion external owners.

AI load metadata/static ownership follow-up: GetRootState82933b10 returns
index0 (authored nameAI). Retained root_name_hash now uses the existing
sk8_hash_value_32 on that name, matching Load82cef218; filename/child name
must not be hashed instead. Delegate.Init82926b60 constructs89 conditions
once under sInit. Conditions now Arc-shared from ActorGraphStartupOwners via
lazy retained OnceLock, instead of rebuilding per pedestrian. Load binds local
behaviours before conditions to preserve generated ctor82932b08 order.
Behaviour shared statics/per-instance split is still unfinished. Syntax parsed
three changed files; no compile/runtime. Collision worker now owns concrete
AI backend and lazy selected-world condition projections, coordinating actual
frame scheduling with animation worker. Root avoids those files concurrently.

Flee lifecycle826ed228/288/398 now routes through targeting bindings. Raw/PDB:
Begin enables1a44bit80, calls SwitchTargetingMode4, copies wants10 target
(state19c+10*12=214) to separate ignore-avoid ID1958. End sets both IDwords-1.
Entity ignore_avoid_target added (ctor82e97c54/58), separate from collision ID.
Update calls IChaser vslot80 GetMaxChaseSpeed (existing entity getter reused),
sets seek speed, increments active iterator with bits3e2aaa7e, gets target,
TargetExplicit(mode1). Shared active-navigation TargetExplicit extracted from
invalidation path; mode2 skips Seek as native82e6be90. No alternate movement
policy or hardcoded chase speed. Three files syntax parsed, no compile/runtime.
Registered worker's new graph::backend module; selected-world services remain
under active implementation. Ignore-avoid consumer wiring still needs review.

### World-avoid query continuation

Source traced Manager.GetWorldAvoidVolumes827032a0 -> ChallengeLivingWorldManager.GetAvoidVolumes825c0e48. The list is initially empty (ctor825baa18), populated only by AvoidArea operations after census culling, and cleared on GameUnload825b6080. It is not the entire trigger registry. ExecuteLivingWorldOperation825bde28 raw825bdea8 uses cmpld for full64-bit duplicate identity; decompiler incorrectly displays low32 equality.

Added ChallengeAvoidVolumes and resolved per-query view in existing pedestrian_navigation_wander.rs. Resolution reuses CameraTriggerGroup standard group0/1/2 retained-entry scan and native u64->double->float GUID comparison; absent streamed instances skip. Existing invalid-entry representation is None, naturally skipped by the scan. Added SimulationRecordHandle.trigger_contains_point using the SAME retained arena's projected collision model/root, not a separate asset cache.

Point membership82e216e0 calls ComputeTriggerVolumeTransform82e21628 and ComputerTriggerVolumeRadius82e21508. Shared affine composition preserves multiply/FMA XYZ sequence. Raw82e21540..7c proves non-Box radius broadcasts XYZ; Box reads shape XYZ at44/48/4c. Membership adds/subtracts signed scaled axes in source order, with inclusive ordered XYZ comparisons; no invented absolute-axis extent or inverse OBB test. Missing model/volume remains explicit error since native transform dereferences it. Exceptional Xenon floating-point behavior remains shared compose limitation. Concrete AI owner/frame attachment delegated to ped_adjust_owners; this is not proof of live NPC readiness.

Validation: rustfmt syntax parse on changed files passed. No compilation or tests run.

### Ordinary road AI callbacks

Connected CheckForRoadTarget826ef650, UseCrossWalk826ecbe8, WanderOnRoad82722f50 through existing dispatch_with_targeting. Existing RoadNetworkTargetIterator.check_for_target, traffic-light can_proceed, native PointInTris_XZ and TargetExplicit owners reused.

CheckForRoadTarget: native raw bgt skips only timer39>0 (NaN falls through); refresh retained road iterator even while NoRoad is selected; target-timeout timer41 result precedes timer39=15 reset. UseCrossWalk: optional navigation; default speed2; current iterator slot20 is CanProceedToTarget, false selects4. All seven non-road iterator vtables bind slot20 to folded true826c6d80; road uses82e6cc10 and existing traffic-light clock. Verified its raw bgt correctly matches existing remaining>crosswalk_time.

WanderOnRoad: Pedestrian+0x70 is inherited LWBasicEntity.mpRAINData (base+32 plus80), not a new spatial query. Road segments supply corners80/96/112/128, triangles(0,1,2),(0,2,3) in segment order. First containing segment chooses closer corner1 or3 via dot3 squared distances (ties corner1), explicit target mode2 then speed4. Does not snap to arbitrary nearby roads. Syntax parsing passed; no build/runtime checks.

Next root scope: concrete FrameServices.evaluate_guard_condition with actual target/Actor/Pedestrian owners, existing guard predicate reuse. Source checked HasTakeDownTargetable8270c8e0, CanAttempt8270c960, CanTaze826f22e8->82e6f738 (chase layout byte9), GetEndChaseReason82e6fdd0; remaining actual wipeout/takedown producer binding not yet completed.

### Guard condition frame and target interface resolution

Added registered pedestrian_graph_frame_guard child: evaluate_guard_condition reuses existing living_world_guard_graph.state_condition, supplies facts lazily for the selected condition, and preserves CanAttemptTakeDownTargetable8270c960 ordering (live disable cvar, sink existence, wipeout, selected takedown type). Shared cvar added by peer from initializer82f5e8b0: Pedestrian_Chase/kDisableTakedowns=false. CanTazeUnreachableTargets826f22e8 delegates82e6f738, reading retained authored chase layout byte9. No hardcoded guard tuning.

Shared target_exists now uses actual Actor/Pedestrian/DMO/Vehicle owners; native ConvertTargetableTypeToITargetable82de3658 resolves all four. PDB enum confirms kinds0/1/2/3 respectively. Frame now borrows existing Vehicle EntityMan24 alongside DmoObjectMan. Invalid IDs/missing entries return absence; unknown valid types error as native assertion.

Shared target_wiped_out: Actor825edad8 consumes actual completed PhysOut.State.is_wiping_out. Pedestrian82e6eba8 has adjusted ITargetable receiver+184; raw load1960 therefore reaches actualentity1a18 state, CA9 bit4 = flags[1]&4. Decompiler incorrectly labels this attribute collection memory. DMO8232cf70 and Vehicle82326b00 ITargetable vtable slot24 both bind folded82784530 (li r3,0;blr), so their existing targets report false. This removes potential incorrect target misses without inventing wipeout state. Guard/chase use same helper; actual trait/frame live scheduling remains peer integration work.

Validation: syntax parsed changed frame/child; no compilation or runtime. No end-to-end NPC readiness claimed.

### Chase navigation and return-to-patrol continuation

Added entity.begin_chase_navigation shared by BlockChasee.Begin826ff0e0/PursueChasee.Begin826ff1a8 and connected both source addresses to dispatch_with_targeting. Preserves explicitmode1 on current nav -> targeting3 (including nonforced seek) -> meshnavigation -> mbSmoothingEnabled=false -> forcedSeek(true). PDB/raw confirms native byte6724 bit20 is smoothing; retained field constructor true from7<<4. This is distinct from navrig enabled bit80 and slideable bit40. Live intent composer must read same field (peer notified).

ReturnToPatrolZone8270ea38/8270ead0/8270ece0 uses LWPatrolZoneOwner slot72 returning flag, slot60 startpoint, slot40 optional entityofinterest. Migrated existing guard-only startpoint into sole Pedestrian base owner, preserving guardPostInitialize write before zone lookup. Base startpoint and returning bool are native MemStorezero; ctor82e978a0..d4 only initializes interface/zone null/entityinterest null/parameters. Reset82e51780 only updates parameters.

Implemented return movement prefix explicitstartmode1 -> seek speed1.8 -> HasArrivedXZ(slot128 forwards currentarrivalradius via82e6bc00); prefix returns arrival so caller resolves optional interest only afterwards. Arrival suffix sets interestposition explicitmode2 if present, then rampedspeed0 and seekspeed0. Frame guard service wiring delegated to shared-frame owner.

SetAltTargetToChaseePosition827091a8 calls currentiterator slot56. Verified seven nonchase concrete vtables use empty825187c0; chase82e6ce90 queries actual mesh cell with live kPathogenYTolerance and only replaces previous alternate on valid cell. Added existing entity target iterator setter accordingly, not unconditional replacement/clearing. Syntax parsed all changed files; no build/runtime checks.

### Execution query lifetime integration
Root re-read IDA LWPedestrianExecutionMan::Init82e9dc50 and
Task_UpdatePositionalData82e72fd8. Existing NavRigMeshInfo/set_cell and entity
update_positional_data already implement cell history and distance ordering;
no duplicate positional implementation added. LivingWorldGrindServices now
retains the existing PedestrianExecutionQueries alongside its execution/roster
owners. Selected terrain startup acquires its native ground A/B (five each)
and default A/B (one each) batches from the same initialized PhysicsWorld query
owner and DMO pool. One-time initialization preserves a failed prefix rather
than retrying lease acquisition. Current receive rejects uninitialized buffers.
Live Update2 current/pre-frame passes are still incomplete: this is ownership
and initialization integration, not evidence of moving pedestrians. Syntax
parsed; compilation and runtime remain unperformed.

### Current-frame intent producer
IDA Task_EvaluateNavModifiers82e72ed8 and Task_UpdateIntents82e72f58 confirm
separate complete target-list passes. Added PedestrianConstruction.update_intent
from82e94f70: first active intent or speed recommendation wins in retained
six-modifier order; managed modifier packets assert; suggestions use the same
queue and AI feedback. Existing IntentGen.Update82e937b0 then writes the same
animation owner's motion input, using target-transform orientation and existing
navigation adapter. Speed mappings resolve the selected/override animation
attributes only on the base-locomotion branch; no new attribute parser.
IsNavRigOverrideEnabled82e94e70 raw PPC reads adjusted IPedestrian+0x1998 bit4
(complete pedestrian6724 bit0x10). Added this independent constructor-true
flag; it is not the existing chase/AStar bit0x80 or smoothing bit0x20.
Syntax parsed only. Update2 scheduling and modifier producer integration remain
pending, so this is not a claim of operational NPC movement.

### Navigation execution dispatch audit
Task_SeekNavigation82e73360, AStar82e733f8 and NavigationDoUpdate82e73490
read active navigation; the update uses each execution target's dt. Added
retained-iterator entity wrappers for MeshNavigation Update/ResolvePath.
Raw PhysicsNavigation vtable823a4390 slots36 and76 both resolve825187c0
(empty), so physics navigation does NOT inherit base speed ramp in these
passes. Pre/PostNavigation Entity slots40/44 are also empty for pedestrian
and security guard (peer vtable audit). No invented work for these slots.
Added IsPointInNavRig82e527f0 over existing heading-distance implementation:
full XYZ delta length first, clearance query second, strict clearance>length.
Raw PPC82e52888..b4 proves stack scalar reload despite misleading decompiler
alias. AStar uses this real rig query for its off-mesh recovery branch, with
same numeric owner. Rig is restored after split-borrow even on returned error.
Syntax only; broader Update2 live scheduling remains incomplete.

### Shared navigation frame inputs
Added FrameServices seek/resolve/update navigation adapters over retained
TargetingWorld. Flee CalculateFleeDirection82e77e98 reads wants[10], checks
actual iterator pedestrian pointer, resolves targetable, reads velocity before
position. Chase resolves actual Chasee rather than passing a fabricated point.
Actor velocity825ed338 reuses PhysOut camera-system damped COM velocity with
no census near-zero clamp; pedestrian82e94658 and vehicle/DMO82d36158 use
existing common-core velocity. Physics-navigation Update/Resolve return before
building unrelated mesh facts, matching the folded empty vtable callbacks.
These are adapter methods; outer batch scheduler is still being connected.
Syntax checked only; no compilation/playtest.

### Live pedestrian LOSim group changes
Connected SelectedRecipeFrame.update_pedestrian_los_states from existing
try_update2/tick, before vehicle UpdateLOSStates. Re-read ComponentMan ped
82e82da8 and actual Entity vtable823a4f58+c8 -> TestLOSim82e94ce0; guard
823a54f8+c8 shares it. Actual existing Chasee forces0; otherwise shared
LWBasicEntity.TestLOSim82e50180 authored radii. On change, remove old bucket,
write core.current_losim, add new bucket in native order. Collision switch
82e50580 only queries optional collision pointer and returns true. Same
EntityMan and ExecutionMan retained; no second roster or distance policy.
Remaining Update2 advance/current/pre/navigation batch integration is not
complete. Syntax parsed, no cargo/build/runtime tests.

### Concrete positional iterator borrowing
Moved the existing entity update_positional_data into the targeting child,
replacing its unusable externally borrowed same-entity iterator parameter.
It now borrows the actual retained mesh iterator lazily through the existing
with_target_iterator dispatcher. The existing NavRig.update_mesh_cell owns
identity comparison/history/score writes; player-distance update follows it.
Rig is restored before returning a checked failure. No second position/history
implementation retained. Added frame entry with source cvar
kDefaultNavMeshCheckYTolerance (peer registered1153, initializer82fa5820,
default1 range0..50), independent of other mesh tolerances. Mesh iterator
selection is preserved even when active navigation is Physics. Syntax only.

### Chase notification queue reuse
Re-read RegisterChaser8271b9e0 and UnRegisterChaser827186e8. LocalChaseStart
kind661379046 and LocalChaseEnd kind1771027901 are native16-byte header-only
messages on the existing Challenge broadcaster/queue7, NOT GameSim queue4.
Generalized existing Challenge queue payload from skater-only into typed
ChallengeMessage/ChallengePayload; skater notifications remain20 bytes and
keep their original kind/port/ID fields. Added send_local_chase using the same
broadcaster mutex and variable message size. SkaterManager now exposes the
same already-registered broadcaster Arc; no second queue/broadcaster created.
Updated preexisting payload assertions, syntax parsed; tests unrun.
RegisterChaser also revealed an outstanding ordering gap in the old pure
helper: native sends start BEFORE appending chaser and mutates count AFTER
optional mood notification. Returned event-vector helper alone cannot preserve
checked enqueue failures at those boundaries. Live ordered dispatch and
StatsBackEnd14 consumer remain to implement; no completion claim.

### Ordered ChaseMan registration
Replaced unused returned-vector registration/unregistration helpers with a
ChaseRegistrationServices interface invoked at actual native boundaries.
Register8271b9e0: duplicate check, Challenge start enqueue, vector insertion,
resolve/read chaser attribute, optional GameSim start, increment separate
mNumChasers, first-chaser timer reset, Stats personal best. Unregister827186e8:
erase, assert/decrement count, resolve/read attribute, GameSim end, optional
local capture/stats/suppression updates, Challenge end. PDB confirms separate
mNumChasers+6304 and fixed_vector256; ChaseIsOn826f3b48 reads count, not vector
length. Failed callbacks retain source prefix and do not run later mutations.
GameSim typed ChaseStart/ChaseEnd20-byte messages added (kinds662517041 and
-1049887331); PDB cMsgChaseStart/End confirms fID at16, not cMsgMoodEvent80
as misleading decompiler template label suggested. Live service implementation
and backend stats/FE consumers still remain. Syntax only, no build/tests.


## Continuation: current-frame order and actual chase side effects

Re-read IDA Manager.Update2 `82715a10` and pedestrian Current/PreFrame
`82e96ef8/82e96fc0`. Split the existing vehicle_update2 implementation into
vehicle_update_los_states, vehicle_update_preframe, vehicle_update_targets.
The sole caller advances vehicle execution exactly once between LOS and pre;
no second vehicle implementation was created. Pedestrian current-frame adapter
now receives the three retained query buffers, selects the original current
execution targets, evaluates the WHOLE target list's modifiers, then updates
the WHOLE list's intents. This adapter is not yet invoked by try_update2:
full prestate/mood/query-send phase integration remains unfinished.

Stats source checks: IsRecordingAllowed8255f408 raw lwz+0x54 is GameBackEnd
mGameState, confirmed by PDB (module1), NOT an unknown vtable flag. Enum0=InGame,
1=NIS,2=Movie,3=Loading. Inlined ctor8258106c writes3; StateLoading leave827582e8
writes0. Added retained GameState and sole StatsBackend projection to existing
LocalBackendOwner, with loading exit only after successful selected startup.
No existing statistics owner found. Stats constructor82a72cc8 loading=false;
Init8255eaf0/Core.Init82a72d48 seeds22 personal values from original thresholds
at82fda0c8, and65 static stats at82fd9db8 start0. Added exact lazy cvar/game-mode/
loading/game-state gates, strict personal best updates8255f4c8, separate record
82a72f40 and default additive UpdateStats82568b90 branch. Special achievement/
replacement branches remain explicitly excluded. This is NOT save/profile or
achievement completion. Raw82568ba8 verifies force is GPR6 (decompiler a5 wrong).

Local guard AddChaser now mutates actual Chasee roster then registers with
actual ChaseMan using shared Challenge queue7/GameSim queue4 and sole stats.
Remove source82de4908 erases entry, calls ChaseMan.UnRegister, THEN clears the
final group reason; split existing helper accordingly so capture is not lost.
GetLocalCaptureChaseSuppressionTime82702330 reads retained group parameter
layout+4. ChaseMan.Reset82702260 now binds original global collection and
retains original flag/timer mutations; initial census calls this reset part
once, before entity population. No per-frame rebinding or duplicate manager.

Still pending: chase escape CentralHUD and optional career achievement consumer,
GameSim subscriber dispatch, full Manager.Update2/preframe integration, MoodMan,
handprop lifecycle completion and remaining NPC renderer paths. Do not mark
NPCs playable from these helpers. Eleven root-modified files passed rustfmt
syntax parsing only. No cargo/build/tests/runtime run.


Next root integration dependency: UpdateHandProp82e9d320, assigned to root
(animation peer remains HeadTracking/PostAdjust). Existing pedestrian_hand_prop
already has shared request_channel, throw away/at position and remove methods;
DO NOT duplicate. Native UpdateHandProp branches: held+throw flags2/1 waits
channel remaining<=release_offset when flags3&80 else timer34<=0. Directed
flags3&40 uses SAME geotest.SolveForInitialVelocity82e56468 already implemented
privately in pedestrian_jump_trajectory.rs (expose/reuse), writes timer35,
then clears held2/sets released1 and DynamicObject.ReleaseHandPropToPhysics.
Undirected reads actual DMO velocity virtual+16 and multiplies by throwforce.
Held+notthrow+tazer flag4&80 handles Into1 and Out3 atchannelremaining<=.2;
Into playsTazerCyc then Cyc;Out clearsstate/action/flags, RemoveHandProp then
RequestExplicitCull. Non-tazer flags3&20 calls UpdateHandPropAction82e9ce28
(unimplemented). After any held branch, channelFinished replays authored
handprop layout+56 string. Released branch compares absolute XYZ DMO-vs-ped
position against separate lazy DisownDistanceXYZ cvars and RemoveHandProp.
ReleaseHandPropToPhysics82d2ae28 RAW IDynamicObject-relative+1080 clears
transform type (complete+110c), collision getter+0xc=false, culltimer+106c
(complete+10f8), optional Body.SetVelocity part0. Does NOT clear monitoring
list despite decompiler misleading member name. Need reuse existing body
pool/backend Part.SetVelocity, not target-transform drive. No new handprop
orchestration code written yet in this continuation.

Peers: adjust owns complete PreStateGraph/MoodMan and managerUpdateMoodables;
ground owns query-send assembly now (plugin eligibility checkpoint first);
execution owns HeadTracking/PostAdjust and livingworld_stamp renderer suffix.


## Checkpoint and root continuation

All three parallel workers stopped early. Worktree edits
are retained, but their remaining scopes are incomplete. Do not infer completion
from prior running status. Root checked actual Vault.try_load: selection is
explicit and does NOT transitively load RefSpec target classes. Added MoodMan
category/mood/result classes to selected_original_asset_classes using shared
constants from pedestrian_mood; otherwise its new eager Init fails at startup.

Added shared body-only Part try_set_body_part_velocity in existing source_parts
module, source Physics.Body.SetVelocity82de7c38: attached authority validation,
frozen activation, cooldown clear, then XYZ velocity write preservingW; detached
missing body skips write. World try_set_dmo_part_velocity uses SAME dynamic
pool island_body view (supports original and constructed bodies), not private
source-only candidate or new pool. Exposed existing trajectory solver for reuse.
These are dependencies; UpdateHandProp orchestration still NOT implemented.
Five changed files rustfmt-parsed; no compilation/tests/runtime.

Ground worker did land four requested handprop cvars indices1185..1188 before
usage failure; inspect query assembly files before resuming to avoid duplicate
work. MoodMan registration and some processing landed; full prestate/Update2
still unfinished. Animation worker head update landed, post-adjust producer
and stamp shader remain to audit/complete. Goal remains active, not playtestready.


## Held-prop release/throw/disown source implementation

Root re-read82e700d0 and82d2ae28, reused existing actual DMO/Part owners.
Added entity.release_hand_prop_to_physics: clears held flag2, sets released1,
switches DMO transform_type Physics, clears collision ignoregravity, samples
original culltimer then optional actual Body.SetVelocity part0. Monitor list,
pedestrian held pointer and collision group are deliberately retained until
existing RemoveHandProp. Native biased decompiler incorrectly labels transform
store as monitoring-vector clear; raw complete offset+110c proves transform.

Added UpdateHandProp throw branch82e9d344..564: exact channel-or-timer34 gate,
directed shared geotest solver (not a second ballistic implementation), timer35,
undirected actual Entity.GetVelocity*live force then release. Added released
branch82e9d704..860 with lazy XYZ threshold reads and existing remove owner.
These scoped methods remain uncalled until full UpdateHandProp orchestration
includes tazer and action branches and post-held channel-finished handling.
No claim of full handprop integration. Current cvars1185..1188 confirmed present.
Syntax parse passed for edited handprop file. No cargo/build/test/runtime run.

### Hand-prop action/update continuation
Source reviewed directly: ChooseNewHandPropAction82e9bed8,
UpdateHandPropAction82e9ce28, UpdateHandProp82e9d320. Existing
pedestrian_hand_prop owner now implements complete ordered held/released,
throw, tazer Into/Cyc/Out and action branches, including final Finished-channel
idle restart after release. Tazer removal queues the existing explicit DMO cull;
no duplicate body destruction or roster. Phone/read use separate timer33 (6/2s),
plugin suppression timer32 (15s), existing channel remap and StateOut audio ids.
StartHandPropAction82e9bc28 already translated in same owner is reused.
Choose uses shared discrete RNG then Manager range RNG; cap16, probability
shortfall clears enabled bit, source defaults preserved. Raw insrdi confirms
class40c4819cbd5a014b and keye4e9c4e514981944. Read-only original skater.vlt
schema confirms POD typee7c886944418f681 size16; PDB confirms action/probability/
min/max offsets0/4/8/12. Both handprop classes added to selected Vault admission.
No callers bypass missing native prestate; full update helper awaits that path.
Syntax parsing passed only. No compilation/tests/runtime.

### Prestate and knowledge adapter continuation
Root verified UpdateEntityKnowledge82e9e620 and reused its existing state kernel,
adding the actual FrameServices adapter (no duplicate knowledge list). Live
CanDetectMoodEvent type15 uses1/1 factors; last known motion only refreshes when
just-added or detected. Existing target positions/COM/velocity reused. Added
ITargetable facing via GetFacing8258f128 column2; Actor.GetTransform825f9e40 ->
GetEffectiveSkaterTransform825ed4c0 reads PhysOut_Skeleton+432, PDB confirms
mEffectiveSkeletonRootMatrix. This avoids confusing raw animation pose/facing.
State ownership is temporarily moved and restored on every Result path.

Prestate prefix now follows LWBasicEntity82e4c248 plugin removal, perception,
timers, mood aging and knowledge; suffix follows UpdateHandProp then preserves
ignore-collision target only while actual target IsWipedOut. Existing DMO locator
owner borrowed into FrameServices for taser cull. Full scheduling still pending
middle phases and query/event integration; helpers are not evidence of live NPCs.
IMPORTANT raw/PDB correction:82e9f3ac loads vptr at biased this+C0, so complete
entity+224 Chasee, slot132 UpdateByChasee82de5200. Decompiler calls it Chaser /
GetBlockArrivalRadius incorrectly. Native middle sequence is UpdateThreats,
UpdateNeedsAndWants, Chasee.UpdateByChasee, UpdateZoneInterests. UpdateByChasee
must update timers, EvaluateBestChasers, AutoSwitchPrimaryChaser, flank orders,
chaser positions, ongoing events when nonempty. Root owns this remaining adapter.
Syntax parsing only; compilation/testing pending.
PostStateGraphUpdate82e6e090 also resolved through raw/PDB: biased this+32
plusA80 is PedestrianPerception2720, virtual slots12/28 are ClearLastAudible
and ClearLastSpectateEvent. Added pedestrian_poststate over existing methods.
Base82e62858 is optional debug text history only, not added to quiet gameplay.
Peers now provide UpdateThreats, UpdateZoneInterests, UpdateNeedsAndWants and
all three query producers; integration still in progress. Queue unification must
preserve actual pause/tick dispatch ordering; do not defer registered Mood events
at pause and call it parity. Root next owns transferable eligibility/ordering;
agent ped_animation owns complete Chasee.UpdateByChasee adapter.


## Mood processing and shared GameSim queue continuation

Source-confirmed MoodMan ReceiveAmbientMoodEventMsg82ce79d8 now selects the
local victim or existing KD40 list (zero broadcast radius becomes1e10), then
delivers through the sole EntityMan/FrameServices. Raw82ce7af8..7ba0 includes
the recipient loop omitted by decompilation. Pedestrian82e9f468/input-state
ProcessNewMoodEntry82e9f010 now use live suppression cvars, native event interest
map and prerequisites82ce7bb0, collision-to-nearbycollision fallback, actual
perception LOS/hearing, authored exposure, and existing merge/add storage.
Actor category key is skater only for actual SkaterManager local index; others
use ai_skater. DMO/Vehicle getters use dynamicobject/vehicle, not entity keys.

FindCurrentMoodResults82e98be0 and UpdateNeedsAndWants82e9e8f0 now read the
original result RefSpecs/layout, prioritize with native strict order, evaluate
target existence/category/onboard/busy, reaction counts and probability timers,
and write actual wants/knowledge. Source updates chosen index/priority BEFORE
testing active wants; preserved even when a later lower-priority candidate is
rejected. Raw probability guard compares absolute probability to EPSILON192
34000000: zero/tiny/NaN skips RNG and succeeds; nonzero uses shared rw_math RNG.
PDB probability timer fixed_vector capacity is3, not8. Results capacity asserts
at8. Pedestrian.IsBusy is wipedout/reacting/waiting/plugin. Actor.IsBusy825f9a90
raw ITargetable slot24 returns NOT IsWipedOut; unusual native behavior retained.
Debug random-roll overlay remains explicit missing-owner error only when that
original default-false debug cvar is enabled. Actor protector resolution remains
explicit when a non-invalid protector is actually present. Source export:
pedestrian-mood-source-2026-09-30.json.

Queue4 is now shared by rate and LW messages in selected sessions. Startup
binds the existing GameSim route to SimulationSimRate before producers run.
Rate kind3fbf5e43/size24 retains float words; camera and StartHub sends propagate
queue failures. Tick flush before queued input and pause-entry flush both use
SelectedMoodFrame with actual recipe runtime, actors, terrain/ground and Vault.
No paused mood deferral or second live queue. Queue.Pop preserves checked
failures and allows sends during callbacks; full flush increments tick once.
Standalone unbound world fixtures retain their previous rate queue, never used
after selected startup bind.

Only source-backed selected registrations (Simulation rate and MoodMan) are
active. Unregistered message kinds follow native EventSys no-listener behavior.
FE handlers826063e8/410/438/460 require MapBackEnd security/chaser icon owners,
registered by FrontEndManager.StartInGame82640450 (xref826408cc); those owners
are not implemented by current MapMenu. Challenge waypoint handler825a5798 is
registered by absent cChallengeCharacterManager ctor825a8e30. Career/Lua/map-icon
consumers are still missing; this dispatcher does not claim those features.

Cvars1189 MoodSuppressed and1190 EnableHeadTracking source defaults false/true;
1199 AllMoodBehaviourSupressed and1200 ShowMoodRandomRolls defaults false.
Existing indices1191..1198 were separately added by execution peer. No build,
tests, or runtime run. Edited Rust files parsed by rustfmt --emit stdout only.
Manager.UpdateMoodables continuous event generation and remaining full-frame
work remain separate unfinished requirements; NPC playability is not asserted.

### Transfer eligibility source continuation
Root reused existing plugin loader and transfer-expression evaluator. Added
ProcessedTransferable.root_state_hash from actual retained generated family
root declaration and shared sk8_hash_value_32 (same existing HC.Load owner),
CanUseBehaviourSource82e86140 querying sole plugin timer, and
DecideToUseBehaviourSource82e9ac50 authored preference/Bernoulli/retry flow.
Raw82e9ad48 is cmpld: compare full64 category, not decompiler LOWORD.
Original skater.vlt confirms preference keyf89323e420a6aaa3, recordtype
e4aac2eabc053d0a stride32; only category8..16/probability24..28 read so pointer
relocations stay validated. Matching failed Bernoulli alone sets retry timer;
no matching preference returnsfalse without RNG or cooldown mutation.
Peer added original cvars1202 transfer Y1.8,1203 retry30. Existing source-list
owner is ActorGraphStartupOwners.plugin_sources (populated by Prop constructor),
NOT another registry. Manager12-frame timer's earlier field remains in dormant
LivingWorldPhysicalServices snapshot; establish live owner carefully rather
than create two authoritative counters. Remaining transfer work: source eligibility,
actual transfer-condition adapters and complete Manager traversal. Plugin odds
uses Simulation.GetRandom(0,1)826fd020, distinct from entity preference global
Bernoulli; do not substitute Manager RNG. Syntax parsed only; no compile/tests.

### Transfer predicate continuation
Added source-backed hand-prop predicate methods (IsHandPropDisposable826fd500
layout byte60, CanSitWithHandProp826fd558 byte61) and original transfer wrappers
82706d78/6e00/6f38 reusing HasHandProp and pending-spawn semantics. Added source
Prop waypoint predicate adapter: HasFirstWaypointAvailable82706508 consults
point0's actual reservation; NearAvailableWaypoint82706610 iterates unlocked
points, reads position lazily and uses shared refined XYZ length with strict
threshold. Unhandled predicate returnsNone, never accepted. Syntax parsed only.
Remaining source predicate notes: HasLineOfSightToProp826fd288 casts from sink
position+(0,1.8,0) to source position, World.LineCollisionTest(null mask,0).
InFrontOfFirstWaypoint82711b38 uses flat orientation to flat sink-minus-point
signed angle and kInFrontOfWaypointAngle. PluginTransferOdds826fd020 uses
Simulation RNG; shared authored preference uses rw::math global Bernoulli.
Full manager transfer adapter remains incomplete and uncalled.

### Transfer waypoint and source eligibility continuation

Re-read InFrontOfFirstWaypoint82711b38 and initializer82f5f1e0:
first waypoint regardless of reservation, flat XZ signed angle around +Y,
strict angle < limit or angle > TAU-limit. Registered original cvar1207,
AI_Prop.kInFrontOfWaypointAngle=0.52359879. Reuses shared native numeric
angle implementation. IsOnRoad82706920 reuses existing entity graph predicate
and its NavRig-cost + RAIN road/intersection gate (82e6e378).

Raw source vtable82394aa8+44 resolves GetBehaviourSinkCount82a96838,
not the decompiler GetSpawnOffset label. Added source admission using actual
behaviour_sinks count vs maximum_participants, never behaviour_copies count.
IsActive82e66aa8 loads source-interface byte68 and shifts7 (existing Prop
flags bit80). CanTransfer82e653a8 checks static GetEditTake address, not a call.

Syntax-only rustfmt parsing passed for predicate/cvar changes. No compilation
or runtime tests. Manager traversal and remaining transfer conditions still
need integration; these helpers alone do not establish working pedestrians.

### Live transfer traversal and RNG wiring

Added SelectedRecipeFrame.evaluate_transferable_behaviours, native82710cc0
cadence (0->1 through11->0, evaluate only0), sole graph plugin_sources in
registration order and existing EntityMan.ids hash iteration. Source gate once
per source, then each sink: no attached plugin, original root-hash retry gate,
strict Y tolerance, flat radius squared subtraction>=0 rejection, declaration
short-circuit expression, category Bernoulli/retry and existing concrete copy/
sink registration. No parallel source registry or copied pedestrian owner.
Live Manager owns evaluation timer; old LivingWorldPhysicalServices field is
only dormant constructor snapshot, never a frame driver.

Simulation.GetRandom8289e808 raw8289e82c..850 confirms unsigned word conversion,
frsp, fmuls and fmadds same arithmetic as LW range but distinct RNG instance.
Added try_range on existing SimulationCommandRandom; transfer odds826fd020
uses it even probability0/1. Tick immediate mode has no simulation RNG; only
an actually reached odds predicate requires it. No substitute random stream.

HasLineOfSightToProp826fd288/rawcall826fd3a8 targets World.LineCollisionTest
with zero fatness and default BF7, from entity position+[0,1.8,0,0] to source.
Reused existing shared world line-batch implementation with native label.
IsChasing uses actual resolved IChasee owner, not just valid target ID.

Remaining explicit transfer predicates: crosswalk, recognizer, conversation.
Executable plugin families beyond LookAt/Trash also still need adapters.
These gaps remain errors, not invented false/success. Syntax parsed only;
build and runtime validation pending. Full pedestrian goal unfinished.


### Concrete ChaseMan.Update and original Jacked frontend

Connected SelectedRecipeFrame.update_chase_manager, native8270f678, over the
same retained ChaseMan, local Actor roster, WorldPainter and backend statistics.
Guard-zone statistic5 precedes suppression timer writes; chase statistic4
precedes both duration writes. Warner iteration retains the native cached end
and shifted/tail slots after erasure. Captured group reason1 searches actual
security guards in chaser order. JackSkater8270fd08 sets the captured flag before
reading live kJackedTimer; PreventManualRespawn825ed228 sets existing Actor input
bit10000000. Removed the unused eager event-vector capture/timer helpers so
there is one executable implementation. Selected ambient freeroam has no
career main challenge or challenge-owned guard; that native null-owner path
leaves the closest authored respawn unchanged.

Captured update decrements its timer before FindByIndex; missing Actor retains
capture; state!=4 clears only captured flag; ordered <=0 preserves NaN behavior.
The same Actor retained BreadCrumbManager and LiveBreadCrumbSelectionSource
execute TeleportSomewhereAppropriate825f0e18. SetRespawnPosition82ce17f8 writes
an actual override, right foot and offboard=true. ChaseMan is restored and LW
lock released before breadcrumb collision/LW queries; final capture fields
clear only after successful teleport. Existing breadcrumb pending last-resort
or nonlocal AI-controller branches remain explicit boundaries, not substituted
spawn points. Career Photo/Pro start teleports remain outside selected freeroam.

Backend setters travel in completed PhysicsFrame commands to the existing sole
app TricksBackend. Constructor82570778 initializes both Jacked booleans false.
UpdateJackedOverlay8256db90 consumes them per completed frame, checks original
kbShowJackedOverlay and requests actual frontend state42. gFEStateInfo at
82fded80 +42*20 has pause=1, dimmer=1, HUDstate=1. Existing frontend input/pause
owner admits that request; AptNext2 (A) alone closes (ReceiveInput826598f0).
Jacked OnEnter82642450 opens screen68; its original template entry at
82fdb358+68*16 has screen type0 (flat2D), transition0. It does not run StartHub's
sim-rate reset. No new pad polling or second backend state machine was added.

Original screens.big jacked/jacked_overlay assets now load in shared UI slot13
with APT fingerprintf5a0ff8e, shared button_menu import/fonts/localization and
the existing dimmer movie. Authored intro1, introcomplete39, outro40 and
outrocomplete54 plus all alpha placements remain original timeline data.
Constructor init-action character16/1304 and root12dc are source translated;
actions1420/143c/1460/147c dispatch dimmer and completion. Button help reuses
same audited button_menu layout with A/ID_COMMON_CONTINUE and white24px
Marker_streak. Raw asset audit copies remain artifacts/jacked-source; shipping
assets continue to load from original archive. Frontend audio remains absent.

Original cvars1204 kHOMSlamThreshold=750 (integer0..10000),1205 kJackedTimer=4.5,
1206 kbShowJackedOverlay=true added. Eighteen changed Rust files passed syntax
parsing with rustfmt --emit stdout only. NO compilation, tests or game runtime;
therefore no live/pixel parity claim. Parent still completing wider NPC flows.

### Crosswalk transfer predicate

Implemented IsNearCrossWalk82706a60 using shared RAIN intersection containment
lookup (not nearest intersection), actual core.position_xz (+160), then full
XYZ distances to original record corners64/80/96/112. Native squared-distance
subtraction uses <=0 inclusion. Raw82706acc confirms core pointer at pedestrian
+724, PDB LWEntityCoreData confirms mPositionXZ+160. Existing point_data owner
reused through intersection_data wrapper; no second spatial index.
Constructor826fd468 only writes distance when attribute exists; factory82704b10
allocates40 bytes via LivingWorld::MemBase allocator flag1 then calls constructor.
Absent authored distance remains explicit unresolved allocation-state evidence,
not silently assumed0. Syntax parse passed, no build/runtime.
CanRecognizePlayer manager8271a1c0 raw insrdi resolves full attribute key
2466b253f1c06fb4 (byte/default0); handed actor-tracker integration to plugin agent.

### Prop-driven spawn and guard audio follow-up

Root owns prop-driven spawning next. Source827215f8 decompiler truncates after
virtual GetPropsWithinRange; raw instructions retain full flow through82721dac.
Implemented Prop.DecideToSpawnPedestrian82e64938 using global Bernoulli, failure
resets actual default timeout only. Added native admission/first unlocked waypoint
and orientation*spawnOffset+location prefix (separate vmul/vadd). Full category
selection/spawn/transfer/failed-spawn cull still needs live integration.
Recovered ScaledDiscreteRandom82704940 raw unsigned conversion, two fmuls,
cumulative fadds and ble(!GT) semantics; helper added to existing census target
module rather than duplicating RNG. Caller must consume global word for empty
input too. Prop categories weight = group member probability * matching entity
PropUseProbability, first matching source key; capacity32, original order.

Guard speech event previously errored unnecessarily. SendSpeechEvent.Begin
826ed200 invokes Pedestrian.SetAudioState82e94ca8, which only stores existing
mPedStateOut.mState. Adapter now performs that exact store; audio playback
remains downstream. Plugin agent informed to reuse same StateOut owner.
Syntax parsed changed sources only. No builds or runtime tests.

### Prop population selection and shared transfer expression

Added census.targets.prop_target implementing raw8272199c..82721be4: iterate
categorygroups weighted members then every entity member, inspect each entities
PropUseProbabilities in order, first full64 matching source key, weight=prop
probability*group probability, append key/weight (capacity32), sum with original
fadds order and shared scaled global draw even for an empty list. Uses existing
reference-array decoder and bounded probability fields, not a new Vault parser.

Extracted check_transfer_conditions from normal transfer pass for reuse by the
prop spawn path. Native spawned pedestrian goes straight to source expression
and TransferBehaviourToSink; it does NOT repeat normal retry/radius/sink
Bernoulli admission. Failed source expression or transfer requests cull on the
new pedestrian. Full spawn adapter still pending actual ground/Simulation RNG
threading through Update1/initial census; root owns these API changes next.
Source prep budget: initial600 successes, otherwise1. Position/orientation:
waypoint.location + orientation*offset, look toward +orientation if offset<=0,
otherwise toward -orientation. Spawn flags from raw82721cac..ce0 require final
signature review before live call. Syntax-only validation, no builds/runtime.

### Prop-driven spawn live census connection

Replaced/removes old evaluate_no_plugin_pedestrian_spawn placeholder. Existing
Services.prop_driven_spawn now uses sole PropMan spatial results (capacity256),
original initial600/normal1 success budget, inner-radius filter, real source
admission/Bernoulli/first unlocked waypoint, point census minimum and original
weighted entity selection. Reuses PedestrianCensusFrame.spawn and its existing
model/residency/security/capacity checks. Source raw82721cac..ce0 flags:
initial,true allowCull,true force,true worldCollision,false ignoreNavGround,
allowHandProp=!Prop.SpawnsHandPropsOnUse. Last is original props array length
keye15e856f2ca9b96b, read through retained characteristics instance.

Uses shared check_transfer_conditions then existing transfer/copy registration;
rejection invokes concrete request_pedestrian_cull. Successful transfer resets
source timer and decrements budget. No duplicate spawn factory or behavior owner.
Ground and optional same SimulationCommandRandom threaded through initial
census/Update1/population; startup borrows actual bound input RNG, tick reborrows
queued RNG before Update2 consumes it. Immediate mode requires RNG only if an
odds expression executes. Census Manager RNG remains separate and restored on
all result paths, including failed callback prefixes.

Seven changed integration sources syntax parsed with rustfmt emit-stdout only.
No cargo/check/tests/playtest. Crosswalk omitted-distance
allocation behavior, actor tracker/conversation, remaining plugin callbacks,
rendering and stream lifecycle gaps still prevent completion claim.


### Five concrete waypoint plugin families

Generated factory now admits Sit9,UseATM11,UseNewspaperBox12,UseVendingMachine14
and UseWaterFountain15 through retained shared delegate bindings. Each source
copy owns a new same TemplateHC graph and actual saved-switch instances;
null CreateInstance callbacks allocate no state. Existing graph cache, parent
selection, expressions, source/sink list and per-entity plugin lifecycle remain
sole owners. LookAt/Trash adapters unchanged.

Native callback sources re-read: LockFirstWaypoint82708020 (first point only,
not first unlocked), TargetWaypoint826ebe90/82708190/826ebf30, facing
826ec058/826ec1d0, lock position826ecf78/826ecf18, explicit direction
826ec968/826ec9d8, monitored packet ctor82723308/Begin82723700/End827235d8,
increment826ec698, speech826ed200 -> retained PedStateOut.state. Packet stages
reuse IntentQueue.create_monitored/suggest/release; speech states validated
against original enum0<state<72. Shared ComparisonCondition constructor and
EvalCompare retain native unordered semantics; facing uses original VMX cosine,
and distance the shared two-refinement native length. Conditions re-read
826f0b60,8270c550,826efef8,826f1ac8; raw8270c6c8 confirms comparison reads
computed length, despite misleading decompiler temporary alias.

Sit adds original source-DMO interaction monitoring and NavRig ignore target
(SetInteractingObject82e6ff80). Existing TargetingWorld now borrows that actual
DmoObjectMan; no duplicate object registry. Native RegisterEntityMonitor82d38488
silently declines at four entries, and SetInteractingObject ignores its boolean.
OwnPluginObject uses state flag byte1 bit1 as native; collision-unlock End clears
byte2 bit80 and starts timer42 with original cvar1208 kPluginUnlockTimer=2.
SetSitTimer826ec878 reads high69f67c678b2673c9 then low1190326371f1a684 and uses
same LW random to set timer24. GoingToStandBackUp826f1dc0 uses actual entity
attributeb040d387aba6e24d and existing rw::math Bernoulli stream.

All thirteen distinct generated update functions for these five families were
re-read from IDA. Empty leaves remain empty only when their real Update slot
is825187c0. TargetWaypoint callbacks execute slot0 at828f8798 or slot1 at
828f87b0. Native active counts are checked, preserving folded caller identity.
Eleven changed plugin files syntax parsed only, no compilation/tests/runtime.
Conversation,ActorTracker and Spectate controllers still require their concrete
nonlocal owners and remain explicit factory failures at this checkpoint.

### Reference-build conversation absence and primary chase audio

Re-read LWPropFactory.InitCallbackMap82e694f0: waypoint_conversation and all
collection descendants register folded callback82784530. Raw body is exactly
li r3,0; blr. No ConversationArea concrete constructor/vtable in linked symbols.
Existing Null spawn callback therefore correct, not unfinished conversation
feature. Transfer EntityAllowedToJoinConversation82706bb0 and GatheringMembers
82706cc0 return false on missing ILWConversationArea; admitted Prop/WaypointGroup
has no such interface. Implemented that real false branch, not fabricated
conversation state. Plugin agent informed not to invent missing native owner.

NewChasee.Begin82708f98..9048 gets actual primary chaser index0 and its entity,
then entity virtual+184 SetAudioState(15); null paths do nothing. Replaced
primary-chaser speech placeholder with writes to same pedestrian StateOut,
including self-chaser and actual peer resolution. No new audio owner.

Crosswalk omitted-distance investigation continues: MemBase ctor82e4f958 selects
XAllocator.GetCoreAllocator8296f4e0 LivingWorld category; CategoryAllocator.Alloc
82973570 forwards masked/or flags to Internal.Alloc82972a18. No clearing seen in
these wrappers, so no evidence yet permitting a zero assumption.
All changed sources syntax parsed only; no builds/runtime.

## Crosswalk omitted-distance allocator audit

IDA audit continued through MemBase82e4f958 -> GetCoreAllocator8296f4e0 ->
CategoryAllocator.Alloc82973570 -> Internal.Alloc82972a18 ->
AllocHeapMem82972240. Both small-object and normal heap paths clear the returned
bytes only when flags & 0x100. The LivingWorld tCategorySetup at82ff0c88 is
24 bytes: name82167780,limit00400000,heap IDs6/0,flags mask0,flags OR0.
AddCategory82975158 installs these exact mask/OR fields; the category allocator
therefore removes the factory's flags1 and does not request MemClear.
Constructor826fd468 writes mDistance only when the descriptor has distance.
This does NOT establish a zero default for omitted distance. Keep that case
explicitly unresolved rather than inventing a crosswalk radius. Further work
must establish any other initialization or an explicit compatibility decision.

## Spectator position review

PDB PhysOut+36 is mSystemReckoning; its +48 member is mCentreOfMass Vector3.
It is not a skeleton root matrix. Sent this verified layout to the Spectate
implementation owner for TurnToFaceSkater826fedc0/IsFacingSkater826ffea8.

## DMO movement to pedestrian monitor connection

Connected DynamicObject.UpdateTransform82d3d148 -> NotifyMonitoringEntitiesOfMovement82d3bc98
before shared LWBasic target/velocity publication. Original SMALL_FLOAT_153 at82327050
is bits37800000; any XYZ absolute delta exceeding it triggers callbacks over a
snapshot of the existing monitor list. The sole DmoWorld adapter resolves the
same live pedestrian roster, no secondary event queue or roster.

Pedestrian.MonitoredObjectMoved82e91fe0 compares only mpInteractingObject
(full object+6704; IPedestrian biased+1984). Small shifts accumulate in AI state
mDMOShiftAmount+3136 and set byte3243 bit1; large shifts request deferred plugin
removal. Owning-plugin byte3241 bit1 and !IntentGen.mAvoidJumpPacket.InQueue
(+960+456) gate the existing Forward jump-avoid packet and shared suggestion
queue. These fields are not grounded/IsJumping despite misleading pseudocode.
PDB virtual LWBasic slot36 confirms RequestBehaviourPluginRemovalDuringStateGraphUpdate;
IPedestrian slot60 confirms SuggestNextIntent(packet). Existing avoid packet
constructor reused, not duplicated. kPluginDMOShiftTolerance initializer82faace0
(default0.005, range0..2) registered by peer as cvar1210.

Unlike the separately heap-allocated crosswalk condition, pedestrian AI state
comes from MemStore: Init82a6de38 explicitly fills bytes zero and FreeBlock82a6df68
clears reused blocks. Thus initial dmo_shift is now Some([0;3]); Reset82e92778
correctly leaves its accumulated value alone while clearing the apply flag.
Five edited Rust files syntax parsed successfully; no build or runtime test run.

## ActorTracker source owner and startup dependencies

Added real ActorTracker payload to the existing Prop/PropMan: one original
store5 allocation, recognizer vector16; no duplicate entity roster. Native
ctor82e675f8, RegisterRecognizer82e682f0 (unique, ignorefull), unregister82e66e88
(firsterase), destructor82e66de0 verified. Prop factory callback now constructs
this payload, and ActorTracker PreState82e64990 replaces the ordinary timer
update with actual local Actor position (current translation/XZ only).
Manager.InitPropData8271d610 reuses the shared SpawnProp path and original
actortracker_spectator category, identity/zero translation, allowCullfalse.
It runs after manager construction in host startup (graph preload occurs earlier
in this host). Original PreLoadStateGraphs8272550c invokes InitPropData; explicit
second source-list append8271d708 is preserved even though base Prop ctor82e6736c
can append already. Manager pointer is an ID into the same PropMan, not a copy.
CanRecognizePlayer827067f8 uses actual tracker count and entity byte attribute
2466b253f1c06fb4 via Manager8271a1c0; preserves evaluation even if list is full.

Original VLT RefSpecs inspected through existing tools/vault_dump.Vault for all
livingworld_entities collections: animation00367b8f33e79c43 points to53e332f67b8124b2,
chasefbc42525e74d946f to1477b0d5343d29fd, locomotion8a973d5659a65f1b
tod521592a5af8ecc1, takedown76ac331fbd5803ff to3032dc4533789895.
Animation class was added by plugin peer; root added the otherthree classes.
Selected Vault only loads selected classes and try_ref_spec_instance requires
the target collection already loaded, so this fixes a construction blocker.
Six ActorTracker-related files syntax parsed; no compile/runtime validation.

### Spectate, ActorTracker plugin execution, and real Plugin AI callback

`pedestrian_plugin_spectate.rs` now shares concrete callbacks between generated
spectate(index10) and actortracker_spectate(index6). Bindings remain shared in
ActorGraphStartupOwners; copied controllers receive fresh original instance
payloads. Source export: `pedestrian-plugin-callback-source-2026-09-30.json`.
Generated Updates828f84c0..828f8778 and828f8068/80d8/8148 preserve occurrence order.
Original false useAudioThresholds in both families avoids inventing a crowd
audio-threshold service. Spectate826fdf40 uses actual perception event/score,
timers9/10/23, same Manager RNG, registered channel remapping, and audio-state
output. Pending delay/channel early return and unordered BGT fallthrough are
retained. Head stack oddity is retained: Spectate.End pops independently of
StandAndWatch.End. StandAndWatch826feb10 follows the actual local Actor COM,
with flattened saved COM in prediction and original multiply-add order.
TurnToFaceSkater.Begin826ec9e8 invokes SwitchNavigationMode(1), verified raw
Entity vslotB0; it does not send audio. PhysOut+36 is SystemReckoning and+48
is COM, verified PDB; skeleton position is not used for facing conditions.

OverrideAnimData826ff038 uses the original class53e332f67b8124b2 and marquee
attribute, setting the override flag before binding. Shared selected Vault now
loads this class. Spectate collisions reuse existing CollidingBehaviour and
FrameServices implementations. ActorTracker RegisterRecognizer callbacks use
root's actual PropMan pointer/payload and preserve absent-owner no-op behavior.
PlayerRecognition82717410 consumes Manager RNG, sets state42/45, remaps GreetChannel
even when busy, requests playback only when Finished, then pushes COM tracking.
MaxRecognizersExceeded preserves count>max (different from transfer count<max)
and always evaluates CanRecognizePlayer with a present tracker. SimpleRandom
uses the original SimulationCommandRandom passed through the actual frame by
the flow peer, never the separate Manager stream.

The actual outer schedule is Behaviours.Plugin.Update826fdcc8 within the AI
graph, not an additional update alongside it. Frame callback now resets the
plugin-root retry timer and executes the attached controller through full
FrameServices. End826eb098 invokes clean End then RemoveBehaviourPlugin82e79ca8:
head OnRemoveBehaviourSource82e743c0 (including dormant slots and slot2-only
stack decrement), ClearCurrentWaypoint, source sink removal, copy destruction.
Prestate removal and both cull callers reuse this sequence. Old targeting-only
plugin entry points were removed after caller migration. Cvars1209..1212:
kSpectateMaxDelay.5, kPluginDMOShiftTolerance.005, kMinPluginTimer30, enabled
kEnableActorTracker; all original initializers/ranges recorded in catalog.

Confidence high, source reviewed and eleven edited files syntax parsed only.
No build, tests, or runtime. Remaining shared collision-service boundaries
(player collision statistics/backend14 and guard-knockdown challenge messaging)
are explicit errors in their existing owner; adding these plugin families does
not claim those callbacks complete. ConversationArea is intentionally absent
in the linked source factory as established by root; no fabricated owner added.

## Collision statistics and guard knockdown callbacks

Removed two live collision service placeholders. Manager.UpdatePlayerCollisionStats
826f55e0 calls LWBasicEntity.GetType82e4bfd0, which returns the full entity key
(not census category). Raw PPC insrdi/cmpld restores fourteen original64-bit
comparands that the decompiler omitted/misrepresented; they select statistics
52..63, with default51. Existing LocalBackendOwner.update_additive_statistic
handles original module14 recording gates and adds1 with forcefalse. No new
statistics store. The selected session already constructs module14 equivalent.

PedestrianColliding.Begin82716b1c..b54 now broadcasts the real guard LW ID via
existing ChallengeEventBroadcaster/queue7. Header kind is -2000196102, size20,
port assigned by broadcaster, ID0. This preserves the source event; it does not
claim a career challenge dispatcher or platform achievement implementation.
Career-only GoodSamaritan for a nonlocal Actor remains outside selected solo
freeroam admission. Both changed Rust files syntax parsed; no compile/runtime.

### Chase-end CentralHUD delivery (source reviewed, no build/runtime)

`ChaseMan.ShowHUDChaseEndMessage82713cc8` now uses the existing global Math RNG
(including the unconditional escape message), `%101 * 0.01`, original caught/
tazer thresholds and original localization keys. Both guard completion and
`UnRegisterChaser827186e8` escape reach the existing local-backend command route
and completed PhysicsFrame frontend consumer. The selected freeroam session has
no initialized career owner; the optional RunningMan achievement remains outside
that session, rather than being fabricated.

Original `CentralHUD.ShowCentralMessage826bbf00`, Update826bc398,
ForceCentralHUDOutro826bc348 and Clear826bbe90 drive the shared APT clip owner.
The 180 counter counts frontend game updates, independent of dt. The actual
screen template76 points to `hud2/hudcentral.swf`, type0 (flat2D), and the original
APT fingerprint is d85e4167. Shared localization/font metrics/glyph atlas render
the original text children. Native white selects ScreenManager off-white;
chase messages supply four empty formatting parameters. Original APT intro,
idle and outro labels retain their authored transforms/visibility. HideAll
calls HideMessage on normal/modified message then stops the root; it does not
hide the district clip. Existing marker CentralHudForceOutro now reaches this
same component. No invented backing geometry is added for the absent mBacking
referenced by this shipped asset's script.

Read-only source extraction/inspection: artifacts/central-source/hudcentral-audit.txt.
The source movie has no imported textures; the existing shared fonts suffice.
Frontend audio backend remains absent as previously documented; original
FE_SOUND_CENTRAL_MESSAGE_DEFAULT and PlayGenCentralMsge hooks are retained but
not audible. Full HUD component ordering/pixel parity and pause scheduling have
not been runtime verified. Rust syntax parsing only passed; no builds, tests or
game execution were run.

Direct Greet mood delivery can now reborrow an actual peer through
FrameServices.receive_direct_mood, using the same ProcessNewMoodEntry implementation
as ambient delivery. Both actual source/target remain resolvable during processing,
and owner references/lookup keys are restored before returning either Result.
The greet caller owns threading the real StaticGroundPool into that callback.

## Waypoint streaming teardown (source-only, continuation)

Manager.DeRegisterWaypointData82722088 clears waypoint data before requesting
Prop culls. The streamed path previously called the low-level PropMan cull
without executing attached pedestrian plugin End/remove. It now reuses the
same recursive Prop cull owner as pedestrian destruction, with actual physics,
Actor roster, painter, avoidance definitions and retained Manager RNG. RNG is
restored on both success and error. No duplicate simulation or cull queue.

Prop.OnCullRequested82e65bf0 raw PPC82e65c0c..50 clears every transferable
copy source through82cef358, clears the copies list, then dispatches source
vtable+28 ForceUnRegisterOfAllSinks82e65418. The shared cull now preserves that
order, so plugin End sees NULL. AttachedPlugin exposes mutable source state
through the existing shared instance; no copied controller or state graph.
ForceUnRegister invokes sink clean-end, removal, then source removal. Existing
parent DeRegisterAllSinks82e50318 still runs before contained-child recursion.
Seven edited Rust files syntax-parsed only; no build/tests/runtime executed.

### Remaining chase leaves connected through the real frame

FrameServices now executes InterceptChasee82709290/82709388,
LostChasee82709af8/82709df8, GiveUpBeingPrimaryChaser826ff270,
TellChaseeToFlee8270b628, SendChaseStateMessage827124c0,
ChaserEvaluateWhoToTakeDown82709e58 and EmotionalResponseToGivingWarning827122c8.
Evidence export: pedestrian-chase-callback-source-2026-09-30.json.

Interception reuses the existing begin_chase_navigation and InterceptTarget
solver; native Y tolerance remains a live cvar, maximum intercept time is a
runtime chase attribute, and exhaustion adds the actual dt. LostChasee uses
retained knowledge for its distance/turn direction while navigation targets the
actual current Chasee position. Its End disables explicit turning. GiveUp shares
the existing primary switch owner, updating timer29 before roster reordering.
The evaluator's ILWTakeDownSource virtual slot20 is SetTakeDownSinkId82e7aa50,
a direct write to the existing state sink. Flee invokes the actual target's
mood processor with event17; warning enqueues ambient self/self event12.
SendChaseStateMessage emits original kind-268789995, size24, state0..4 on the
same ordered GameSim queue; selected startup still has no registered frontend
security-map subscriber, so no map icon state is fabricated.

UpdateBlockPrediction827096a0 now invokes Chaser82de3220 over existing retained
should-block/position fields. Radius and recommended offsets come from the
actual Chasee formation owner; arrival radius, approach speed/angle and maximum
distance remain runtime chase attributes. The source's two different vectors
are preserved: normalized XZ delta for speed, negative original XZ delta for
angle/projection. VMX source inspection shows the projected displacement uses
recommendedOffset multiplied by dot(negativeDelta,recommendedOffset), followed
by separate addition to Chasee position. This unusual expression is not replaced
with a geometrically plausible normalized projection. Raw scalar BGE/BLE retain
unordered-value branch behavior. Parent independent source review requested for
this unusual VMX sequence. Confidence high for ownership/branch wiring, medium
for this vector-expression interpretation pending that review.

Cvars registered with exact source defaults:1220 intercept Y tolerance1;
1221 marquee face false;1222 marquee hand true;1223 takedown Y tolerance1.25.
No duplicate cvar owner; new registrations coordinated among agents. Syntax
parsing only passed. No compilation, tests, or game runtime was performed.

## Guard takedown lifecycle and actual Actor wipeout input

Root connected AttemptTakeDownTargetable.Begin82717660/Update8270a038/
End8270a2e8, Failure8270a398/8270a528, Success82717918,
TakedownTauntVictim8270b100/826ee3c8, TazeWantTarget8270bb48 and prediction
82709f70. Agent's DetermineTakeDownType82e7a6c0 consumes the original typed
40-byte tAnimTakedown Vault array; root Begin uses the chosen descriptor and
mirror bit with existing monitored packet/queue/head owners. Monitored hashes
read from native pointers are ActiveTakedown and SGIntent. Failure uses native
tripChance default0.35 and the shared global Math Bernoulli stream only on its
original branch. Success changes stage1 from TakedownFailed to TakedownSuccessful,
clears RequiredToBeginColliding and sets the existing collision-suppression bit.

PDB cLivingWorldCollision+8=RequiredToBeginColliding. ITargetable+16 for Actor,
regular pedestrian, guard, DMO and vehicle is folded826c6d80 returning1;
BigBlack vtable returns0. ITargetable+20 is empty for those non-Actor classes.
Actor.ForceWipeOut825edad0 invokes LivingWorldWipeOut825ed130: normalize actual
position-source (zero fallback), store existing mLivingWorldImpulse, set Actor
input-generation flag40000000. Implemented in existing actor_physics_auxiliary
owner, feeding existing SetUpPhysics suffix and skeleton processed animation
input. Downstream consumption/reset separately under review; not runtime proven.
Taze uses IChasee.GetNumChasers slot32 and SetGroupEndChaseReason slot24 before
local-player HUD notification. Entity vtable52 is IsLocalPlayer (not IsActor).

Independent review found BlockPrediction's slot68 producer must read
GetChaserInterceptDirection82de2628, not recommended position. Correction sent
to owning agent. Raw VMX multiplies that direction by dot(-unnormalized delta,
direction), then adds chasee position; native ordering retained.
All source-only: syntax parsing, no compilation or game tests.

BlockPrediction independent review correction: root identified IChasee slot68
as GetChaserInterceptDirection82de2628, loading ChaserInfo+32. The adapter now
uses that actual direction, not recommended_position. Thus projected displacement
is interceptDirection * dot(negativeDelta,interceptDirection). The earlier
paragraph's recommendedOffset expression is superseded. Missing entries return
zero as source. Further raw audit of tChaserInfo ctor82de1fa8 found explicit
zero stores at1fc4/1fd4/1fe4/1ffc/200c/201c/202c/2044/2060 omitted by Hex-Rays;
its three vector fields are initialized zero, so retained intercept_direction
now starts Some([0;3]) instead of the previous unsupported None. NormalizeXZSafe
82de2290 sets Y/W zero and delegates directly to Math.NormalizeSafe824b18e0,
which is precisely the shared try_normalize_safe helper (same epsilon squared,
ordered positive gate and two reciprocal-root refinement iterations).

### Prop cull order follow-up
Independent raw/decompile review of RequestExplicitCull82e675b0,
RequestCull8271a5e8, LWBasicEntity.OnCullRequested82e502a8 and
LWBehaviourSink.OnCullRequested82e4c500 confirmed the callback-before-detach
order. Prop destructor82e66840 notifies pedestrians then removes BehaviourMan
source at82e668bc. Removed premature source-list removal from the recursive
request helper; existing process_culls retains sole destructor ownership.
Also moved allow_culling=true to the request prefix, before plugin callbacks,
as native82e675d0 requires. Syntax parsing only; no compilation/tests.

### Static end-to-end callback audit after chase integration

A source-declaration audit found and repaired two live dispatch gaps:
* Flee Begin826ed228/Update826ed288/End826ed398 existed in targeting but were
  absent from FrameServices dispatch. They now reach that existing owner, with
  the actual want10 source resolved on the first Begin as well as later Update.
* SendSpeechEvent Begin826ed200 had a service setter but no leaf binding.
  SimpleBehaviour now retains the ctor826ed130 speechvalue (float-to-signed
  truncation, original enum assertion1..71); Begin writes existing StateOut.

Static comparison of every native pedestrian lifecycle against local bindings,
frame address dispatch and guard/chase named dispatch finds no unmatched native
callback candidates. All89 declared conditions have constructor cases. All six
non-null instance factories have retained per-occurrence resets: modifier,
DrawTazer, Patrol, Colliding, StopAndFace and WatchWant. Native-empty addresses
alone bypass callback execution; named declarations do not generically suppress
unmatched nonlocal callbacks. This is source inspection, not a passing build or
runtime coverage claim.

Session inspection confirms actual preframe passes simulation RNG and ground
owners into complete prestate/state/poststate; current-frame query reception,
modifier passes and intent update share the same live EntityMan payloads.
Normal callback audit found no additional missing-owner path beyond the known
unwritten crosswalk transfer distance. Existing explicit feature boundaries
remain: initialized BigBlack/protector owners, optional career achievement
services, dynamic graph fallback when generated selection is disabled, and the
Marquee presentation work owned by the presentation agent. These boundaries are
not substituted with guessed values. Pixel/runtime parity remains unverified.

Independent original XML/binary crosswalk descriptor audit completed:
[pedestrian-crosswalk-descriptor-audit-2026-09-30.md](pedestrian-crosswalk-descriptor-audit-2026-09-30.md).
All six packaged conditions contain only the name attribute; fresh native
per-element maps provide no inheritance/default. Complete binary decoding and
matching original XML rule out host attribute loss. The known unwritten distance
remains unresolved; no radius was invented or gameplay path bypassed.

## Census destruction before replacement (source-only correction)

Re-read IDA CensusMan.Update82702780 and EvaluateCensus82727a38. Update
advances/reset pedestrian reserve, samples backend sliders, then drains
pedestrian, vehicle, prop, DMO queues in that order. EvaluateCensus invokes
this between requested culls and spawning (82727c58/82727e54). The selected
bridge previously drained only DMO and vehicles there, leaving detached
pedestrian allocations until next frame. Shared try_update_census now reuses
all existing destruction owners in native order at both schedule points.
Initial evaluation uses enable && !sampled_fe for the initial Update.
Existing CensusState.update_settings is now called rather than merely setting
sampled_fe. Current NoProfileFreeSkate uses native sliders1; mode3 explicitly
requires unavailable real profile sliders rather than fabricating settings.
No new entity queues/allocators. Rustfmt syntax parse only; builds
and runtime validation pending.

## Remaining recipe-release audit (not implemented yet)

SelectedRecipeFrame Bridge::deregister_simulation currently rejects any
populated pedestrian/vehicle roster; unload still reports completion pending.
This must not be mistaken for completed streamed model-release support.
IDA ModelMan.RemoveRecipe82ce8cd0 calls Manager.NotifyOfAssetRemoval826f5508
before removing its recipe map entry. Manager visits ped/vehicle/prop/DMO.
Ped component82e82948 invokes LWBasicEntity virtual slot0x6c, raw82e829dc.
Pedestrian vtable823a4e70+0x6c resolves825187c0 (empty folded callback).
Thus a nonempty pedestrian roster alone is NOT a native removal failure.
Do not implement forced pedestrian culls from this notification. Remaining
work: verify all other entity overrides, presentation notification and actual
asset unload completion before replacing host pending guards. No live recipe
release caller found in selected_living_world_recipes during this pass;
ordinary census despawning uses its separate already-connected cleanup.

## Independent guard acquisition, pursuit, takedown and despawn review

A fresh IDA review followed the existing live owners rather than adding a
parallel chase implementation. CanNewChaseStart82e6edc0 delegates to the
retained ChaseMan gate. CanChaseeAddNewChaser8270ca40 resolves want1's actual
Entity/IChasee and checks its capacity/end reason. NewChasee82708f98 retains
its unusual native ordering: speech on the previous chasee's primary chaser,
reset takedown count, remove from the NEW want target when an old chasee
exists, clear own chasee, then add to the new target. The existing callback
already preserves that ordering; replacing the removal receiver with the
old chasee would change the original behavior.

PursueChasee827099b8 consumes the existing recommended position, sets the
chasing flag and maximum seek speed, increments exhaustion by graph dt and
sets navigation target mode1. BlockChasee's rest path uses RestChaser82e6fd08:
subtract dt and select zero for a negative/NaN remainder. Existing services
match these operations. CanAttemptTakeDownTargetable8270c960 reads the live
disable cvar, resolves the sink, rejects wipeout, then checks the previously
selected takedown type. It does not perform selection inside this condition.
AttemptTakeDownTargetable.Update8270a038 checks the existing ActiveTakedown
monitor and active packet, leaves a completed status alone, and sets failure3
for stage1 without required collision or success2/failure3 from the matching
collider's CanBeTakenDown result. End8270a2e8 releases the monitor before
popping head tracking; it does not cancel the queued packet. Failure.End
8270a528 clears state flag1 bit0x80. Current owners match each reviewed branch.

Despawn was checked against Pedestrian.OnCullRequested82e85fd0,
SecurityGuard's forwarding thunk82e86280, ChaserOnCullRequested82e6edf0,
ChaseeOnCullRequested82de4b70 and indexed RemoveChaser82de4a08. Following base
cleanup and hand-prop cleanup, native removes this entity from its chasee,
reverse-removes its own chasee's chasers, then unregisters this entity from
ChaseMan. ChaseeOnCullRequested does not send an extra callback or reset each
removed chaser's target. The existing request_pedestrian_cull sequence follows
that order, before cull messages, execution/roster detach and retained
teardown. Local chase registration effects still run before the empty
chasee clears its group end reason, preserving captured/escaped reporting.

No additional missing normal-freeroam service was found in these reviewed
callbacks. Corrected one stale ChaserInfo comment: its intercept direction
is already initialized to zero by the implemented raw-ctor correction.
This is source comparison only, not compilation or runtime verification;
it does not establish that the entire remaining pedestrian goal is complete.

Independent Prop cull branch correction: raw8271f57c is40980090, BO4 BI24,
i.e. branch if CR6.LT==0 after fcmpu. This includes unordered, so Rust must
use !(distanceSquared-radiusSquared < 0), not ordered >=. Corrected the
new adapter; previous ordered-comparison claim is superseded. Existing
vehicle comparator was already correct. Syntax parsed only.

## Independent live admission/presentation review

Re-read Spawn82e9a510 against selected_living_world_pedestrian_spawn.rs:
navmesh/forced spawn, density and DMO overlap checks precede factory creation;
model admission precedes cell/target-transform setup, collision-world insertion,
contained props and guard PostInitialize. Newly constructed cell assignment's
zero threshold is consistent with the initially absent target iterator, not a
missing runtime threshold query. Runtime mesh-cell updates retain the actual
target threshold owner. No duplicate spawning implementation added.

Host render review: pedestrian_render selects geometry through
SelectedPedestrianVisual.select_geometry_lod, which actually rebuilds the
shared CharacterPalette for the selected LOD and repacks synchronized matrices.
The returned LOD is not merely a GPU cache label. Opacity updates are reached
from LiveSkatingClient's completed queued-frame branch (live_skating_client.rs),
so repeated uncapped presentation draws do not independently advance spawn
fade frames. No speculative LOD/fade fix was applied.

Readiness remains source-only: population/collision/movement/chase/animation
owners and their live callers exist, and source reviews cover normal census
families. Still unresolved: omitted authored crosswalk distance semantics,
remaining shared rendering details under review, and build/runtime evidence
still pending. Recipe-release support is separately incomplete; ordinary
entity culling does not depend on that release callback.

## Original census admission data audit

Read-only `tools/vault_dump.py` inspection of the original extracted
`%TEMP%/skate-ped-vault/data/db/skater.vlt` and `skater.bin` followed inherited
entity attributes and serialized pointer fixups, including string relocations
in inline nodes. Of92 livingworld_entities collections,19 resolve
Pedestrian.xml,11 Vehicle.xml,38 marquee.xml,2 Photographer.xml,2 Big_Black
variants, and20 have no AI graph. This alone does not establish reachability.
Traversing every authored census group (25), its category member arrays and
entity member arrays yields24 distinct entity keys:13 Pedestrian.xml,
10 Vehicle.xml and default. Default appears only in the default group's
zero-weight entry, so the original discrete selection cannot select it.
No group/category path selects a Marquee, Photographer or BigBlack AI entity.
Thus the existing explicit unsupported-family errors are not an ordinary
freeroam census admission gap in this corpus; no exclusion was added.

All13 pedestrian model RefSpecs resolve to original livingworld_models
collections. The entity keys are3457c13fe6d128f1,58b60bae732256bb,
5ac95e9c35f65bed,5ef2edddc75f25de,6daf17663d47fd14,81518f0edafe70a6,
9a4d5e87ade3af28,a7e81ade65ca24e7,e2a81660722ed157,e6a5ffe68d7d12b9,
ef73f40330bd5d77,f977b957a0e868cb,fe1f4594b6f918e2. Separately, all19
Pedestrian.xml entities have valid animation/chase/locomotion/takedown
RefSpecs (76 references checked), including inherited resolution. Actual
model residency remains the existing RecipeMan/ModelMan spawn gate; this
attribute audit does not prove residency or successful runtime spawning.
No build, test or game launch was performed.

## Prop simulation-detail scheduling corrected

Manager.Update2 calls UpdateLOSStates826f5410 before ChaseMan and execution
group advancement. That manager visits ped, vehicle, prop, DMO. Selected
bridge omitted prop switching. Prop component82e66168 directly calls shared
LWBasicEntity.TestLOSim82e50180, then OnLOSimSwitch (Entity vtable82394be8
slot3c points folded return-true826c6d80), removes old execution group, writes
new level, adds new group. Implemented that existing-owner flow before group
advancement. It affects prop preframe cadence and pedestrian spawn timers.
Extracted the already-used pedestrian distance calculation into CommonEntityCore
so props reuse it; no new radius/settings or scheduling owner. Same fullXYZ
distance and cached high/low radii. Four Rust files syntax parsed only.

## Final live motion-dispatch audit

The generated Pedestrian motion declarations were compared against the live
backend, including ancestor behavior slots rather than only the leaf's local
section. All572 states resolve one of18 implemented update bodies. Across
all root-to-leaf paths, the exact number of active slots matches each update
body and every Animation, JumpAnimation, Monitor and Timer slot references the
correct retained behavior type. All472 behavior occurrences (14 families)
and102 conditions (9 families) have concrete constructors and dispatch paths.
All state and441 transition expression references resolve among the114 explicit
precondition cases; all condition references are within0..101. All441 authored
transition-hook sections are empty. These were read-only source-table checks,
not execution of graph/runtime tests.

IDA independently reconfirmed828f5e40's four slots,828f5ec8's nested six-slot
path,828f5cc8's three empty calls and828f81d0's single empty call. Their empty
updates call actual folded825187c0 (raw empty function), not an omitted source
callback. Re-read full sustain evaluator82925fb8: its21 explicit cases and
condition indexes match the retained delegate, including native defaulttrue.
Both masks use the same delegate condition-result cache.

The concrete MotionObjects backend reaches same-entity LiveServices for
playback/remap, timers, completion flags, jump, channel stop, takedown flags
and speech output. ExternalOwners is implemented by the actual FrameServices
for RNG, hand-prop probability, waypoint interaction selection, pending prop
spawn and original telemetry gate. Frame update restores moved delegate,
objects and intent snapshot even on callback failure. No declaration reaches
an unimplemented dispatch branch in this checked normal pedestrian/guard
motion graph. No code change was warranted; compilation and runtime behavior
remain unverified (no build or test was run).

CommonEntityCore.TestLOSim is now also used by the existing vehicle path.
Removed its identical arithmetic copy so ped/prop/vehicle all share the native
LWBasicEntity distance kernel. Their category-specific overrides and execution
list changes remain with their original owners. Syntax-only parse passed.

## Collision identity/liveness follow-up

Re-read DetermineOverallCollisionType82e808d8 and GetChasee82e86220 against
the actual Island contact adapter. Fixed a concrete liveness mismatch in
pedestrian_collision_contacts.rs: the far-LOSim NPC/NPC suppression branch
previously treated a syntactically valid chasee ID as an existing chasee.
Native82e80cac instead resolves Entity then IChasee; a culled target returns
null. The adapter now queries the retained SkaterManager Actor roster or the
existing split pedestrian roster only on the applicable skater-body/NPC,
nonignored, LOSim2, zero-chasers branch. DMO/vehicle have no IChasee. No new
entity owner or collision classification implementation was introduced.

Contact dispatch retains actual sealed body identities from the Island spy
list. World-body vehicle contacts select AdjustVehicle; interacting DMO
contacts select AdjustDmoInteraction; ordinary world contacts follow authored
adjustment/direction gates. Skater-body contacts resolve actual Actor or
pedestrian owners, honor the existing ignored target and collision flags,
and classify light/full responses. Actor player index drives target identity
and scoring bits; other pedestrian state drives takedown collision behavior.
Every contact reaches the existing classifier and response owner, without a
first-hit break. Light response enters the retained channel and stateOut8;
full response enters record_accepted_full_collision and the retained Colliding
graph/intent packet path. No proximity-based identity substitution was found.
Validation was IDA/source comparison and rustfmt syntax parsing only; no
compilation, tests or runtime contact simulation.

## Active DMO-contained pedestrian interaction gap

Root source trace: LWDynamicObjectCensusMan.Spawn82d31ba0 calls actual
InitContainedProps82d391e0 at82d31d60 after collision insertion/allowCull, before
return. No kAllowContainedProps guard occurs at this call. The override reads
model waypoint groups (+124 vector), uses current entity transform, SpawnProp
allowCull=false, sets containing entity, IWaypointGroup.SetData(source),
IBehaviourSource.Deactivate(operation1), appends original capacity8 child list.
DeInitContainedProps82d2e098 clears waypoint SetData(NULL) before base recursive
prop cleanup. Activate82d2e160 validates optional navmesh offset using actual
waypoint point position/orientation, source Y tolerance and score epsilon;
Deactivate82d2e430 uses active flag80, per-source operation then clears flag.

Work split: ped_mood builds original hotpoint waypoint-group model data using
existing WaypointGroupSource; ped_flow DMO execution/LOSim and schedule;
ped_animation activation/deactivation session helper; root initialization and
destruction bridge. Do not duplicate either existing PropMan or DMO map.
This gap affects NPC use of movable city objects; it is not merely an optional
DMO optimization. Not yet wired/complete at this checkpoint.

## DMO model hotpoint waypoint groups

InitializeHotpoints82d38ec0 now feeds both hand-hotpoint and waypoint consumers
from the existing retained EmbeddedData parser and arena reference resolver.
Type0 preserves last-authored hand transform. Type1 maps to
waypoint_usetrashbin, type2 to waypoint_newspaperbox, and every other nonzero
type to waypoint_sit, exactly as82d38f40..64. Each nonzero mapped key allocates
one retained group with one point; groups preserve authored hotpoint order.
Raw PPC82d38fb4/fb8 stores zero to both GUIDs (Hex-Rays' overlapping v4
reconstruction is misleading). Group entity key is the full StringToKey
result at+72; location copies affine translation and orientation copies
forward. Centroid uses zero-plus-location and the one-point scale of1.

LwDynamicObjectModel.waypoint_groups uses the existing WaypointGroupSource
interface. Its new Arc-backed hotpoint storage variant is the original
runtime-created source group, alongside the existing serialized asset source;
WaypointGroup.SetData, reservation, transform and Prop ownership are shared.
No fabricated serialized file, replacement parser, second waypoint registry
or generated GUID was added. Root owns DynamicObject.InitContainedProps and
lifecycle consumption; this model patch alone is not the completed live DMO
interaction bridge.

Added actual DMO execution cvars1224..1226, defaults30/10/3, from initializers
82f8c058/0b0/108, with original reversed LoFi/NoFi display titles retained.
try_dynamic_object_execution_rates supplies the peer-owned execution scheduler.
Added kContainedWaypointYTolerance1227/default.25 from82f8cdc0 for the
peer-owned activation gate. All changed files passed rustfmt syntax parsing;
no compilation, tests or runtime verification was performed.

DMO child initializer now exists in selected_living_world_dmo_props.rs and is
registered under SelectedRecipeFrame. It reuses model waypoint_groups,
spawn_waypoint_prop, SetData, release_dmo_waypoint(operation1), and retains
child IDs on DynamicObject. ped_flow is wiring consumer-order boundaries.
This is not complete: pending DMO cull records currently retain only body and
grab splines, so child cleanup must be integrated before destruction (including
internal replacement culls), not dropped/deferred past dependent consumers.
Root owns this next. No native cap/callback errors suppressed. Syntax parsed
initializer only, dependencies are concurrent active work; no compile claim.

Contained-prop activation and waypoint release are now implemented in
selected_living_world_dmo_prop_activation.rs. IDA ActivateContainedProps82d2e160
visits the existing capacity-eight child vector, tests waypoint zero only when
Prop.flags bit20 is set (TestNavMeshOnActivation82e66ae0), and probes position
plus orientation times the live Prop test-navmesh offset. The actual MeshStore
uses kContainedWaypointYTolerance (initializer82f8cdc0, default0.25); invalid
cells or abs(GetCellScore(0)) greater than EPSILON_160 (82327080,34000000) take
operation1 deactivation. Otherwise Prop.Activate82e66ab8 sets flags bit80.
The DMO flags bit80 is set after the complete visit. DeactivateContainedProps
82d2e430 gates on that bit and clears it only after the complete child visit.

WaypointGroup.Deactivate82e65718 first clears Prop active bit80, then visits
locked points in vector order. Nonzero operation sets the actual pedestrian's
allow-cull flag, requests deferred behaviour-plugin removal through the existing
state-update flag (IBehaviourSink vslot36,8243a130), clears its current waypoint,
and explicitly unlocks the visited point. It does not immediately End the
plugin or independently clear wants. Operation zero instead uses the existing
full explicit pedestrian census-cull path before unlocking the point. No second
reservation, plugin, Prop, or pedestrian owner is introduced.

UpdateContainedPropTransforms82d2e510 is implemented beside activation: after
the base pass updates every contained Prop transform, each waypoint group
re-reads immutable authored local points and indexes the live point vector by
source point ID at+32. Position uses three native-ordered fused affine steps;
direction uses initial multiply and two fused rotational steps. Centroid uses
the same affine operation, and the final Prop transform has identity axes plus
that world centroid. This avoids cumulative transform drift and keeps
activation queries on the native published waypoint coordinates. Confidence high for traced
control flow; two modified Rust files passed syntax-only parsing and scoped
diff checks. No compilation, tests, or runtime verification performed.

The DMO waypoint transform producer is now connected at its verified native
call site. LWComponentMan<DynamicObject>.UpdateTransforms82d2f598 invokes each
entity's PreUpdateTransform, UpdateTransform, then PostUpdateTransform before
advancing to the next hash-map entry. LWBasicEntity.PostUpdateTransform82e4c2b8
reads the current transform and calls virtual UpdateContainedPropTransforms
at82e4c324. DmoFrame.try_update_post_positions now dispatches that hook directly
after each existing try_update_transform, including its locator publication,
and before the next object, priority or persistence update. The DmoWorld adapter
borrows FrameOwners.props and invokes the same PropMan transform implementation;
there is no duplicate coordinate cache or second pass over the roster.
Spawn82d31ba0 calls InitContainedProps and returns without PostUpdateTransform;
therefore initialization does not add an eager, unsupported transform update.
Three affected Rust files passed syntax-only parsing and scoped diff checks;
no compilation, tests, or runtime verification performed.

DMO safety-index producer audit and implementation: UpdatePositionalData
82d2c708 first invokes base82e50040 (distance, virtualRAIN, virtualWorldPainter),
then combines the RAIN and WorldPainter safety values. PDB identifies complete
object+1100 as mWPSafetyIndex and+1104 as mSafetyIndex; constructor82d3de28/2c
initializes both to1. WorldPainter82d393e8 gathers default collection
d7edbd362d7d2152 then layer14 keys at the actual bbox centre and four retained
farthest corners, preserving duplicates. Class d558c5d63afebcad layout+0 is an
unsigned safety word; first zero dominates, otherwise unsigned maximum starting
at1. Final82d2c708 uses collection991df788b8f61295 if mpRAINData is nonnull,
otherwise1; either zero yields zero, otherwise it takes unsigned maximum.
The mpRAINData pointer is distinct from the on-road flag and is not synthesized.

selected_living_world_dmo_positional.rs now implements that producer with the
existing live Vault/WorldPainter/RAIN owners. It preserves per-selected-target
distance->RAIN->WorldPainter->safety order before all stategraph tasks. The
previous all-DMO road-flag update inside vehicle LOSim was removed; the existing
RAIN helper now runs at the native positional phase and handles the original
IsSetAsHandProp gate82d2a7f0 (clearbit8 without roadqueries). RAIN samples now
reuse retained farthest_corners instead of recomputing a parallel copy. Root
registers the class/module; execution/reset owner wires the positional helper
and real safety-zero reset path. Syntax-only parsing/diff checks performed;
no compilation, tests, or runtime verification performed.

## Shared borrowed frame/prop cleanup boundary

FrameOwners now exposes split borrows of the existing pedestrian roster,
MoodMan, PropMan, mesh, RainMan, ChaseMan, vehicles, event route and BigBlack
identity. It deliberately excludes DMOMan, locators, physics world and RNG,
which a DmoFrame callback already borrows. Its with_entity method is the sole
FrameServices constructor; existing with_entity_and_random_sim delegates to it.
No entity state or manager is cloned or removed to make this borrow possible.

The previous PropRecipeFrame force_prop_sinks/request_contained_prop_cull
implementations were moved to this borrowed context; the original recipe APIs
now delegate. Both paths execute the same recursive child cleanup, shared
plugin clean End/remove, source-pointer clearing, sink removal and PropMan
explicit-cull call. This permits the parent-owned DMO BeforeCull boundary to
run native child cleanup before monitor notifications, rather than scheduling
a second cleanup queue for frame end. Caller integration remains parent-owned.
Rustfmt syntax parsing passed for both edited modules; no compile/tests/run.

## Streaming DMO child cleanup connection

The kind-31 deregistration caller now supplies the same shared
FrameOwners.before_dmo_cull boundary as census/replacement removals. The arena
callback uses DmoCullEvent.BeforeCull before Monitor events, preserving the
original DynamicObject.OnCullRequested82d3cda0 base call before notification.
The shared implementation releases all child sinks, nulls waypoint source data,
then requests child culls before parent monitor callbacks and locator removal.

SelectedDmoPass resolves its avoid-volume view from the caller's existing
PropRecipeFrame before synchronous deregistration, and retains only that view,
the existing Vault Arc and existing painter Arc for the call. This avoids
relocking RecipeMan, whose guard the terrain caller already holds. The context
is cleared on both successful and failed Result paths. Callback lock order
remains physical World then LivingWorld; the one Manager RNG is restored on
every Result path. No extra entity roster or deferred cleanup queue was added.
The context-free route retains its explicit rejection of child-bearing culls.
All original handprop models were independently checked to have no nonzero
hotpoints; see handprop-hotpoint-audit-2026-09-30.json.

The activation module registration was also added. Scoped Rust syntax parsing
passed; no compilation, tests or runtime checks were run. During integration
review, GetSafetyIndex82d362e8 was verified as a stored +1104 field, but tracing
its producer revealed missing native positional RAIN/WorldPainter updates.
Those updates and the reset/capsule flow are active follow-up work; constructor
value1 must not be used as evidence that native safety-zero is unreachable.

### Reset collision geometry correction from independent source review

BuildRecursiveObjectResetList82d37c70 (call82d37dd0) and
BuildObjectLocatorResetListVolumeTestInfo82d38ba8 (call82d38c8c) both use
LWModel vslot72 and dereference the returned first collision volume. The
LWDynamicObjectModel vtable8232ce20 entry+72 is82d16240, whose raw instructions
read model+58. This is the HiLOD world collision definition, not the bbox
volume getter at slot40/model+50. The initial reset translation's bbox-only
queries were therefore incorrect and have been replaced before playtesting.

The shared physics query now accepts the original model's first root and
instances it through the existing append_list_test_instances implementation.
The primitive query delegates to the same pool traversal; an aggregate query
keeps pool ordering and emits each matching body once, rather than concatenating
separate per-primitive query results. The original model AABB still supplies
the broad-phase test at the initial locator transform. Capsule-vs-reset-list
checks use that same actual model root and native mapped-array instancing.
No collision parser, bounding-box substitute, or alternate pool was added.
Both edited files passed Rust syntax parsing; compilation/tests remain unrun.


## Synchronous DMO child cleanup at OnCullRequested

DmoObjectMan now has a shared lifecycle dispatch, BeforeCull followed by the
original monitor-list snapshot and monitor callbacks. DmoFrame uses this for
census culls and internal replacement culls. LivingWorldDmoWorld borrows the
existing FrameOwners, RNG, Actor roster, painter and resolved challenge avoid
volumes through the selected recipe context; no deferred queue or copied
manager is introduced. Initial/population census, pending hand-prop spawning,
Update1 and post-transform frame callers now thread that same context.

FrameOwners.before_dmo_cull implements the verified base-first boundary:
LWBasicEntity.OnCullRequested82e502a8 deregisters every contained prop's sinks;
DynamicObject.DeInitContainedProps82d2e098 then clears each IWaypointGroup with
SetData(NULL), and invokes the existing recursive contained-prop explicit
cull. Point allocation lists are drained there, so the later existing Prop
destructor's SetData(NULL) has nothing left to free. Only after that does
DynamicObject.OnCullRequested82d3cda0 reach NotifyMonitoringEntities82d3bd80,
locator/priority detachment and retained destruction.

Locator removal exposes the same lifecycle callback for the streaming owner.
The narrow monitor-only adapter explicitly rejects child-bearing DMOs before
any monitor callback; original hand-prop models were independently checked to
contain grip hotpoints only, so their existing three narrow call sites remain
valid for the original asset corpus. Streaming context wiring is tracked by
the parent integration task rather than silently bypassing child cleanup.
All six affected Rust modules passed syntax-only parsing (edition2024 for the
existing let-chain module); no compilation, tests or runtime verification.


## DMO reset volume producer and actual board hide

UpdateTransforms82d3d730..daa0 now publishes retained local-skater and board
capsules after per-entity transforms, priorities and persistence. The skater
shape is distinct from the spawn test: radius comes from DMO_Resetting
kSkaterTestCapsuleWidth; half-height is the native refined length of head minus
the unmodified two-foot midpoint. Its basis is I,-K,J and its translation is
foot midpoint plus J times half that length. The board uses the existing
PhysOut SkateboardReckoning effective transform (+64), with the original
kSkateboardTestCapsuleWidth and kSkateboardTestCapsuleHeight. Shared concrete
Volume.GetBBox computes both AABBs; no duplicate bounds routine or spawn
capsule approximation is used. Cvar initializers82f8c810/870/8d0 give defaults
0.35/0.15/0.63 and range0.01..2 (catalog1228..1230).

DmoObjectMan retains these two volumes; absence denotes that their native
UpdateTransforms producer has not run. Constructor82d387a8 initializes shape
dimensions/transforms and Init82d3e030 allocates volumes, but neither populates
the AABBs consumed by reset, so no fabricated initial overlap bounds are used.
No-local UpdateTransforms leaves prior retained volumes untouched.

Actor.RequestHideSkateboard825f0da8 validates the actual offboard PhysOut state
and finds the same physical owner's retained board. PhysicalPlayer request
82dffe38 delegates to SkateboardController.RequestHideBoard82dd17a0: controller
must be active, existing HIDING is a no-op, otherwise the existing actual
InitSkateboardHiding path executes and only then state3 is stored. This reuses
collision disabling, grip and hand-drive teardown, retrieval start, and angular
drive setup. The reset caller owns its native offboard gate before this Actor
assertion. Five Rust modules passed syntax parsing and scoped diff checks;
no compilation, tests or runtime verification performed.

### DMO contained children now initialize inside each spawn

Source `LWDynamicObjectCensusMan::Spawn82d31ba0` invokes `InitContainedProps` at `82d31d60`, before returning to `RequestSpawn` manager attachment and `OnSpawnComplete`. The former delayed roster scan after a census/hand-prop batch changed the shared entity-ID stream, recipe-selection random consumption, and capacity/cull behavior when an earlier object was removed in the same batch.

`DmoFrame::try_spawn` now retains the pending DynamicObject and invokes the shared `PropRecipeFrame::initialize_dmo_props` through its existing `DmoWorld` adapter before attachment. `PropSpawnOwners` splits references to the existing PropMan, ID allocator, CensusTargetCache and DmoModelMan; it does not introduce another factory, random source or manager lock. Each child still uses the original prop spawn implementation, then sets its parent, copies waypoint source data and runs operation-one deactivation before entering the parent's ordered capacity-eight vector. Failed prefixes remain retained. All delayed batch scans were removed. No eager waypoint transform was added: original post-transform publication remains the world-coordinate producer.

The six affected Rust sources passed syntax-only rustfmt parsing. No build, test or runtime execution was performed.


Independent static review of per-spawn contained-prop refactor: PropSpawnOwners
borrows the existing u8 census flag, shared entity allocator, target cache,
model store and PropMan. Recipe selection continues using the existing global
RW RNG at each inline child spawn. DMO Spawn82d31d60 calls child initialization
before parent entity/execution/KD admission and before locator/priority setup;
InitContainedProps82d391e0 preserves parent assignment, SetData, deactivation,
and append order. Old batch initialization callers are gone. A callback error
retains the pending DMO in spawn_unfinished, while admitted/unfinished child
Props remain retained by their existing PropMan owners; another spawn refuses
to overwrite that prefix. Callback reference lifetimes, field reborrows and
crate module visibility were reviewed without compilation. No new defect was
found; this is source review, not runtime proof.

## Source handoff

Final independent gameplay, presentation and lifecycle reviews found no further
concrete ordinary-freeroam blocker. The optional gathering constructor has no
references in the inspected original freeroam assets; see the final reachability
audit in pedestrian-waypoints-2026-09-30.md. It remains explicitly unsupported,
not replaced by another constructor.

Zero distance is used for the six omitted crosswalk fields. Only
predicate evaluation supplies this deterministic compatibility value; authored
bits and parser omission tracking remain unchanged. See the crosswalk descriptor
audit for the native defect and policy distinction. The edited transfer source
passed syntax parsing; no compilation, tests or runtime validation was performed.

Handoff: build-release.bat, then play-quiet.bat. Review pedestrian spawning,
walking/interaction, guard pursuit/contact and movement between streamed areas.
This handoff establishes source-review confidence, not verified gameplay parity.
Original asynchronous startup timing is still subject to the documented initial
reset-capsule publication adaptation. Recipe release/unload completion and
non-freeroam career/protector paths retain their documented limitations.
