# CharacterGesture first animation tick dependency map

2026-09-12. Read-only source audit of Update `82cc3198`; no new execution claimed. Fresh pseudocode and raw instructions are retained externally in `%TEMP%/skate2-character-gesture-update.json`; virtual names were independently resolved from PDB in `%TEMP%/skate2-character-gesture-vtables.json`.

## Unconditional work

1. Resolve and identity-check the SAME Actor's IPhysical, IAnimatable, IChannelAnimatable, ISkaterAnim, ISkaterMotionGraph, IMotionGraph and IActor. All seven must exist even when there is no gesture input.
2. Read live `kEnableDebugGestures`. Its initializer `82f85df0` establishes false in section Animation_Gestures. False calls ISkaterAnim::GetGestureSet at virtual+38 (`82cc3a74`), actual getter `82cb77b8`, returning up/down/left/right indices. The existing retained player.gestures already supplies these fields after actual initialization. True uses four live cvar indices instead.
3. Build both-hand/left/right animation choices for all four directions from original packed FastString36 tables. Most of the enormous decompiler output is these fixed-size copies, not complex branching.
4. Call IMotionGraph::GetMotionGraphIntents virtual+10 (`82cc3edc`). Check presence in exact order Down, Left, Right, Up; later present directions replace earlier selections. These are presence tests, not tests that the stored float is nonzero. Each present direction also reads its corresponding gesture cvar even on the non-debug path: preserve this source quirk.
5. Call IChannelAnimatable::HasChannel virtual+18 for GestureLeftSkaterAnim (`82cc4240`), GestureRightSkaterAnim (`82cc4264`), GestureBothSkaterAnim (`82cc4288`). Both channel implies both hands occupied. Actual HasChannel `82cc0a30` is dictionary membership, with no active-channel status filter. `InitializedAnimationChannels.dictionary` is an existing real retained store initialized empty; it can implement this lookup without a fabricated false result.
6. Independently query actual PhysOut through IPhysical+28 twice: Filtered+0 and Ground+0x125 (IsHippyJumping). If filtered state is neither Offboard6 nor OffboardAir7, or IsHippyJumping is true, call ISkaterMotionGraph::GetGrabStateReadOnly virtual+2c (`82cc4418`). Its two busy-hand integers already exist in ActorSkaterGraphState.grab_busy. They constrain free-hand channels. Otherwise query OffBoard+0x3f (IsHoldingSkateboard), which can suppress right/both gestures.

## Conditional effects

- Offboard holding-board branch ends actual right and both channels through IChannelAnimatable::EndChannelPrematurely+2c if the right hand was gesturing (`82cc4388`, `82cc43a0`). It prevents starting right/both gestures even when no channel exists.
- Only when a directional intent is present, choose a free channel in priority Both, Left, Right and call NewChannel+0c (`82cc44ec`). This requires the real free channel/tree pools and original tChannelParams. There is no need to execute this branch when the real input map proves no gesture event.
- If an old left/right gesture remains or a new channel was started, SetIsGesturing+0xc4 (`82cc4538`) writes source enum and true. If only an old channel remains, enum35 is used. Actual setter `825fe178` is those two writes; retained ActorSkaterGraphState has both fields. It does not clear false on the no-gesture path.
- If an old gesture remains or any gesture input exists, query PhysOut_State+0x41 IsOffboard. Call IAnimatable::AddSettableAtt+0x48 with GstrDistToCogName and either1 (offboard) or actual PhysOut_Animation+0x48 CentreOfMassToDeck. Raw optional arguments are false and -1 (`82cc45a0`/`82cc45ec`). The animation attribute pool mutation, not just a diagnostic flag, is required when this branch is taken.

## Actual input producer

`ActorConstruction::try_generate_action_intents` already clears/fills the SAME Action map through its retained listener and GameInputPlayer, including derived input, pattern suffix and real Juice collaborator. Directional gestures come from rising D-pad bits only when neither bumper is held (`action_graph_input_fill.rs`), not a synthetic sample.

