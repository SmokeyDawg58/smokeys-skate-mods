# Selected loaded-face Actor constructor

2026-09-11. Confidence: **high** for the selected original face constructor
and integer RAW channel ownership; not a per-tick numerical parity claim.
Implementation author: animation_path. Root remains independent reviewer and
sole integrator. This document does not itself certify the final Cargo gate.

## Producer and source ownership

The default character is not a null-face case. Original recipe loading at
`82d0a220` calls `CharacterFaceManager::LoadFaceData` for category 0/3 and waits
for completion. Actor's selected marquee cached-layout text0 resolves to
`tutorial_male`. `SetupFacePose` only skips the manager for an actual null pointer;
nonnull empty text is not that branch.

| Source symbol | VA | Relevant behavior |
| --- | --- | --- |
| `CharacterFaceManager::LoadFaceData` | `82d04cf0` | unsigned-byte hash, retained request, first load |
| `CharacterFaceManager::FaceHolder::OnComplete` | `82d008d0` | buffer transfer, loaded publication, SIM event |
| `CharacterFaceManager::HasFaceData` | `82d01868` | hash-only holder/loaded lookup |
| `CharacterFaceManager::GetFaceData` | `82d01920` | separate allocation and full buffer copy |
| `CharacterFaceManager::RecieveFaceLoadFinishedMsg` | `82d01a18` | request-order presentation enqueue then erase |
| `Sk8::Skater::SkaterAnim::SetupFacePose` | `82cbf700` | database/playback/ACS and all fixed frames |
| `Andale::DataBase::DataBase` | `82d74128` | insert in actual first-free manager slot |
| `Andale::DataBaseManager::FindFreeSlot` | `82d6bc38` | first-free scan, source assertion when full |
| `Andale::DataBaseManager::DataBaseManager` | `82d79260` | 32 database slots, 31 part-name slots |
| `Andale::DataBase::SetDBContent` | `82d7f7e8` | content ownership, sorted clip/pose GUIDs |
| `Andale::DataBaseManager::UpdateDatabase` | `82d76628` | append missing global part names |
| `Andale::DataBase::UpdatePartNames` | `82d6e728` | hierarchy, pose, then clip part IDs |
| `Anim::AnimCommandSystemManager::NewACS` | `824d62c0` | named ACS/fetch registration |
| `Andale::PlaybackData::PlaybackData` | `82d810b8` | name/manager and playback registration |
| `Andale::PlaybackDataManager::AddPlayBackData` | `82d80fc0` | duplicate name/index assertion |
| `Andale::PoseBufferExt<AnimCmdSys::SQTExt>::Init` | `8258be00` | hierarchy then uninitialized 48*N allocation |
| `AnimCmdSys::AnimCommandSystem::RegisterFixedSQT` | `82a288e8` | copy 48*N external bones, increment count/max |

Representative exact decorated symbol:
`?SetupFacePose@SkaterAnim@Skater@Sk8@@AAAXPBDPAVDataBaseManager@Andale@@PAVPlaybackDataManager@5@PAVAnimCommandSystemManager@Anim@@@Z`.
The recipe manager is owned by the sk8recipes library; playback/database belong
to Andale, ACS to Anim/AnimCmdSys, and Actor/SkaterAnim to Sk8.

The face hash starts at `ffffffff`, adds unsigned bytes after wrapping multiply33,
and stops at NUL. Hash collisions intentionally share a holder. A loaded repeated
request increments references and returns true without another callback request.
Pending duplicates retain separate callbacks but start only one IO operation.
Success requires completed status plus nonempty transferred bytes. Loaded is
published before the SIM event; presentation callbacks remain separate delivery.
The native snprintf path has capacity256 including terminator and then converts
slashes. The host load ticket exposes that issued path to the asset service.

`GetFaceData` copies every byte into a new allocation. The load holder retains its
own buffer. The selected `E_DELETE` database owns that copied allocation; native
header/part mutations are represented by retained parsed structures, not by a
guest memory subsystem. DB slot assignment is never hardcoded: it comes from the
retained manager's current 32-slot occupancy. GUIDs OR in slot low5 and sorted
clip/pose ordinal bits5..15. Names append in hierarchy order up to31; only after
that pass do hierarchy IDs, pose part headers/offsets and clip headers/offsets
change. ACS registration replaces an equal key; playback duplicates are checked
errors corresponding to the native assertion. Prior allocations/registrations
remain retained on later checked failure.

