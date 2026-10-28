# HiLOD constructor remaining closure

Fresh source audit, 2026-09-12. Confidence high for the call order and inspected
constructor bodies; this is a work map, not a complete implementation claim.

## Actor allocation, registration and publication

Actor825ff2d8 allocates 0x340 bytes in Sim, flags0. Call825ff2f4 invokes
PhysicalPlayerHiLOD82e03c50. Only after return does825ff300 store Actor+200.
HiLOD82e03c84 forms Entity+4 (its UCOM Object), with a separate null-owner branch,
and calls IPhysical82dfe5d0 at82e03c94 before any child construction.
IPhysical stores its Object owner+4 at82dfe5f8, checks owner nonnull, checks
Find(key825ebe48) absent at82dfe628, and registers the SAME interface object
at82dfe658. The source key is the IPhysical._IName function address, not the
string it returns. IPhysical is8 bytes, at HiLOD offset0; HiLOD mOwner is+8,
PhysOut+2ac. Actor's cached physical member is+200.

The new Actor allocation mount retains the same factory owner and registry
occurrence before any selected body construction. It deliberately does not
publish the cached member. Selected assembly Complete cannot justify that
publication because the constructor stages below remain incomplete.

TimeSinceLastInput82c98b78 and PhysFilteredState82c99618 require the cached
Actor member to be nonnull and identical to UCOM Find before calling GetPhysOut
at virtual+28. They read Animation+80 and Filtered+0 respectively. Those
conditions cannot consume a different pair or infer Ground from fresh output.

## Outer source order

These are actual BL addresses inside HiLOD82e03c50, not decompiler return
assignment comments. Several are selected projections already implemented;
their presence here does not imply their complete native parent is recovered.

| Call site | Constructor or action |
|---|---|
|82e03ef0|ProcessedPhysIn|
|82e03f20|PhysOut|
|82e03f58|PhysOutConditioners|
|82e03f90|SurfacePhysics|
|82e03fc4|GrindAirAdjust|
|82e03ff8|Reckoning (publication82e04008)|
|82e04030|TrajectorySelector|
|82e04068|OffboardTrajectorySelector|
|82e04098|HandPlantManager|
|82e040c8|Pumping.Reset|
|82e040f8|AnimOut|
|82e0413c|Skateboard|
|82e041c8|Skeleton, after PhysicsDebugSystem allocation and board remain-on-reset|
|82e04204|PhysicalPlayerStateChanger|
|82e04228|Skeleton.Init|
|82e0427c|Wipeout.Reset after inline owner bindings|
|82e042bc|GrindManager|
|82e042f4|FootPlantManager|
|82e0432c|PlayerGrabSpline|
|82e0437c|SkateboardController after actual GetPhysicsIsland|
|82e043b4|OffBoardFeetIK|
|82e043f0|LandingOnDeckManager|
|82e04424|OffboardLocationTest|
|82e04458|OffboardJumpingTest|

SkeletonController then initializes inline and the actual PhysicsSystems
pointer bundle is assembled before state allocation. All26 state objects are
constructed in this order: Sleeping, SlideGround, RevertGround, GroundAnimation,
PhysicsAir, KnownAir, Grind_Boardslide, Grind_5050, Grind_Tipslide, Grind_5_O,
Grind_Backslash, WipeoutGround, Nonspecific, Teleporting, FollowPath, Skitching,
HandPlant, FootPlant, Boneless, OffBoard, OffBoard_Locomotion, OffBoard_Pushing,
LandingOnDeck, PhysicsGround, OffBoard_Air, OffBoard_Jump. The final two use the
same allocation debug name but are distinct classes and sizes.

State call anchors: Sleeping82e0456c, Slide82e045bc, Revert82e04610,
GroundAnimation82e0464c, PhysicsAir82e04688, KnownAir82e046c4,
five Grind_Common calls82e04704/754/7a4/7f4/844, WipeoutGround82e04890,
Nonspecific82e048d0, Teleport82e04928, FollowPath82e04968, Skitch82e049b4,
HandPlant82e049f0, FootPlant82e04a2c, Boneless82e04a68, OffBoard82e04aa4,
Locomotion82e04ae0, Pushing82e04b1c, LandingOnDeck82e04b58,
PhysicsGround82e04b98, OffBoard_Air82e04bd4, OffBoard_Jump82e04c10.