CharacterGesture consumes the separate Motion map. The missing bridge is actual generated Action execution: root Action state0 contains11 mandatory behaviours, including four CreateMGIntentFromAGIntent occurrences1..4 copying GestureDown/Left/Right/Up. Their real lifecycle is required: Update `82cd62c8`, CreateIntent `82cd52d0`, RemoveIntent `82cc1e58`, plus CreateInstance/Begin. Do not alias the Action and Motion maps or directly insert gesture values into Motion to bypass that bridge. Generated controller hierarchy selection and source lifecycle still must precede the first Motion root update.

## Bounded assignments

1. Input/Action owner: close actual generated Action root CreateMGIntentFromAGIntent lifecycle and map mutation using retained input owner; preserve the rest of Action selection/ordering. Add five actual gesture cvar registrations: bool initializer82f85df0/ref830a385c; Up82f88d20/ref830a393c/default34, Down82f88d70/ref830a3f38/default33, Left82f88dc0/ref830a38c0/default32, Right82f88e10/ref830a3604/default31. Cvar enum strings and callback-null are present in fresh IDA. No assumption that the live value equals its default.
2. Animation owner: add real GetGestureSet and channel HasChannel lookups to initialized payload; recover the three original packed name tables and channel constants. Add owned NewChannel/EndChannel and AddSettableAtt continuations for branches that real input/channel state activates. Keep unavailable branches explicit until implemented.
3. Motion owner/root: expose same Actor grab state and SetIsGesturing two writes, and read the real retained PhysOut fields listed above. Run CharacterGesture through actual generated Motion lifecycle and context identity. Root CreateInstance82bb3da8 returns native NULL for this class; a non-null placeholder would be wrong.

A first tick whose actual Motion intent map and actual channel dictionary are empty still performs every unconditional read above, then reaches the source no-write return. That limited branch is source-valid only after actual input generation and Action execution establish those maps; constructor zeros by themselves do not establish a delivered runtime sample.

## Owned runtime continuation, 2026-09-12

Author implementation: `generated_motion_character_gesture.rs`, registered as an
Actor-construction child; `actor_animation_gesture_writes.rs` as an animation-
construction child. Neither module substitutes recognizer input for the separate
Motion intent map. Context entity and controller issuance must match this Actor
and its generated Motion wrapper. Animation registrations must be the actual
retained occurrence. Physical cached publication must retain the same allocation
as the registered IPhysical occurrence; merely completing the selected assembly
constructor does not establish cached Actor+200.

The callback now reads real retained player gesture indices, or the five live
cvar getters, before visiting Down/Left/Right/Up presence in the Motion map.
Each present intent reads its directional enum even when debug gestures are off;
zero-valued intents count as present. Initializers 82f84678/82f84d20/82f853c8
contain exactly 35 Both/Left/Right animation names; the new names module retains
all 105 in original order. Both indices2,8,10 intentionally use LEFT names.
Gesture channels come from initializers82f84570/88/a0: GestureBoth/Left/Right.

Raw 82cc4428/4458 reads GrabState+4 (mFsHandBusy) for the left hand;
82cc447c reads +0 (mBsHandBusy) for the right hand. Both checks Fs before Bs.
The two initial PhysOut calls and later conditional calls remain separate reads.
No existing gesture channel and no intent produces no writes; existing channels
can set the actual graph gesture to Invalid35 and is_gesturing=true. The callback
does not clear is_gesturing on an idle tick.

### Animation write helpers

`ChannelAnimatable::EndChannelPrematurely82d7d5d0` performs dictionary lookup;
a missing entry is a source success. Existing Channel+44 is node.blendOutTime
(PDB Channel.mNode+8, ChannelBlend.mBlendOutTime+36). If it equals either signed
zero, Channel+60 / ChannelBlend flags+52 ORs 0x28000000. Other fields are untouched.
An unwritten field in a host channel remains an explicit error, never zero.
The gesture caller ends Right before Both, then clears only its local hadRight.

`Animatable::AddSettableAtt82d7f5f0` checks value before scanning fixed150 storage.
Raw assertion instruction82d7f628=1bbef065 selects VA62 and VB62 (IDA prints
misleading VA94), after lvlx/splat of the passed f32. All-true CR6 means ordered
self-equality, hence NaN asserts while both infinities are accepted. Root
independently corroborated the raw operand decoding. Name is five packed words;
sequence is native r7, normalization is r6. First matching name+sequence changes
only value, sequence and normalized bit. Append emits begin/end=-1, flags6,
type0 scalar, given sequence and normalized bit. Existing AndaleRuntimeAttribute
cannot represent native sequence identity; AnimationSettableAttribute therefore
retains packed name and sequence alongside that format value.

