# Pedestrian post-adjust producer continuation

Source-only continuation; no compile, test or runtime invocation. Existing dirty
work was preserved. This does not establish playable pedestrians.

## Native owners and order

Re-read IDA `cLivingWorldPostAdjust.Update82e7cb70`,
`LWPedestrianMan.UpdateAnimPostAdjust82e83708`, and
`Manager.UpdatePostTransforms827031e8`. The last orders DMO targets/culling,
pedestrian/vehicle/DMO collisions, HeadTracking, PostAdjust, vehicle line tests,
skitch flags, all-category transforms, then wheels. The existing selected DMO
post-transform caller now invokes the new all-roster PostAdjust wrapper directly
after the existing head-tracking pass. Entity roster and adjustment allocation
owners remain unchanged.

Manager skips nonrenderable entities and entities whose retained HeadTracking
owner rejects the current locomotion-turn-complete graph input. Skips preserve
prior quaternion records. Update reads `kEnableHeadTracking`; disabled resets
six compressed records to identity but preserves procedural yaw/pitch history.
Initializer `82fa8c08` establishes category
`Pedestrian_Animation_Debug_HeadTracking`, name `kEnableHeadTracking`, title
`Enable Head Tracking`, default true, null callback. Mood peer added the live
accessor at index1190.

## Arithmetic and retained state

Raw caller `82e8386c..880` establishes R5=target transform, R6=SimLODPoseBuffer,
R7=resolved head target (the decompiler's argument labels are shifted by float
argument handling). The head record bone byte is sign-extended for the actual
pose-buffer index. No fabricated head pose or substitute target is used.

IDA's VMX128 VA register labels are unreliable: raw `179fe87d` at82e7cbdc
uses VA63, not printed95; `17fcfc75` at82e7d1bc uses VA124. Binary three-register
VA decode is `(word>>16)&31 | word&32 | (word>>4)&64`. This also confirms
the signed yaw step is subtracted from the remaining angle, including negative
steps; no absolute-value repair was introduced.

Yaw uses the source .1 horizontal-length gate, global-Y rotation of current
head yaw, actual head Y basis and target orientation. Authored layout+0 gives
normal yaw speed; exceeding total maximum yaw lazily reads reverse speed key
`f23888ec33eaefb9`. Opposing step/history traverses Spine1 toward Head and
skips bones already turning with the step; ordinary traversal is Head toward
Spine1. Every bone uses its retained authored maximum and original strict
unsaturated stop test.

Pitch uses the head's local-Z rotation, actual head pose/target bases, and the
native direct subtraction of head-pose translation from target displacement.
Layout+4 supplies rate, +8 lower-limit multiplier, sparse key
`592536a4699594d9` upper multiplier. Native degree conversion is3c8efa35.
Output composes head-only Z pitch and each bone's X yaw, publishes the six
original compressed records, and recomputes summed head yaw in order.

Reused existing `reckoning_numeric.try_unsigned_angle` (829b0528's lazy squared
length guards and single reciprocal-square-root refinement), shared SinCos,
cross/dot numeric boundary, and the existing axis-angle matrix helper (only its
visibility changed). The existing six-byte codec now has one module owner under
pedestrian_adjust_owners; PRES imports it there. No second codec or pose owner.
Host exceptional VMX min/max behavior retains the repository's existing numeric
limitations; no full bitwise parity claim.

PDB `Sk8::LivingWorld::tAnimData` also confirms SimLODPoseBuffer+5f90,
SimLODBoneGlobal+6490. Taze origin+6580 is global bone3 translation; shared with
the query-flow peer for reuse of the existing right-hand pose producer.

## Remaining rendering scope

The extracted original `livingworld_stamp.a2f2e54a9729` UPDB shader and final
manifest remain in `%TEMP%/skate2-pedestrian-shaders-20260930`. The current GPU
family initially covered `defaultlivingworld` only. Stamp requires skinned tangent/binormal,
normal texture, original nine-vector SH, seven material parameter vectors,
light/view inputs and character/world shadow factors. Its default fragment
retains colourization, full character lighting and fog/tone; crowd is a distinct
pass. The later final-program audit and implementation are recorded in
[pedestrian-rendering-2026-09-30.md](pedestrian-rendering-2026-09-30.md).
Stamp now has its own shader/material inputs, opacity transitions and material
LOD. Full hands/face composition and geometry LOD remain unfinished.