Tail: allocate a real collision capsule with radius/height1 via82e04c60;
reset three trajectory-batch pointers and corresponding line records/start
positions; select the actual Sleeping object then SetPhysicsState(Ground)
at82e04d10; set Wipeout invincibility to0.039999999; zero the source vector;
query network-game status and perform source collision-mode changes;
IPhysicalWorldServices.Add(this); JPhy_State=Invalid; return.
Offline normally invokes board CollideWithWorldOnly82e04d70 and controller
StartCollideWithWorldOnly82e04d78 if the retained flag was clear. The alternate
branch calls CollideNormally82e04d9c/StopCollideWithWorldOnly82e04da4 only if
the retained flag requires it. These are constructor effects, not optional
presentation or LivingWorld completeness.

## Conditioner constructor dependency closure

PhysOutConditioners82e316f0 owns11 pointer slots, size44. Child allocations
use Sim flags1, whose category OR100 proves zero initial storage. NULL is
retained and allocation continues, but later dereferences may fail. Sequence:

| Child | Size | Constructor |
|---|---:|---|
|Intents|8|inline owner only|
|Grinds|0x1200|82e2d2c8|
|MissionSystem|8|inline owner only|
|Audio|0x300|827c6188|
|Camera|8|inline owner only|
|Physics|0x110|82e2f8f0|
|Animation|0x110|82e2fdc0|
|Scoring|0x14|82e2b3a8|
|Stats|8|inline owner only|
|PathRecording|0x40|827c57b0|
|GameCam|0x80|82e2cf80|

Grinds constructs one approach reckoning and30 history entries (112-byte
payload,128-byte timed entry), resets three chromosomes, then freshly samples
kMinApproachNonGrindFrames and stores its value+1. Other counters/category/name
prefixes initialize explicitly. Physics.Reset82e2d6d0 sets two filtered states
Invalid and mMustChangeFilteredState=true. Audio constructor first checks the
actual output latch and reads State.mIsWipingOut, then Reset827c4b28; Reset
preserves that sampled boolean. Its four wheel contacts initialize true.

Animation constructs shared AnimOutTrickData, then three PIDFilter<float>
objects from independently sampled physics_animation_default layouts at
controller indices33,32,31 (16-byte PhysicsControllerData). PID ctor82e48e08
copies the four control floats, initializes value/lastTarget0 and three history
floats0. Two actual one-byte PhysicsDebug allocations follow. Reset82e2ef78
sets nose axis I, body tilt zero, clears eight scalar histories, resets the
three PID values and previous trick data. It does not discard controller data.
GameCam uses PopSmoother82e2c538 with four zero vectors and smoothness0.95;
PathRecording starts actual path/node/trajectory pointers null and counters0.

### Scoring is not an empty conditioner

Scoring82e2b3a8 allocates ScoreModule82dfb508 and calls Reset82df26f0 even
after a NULL allocation (a real source fault boundary). ScoreModule itself
constructs eleven children then calls Reset once, so the enclosing conditioner
causes two resets. Children: GlobalMetrics82dee818, ScoreHolder82df35c8,
AirCollector82df8e18, GrindCollector82df9490, GroundCollector82df9a78,
HandplantCollector82dfabf8, OffboardCollector82dfafd0, OtherCollector through
Nursary82df86f8, LineTimer and MomentumMeter through ScoringMeter82dedce8,
and WipeoutScorer82df5618. ScoreHolder allocates an additional Sequence;
WipeoutScorer allocates three distinct entity-scoring owners.

