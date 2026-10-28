# Retained Actor hand/face stream publication

Scope: the actual Actor animation owner after body AddTime/Eval, through
`DispatchHandsFaceAnim825f87b8`, into the same hand/face SQT allocations consumed
by `WriteSkater827d8ab8`. This is not a named-animation presentation probe.
Confidence: high source/control ownership; host numeric policy remains the same
explicit boundary as existing ACS blend/RAW decoding. No live output or full
presentation claim until the command owner, extraction jobs and manager schedule
are mounted and original-data tests pass.

## Token producer

`Animatable::EvalBlendPose82d6bb68` selects the actual current tree; a null tree
selects the bind Pose node. `Pose::EvalBlendPose82587b98` asserts, so null animation
cannot silently produce an empty/identity hand stream. AddSQT825884e8 evaluates
only its motion child. Mirror82c92878 evaluates its child then toggles the active
part-name bank. ChannelBlend82d79bf0 evaluates the base first, reads its live
coefficient afterwards, and evaluates/merges the secondary tree only for ordered
positive coefficient (raw82d79c28 NOTGT exits).

Clip82d77e00 reads its actual retained DataHeader's aligned payload. Payload+0x34
bits13..17 give stream count; +0x30 is the table offset. Each table entry is a
payload-relative stream offset. Stream+0 contains the six packed part-name words,
+0x18 contains six mirror-name words, +0x30 signed key count, and +0x34 holds
four-byte keys: big-endian i16 normalized time and u16 pose ID. Nonpositive key
count skips the key loop. The first key with NOT ordered `(end < current_time)`
wins: raw82d77f00 is `419800dc` BLT skip, so unordered selects a key too. Encoded
time converts exactly to f32, multiplies constant bits39000400, then clip length.
First key emits its pose; subsequent key emits previous/current poses, coefficient
`1 - (end-time)/(end-start)`, then pair blend. Each unwritten one of all three
lanes gets pose0, including lanes not registered as named streams.

PhaseBlend82d793b8 is deliberately separate from ordinary body Eval smoothing.
Raw93ec..9414 uses scalar FSEL at fixed endpoints bits3e99999a/3f333333 and
multiplier40200001. The remapped value stays in f31. Optional first/last endpoint
smoothing changes f0. Raw9488 compares smoothed f0 with1; 9490 NOTLT selects only
child2. Otherwise child1 evaluates, 94ac rereads the stored raw blend, and 94b8
NOTGT skips child2. If child2 runs, Blend94f4 receives **f31** via94f0, not smoothed
f0. Root independently raw-reviewed this distinction on 2026-09-12 and agreed.

BlendPoseBuffer has two pools of three 8000-word lanes. Constructor825f7738 and
Init825efb58 retain each current pool top as the buffer's start and initialize
relative tops to-1. Main pool is0, raw825f8880. Dispatch resets all six global tops
before/after use, retaining allocations. CreateStream82d75338 deduplicates the
current name bank and permits three names; fourth distinct name asserts.

PushPose82d6b2c8 shifts pose by23 without masking its incoming word. PushCoef
82d6b3b0 reserves relative top and zeroes the token before fctidz; global top is
published afterwards. Multiplier bits497ffff0 is1048575; low32 integer bits OR
with40000000, without masking conversion to20bits. Decode uses lower20bits times
bits35800008. Native capacity assertion is global top<7999. Checked host fctidz
admission reuses the existing converter and retains the pre-conversion writes.
PushBlend82d6b4d8 preserves the unusual backward duplicate scan starting at
relative-top minus keycount; this is not a generic pose equality optimization.
Blend82d78038 re-encodes each source token into corresponding destination stream,
then adds outer coefficient/pair blend only when that stream already existed.

## Consumer and actual storage

