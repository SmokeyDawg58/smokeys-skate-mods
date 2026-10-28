# Pedestrian motion delegate — source integration, 2026-09-30

## Behaviour construction and execution continuation

### Concrete live adapter follow-up

pedestrian_motion_frame.rs provides the actual AnimUpdate82e95da0 producer API:
sum retained OffsetTime+dt, reuse BuildAnimTree schedule, run loaded delegate with
same-entity LiveServices and previously captured graph intents, restore moved
owners on errors, advance shared animation, then query native cached foot tags.
RightToe/LeftToe/RightHeel/LeftHeel order and packed valid/foot bits are retained;
mirroring swaps corresponding feet. Returned FootContactOutput writes only native
mPedStateOut byte20 bits4/5. TurnBranchWindow query follows, then OffsetTime0 and
ValidBlendPoseBufferfalse, then concrete channel request consumer. tAnimData
ctor82e934a0 directly confirms initial OffsetTime0/ValidBlendPoseBufferfalse.
Root owns FrameMetadata retention and caller phase/actual LOS+cvar inputs.
The intent snapshot is intentionally NOT recaptured from inside AnimUpdate.

The live adapter now calls retained animation.channels.stop directly for both
StopChannel and SetAttemptingTakedown(true), removing that external contract.

Cycle swapping now reads/writes the retained motion_intents field, initialized0
by root from82e867a0; its external callback pair is removed. Root retains shared
ActorGraphIntents and refreshes it through update_animation_data at native phase.

pedestrian_motion_channel.rs now retains cPedAnimData's actual ChannelInfo and
SequenceInfo using EXISTING channel_runtime::ChannelParams (no second channel
engine/parameter representation). Constructor82e8d268 sets statusFinished7,
empty name, transitionTime0, priorityLow0, speed1, blends.2, allboolsfalse forboth.
PDB status enum: NotStarted0, NotStartedSequence1, WaitingToTransition2,
WaitingToSequence3, Playing4, BlendOut5, WaitingForBlendOut6, Finished7.
StopChannels82e6d350 only changes nonFinished channel request toBlendOut with
new length. Transition82e905a8 resets params, stores lengths/cycle and positive
length endpoint flags, then status0 if previouslyFinished else2. Play82e93f20
initializes a Finished request; its nonFinished path genuinely passes blendOut
as BOTH blend lengths with transition.2 (raw940c0/f3=f30,940c8/f2=f30).

The complete request consumer82e96198..6458 follows BuildAnimTree and foot/turn
tags in AnimUpdate. Status0/1 use shared TransitionTo length.2; status2 uses stored
transitionTime; status3 uses SequenceInfo. All use resurrect/create=true and
useAttributes=false, verified raw stack/argument stores. Nonzero Andale status
invokes PlayMissingChannelAnimation82e91ee8: shared TransitionTo PedChannel,
HIP_TEST_01,length.01,defaultparams,flags(true,true,false), assertingnonzero.
PedChannel comes directly from initializer82faba00. Status5 calls shared
try_end_all_channels_prematurely(length,true), then6. Status4/6 query actual
HasChannel and become7 only when absent. Metadata does not remove channels.
Root owns field retention and exact consumer scheduling. No compile/check/run.

pedestrian_motion_live.rs now borrows PedestrianConstruction and verifies exact
delegate/entity/core animation authority issuance. It uses the existing shared
ActorGraphIntents fixed64 map, shared core.try_is_transitioning, body clock and
play/blend/sequence methods; existing PedestrianAnimation feedback methods;
state/intent queue; and actual entity.jump_now/SpatialAdjust jump times.
capture_motion_intents must be called at UpdateAnimationData82e9a078, not on each
graph query: native clears the map, zeroes FastString36, copies five packed words
from FastString30 and stores1.0 in authored intent order. Completion callbacks
then affect next input without mutating that retained snapshot.

LWAnimHelper Setup82e90730 and Update82e90b50 are translated in the adapter.
Setup preserves old attributes/branch-foot sampling, register-new-attributes,
MirrorToMatchFeet82e6d370, shared remap selection, frame tag resolution, next
transition override/read-clear, actual core playback, cycle-tag resolution and
cycle write. Raw82e909ec..a24 confirms blend current-frame=false/placement0 and
sequence placement0. Failure honors kAssertOnMissingAnim, otherwise tries the
original HIP_TEST_01 and asserts its failure. No invented visual fallback.

Update clamps cycle probability using the two original fsel operations (NaN
selects1), queries Transitioning before expiration, then consumes shared Bernoulli
only when eligible. A successful draw remaps and plays at requested speed/time0.
It does NOT continuously overwrite playback speed every frame.

Pedestrian IAnimOperation vtable823a4e30 slots4/8/12 all point to825187c0 raw blr:
the play/sequence/blend notifications are proven noops. Set/Get cycle at slots20/24
are82e6e858/868. PDB tPedMotionGraphInput places frameOffset92, cycle96,
registeredClipBlock100, registeredTransition104 and AnimAttributes108; ctor
82e867a0 initializes the numbers0 and pointer fieldsnull.

ExternalOwners still requires actual retained animation attribute registration,
branch-window/tag/registered override state, next-transition params, channel
request status, shared RNG, hand-prop/waypoint and audio state. These
owners have no fabricated defaults or success stubs. Parent owns adding missing
retained fields and binding these source services to the session. This adapter
alone is not evidence of a playable animation path. No checks/build/tests/run.

Follow-up raw82e6fd68 plus PDB resolves SetAttemptingTakedown fully: update
state.flags[0] bit2; if true inline StopChannels(.2). tAnimData.PedAnimData=16,
cPedAnimData.ChannelInfo=124, tPedChannelInfo.params=32 and blendOutLength=20.
Therefore native mpAnimData+140 is ChannelInfo.Status (Finished7/BlendOut5), and
mpAnimData+192 is blendOutLength. The live adapter now implements the flag and
reuses stop_channels(.2); no separate unknown takedown operation remains.

New pedestrian_motion_behaviours.rs constructs all472 occurrences across all14
native classes. New pedestrian_motion_backend.rs connects these objects and the
condition registry to the existing pedestrian delegate: retain MotionObjects::bind()
once at Load, then call objects.update(delegate, dt, services). The adapter retains
specific service errors rather than reporting a successful or empty callback.
This does not construct missing live services or establish playable pedestrians.
No compile, syntax check, tests or runtime commands were run for this continuation.

Direct IDA decompilation and raw PPC confirm the following (confidence exact for
control flow; host implementation remains uncompiled):

* Base82cf6f88 inherits parent enabled and reads local enabled(defaulttrue).
  Behaviour82cf70f0 retains parent registration. All14 CreateInstance vtable
  slots point to82bb3da8: returned intrusive pointer is null, not a runtime token.
* StopChannel826eba08/826eba78 retains blendOutTime(default0.2) and uses existing
  cPedAnimData::StopChannels. KeepSimpleTimerActive826ec7e0 uses timerName with
  default TimedRangeRandom. Update826ef798 samples the existing timer, subtracts1,
  selects original timer if nonnegative and1 otherwise, then writes same owner.
* KeepMonitorIntentInQueue826ee980 hashes required nonempty intentName using
  existing sk8_hash_value_32. Update826ef6b8 reads the retained monitor map;
  only PacketState4 resubmits that SAME packet through existing IntentQueue::suggest.
* MajorIntentComplete826f2d98 and IntentStageComplete826f2e60 write retained
  AiGraphInput; decrement(defaultfalse) selects status2 rather than1.
  LocomotionTurnComplete826f2ed0 uses existing complete_locomotion_turn.
  Transition Begin826f2fc0/82e79580 clears transition_complete BEFORE calling
  existing IntentQueue::lock_base_locomotion82e75068. End826f3010 uses existing
  complete_locomotion_transition, preserving count-only intent truncation.
* SetAttemptingTakedown826f3218/826f3278 delegates to the full existing entity
  setter82e6fd68, which includes animation state/timer effects; a lone flag
  mutation is insufficient. TriggerPedestrianJump826f32d8 uses actual jump owner.
* ChanceToDropHandPropOnLargeAvoid826f3338 preserves BOTH HasHandProp calls and
  shared Bernoulli draw before actual ThrowHandPropAway. No locally seeded RNG.
* SpawnInteractionBasedHandProp82701b08 resolves the actual current waypoint,
  group and group behaviour. Raw82701b68..b78 moves the FULL64 return r3 to r4,
  compares cmpldi and calls SetHandPropToSpawn82e7af20 (decorated QAAX_K).
  Decompiler's apparent int/extra-argument signature is wrong; host key is u64.
* SendSpeechEvent826ed130 retains only numeric speechvalue after fctiwz and
  assertion1..71; speechevent is read/copied into a temporary, not retained.
  Begin826ed200 sets the actual pedestrian audio state when entity exists.
* PlayRemappedAnimation reuses existing constructor, Begin and Update directly.
  JumpAnimation82718110 adds originalLength(default1) and inDownwardMotion(false).
  Begin82718520 writes optional mirror BEFORE reading actual jump time, then
  computes originalLength/time and uses the same remapped playback owner.
  Update826f3408 resamples actual up/down time and reuses the generic Update.

All18 distinct generated StateUpdate bodies were reread directly in IDA.
Each slot writes the ORIGINAL Context instance before its direct call, even
when that call is raw blr825187c0. N=noop, M=monitor, T=timer, J=jump animation,
A=ordinary remapped animation; slots are ascending as written:

| Address | Direct slot sequence |
| --- | --- |
| 825187c0 | no slots |
| 828f5920 | N M T |
| 828f5990 | N N J |
| 828f5a00 | N J |
| 828f5a60 | N A N |
| 828f5ad0 | A N N |
| 828f5b40 | A N |
| 828f5ba0 | N M T A |
| 828f5c28 | A |
| 828f5c40 | N N A N |
| 828f5cc8 | N N N (folded symbol names delegate_sit) |
| 828f5d38 | N N A |
| 828f5da8 | N N N M T |
| 828f5e40 | N N N A |
| 828f5ec8 | N N N M T A (calls5da8 before slot5) |
| 828f81d0 | N |
| 828f81e8 | N N |
| 828f89c0 | N A |

Root integration still supplies a services borrow tied to the delegate's same
entity authority. Reuse PedestrianAnimation/core animation playback and its
AiGraphInput, existing PedestrianStateStorage, IntentQueue, SpatialAdjust,
waypoint/group, hand-prop owner and shared RNG. Conditions additionally need
actual animation attributes/probes/branch info, plugin lookup and HasIntent/
WillExpire collaborators. Missing audio or takedown owners remain explicit
service errors, never no-op success. Native Load passes call_behaviours=true;
allowGenerated is a separate routing argument, not this lifecycle boolean.

## Precondition and condition continuation

pedestrian_motion_expressions.rs now implements the complete114-case precondition
switch82922538, preserving negation, short-circuit order, default true and the
shared condition90 tail for expressions107/108. The runner invokes it directly
through the same condition cache as sustain. Backend.precondition is removed.
pedestrian_motion_conditions.rs binds all102 native conditions and evaluates
them using shared HasIntent/WillExpire constructors and activation kernels,
the existing declaration-parameterized state_matches helper, and native branch,
plugin and jump predicates. The state helper receives this pedestrian table;
no onboard graph or actor state is substituted.

Condition services still require actual live interface/fact producers: the
existing HasIntent/WillExpire collaborators, IBehaviourSink plugin result,
authored branch-window array/count, actual animation time/transition/properties,
typed BranchAnimInfo and original branch-window cvar, and actual spatial-adjust
jump bits. Missing owners remain errors or genuine optional-interface absence.
There is no successful stub implementation for these services.

Raw Load82cef180 proves the separate flags:82cef1bc r10=1(call_behaviours),
82cef1c4 r9=0(require_generated),82cef1c8 r8=1(allow_generated). This binary
ignores the incoming Load allowGenerated parameter and supplies the constants
to Create829513f0. At Pedestrian constructor82e97fd0 construct the selected
delegate with call_behaviours=true, following authored graph routing, after
tAnimData and before post/head/spatial adjustment and rig construction.

Additional source: InBranchWindow ctor826f3470 requires BranchAnimInfo;
LastState ctor82caa638 requires state. BranchWindowEnabled826fd618 checks
attribute-store/animatable interfaces and count!=0. InTurnBranchWindow826fd5b8
dispatches Pedestrian82e6e750: missing attributes false, expired/looped true,
otherwise current branch window. GetCurrentBranchWindow82e6e668 scans authored
windows with nonnegative start and inclusive time bounds. InBranchWindow827127b0
uses branch type0 for animation start/end/expired window and type1 for the
authored time interval. Raw82e6e6f0..708 and82712880..a0/e0/ec preserve CR
unordered behavior through negated LT/GT predicates. Jumpers826f3570/35c0 read
actual IsJumping82e702f0 and IsInDownwardJumpMotion82e70300, retaining null-entity
false. The predicate name IsLanding means NOT IsJumping, not a guessed contact.

No compilation, syntax checks, tests or runtime execution were performed.

This work adds the actual MotionGraph_Pedestrian declarations and a retained
lifecycle adapter. It does not claim a runnable pedestrian motion graph.
No compilation, syntax checks, tests or runtime execution were performed.

## Ownership and integration

New source files (including continuation):
- crates/skate2-core/src/animation/graphs/pedestrian_motion_delegate.rs
- crates/skate2-core/src/animation/graphs/pedestrian_motion_data.rs
- crates/skate2-core/src/animation/graphs/pedestrian_motion_expressions.rs
- crates/skate2-core/src/animation/graphs/pedestrian_motion_conditions.rs

Register only pedestrian_motion_delegate in lib.rs; its data module is a child.
The runner must become a field of the existing PedestrianAnimation, constructed
with the existing PedestrianConstruction authority. Do not issue another entity
identity. Native Pedestrian82e97748 loads the authored motion graph at82e97fd0
before PostAdjust/store7, HeadTracking/store8, SpatialAdjust/store9, NavRig/store3.
Default generated routing already recognizes MotionGraph_Pedestrian.xml as
delegate4. The onboard generated_motion_load is not this graph.

The runner reuses generated_graph_selection::Graph<7> and original declaration
types. Independent decompilation of GetNextState82951228, EvaluateState829326a8,
SearchTransitions829327b0/82932978 and FindCommonAncestors82931ae8 supports the
same selection template. It preserves:
- Update829562e8 clears all four cached-condition active words before selection.
- UpdateLists82952368 advances timers before publishing last/current state.
- Exit82932360 visits occurrences in descending order, copies all context slots,
  overlays the instance for End, removes/releases occurrences, then removes timers.
- Enter82953278 visits numeric state order, sets timers0, creates actual instances,
  inserts occurrences before Begin and overlays the instance on a context copy.
- UpdateBehaviours82931658 uses the selected generated StateUpdate function and
  may mutate the original context instance slot. It is not a loop over virtual Update.
- call_behaviours=false suppresses Begin/End/generated Update, but still constructs
  and releases actual instances.
- Transition829318f8 invokes no hooks because all441 native hook counts are zero.
- Entire sustain expression switch82925fb8 is translated, including default true.
  Listed conditions share the precondition cache by condition index.

PDB TemplateHC layout confirms seven timer states and seven active occurrences:
current8,last10,dt12,time map16,active244,instances264 (seven eight-byte records),
hooks320,delegate324,callBehaviours356,allowEvaluations357.
The runner's Rust representation is semantic, not guest ABI storage.

## Exact native declarations

| Array | Address | Entries | Entry bytes |
|---|---|---:|---:|
| kAttribs | 821c1f10 | 644 | 8 |
| kIndexArray | 821c3330 | 441 | 2 |
| kStates | 821c5880 | 572 | 32 |
| kTransitions | 821ca000 | 441 | 16 |
| kBehaviours | 821ce648 | 472 | 16 |
| kConditions | 821d08d0 | 102 | 8 |

Export used IDA's current native big-endian get_byte/get_word/get_dword and bounded
array counts read from PDB tinfo. Text reads followed the actual pointer fields.
Shared Section ordering is count then index. All behavior parameter counts and
transition hook counts are zero; the one-entry kHooks/kParams sentinel arrays
are not executable payloads and are not replaced with fabricated entries.

## Remaining explicit services

PedestrianMotionBackend requires concrete behavior constructors, live condition
service adapters, actual instance allocation/release, Begin/End and each selected
generated state-update body. Precondition and condition dispatch are now supplied
by the continuation above. The adapter
does not silently return successful results for these services. ControllerState
provides current/last/delta/timers directly from the retained runner, including
the pre-selection and post-selection timing distinction. The shared entity
authority and controller context are passed to callbacks.

Condition classes: BranchWindowEnabled, HasIntent, HasPlugin, InBranchWindow,
InTurnBranchWindow, LastState, PedestrianJumperInDownwardMotion,
PedestrianJumperIsLanding, WillExpire.

Behavior classes: ChanceToDropHandPropOnLargeAvoid, IntentStageComplete,
KeepMonitorIntentInQueue, KeepSimpleTimerActive, LocomotionTransition,
LocomotionTurnComplete, MajorIntentComplete, PlayPedestrianTimeScaledJumpAnimation,
PlayRemappedAnimation, SendSpeechEvent, SetAttemptingTakedown,
SpawnInteractionBasedHandProp, StopChannel, TriggerPedestrianJump.

Reuse the existing PlayRemappedAnimation and shared Andale animation owner.
The generic dynamic fallback's four-leaf coverage is insufficient for this table,
and must not be forced in place of native generated selection.

