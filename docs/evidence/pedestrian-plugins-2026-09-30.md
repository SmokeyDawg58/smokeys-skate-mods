# Generated pedestrian plugin controllers

Export tools/ida/export_pedestrian_plugins.py reads the ten original generated
delegate tables into pedestrian-native-plugins-2026-09-30.json. The Rust table
emitter reuses existing StateDeclaration/TransitionDeclaration/BehaviourDeclaration
and ConditionDeclaration types. No guest callbacks execute on the host.

PluginController calls PedestrianGraph.update_declarations, the same lifecycle
implementation used by ordinary Pedestrian. The original Pedestrian update
entry and public fields remain intact. Family expression pre/sustain masks are
from native EvaluateExpression bodies; actortracker/spectate/lookat/trash sustain
paths return true (829358f0/8293d208/829394d8/82942b48). Lookat pre expression0
calls LookAtTimeOut; trash pre0 negates HasHandProp. All other switches retain
native lazy predicate order and default true.

Original PDB TemplateHC members prove timer-depth/active-instance capacities:
actortracker3/5, conversation7/8, lookat2/3, sit6/8, spectate3/9, ATM5/6,
newspaper6/6, trash2/3, vending6/6, fountain6/6. EndAll8293df00 removes active
occurrences descending without clearing state identity/timers.

The sole ActorGraphStartupOwners is now shared from selected world startup
through LivingWorld and later player owners using Arc/Mutex. Native InitLWMan
82702e8c passes Simulation.mWorkspace.mStateCache to InitStateGraphCache82e4f328;
there is no separate Prop state cache. Manager ctor82721f0c constructs its
BehaviourMan before Init/preload. Generated flags/cache are not cloned.

Direct plugin leaves currently implemented: LookAt ctor826eaf20 (duration30
unless authored), Begin826fda78 (real source headtracking and timer6),
End826eafa8 (pop even if Begin had null source), LookAtTimeOut826f2598;
DisallowHandPropActions Begin826ee8c8 and End826ee920 set/clear stateCAB bit10;
SetSimpleTimer reuses the existing payload and callbacks. Source headtracking
push82e6b720 stores real source identity and retains the dormant timer.

LookAt now has concrete immutable delegate bindings and an executable controller
using the shared update/end algorithm. Prop construction reads the authored
descriptor through the retained mounts and sole BehaviourGraphCache, then
retains a ProcessedTransferable only after all LookAt bindings are constructed.
Other families fail explicitly before processed admission; dynamic fallback
also remains unsupported. Actual plugin transfer/attachment and live updates
remain to be wired, so this does not establish working plugin-driven NPCs.

Additional source-backed leaves for Spectate and other families now reuse the
existing owners: DisableHeavyCollision826ecdc8/826ecde0, IgnoreStandingCollisions
826ece00/826ece18, DisableCollisionSliding826ece38/826ece50, and spatial position
lock826ece98/826ecf18. The latter End only clears the lock. DisableAllMoods
826ff310/826ff340 and ChangeNavModifierSetting826ff568/826ff608 retain a separate
per-occurrence bool instance (folded CreateInstance827126f8 initializes false).
Modifier ctor826ee5f8 accepts exactly ChaseGlue/Pedestrian/SkaterAvoidance/
VehicleAvoidance/BigBlackAvoidance, and existing NavigationModifiers owns effects.

LockAndTargetWayPoint.Begin82708418 selects GetFirstUnlockedWaypoint82e64b00
(raw vtable dispatch offset58, Entity vtable82395098). Update82708548 targets
an existing reservation through actual TargetExplicit mode1, or acquires one
without targeting it until the next update; both branches set seek speed2.
Only successful Begin acquisition enables the navigation rig. UnlockWaypoint
826ebf98 releases the same sole PropMan reservation and clears the rig bit.
These additional callbacks still require family backend wiring before admission.
No compilation/tests/runtime; syntax parsing only.

## Later executable/lifecycle checkpoint

Trash-bin is a second processed family. Its three null-instance leaves reuse
DisallowHandPropActions, ExitPlugin, and actual ThrowHandPropAtTrashBin8270b520.
The latter reads waypoint0 and calls ThrowHandPropAtPosition82e9b5a0. Directed
throws retain full4 target lanes, update release flags/timer34/velocity, use the
existing signed-angle kernel, and select the original light/attack channel.
Channel remapping/transition is shared with the ordinary AI frame callback.

TransferableBehaviour.Copy82cf3b38 constructs a fresh controller while sharing
delegate payloads and descriptor ownership. Prop.TransferBehaviourToSink82e68230
retains the copy identity, assigns the actual entity plugin, then registers its
sink. Entity plugin is now an actual AttachedPlugin, not parsed metadata.
CleanlyEndAllPluginBehaviours82e4c578 ends active callbacks without nulling the
pointer. Null assignment signals source copy deletion82e66d58 (first identity),
then resets removal-request state. Source sink removal remains the separate
ExitPlugin operation, preserving the two native containers.

Transfer eligibility is NOT connected. Source Manager82710cc0 evaluates every
12th invocation (counter0..11 then0). Manager82710a78 orders empty-plugin,
CanUseBehaviourSource, strict Y tolerance, strict XZ radius, source expression,
then authored entity preference/Bernoulli before transfer. CanUse82e86140 reads
the existing plugin cooldown keyed by controller root hash. Decide82e9ac50 scans
32-byte preference records and records retry cooldown only after a failed
matching Bernoulli. No unconditional transfer or fake successful predicate was
added. The source expression evaluator82cefd38 now preserves lazy None/And/Or/
Not evaluation and empty identities; concrete condition dispatch remains work.
Raw82e653cc..d8 compares the ADDRESS of ICEController::GetEditTake with zero,
not its returned state; this decompiler artifact must not become a false gate.

Perception CanDetect82e73df0 now follows near override (XZ squared<1,Y squared<4),
hearing, PointInCone82e4e338, then shared BF7 World line test82e4e578. Exact
parameter hashes and original numeric kernels are retained. Both Island virtual
line entries at820ffa10+68/+72 point to827b2538, forwarding World.LineCollisionTest.
