# Pedestrian preframe follow-up

Source-only continuation. No compilation, tests, or runtime were run.

Re-read Manager.Update2 `82715a10`, CurrentFrame `82e96ef8`, PreFrame
`82e96fc0`, Task_UpdateStateGraphs `82e73230`, TransformNavRigs `82e735c0`,
CreateGround `82e72320`, CreateGroundDMO `82e72b08`, and AddGroundDMO
`82e789c8` in the live IDA database.

Added selected session adapters in
`selected_living_world_pedestrian_execution.rs`, retaining the existing
EntityMan40, ExecutionMan and MeshMan validator:

- `prepare_pedestrian_preframe`: collects/sorts the native preframe targets,
  then updates all positional data, returning that same target list.
- `update_pedestrian_preframe_navigation`: caller must first complete every
  target's PreStateGraphUpdate/UpdateState/PostStateGraphUpdate. Runs entire
  target list through Seek, ResolvePath, Update, Transform, Validate, shared
  ProcessBatch, Distribute, and Orient. Uses target-specific dt. Pedestrian
  and guard Pre/PostNavigation are the folded empty virtual callbacks.
- `send_pedestrian_ground_queries`: gated original ground Begin/Add/End,
  stopping entity visits when AddTest rejects. Disabled cvar skips the whole
  phase and preserves buffer state, matching the source.

The complete pedestrian preframe wrapper now includes actual prestate
perception/timers/mood/knowledge, threats, needs/wants, Chasee update, zone
interests, handprop update, guard mesh/guardee velocity, AI graph, intent-valid
clear and poststate perception clears. Shared peer adapters own those phases.
Volume and GroundDMO batch producers are also implemented. The outer Update2
now invokes LOS, ChaseMan, execution-group advance, MeshMan.FrameBegin,
audibles, moodables, transfer, pedestrian current/preframe, vehicle preframe,
Prop preframe and vehicle target/query/skitch phases. Transfer uses the actual
queued Simulation RNG, separate from the Manager RNG. Missing predicate paths
remain checked by the shared transfer adapter; this is not a playability claim.

Added query cvars1191..1198 and `try_pedestrian_ground_dmo_tuning` in the
existing InputCvarReferences owner. Source initializers and defaults:

| Index | Label | Initializer | Default |
|---|---|---|---|
|1191|kGroundPlaneTestDistanceDown|82fa6680|2|
|1192|kGroundPlaneTestDistanceUpKnockDown|82fa6740|0.3|
|1193|kNavRigJumpForwardFatness|82fa9d08|0.5|
|1194|kNavRigJumpForwardHeight|82fa9d68|1.8|
|1195|kNavRigJumpHeightFatness|82fa9dc8|0|
|1196|kTazeLineStartOffset|82fa5ce8|0.5|
|1197|kNavRigJumpForwardDistance|82fa3cf8|20|
|1198|kNavRigJumpHeightDistance|82fa3d58|20|

Reused existing UpDefault1102, HeightHeight1097, HeightLead1098 and start
offset1045. GetStartingJumpForward/HeightDistance `82e4d730/748` raw PPC
reads those cvars; previous jump hit distances are not their inputs. Updated
definition count to1199, including mood peer's1189/1190.

Follow-up source results:

1. Manager.Update2 is wired as above; vehicle execution advances once.
   Baseline DmoObjectMan has no ExecutionMan/state-graph scheduler, which is
   a remaining native category gap (PreFrame82d38e50), not an empty callback.
2. Per-target state graph phase now uses all three native callbacks; only
   PostState debug-text history is omitted from the quiet gameplay path.
3. Implemented volume creation: native ped/vehicle/DMO/skater/challenge-volume order,
   consistent nonzero identity tokens for ped/DMO/skater exclusions,
   current wipeout flags, and challenge trigger geometry. Existing vehicle
   builder intentionally uses null_mask (`82d22c08`); preserve that behavior.
4. Implemented GroundDMO creation: actual per-entity facts, including ignore-navmesh
   entity+6724 bit08 and optional taze animation origin+25984. Existing
   buffer owns positioning/jump/taze line construction and submission.
   Positioning base is common core+128 (target-transform translation), while
   centroid height comparison uses entity GetPosition. WipedOut raw
   `82e6eba8` reads AI-state+3241 bit04; IsAbleToJump `82e94cd0` reads
   NavRig+321 (existing distances.do_jump_test), independent of rig enable.