## Actual external asset

Original root remains outside the repository:
`<game folder>`.

Viv4 `data/big/anim.bh` entry `data\anim\faces\tutorial_male.abin` has hash
`362b3974`, `anim.big` offset16,816,128, stored size16,336, unpacked field0.
Direct read-only extraction independently reverified SHA256:
`50bf9f3d7346787c8abbc9455fa1202ff9362a4f7de3d2fdbc45affb0b74d1d7`.

Records: hierarchy offset30 size410 (31 bones, three parts); clip440 size37f0;
pose3c30 size2c0; identity3ef0 sized0. The clip is60fps with30 frames, codec4452,
flags00780300. Every part uses RAW selectors `[0,1]` and odd masks1/3/5 zero.

| Part | Bones | Scale mask0 | Quaternion mask2 | Translation mask4 | Stride | Data bytes |
| --- | ---: | --- | --- | --- | ---: | ---: |
| 0 | 3 | 0 | 1 | 5 | 28 | 840 |
| 1 | 1 | 0 | 1 | 1 | 16 | 480 |
| 2 | 27 | 06004007 | 07547a8f | 06fdffbf | 424 | 12720 |

Independent Python traversal of all30 frames consumes every stride exactly:
180 scale samples,540 quaternion samples,810 translation samples. All2,970
authored scale/translation float lanes are finite. This validates layout, not
the scalar quaternion helper's numerical equivalence to Xenon.

## RAW extraction and frame scheduling

`Andale::Vector_ExtractPackedN` (`82d6f838`) loads selector bytes at header+27/+28,
stride u16 at+24, and six u32 masks at+0..20. `82d6fd24` multiplies stride by the
selected integer frame. In the supported selector family each bone consumes
selected scale float3 (12 bytes), packed quaternion u32 (4 bytes), and translation
float3 (12 bytes), in that order. Raw quaternion expansion reuses the established
`QuatDecompress` (`825e9e68`) helper, with its existing high-confidence numerical
limits.

Missing channels are not filled from a guessed bind pose. At `82d6f86c/884`,
VMX builds scale `[1,1,1,1]`; `82d6f874` builds zero; `82d6f8dc` builds quaternion
`[0,0,0,1]`. The first extract (`sqtOffset==0`) initializes the full destination
before selected channels replace it. The ordinary bit29-clear clip chooses
translation `[0,0,0,1]`. Delta channels differ and explicitly refuse. The complete
clip API also refuses gaps/overlaps, incomplete coverage, mismatched part IDs,
stride mismatches and unsupported selectors.

Face setup evaluates each requested frame through the following selected route:

1. `Clip::SetFrame` (`825ecdb8`) divides frame by N-1 and multiplies length.
2. `Clip::FrameToTime` (`82805ed0`) forms length from `(N-1)/(baseSpeed*fps*speed)`;
   setup's speed is1. `Clip::Eval` (`82d6ef78`) rescales and bounds current/old time.
3. ACSParams mode at offset52 is1, loaded by setup's store at `82cbf9bc`.
   Buffered `Fetch` (`82a2f740`) uses `FrameTimePrecision` at821fb794,
   bits44b60b33. It floors the f32 product. The inverse constant is3a34002d.
4. Equal old/current time gives a zero delta after the source epsilon comparison.
   The selected implementation checks that condition; it does not assume it for
   arbitrary times. `FetchEngine::EnQueueCmd` (`82a30458`) decodes the command.
5. Unblended `FetchSys::BatchFetch` (`82d737d0`) bounds fps*time and truncates after
   adding the setup's0.5 offset. The selected frame is then decoded by RAW.
6. BeginCommandSet/BeginCommands, Fetch/Extract, EndCommands/EndCommandSet, Sync,
   RegisterFixedSQT follow the original per-frame order. Each fixed buffer owns
   a copy. The temporary extraction buffer is released; the separate local face
   pose remains allocated with unknown elements.