PhysicsScoring category row45@82ff0dc0 has maskFFFFFFFF/OR0, unlike Sim
row55@82ff0eb0 maskFFFFFFFF/OR100. Several collectors and ScoreModule are
PhysicsScoring allocations and must preserve constructor-unwritten unknowns,
including packed bits left by low29-bit masks. ScoreHolder/AirCollector and
WipeoutScorer use Sim. Do not assume every nested scoring allocation is zero.

ScoreModule.Reset resets ScoreHolder, GlobalMetrics history counts, each of
six collectors and both meters; selects and ENTERS OtherCollector; writes
its scalar histories and then checks the same output latch. Its last store
82df2878 sets SAME PhysOut.Scoring2.mHasBeenReset=true. Thus complete conditioner
construction changes output from the standalone PhysOut constructor's false.
No unconditional-success scoring placeholder or fabricated collector is valid.

## Generated Action continuation retained for later work

Fresh Update82955c78 clears five condition-active words, stores dt, computes
lists, exits, transition, enters, and calls the generated current-state update.
Decompiler signatures are wrong: raw passes dt in f1, exit list at r5/sp+50,
update list at r6/sp+90 and enter list at r7/sp+70 to UpdateLists82952068.
No third user argument exists. The later calls consume sp+50 for exit and
sp+70 for enter. Do not port decompiler-shifted argument names.

Initial Evaluate starts at0 and walks active children in descriptor order with
short-circuit expressions. OffBoard2 is initially inactive. Fresh PhysOut's
Filtered.Invalid does not satisfy OnGround19 (expression56 first compares
condition74 to Ground), so the historical candidate Ground leaf is not a
default to select directly. Root0 has11 behaviours; Default3 adds PrintText2D.
Even a default initial tick needs actual earlier predicate reads and concrete
JuiceHook/Action-to-Motion behaviour instances. Startup Action execution remains
blocked on real cached physical publication and the complete constructor path.

## Wipeout velocity cvar admission (2026-09-12)

Fresh initializers82f8fad0/82f8fb30/82f8fb90 and table82fd5790/5794/5798 establish Physics_Wipeout references kUpVelocityScaleLower (default0, range0..1, title `Lower Body Scale vel Up ` including trailing space), kUpVelocityScaleUpper (default bits3dcccccd, range0..1, title `UpperBody Scale vel Up`), kMaxSkeletonPartVelocity (default20, range1..100, title `Max Skeleton Part Velocity`). Raw calls82f8fb0c/70/d0 use callback r10=0; decompiler callback arguments for upper/max are wrong ABI reconstruction. Admitted these as indices32..34 in the same live registry; initializer order after82f8efd8 ShowVelRemoval and before82f98540 HandDrives. Runtime float deserialization retains source no-clamp semantics. Registered test covers unlinked targets, exact defaults, out-of-GUI-range INI values and relinking defaults.

## Concrete scoring constructor progress (unregistered modules)

GlobalMetrics82dee818, Sequence82df2d68/Reset82dedbe0 and ScoreHolder82df35c8/Reset82dee248 now have typed constructor modules. Sequence and holder are Sim-cleared allocations, so their embedded Scorable residual fields start0. PhysicsScoring collectors use a separate Scorable representation preserving switch-variant and all8 connector cells as unknown: constructor82de97f0 never writes those fields. OtherCollector table8236c3ac routes both Reset and Enter to82deb5e8, clearing only analysis bits and IDs. Graph addresses in timed arc trackers retain the actual collection/cached-layout instance and index, without reading graph sample bytes during constructor. Ground/Offboard total trick score and tracker matrices/low29 flag bits remain unknown in non-cleared allocations.

Fresh class-key raw: states824b64d4..64ec =>546c36b656038e04; general828b14b4..14cc =>349215e2e817703c; ScoreHolder default82df35dc..35f8 =>d7edbd362d7d2152. Grind GUID raw82df94d0 li r10,-1 and std82df9544/9554/9558 prove all64 bits set, despite decompiler overlapping pointer high halves. PatternItem82de9660 raw writes0 to matchType and union value; Invalid for this enum is0.