Taze animation origin is the sole published SimLOD bone3 translation, confirmed
by PDB offsets (tAnimData+0x6490 +3*64 +48 =+0x6580). Query exclusion identity
uses per-batch stable nonzero TargetableId tokens; no entity state is copied.
The shared original trigger geometry decoder now serves challenge avoid volumes.

Additional cvars1201 guard mesh cost(.51),1202 plugin Y tolerance(1.8),1203
plugin retry timer(30) are registered with their original labels/defaults.
Pedestrian contained Props now recursively use existing PropRecipeFrame factory,
set containing entity and retain child IDs before publication, source82e615a0.
Prop preframe now advances native spawn timeout and shared positional data for
admitted no-AI Props; graph-bearing Prop construction remains checked elsewhere.

Census RequestCull now invokes real End/plugin/contained-Prop/handprop/chase
callbacks before execution/map detach and retained deferred destruction.
Native request8271a5e8 / pedestrian OnCullRequested82e85fd0 and destructor
82e940e8 were re-read, including SecurityGuard destructor82e96540. The apparent
animation pointer in the decompiler OnCullRequested is actually mpHandProp,
owing to LWBasicEntity's +32 adjustment. The held DMO is removed then culled.
Destructor frees original typed allocations in source order, reuses animation
cleanup and clears waypoint/pathing/collision/RAIN ownership. Detached Prop
cleanup is shared with stream deregistration and now scheduled in Update1.

DMO RequestCull now snapshots monitor IDs and calls actual pedestrian
MonitoredObjectBeingCulled82e7ae88 before priority/locator/map removal; it emits
RequestCull on the same GameSim route. This covers DmoFrame and pedestrian
handprop/cull paths. Stream-forced cull now borrows the SAME physical-world
Arc from SelectedDmoDispatch before its Manager lock and invokes these callbacks
before removal, so streamed held/interacting objects release live ownership.

Actor after-physics caller now invokes the peer's actual shared Chasee update,
followed by original Actor sound timer decay, instead of the prior empty-only
Chasee boundary. No additional Actor.Update is scheduled.

Changed Rust files are syntax parsed with rustfmt `--emit stdout`; this does
not establish type correctness or runtime behavior. No compilation/tests/run.

Pending hand props now run EvaluateHandProps82e9d150 in Update1 after
Census.Update and before EvaluateCensus, source82728250. SpawnAndSetHandProp
uses existing DmoFrame factory/ModelMan/Vault and actual SimLOD bone3 plus the
shared ReorientHandTransformToHandPropBase. SetHandProp82e7b418 replaces/culls
old ownership, applies groups12/15, gravity/type and monitor registration.
Authored usage key864a7084ca220523/type d6778895b505d1db/stride56 is confirmed
in original skater.vlt; low32 collection matching and existing action RNG remain.
Selected freeroam has no main challenge; native hand-prop gate is true there,
and only disables challenge types35/36 when a main challenge is present.

Held DMO collision targets now feed the SAME physical Part drive82e506d8.
FillCollisionTargetTransforms82d36610 emits one target only for HandProp;
free-physics objects emit no targets. Vehicle collision prefix was split from
line/skitch suffix to preserve Pedestrian->Vehicle->DMO before head adjustment.
UpdateTargetTransform82d2e970..e9c8 also decrements released/no-locator hand-prop
timers; it leaves held objects alone. No new physics integrator or teleports.

Startup attribute audit reads original skater.vlt through the existing read-only
vault_dump helper. Added missing dmo_models da7a8f0ef63be539 plus all three
required initialization RefSpec target classes: Navigation b8b60ddf94b96b4a,
PostAdjust f74bf69f6a106edf, HeadTracking2e3bf6df89614045. Added patrol/guard
b5de32c970605efa and contained Prop characteristics5cac37845e0c4722. All actual
entity RefSpec target classes consumed by pedestrian construction/perception,
collision and navigation are now selected alongside root's chase/locomotion/
takedown and the animation peer's animation class. Character collision models
read capsules from already-loaded livingworld_models and shared global tuning.

The AI Plugin leaf (Update826fdcc8/End826eb098), not an extra outer frame pass,
owns plugin controller execution. FrameServices now can borrow the same optional
Simulation RNG through with_entity_sim/with_entity_and_random_sim; Update2
reborrows it after transfer into each preframe AI target. Ordinary frame callers
retain None, so a requested Simulation draw cannot silently use another stream.
Both cull clean-end paths use full FrameServices, and virtual RemoveBehaviourPlugin
uses actual remove_plugin, including source notification and waypoint clear.