All30 original requested frames select themselves after that quantization.
`SetupFacePose` ORs00800000 into SkaterAnim+612c at `82cbfb50` only after the loop.
The constructor returns before Actor publishes ISkaterAnim at+1fc (`825feefc`).
Face names and playback `%s_%03d` names use the actual native36-character encoding,
including signed decimal player formatting and NUL/truncation behavior.

## Runtime route, tests, and limits

Files: `andale_raw.rs`/tests; `character_face_manager.rs`/tests;
`actor_face_playback.rs`/tests; constructor attachment in
`actor_animation_construction.rs`. Root owns `actor_animation_mount.rs` and the
selected mounted-archive/Actor corpus integration. Root capture order is actual
command, playback, database managers after animation allocation and before its
first interface registration. The SAME world/Entity allocation is retained.
`try_finish_loaded_face` requires a real loaded holder, retains the copy/database/
playback/commands on the animation, and only finishes after all fixed frames.

Focused tests cover second-stride selection, untouched identity channels,
unsupported selectors/truncation, pending duplicates and callback order, distinct
data copies, foreign/failed loads, quantized frame selection, and actual manager
ownership. The ignored original face test exercises loaded holder -> copied DB
-> GUID/part remapping -> named playback/ACS ->30 fixed31-bone frames. Root's
separate corpus mounts the archive index and attaches that path to the Actor.
The author ran no Cargo; final gate results belong in root status after completion.

Scope remains selected host-success construction. This is not a general ACS job
scheduler/body command implementation, animation tick, rendering skinning path,
complete recipe loader, or full Simulation startup. Other RAW selectors, delta
channels and interpolation refuse; one-frame clips, reused initialized database
buffers and out-of-domain command times refuse. Duplicate database names now
follow the original last-record replacement described below.
Prior body database occupancy must be supplied through the same manager schedule;
an isolated slot0 test is not proof of native complete-startup occupancy.
No per-tick Xenon trace or new bit-exact quaternion claim is made.

## Body and hand startup producer, 2026-09-11

`Sk8::Simulation::InitAnimation` (`?InitAnimation@Simulation@Sk8@@QAAXXZ`,
828a5640) precedes Actor construction. `Andale::Init`82d74228 creates the shared
attributes PoolStack owner, records the current thread ID, then resizes to
0x19000 bytes and20 element entries. The startup owner retains allocated unknown
storage; it does not initialize a pose there. The corpus thread ID1 is an explicit
controlled fixture; the application source must supply its actual identity.

The kUseVBR dynamic initializer82f73178 creates category `Animation`, name
`kUseVBR`, description `Use VBR Codec`, defaulttrue, callbacknull, reference
8305904c. InitAnimation reads this live target twice: first to choose
`data\anim/OffBoard.abin` then `data\anim/OnBoard.abin` (or RAW suffixes), and
again after those database loads to select the body fetch codec. The supplied
native cvar owner is read separately at both call sites; the original-data
startup corpus uses the retained original-INI cvars rather than a fixedtrue
provider.

The database registration order is OffBoard, OnBoard, left hand, right hand.
Nine body playback/command entries use name `Skater` and player indices0..8;
each retains the OnBoard hierarchy and the ordered database list
`[OffBoard, OnBoard]`. The ACS name initializers82f7f6c8/6e0/6f8 establish names
`Skater`, `SkaterLeftHand`, `SkaterRightHand`. Body ACS parameters are capacities
[200,100,0,0,2], averages[30,20,0,0,2], maxNBlend4, fixedSQT0/fixedmatrix0,
mode1/fetchmode1/category0, characters1, truncation0.5, trajectorytrue, and the
second sampled codec. This retains setup inputs and the selected host command
owners; general NewACS job execution is still outside the implemented slice.

`DataBase::SetDBContent`82d7f7e8 walks records in file order, writing
vector_map::operator[]. Duplicate names replace previous pointers. Only surviving
clip/pose map entries receive GUID ordinals (sorted packed names) and part-name
updates. `PlaybackData::GetPose`82d7cab0 then searches databases front to back,
returning the first database hit. The former face-only duplicate refusal has
been replaced with this source behavior; overwritten serialized records remain
retained and unmodified.

