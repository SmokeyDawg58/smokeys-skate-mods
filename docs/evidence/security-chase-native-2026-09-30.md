# Security and chase source recovery, September 30

Active implementation; not compiled or executed.
These owners are prerequisites, not evidence of functioning
live pedestrian spawning, chase, animation or capture.

Inventory before new code found no SecurityGuard, ChaseMan, Chaser or Chasee
runtime. Existing Census, EntityCore, RAIN/NavMesh, character collision/model,
WorldPainter and generic graph owners are reused; no duplicate entity registry,
map loader or navigation world was added. Root owns Pedestrian resources/entity
and graph integration; population agent owns census/RAIN node consumers.

SecurityGuard constructor82e98778 allocates store10 from the shared pedestrian
memory owner, installs WorldPainterPatrolZone, sets Pedestrian flag+6724 bit0x80.
Root retains this allocation. PostInitialize82e70330 sets zone start to actual
spawn position, initializes name if invalid, sets AI storage+3240 bit8 and zeros
guardee closing velocity/previous position. The flag belongs the entity owner.
PreStateGraphUpdate82e9f5a8 first calls the ordinary Pedestrian prefix, sets the
actual nav rig path cost threshold, then stores guardee displacement length/dT
and previous point when guardee exists. Our helper covers this displacement;
nav rig threshold remains the concrete controller's responsibility.

WorldPainterPatrolZone SetZoneByPosition82e57c38 reads layer6 at XZ, strips the
first literal Area, and retains the prefix. Contains82e579c0/82e57a80/82e57b40
compare exact prefix+Area / prefix+Buffer. Invalid names are native assertions.
LayerMan.GetRegionName82ceb1e8 uses the same TOC entry as GetAttribData; the
existing dictionary/palette traversal now exposes original names and is shared
with attribute-key lookup. No geometrical circles or fabricated security zones.
SetIsOutsideCullRadius82e96680 updates only range flag; ForceBreakOff82e96698
sets a persistent independent latch. ChallengeSystemCanCull82e70420 requires
no actual chasee and outside range; census must not ordinary-cull active guards.

ChaseMan ctor82715770/reset82702260/allow-disallow826f3e58/826f3e80 retain native
flags and timer ordering. Three ordered fixed-capacity256 lists use native
TargetableId(kind,id), invalid(-1,-1), Pedestrian kind1. RegisterChaser8271b9e0
emits start for every newly registered chaser, not only the first. Warner
registration8271bce8 excludes existing chasers. Tazer registration8271be10 and
unregister82718c48 preserve order. Last chaser removal827186e8 emits end and
uses authored capture-suppression timer. Side effects are returned explicitly
as ChaseEvent values requiring real challenge/mood/stats/Actor consumers.

JackSkater8270fd08 prevents manual respawn and captures the Actor ID/timer,
clears warners and selects HUD behavior from real mode/challenge facts. Native
SetJackedRespawnPosition826f3c28 uses nearest valid location in the GUARD'S
CHALLENGE OWNER array, only if that array exists and is nonempty; no ambient
jail location is invented. Update8270f678 waits captured Actor state4 and timer
expiry before ordinary breadcrumb respawn or actual Photo/Pro challenge
teleport. Helpers retain state and expose effects; concrete dispatch must exist
before capture can be considered integrated. No missing effect is silently
claimed as handled.

ChaseeReset82de2310 and original global attributes supply formation. Add/remove
82de4bf0/82de4908 preserve order and capacity5. GenerateIdealPositions82de2830
uses max(n,2*n-2) ring candidates, velocity-scaled authored radius clamp, native
XMVectorSinCos through existing polynomial, signed arc / radial coordinates,
stable near-first ordering and greedy candidate assignment with wraparound.
UpdateChaserPositions82de38b0 copies ideal offsets to recommended offsets.
ShouldSwapChaser82de3b98 compares primary cooldown, wipeout/collision/resting/
failed-takedown flags, and native interception time. EvaluateBestChasers sorts
only secondaries, and AutoSwitchPrimaryChaser82de50f8 rotates the replaced
primary to the back with its cooldown effect returned. Formation calculations
reuse existing host numeric approximations and do not claim Xenon bit parity.

