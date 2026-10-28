# Pedestrian and guard animation investigation

Status: source reconstruction in progress; no compilation, tests or game launch.
The scheduling and attribute-reader additions are foundations, not a completed
pedestrian presentation path. No placeholder or procedural animation is admitted.

## Original owners and execution

- `Pedestrian::Pedestrian82e97748` selects entity animation data (or explicit
  override), reads its motion graph string, then loads the shared state graph.
  Entity animation RefSpec key is `00367b8f33e79c43` (parent integrator PPC audit).
- `cPedestrianAnim::cPedestrianAnim82e90390`: full hierarchy, bind-pose SQT ID,
  playback `pedestrian`; properties playing/expired false, times -1. Animatable
  Init arguments 1024,3,2,32; ChannelAnimatable Init 256,3. Playback string proven
  by initializer82fa55a8. Mirrors initially false.
- `AnimMan::Init82718d08` loads original `data/anim/PedestrianSkeletonPres.abin`
  then `PedestrianSkeletonSim.abin`. `TrajPedestrianACS` uses SIM hierarchy;
  full playback uses PRES hierarchy with the same ACS/fetch owner. Initial full
  database list is PRES skeleton only. Trajectory playback has SIM skeleton.
  Animation database appends and presentation ACS remain under investigation.
- `BuildAnimTree82e6e878`: two alternating graph pools, initial indices2/3
  decrement by2 and update immediately. Active0/1 updates only with valid
  intents, using `dt*2 + offset`, resetting offset. Invalid intents swap pool
  `(index-1)&1`, save dt and skip update. Skipped update without a current tree
  is forced at unscaled dt. Every branch then SetAttributes, AddTime(dt),
  ProcessTreeAttrs. New scheduling owner preserves these decisions.
- `cPedestrianAnim::AddTime82e904f0`: save prior base expired flag, update
  transitions, remove expired channels, base AddTime, copy base properties,
  OR prior expired flag into pedestrian property.
- `AddBindPose82e7f460`: absent tree stays absent; otherwise SetSQTById bind
  pose plus AddSQT(current,bind), motion operand=current.
- `EvalBlendPoseBuffer82e79d30`: regenerate only when renderable and current
  tree exists; validity false otherwise. `RegenerateBlendPoseBuffer82e78430`
  calls shared BlendPoseBuffer Init/Eval.

## Remap semantics and implementation

`Pedestrian::RemapAnim82e8d8e0` first consults registered per-entity semantic
overrides. Otherwise hashes the original name via Attrib::StringToKey and reads
selected animation data. Missing attribute retains original name. Definition
array flag1 selects length; only length>1 consumes one shared LW RNG value,
modulo length. Scalar and <=1 use index0. Native compares the FULL type constant
`5088a439d663388c` (`lis/ori/insrdi82e8d9d8..f4`). This is
`Anim::tAnimAttributes`, PDB size56, Clip pointer+0, foot fields+4/+8/+12,
branch count+16, three12-byte branch records+20. Other type branch reads char*.

The new child `attribute_runtime_pedestrian_animation.rs` reuses retained Vault
node/parent/definition/pointer owners. Structured pointer projection uses the
shared TextField projection. Stored-pointer arrays use original count/stride/
encoded padding. Other array storage domains remain explicit errors, matching
existing typed-array readers. Read-only Python corpus audit used the original
db.big extracted to `%TEMP%/skate-ped-vault` and existing vault_dump.py reader:
class `53e332f67b8124b2` has13 collections and61 tAnimAttributes definitions.
Parent-resolved values comprise728 scalars (serialized node flags0) and65 arrays
(flags2). All882 Clip pointer slots resolve through original relocation tables.
Array strides are56 and counts1..6. Thus all original tAnimAttributes storage
lies in this implemented projection domain. FwdWalkCyc correctly resolves to
NPC_WNDR_WLK_N_0_CYC, GRAN_WNDR_WLK_N_0_CYC, FEM_WNDR_WLK_N_0_CYC,
BUM_WNDR_WLK_N_0_CYC or BB_PTRL_WLK_N_0_CYC by selected collection.