Independent external archive audit (no asset files copied into the repository):

| Entry | hash | offset | stored bytes | SHA256 |
|---|---|---:|---:|---|
| OffBoard.abin | 07138dc7 | 6912 | 4597256 | 8a576a8bb3baa7653b0875b1f957617e8d55d863a8f52f879ee59212aab47838 |
| OnBoard.abin | 2faae1c9 | 4604224 | 8490280 | a4cb025ad119fc93a5b1e68b8e7cbe4ea17c663d8c1ead02bf5d55839a338586 |
| SkaterLeftHand.abin | 87b1b534 | 13094528 | 14288 | 68c09765f20b575d3d2959c8c26c5736e96b6fec0816e89f3130166de5302c81 |
| SkaterRightHand.abin | ddf43867 | 13108864 | 5504 | 8727232842c440ffa84c83bbe50e64e2becfe8cfa083025c37a68dce049d553f |

Each unpacked field is0. OffBoard contains28 copies each of rig_tpose,
board_backwards, board_backwards_ik; their actual winners are records4614d0
(size6b0),461b80(size530),4620b0(size530). OnBoard has24 each and winners8172f0,
8179a0,817ed0. All these winning pose records use VBR00524256, one frame,
eight parts with bone counts[1,5,3,8,8,1,6,4], and pureconstant S/Q/T blocks
(components3/4/3, animatedentrycount0). The existing constant VBR decoder can
therefore materialize the actual36-bone OffBoard rig; no presentation-rig
substitution is necessary. The startup pose reference retains that winning
record and its database owner.

Both hands always load their named files independently of kUseVBR. Each uses
its named ACS/playback at index0, one database, trajectoryfalse, RAW codec0,
truncation0.5, characters9 (raw left NewACS call:828a5c44), capacities
[200,1,1,1,1], averages[25,1,1,1,1], fixedSQT count from `Poses`, fixedmatrix0,
mode1/fetchmode0/category0/maxNBlend4. `Poses` has61 frames at60fps and17 bones.
Left record270 size33d0 has masksS0,Q1ffff,T1ddb6,stride212; right record270
size1180 has masksS0,Q1ffff,T0,stride68. Both use the supported RAW[0,1]
selectors. Startup completes each left fixed frame before loading the right:
BeginCommandSet, BeginCommands, SetFrame, Eval, ExtractSQT, EndCommands,
EndCommandSet, Sync, RegisterFixedSQT; temporary extraction storage is released
after each completed hand set.

The implemented readiness field is deliberately
`databases_and_fixed_hands_ready`, not full Simulation boot completion.
`simulation_animation_startup.rs` retains the common database/playback/command
bundle and shared attributes owner; its source-path IO trait returns owned actual
bytes with checked failures. Failed IO retains its already registered database;
failed fixed extraction retains preceding registered playback and fixed frames.
Root mounts this producer before the selected Actor face, which then takes slot4
in the same manager, rather than a separate empty slot0 manager.

The next consumer is `SkaterAnim::Initialize`82cb9920 (Actor call825ff008):
GetPose rig_Tpose, board_backwards, board_backwards_ik; choose job/local attributes
pool; create AnimBlendSpace job entry; Animatable::Init82d7c6e0 with treebytes1cc00,
four trees, three transition blends,2048maxclips; InitChannelAnimatable82d7cc68
with1024bytes/sixchannels; store actual CAC stance/gestures/proskater. Actor then
looks up each hand playback at index0 and calls InitLeftHand82c96a48 and
InitRightHand82c96ae0, allocating unknown48*N local SQT storage and clearing
respective02000000/01000000 flags. Animatable::SetPlaybackData82d6e450 allocates
unknown64*N local/global matrices, not seeded identity SQT values. Actual body
channel/tree/job ownership and this consumer remain pending; an assets-ready
startup must not be used to claim Initialize or scheduled animation is complete.