SetUpBlendPoseStream825f8398 first resolves PlaybackData through the actual
Simulation workspace manager (player0), then BeginCommands846c. Token type0:
nonnull clip SetFrame85e4 and actual Clip Eval8600, otherwise SetSQTById860c.
Type1 appends decoded coefficient to the ten-element temporary list. Every other
type reaches the blend branch: one coefficient with stack depth>1 requires native
dimension2 then Blend8528; >1 coefficients must equal dimension then NBlend8568;
remaining cases assert. Supported current tree producers emit only pair blends;
n-way execution must remain an explicit boundary until a reached producer needs it.

Clamping is tested **only after blend**, with strict pose-count>clamp and remaining
stack depth<2. It does not simply truncate tokens or cap pose count. With
addbindpose=true, the actual playback rig_TPose is fetched and AddSQT(true,true)
occurs if any pose was emitted. With addbindpose=false and no selected stream or
pose, the actual IdentityPose is fetched. ExtractSQT87a0 and EndCommands87a8 follow.

Dispatch face BeginCommandSet8858 occurs if the actual face manager exists.
Flags bits23/24/25 are cleared at88a4; IsLocalPlayer88b4 is sampled before LOD
selection. LOD<6 and (local or LOD<3) admits token evaluation. Face is admitted for
(local and LOD<4) or LOD0, then its init flag is set. Hands use source clamp10000
except localLOD5/nonlocalLOD2 read the live HandsCommandsClamp target. Left then
right setup occurs. Any uninitialized hand then gets actual fallback setup using
the Skateboard_Trucks part name, still with its own hand playback and rig_TPose.
Face EndCommandSet8b2c follows token-pool resets.

Registered storage amendment replaces each hand's MaybeUninit vector and face's
Option vector with a retained `OwnedAnimationSqtBuffer` Arc allocation. Unknown
contents remain unknown; emission does not fabricate values. The producing job
(or native fixed-copy scheduling) writes that allocation, advances its host
generation, and Sync permits
reading. Constructor ownership tests inspect actual unknown storage. This does
not itself execute a hand/face command or constitute presentation publication.

## Concrete shared ACS prerequisites

The original body admission remains VBR-only. The registered selected RAW
constructor now constructs the existing hand/face buffered owner beside its
retained fixed RAW frame arrays. Native hand setup is capacities[200,1,1,1,1], face[100,8,1,1,1]; both
averages[25,1,1,1,1], codecRAW, fetch_mode0, CMDBUFFER, nine characters, no trajectory.
The RAW NewACS path824d63e4..6430 allocates entry then requires the same job manager,
names it AnimFetch. This must occur at startup/face construction, before actual
manager registry publication, not lazily in Dispatch.

Fresh source sizing checks: FetchSys82d6f3b4 sets numChars*maxFetchNodes;
InitFetchSysForACS82d6d9d8 returns parts*maxFetches records,64 bytes each,512 per RAW
job (VBR256). FetchEngine82a31de8 uses ceil(records/perjob)+9 jobs. Data82a3237c
computes trunc(((numChars*max(avgFetch+avgReplace,2))+(numChars-1))/numChars), and
Data3028c multiplies by characters: selected hand/face18 dynamic SQT slots.
SQTEngine82a31c40 uses18 jobs and maxoperations/10+1 work units when operations>164;
maxoperations=(SQTcap-Fetchcap)*characters. SetDataEngine82a31fe0 uses12jobs,
maxops=2*replacecap*characters (18 here), with >104 quarter-size rule unreached.

SetSQTById82a2c6a0 preserves fixed operands. It must not consume a dynamic fetch
slot or replace actual retained fixed frames. Blend raw82a25d18..4c selects the
first dynamic operand, otherwise second dynamic, otherwise GetNextLoc when both
are fixed. Existing body operations remain all-dynamic. ExtractData82a24400 emits
a dependent command referencing actual output storage. It copies48*externalBones,
so padding/internal extra bones cannot be published as real pose bones.

The registered implementation extends the existing TreeCommands/ACS
job execution with fixed operand metadata, original mapped RAW pose decoding,
ExtractSQT output work and hand/face command-set scheduling. Existing formats
sample_raw_pose_sqt already decodes source RAW pose channels; no VBR or authored
bind replacement is allowed for a missing dynamic result. Root owns the actual
manager schedule and final presentation stream producer.