Graph admission follow-up (source inspected, syntax parsed only; no builds):
WatchWantTargetWithoutInterruptingLocomotion826ff678/8270b768/826ff738 and
StopAndFaceWantTarget826ff7f0/8270ba10/826ff8b0 now retain native float/bool
instance state. GetSeekSpeed82e6bbb0 snapshots ramped speed when enabled;
actual target COM/velocity, signed-angle helper, head stack and navigation
explicit direction are reused. These were the final two missing nonnull
CreateInstance implementations in the extracted AI graph.

Frame want callbacks now cover AlertToWantTarget826fde30/826eb7d0,
ApproachWantTarget826eb920, RefreshTargetExplicitWantTarget826ef4f0,
ApproachZonePointOfInterest8270e840, DropHandProp826ef2a8,
IgnoreCollisionsWithWantTarget826ef3c0, ThrowHandPropAtWantTarget826eead0/
826eeb50, ChannelGreetWantTarget82707d48/82707ef0 and RunFromHonker826fe580/
826fe5e0. ThrowHandPropAtTargetable82e9b960 shares native interception and
Manager GetRandomPos then the existing hand-prop release/animation owner.
Its no-intersection fallback is the predicted THROWER position, preserved.
Honker uses three unsafe normalization passes, source cross/plane test,
Navigation.mPathTarget+112 and distance-squared threshold4 (speeds6/3).
DrawDebugUpdateMarker826ff360 has only debug-overlay/position reads; quiet
gameplay deliberately produces no optional diagnostic drawing.

Greet uses original model AudioCharacter cached-layout+4 via GetAudioCharacter
82e78490. Direct mood delivery temporarily swaps the borrowed active/peer
entity references and restores them; actual mood/perception/LOS processor is
shared. StaticGroundPool is threaded into actual AI frames alongside RNG.
Cvar array collision found by static integration review is repaired: face/hand
1213/14, watch angle/predict1215/16, attack perturb/Y1217/18, greet1219.
Native defaults come from82f6ed60/82f6edb0,82f5f240/82f5f2a0,82fab170/
82fab1d0 and82f5f360. Definition/GUI arrays now1220 entries.

DetermineTakeDownType82e7a6c0 / ChaseeTargetAvailable82e79dd0 now share actual
FrameServices targetable position/velocity and retained model transform.
Original take-down class3032dc4533789895 attribute76ac331fbd5803ff has
TYPE8ca6a472b87760dc/stride40, verified in skater.vlt. Both attack-name
pointers resolve through existing typed array relocation/text service; POD
fields read only+8..39. Raw PPC confirms lag98da4760b3c62ed7 and width
8b45ca8123ad04d4 (decompiler truncates keys). Ordered authored selection
clears only selected descriptor before scan; first eligible approach/radial/
angle record writes target+3104, descriptor index representing pointer+3196,
flags+3240bit7 mirror. Both min/max speed-length evaluations and fused lag
terms remain, lower bound strict/upper inclusive, both angular bounds strict.
Target prediction Y rejection uses kTakedownYTolerance82fa5f78 (peer owns
cvar registration), then replaces predicted Y with pedestrian Y.

Final raw throw review caught the decompiler dropping branch82e9bb80: Manager
GetRandomPos draws occur only for abs(perturb radius)>EPSILON_192 at823953d0
bits34000000. That branch is now preserved, including no RNG consumption
when radius is zero. Syntax parsing only; no compilation or runtime verification.

ForceWipeOut downstream audit: Actor.LivingWorldWipeOut825ed130 writes
normalized Actor position minus source and Actor+258 bit40000000.
SetUpPhysics825ff804/814 copies bit and vector into the distinct AnimOutPhysIn
owner. Existing Actor.Update825f1138 clears source flag through97ffffff AFTER
that copy; selected_player_tick actually invokes this after phase2. The vector
remains retained, with activation false next frame. ProcessAnimationInput copies
activation to Processed8c4bit8 and impulse+5b0; ProcessInput controls maps bit8
to wiping-out40000, and animation attribute suffix also forces wiping-out.

The concrete WipeoutGround.Enter82d8ecb4..ed50 branch previously stopped with
PendingLivingWorldImpulse. It now reads Processed+5b0, fetches live original
physics_wipeout/default KEY58fc35c49844acc6 through existing global reader,
scales the vector and invokes Skeleton.PushRagdoll82b59868. PDB PART_SPINE2=12
receives shared SkeletonBody.ApplyDisplacementToPart82b545e0, using the sole
actual body inverse mass/old-force accumulator and cooldown reset. Runs after
velocity clamp and before ragdoll collision publication. No separate impulse
integrator or sticky input flag introduced. Source-only validation and syntax
parse; no compilation/test/runtime.