Confidence is exact for the stated source ordering, names, maps, flags and
asset metadata; high for existing decoded constant VBR numerics and RAW packed
quaternions. Focused new tests cover retained first-IO/thread-failure prefixes;
the new original-data corpus checks actual winning pose offsets,36-bone finite
decoded rig, all61 frames of both hands,11 ordered command/playback entries,
and next database slot4. The implementation author did not run Cargo; gate
results remain the integrator's responsibility.

## First Initialize and dormant job owners

The next implementation now occupies the constructor's child module
`actor_animation_initialization.rs`, with `animation_job_owners.rs` supplying
typed dormant allocations. Root independently reviews and integrates these
drafts; their final gates are not asserted here.

`kUseJobsForSkaterAnimUpdate` initializer82f56ae8/ref830480b0 creates category
Animation, name kUseJobsForSkaterAnimUpdate, description `Use Jobs for Skater
Anim Update`, defaulttrue, callbacknull. The actual linked cvar owner exposes a
fresh read at the Initialize call. The default branch allocates the Actor's
0x26400byte/20entry attributes pool; the false branch retains the SAME startup
shared attributes pool. Both branches then allocate the BlendSpace entry point
and require an actual job-manager owner. Disabling this cvar does not eliminate
that source dependency.

`AnimCommandSystemManager` ctor824d56b8 constructs an `AnimJobScheduler` whose
only native state is its vtable; this real scheduler now belongs to the retained
command manager. `Sk8::Job::cJobManager` ctor826e33f8 sets its scheduler pointer
null. A constructed job manager may therefore legitimately have no backend;
no backend readiness or submitted job is implied by its existence.

`EA::Jobs::EntryPoint` ctor829ad7a8 initializes name[0]0, supplied local code,
priority128, affinity63, environmentLOCAL0, kernelSwap0, codeRecycle1,
breakOnEntryfalse and allowSleepfalse. SetName829acb90 stores at most15 bytes
plus the C terminator. The BlendSpace entry code/name are selected source
identities, not a dummy function invocation.

`AnimJobScheduler::CreateJob`824d4cf0 allocates native848 bytes at alignment16,
constructs Job829ae658, then SetData829accf0(null,0), SetCode(null), SetName(null).
Job::Clear829ad7e0 sets params[4]0, seenfalse, hasJobDependencyfalse; initializes
the entry point defaults; empties dependency/dependent/event extension lists.
The Job ctor's raw829ae6a8 `std r31,0x40` uses zero r31 and proves the64-bit
submissionID is zero despite decompiler uncertainty. Backend, index and padding
are zero too. Dependency bucket ctor829ad3a8 initializes10 null jobs/zero
handles, but leaves their trigger words unknown. Dependent inline pointers are
unknown with count0. Each of two10-entry event buckets has typeNONE0 with
unwritten union/decrementer payload; startEvent likewise has typeNONE0. These
unknowns remain explicitly unreadable in the native Rust owner. No job has been
submitted and no scheduled worker implementation is claimed.

Initialize now retains the source partial effects: three ordered pose queries;
attributes owner; entry allocation before the manager check; four actual tree
pools of0x1cc00 bytes with4096 element slots (`TreeStore::Resize`82d7c468 doubles
maxClips2048); transition/free-list resets (`TransitionStore::Resize`82d7c598);
Clear82d7a6b8 attributes/construction-map/trajectory state; playback/hierarchy
and unknown64*N matrix allocation; the deliberately late missing-bind-pose
check; scheduler/dormant job; then channel initialization and stance/gestures.
The first-call restriction preserves that constructor-owned preexisting state
is untouched by any earlier animation evaluation.

`InitChannelAnimatable`82d7cc68 receives1024/sixchannels. PDB Channel size148
contains tree pointers at0/4, ChannelBlend at8, transition pointer44, Sequence48,
TransitionBlend64, transitioning byte84,parent88,base8c,priority90. Constructors
leave ChannelBlend numeric/pointer/flag fields unknown except its inherited
evaluatedfalse. Tree pointers/transition/parent/base null, priority0,
transitioningfalse. The concrete dictionary is initially empty; free channels
append0..5, free trees append0..11, each tree has1024bytes/20element slots. Root
channel remains constructor-null. There is no invented graph tick here.