Validation: changed Rust sources passed rustfmt syntax parsing with
`--emit stdout --config skip_children=true`; this performs no writes, compilation,
tests or runtime validation.

## Prestate threat and zone-interest adapters

Root assigned a follow-up source pass for `UpdateThreats82e9aef0` and
`UpdateZoneInterests82e6fa78`. Implemented both on the existing FrameServices
owner; root retains outer PreStateGraph scheduling.

Threats snapshots timer25/26 due flags and own-zone membership at entry. Presence
queries the same pedestrian KD tree with source radius35 and raw capacity20
(`82e9b064`), skips self, and requires the retained CommonCoreDataProxy state
(actual interface+720 slot12 IsInitialized, not renderability). It immediately
calls the existing mood processor with type8, visibility point, perpetrator,
self victim, and both distance factors1. Actor iteration follows; full XYZ
squared distance must be strictly below1225. Type9 delivery follows presence
only when the entry buffer timer was due, a valid patrol zone exists, owner was
outside, live timer18 remains positive, and the actor is in boundary/buffer.
Both authored category update rates are read before either due timer resets,
even if neither was due. Raw key is `b1dd83e0617af652`, so existing MoodMan
`update_rate` is reused. No new mood queue or delayed delivery was added.

Zone interests short-circuits without a valid patrol zone, chooses actual chasee
first, then active want7, then null. It writes the retained entity interest
before position lookup. Only an interest inside boundary/buffer updates the
retained point; absence/outside preserves the old point. PDB confirms separate
LWPatrolZoneOwner mPointOfInterest+20 (complete290) versus start point+10
(complete280). Added only that previously absent field on the existing entity.
Pedestrian vtable823a4ba8 slots24/2c resolve folded setters82e6a480/82a736e8;
the former is a full vector store and the latter a pointer store. There is no
second patrol-zone owner. These adapters require the parent's real local-player
index and shared StaticGroundPool for mood detection.

All three adapter/owner Rust files passed syntax parsing only; the outer caller
was intentionally left to the root agent's prestate integration ownership.

## Chasee per-frame adapter

Follow-up re-read `UpdateByChasee82de5200`, `EvaluateBestChasers82de3fc0`,
`AutoSwitchPrimaryChaser82de50f8`, `ShouldSwapChaser82de3b98`,
`InternalChangePrimaryChaser82de4e80`, `EvaluateFlankOrders82de2730`,
`UpdateChaserPositions82de38b0`, `GenerateIdealPositions82de2830` and
`EmitOngoingEvents82de39f0`. Added FrameServices.update_chasee(entity,dt) in
pedestrian_graph_frame_chase_update.rs, registered under the existing frame.

Empty roster returns before timers; otherwise the same Chasee timers advance,
all chaser interfaces resolve, each existing prediction writes its actual first
intercept time (or FLT_MAX), secondaries rank, optional primary switch writes the
actual old chaser's timer29 before rotating the roster, flank radius updates,
existing formation generation runs, and only then all recommended positions
copy ideal positions. The previous unused rank_and_switch helper was replaced
by staged helpers to preserve timeout-write failure boundaries. The existing
formation helper no longer publishes recommended positions early per assignment.

ChaseMan's three primary-switch attributes are read lazily from its retained
global collection. PDB confirms Actor has Chasee/IProtectee but no Chaser;
Pedestrian has Chaser/Chasee and no IProtectee. This pedestrian adapter uses the
source default-minimum-radius flank branch, not a guessed protector. Guard is
the same Pedestrian owner. Getter82e899c0/82e9aee0 uses timer29; colliding getter
82e948b8 resolves cLivingWorldCollision.Colliding+0a; resting and failed-takedown
read state flags3241 bits02/80. Existing wiped-out flag04 is reused.

Ongoing events use type14 and the actual shared GameSim queue, once per valid
chaser when due and not group-forced-to-end. Raw stores82de3b58/60 prove chaser
is perpetrator and Chasee is victim, at current Chasee position, factors1/1.
Category update rate is sampled even while the event timer is positive. The
timer is reset only after every checked enqueue succeeds. This deliberately
uses no direct mood-detection call, ground argument or local-player argument.