GstrDistToCogName initializer82f82060 assigns `GstrDistToCog`. Gesture callback
passes State.IsOffboard?1:Animation.CentreOfMassToDeck, normalizefalse, sequence-1.
Focused helper tests cover same name with distinct sequence IDs, replacement in
original slot, negative-zero payload, NaN pre-write failure and infinity success.
They use the actual Animatable base constructor. Author ran rustfmt only;
Cargo and original-corpus integration belong to root.

### Remaining mandatory channel construction

`NewChannel82d7eb98` returns2 for existing name,3 for no free channels,4 when
actual PlaybackData.GetAnimTree returns NULL, and0 on success. It selects the
last free channel and last free tree, stores the channel tree pointer BEFORE
GetAnimTree82d7ec74, then calls SetSpeed, actual virtual animation-tree adjustment,
optional mirror construction, and ChannelBlend.Init before popping free lists
and publishing the dictionary and priority links. Gesture passes priority0,
cyclefalse,mirrorfalse,speed1,blendIn0.3,blendInToFirstFramefalse,blendOut0.3,
blendOutFromLastFramefalse,useChannelAttributesfalse. PDB tChannelParams offsets
are0,4,5,8,12,16,20,24 respectively. Unknown padding is not represented as zero.

The subsequent native tree producer described below replaces NewChannelPending.
Original NewChannel status remains ignored by the caller: a real nonzero status
still proceeds to SetIsGesturing with the selected live enum. Full generated
Motion scheduling, actual full HiLOD publication, and mutable tree evaluation
remain necessary before claiming a usable first animation tick or playback.


## Native gesture tree producer, 2026-09-12

Owned new modules: actor_animation_tree.rs (native typed pool/nodes),
actor_animation_tree_database.rs (actual PlaybackData/DataBase methods),
actor_animation_channel_runtime.rs (same SkaterAnim NewChannel/AddBindPose).
Root approved narrow registration and owner-field additions in existing files.
All retained source references are actual database/pose/hierarchy Arc owners.
No decoded/evaluated pose is substituted for a Clip, PhaseBlend or channel tree.

Read-only original corpus extraction from anim.bh/big using existing source
archive header layout and Andale record layout found:

- OffBoard.abin: zero GSTR records.
- OnBoard.abin:319 GSTR records,214 Clip and105 PhaseBlend.
- Every105 entry across the Both/Left/Right gesture name tables resolves to a
  PhaseBlend record, each with exactly two Clip children. Table aliases remain
  aliases. The mutable tree must therefore construct PhaseBlend as well as Clip.
- Original channel pool limits from Initialize82cb9920:1024bytes,20elements;
  12pools and6channels. These actual allocated capacities feed the typed pools.

External record inventory remains %TEMP%/skate2-gesture-records.json, and extracted
source assets remain external to the repository. An ignored original corpus test
uses actual startup-owned parsed records to independently construct all105 GSTR
PhaseBlend trees, check two actual retained Clip database owners, and call the
same extra SetSpeed(1) used by NewChannel. No author Cargo execution.

PoolStack Push references: Clip828087a8 allocates64; PhaseBlend82d78310 allocates128
(PDB class124 rounded16); Pose82b644c0 allocates16; AddSQT8258bf60 allocates32
(PDB24); Mirror82c96568 allocates32(PDB24). Counts must remain below maxElements;
new location must be strictly less than maxSize, so exact-end allocation asserts.
A missing Pose header assertion occurs after its pool push and is retained.
Pop removes only the latest node, preserving the peculiar PhaseBlend recursive
missing-child prefix. Per-pool handles compare by actual issuance, not Arc<()> 
payload equality or equal node indices. Padding/unused native bytes are not
represented as meaningful initialized fields.