Initialize writes last/natural stance and four gestures only after these
allocations. A supplied pro name is copied as FastString30 by SetProSkater82cb7ba0.
Natural stance GOOFY1 sets relativeNATURAL0 and clears flagsC0000000; otherwise
sets flagsC0000000 and relative is `(stance != 0)`, preserving out-of-domain raw
input behavior. Separate left/right Init methods retain actual named playback
and hierarchy, allocate unknown48*N local SQT, then clear flags02000000/01000000
and reset the corresponding clip pointer. Actor caller ordering remains root's
responsibility, including retrieving each actual playback at the source call.

`Animatable::Init` still does not populate its local/global matrices. The actual
next physical bind-pose producer is `GetBindpose`82d6e548 and its ACS Pose/Eval/
ExtractMatrix sequence, which root is recovering independently. The controlled
SkaterBodyPres physical fixture must not be replaced by a guessed composition
of the new winning VBR pose. Host Arc/container allocations are not native
allocator-failure parity; checked source/parse/missing-owner failures preserve
their demonstrated earlier effects.

## Buffered bind-pose producer and scheduler proof (2026-09-11)

Live IDA recovery closes the selected `GetBindpose`82d6e548 command path:
output hierarchy/allocation, BeginCommandSet82a22a20, BeginCommands82a229c0,
Pose::Eval825852d0 (GetPoseGuid82d6da40 then FetchPose82a2f478),
Pose82a27ea8, ExtractMatrix82a2c650, EndCommands82a289e0,
EndCommandSet82a330c8, Sync82a30088. NewACS824d62c0 explicitly builds its
Skeleton with trajectory index zero. GetNextLoc82a24c28 increments each
initially empty dynamic SQT/matrix list and returns the preceding index zero.

The emitted three nodes are FetchPose opcode13 -> Pose opcode4 ->
ExtractMatrix opcode11. Evaluate82a33018 visits dependency-free nodes in
ascending order. Schedule82a32800 fuses a following extraction into the
preceding engine job's extra-output pointer. Thus the actual selected job
dependency graph is a null completion job depending on ACSpose, depending
on FetchVBR; it is not three independently submitted commands.

FetchEngine::EnQueue82a30458 calls FillExtractBuffers82d74420. The latter
resolves the low16-bit GUID through DataBaseManager::GetPoseHeader82d6e928:
slot is GUID&31 and sorted surviving pose ordinal is GUID>>5. PoseBatchFetch
82d6f508 walks the **destination OnBoard hierarchy**, matching globally mapped
part IDs in the winning OffBoard pose. Eight extraction records cover its36
bones. Pure constant VBR decoding is supported; other codecs, missing parts,
gaps/overlaps and animated VBR channels remain explicit unsupported paths.
The original winning rig pose remains offset0x004614d0 in OffBoard, not the
similarly named OnBoard record or the separate SkaterBodyPres fixture.

FetchJobVBR82d71b70 calls Vector_ExtractPackedNVBR82d71268. First extraction
offset zero initializes destination SQT lanes; each source part is decompressed
at frame zero with ordinary pose channel semantics. ACSpose_job82a23e00 calls
the existing Anim_Tools::Pose evaluator and then copies64*numBones bytes to
its extra output. This is the source of GetBindpose's independent output;
Animatable Init's earlier local/global allocations remain separate.

Flush82a313a0 configures pending fetch before pose and finally updates null-job
dependencies82a31188 and calls ScheduleTree824d5310. JobPool::ScheduleCurrJob
82a242f8 only sets entry/category/parameters/dependencies; it does not submit.
ScheduleTree requires both the actual manager and initialized backend.