The runner is not registered or called by this worker; root owns shared files.
No entity admission, scheduler, teardown-all lifecycle, full animation evaluation
or pose publication is claimed. Existing partial PedestrianGraph AI remains
separate from this motion delegate.

## IDA source appendix

The following unmodified pseudocode is source evidence, not compiled host code.
Hex-Rays argument and union guesses require native instruction review when
implementing additional callbacks.


### 0x82922408

```cpp
void __fastcall StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::Init(
        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian *this)
{
  eastl::intrusive_ptr<BehaviourGraph::TransferExpression> *v2; // r24
  unsigned int i; // r26
  StateGraph::Factory<BehaviourGraph::TransferableBehaviour,BehaviourGraph::NodeBase> *v4; // r3
  BehaviourGraph::TransferExpression *v5; // r3
  IAttributeMap v6[32]; // [sp+50h] [-80h] BYREF

  if ( !StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sInit )
  {
    v2 = (eastl::intrusive_ptr<BehaviourGraph::TransferExpression> *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions;
    for ( i = 0; i < 0x66; ++i )
    {
      if ( (unsigned __int16)i >= 0x66u )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
          86);
        DebugBreak();
      }
      v4 = (StateGraph::Factory<BehaviourGraph::TransferableBehaviour,BehaviourGraph::NodeBase> *)SingletonHolder<StateGraph::Factory<StateGraph::Condition,StateGraph::ExpressionOperator>,DynamicMemorySingletonTypeTraits<StateGraph::Factory<StateGraph::Condition,StateGraph::ExpressionOperator>>>::sInstance;
      v6[0].__vftable = (IAttributeMap_vtbl *)&StateGraph::GenUtil::GenAttributeMap<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::`vftable';
      v6[1] = (IAttributeMap)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kConditions[(unsigned __int16)i].mName;
      v6[2].__vftable = (IAttributeMap_vtbl *)&StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kConditions[(unsigned __int16)i].mAttribs;
      if ( !SingletonHolder<StateGraph::Factory<StateGraph::Condition,StateGraph::ExpressionOperator>,DynamicMemorySingletonTypeTraits<StateGraph::Factory<StateGraph::Condition,StateGraph::ExpressionOperator>>>::sInstance )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\versioned\\singleton\\1.00.00\\singleton/singletonholder.h",
          126);
        DebugBreak();
        v4 = (StateGraph::Factory<BehaviourGraph::TransferableBehaviour,BehaviourGraph::NodeBase> *)SingletonHolder<StateGraph::Factory<StateGraph::Condition,StateGraph::ExpressionOperator>,DynamicMemorySingletonTypeTraits<StateGraph::Factory<StateGraph::Condition,StateGraph::ExpressionOperator>>>::sInstance;
      }
      v5 = (BehaviourGraph::TransferExpression *)StateGraph::Factory<BehaviourGraph::TransferableBehaviour,BehaviourGraph::NodeBase>::Create(
                                                   v4,
                                                   nullptr,
                                                   v6);
      eastl::intrusive_ptr<StateGraph::Transition>::operator=(v2, v5);
      v6[0].__vftable = (IAttributeMap_vtbl *)&IAttributeMap::`vftable';
      ++v2;
    }
    StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sInit = 1;
  }
  this->mConditionResults.mActive[0] = 0;
  this->mConditionResults.mActive[1] = 0;
  this->mConditionResults.mActive[2] = 0;
  this->mConditionResults.mActive[3] = 0;
}

```


### 0x82930ce8

```cpp
StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *__fastcall StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>(
        StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *this,
        StateGraph::Context *context,
        StateGraph::eDebugLevel debugLevel,
        StateGraph::eDebugFlushWhen debugFlushWhen,
        bool call_behaviours)
{
  StateGraph::GenUtil::BehaviourInstance *mInstances; // r10
  int i; // r9
  unsigned int v9; // r20
  eastl::intrusive_ptr<BehaviourGraph::TransferExpression> *v10; // r26
  StateGraph::Factory<BehaviourGraph::TransferableBehaviour,BehaviourGraph::NodeBase> *v11; // r3
  BehaviourGraph::TransferExpression *v12; // r3
  int mCount; // r31
  _WORD *v14; // r30
  int v15; // r29
  eastl::intrusive_ptr<BehaviourGraph::TransferExpression> v16; // r3
  _DWORD v18[4]; // [sp+50h] [-B0h] BYREF
  IAttributeMap v19[40]; // [sp+60h] [-A0h] BYREF

  StateGraph::AbstractHierarchicalController::AbstractHierarchicalController(
    this,
    context,
    35,
    debugLevel,
    debugFlushWhen);
  this->mCurrentState = -1;
  this->mDeltaTime = 0.0;
  this->__vftable = (StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>_vtbl *)&StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::`vftable';
  this->mLastState = -1;
  eastl::fixed_map<unsigned short,float,7,0,eastl::less<unsigned short>,eastl::allocator>::fixed_map<unsigned short,float,7,0,eastl::less<unsigned short>,eastl::allocator>(&this->mTimeInState);
  mInstances = this->mInstances;
  this->mCurrentBehaviours.mTop = 0;
  for ( i = 6; i >= 0; --i )
  {
    mInstances->mBehaviour = nullptr;
    mInstances->mInstance.mpObject = nullptr;
    ++mInstances;
  }
  this->mHooks[0].mpObject = nullptr;
  this->mDelegate.mConditionResults.mActive[0] = 0;
  this->mDelegate.mConditionResults.mActive[1] = 0;
  this->mDelegate.mConditionResults.mActive[2] = 0;
  this->mDelegate.mConditionResults.mActive[3] = 0;
  this->mCallBehaviours = call_behaviours;
  this->mAllowEvaluations = 0;
  if ( !`StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>'::`2'::printed )
    `StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>'::`2'::printed = 1;
  if ( !StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::sInit )
  {
    v9 = 0;
    v10 = (eastl::intrusive_ptr<BehaviourGraph::TransferExpression> *)StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::sBehaviours;
    do
    {
      if ( (unsigned __int16)v9 >= 0x1D8u )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
          85);
        DebugBreak();
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
          85);
        DebugBreak();
      }
      v11 = (StateGraph::Factory<BehaviourGraph::TransferableBehaviour,BehaviourGraph::NodeBase> *)SingletonHolder<StateGraph::Factory<StateGraph::Behaviour,StateGraph::State>,DynamicMemorySingletonTypeTraits<StateGraph::Factory<StateGraph::Behaviour,StateGraph::State>>>::sInstance;
      v19[0].__vftable = (IAttributeMap_vtbl *)&StateGraph::GenUtil::GenAttributeMap<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::`vftable';
      v19[1] = (IAttributeMap)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kBehaviours[(unsigned __int16)v9].mName;
      v19[2].__vftable = (IAttributeMap_vtbl *)&StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kBehaviours[(unsigned __int16)v9].mAttribs;
      if ( !SingletonHolder<StateGraph::Factory<StateGraph::Behaviour,StateGraph::State>,DynamicMemorySingletonTypeTraits<StateGraph::Factory<StateGraph::Behaviour,StateGraph::State>>>::sInstance )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\versioned\\singleton\\1.00.00\\singleton/singletonholder.h",
          126);
        DebugBreak();
        v11 = (StateGraph::Factory<BehaviourGraph::TransferableBehaviour,BehaviourGraph::NodeBase> *)SingletonHolder<StateGraph::Factory<StateGraph::Behaviour,StateGraph::State>,DynamicMemorySingletonTypeTraits<StateGraph::Factory<StateGraph::Behaviour,StateGraph::State>>>::sInstance;
      }
      v12 = (BehaviourGraph::TransferExpression *)StateGraph::Factory<BehaviourGraph::TransferableBehaviour,BehaviourGraph::NodeBase>::Create(
                                                    v11,
                                                    nullptr,
                                                    v19);
      eastl::intrusive_ptr<StateGraph::Transition>::operator=(v10, v12);
      if ( (unsigned __int16)v9 >= 0x1D8u )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
          85);
        DebugBreak();
      }
      mCount = StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kBehaviours[(unsigned __int16)v9].mParams.mCount;
      v14 = (_WORD *)((char *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kIndexArray
                    + __ROL4__(
                        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kBehaviours[(unsigned __int16)v9].mParams.mIndex,
                        1));
      if ( StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kBehaviours[(unsigned __int16)v9].mParams.mCount )
      {
        do
        {
          v15 = (unsigned __int16)*v14;
          if ( *v14 )
          {
            xcore::FailAssert(
              nullptr,
              "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
              89);
            DebugBreak();
          }
          v16.mpObject = v10->mpObject;
          v18[0] = &StateGraph::GenUtil::GenAttributeMap<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::`vftable';
          v18[1] = "param";
          v18[2] = &StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kParams[v15];
          ((void (__fastcall *)(BehaviourGraph::TransferExpression *__struct_ptr, _DWORD *))v16.mpObject->__vftable[1].CastToExpressionOperator)(
            v16.mpObject,
            v18);
          --mCount;
          v18[0] = &IAttributeMap::`vftable';
          ++v14;
        }
        while ( mCount );
      }
      ++v9;
      v19[0].__vftable = (IAttributeMap_vtbl *)&IAttributeMap::`vftable';
      ++v10;
      StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::sInit = 1;
    }
    while ( v9 < 0x1D8 );
  }
  StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::Init(&this->mDelegate);
  return this;
}

```


### 0x829562e8

```cpp
void __fastcall StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::Update(
        StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *this,
        double dt,
        StateGraph::GenUtil::IndexSet<7> *a3)
{
  unsigned __int16 mCurrentState; // r30
  unsigned __int16 updated; // r29
  StateGraph::GenUtil::IndexSet<7> v6; // [sp+50h] [-80h] BYREF
  StateGraph::GenUtil::IndexSet<7> v7; // [sp+70h] [-60h] BYREF
  StateGraph::GenUtil::IndexSet<7> v8[3]; // [sp+90h] [-40h] BYREF

  v6.mTop = 0;
  v7.mTop = 0;
  this->mDelegate.mConditionResults.mActive[0] = 0;
  this->mDelegate.mConditionResults.mActive[1] = 0;
  this->mDelegate.mConditionResults.mActive[2] = 0;
  this->mDelegate.mConditionResults.mActive[3] = 0;
  v8[0].mTop = 0;
  this->mDeltaTime = dt;
  mCurrentState = this->mCurrentState;
  updated = StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateLists(
              this,
              dt,
              a3,
              &v6,
              v8);
  StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateExitStates(this, &v6);
  StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateTransition(
    this,
    mCurrentState,
    updated);
  StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateEnterStates(this, &v7);
  StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateBehaviours(this);
}

```


### 0x82952368

```cpp
int __fastcall StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateLists(
        StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *this,
        double dT,
        StateGraph::GenUtil::IndexSet<7> *exitStates,
        StateGraph::GenUtil::IndexSet<7> *updateStates,
        StateGraph::GenUtil::IndexSet<7> *enterStates,
        StateGraph::GenUtil::IndexSet<7> *a6)
{
  unsigned __int16 mTargetState; // r30
  int NextState; // r3
  int v13; // r8
  int v14; // r25
  const StateGraph::GenUtil::StateDesc<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *v15; // r3
  eastl::rbtree_node_base *i; // r3
  unsigned __int16 v18; // r10
  unsigned __int16 v19; // [sp+50h] [-70h] BYREF
  unsigned int v20; // [sp+54h] [-6Ch] BYREF
  unsigned int v21; // [sp+58h] [-68h] BYREF
  StateGraph::GenUtil::IndexSet<7> v22; // [sp+60h] [-60h] BYREF

  mTargetState = -1;
  this->mAllowEvaluations = 1;
  v19 = -1;
  v22.mTop = 0;
  NextState = StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::GetNextState(
                this,
                &v19);
  v13 = v19;
  v14 = NextState;
  this->mAllowEvaluations = 0;
  if ( v13 == 0xFFFF )
  {
    xcore::FailAssert(
      nullptr,
      "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8statecore\\dev/include\\state/StateTemplatedHierarchicalController.h",
      946);
    DebugBreak();
  }
  if ( (unsigned __int16)v14 == 0xFFFF )
    goto LABEL_8;
  if ( (unsigned __int16)v14 >= 0x1B9u )
  {
    xcore::FailAssert(
      nullptr,
      "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
      87);
    DebugBreak();
  }
  mTargetState = StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kTransitions[(unsigned __int16)v14].mTargetState;
  if ( !StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::StateHasDescendent(
          this,
          mTargetState,
          this->mCurrentState) )
  {
LABEL_8:
    StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::FindCommonAncestors(
      this,
      v19,
      mTargetState,
      &v22,
      enterStates,
      updateStates,
      a6);
  }
  else
  {
    v15 = StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::State(mTargetState);
    StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::FindCommonAncestors(
      this,
      v19,
      v15->mParent,
      &v22,
      enterStates,
      updateStates,
      a6);
    StateGraph::GenUtil::IndexSet<7>::Insert(&v22, mTargetState, &v21, &v20);
  }
  for ( i = this->mTimeInState.mAnchor.mpNodeLeft; i != &this->mTimeInState.mAnchor; i = eastl::RBTreeIncrement(i) )
    *(float *)&i[1].mpNodeLeft = (float)dT + *(float *)&i[1].mpNodeLeft;
  v18 = v19;
  this->mLastState = this->mCurrentState;
  this->mCurrentState = v18;
  return v14;
}

```


### 0x82932360

```cpp
void __fastcall StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateExitStates(
        StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *this,
        StateGraph::GenUtil::IndexSet<7> *exitStates)
{
  StateGraph::GenUtil::IndexSet<4> *p_mCurrentBehaviours; // r25
  unsigned int mTop; // r28
  int v6; // r26
  unsigned int *v7; // r29
  unsigned int v8; // r27
  unsigned int v9; // r31
  int mParentState; // r4
  StateGraph::Context *mContext; // r11
  eastl::rbtree_node_base **p_mpNodeParent; // r10
  int v13; // r3
  int v14; // ctr
  unsigned int v15; // r29
  eastl::rbtree_node_base *p_mAnchor; // r27
  eastl::fixed_map<unsigned short,float,7,0,eastl::less<unsigned short>,eastl::allocator> *p_mTimeInState; // r31
  unsigned __int16 *mStack; // r28
  eastl::rbtree_node_base *v19; // r30
  eastl::fixed_pool_base::Link *mpHead; // r10
  unsigned __int16 v21; // [sp+50h] [-90h] BYREF
  eastl::rbtree<unsigned short,eastl::pair<unsigned short const ,float>,eastl::less<unsigned short>,eastl::fixed_node_pool<24,5,4,0,0,eastl::allocator>,eastl::use_first<eastl::pair<unsigned short const ,float> >,1,1> v22[3]; // [sp+54h] [-8Ch] BYREF

  p_mCurrentBehaviours = (StateGraph::GenUtil::IndexSet<4> *)&this->mCurrentBehaviours;
  mTop = this->mCurrentBehaviours.mTop;
  if ( mTop )
  {
    v6 = 2 * (mTop + 1);
    v7 = (unsigned int *)&this->mCurrentBehaviours.mStack[4 * mTop + 6];
    v8 = mTop - 1;
    do
    {
      if ( v8 >= p_mCurrentBehaviours->mTop )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8statecore\\dev/include\\state/StateTemplatedHierarchicalController.h",
          322);
        DebugBreak();
      }
      v9 = *(unsigned __int16 *)((char *)&p_mCurrentBehaviours->mTop + v6);
      if ( v9 >= 0x1D8 )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
          85);
        DebugBreak();
      }
      mParentState = StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kBehaviours[v9].mParentState;
      if ( mParentState != 0xFFFF && StateGraph::GenUtil::IndexSet<7>::test(exitStates, mParentState) )
      {
        if ( this->mCallBehaviours )
        {
          mContext = this->mContext;
          p_mpNodeParent = &v22[0].mAnchor.mpNodeParent;
          v13 = *(v7 - 1);
          v14 = 6;
          do
          {
            *p_mpNodeParent = (eastl::rbtree_node_base *)mContext->mController;
            mContext = (StateGraph::Context *)((char *)mContext + 4);
            ++p_mpNodeParent;
            --v14;
          }
          while ( v14 );
          v22[0].mnSize = *v7;
          (*(void (__fastcall **)(int, eastl::rbtree_node_base **))(*(_DWORD *)v13 + 56))(
            v13,
            &v22[0].mAnchor.mpNodeParent);
        }
        StateGraph::GenUtil::IndexSet<9>::removeat<StateGraph::GenUtil::BehaviourInstance>(
          p_mCurrentBehaviours,
          v8,
          this->mInstances);
      }
      --mTop;
      v6 -= 2;
      v7 -= 2;
      --v8;
    }
    while ( mTop );
  }
  v15 = 0;
  if ( exitStates->mTop )
  {
    p_mAnchor = &this->mTimeInState.mAnchor;
    p_mTimeInState = &this->mTimeInState;
    mStack = exitStates->mStack;
    do
    {
      v21 = *mStack;
      eastl::rbtree<unsigned short,eastl::pair<unsigned short const,float>,eastl::less<unsigned short>,eastl::fixed_node_pool<24,11,4,0,0,eastl::allocator>,eastl::use_first<eastl::pair<unsigned short const,float>>,1,1>::find(
        v22,
        (eastl::rbtree_iterator<eastl::pair<unsigned short const ,float>,eastl::pair<unsigned short const ,float> *,eastl::pair<unsigned short const ,float> &> *)p_mTimeInState,
        &v21);
      v19 = *(eastl::rbtree_node_base **)&v22[0].mCompare.eastl::binary_function<unsigned short,unsigned short,bool>;
      if ( *(eastl::rbtree_node_base **)&v22[0].mCompare.eastl::binary_function<unsigned short,unsigned short,bool> == p_mAnchor )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8statecore\\dev/include\\state/StateTemplatedHierarchicalController.h",
          1118);
        DebugBreak();
      }
      --p_mTimeInState->mnSize;
      eastl::RBTreeIncrement(v19);
      eastl::RBTreeErase(v19, &p_mTimeInState->mAnchor);
      mpHead = p_mTimeInState->mAllocator.mPool.mpHead;
      ++v15;
      ++mStack;
      --p_mTimeInState->mAllocator.mPool.mnCurrentSize;
      v19->mpNodeRight = (eastl::rbtree_node_base *)mpHead;
      p_mTimeInState->mAllocator.mPool.mpHead = (eastl::fixed_pool_base::Link *)v19;
    }
    while ( v15 < exitStates->mTop );
  }
}