## Validation status

Root now independently reviewed raw DispatchHandsFace825f87b8..8b34: face
command-set gate, pool reset, flags clear, local/LOD gates, face then left/right
streams, fallback trucks channels, final pool reset and face EndCommandSet.
Raw Sync82a30088 waits this ACS's root and reinitializes this ACS only; the new
selected backend retirement preserves peer command sets instead of clearing
the entire host queue. Constructor output ownership remains distinct from Sync.

The actual face corpus now retains and passes its original MountedSource/job
manager into source-bearing NewACS construction. A selected hands/face corpus
is mounted after both actual Motion Play + phase2/phase3 updates, sharing the
same retained token pool and original output allocations. Constructor-only
body fallback cannot be used here: native EvalBlendPose's Pose node asserts.
Generation advancement and original-data results are pending the next gate;
Clippy41 completed with cleanup findings, which are being fixed.

Six token tests added: authored key interpolation/20bit token, unordered first-key
selection, backward duplicate scan, numeric-failure partial writes, separate phase
gate/coefficient, and pool reset retaining allocation. Four unregistered token
consumer call-trace tests cover strict clamp, unreduced stack, type3 branch and
dimension assertion. Root owns Cargo and original-data integration gates. No
claim yet that these producers have initialized the live Actor's hands or face.


## Extract scheduling correction, 2026-09-12

Fresh raw review of Schedule82a32800 rejects a separate trailing SetData job for
ExtractSQT. At32904..32918 the parent command is compared with0x58000000; its
actual destination is loaded into r28 at32938. Dispatch subtracts4 from opcode
at3299c before indexing table329c0: opcode11 uses table index7->32e20 and queues
nothing; opcode12 uses index8->32a38 (SetData); opcode13/14 use indices9/10->329f0
(Fetch); other SQT arithmetic uses32e00. The Fetch call32a0c and SQT call32e18
receive the parent's destination as r7 (raw329fc/32e0c `mr r7,r28`).

Fixed SetSQTById followed by ExtractSQT takes the fixed branch32a40, reads the
same fixed SQT pointer table+18 at32ce4 and external bone count+10 at32ce8,
computes48*count at32cec..cf8, and calls XMemCpy at32d18 while scheduling. No
fetch/blend/copy work job is created for that fixed-only result. Its ACS null
root still governs the command-set Sync boundary.

The corrected selected dispatcher therefore publishes its retained destination
from the final FetchRaw or SqtBlend work item, or synchronously during scheduling
for a fixed result. It creates no SetData job for extraction. The initialized
SetData engine remains retained because native RAW construction creates it;
this selected graph simply does not consume it. Fixed scheduling drops the outer
FaceCommands lock before resolving the same fixed-frame owner, avoiding a host
recursive mutex lock without changing source ownership. Results are still
unreadable to presentation until the actual command-set Sync completes.

Two focused tests cover fixed scheduling and blend-owned extraction timing,
including unknown-before-producer and inaccessible-before-Sync storage. Three
additional tests cover fixed dynamic-slot allocation/foreign ACS rejection and
same-allocation generation advancement. All are registered for the next root
gate; no agent Cargo was run. Nonnull RAW Clip and n-way token blending remain
explicit unsupported branches, and multiple actors sharing one hand ACS still
require source cross-stream dynamic slot allocation before admission.

Parent validation follow-up: Clippy43 passes. Original30 executes both selected
Motion updates with the same Actor and completes hand/face dispatch and all three
independent ACS synchronizations each time. Each original output allocation is
nonempty and advances its generation by one on the second pass. The overall
corpus remains16/17 because the later GroundSkateboard call stops at an unrelated
obsolete known-W proxy-target assertion. Workspace33 has eight failures outside
this animation path; its new hand/face tests pass. These results cover the
selected loaded Motion path, not arbitrary trick/grab graphs or rendered poses.