JobScheduler::AddTree829aec18 allocates all reachable instances not-ready in
breadth-first root->pose->fetch order, adds their start barriers, submits end
dependency events829ae8b0, then releases starts in that same queue order.
AddBarrier829ae000 raw stores EVENT_TYPE_WRITE1 at Event+0, ready destination
and value at+4/+8, and the instance decrementer at+12. Event::Run829acc50
decrements the barrier and writes readiness only on zero. Only fetch is ready
after start release. JobInstance::Run829aff10 runs begin events, the actual
entry function if nonnull, and end events; those release pose and then root.
WaitForJob824d53a8 category0 joins LocalBackend::RunSingleJob829b00b0 through
JoinLocalQueueForOneJob829acf10 with affinity0. Sync resets buffers/jobs only
after root completion. Job::Clear829ad7e0 resets entry/parameters/list counts
but does not overwrite the previous instance handle or start event.

Manager Init826e3490 creates the actual local backend with queue arguments
512/128 and three LOCAL/ANY workers with source processor hints1,3,5, then
enables deferred calling-thread jobs. The host backend owns three named worker
threads and a retained queue/barriers plus the caller-join route. Xbox processor
numbers remain source hints, not claims of equivalent host CPU pinning. Only
the proven selected three-node graph is executable; concurrent graphs, other
entry codes and backend substitution while waiting are explicit unsupported
paths. A failed work item remains failed and cannot publish root completion.

NewACS constructor ownership precedes registry publication: fetch entry capture,
null job, ACSreuseBuffer capture,13 fetch jobs/800 records with52864-byte logical
extract storage plus256 allocation padding, ACSsqtBlend capture/18 dormant jobs,
ACSsetData capture/12 dormant jobs. The last two constructors are82a31c40 and
82a31fe0; even zero set-data operations still allocate their12 jobs. Four actual
job-manager captures occur per body ACS. PoseEngine initially has no pose pool;
first Pose scheduling loads ACSpose and allocates14 jobs with19 dependency
slots and one operation per job for the body MaxPoseOps2 configuration.

The new source/corpus tests require independent parent review and gates before
being counted as validated. The corpus route uses the actual initialized
manager retained by startup, checks OffBoard GUID->OnBoard matrices, observed
command/job ownership and successful Sync/reset, including a repeat command set.

The queued ACSpose writes the same retained typed output allocation created
before BeginCommandSet. Read access remains unavailable until Sync; there is
no second replacement output allocation. A controlled entry-point unit check
verifies actual writes before the readable flag, independently of the full
original-data graph test. Root independently recovered and corrected the
existing matrix evaluator's VMX fused operation ordering; numeric acceptance
depends on that separate source review and its adversarial tests.

## Actual physical-init caller and physics parameter owner

Actor825fe850 uses a constructor-local `Init_IPhysical` (PDB size0x30) atSP+C0.
Its `animPhysicsData` starts at+0: hierarchyHandle+0, physicsParamHandle+4,
PoseBufferExt+8 containing mHir+0 and mPoseBuffer+4. The temporary pose therefore
lives atSP+C8. Instruction825ff108 initializes only its buffer pointer to null;
ff124 stores GetHierarchy, ff168 performs GetPhysicsParam("phys_Tpose").
World/services/LW/bouncy/human fields follow; ff2b4 calls GetBindpose withSP+C8,
ff2c0 reads HatType, ff2d8 allocates HiLOD0x340, ff2f4 constructs HiLOD using the
whole Init_IPhysical, and ff300 publishes Actor+0x200. At the end of the Actor
constructor, after the three interface validations, the temporary pose buffer
is explicitly deleted. A host retained physical-init owner models this local's
lifetime/failed progress; it is not an invented permanent native Actor field.

GetPhysicsParam82d7cb30 walks the actual playback DB list in insertion order
and uses each database's surviving kind5 name map. The selected body playback
therefore wins **OffBoard** phys_tpose at0x8b80, size0xac0. OnBoard contains its
same-name copy at0x2e00. Direct reads of the original anim.big entries prove both
whole records have SHA256
136d6dcaf907c96f51e6e5184f5c7ca46357ef1ef21ce0adf755162af7ed0ef5.
The previous physical fixture's numeric physics-parameter payload consequently
matches, while its owner and presentation RAW pose route are not the native
caller. Correct new inputs retain OffBoard physics ref, OnBoard hierarchy,
OnBoard pose hierarchy and the buffered GetBindpose matrices. Missing keys
return null/None; malformed retained references fail checked snapshot access.