```


### 0x82953278

```cpp
void __fastcall StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateEnterStates(
        StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *this,
        StateGraph::GenUtil::IndexSet<7> *enterStates)
{
  unsigned int v4; // r25
  unsigned __int16 *mStack; // r22
  unsigned int v6; // r31
  int mCount; // r30
  unsigned int v8; // r31
  unsigned int mIndex; // r11
  unsigned int v10; // r26
  unsigned int v11; // r29
  eastl::intrusive_ptr<StateGraph::Behaviour> *v12; // r28
  unsigned int mpObject; // r31
  eastl::intrusive_ptr<Sk8::SyncLog> *v14; // r3
  Sk8::SyncLog *v15; // r30
  char v19; // cr34
  unsigned int v20; // r3
  StateGraph::Context *mContext; // r11
  StateGraph::HierarchicalController **v22; // r10
  int v23; // ctr
  char v28; // cr34
  unsigned __int16 v29; // [sp+50h] [-C0h] BYREF
  char v30[4]; // [sp+54h] [-BCh] BYREF
  StateGraph::GenUtil::BehaviourInstance v31; // [sp+58h] [-B8h] BYREF
  StateGraph::GenUtil::BehaviourInstance v32; // [sp+60h] [-B0h] BYREF
  _BYTE v33[8]; // [sp+70h] [-A0h] BYREF
  StateGraph::Instance *v34; // [sp+78h] [-98h]

  v4 = 0;
  if ( enterStates->mTop )
  {
    mStack = enterStates->mStack;
    do
    {
      v29 = *mStack;
      *eastl::map<unsigned short,float,eastl::less<unsigned short>,eastl::fixed_node_pool<24,9,4,0,0,eastl::allocator>>::operator[](
         (eastl::map<unsigned short,float,eastl::less<unsigned short>,eastl::fixed_node_pool<24,5,4,0,0,eastl::allocator> > *)&this->mTimeInState,
         &v29) = 0.0;
      if ( v4 >= enterStates->mTop )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8statecore\\dev/include\\state/StateTemplatedHierarchicalController.h",
          322);
        DebugBreak();
      }
      v6 = *mStack;
      if ( v6 >= 0x23C )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
          84);
        DebugBreak();
      }
      mCount = StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kStates[v6].mBehaviours.mCount;
      if ( v4 >= enterStates->mTop )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8statecore\\dev/include\\state/StateTemplatedHierarchicalController.h",
          322);
        DebugBreak();
      }
      v8 = *mStack;
      if ( v8 >= 0x23C )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
          84);
        DebugBreak();
      }
      mIndex = StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kStates[v8].mBehaviours.mIndex;
      v10 = mIndex + mCount;
      v11 = mIndex;
      if ( mIndex < mIndex + mCount )
      {
        v12 = &StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::sBehaviours[mIndex];
        do
        {
          mpObject = (unsigned int)v12->mpObject;
          if ( v12->mpObject->mEnabled )
          {
            v14 = (eastl::intrusive_ptr<Sk8::SyncLog> *)(*(int (__fastcall **)(char *, StateGraph::Behaviour *))(*(_DWORD *)mpObject + 44))(
                                                          v30,
                                                          v12->mpObject);
            StateGraph::GenUtil::BehaviourInstance::BehaviourInstance(
              (eastl::pair<unsigned int,eastl::intrusive_ptr<Sk8::SyncLog> > *)&v31,
              mpObject,
              v14);
            v32 = v31;
            v15 = (Sk8::SyncLog *)v31.mInstance.mpObject;
            if ( v31.mInstance.mpObject )
            {
              _R11 = &v31.mInstance.mpObject->mRefCount;
              do
              {
                __asm
                {
                  mfmsr     r9
                  mtmsree   r13
                  lwarx     r10, 0, r11
                }
                ++_R10;
                __asm
                {
                  stwcx.    r10, 0, r11
                  mtmsree   r9
                }
              }
              while ( !v19 );
            }
            v20 = StateGraph::GenUtil::IndexSet<7>::set<StateGraph::GenUtil::BehaviourInstance>(
                    &this->mCurrentBehaviours,
                    v11,
                    this->mInstances,
                    &v32);
            if ( this->mCallBehaviours )
            {
              mContext = this->mContext;
              v22 = (StateGraph::HierarchicalController **)v33;
              v23 = 6;
              do
              {
                *v22 = mContext->mController;
                mContext = (StateGraph::Context *)((char *)mContext + 4);
                ++v22;
                --v23;
              }
              while ( v23 );
              v34 = this->mInstances[v20].mInstance.mpObject;
              (*(void (__fastcall **)(unsigned int, _BYTE *))(*(_DWORD *)mpObject + 48))(mpObject, v33);
            }
            v31.mBehaviour = nullptr;
            if ( v15 )
            {
              _R11 = &v15->mRefCount;
              do
              {
                __asm
                {
                  mfmsr     r9
                  mtmsree   r13
                  lwarx     r10, 0, r11
                }
                _R10 = _R10 - 1;
                __asm
                {
                  stwcx.    r10, 0, r11
                  mtmsree   r9
                }
              }
              while ( !v28 );
              if ( !_R10 )
                ((void (__fastcall *)(Sk8::SyncLog *, int))v15->dtr_RefCounter)(v15, 1);
            }
          }
          ++v11;
          ++v12;
        }
        while ( v11 < v10 );
      }
      ++v4;
      ++mStack;
    }
    while ( v4 < enterStates->mTop );
  }
}

```


### 0x82931658

```cpp
void __fastcall StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateBehaviours(
        StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *this)
{
  unsigned int v2; // r28
  StateGraph::GenUtil::IndexSet<7> *p_mCurrentBehaviours; // r29
  int v4; // r31
  StateGraph::GenUtil::BehaviourInstance *mInstances; // r30
  unsigned int mCurrentState; // r31
  StateGraph::Context *mContext; // r30

  v2 = 0;
  p_mCurrentBehaviours = &this->mCurrentBehaviours;
  if ( this->mCurrentBehaviours.mTop )
  {
    v4 = 4;
    mInstances = this->mInstances;
    do
    {
      if ( v2 >= p_mCurrentBehaviours->mTop )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8statecore\\dev/include\\state/StateTemplatedHierarchicalController.h",
          322);
        DebugBreak();
      }
      if ( mInstances->mBehaviour != *(StateGraph::Behaviour **)((char *)&StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::sBehaviours[0].mpObject
                                                               + __ROL4__(
                                                                   *(unsigned __int16 *)((char *)&p_mCurrentBehaviours->mTop
                                                                                       + v4),
                                                                   2)) )
      {
        xcore::FailAssert(
          nullptr,
          "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8statecore\\dev/include\\state/StateTemplatedHierarchicalController.h",
          1002);
        DebugBreak();
      }
      ++v2;
      v4 += 2;
      ++mInstances;
    }
    while ( v2 < p_mCurrentBehaviours->mTop );
  }
  if ( this->mCallBehaviours )
  {
    mCurrentState = this->mCurrentState;
    mContext = this->mContext;
    if ( mCurrentState >= 0x23C )
    {
      xcore::FailAssert(
        nullptr,
        "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
        84);
      DebugBreak();
    }
    StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kStates[mCurrentState].mUpdateFunc(mContext, this->mInstances);
  }
}