Root takedown Begin82717660/Update8270a038/Success82717918 reviewed against
IDA and retained packet APIs. Success raw82717a50..58 writes State+0xC5C
(3164) byte|=20: PDB EntityCollisionData at3152+12 mbCanClear. Existing
state.collision_flags is the correct owner, not packed flags[0..4].

Static Rust integration review (no compiler/test invocation): repaired missing
ReckoningVector trait imports in actor_chase_update.rs,
selected_living_world_pedestrian_moodables.rs and living_world_chase_frame.rs.
All three call SkeletonRootVector.operand, which has no inherent method.
Reviewed FrameServices construction and sole call graph for ground/Simulation
RNG reborrows, mutable peer slices, direct mood swaps, transfer/census callback
borrows, PropRecipeFrame moved cull methods and all four struct literals,
stream waypoint world/living lock acquisition, and wipeout operation enum
match/typed error path. No further definite type/API issue found in these
paths by inspection. Source method-name inventory found no missing try_*
method declarations in pedestrian/session LW files (excluding standard library
try_into/try_reserve). Cvar declared length/definition count/registration count/
unique count all1224, maximum1223 at this checkpoint. Static inspection and
syntax parsing do not establish compilation or runtime correctness.

Prop census category completion: CensusPlan now schedules category3 culls and
selected population evaluation invokes EvaluatePropCensus before Census.Update
and replacement spawning. New selected_living_world_prop_census.rs consumes
the existing livingworld_census_ranges `props` collection (ctor827209b8..9c0,
retained key+0x78). Uses the shared circle interpolation, original speed*3.6,
NormalizeSafe and shifted centre. EvaluativeCullProps8271f410 snapshots the
existing Prop entity order, gates allow_culling, uses fullXYZ squared distance,
and includes the radius boundary. Raw82720fb0 sets r6=1 (outside); raw8271f57c
bge preserves the ordered >= comparison, including its NaN behavior. Culls use
the existing recursive PropRecipeFrame owner, with real plugin cleanup,
notifications/detach and deferred Census.Update destruction. There is no Prop
spawn phase in this native function. Source/raw review and syntax parsing only;
no compilation, tests or runtime. New Census.Update helper separately reviewed:
living-world lock drops before the DMO frame; no definite borrow issue found.

DMO execution integration: raw/source Manager.UpdateLOSStates826f5410 confirms
pedestrian/vehicle/Prop/DMO order. Update282715a70 advances DMO groups; original
UpdateEntitiesPreFrame8270353c runs DMO after vehicles and before Props. Added
DMO specialization of shared ExecutionMan, adapting the existing HashMap via
ExecutionRoster instead of allocating a second roster. EntityMan and boxed
EntityMan adapters preserve existing callers. Transform remains in the existing
DynamicObject owner; only LOSim/explicit-update/player-distance/cache fields are
added. Shared CommonEntityCore.test_losim_at avoids copying its radius kernel.
Original cvars Hi/Lo/No DynamicObjectsFrameCount initialize groups30/10/3.
Spawn attaches group0; cull detaches its actual current group. Same retained
CensusTargetCache resolves the DMO model on its first LOSim evaluation.

New selected_living_world_dmo_prestate.rs follows82d2ea08: base DMO plugin getter
is folded-null, static/empty-child/null-locator/null-characteristics gates,
position tolerance2114f9329e98f00c, rotation tolerancee43445036aaee7c5. Native
EPSILON is37800000 at82327050 and degrees multiplier3c8efa35. Relative matrix
uses existing Matrix33 orthonormalize/rotation-vector kernels. It invokes the
shared PropRecipeFrame activate/release callbacks, preserving native child order.
DMO AI getter is folded-null; UpdateState82e4c1d0 therefore has no controller
mutation. PostState82e62858 is debug history only. The reset suffix is gated by
Entity.GetSafetyIndex()==0, confirmed complete-object vtable+84?82d362e8; current
sole concrete producer initializes1. A future zero value explicitly requires
the still missing recursive CanResetDynamicObject/ResetDynamicObject chain.

Selected scheduling now initializes contained Props after DMO census spawn and
before pedestrian replacement spawn, after handprop spawn, and before Update2
LOS evaluation. Per-object initialization and transform/cull callback details
remain in the existing collaborating contained-Prop owners. Static Rust review
and syntax parsing only; no compilation, tests, or runtime. Prop/vehicle shared
LOSim extraction independently reviewed: field borrows and Arc cache coercions
are consistent, and existing native phase ordering is retained.