Global chase class1477b0d5343d29fd constructor826fc7c8 and all64bit attribute
keys were verified from raw PPC insrdi. globalKey is StringToKey("global")
82f64d78/82f9d030. Shared-vault ChaseSettings loader reads original values.
Read-only Jan17 skater.vlt/bin inspection confirms global7640367dfc82ba38:
min radius3,max5,velocity scalar0.25; auto switch3/improvement1/intercept
improvement0.5; capture suppression layout+4=5.5; max chasers inline byte5.
These are corpus observations, not replacement defaults in implementation.

Outstanding in this ongoing slice: concrete generated-graph hooks, native
PredictIntersectAtMaxSpeed82e54ca8 raw VMX recovery, block/intercept prediction,
nav-controller actions, perception/mood wants, capture effect dispatch, and
ChaseMan's HUD/stats/audio consumers. Native condition-object WantsToChase
ActivationFunc826f0620 only asserts; generated graph inlines its real predicate.
Do not replace it with false or call the assertion stub as a gameplay shortcut.


Typed shared AI storage and graph leaves (continuing integration)

`living_world_pedestrian_state.rs` translates ctor82e92668/reset82e92778,
SetWantActive82e9e310/ActivateRelatedWant82e9e3c8, timers82e89e90/82e9a140,
knowledge82e92898/82e929e0/82e9e620 and mood reset82e9e1a0. It preserves
inactive want targets and explicit bits across reset; deactivation increments
actual knowledge reaction count or removes related mood/knowledge entries.
Ordinary timers deliberately survive reset and a negative crossing frame.
Plugin reset82e9a230 raw82e9a274 compares converted KEY against the retained
TIME when choosing eviction; this unusual original branch is preserved.
Knowledge visibility/target motion comes only through real owner callbacks.
Native uninitialized fields are Options, not invented initial values.

`living_world_guard_graph.rs` adds typed Begin/Update/End leaves for warning,
new/end/group chase, wants, patrol return, timers, mood reset/suppression,
tazer registration and chase knowledge. Its services are required real owner
calls, not no-op defaults. Constructors must provide original occurrence
arguments. Warning59/60 reads ILWMoodable::GetNumReactions(+20).
NewChasee827090e8 calls RemoveChaser on the WANT target before adding to it;
this receiver was preserved after source review. Native IsPatrolling requires
both AI flag+3240 bit8 and a current waypoint, not just guard classification.
Takedown condition status Success2/Fail3 and sink IsWipedOut were verified.
The interception prediction VMX path and actual takedown/animation consumers
remain required integration work; these modules alone do not spawn or chase.
Syntax-only rustfmt parsed these files; no compilation/test/runtime checks.


Prediction source translation (awaiting independent source review)

`living_world_chase_prediction.rs` now carries geotest82e54ca8 and
Pedestrian::InterceptTarget82e7ab78. The implementation follows the X/Z
coordinate-dominant branches and their exact multiply/add order, original
1e-5 threshold, negative-FLT_MAX missing-time sentinels, two nonnegative
solutions, sorted times, and flattened line velocity. Raw VMX128 sourceA
is decoded as ((w>>16)&31)|(((w>>5)&1)<<5)|(((w>>10)&1)<<6), correcting
IDA's misleading extended source registers. Scalar VMADD uses A*C+B.
The source uses strict coordinate dominance (equal diagonal differences
select neither branch) and an unusual positive velocity combination at
82e54eb0/82e55154. Parent was explicitly asked to independently inspect
these before live integration. The outer angular gate preserves possible=true
while choosing the current target point outside its allowed angle interval.
Shared reciprocal/rsqrt/angle helpers preserve their documented host numeric
boundary, not Xenon bit parity. No compilation or numerical tests were run.