```


### 0x829318f8

```cpp
void __fastcall StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian>::UpdateTransition(
        StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian> *this,
        unsigned __int16 fromState,
        unsigned __int16 transition)
{
  unsigned int v3; // r30
  unsigned int v5; // r29
  unsigned __int16 mCount; // r30
  bool v7; // zf
  char *v8; // r29
  unsigned int i; // r31
  StateGraph::TransitionHook *mpObject; // r3

  v3 = transition;
  if ( transition != 0xFFFF )
  {
    if ( transition >= 0x1B9u )
    {
      xcore::FailAssert(
        nullptr,
        "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
        87);
      DebugBreak();
    }
    v5 = v3;
    v7 = v3 < 0x1B9;
    mCount = StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kTransitions[v3].mHooks.mCount;
    if ( !v7 )
    {
      xcore::FailAssert(
        nullptr,
        "D:\\checkouts\\sk82\\main\\packages\\sk8\\sk8generated\\dev\\generated/stategraph/MotionGraph_Pedestrian.cpp",
        87);
      DebugBreak();
    }
    v8 = (char *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kIndexArray
       + __ROL4__(StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::kTransitions[v5].mHooks.mIndex, 1);
    if ( mCount )
    {
      for ( i = 0; i < mCount; i = (unsigned __int16)(i + 1) )
      {
        mpObject = this->mHooks[*(unsigned __int16 *)&v8[2 * i]].mpObject;
        mpObject->OnTransition(mpObject, this->mContext);
      }
    }
  }
}

```


### 0x82922538

```cpp
int __fastcall StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::EvaluateExpression<1>(
        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian *this,
        unsigned __int16 expression,
        const StateGraph::Context *c,
        const StateGraph::State *p)
{
  int result; // r3
  bool HasIntent; // r3
  unsigned int v9; // r3
  unsigned int *mValues; // r28
  __int16 v11; // r3
  unsigned __int8 v12; // r11
  bool HasPlugin; // r3
  unsigned __int8 v14; // r3
  unsigned __int8 v15; // r3
  unsigned __int8 v16; // r3
  unsigned __int8 v17; // r3
  unsigned __int8 v18; // r3
  unsigned __int8 v19; // r3
  unsigned __int8 v20; // r3
  unsigned __int8 v21; // r3
  unsigned __int8 v22; // r3
  unsigned __int8 v23; // r3
  int v24; // r3
  bool v25; // r3
  unsigned int v26; // r3
  unsigned int *v27; // r28
  int v28; // r3
  unsigned __int8 v29; // r11
  unsigned int v30; // r3
  unsigned int *v31; // r28
  int v32; // r3
  unsigned __int8 v33; // r11
  __int16 v34; // r3
  unsigned int *v35; // r28
  int v36; // r3
  unsigned __int8 v37; // r11
  unsigned int v38; // r3
  unsigned int v39; // r3
  unsigned __int8 v40; // r11
  unsigned int v41; // r3
  unsigned int v42; // r3
  unsigned __int8 v43; // r11
  unsigned int v44; // r3
  unsigned int v45; // r3
  unsigned __int8 v46; // r11
  unsigned int v47; // r3
  unsigned int v48; // r3
  unsigned __int8 v49; // r11
  unsigned int v50; // r3
  unsigned int v51; // r3
  unsigned __int8 v52; // r11
  unsigned int v53; // r3
  unsigned int v54; // r3
  unsigned __int8 v55; // r11
  unsigned int v56; // r3
  unsigned int v57; // r3
  unsigned __int8 v58; // r11
  unsigned int v59; // r3
  unsigned int v60; // r3
  unsigned __int8 v61; // r11
  unsigned int v62; // r3
  unsigned int *v63; // r28
  int v64; // r3
  int v65; // r3
  unsigned int State; // r3
  unsigned int v67; // r3
  unsigned int v68; // r3
  unsigned __int8 v69; // r11
  unsigned int v70; // r3
  unsigned int v71; // r3
  unsigned int v72; // r3
  unsigned int v73; // r3
  unsigned int v74; // r3
  unsigned int v75; // r3
  unsigned int v76; // r3
  unsigned int v77; // r3
  unsigned int v78; // r3
  unsigned int v79; // r3
  unsigned int v80; // r3
  unsigned __int8 v81; // r11
  unsigned int v82; // r3
  unsigned int v83; // r3
  unsigned int v84; // r3
  unsigned int v85; // r3
  unsigned __int8 v86; // r11
  unsigned int v87; // r3
  unsigned int v88; // r3
  unsigned int v89; // r3
  unsigned int v90; // r3
  unsigned __int8 v91; // r11
  unsigned int v92; // r3
  unsigned int v93; // r3
  unsigned int v94; // r3
  unsigned int v95; // r3
  unsigned int v96; // r3
  unsigned __int8 v97; // r11

  switch ( expression )
  {
    case 0u:
      if ( (this->mConditionResults.mActive[0] & 1) != 0 )
        return this->mConditionResults.mValues[0] & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[0].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 1u;
      this->mConditionResults.mValues[0] = this->mConditionResults.mValues[0] & 0xFFFFFFFE | result & 1;
      return result;
    case 1u:
      if ( (this->mConditionResults.mActive[0] & 8) != 0 )
        return (this->mConditionResults.mValues[0] >> 3) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[3].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 8u;
      this->mConditionResults.mValues[0] = (8 * (unsigned __int8)result) & 8
                                         | this->mConditionResults.mValues[0] & 0xFFFFFFF7;
      return result;
    case 2u:
      if ( (this->mConditionResults.mActive[0] & 0x10) != 0 )
        return (this->mConditionResults.mValues[0] >> 4) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[4].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x10u;
      this->mConditionResults.mValues[0] = (16 * (unsigned __int8)result) & 0x10
                                         | this->mConditionResults.mValues[0] & 0xFFFFFFEF;
      return result;
    case 3u:
      if ( (this->mConditionResults.mActive[0] & 0x20) != 0 )
        return (this->mConditionResults.mValues[0] >> 5) & 1;
      result = StateConditionDirect_WillExpire(
                 (Sk8::Conditions::WillExpire *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[5].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x20u;
      this->mConditionResults.mValues[0] = (32 * (unsigned __int8)result) & 0x20
                                         | this->mConditionResults.mValues[0] & 0xFFFFFFDF;
      return result;
    case 4u:
      if ( (this->mConditionResults.mActive[0] & 0x40) != 0 )
        return (this->mConditionResults.mValues[0] >> 6) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[6].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x40u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 6) & 0x40
                                         | this->mConditionResults.mValues[0] & 0xFFFFFFBF;
      return result;
    case 5u:
      if ( (this->mConditionResults.mActive[0] & 0x80) != 0 )
        return (this->mConditionResults.mValues[0] >> 7) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[7].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x80u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 7) & 0x80
                                         | this->mConditionResults.mValues[0] & 0xFFFFFF7F;
      return result;
    case 6u:
      if ( (this->mConditionResults.mActive[0] & 2) != 0 )
        return (this->mConditionResults.mValues[0] >> 1) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[1].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 2u;
      this->mConditionResults.mValues[0] = (2 * (unsigned __int8)result) & 2
                                         | this->mConditionResults.mValues[0] & 0xFFFFFFFD;
      return result;
    case 7u:
      if ( (this->mConditionResults.mActive[0] & 0x100) != 0 )
        return (this->mConditionResults.mValues[0] >> 8) & 1;
      result = StateConditionDirect_WillExpire(
                 (Sk8::Conditions::WillExpire *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[8].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x100u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 8) & 0x100
                                         | this->mConditionResults.mValues[0] & 0xFFFFFEFF;
      return result;
    case 8u:
      if ( (this->mConditionResults.mActive[0] & 1) != 0 )
        return (this->mConditionResults.mValues[0] & 1) == 0;
      HasIntent = StateConditionDirect_HasIntent(
                    (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[0].mpObject,
                    c,
                    p);
      this->mConditionResults.mActive[0] |= 1u;
      this->mConditionResults.mValues[0] = this->mConditionResults.mValues[0] & 0xFFFFFFFE | HasIntent;
      return !HasIntent;
    case 9u:
      if ( (this->mConditionResults.mActive[0] & 0x200) != 0 )
        return (this->mConditionResults.mValues[0] >> 9) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[9].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x200u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 9) & 0x200
                                         | this->mConditionResults.mValues[0] & 0xFFFFFDFF;
      return result;
    case 0xAu:
      if ( (this->mConditionResults.mActive[0] & 0x400) != 0 )
        return (this->mConditionResults.mValues[0] >> 10) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[10].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x400u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 10) & 0x400
                                         | this->mConditionResults.mValues[0] & 0xFFFFFBFF;
      return result;
    case 0xBu:
      if ( (this->mConditionResults.mActive[0] & 0x800) != 0 )
        return (this->mConditionResults.mValues[0] >> 11) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[11].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x800u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 11) & 0x800
                                         | this->mConditionResults.mValues[0] & 0xFFFFF7FF;
      return result;
    case 0xCu:
      if ( (this->mConditionResults.mActive[0] & 0x1000) != 0 )
        return (this->mConditionResults.mValues[0] >> 12) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[12].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x1000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 12) & 0x1000
                                         | this->mConditionResults.mValues[0] & 0xFFFFEFFF;
      return result;
    case 0xDu:
      if ( (this->mConditionResults.mActive[0] & 0x2000) != 0 )
        return (this->mConditionResults.mValues[0] >> 13) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[13].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x2000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 13) & 0x2000
                                         | this->mConditionResults.mValues[0] & 0xFFFFDFFF;
      return result;
    case 0xEu:
      if ( (this->mConditionResults.mActive[0] & 0x4000) != 0 )
        return (this->mConditionResults.mValues[0] >> 14) & 1;
      result = StateConditionDirect_WillExpire(
                 (Sk8::Conditions::WillExpire *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[14].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x4000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 14) & 0x4000
                                         | this->mConditionResults.mValues[0] & 0xFFFFBFFF;
      return result;
    case 0xFu:
      if ( (this->mConditionResults.mActive[0] & 0x8000) != 0 )
      {
        v9 = (this->mConditionResults.mValues[0] >> 15) & 1;
      }
      else
      {
        LOBYTE(v9) = StateConditionDirect_HasIntent(
                       (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[15].mpObject,
                       c,
                       p);
        this->mConditionResults.mActive[0] |= 0x8000u;
        this->mConditionResults.mValues[0] = ((unsigned __int8)v9 << 15) & 0x8000
                                           | this->mConditionResults.mValues[0] & 0xFFFF7FFF;
      }
      mValues = this->mConditionResults.mValues;
      if ( !(_BYTE)v9 )
      {
        if ( (this->mConditionResults.mActive[0] & 0x10000) != 0 )
        {
          v11 = *(_WORD *)mValues & 1;
        }
        else
        {
          LOBYTE(v11) = StateConditionDirect_HasIntent(
                          (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[16].mpObject,
                          c,
                          p);
          this->mConditionResults.mActive[0] |= 0x10000u;
          *mValues = ((unsigned __int8)v11 << 16) & 0x10000 | *mValues & 0xFFFEFFFF;
        }
        v12 = 1;
        if ( !(_BYTE)v11 )
          return v12;
      }
      return 0;
    case 0x10u:
      if ( (this->mConditionResults.mActive[0] & 0x20000) != 0 )
        return (this->mConditionResults.mValues[0] >> 17) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[17].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x20000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 17) & 0x20000
                                         | this->mConditionResults.mValues[0] & 0xFFFDFFFF;
      return result;
    case 0x11u:
      if ( (this->mConditionResults.mActive[0] & 0x40000) != 0 )
        return (this->mConditionResults.mValues[0] >> 18) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[18].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x40000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 18) & 0x40000
                                         | this->mConditionResults.mValues[0] & 0xFFFBFFFF;
      return result;
    case 0x12u:
      if ( (this->mConditionResults.mActive[0] & 0x80000) != 0 )
        return (this->mConditionResults.mValues[0] >> 19) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[19].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x80000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 19) & 0x80000
                                         | this->mConditionResults.mValues[0] & 0xFFF7FFFF;
      return result;
    case 0x13u:
      if ( (this->mConditionResults.mActive[0] & 0x100000) != 0 )
        return (this->mConditionResults.mValues[0] >> 20) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[20].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x100000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 20) & 0x100000
                                         | this->mConditionResults.mValues[0] & 0xFFEFFFFF;
      return result;
    case 0x14u:
      if ( (this->mConditionResults.mActive[0] & 0x200000) != 0 )
        return (this->mConditionResults.mValues[0] >> 21) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[21].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x200000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 21) & 0x200000
                                         | this->mConditionResults.mValues[0] & 0xFFDFFFFF;
      return result;
    case 0x15u:
      if ( (this->mConditionResults.mActive[0] & 0x400000) != 0 )
        return (this->mConditionResults.mValues[0] >> 22) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[22].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x400000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 22) & 0x400000
                                         | this->mConditionResults.mValues[0] & 0xFFBFFFFF;
      return result;
    case 0x16u:
      if ( (this->mConditionResults.mActive[0] & 0x800000) != 0 )
        return (this->mConditionResults.mValues[0] >> 23) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[23].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x800000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 23) & 0x800000
                                         | this->mConditionResults.mValues[0] & 0xFF7FFFFF;
      return result;
    case 0x17u:
      if ( (this->mConditionResults.mActive[0] & 0x1000000) != 0 )
        return HIBYTE(this->mConditionResults.mValues[0]) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[24].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x1000000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 24) & 0x1000000
                                         | this->mConditionResults.mValues[0] & 0xFEFFFFFF;
      return result;
    case 0x18u:
      if ( (this->mConditionResults.mActive[0] & 0x2000000) != 0 )
        return (this->mConditionResults.mValues[0] >> 25) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[25].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x2000000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 25) & 0x2000000
                                         | this->mConditionResults.mValues[0] & 0xFDFFFFFF;
      return result;
    case 0x19u:
      if ( (this->mConditionResults.mActive[0] & 0x4000000) != 0 )
        return (this->mConditionResults.mValues[0] >> 26) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[26].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x4000000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 26) & 0x4000000
                                         | this->mConditionResults.mValues[0] & 0xFBFFFFFF;
      return result;
    case 0x1Au:
      if ( (this->mConditionResults.mActive[0] & 0x8000000) != 0 )
        return (this->mConditionResults.mValues[0] >> 27) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[27].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x8000000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 27) & 0x8000000
                                         | this->mConditionResults.mValues[0] & 0xF7FFFFFF;
      return result;
    case 0x1Bu:
      if ( (this->mConditionResults.mActive[0] & 0x80000000) != 0 )
        return this->mConditionResults.mValues[0] >> 31;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[31].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x80000000;
      this->mConditionResults.mValues[0] = (result << 31) | this->mConditionResults.mValues[0] & 0x7FFFFFFF;
      return result;
    case 0x1Cu:
      if ( (this->mConditionResults.mActive[1] & 1) != 0 )
        return this->mConditionResults.mValues[1] & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[32].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 1u;
      this->mConditionResults.mValues[1] = this->mConditionResults.mValues[1] & 0xFFFFFFFE | result & 1;
      return result;
    case 0x1Du:
      if ( (this->mConditionResults.mActive[1] & 2) != 0 )
        return (this->mConditionResults.mValues[1] >> 1) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[33].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 2u;
      this->mConditionResults.mValues[1] = (2 * (unsigned __int8)result) & 2
                                         | this->mConditionResults.mValues[1] & 0xFFFFFFFD;
      return result;
    case 0x1Eu:
      if ( (this->mConditionResults.mActive[1] & 4) != 0 )
        return (this->mConditionResults.mValues[1] >> 2) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[34].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 4u;
      this->mConditionResults.mValues[1] = (4 * (unsigned __int8)result) & 4
                                         | this->mConditionResults.mValues[1] & 0xFFFFFFFB;
      return result;
    case 0x1Fu:
      if ( (this->mConditionResults.mActive[1] & 8) != 0 )
        return ((this->mConditionResults.mValues[1] >> 3) & 1) == 0;
      HasPlugin = StateConditionDirect_HasPlugin(
                    StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[35].mpObject,
                    c,
                    p);
      this->mConditionResults.mActive[1] |= 8u;
      this->mConditionResults.mValues[1] = (8 * HasPlugin) & 8 | this->mConditionResults.mValues[1] & 0xFFFFFFF7;
      return !HasPlugin;
    case 0x20u:
      if ( (this->mConditionResults.mActive[0] & 0x20000000) != 0 )
        return (this->mConditionResults.mValues[0] >> 29) & 1;
      result = StateConditionDirect_LastState(
                 (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[29].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x20000000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 29) & 0x20000000
                                         | this->mConditionResults.mValues[0] & 0xDFFFFFFF;
      return result;
    case 0x21u:
      if ( (this->mConditionResults.mActive[0] & 0x10000000) != 0 )
        return (this->mConditionResults.mValues[0] >> 28) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[28].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[0] |= 0x10000000u;
      this->mConditionResults.mValues[0] = ((unsigned __int8)result << 28) & 0x10000000
                                         | this->mConditionResults.mValues[0] & 0xEFFFFFFF;
      return result;
    case 0x22u:
      if ( (this->mConditionResults.mActive[1] & 0x20) != 0 )
        return (this->mConditionResults.mValues[1] >> 5) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[37].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x20u;
      this->mConditionResults.mValues[1] = (32 * (unsigned __int8)result) & 0x20
                                         | this->mConditionResults.mValues[1] & 0xFFFFFFDF;
      return result;
    case 0x23u:
      if ( (this->mConditionResults.mActive[1] & 0x40) != 0 )
        return (this->mConditionResults.mValues[1] >> 6) & 1;
      result = StateConditionDirect_BranchWindowEnabled(
                 StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[38].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x40u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 6) & 0x40
                                         | this->mConditionResults.mValues[1] & 0xFFFFFFBF;
      return result;
    case 0x24u:
      if ( (this->mConditionResults.mActive[1] & 0x80) != 0 )
        return (this->mConditionResults.mValues[1] >> 7) & 1;
      result = StateConditionDirect_InTurnBranchWindow(
                 StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[39].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x80u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 7) & 0x80
                                         | this->mConditionResults.mValues[1] & 0xFFFFFF7F;
      return result;
    case 0x25u:
      if ( (this->mConditionResults.mActive[0] & 0x8000000) != 0 )
        return ((this->mConditionResults.mValues[0] >> 27) & 1) == 0;
      v14 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[27].mpObject,
              c,
              p);
      this->mConditionResults.mActive[0] |= 0x8000000u;
      this->mConditionResults.mValues[0] = (v14 << 27) & 0x8000000 | this->mConditionResults.mValues[0] & 0xF7FFFFFF;
      return v14 == 0;
    case 0x26u:
      if ( (this->mConditionResults.mActive[1] & 0x100) != 0 )
        return (this->mConditionResults.mValues[1] >> 8) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[40].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x100u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 8) & 0x100
                                         | this->mConditionResults.mValues[1] & 0xFFFFFEFF;
      return result;
    case 0x27u:
      if ( (this->mConditionResults.mActive[1] & 0x200) != 0 )
        return (this->mConditionResults.mValues[1] >> 9) & 1;
      result = StateConditionDirect_PedestrianJumperInDownwardMotion(
                 StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[41].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x200u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 9) & 0x200
                                         | this->mConditionResults.mValues[1] & 0xFFFFFDFF;
      return result;
    case 0x28u:
      if ( (this->mConditionResults.mActive[1] & 0x400) != 0 )
        return (this->mConditionResults.mValues[1] >> 10) & 1;
      result = StateConditionDirect_PedestrianJumperIsLanding(
                 StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[42].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x400u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 10) & 0x400
                                         | this->mConditionResults.mValues[1] & 0xFFFFFBFF;
      return result;
    case 0x29u:
      if ( (this->mConditionResults.mActive[1] & 0x800) != 0 )
        return ((this->mConditionResults.mValues[1] >> 11) & 1) == 0;
      v15 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[43].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x800u;
      this->mConditionResults.mValues[1] = (v15 << 11) & 0x800 | this->mConditionResults.mValues[1] & 0xFFFFF7FF;
      return v15 == 0;
    case 0x2Au:
      if ( (this->mConditionResults.mActive[1] & 0x1000) != 0 )
        return (this->mConditionResults.mValues[1] >> 12) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[44].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x1000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 12) & 0x1000
                                         | this->mConditionResults.mValues[1] & 0xFFFFEFFF;
      return result;
    case 0x2Bu:
      if ( (this->mConditionResults.mActive[1] & 0x1000) != 0 )
        return ((this->mConditionResults.mValues[1] >> 12) & 1) == 0;
      v16 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[44].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x1000u;
      this->mConditionResults.mValues[1] = (v16 << 12) & 0x1000 | this->mConditionResults.mValues[1] & 0xFFFFEFFF;
      return v16 == 0;
    case 0x2Cu:
      if ( (this->mConditionResults.mActive[1] & 0x2000) != 0 )
        return (this->mConditionResults.mValues[1] >> 13) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[45].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x2000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 13) & 0x2000
                                         | this->mConditionResults.mValues[1] & 0xFFFFDFFF;
      return result;
    case 0x2Du:
      if ( (this->mConditionResults.mActive[1] & 0x2000) != 0 )
        return ((this->mConditionResults.mValues[1] >> 13) & 1) == 0;
      v17 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[45].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x2000u;
      this->mConditionResults.mValues[1] = (v17 << 13) & 0x2000 | this->mConditionResults.mValues[1] & 0xFFFFDFFF;
      return v17 == 0;
    case 0x2Eu:
      if ( (this->mConditionResults.mActive[1] & 0x4000) != 0 )
        return (this->mConditionResults.mValues[1] >> 14) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[46].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x4000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 14) & 0x4000
                                         | this->mConditionResults.mValues[1] & 0xFFFFBFFF;
      return result;
    case 0x2Fu:
      if ( (this->mConditionResults.mActive[1] & 0x8000) != 0 )
        return (this->mConditionResults.mValues[1] >> 15) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[47].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x8000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 15) & 0x8000
                                         | this->mConditionResults.mValues[1] & 0xFFFF7FFF;
      return result;
    case 0x30u:
      if ( (this->mConditionResults.mActive[1] & 0x4000) != 0 )
        return ((this->mConditionResults.mValues[1] >> 14) & 1) == 0;
      v18 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[46].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x4000u;
      this->mConditionResults.mValues[1] = (v18 << 14) & 0x4000 | this->mConditionResults.mValues[1] & 0xFFFFBFFF;
      return v18 == 0;
    case 0x31u:
      if ( (this->mConditionResults.mActive[1] & 0x10000) != 0 )
        return HIWORD(this->mConditionResults.mValues[1]) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[48].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x10000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 16) & 0x10000
                                         | this->mConditionResults.mValues[1] & 0xFFFEFFFF;
      return result;
    case 0x32u:
      if ( (this->mConditionResults.mActive[1] & 0x100000) != 0 )
        return ((this->mConditionResults.mValues[1] >> 20) & 1) == 0;
      v19 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[52].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x100000u;
      this->mConditionResults.mValues[1] = (v19 << 20) & 0x100000 | this->mConditionResults.mValues[1] & 0xFFEFFFFF;
      return v19 == 0;
    case 0x33u:
      if ( (this->mConditionResults.mActive[1] & 0x200000) != 0 )
        return (this->mConditionResults.mValues[1] >> 21) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[53].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x200000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 21) & 0x200000
                                         | this->mConditionResults.mValues[1] & 0xFFDFFFFF;
      return result;
    case 0x34u:
      if ( (this->mConditionResults.mActive[1] & 0x400000) != 0 )
        return ((this->mConditionResults.mValues[1] >> 22) & 1) == 0;
      v20 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[54].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x400000u;
      this->mConditionResults.mValues[1] = (v20 << 22) & 0x400000 | this->mConditionResults.mValues[1] & 0xFFBFFFFF;
      return v20 == 0;
    case 0x35u:
      if ( (this->mConditionResults.mActive[1] & 0x800000) != 0 )
        return (this->mConditionResults.mValues[1] >> 23) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[55].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x800000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 23) & 0x800000
                                         | this->mConditionResults.mValues[1] & 0xFF7FFFFF;
      return result;
    case 0x36u:
      if ( (this->mConditionResults.mActive[1] & 0x1000000) != 0 )
        return (this->mConditionResults.mValues[1] & 0x1000000) == 0;
      v21 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[56].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x1000000u;
      this->mConditionResults.mValues[1] = (v21 << 24) & 0x1000000 | this->mConditionResults.mValues[1] & 0xFEFFFFFF;
      return v21 == 0;
    case 0x37u:
      if ( (this->mConditionResults.mActive[1] & 0x2000000) != 0 )
        return (this->mConditionResults.mValues[1] >> 25) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[57].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x2000000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 25) & 0x2000000
                                         | this->mConditionResults.mValues[1] & 0xFDFFFFFF;
      return result;
    case 0x38u:
      if ( (this->mConditionResults.mActive[1] & 0x4000000) != 0 )
        return (this->mConditionResults.mValues[1] >> 26) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[58].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x4000000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 26) & 0x4000000
                                         | this->mConditionResults.mValues[1] & 0xFBFFFFFF;
      return result;
    case 0x39u:
      if ( (this->mConditionResults.mActive[1] & 0x8000000) != 0 )
        return ((this->mConditionResults.mValues[1] >> 27) & 1) == 0;
      v22 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[59].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x8000000u;
      this->mConditionResults.mValues[1] = (v22 << 27) & 0x8000000 | this->mConditionResults.mValues[1] & 0xF7FFFFFF;
      return v22 == 0;
    case 0x3Au:
      if ( (this->mConditionResults.mActive[1] & 0x10000000) != 0 )
        return (this->mConditionResults.mValues[1] >> 28) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[60].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x10000000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 28) & 0x10000000
                                         | this->mConditionResults.mValues[1] & 0xEFFFFFFF;
      return result;
    case 0x3Bu:
      if ( (this->mConditionResults.mActive[1] & 0x20000000) != 0 )
        return ((this->mConditionResults.mValues[1] >> 29) & 1) == 0;
      v23 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[61].mpObject,
              c,
              p);
      this->mConditionResults.mActive[1] |= 0x20000000u;
      this->mConditionResults.mValues[1] = (v23 << 29) & 0x20000000 | this->mConditionResults.mValues[1] & 0xDFFFFFFF;
      return v23 == 0;
    case 0x3Cu:
      if ( (this->mConditionResults.mActive[1] & 0x40000000) != 0 )
        return (this->mConditionResults.mValues[1] >> 30) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[62].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x40000000u;
      this->mConditionResults.mValues[1] = ((unsigned __int8)result << 30) & 0x40000000
                                         | this->mConditionResults.mValues[1] & 0xBFFFFFFF;
      return result;
    case 0x3Du:
      if ( (this->mConditionResults.mActive[2] & 4) != 0 )
        return (this->mConditionResults.mValues[2] >> 2) & 1;
      result = StateConditionDirect_WillExpire(
                 (Sk8::Conditions::WillExpire *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[66].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 4u;
      this->mConditionResults.mValues[2] = (4 * (unsigned __int8)result) & 4
                                         | this->mConditionResults.mValues[2] & 0xFFFFFFFB;
      return result;
    case 0x3Eu:
      if ( (this->mConditionResults.mActive[1] & 0x10) != 0 )
        return (this->mConditionResults.mValues[1] >> 4) & 1;
      result = StateConditionDirect_LastState(
                 (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[36].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x10u;
      this->mConditionResults.mValues[1] = (16 * (unsigned __int8)result) & 0x10
                                         | this->mConditionResults.mValues[1] & 0xFFFFFFEF;
      return result;
    case 0x3Fu:
      if ( (this->mConditionResults.mActive[2] & 8) != 0 )
        return (this->mConditionResults.mValues[2] >> 3) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[67].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 8u;
      this->mConditionResults.mValues[2] = (8 * (unsigned __int8)result) & 8
                                         | this->mConditionResults.mValues[2] & 0xFFFFFFF7;
      return result;
    case 0x40u:
      if ( (this->mConditionResults.mActive[2] & 0x10) != 0 )
        return (this->mConditionResults.mValues[2] >> 4) & 1;
      result = StateConditionDirect_WillExpire(
                 (Sk8::Conditions::WillExpire *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[68].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x10u;
      this->mConditionResults.mValues[2] = (16 * (unsigned __int8)result) & 0x10
                                         | this->mConditionResults.mValues[2] & 0xFFFFFFEF;
      return result;
    case 0x41u:
      if ( (this->mConditionResults.mActive[2] & 0x20) != 0 )
        return (this->mConditionResults.mValues[2] >> 5) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[69].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x20u;
      this->mConditionResults.mValues[2] = (32 * (unsigned __int8)result) & 0x20
                                         | this->mConditionResults.mValues[2] & 0xFFFFFFDF;
      return result;
    case 0x42u:
      if ( (this->mConditionResults.mActive[2] & 0x40) != 0 )
        return (this->mConditionResults.mValues[2] >> 6) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[70].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x40u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 6) & 0x40
                                         | this->mConditionResults.mValues[2] & 0xFFFFFFBF;
      return result;
    case 0x43u:
      if ( (this->mConditionResults.mActive[2] & 0x80) != 0 )
        return (this->mConditionResults.mValues[2] >> 7) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[71].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x80u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 7) & 0x80
                                         | this->mConditionResults.mValues[2] & 0xFFFFFF7F;
      return result;
    case 0x44u:
      if ( (this->mConditionResults.mActive[2] & 0x100) != 0 )
        return (this->mConditionResults.mValues[2] >> 8) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[72].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x100u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 8) & 0x100
                                         | this->mConditionResults.mValues[2] & 0xFFFFFEFF;
      return result;
    case 0x45u:
      if ( (this->mConditionResults.mActive[2] & 0x200) != 0 )
        return (this->mConditionResults.mValues[2] >> 9) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[73].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x200u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 9) & 0x200
                                         | this->mConditionResults.mValues[2] & 0xFFFFFDFF;
      return result;
    case 0x46u:
      if ( (this->mConditionResults.mActive[2] & 0x400) != 0 )
        return (this->mConditionResults.mValues[2] >> 10) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[74].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x400u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 10) & 0x400
                                         | this->mConditionResults.mValues[2] & 0xFFFFFBFF;
      return result;
    case 0x47u:
      if ( (this->mConditionResults.mActive[2] & 0x800) != 0 )
        return (this->mConditionResults.mValues[2] >> 11) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[75].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x800u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 11) & 0x800
                                         | this->mConditionResults.mValues[2] & 0xFFFFF7FF;
      return result;
    case 0x48u:
      if ( (this->mConditionResults.mActive[2] & 0x1000) != 0 )
        return (this->mConditionResults.mValues[2] >> 12) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[76].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x1000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 12) & 0x1000
                                         | this->mConditionResults.mValues[2] & 0xFFFFEFFF;
      return result;
    case 0x49u:
      if ( (this->mConditionResults.mActive[2] & 0x2000) != 0 )
        return (this->mConditionResults.mValues[2] >> 13) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[77].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x2000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 13) & 0x2000
                                         | this->mConditionResults.mValues[2] & 0xFFFFDFFF;
      return result;
    case 0x4Au:
      if ( (this->mConditionResults.mActive[2] & 0x4000) != 0 )
        return (this->mConditionResults.mValues[2] >> 14) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[78].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x4000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 14) & 0x4000
                                         | this->mConditionResults.mValues[2] & 0xFFFFBFFF;
      return result;
    case 0x4Bu:
      if ( (this->mConditionResults.mActive[2] & 0x8000) != 0 )
        return (this->mConditionResults.mValues[2] >> 15) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[79].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x8000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 15) & 0x8000
                                         | this->mConditionResults.mValues[2] & 0xFFFF7FFF;
      return result;
    case 0x4Cu:
      if ( (this->mConditionResults.mActive[2] & 0x10000) != 0 )
        return HIWORD(this->mConditionResults.mValues[2]) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[80].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x10000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 16) & 0x10000
                                         | this->mConditionResults.mValues[2] & 0xFFFEFFFF;
      return result;
    case 0x4Du:
      if ( (this->mConditionResults.mActive[2] & 0x20000) != 0 )
        return (this->mConditionResults.mValues[2] >> 17) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[81].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x20000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 17) & 0x20000
                                         | this->mConditionResults.mValues[2] & 0xFFFDFFFF;
      return result;
    case 0x4Eu:
      if ( (this->mConditionResults.mActive[2] & 0x40000) != 0 )
        return (this->mConditionResults.mValues[2] >> 18) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[82].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x40000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 18) & 0x40000
                                         | this->mConditionResults.mValues[2] & 0xFFFBFFFF;
      return result;
    case 0x4Fu:
      if ( (this->mConditionResults.mActive[2] & 0x80000) != 0 )
        return (this->mConditionResults.mValues[2] >> 19) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[83].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x80000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 19) & 0x80000
                                         | this->mConditionResults.mValues[2] & 0xFFF7FFFF;
      return result;
    case 0x50u:
      if ( (this->mConditionResults.mActive[2] & 0x200000) != 0 )
        return (this->mConditionResults.mValues[2] >> 21) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[85].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x200000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 21) & 0x200000
                                         | this->mConditionResults.mValues[2] & 0xFFDFFFFF;
      return result;
    case 0x51u:
      if ( (this->mConditionResults.mActive[2] & 0x400000) != 0 )
        return (this->mConditionResults.mValues[2] >> 22) & 1;
      result = StateConditionDirect_LastState(
                 (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[86].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x400000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 22) & 0x400000
                                         | this->mConditionResults.mValues[2] & 0xFFBFFFFF;
      return result;
    case 0x52u:
      if ( (this->mConditionResults.mActive[2] & 0x1000000) != 0 )
        return HIBYTE(this->mConditionResults.mValues[2]) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[88].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x1000000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 24) & 0x1000000
                                         | this->mConditionResults.mValues[2] & 0xFEFFFFFF;
      return result;
    case 0x53u:
      if ( (this->mConditionResults.mActive[2] & 0x8000000) != 0 )
        return (this->mConditionResults.mValues[2] >> 27) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[91].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x8000000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 27) & 0x8000000
                                         | this->mConditionResults.mValues[2] & 0xF7FFFFFF;
      return result;
    case 0x54u:
      if ( (this->mConditionResults.mActive[2] & 0x20000000) != 0 )
        return (this->mConditionResults.mValues[2] >> 29) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[93].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x20000000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 29) & 0x20000000
                                         | this->mConditionResults.mValues[2] & 0xDFFFFFFF;
      return result;
    case 0x55u:
      if ( (this->mConditionResults.mActive[2] & 0x40000000) != 0 )
        return (this->mConditionResults.mValues[2] >> 30) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[94].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x40000000u;
      this->mConditionResults.mValues[2] = ((unsigned __int8)result << 30) & 0x40000000
                                         | this->mConditionResults.mValues[2] & 0xBFFFFFFF;
      return result;
    case 0x56u:
      if ( (this->mConditionResults.mActive[1] & 0x80000000) != 0 )
        return this->mConditionResults.mValues[1] >> 31;
      result = StateConditionDirect_LastState(
                 (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[63].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[1] |= 0x80000000;
      this->mConditionResults.mValues[1] = (result << 31) | this->mConditionResults.mValues[1] & 0x7FFFFFFF;
      return result;
    case 0x57u:
      if ( (this->mConditionResults.mActive[2] & 1) != 0 )
        return this->mConditionResults.mValues[2] & 1;
      result = StateConditionDirect_LastState(
                 (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[64].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 1u;
      this->mConditionResults.mValues[2] = this->mConditionResults.mValues[2] & 0xFFFFFFFE | result & 1;
      return result;
    case 0x58u:
      if ( (this->mConditionResults.mActive[2] & 0x80000000) != 0 )
        return this->mConditionResults.mValues[2] >> 31;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[95].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[2] |= 0x80000000;
      this->mConditionResults.mValues[2] = (result << 31) | this->mConditionResults.mValues[2] & 0x7FFFFFFF;
      return result;
    case 0x59u:
      if ( (this->mConditionResults.mActive[2] & 0x80000000) != 0 )
        return this->mConditionResults.mValues[2] >> 31 == 0;
      v24 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[95].mpObject,
              c,
              p);
      this->mConditionResults.mActive[2] |= 0x80000000;
      this->mConditionResults.mValues[2] = (v24 << 31) | this->mConditionResults.mValues[2] & 0x7FFFFFFF;
      return (unsigned __int8)v24 == 0;
    case 0x5Au:
      if ( (this->mConditionResults.mActive[3] & 1) != 0 )
        return this->mConditionResults.mValues[3] & 1;
      result = StateConditionDirect_LastState(
                 (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[96].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[3] |= 1u;
      this->mConditionResults.mValues[3] = this->mConditionResults.mValues[3] & 0xFFFFFFFE | result & 1;
      return result;
    case 0x5Bu:
      if ( (this->mConditionResults.mActive[3] & 2) != 0 )
        return (this->mConditionResults.mValues[3] >> 1) & 1;
      result = StateConditionDirect_InBranchWindow(
                 StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[97].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[3] |= 2u;
      this->mConditionResults.mValues[3] = (2 * (unsigned __int8)result) & 2
                                         | this->mConditionResults.mValues[3] & 0xFFFFFFFD;
      return result;
    case 0x5Cu:
      if ( (this->mConditionResults.mActive[3] & 4) != 0 )
        return ((this->mConditionResults.mValues[3] >> 2) & 1) == 0;
      v25 = StateConditionDirect_HasIntent(
              (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[98].mpObject,
              c,
              p);
      this->mConditionResults.mActive[3] |= 4u;
      this->mConditionResults.mValues[3] = (4 * v25) & 4 | this->mConditionResults.mValues[3] & 0xFFFFFFFB;
      return !v25;
    case 0x5Du:
      if ( (this->mConditionResults.mActive[3] & 8) != 0 )
        return (this->mConditionResults.mValues[3] >> 3) & 1;
      result = StateConditionDirect_HasIntent(
                 (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[99].mpObject,
                 c,
                 p);
      this->mConditionResults.mActive[3] |= 8u;
      this->mConditionResults.mValues[3] = (8 * (unsigned __int8)result) & 8
                                         | this->mConditionResults.mValues[3] & 0xFFFFFFF7;
      return result;
    case 0x5Eu:
      if ( (this->mConditionResults.mActive[0] & 2) != 0 )
      {
        v26 = (this->mConditionResults.mValues[0] >> 1) & 1;
      }
      else
      {
        LOBYTE(v26) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[1].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 2u;
        this->mConditionResults.mValues[0] = (2 * (unsigned __int8)v26) & 2
                                           | this->mConditionResults.mValues[0] & 0xFFFFFFFD;
      }
      v27 = this->mConditionResults.mValues;
      if ( (_BYTE)v26 )
        return 1;
      if ( (this->mConditionResults.mActive[0] & 4) != 0 )
      {
        v28 = (*v27 >> 2) & 1;
      }
      else
      {
        LOBYTE(v28) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[2].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 4u;
        *v27 = (4 * (unsigned __int8)v28) & 4 | *v27 & 0xFFFFFFFB;
      }
      v29 = 0;
      if ( (_BYTE)v28 )
        return 1;
      return v29;
    case 0x5Fu:
      if ( (this->mConditionResults.mActive[0] & 0x8000) != 0 )
      {
        v30 = (this->mConditionResults.mValues[0] >> 15) & 1;
      }
      else
      {
        LOBYTE(v30) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[15].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x8000u;
        this->mConditionResults.mValues[0] = ((unsigned __int8)v30 << 15) & 0x8000
                                           | this->mConditionResults.mValues[0] & 0xFFFF7FFF;
      }
      v31 = this->mConditionResults.mValues;
      if ( !(_BYTE)v30 )
        return 0;
      if ( (this->mConditionResults.mActive[0] & 0x20) != 0 )
      {
        v32 = (*v31 >> 5) & 1;
      }
      else
      {
        LOBYTE(v32) = StateConditionDirect_WillExpire(
                        (Sk8::Conditions::WillExpire *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[5].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x20u;
        *v31 = (32 * (unsigned __int8)v32) & 0x20 | *v31 & 0xFFFFFFDF;
      }
      v33 = 1;
      if ( !(_BYTE)v32 )
        return 0;
      return v33;
    case 0x60u:
      if ( (this->mConditionResults.mActive[0] & 0x10000) != 0 )
      {
        v34 = HIWORD(this->mConditionResults.mValues[0]) & 1;
      }
      else
      {
        LOBYTE(v34) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[16].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x10000u;
        this->mConditionResults.mValues[0] = ((unsigned __int8)v34 << 16) & 0x10000
                                           | this->mConditionResults.mValues[0] & 0xFFFEFFFF;
      }
      v35 = this->mConditionResults.mValues;
      if ( !(_BYTE)v34 )
        return 0;
      if ( (this->mConditionResults.mActive[0] & 0x20) != 0 )
      {
        v36 = (*v35 >> 5) & 1;
      }
      else
      {
        LOBYTE(v36) = StateConditionDirect_WillExpire(
                        (Sk8::Conditions::WillExpire *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[5].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x20u;
        *v35 = (32 * (unsigned __int8)v36) & 0x20 | *v35 & 0xFFFFFFDF;
      }
      v37 = 1;
      if ( !(_BYTE)v36 )
        return 0;
      return v37;
    case 0x61u:
      if ( (this->mConditionResults.mActive[0] & 0x10000000) != 0 )
      {
        v38 = (this->mConditionResults.mValues[0] >> 28) & 1;
      }
      else
      {
        LOBYTE(v38) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[28].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x10000000u;
        this->mConditionResults.mValues[0] = ((unsigned __int8)v38 << 28) & 0x10000000
                                           | this->mConditionResults.mValues[0] & 0xEFFFFFFF;
      }
      if ( (_BYTE)v38 )
        return 1;
      if ( (this->mConditionResults.mActive[1] & 0x10) != 0 )
      {
        v39 = (this->mConditionResults.mValues[1] >> 4) & 1;
      }
      else
      {
        LOBYTE(v39) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[36].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x10u;
        this->mConditionResults.mValues[1] = (16 * (unsigned __int8)v39) & 0x10
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFEF;
      }
      v40 = 0;
      if ( (_BYTE)v39 )
        return 1;
      return v40;
    case 0x62u:
      if ( (this->mConditionResults.mActive[0] & 0x10000000) != 0 )
      {
        v41 = (this->mConditionResults.mValues[0] >> 28) & 1;
      }
      else
      {
        LOBYTE(v41) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[28].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x10000000u;
        this->mConditionResults.mValues[0] = ((unsigned __int8)v41 << 28) & 0x10000000
                                           | this->mConditionResults.mValues[0] & 0xEFFFFFFF;
      }
      if ( !(_BYTE)v41 )
        return 0;
      if ( (this->mConditionResults.mActive[1] & 0x80) != 0 )
      {
        v42 = (this->mConditionResults.mValues[1] >> 7) & 1;
      }
      else
      {
        LOBYTE(v42) = StateConditionDirect_InTurnBranchWindow(
                        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[39].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x80u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v42 << 7) & 0x80
                                           | this->mConditionResults.mValues[1] & 0xFFFFFF7F;
      }
      v43 = 1;
      if ( !(_BYTE)v42 )
        return 0;
      return v43;
    case 0x63u:
      if ( (this->mConditionResults.mActive[1] & 0x20000) != 0 )
      {
        v44 = (this->mConditionResults.mValues[1] >> 17) & 1;
      }
      else
      {
        LOBYTE(v44) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[49].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x20000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v44 << 17) & 0x20000
                                           | this->mConditionResults.mValues[1] & 0xFFFDFFFF;
      }
      if ( (_BYTE)v44 )
        return 0;
      if ( (this->mConditionResults.mActive[1] & 0x40000) != 0 )
      {
        v45 = (this->mConditionResults.mValues[1] >> 18) & 1;
      }
      else
      {
        LOBYTE(v45) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[50].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x40000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v45 << 18) & 0x40000
                                           | this->mConditionResults.mValues[1] & 0xFFFBFFFF;
      }
      v46 = 1;
      if ( (_BYTE)v45 )
        return 0;
      return v46;
    case 0x64u:
      if ( (this->mConditionResults.mActive[1] & 0x80000) != 0 )
      {
        v47 = (this->mConditionResults.mValues[1] >> 19) & 1;
      }
      else
      {
        LOBYTE(v47) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[51].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x80000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v47 << 19) & 0x80000
                                           | this->mConditionResults.mValues[1] & 0xFFF7FFFF;
      }
      if ( (_BYTE)v47 )
        return 1;
      if ( (this->mConditionResults.mActive[1] & 0x100000) != 0 )
      {
        v48 = (this->mConditionResults.mValues[1] >> 20) & 1;
      }
      else
      {
        LOBYTE(v48) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[52].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x100000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v48 << 20) & 0x100000
                                           | this->mConditionResults.mValues[1] & 0xFFEFFFFF;
      }
      v49 = 0;
      if ( !(_BYTE)v48 )
        return 1;
      return v49;
    case 0x65u:
      if ( (this->mConditionResults.mActive[2] & 0x100000) != 0 )
      {
        v50 = (this->mConditionResults.mValues[2] >> 20) & 1;
      }
      else
      {
        LOBYTE(v50) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[84].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 0x100000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)v50 << 20) & 0x100000
                                           | this->mConditionResults.mValues[2] & 0xFFEFFFFF;
      }
      if ( !(_BYTE)v50 )
        return 0;
      if ( (this->mConditionResults.mActive[1] & 0x40) != 0 )
      {
        v51 = (this->mConditionResults.mValues[1] >> 6) & 1;
      }
      else
      {
        LOBYTE(v51) = StateConditionDirect_BranchWindowEnabled(
                        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[38].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x40u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v51 << 6) & 0x40
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFBF;
      }
      v52 = 1;
      if ( !(_BYTE)v51 )
        return 0;
      return v52;
    case 0x66u:
      if ( (this->mConditionResults.mActive[3] & 4) != 0 )
      {
        v53 = (this->mConditionResults.mValues[3] >> 2) & 1;
      }
      else
      {
        LOBYTE(v53) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[98].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[3] |= 4u;
        this->mConditionResults.mValues[3] = (4 * (unsigned __int8)v53) & 4
                                           | this->mConditionResults.mValues[3] & 0xFFFFFFFB;
      }
      if ( !(_BYTE)v53 )
        return 0;
      if ( (this->mConditionResults.mActive[0] & 0x100) != 0 )
      {
        v54 = (this->mConditionResults.mValues[0] >> 8) & 1;
      }
      else
      {
        LOBYTE(v54) = StateConditionDirect_WillExpire(
                        (Sk8::Conditions::WillExpire *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[8].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x100u;
        this->mConditionResults.mValues[0] = ((unsigned __int8)v54 << 8) & 0x100
                                           | this->mConditionResults.mValues[0] & 0xFFFFFEFF;
      }
      v55 = 1;
      if ( !(_BYTE)v54 )
        return 0;
      return v55;
    case 0x67u:
      if ( (this->mConditionResults.mActive[3] & 0x10) != 0 )
      {
        v56 = (this->mConditionResults.mValues[3] >> 4) & 1;
      }
      else
      {
        LOBYTE(v56) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[100].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[3] |= 0x10u;
        this->mConditionResults.mValues[3] = (16 * (unsigned __int8)v56) & 0x10
                                           | this->mConditionResults.mValues[3] & 0xFFFFFFEF;
      }
      if ( !(_BYTE)v56 )
        return 0;
      if ( (this->mConditionResults.mActive[1] & 0x40) != 0 )
      {
        v57 = (this->mConditionResults.mValues[1] >> 6) & 1;
      }
      else
      {
        LOBYTE(v57) = StateConditionDirect_BranchWindowEnabled(
                        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[38].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x40u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v57 << 6) & 0x40
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFBF;
      }
      v58 = 1;
      if ( !(_BYTE)v57 )
        return 0;
      return v58;
    case 0x68u:
      if ( (this->mConditionResults.mActive[3] & 0x20) != 0 )
      {
        v59 = (this->mConditionResults.mValues[3] >> 5) & 1;
      }
      else
      {
        LOBYTE(v59) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[101].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[3] |= 0x20u;
        this->mConditionResults.mValues[3] = (32 * (unsigned __int8)v59) & 0x20
                                           | this->mConditionResults.mValues[3] & 0xFFFFFFDF;
      }
      if ( !(_BYTE)v59 )
        return 0;
      if ( (this->mConditionResults.mActive[1] & 0x40) != 0 )
      {
        v60 = (this->mConditionResults.mValues[1] >> 6) & 1;
      }
      else
      {
        LOBYTE(v60) = StateConditionDirect_BranchWindowEnabled(
                        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[38].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x40u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v60 << 6) & 0x40
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFBF;
      }
      v61 = 1;
      if ( !(_BYTE)v60 )
        return 0;
      return v61;
    case 0x69u:
      if ( (this->mConditionResults.mActive[0] & 0x10000000) != 0 )
      {
        v62 = (this->mConditionResults.mValues[0] >> 28) & 1;
      }
      else
      {
        LOBYTE(v62) = StateConditionDirect_HasIntent(
                        (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[28].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x10000000u;
        this->mConditionResults.mValues[0] = ((unsigned __int8)v62 << 28) & 0x10000000
                                           | this->mConditionResults.mValues[0] & 0xEFFFFFFF;
      }
      v63 = this->mConditionResults.mValues;
      if ( !(_BYTE)v62 )
        return 0;
      if ( (this->mConditionResults.mActive[0] & 0x20000000) != 0 )
      {
        v64 = (*v63 >> 29) & 1;
      }
      else
      {
        LOBYTE(v64) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[29].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[0] |= 0x20000000u;
        *v63 = ((unsigned __int8)v64 << 29) & 0x20000000 | *v63 & 0xDFFFFFFF;
      }
      if ( !(_BYTE)v64 )
      {
        if ( (this->mConditionResults.mActive[0] & 0x40000000) != 0 )
        {
          v65 = (*v63 >> 30) & 1;
        }
        else
        {
          LOBYTE(v65) = StateConditionDirect_LastState(
                          (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[30].mpObject,
                          c,
                          p);
          this->mConditionResults.mActive[0] |= 0x40000000u;
          *v63 = ((unsigned __int8)v65 << 30) & 0x40000000 | *v63 & 0xBFFFFFFF;
        }
        if ( !(_BYTE)v65 )
          return 0;
      }
      return 1;
    case 0x6Au:
      if ( (this->mConditionResults.mActive[1] & 0x80000000) != 0 )
      {
        State = this->mConditionResults.mValues[1] >> 31;
      }
      else
      {
        State = StateConditionDirect_LastState(
                  (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[63].mpObject,
                  c,
                  p);
        this->mConditionResults.mActive[1] |= 0x80000000;
        this->mConditionResults.mValues[1] = (State << 31) | this->mConditionResults.mValues[1] & 0x7FFFFFFF;
      }
      if ( (_BYTE)State )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 1) != 0 )
      {
        v67 = this->mConditionResults.mValues[2] & 1;
      }
      else
      {
        LOBYTE(v67) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[64].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 1u;
        this->mConditionResults.mValues[2] = this->mConditionResults.mValues[2] & 0xFFFFFFFE | v67 & 1;
      }
      if ( (_BYTE)v67 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 2) != 0 )
      {
        v68 = (this->mConditionResults.mValues[2] >> 1) & 1;
      }
      else
      {
        LOBYTE(v68) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[65].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 2u;
        this->mConditionResults.mValues[2] = (2 * (unsigned __int8)v68) & 2
                                           | this->mConditionResults.mValues[2] & 0xFFFFFFFD;
      }
      v69 = 0;
      if ( (_BYTE)v68 )
        return 1;
      return v69;
    case 0x6Bu:
      if ( (this->mConditionResults.mActive[1] & 0x40) != 0 )
      {
        v70 = (this->mConditionResults.mValues[1] >> 6) & 1;
      }
      else
      {
        LOBYTE(v70) = StateConditionDirect_BranchWindowEnabled(
                        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[38].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x40u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v70 << 6) & 0x40
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFBF;
      }
      if ( !(_BYTE)v70 )
        return 0;
      if ( (this->mConditionResults.mActive[2] & 0x2000000) != 0 )
      {
        v71 = (this->mConditionResults.mValues[2] >> 25) & 1;
      }
      else
      {
        LOBYTE(v71) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[89].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 0x2000000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)v71 << 25) & 0x2000000
                                           | this->mConditionResults.mValues[2] & 0xFDFFFFFF;
      }
      if ( (_BYTE)v71 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 0x4000000) == 0 )
        goto LABEL_437;
      goto LABEL_438;
    case 0x6Cu:
      if ( (this->mConditionResults.mActive[1] & 0x40) != 0 )
      {
        v73 = (this->mConditionResults.mValues[1] >> 6) & 1;
      }
      else
      {
        LOBYTE(v73) = StateConditionDirect_BranchWindowEnabled(
                        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[38].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x40u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v73 << 6) & 0x40
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFBF;
      }
      if ( !(_BYTE)v73 )
        return 0;
      if ( (this->mConditionResults.mActive[2] & 0x10000000) != 0 )
      {
        v74 = (this->mConditionResults.mValues[2] >> 28) & 1;
      }
      else
      {
        LOBYTE(v74) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[92].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 0x10000000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)v74 << 28) & 0x10000000
                                           | this->mConditionResults.mValues[2] & 0xEFFFFFFF;
      }
      if ( (_BYTE)v74 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 0x4000000) != 0 )
      {
LABEL_438:
        v72 = (this->mConditionResults.mValues[2] >> 26) & 1;
      }
      else
      {
LABEL_437:
        LOBYTE(v72) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[90].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 0x4000000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)v72 << 26) & 0x4000000
                                           | this->mConditionResults.mValues[2] & 0xFBFFFFFF;
      }
      return (_BYTE)v72 != 0;
    case 0x6Du:
      if ( (this->mConditionResults.mActive[1] & 0x40) != 0 )
      {
        v75 = (this->mConditionResults.mValues[1] >> 6) & 1;
      }
      else
      {
        LOBYTE(v75) = StateConditionDirect_BranchWindowEnabled(
                        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[38].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x40u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v75 << 6) & 0x40
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFBF;
      }
      if ( !(_BYTE)v75 )
        return 0;
      if ( (this->mConditionResults.mActive[2] & 0x10000000) != 0 )
      {
        v76 = (this->mConditionResults.mValues[2] >> 28) & 1;
      }
      else
      {
        LOBYTE(v76) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[92].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 0x10000000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)v76 << 28) & 0x10000000
                                           | this->mConditionResults.mValues[2] & 0xEFFFFFFF;
      }
      if ( !(_BYTE)v76 )
      {
        if ( (this->mConditionResults.mActive[2] & 0x2000000) != 0 )
        {
          v77 = (this->mConditionResults.mValues[2] >> 25) & 1;
        }
        else
        {
          LOBYTE(v77) = StateConditionDirect_LastState(
                          (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[89].mpObject,
                          c,
                          p);
          this->mConditionResults.mActive[2] |= 0x2000000u;
          this->mConditionResults.mValues[2] = ((unsigned __int8)v77 << 25) & 0x2000000
                                             | this->mConditionResults.mValues[2] & 0xFDFFFFFF;
        }
        if ( !(_BYTE)v77 )
          return 0;
      }
      return 1;
    case 0x6Eu:
      if ( (this->mConditionResults.mActive[1] & 0x80000000) != 0 )
      {
        v78 = this->mConditionResults.mValues[1] >> 31;
      }
      else
      {
        v78 = StateConditionDirect_LastState(
                (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[63].mpObject,
                c,
                p);
        this->mConditionResults.mActive[1] |= 0x80000000;
        this->mConditionResults.mValues[1] = (v78 << 31) | this->mConditionResults.mValues[1] & 0x7FFFFFFF;
      }
      if ( (_BYTE)v78 )
        return 1;
      if ( (this->mConditionResults.mActive[1] & 0x10) != 0 )
      {
        v79 = (this->mConditionResults.mValues[1] >> 4) & 1;
      }
      else
      {
        LOBYTE(v79) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[36].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x10u;
        this->mConditionResults.mValues[1] = (16 * (unsigned __int8)v79) & 0x10
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFEF;
      }
      if ( (_BYTE)v79 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 1) != 0 )
      {
        v80 = this->mConditionResults.mValues[2] & 1;
      }
      else
      {
        LOBYTE(v80) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[64].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 1u;
        this->mConditionResults.mValues[2] = this->mConditionResults.mValues[2] & 0xFFFFFFFE | v80 & 1;
      }
      v81 = 0;
      if ( (_BYTE)v80 )
        return 1;
      return v81;
    case 0x6Fu:
      if ( (this->mConditionResults.mActive[1] & 0x80000000) != 0 )
      {
        v82 = this->mConditionResults.mValues[1] >> 31;
      }
      else
      {
        v82 = StateConditionDirect_LastState(
                (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[63].mpObject,
                c,
                p);
        this->mConditionResults.mActive[1] |= 0x80000000;
        this->mConditionResults.mValues[1] = (v82 << 31) | this->mConditionResults.mValues[1] & 0x7FFFFFFF;
      }
      if ( (_BYTE)v82 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 2) != 0 )
      {
        v83 = (this->mConditionResults.mValues[2] >> 1) & 1;
      }
      else
      {
        LOBYTE(v83) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[65].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 2u;
        this->mConditionResults.mValues[2] = (2 * (unsigned __int8)v83) & 2
                                           | this->mConditionResults.mValues[2] & 0xFFFFFFFD;
      }
      if ( (_BYTE)v83 )
        return 1;
      if ( (this->mConditionResults.mActive[1] & 0x20000) != 0 )
      {
        v84 = (this->mConditionResults.mValues[1] >> 17) & 1;
      }
      else
      {
        LOBYTE(v84) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[49].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x20000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v84 << 17) & 0x20000
                                           | this->mConditionResults.mValues[1] & 0xFFFDFFFF;
      }
      if ( (_BYTE)v84 )
        return 1;
      if ( (this->mConditionResults.mActive[1] & 0x10) != 0 )
      {
        v85 = (this->mConditionResults.mValues[1] >> 4) & 1;
      }
      else
      {
        LOBYTE(v85) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[36].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x10u;
        this->mConditionResults.mValues[1] = (16 * (unsigned __int8)v85) & 0x10
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFEF;
      }
      v86 = 0;
      if ( (_BYTE)v85 )
        return 1;
      return v86;
    case 0x70u:
      if ( (this->mConditionResults.mActive[2] & 1) != 0 )
      {
        v87 = this->mConditionResults.mValues[2] & 1;
      }
      else
      {
        LOBYTE(v87) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[64].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 1u;
        this->mConditionResults.mValues[2] = this->mConditionResults.mValues[2] & 0xFFFFFFFE | v87 & 1;
      }
      if ( (_BYTE)v87 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 2) != 0 )
      {
        v88 = (this->mConditionResults.mValues[2] >> 1) & 1;
      }
      else
      {
        LOBYTE(v88) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[65].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 2u;
        this->mConditionResults.mValues[2] = (2 * (unsigned __int8)v88) & 2
                                           | this->mConditionResults.mValues[2] & 0xFFFFFFFD;
      }
      if ( (_BYTE)v88 )
        return 1;
      if ( (this->mConditionResults.mActive[1] & 0x20000) != 0 )
      {
        v89 = (this->mConditionResults.mValues[1] >> 17) & 1;
      }
      else
      {
        LOBYTE(v89) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[49].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x20000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v89 << 17) & 0x20000
                                           | this->mConditionResults.mValues[1] & 0xFFFDFFFF;
      }
      if ( (_BYTE)v89 )
        return 1;
      if ( (this->mConditionResults.mActive[1] & 0x10) != 0 )
      {
        v90 = (this->mConditionResults.mValues[1] >> 4) & 1;
      }
      else
      {
        LOBYTE(v90) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[36].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x10u;
        this->mConditionResults.mValues[1] = (16 * (unsigned __int8)v90) & 0x10
                                           | this->mConditionResults.mValues[1] & 0xFFFFFFEF;
      }
      v91 = 0;
      if ( (_BYTE)v90 )
        return 1;
      return v91;
    case 0x71u:
      if ( (this->mConditionResults.mActive[1] & 0x80000000) != 0 )
      {
        v92 = this->mConditionResults.mValues[1] >> 31;
      }
      else
      {
        v92 = StateConditionDirect_LastState(
                (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[63].mpObject,
                c,
                p);
        this->mConditionResults.mActive[1] |= 0x80000000;
        this->mConditionResults.mValues[1] = (v92 << 31) | this->mConditionResults.mValues[1] & 0x7FFFFFFF;
      }
      if ( (_BYTE)v92 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 2) != 0 )
      {
        v93 = (this->mConditionResults.mValues[2] >> 1) & 1;
      }
      else
      {
        LOBYTE(v93) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[65].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 2u;
        this->mConditionResults.mValues[2] = (2 * (unsigned __int8)v93) & 2
                                           | this->mConditionResults.mValues[2] & 0xFFFFFFFD;
      }
      if ( (_BYTE)v93 )
        return 1;
      if ( (this->mConditionResults.mActive[1] & 0x20000) != 0 )
      {
        v94 = (this->mConditionResults.mValues[1] >> 17) & 1;
      }
      else
      {
        LOBYTE(v94) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[49].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[1] |= 0x20000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)v94 << 17) & 0x20000
                                           | this->mConditionResults.mValues[1] & 0xFFFDFFFF;
      }
      if ( (_BYTE)v94 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 1) != 0 )
      {
        v95 = this->mConditionResults.mValues[2] & 1;
      }
      else
      {
        LOBYTE(v95) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[64].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 1u;
        this->mConditionResults.mValues[2] = this->mConditionResults.mValues[2] & 0xFFFFFFFE | v95 & 1;
      }
      if ( (_BYTE)v95 )
        return 1;
      if ( (this->mConditionResults.mActive[2] & 0x800000) != 0 )
      {
        v96 = (this->mConditionResults.mValues[2] >> 23) & 1;
      }
      else
      {
        LOBYTE(v96) = StateConditionDirect_LastState(
                        (Sk8::Conditions::LastState *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[87].mpObject,
                        c,
                        p);
        this->mConditionResults.mActive[2] |= 0x800000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)v96 << 23) & 0x800000
                                           | this->mConditionResults.mValues[2] & 0xFF7FFFFF;
      }
      v97 = 0;
      if ( (_BYTE)v96 )
        return 1;
      return v97;
    default:
      return 1;
  }
}