## Complete conditioner construction owner implementation (pending independent integration)

New `physical_player_phys_out_conditioners.rs` and twelve typed child/test modules implement the entire eleven-child constructor, including all nested scoring allocations. The interface binds the existing PhysOut occurrence, retains allocation values separately from published native pointers, prohibits replay after an attempted construction, and allows real NULL allocation outcomes to continue to the original dependent fault. ScoreHolder's missing Sequence fails inside Reset; missing ScoreModule fails in the Scoring constructor's second Reset. Both original ScoreModule resets run against the same output; the final guarded store is Scoring2.has_been_reset=true.

AirCollector.Reset82df4720 samples the original write latch before skateboard position, Skeleton matrix and skateboard matrix reads, with prior stores retained if a later dependency fails. AirGroundData.Set82df44d8 uses retained Ground+192/startpos and optional valid ground point, surface masks7f==10 andf80==400; no external query is fabricated. Rotation enum raw82df56d0/d8/e0 is Pitch2/Yaw0/Roll1. ScoreModule reset raw82df2804 stores collectorType4 and82df280c stores gesture35. OtherCollector Enter uses the same owned Other collector reset target82deb5e8.

Mandatory nested DmoScorer constructor82df55a0 reads the original generic attribute keyf54cf3579c22165b; NULL uses DefaultDataArea4, malformed nonnull data fails explicitly with partial child retained/unpublished. This constructor requirement does not expand parked DMO gameplay. Wipeout-scoring class raw82deda2c..44 is9e4c471bc6eab615. Its seven character damage records retain fulli32 enum fields (distinct from PhysOut's compacti8 damage output);25 bone records and three entity-score allocations are real typed owners. DebugGraphXY exact PDB nameSk8::util::DebugGraphXY has size1 and no payload members.

Five new focused tests cover29 source allocation sites and same-output scoring writes; unlinked Grinds constructor failure before pointer publication; nested NULL Sequence failure at holder reset; Sequence reset preservation of patterns and inactive trick payloads; and locked Audio allocation retained before Reset. Tests/formatting are not claimed run by the source agent; root owns registration, independent review, original-data probes and Cargo gates.

## Grind balance cvar admission (registry38)

Initializers82f99dd0/82f99e28, table82fd5ee4/5ee8, establish Physics_Grinds_Balance kLeanMinNonGrindFrames (Int20, range0..100, title `Lean Min Non Grind Frames`) and kCurbExitStartTime (Float2, range0..4, title `Curb Exit Start Time`), callbacksNULL rawr10=0. Fresh GrindManager raw corrects a proposed combined-capture interpretation:82dd384c and3864 are ADDI reference-address formation, not target loads. Lean target3874/value3878 precedes wrapping+1/counterstore3880; curb target3884/value3888 follows that store. Separate fresh getters preserve intervening relinking and prefix state. The focused test distinguishes this ordering and validates unchanged source no-clamp startup semantics.

Motion cvar admitted earlier at index35: initializer82f867d0/table82fd4f38/reference830a3910, Animation_MovingObjects kMOAngleMaxDelta, title `Angle Max Delta per Tick`, Float2 range0..90, callbackNULL raw82f8680c. Motion constructor must sample this actual current value even when XML overrides the default. Tests cover unlinked/default/INI120/relink2.


### Gesture cvar enum owner extension (2026-09-12)

Fresh IDA initializer review: EnableDebugGestures82f85df0/table82fd4ec8/ref830a385c is Boolean false. Animation_Gestures enum references Up82f88d20/ref830a393c/table82fd5168, Down82f88d70/ref830a3f38/table82fd516c, Left82f88dc0/ref830a38c0/table82fd5170, Right82f88e10/ref830a3604/table82fd5174 have defaults34/33/32/31; all callbacks NULL. Labels are kAnimUpGesture/kAnimDownGesture/kAnimLeftGesture/kAnimRightGesture. Titles Up/Down/Left/Right Gesture. All share the actual 35 sequential tags Sky through Axe (including Fonz5, Push31, RaiseTheRoof32, Wings33, Axe34); trailing comma creates no empty tag.

cVarEnumRef constructor82a8afa0/Link82a8aa38 and CreateEnumRef82a892b8 retain distinct enum kind, current/default and parsed tag metadata; ParseEnumTags82a87dc0 assigns sequential values for this exact source string. PDB tVarEnum current+d0/default+cc differs from ordinary integer current+d8. ReadLabel82a8a5a8 enum case calls cIniFile::GetInt and writes raw current integer, without name conversion or tag bounds clamp. The native model now preserves an Enumeration variant and shared immutable parsed tag metadata, while startup INI writes raw integers. No generic enum-tag grammar was added for unselected definitions.

Registry grows38 to43. New stable indices38 bool and39..42 directions. Source initialization order places38 before35, and39..42 between35 and30. Existing live session references and manager relinking semantics are retained. Independent getters sample their own actual target; no cross-direction snapshot. Focused test exercises null targets, defaults, preserved enum metadata/default, numeric values999/-7, nonnumeric Fonz converting through existing integer parser to0, existing-manager relink retention and fresh-manager relink defaults. Root owns compilation, strict checks and independent source approval.


### Four remaining offboard owners, concrete draft

New physical_player_remaining_constructor_owners.rs owns FeetIK, LandingOnDeck, LocationTest and JumpingTest. Root owns registration/mount; each remains Option until its real constructor stage. FeetIK/Landing retain the SAME Processed Arc and actual skeleton assembly handle, weak physical parent; Landing/Location/Jump retain the SAME world manager authority and weak parent. NULL query batch slots remain absent, with no completed query or miss fabricated.

Caller/callee/PDB: HiLOD82e043b4 -> OffBoardFeetIK82dcd048 (320 bytes);043f0 -> LandingOnDeckManager82dce3a0 (224);04424 -> OffboardLocationTest82dd1f08 (80);04458 -> OffboardJumpingTest82dd4fd8 (112). FeetIK constructor first resets two SurfaceQueryResults then stores Processed+308/Skeleton+312 and invokes Reset82dc8728. Reset explicitly initializes all ten vectors, seven float metrics, state+284=2 FEETIK_STATE_NO_FOOT_FIXED, three integer counters and eight independent booleans. Both ground normals are UP: raw82dc8810 writes 1 into first normal Y and882c writes 1 into reused stack vector before the second normal load891c. Left-foot vector load8820 occurs BEFORE that reused stack Y change. All other vector lanes zero, including W stores through scalar integer aliases. Surface results reset position/normal/surfaceID/valid, with no live query.

Landing raw82dce3c8..dc proves actual class2377CEF0FF59C4D2 and defaultD7EDBD362D7D2152; decompiler miscombines both64-bit arguments. Constructor binds actual Vault Instance before Processed/Skeleton/world stores. Both trajectories have position/velocity/gravity ZERO and time splat-1 (raw424/434 writes the shared stack scalar before respective loads). Reset82dc8c80 preserves the ideal trajectory and actual owner/attribute references; zeros trajectory time and landing metrics/error/velocity, resets active trajectory ZERO/-1, clears five independent flags, and does not manufacture a trajectory batch. Original fixture must select the actual landing class before asserting nonNULL attributes.

Location raw82dd1f28..64 explicitly initializes all three input vectors (ground position/look direction/jump velocity) including W=0; HexRays incorrectly omits stack stores. False jumping/collideWorldOnly/sync flags and NULL line/trajectory batches follow, then actual world pointer. Jump raw82dd4ff8..5034 likewise explicitly zeroes ground position/jump direction/landing position, jumpDist0, result valid/leapOfFaith false, three NULL batch slots, actual world, resultValidCounter0. No named unknown fields remain in these four constructors.

Original assertion helper accepts the actual retained four slots, physical owner, Processed and skeleton handle; verifies pointer identities, real landing attributes, normals/state, initial trajectories and unpublished query state. Root owns source review, compilation and corpus invocation.