Parent/flow owns PreStateGraph invocation. Four changed Rust sources passed
syntax parsing only. No compilation, tests or runtime run.

## UpdateMoodables and Actor chase continuation

Source82e83e40 now runs through the retained Manager, Actor roster, PhysOut and
GameSim queue. LWPedestrianMan.Init82e9ef90..eff4 binds
livingworld_entities_perceptions class86eb6b564cf12561, collection global, after
execution groups. This is a distinct global instance from entity perception
RefSpecs. All six attributes are read in native order at each pass; no values
were frozen from the corpus. Mood category5 rate is retained and category4
rate is still read/discarded before the actor loop.

Actor fields+588/+592/+596 remain the original constructor-owned sound level,
sound duration and grind throttle. Existing PhysOut audio/score/state owners
supply landing edges, strengths, grinding, finished grind, qualifying finished
air categories3/4/5/6, vehicle skitching and HoM bail/score. The pass emits
7 then6 then5 then4, resets the grind throttle, then emits2, using actual
Actor targetable ID and skeleton-root position and an invalid victim. HoM
threshold is lazily read and compared unsigned as native subfc/subfe.

Native sound conversion uses the existing FakeTorque XMVectorLog/Exp
polynomials, now shared in that existing numeric owner. IDA constants matched
820f0fa0/fb0,821ee8b0/c0 and Ln2 at8227e950. Force calibration preserves
power10(level*.1)/force, then force*calibration rounding. Noise distance factor
is the SQUARE of power10(sound_level*.1)/reference_power, confirmed by raw
82e84ac4..ad8 before82e84b38; the decompiler misorders the scalar division.
Two reciprocal refinements on calibration/log division lack the polynomial
reciprocal's NaN-select. Desktop log2/exp2/reciprocal estimates remain the
existing explicit host policy; Xenon instruction bit accuracy is unverified.

Actor.Update825f1144..118c now owns actual timer decay after its Chasee callback.
A sound timer that lands exactly zero retains its level until the next update;
a strict crossing clears it. Raw BLE/BGE unordered behavior is preserved.

Actor and Pedestrian now share a single Chasee.UpdateByChasee82de5200
orchestrator (living_world_chase_update.rs), with borrowed adapters for each
actual entity. Actor's previous nonempty-Chasee error is replaced on the live
path by intercept prediction, ranking, switch timeout writes, flank radius,
formation and queued ongoing events. The constructor's actual invalid
protector ID follows the default radius branch. A later valid Actor protector
still requires the missing typed BigBlack/protector owner; it is an explicit
failure, not an invented radius. Caller was moved after SetUpPhysics at its
existing Actor.Update schedule position. No extra Actor update per frame.

Cull audit correction: OnCullRequested82e85fd0 receives LWBasicEntity+32.
Its raw+1a14 pointer is complete-object+1a34=mpHandProp, NOT mpAnimData as the
misadjusted decompiler says. It saves the hand prop, calls RemoveHandProp on
the complete entity, then calls the saved IDynamicObject slot+12. No animation
End should be invented at this site. Actual tAnimData::~tAnimData82e93528
releases ChannelAnimatable, MotionGraph, override/entity attribute instances.
Parent/flow owns actual cull/destruction scheduling.

Validation: changed sources syntax-parsed only. No compilation, tests or
runtime execution. Native behavior confidence
high for control flow/field mapping; estimate bit parity remains medium.

Destruction follow-up: pedestrian animation exposes prepare_destruction before
its owner is dropped/free6. ChannelAnimatable82cb7298 drops its owned channel
storage, then Animatable.Clear82d7a6b8 and job deletion run. The existing Clear
implementation was extracted from ResetSkaterAnim so both use one owner.
Pedestrian initialization supplies no blend scheduler; if a later caller
introduces a started job, the existing missing WaitForJob boundary remains
explicit. MotionGraph82ce9f80 -> TemplateHC82932a08 does not invoke behaviour
EndAll. IntentGen82e8e4b0 reuses queue.clear(false,None) before dropping packet
storage. PostAdjust82e74540 and HeadTracking82e74360 only destroy attributes;
SpatialAdjust82a891d0 is trivial. Flow owns exact field/pool release order.