```


### 0x82925fb8

```cpp
int __fastcall StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::EvaluateExpression<2>(
        StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian *this,
        unsigned __int16 expression,
        const StateGraph::Context *c,
        const StateGraph::State *p)
{
  int result; // r3

  switch ( expression )
  {
    case '&':
      if ( (this->mConditionResults.mActive[1] & 0x100) != 0 )
      {
        result = (this->mConditionResults.mValues[1] >> 8) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[40].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[1] |= 0x100u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)result << 8) & 0x100
                                           | this->mConditionResults.mValues[1] & 0xFFFFFEFF;
      }
      break;
    case '1':
      if ( (this->mConditionResults.mActive[1] & 0x10000) != 0 )
      {
        result = HIWORD(this->mConditionResults.mValues[1]) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[48].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[1] |= 0x10000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)result << 16) & 0x10000
                                           | this->mConditionResults.mValues[1] & 0xFFFEFFFF;
      }
      break;
    case '3':
      if ( (this->mConditionResults.mActive[1] & 0x200000) != 0 )
      {
        result = (this->mConditionResults.mValues[1] >> 21) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[53].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[1] |= 0x200000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)result << 21) & 0x200000
                                           | this->mConditionResults.mValues[1] & 0xFFDFFFFF;
      }
      break;
    case '5':
      if ( (this->mConditionResults.mActive[1] & 0x800000) != 0 )
      {
        result = (this->mConditionResults.mValues[1] >> 23) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[55].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[1] |= 0x800000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)result << 23) & 0x800000
                                           | this->mConditionResults.mValues[1] & 0xFF7FFFFF;
      }
      break;
    case '7':
      if ( (this->mConditionResults.mActive[1] & 0x2000000) != 0 )
      {
        result = (this->mConditionResults.mValues[1] >> 25) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[57].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[1] |= 0x2000000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)result << 25) & 0x2000000
                                           | this->mConditionResults.mValues[1] & 0xFDFFFFFF;
      }
      break;
    case ':':
      if ( (this->mConditionResults.mActive[1] & 0x10000000) != 0 )
      {
        result = (this->mConditionResults.mValues[1] >> 28) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[60].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[1] |= 0x10000000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)result << 28) & 0x10000000
                                           | this->mConditionResults.mValues[1] & 0xEFFFFFFF;
      }
      break;
    case '<':
      if ( (this->mConditionResults.mActive[1] & 0x40000000) != 0 )
      {
        result = (this->mConditionResults.mValues[1] >> 30) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[62].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[1] |= 0x40000000u;
        this->mConditionResults.mValues[1] = ((unsigned __int8)result << 30) & 0x40000000
                                           | this->mConditionResults.mValues[1] & 0xBFFFFFFF;
      }
      break;
    case 'H':
      if ( (this->mConditionResults.mActive[2] & 0x1000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 12) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[76].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x1000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 12) & 0x1000
                                           | this->mConditionResults.mValues[2] & 0xFFFFEFFF;
      }
      break;
    case 'I':
      if ( (this->mConditionResults.mActive[2] & 0x2000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 13) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[77].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x2000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 13) & 0x2000
                                           | this->mConditionResults.mValues[2] & 0xFFFFDFFF;
      }
      break;
    case 'J':
      if ( (this->mConditionResults.mActive[2] & 0x4000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 14) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[78].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x4000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 14) & 0x4000
                                           | this->mConditionResults.mValues[2] & 0xFFFFBFFF;
      }
      break;
    case 'K':
      if ( (this->mConditionResults.mActive[2] & 0x8000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 15) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[79].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x8000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 15) & 0x8000
                                           | this->mConditionResults.mValues[2] & 0xFFFF7FFF;
      }
      break;
    case 'L':
      if ( (this->mConditionResults.mActive[2] & 0x10000) != 0 )
      {
        result = HIWORD(this->mConditionResults.mValues[2]) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[80].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x10000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 16) & 0x10000
                                           | this->mConditionResults.mValues[2] & 0xFFFEFFFF;
      }
      break;
    case 'M':
      if ( (this->mConditionResults.mActive[2] & 0x20000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 17) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[81].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x20000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 17) & 0x20000
                                           | this->mConditionResults.mValues[2] & 0xFFFDFFFF;
      }
      break;
    case 'N':
      if ( (this->mConditionResults.mActive[2] & 0x40000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 18) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[82].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x40000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 18) & 0x40000
                                           | this->mConditionResults.mValues[2] & 0xFFFBFFFF;
      }
      break;
    case 'O':
      if ( (this->mConditionResults.mActive[2] & 0x80000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 19) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[83].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x80000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 19) & 0x80000
                                           | this->mConditionResults.mValues[2] & 0xFFF7FFFF;
      }
      break;
    case 'P':
      if ( (this->mConditionResults.mActive[2] & 0x200000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 21) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[85].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x200000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 21) & 0x200000
                                           | this->mConditionResults.mValues[2] & 0xFFDFFFFF;
      }
      break;
    case 'R':
      if ( (this->mConditionResults.mActive[2] & 0x1000000) != 0 )
      {
        result = HIBYTE(this->mConditionResults.mValues[2]) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[88].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x1000000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 24) & 0x1000000
                                           | this->mConditionResults.mValues[2] & 0xFEFFFFFF;
      }
      break;
    case 'S':
      if ( (this->mConditionResults.mActive[2] & 0x8000000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 27) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[91].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x8000000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 27) & 0x8000000
                                           | this->mConditionResults.mValues[2] & 0xF7FFFFFF;
      }
      break;
    case 'T':
      if ( (this->mConditionResults.mActive[2] & 0x20000000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 29) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[93].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x20000000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 29) & 0x20000000
                                           | this->mConditionResults.mValues[2] & 0xDFFFFFFF;
      }
      break;
    case 'U':
      if ( (this->mConditionResults.mActive[2] & 0x40000000) != 0 )
      {
        result = (this->mConditionResults.mValues[2] >> 30) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[94].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[2] |= 0x40000000u;
        this->mConditionResults.mValues[2] = ((unsigned __int8)result << 30) & 0x40000000
                                           | this->mConditionResults.mValues[2] & 0xBFFFFFFF;
      }
      break;
    case ']':
      if ( (this->mConditionResults.mActive[3] & 8) != 0 )
      {
        result = (this->mConditionResults.mValues[3] >> 3) & 1;
      }
      else
      {
        result = StateConditionDirect_HasIntent(
                   (Sk8::Conditions::HasIntent *)StateGraph::Gen::HCDelegate_MotionGraph_Pedestrian::sConditions[99].mpObject,
                   c,
                   p);
        this->mConditionResults.mActive[3] |= 8u;
        this->mConditionResults.mValues[3] = (8 * (unsigned __int8)result) & 8
                                           | this->mConditionResults.mValues[3] & 0xFFFFFFF7;
      }
      break;
    default:
      result = 1;
      break;
  }
  return result;
}