Clip constructor establishes speed/length/time/oldTime/loops/frames0,fps30,
baseSpeed1,loopingfalse,trimmedfalse,debug counters0. Clip::Init82807260 retains
header then frameCount,looping flag0x10000000,fps,loops0,baseSpeed,oldTime; calls
SetSpeed then SetTime then computes length again and validates time range.
FrameToTime82805ed0 computes frame/((baseSpeed*fps)*speed), separate f32 products.
Clip::SetSpeed82807108 retains elapsed animation phase using previous speed then
rescales current/oldtime by reciprocal new speed*baseSpeed, and recomputes length.
Raw bgt guards82807140/828072b8 reject NaN speeds as well as nonpositive values;
Hex-Rays if(speed<=0) is misleading for unordered. SetTime828071f0 raw bge means
not-LT and accepts NaN time. SetTime overwrites current/oldtime and resets loops0.

PhaseBlend ctor82d75810 sets blend0,lastAttributeFLT_MAX,time0,length0,cull0,
selected childrenNULL,lastAttributeNormalizedtrue and empty fixed5children.
Database constructor copies attribute name and time0, recurses in original child
order and invokes AddChild82d7af60. Append starts child value0; first child becomes
child1 then supplies length; second becomes child2. SetSpeed82d75a80 calls each
child in stored order; normalized phase denominator uses FSEL(-length,1,length),
so nonpositive length selects1 while NaN selectsNaN. New length is source FMADD
82d75b78: child2.length*blend + rounded(child1.length*(1-blend)). SetTime82d75970
asserts nonzero length, sets child1 then queries/sets child2, then own time.

SkaterAnim::AddBindPose82c96b78 always pushes actual bind-pose Pose and AddSQT
(animation,pose,motion=animation). Flag80000000 inserts board-backwards Pose and
AddSQT, then board-backwards-IK Pose and AddSQT in source operand order. Flag
40000000 adds Mirror with trajectoryUse2. Flag00400000 adds one-shot Mirror with
trajectoryUse1 if00200000 else2, then clears00400000;00200000 is cleared always.
Known source flag bits are required only at their actual reads. Cleared outputs
become known. Mirrors retain the actual hierarchy database, not copied matrices.

ChannelBlend::Init82d6d500 receives exact Gesture params described above and
establishes semantic flags. For blendOutLength0.3 only blendOutEarly is initially
true (upper semantic bits0x08000000), all othersfalse. Lower25 padding is not
claimed as initialized. The existing raw field mirrors upper semantic bits;
new consumers use named runtime flags. EndChannel OR0x28000000 changes both raw
semantic mirror and runtime blending_out/blend_out_early.

NewChannel preserves lookup->free channel/tree selection->channel tree pointer
write->GetAnimTree->SetSpeed->AddBindPose->base query->ChannelBlend.Init->free list
pops->dictionary publication->maxChannels assertion->priority/link writes.
The root channel or actual NULL mBaseAnim is retained as channel base identity.
GetBaseAnim82592b48 is a direct mBaseAnim getter. Constructed body animation
without a corresponding typed runtime owner remains an explicit boundary;
no bind-pose substitution is made. Priority comparisons are signed native values;
new gesture priority0 inserts relative to actual existing parent/base links.
The callback ignores returned native statuses as original and still marks the
selected live gesture enum after an actual completed NewChannel call.

Validation authored: rustfmt passed; two pool tests and original105gesture tree
corpus test added. Root owns compiler/clippy/test execution and independent source
review. This establishes constructors and mutable timing setters only; native
Clip/PhaseBlend evaluation, channel blending, state graph scheduling and final
HiLOD publication are not claimed complete by these modules.


Root review follow-up: tree records preserve exact packed identity directly from
retained original bytes using each actual record.data_range (PhaseBlend attr+16,
children+36+24*i), rather than re-encoding a decoded name. No formats schema edit
was needed. NewChannel passes its actual Animatable construction-map entries to
PlaybackData; the current Clip/PhaseBlend-only loader explicitly rejects a nonempty
map with UnsupportedConstructionMap, rather than ignoring future replacements.
The actual Animatable Initialize resets this map's len0. New errors implement
Display consuming payloads for runtime diagnostics. External fresh raw/code/hash
packet for14functions: %TEMP%/skate2-native-gesture-tree-source.json.