Handprop contained-child reachability audit: see handprop-hotpoint-audit-2026-09-30.json.
Read-only Python inspection of original Jan17 worldmisc.big named entry
DMO/cPres_Global.xsf, using the repository RefPack and Vault research helpers
and the exact stream/RW4/EmbeddedData layouts from skate2-formats. Decoded all
125 arenas and93 DMO instance records. Every one of the22 collections in
hand-prop class ec8b756546e226f6 was resolved by layout+40 Model.collection,
then dmo_models layout+0 asset/local GUID, matching the live spawn adapter.
The22 collections reference18 unique models. Every model has exactly one
hotpoint, type0 (hand grip); none has any nonzero contained-waypoint hotpoint.
Across the entire bank all19 grip-bearing models have only type0;22 different
models do contain nonzero hotpoints (types1,2,6). Thus the decoder positively
observed the waypoint cases; absence is specific to every admitted handprop.
The sole live set_hand_prop call is selected_living_world_hand_prop_spawn.rs.
Therefore the narrow held-prop with_monitors path's empty-contained-list guard
cannot reject any original admitted handprop. This is corpus evidence, not a
claim that arbitrary future/replacement model data cannot include both kinds.
No game executable, build, check, test, or runtime was invoked.


### Dynamic-object recursive reset completion (source only)

`CanResetDynamicObject82d3a908` and `ResetDynamicObject82d39e10` now share
`BuildRecursiveObjectResetList82d37c70`: preserve depth-first physical-pool
order, retain each live entity once, test the model HiLOD collision root at its
locator's **initial stream transform** (+64), and recursively include overlaps.
The native fixed capacity50 remains an explicit checked boundary. Existing
physical-pool overlap traversal and shared narrow-phase primitives are reused.
`BuildObjectResetListVolumeTestInfo82d39c60/82d38ba8` uses the locator's model HiLOD collision root
and initial transform (not the current colliding body's transform). The
retained local-skater capsule gates eligibility; Reset rebuilds the list and
its retained board capsule gates the original offboard hide-board callback.

`ResetDynamicObjectsFromResetList82d2b6e0` skips IDs no longer live, sends
locator-less objects through full existing cull, and resets locator-backed
objects through that same child/monitor/cull owner, then updates current
locator transform, existing persistence quaternion-position, and passive KD
coordinates (`ResetLocatorTransform82d3c740`). Bodies remain owned by the
existing pending-cull/Census.Update phase; this operation does not respawn.

Preframe now calls the real distance/RAIN/WorldPainter safety producer before
state updates. Contained-prop tolerance also uses the original stream frame,
confirmed `82d2ead8` loading locator+64. Native Task_UpdateStateGraphs82d2b890
retains pointers until destruction, so recursive reset can affect later
retained targets. A specifically pending-cull target has already had its
locator cleared, which disables **both** meaningful prestate branches; its
AI graph getter is null and poststate is debug-only. The host therefore skips
only IDs proven present in PendingCull; an unexplained missing target remains
an error. This preserves the native gameplay remainder without duplicating
objects or discarding their destruction owner.

Rust syntax parsing only; no build, type-check, tests, or runtime execution.


### Initial reset-volume publication and original loading schedule

Source audit: `Simulation.SimUpdate1`8289f410 calls Manager.Update2/prestate
at8289f63c; `SimUpdate2`8289f980 is the sole direct caller of
Manager.UpdatePostTransforms (8289fc50). Both require simulation bit1 and
`mEnableLivingWorldUpdate`. `CharacterPrepareState`82760058 enters initial
census at827601e0 after GameLoading readiness; PlayerLoader.IsLoaded828141c8
only tests characterID!=invalid, not a completed physics frame. Native
GameModule.Update8273edc8 may already perform simulation while loading, but
no deterministic one-call loading publication boundary was established.
Execution buckets can select objects on the first frame, so they cannot
justify ignoring the missing retained reset volumes.

The synchronous selected startup now explicitly calls **the same actual
UpdateTransforms reset-capsule producer** once after its completed initial
PhysOut and before initial census. This is a host startup boundary adaptation,
not a claim that native CharacterPrepareState contains this direct call.
It derives both capsule shapes/transforms/AABBs from actual retained initial
PhysOut and the original cvars. No guessed AABBs, suppressed safety test,
extra simulation frame, or RNG draw is introduced. The normal post-transform
producer remains unchanged and refreshes that same retained cache each tick.