```

### 0x82cef180

```cpp
BOOL __fastcall StateGraph::HierarchicalController::Load(
        StateGraph::HierarchicalController *this,
        StateGraph::Cache *stateGraphCache,
        const char *filename,
        bool allowGenerated)
{
  StateGraph::AbstractHierarchicalController *mController; // r3
  StateGraph::AbstractHierarchicalController *v8; // r3
  StateGraph::AbstractHierarchicalController_vtbl *v9; // r30
  int v10; // r3
  const char *v11; // r3

  mController = this->mController;
  if ( mController )
    ((void (__fastcall *)(StateGraph::AbstractHierarchicalController *, int))mController->dtr_AbstractHierarchicalController)(
      mController,
      1);
  this->mController = nullptr;
  v8 = StateGraph::AbstractHierarchicalController::Create(
         stateGraphCache,
         filename,
         &this->mContext,
         DEBUG_ALL_CONDITIONS,
         DEBUG_FLUSH_MANUALLY,
         1,
         0,
         1);
  this->mController = v8;
  if ( v8 )
  {
    v9 = v8->__vftable;
    v10 = (int)v8->GetRootState(v8);
    v11 = v9->GetStateName(this->mController, (const StateGraph::State *)v10);
    this->mRootStateNameHash = GetHashValue32(v11);
  }
  return (_cntlzw((unsigned int)this->mController) & 0x20) == 0;
}