`LWAnimHelper::SetupRemappedAnimation82e90730` also mirrors to match feet,
resolves frame tags, consumes motion transition override, then calls shared
Play/Sequence/Blend. Successful play sets cycle swapping probability; missing
animation asserts conditionally then plays original `HIP_TEST_01`. This fallback
must not be used to hide unsupported graph/runtime behavior.
`UpdateRemappedAnimation82e90b50` gates random cycle substitution on expiration
and transition state; it is not a per-frame unconditional clip restart.

## Real graph corpus and remaining work

Original named `data/big/miscload.big` extracted read-only with existing
`tools/extract_big.py` to `%TEMP%/skate-ped-audit`. The original pedestrian
compiled graph has2947 nodes,4264 attributes. XML expands26 include files.
Default WalkCyc ultimately chooses semantic `FwdWalkCyc`, whose actual clip is
per-entity remapped; selecting a fixed clip is not equivalent.

Existing generic motion backend supports HasIntent/WillExpire/PlayAnimation/
PlayRemappedAnimation. Original pedestrian graph additionally needs LastState,
BranchWindowEnabled, InTurnBranchWindow, InTransitionInfoWindow, InBranchWindow,
HasPlugin, jump conditions, LocomotionTransition/TurnComplete, intent completion,
transition registration, speech, hand-prop interaction and channel callbacks.
These cannot be silently ignored or substituted by generated walk cycles.

Reusable owners exist: `AnimationTreePool` and its tree time/attributes/eval,
`FacePlaybackData::try_construct_animation_tree`, actual Andale codecs,
shared CharacterPalette/material path and shared recipe registry. Actor body
Play/Blend/channel methods currently couple to ActorAnimationConstruction;
pedestrian owner extraction/adapter remains required. No separate Andale parser,
fake skeleton, secondary recipe cache, or static pedestrian display was added.

Confidence: high for named native control flow and PDB layouts, source-reviewed
scheduling/remap translation. Complete runtime/pixel parity is not established.

Further presentation source: InitRegularPedestrianAnimSys8280cd40 finds shared
database `PedestrianSkeletonPres` (initializer82fa5578), creates FullPedestrianACS
against that hierarchy, registers presentation playback `pedestrian` with that
same DB, and sets callbacks for entity SQT/palette/inverse-bind ownership.
Presentation rig name is `Pedestrian_rig_Tpose` (initializer82fa5590).

## Playback startup and ACS branch

`PedestrianPlaybackStartup` now calls existing SimulationAnimationStartup
database/playback registration and existing PresentationAnimation registration.
It retains PRES/SIM DB references and the three original playback identities:
simulation `pedestrian`, simulation `pedestriantrajonly`, presentation
`pedestrian`. The two SIM playbacks share TrajPedestrianACS. Full SIM playback
has PRES hierarchy although its ACS has SIM hierarchy, as in the source.
Original constant VBR bind SQTs are inserted into the existing fixed-frame
registry. This does not publish render poses. Matrix callback registration and
entity command submission remain to be completed before rendering.

Original anim.big corpus inspection: PedestrianSkeletonPres.abin contains
543 VBR clips, one VBR pose, one BlendSpace, one hierarchy (2,565,776 bytes).
PedestrianSkeletonSim.abin contains only hierarchy (832 bytes). The full database
already supplies ordinary pedestrian clips; its name does not mean it lacks
animation data. Extracted copies remain outside the repository.

The existing BufferedBodyCommands rejects arbitrary descriptors. A dedicated
source-verified child now admits only original40-pedestrian descriptors:
capacities[12,4,2,0,1], averages[3.6,1.2,.6,0,1], maxNBlend2,fixedSQT1,
mode1,fetch0,codec1,trajectorytrue,truncation0; SIM(category0,fixedMatrix1),
PRES(category2,fixedMatrix2). Original skater/hands branches remain unchanged.
PDB ACSParams64-byte offsets plus PPC82718ed0..f34/8280cdb0..ce30 checked.
ACS82a32168, FetchEngine82a31de8, SQT82a31c40 and Set82a31fe0 establish
80 dynamicSQTs,40 dynamicMatrices,160*parts extract records,256 perVBR job,
18 blend jobs with33 ops each,12 set jobs with41 ops each,40 pose operations.
Files syntax parsed with rustfmt only; no compile/runtime validation.