```

### 0x826f3470

```cpp
Sk8::Conditions::InBranchWindow *__fastcall Sk8::Conditions::InBranchWindow::InBranchWindow(
        Sk8::Conditions::InBranchWindow *this,
        StateGraph::ExpressionOperator *parent,
        IAttributeMap *attr)
{
  const char *v5; // r3
  char *BranchAnimName; // r11
  int v7; // r10

  StateGraph::Condition::Condition(this, parent, attr);
  this->__vftable = (Sk8::Conditions::InBranchWindow_vtbl *)&Sk8::Conditions::InBranchWindow::`vftable';
  v5 = attr->GetString_2(attr, "BranchAnimInfo");
  BranchAnimName = this->BranchAnimName;
  do
  {
    v7 = *(unsigned __int8 *)v5++;
    *BranchAnimName++ = v7;
  }
  while ( v7 );
  return this;
}

```

### 0x82caa638

```cpp
Sk8::Conditions::LastState *__fastcall Sk8::Conditions::LastState::LastState(
        Sk8::Conditions::LastState *this,
        StateGraph::ExpressionOperator *parent,
        IAttributeMap *attr)
{
  const char *v5; // r4
  const char *v6; // r11

  StateGraph::Condition::Condition(this, parent, attr);
  this->__vftable = (Sk8::Conditions::LastState_vtbl *)&Sk8::Conditions::LastState::`vftable';
  this->mStateName.mpBegin = (char *)&eastl::gEmptyString;
  this->mStateName.mpEnd = (char *)&eastl::gEmptyString;
  this->mStateName.mpCapacity = (char *)&eastl::gEmptyString + 1;
  if ( !attr->HasAttribute(attr, "state") )
  {
    xcore::FailAssert(
      nullptr,
      "d:\\checkouts\\sk82\\main\\packages\\sk8\\skater\\dev\\source\\state/CommonConditions.cpp",
      479);
    DebugBreak();
  }
  v5 = attr->GetString_2(attr, "state");
  v6 = v5;
  while ( *v6++ )
    ;
  eastl::basic_string<char,eastl::allocator>::assign(&this->mStateName, v5, v6 - 1);
  return this;
}

```

### 0x826fd618

```cpp
int __fastcall Sk8::Behaviours::BranchWindowEnabled::ActivationFunc(
        Sk8::Behaviours::BranchWindowEnabled *this,
        const StateGraph::Context *context)
{
  UCOM::IUnknown *v3; // r31
  int v4; // r3
  unsigned __int8 v5; // r11

  v3 = UCOM::Object::_IList::Find(&context->mEntity->_mInterfaces, Anim::IAnimAttributeStore::_IName);
  if ( !v3 || !UCOM::Object::_IList::Find(&context->mEntity->_mInterfaces, Andale::IAnimatable::_IName) )
    return 0;
  v4 = ((int (__fastcall *)(UCOM::IUnknown *))v3->__vftable[2].dtr_IUnknown)(v3);
  if ( !v4 )
    return 0;
  v5 = 1;
  if ( !*(_DWORD *)(v4 + 16) )
    return 0;
  return v5;
}

```

### 0x826fd5b8

```cpp
BOOL __fastcall Sk8::Behaviours::InTurnBranchWindow::ActivationFunc(
        Sk8::Behaviours::InTurnBranchWindow *this,
        const StateGraph::Context *context)
{
  UCOM::IUnknown *v2; // r3

  v2 = UCOM::Object::_IList::Find(&context->mEntity->_mInterfaces, Anim::IAnimAttributeStore::_IName);
  return v2 && ((bool (__fastcall *)(UCOM::IUnknown *))v2->__vftable[4].dtr_IUnknown)(v2);
}

```

### 0x827127b0

```cpp
int __fastcall Sk8::Conditions::InBranchWindow::ActivationFunc(
        Sk8::Conditions::InBranchWindow *this,
        const StateGraph::Context *context)
{
  int v4; // r28
  Anim::IAnimInfo *v5; // r30
  UCOM::IUnknown *v6; // r31
  unsigned __int64 v7; // r6
  const Anim::tAnimBranchInfo *v8; // r30
  int v9; // r3
  double v10; // fp1
  double v11; // fp31
  int v13; // r3
  double mValue; // fp0

  v4 = 0;
  v5 = (Anim::IAnimInfo *)UCOM::Object::_IList::Find(&context->mEntity->_mInterfaces, Anim::IAnimInfo::_IName);
  if ( v5 )
  {
    v6 = UCOM::Object::_IList::Find(&context->mEntity->_mInterfaces, Andale::IAnimatable::_IName);
    if ( v6 )
    {
      Attrib::StringToKey(this->BranchAnimName);
      v8 = Anim::IAnimInfo::GetAnimInfo<Anim::tAnimBranchInfo>(v5, v7);
      if ( !((unsigned __int8 (__fastcall *)(UCOM::IUnknown *))v6->__vftable[15].dtr_IUnknown)(v6) )
      {
        v9 = ((int (__fastcall *)(UCOM::IUnknown *))v6->__vftable[10].dtr_IUnknown)(v6);
        v10 = ((double (__fastcall *)(int))*(_DWORD *)(*(_DWORD *)v9 + 24))(v9);
        v11 = v10;
        if ( v8->Type )
        {
          if ( v8->Type == eBracnhType_BranchPoint
            && v10 >= v8->BranchTime
            && v10 <= (float)(Sk8::Conditions::kBranchWindow.mTarget->mValue + v8->BranchTime) )
          {
            return 1;
          }
        }
        else
        {
          v13 = ((int (__fastcall *)(UCOM::IUnknown *))v6->__vftable[14].dtr_IUnknown)(v6);
          mValue = Sk8::Conditions::kBranchWindow.mTarget->mValue;
          if ( v11 <= mValue || *(float *)(v13 + 8) <= mValue || *(_BYTE *)(v13 + 1) )
            return 1;
        }
      }
    }
  }
  return v4;
}

```

### 0x82e6e668

```cpp
const Anim::tBranchWindow *__fastcall Sk8::Pedestrian::GetCurrentBranchWindow(Sk8::Pedestrian *this)
{
  int v2; // r3
  int v3; // r30
  int v4; // r3
  double v5; // fp1
  int v6; // r9
  int v7; // r11
  float *i; // r10
  double v9; // fp0

  v2 = ((int (__fastcall *)(Sk8::Pedestrian *))this->GetPosition)(this);
  v3 = v2;
  if ( !v2 )
    return nullptr;
  if ( *(int *)(v2 + 16) <= 0 )
    return nullptr;
  v4 = (*((int (__fastcall **)(Attrib::Class **))&this->mChaseParameters.mCollection[75].mClass[2].mPrivates + 1))(&this->mChaseParameters.mCollection[75].mClass);
  (*(void (__fastcall **)(int))(*(_DWORD *)v4 + 24))(v4);
  v6 = *(_DWORD *)(v3 + 16);
  v7 = 0;
  if ( v6 <= 0 )
    return nullptr;
  for ( i = (float *)(v3 + 20); ; i += 3 )
  {
    v9 = *i;
    if ( v9 >= 0.0 && v5 >= v9 && v5 <= i[1] )
      break;
    if ( ++v7 >= v6 )
      return nullptr;
  }
  return (const Anim::tBranchWindow *)(12 * v7 + v3 + 20);
}

```

### 0x82e6e750

```cpp
BOOL __fastcall Sk8::Pedestrian::InTurnBranchWindow(Sk8::Pedestrian *this)
{
  if ( !((int (__fastcall *)(Sk8::Pedestrian *))this->GetPosition)(this) )
    return 0;
  if ( *(_BYTE *)(((int (__fastcall *)(Attrib::Class **))this->mChaseParameters.mCollection[75].mClass[3].mPrivates)(&this->mChaseParameters.mCollection[75].mClass)
                + 1) )
    return 1;
  return (_cntlzw(((int (__fastcall *)(Sk8::Pedestrian *))this->GetFacing)(this)) & 0x20) == 0;
}

```

### 0x82701740

```cpp
BOOL __fastcall StateConditionDirect_HasPlugin(
        StateGraph::Condition *c,
        const StateGraph::Context *context,
        const StateGraph::State *parent)
{
  UCOM::IUnknown *v3; // r3

  c->mParent = &parent->StateGraph::Base;
  v3 = UCOM::Object::_IList::Find(&context->mEntity->_mInterfaces, Sk8::LivingWorld::IBehaviourSink::_IName);
  return v3 && (_cntlzw(((int (__fastcall *)(UCOM::IUnknown *))v3->__vftable[4].dtr_IUnknown)(v3)) & 0x20) == 0;
}

```

### 0x826f3570

```cpp
BOOL __fastcall Sk8::Conditions::PedestrianJumperIsLanding::ActivationFunc(
        Sk8::Conditions::PedestrianJumperIsLanding *this,
        const StateGraph::Context *context)
{
  Sk8::Entity *mEntity; // r11
  BOOL result; // r3

  mEntity = context->mEntity;
  result = 0;
  if ( mEntity )
    return ((unsigned __int8 (__fastcall *)(UCOM::Object::_IPair **))mEntity[5]._mInterfaces.mpCapacity[17].ref)(&mEntity[5]._mInterfaces.mpCapacity) == 0;
  return result;
}

```

### 0x826f35c0

```cpp
BOOL __fastcall Sk8::Conditions::PedestrianJumperInDownwardMotion::ActivationFunc(
        Sk8::Conditions::PedestrianJumperInDownwardMotion *this,
        const StateGraph::Context *context)
{
  Sk8::Entity *mEntity; // r11
  BOOL result; // r3

  mEntity = context->mEntity;
  result = 0;
  if ( mEntity )
    return ((bool (__fastcall *)(UCOM::Object::_IPair **))mEntity[5]._mInterfaces.mpCapacity[18].handle)(&mEntity[5]._mInterfaces.mpCapacity);
  return result;
}

```

### 0x82e702f0

```cpp
int __fastcall Sk8::Pedestrian::IsJumping(Sk8::Pedestrian *this)
{
  return *(unsigned __int8 *)(this->mChaseParameters.mMsgPort + 100) >> 7;
}

```

### 0x82e70300

```cpp
int __fastcall Sk8::Pedestrian::IsInDownwardJumpMotion(Sk8::Pedestrian *this)
{
  return (*(unsigned __int8 *)(this->mChaseParameters.mMsgPort + 100) >> 6) & 1;
}

```

## September 30: retained frame output and real SimLOD producer

`pedestrian_motion_frame::PedStateOut` retains the native output fields. PDB
places it at Pedestrian+0xa80, with flags at+20. Constructor82e97a38..80 writes
state72, length3, topic18, greet-character100 and clears high4 flag bits.
AnimUpdate writes right/left foot bits4/5 before querying TurnBranchWindow or
updating channels, including when a later operation returns an error.

`pedestrian_motion_pose::SimLodPose` queues the existing retained Andale tree
through `eval_reference`, shared TreeCommands, real VBR fetching, shared matrix
jobs and the existing scheduler. Native sources are EvalCurrAnimForSimLOD82e6d170,
SetUpSimLODAnimation82e6ea28, UpdateSimLODGlobal82e6ab08 and
UpdateTargetTransform82e7fc88..ca0. The9 lookup names come from82e8d650. Global
bones are produced only after actual extraction/Sync, by composing each SimLOD
matrix with the final target transform. They are published after
SetTargetTransform and before hand-prop target updates. Missing poses do not
become identity or bind poses. The native null-current-animation branch uses
the retained authored bind node through the real evaluator.

Important callback correction: GetPedestrianInvBindPoseBuffer826f4420 returns
AnimMan+16, whose PDB field is `InvBindPoseMatId`, a registered FIXED matrix ID.
It is not an inverse-bind pointer. AnimMan::Init82718d08 loads separate Pres/Sim
skeletons, creates shared TrajPedestrianACS for40 characters, registers identity
matrices827191f0..19370 and resolves the9 lookup indices against the Sim skeleton
(max20 bones). `register_sim_lod_identity` is its startup registration operation;
`SimLodPose::new(core, id)` retains that same command owner's registered slot.
The shared fixed registry copies matrices like RegisterFixedMatrix82a28968.
SetMatrixByIdViaCallback82a2c708 / SetDataById82a24890 preserve fixed operands;
they do not introduce an external SetData producer.

Shared command integration extends existing owners: FaceCommands.fixed_matrices,
MatrixProgram::new_fixed, TreeCommands::new_stream, VBR fetch-code selection,
and the existing SQT dispatcher. BeginCommands82a229c0 preserves dynamic
allocation; EndCommands82a289e0 leaves mNum at its high-water mark. Bounded
pedestrian streams therefore use disjoint SQT slots across one command set.
The new shared `actor_animation_pedestrian_batch_jobs` child partitions real
fetches, blend operations and pose operations by retained pool limits. Its
producer dependencies feed the existing scheduler and existing Sync fence.
Sources: FetchEngine::EnQueueCmd82a30458 (worst-next-fetch capacity check),
SQTProcessEngine::EnQueueCmd82a2b0d0, PoseEngine::PoseWithMatMult82a2add0,
PoseEngine::EnQueueCmd82a31520 (14jobs, maxPoseOps/6+1 per job),
Schedule82a32800. Job::DependsOn829ae838 and dependency bucket Add829ae3e8
verify the existing owner's10-entry dependency and6-entry dependent extensions.

Required existing-owner integration: register the identity matrix once during
AnimMan startup and retain its ID; initialize animation.sim_lod_pose after the
actual playback/ACS owners exist; BeginCommandSet once for selected pedestrians,
queue each SimLOD tree, submit/sync the shared ACS, then install
sim_lod_bone_global from the final-target-transform phase. The completed
`queue_simulation_pose` selects the native simple LOSim2/non-Marquee branch or
the FullSkelMarquee branch described below. Shared dispatch supports mixed
external-SQT/fetch streams and the original bounded reuse boundaries.
No compilation, syntax check, test execution or game runtime was performed.
Existing test struct literals were only amended for new required fields.

Startup integration follow-through: `PedestrianPlaybackStartup::initialize` now
registers and retains `simulation_fixed_matrix_id` immediately after its real
SIM bind-pose registration. `PedestrianConstruction::initialize_animation`
accepts that ID and initializes the retained animation's SimLodPose, preserving
the constructed owner if pose initialization fails. The animation child
`pedestrian_animation_pose.rs` owns queue_sim_lod_pose and the completed
update_sim_lod_global publication. The native final-target update phase itself
is not yet present in the existing pedestrian entity; its scheduler must call
this publication at82e7fc94 rather than pretending the current transform is the
completed target. Services' existing construction-info reference remains the
same owner; no new services/roster was introduced.

FullSkelMarquee82e6d238 now queues SetSQTViaCallback3 (AnimMan+8 actual
SetIdentitySQTs), AddSQT(true,true), UniformScale1, and dual SQT/matrix extraction.
GetPedestrianSQTBuffer827023f0 callback2 returns AnimMan+12 ExtractSQTs, a shared
scratch allocation, not a per-pedestrian output buffer. Startup retains one
`FullSkeletonScratch` with the native PRES-sized identity and extraction
allocations. SIM commands access their SIM prefix. Init827192c4..310 initializes
actual identity SQTs, including translation W0; no evaluated pose is invented.
UniformScale82a27038 retains the quantized coefficient819 for scale1 and the
worker's decode constant0x3aa00500. Raw82e6d2f0..300 confirms true trajectory
scaling and root0 (the float argument consumes r4's parameter slot).

Cross-stream allocation inherits the existing ACS cursor and global seven-reuse
count. GetNextLoc82a24c28 and DispatchReuseBufferCmd82a2fc18 divide retained
engine batches into segments. Earlier producers and extractors complete before
the shared reuse barrier; later engine jobs depend on it. Empty copy lists use
the native null barrier. Nonempty lists reuse the original ordered-copy worker.
Pool bounds and the same scheduler/Sync owner remain authoritative.

NBlend82a266c0..267a0 copies coefficients and encodes popped input IDs before
requesting a new dynamic slot. Its input list is not rewritten after reuse.
The removed host `ReusePendingOperands` guard had rejected this legal native
path. The retained output-table remap now updates only host graph-root
consumption; native encoded IDs stay unchanged. Graph links are attached after
allocation at82a26918 onward. All live roots at the boundary remain included in
reuse ordering, even when the command operand stack is already empty.

Manual source review covered the new command variants, shared FaceCommands and
MatrixProgram literals, retained scratch ranges, scheduler dependency extension
types, module visibility and existing error conversions. No compiler, parser,
tests or runtime were invoked; session scheduling/publication still needs the
existing owner's integration and validation.

## Live selected-session continuation

The selected tick now calls the animation manager after World.Update1's grab
dispatch and before the existing combined solver/locomotion call, matching
SimUpdate1 8289f77c (Manager.Update3 82726e80). After World.Update3 it calls the
ACS fence at SimUpdate2 8289fa34, ahead of UpdatePostTransforms8289fc50. The
host's pre-existing combined physics phase remains; no native asynchronous
timing equivalence is claimed. No build, parser, tests or runtime were run.

SelectedRecipeFrame carries the actual terrain trigger owner. Actor callback
membership comes from SimulationEntityRoster.snapshot, retaining its same Arc
Actors and vector order. The existing shared FrameServices/with_entity borrows
the sole EntityMan40 payload, physical world, Manager RNG and actual auxiliary
managers. It does not reacquire the world mutex or create another roster.

Manager.UpdateAnimManager82725538 queries camera XZ radius30. AnimMan.Update
82723cd0 first captures UpdateAnimationData for ALL native hash-order entities,
then AnimUpdate82e95da0 for all, then one shared TrajPedestrianACS command set.
Simple/full extraction reuses the already implemented SQT dispatch, scheduler
and Sync. UpdateBlendPoseBuffers826f4470 resets the same tick token pool used by
skater hand/face evaluation and visits the nearby query results. Renderable
entities with a current animation regenerate retained BlendPoseBuffer through
the existing Animatable/ChannelBlend token evaluator; others clear validity.
cPedestrianAnim82e903f4 selects pool0; recursive evaluation uses pool1. The
native ActiveMotionGraphPool is retained on PedestrianPlaybackStartup,
constructor82721ef4 starts0 and successful Update rotates (pool-1)&1.

Direct vtable checks resolve a potentially misleading visibility assumption:
Pedestrian LWBasic vtable823a4e70 and SecurityGuard823a5410 slotbc BOTH bind
LWBasicEntity.UpdateRenderStates82d1ced8, which only calls SetIsRenderable(true).
ComponentMan82e82ce0 visits every native map entry. This exact producer now runs
after AnimMan evaluation. There is no camera/frustum gate in this method.

UpdateTargetTransform82e7f660 has a LWBasic receiver (complete+32). Its first
slotb4 is IsRenderable82d1ceb8, not the decompiler's GetStateOut. PDB tAnimData
+5f90 is SimLODPoseBuffer[20], so the first matrix is the actual completed root
delta used after Sync. The live target producer composes that delta with the
previous target, calls the existing SpatialAdjust.update with actual collision
and NavRig, reorients via shared LookAt, applies the active intent's angular
rotation, consumes pending DMO displacement, sets the core target and publishes
the nine actual global bones. Colliding+0xa and force0 select ground offset;
sliding uses entity+1a44bit40, !stateCA8bit2, !stateCAAbit40. The same retained
adjust_world_collisions field already represents bit40, so no duplicate flag was
added. Source ReorientToGlobalUp82e4e5c8 and RotateToTarget82e56640 were read
directly; shared two-refinement normalization, ACos, SinCos and axis matrix
operators are reused. Exceptional VMX host numeric limits remain shared.

UpdateHandPropTargetTransform82e7b2b0 now consumes actual global bone3, the held
DMO's authored grip and hand-prop offset layout. Reorient82e7af78 composes authored
Euler/translation with inverse grip then hand basis. (Corrected 2026-10-06: it
stores the composition as is; an emulation of the raw VMX128 code matches
hand x offset x inverse grip to 5 decimals with a +1 determinant, while the
X/Z exchange first ported here mirrored every held prop.)
Prediction updates the retained last hand position only when the registered
cvar enables it. DynamicObject.SetHandPropTargetTransform82d2aec0 raw1080 with
IDynamicObject receiver+8c asserts complete110c transform type1 before the target
write. LWDynamicObjectModel.InitializeHotpoints82d38ec0 now retains the last
embedded type0 matrix through the same processed arena. PDB tHotPoint is affine
at0 and type at60; ctor82d336cc initializes the pointer null. This does not add
the separate nonzero-hotpoint waypoint allocation path.

Motion callbacks now read actual authored animation metadata, shared transition
override, registered clips and Manager RNG. Source Bernoulli/discrete hand-prop
selection uses the separate original global math RNG. Speech82e94ca8 writes
retained PedStateOut.state. ThrowAway82e9b528 schedules the native release timer;
RemoveHandProp82e70118 changes actual collision groups through the existing
Island pools, releases the DMO physics mode/gravity/cull timer, removes the
first monitoring membership and clears the retained held pointer. These are
not immediate fake throws or audio noops. HandPropUsage zero initialization is
backed by MemStore.Init82a6dd98 and FreeBlock82a6df20 clearing allocation bytes.
Full SpawnAndSetHandProp and all cull/chase teardown remain separate work.

Navigation task audit for the execution integrator: Task_PreNavigation82e732e0
uses complete Pedestrian virtualslot40; PostNavigation82e73528 uses44. Both
slots in Pedestrian Entity vtable823a4f58 and Guard823a54f8 bind folded empty
825187c0. No additional state mutation or callback backend is required there.

The caller integration is source work, not evidence of live NPC readiness.
Parent work still owns AI/current/preframe execution. Presentation consumption,
cull lifecycle and pending hand-prop creation
must be completed and builds/playtests remain outstanding.

Post-transform continuation: Manager.UpdateTransforms826f52a8 orders Pedestrian,
Vehicle, then DMO. Both Pedestrian/Guard LWBasic slot8c bind82e5b358, so the live
all-roster pass now calls the existing CommonEntityCore.update_transform,
retaining its two reciprocal refinements, full velocity lanes and source old
matrix assertion. The vehicle helper's existing final transform loop was moved
into vehicle_update_transforms so its earlier drive/line/skitch work remains
ahead of the three-category publication pass. The sole caller was updated.
DMO target/cull prefix now precedes that post-transform work. Native
DynamicObject.UpdateTargetTransform82d2e7c8 skips physics target replacement for
type1 hand props; the shared existing DMO frame now preserves that branch,
instead of overwriting the completed hand target. Pedestrian collision model
drives reuse the retained FillWorld/FillSkater method before current publication.
Head tracking and animation post-adjust producers are still outstanding.

### Full presentation command replay and first live GPU consumer

Source continuation September30: UpdateAnimationPoses82809610 loads the SIM
command stream into the existing FullPedestrianACS; SyncAnimationPoses8280a640
fences it. The typed host replay now retains the accepted stream before SIM
submission consumes it, preserves encoded fetch times and dynamic/reuse slots,
and rebuilds fetch descriptors against the actual PRES hierarchy. It does not
reevaluate the animation graph or advance clip clocks. Callback3 binds the
retained per-entity post-adjust SQT allocation. cPresPedestrian constructor
828122b0 initializes that allocation to identity; procedural updates remain
outstanding below. Existing CharacterPalette provides Hips remapping,
authored model inverse-bind matrices and completed matrix/world packing
(InitSkinning828063b8, Update828121d8). The existing selected tick calls this
producer after UpdatePostTransforms.

SelectedVisualSnapshot now publishes the same completed palette Arc, and
LiveSkatingView retains pedestrian GPU objects in its shared scene object
list. Existing material sorting and character shadow passes see those objects.
The ordinary defaultlivingworld effect has its own GPU family, reusing the
shared four-weight vertex decoder/palette uploader. Its fragment uses original
diffuse gamma squaring, green<0.001225 tint gate, actual two entity colours,
material m_params[0].y, the SAME WorldPainter fog, g_charattributes[2].z and
source tonemap/sqrt. It does not relabel an unrelated character effect.

Shader evidence was extracted read-only using tools/extract_big.py and
updb_source.py from the original render.big to
%TEMP%/skate2-pedestrian-shaders-20260930. Final source hashes:
defaultlivingworld.6788fdd8a297 and livingworld_stamp.a2f2e54a9729.
The ordinary manifest default pass uses FRONT culling; stencil attachment
behavior follows the existing character pipeline limitation.

Still incomplete: livingworld_stamp's normal/SH/lighting shader, pedestrian
LOD/opacity transitions, full hands/face composition, HeadTracking.Update and
PostAdjust.Update producers. The latter must publish source quantized six-bone
adjustments; retaining constructor identity is not their implementation.
Unsupported material effects report their actual name. No claim of complete
NPC rendering/playability or source numerical parity from syntax inspection.
29 scoped Rust files passed rustfmt parsing (emit stdout, skip_children=true);
no files were formatted by that check. No compiler/build/tests/GPU/runtime run.
