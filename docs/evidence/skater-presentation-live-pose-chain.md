# Live Actor to full presentation pose

September22 status note: the historical missing hand/face producer described below
has since been implemented in the retained Actor hands/face session and selected
frame path. Presentation InitAnimation/offset/restore/compression/root/twist drafts
already exist but remain unregistered; do not recreate them. The live simulation
publication to WriteSkater/ReadSkater/full-pose/GPU consumer chain is still missing.
This note updates integration status, not the historical source evidence below.

2026-09-12 bounded read-only source audit. Confidence **high** for the owner and
call ordering below; numeric lifting of presentation compression, root and twist
operations remains unimplemented/unverified. No Cargo, new executable probe,
fabricated pose or production change accompanies this packet.

## Actual simulation publication

`SubmittableSkater::GetLocalPose825f04e0` checks the SAME Actor's cached physical
interface and calls its GetLocalPoseBuffer. Concrete
`PhysicalPlayerHiLOD::GetLocalPoseBuffer82dffd88` returns
`mAnimOutPhysIn->localPoseBuffer`. Thus the source is the retained physics handoff
local allocation, not a new full-rig pose or the body's global animation buffer.
Existing `actor_anim_out_phys_in.rs` and `actor_animation_get_phys_update_data.rs`
own this allocation and its actual global-then-local copy.

`SkaterPresentation::DoSimSubmit827dd2e8` selects actual submittable Actors from
SkaterManager, then `WriteData827dbf98` writes their records and calls
`WriteSkater827d8ab8` at827dc610. The outer submission call is
`PresentationObj<SkaterPresentation>::SubmitNow827f0320` at827f038c.
`WriteData` records full local hips/board matrices from that same local pose at
827dc3c0..43c, separately from the compressed per-bone channels.

`WriteSkater` reads local pose (8b08), face buffer (8b20), right hand (8b38),
left hand (8b4c), in that order. It converts selected local matrices to Q4
quaternions, compresses truck translations with range0.5, then appends Q3 hand
rotations and face Q3/translations. SubmitData sends quaternion data first
(90ec), translations second (9100). Compression must remain observable in a
source-faithful local handoff; directly copying full-precision matrices would
skip the native quantization. Existing formats helpers decode Q3/Q4/translation,
but their numeric policy needs independent comparison with these specific calls.

`InitOffsets827dd548` supplies the exact ordered selection: Torso, Head, Arms,
Legs, Skateboard, Skateboard_Trucks rotations; truck translations; left/right
hand counts; Face0/1/2 ranges. Its hierarchy inputs are original SkaterBodyPres,
SkaterLeftHandPres, SkaterRightHandPres and SkaterFaceSimPres databases, loaded
by InitAnimation. Do not infer these offsets from the current body pose length.

The hand getters825f0670/06e8 and face getter825f05f8 read the SAME SkaterAnim's
local SQT allocations (full-owner +52e8/+52f0/+52f8 PoseBufferExt members).
`IsInitFace825f0230` reads actual flag23. Null hands have a source zero-Q3 path;
uninitialized nonnull buffers are not null and cannot use that branch.
Current hand owners in `actor_animation_initialization.rs` allocate
`MaybeUninit<AndaleSqt>`; face owns `Vec<Option<AndaleSqt>>` in
`actor_face_playback.rs`. Their allocations alone do not prove published values.

## Missing hand/face producer

`SkaterManager::UpdateAnimation3 825fa4e0` completes body phase3 jobs, gets the
actual left/right hand ACS owners, begins their command sets at a8c4/a8cc,
calls `Actor::DispatchHandsFaceAnim825f87b8` at a8f0 with source fixed dt, then
ends right and left sets at a904/a90c. Actual manager synchronization must
complete extraction before WriteSkater consumes these allocations.

Dispatch clears actual initialization bits, uses actual local-player/LOD gates,
and evaluates the same retained body/channel tree into BlendPoseBuffer.
Raw825f88f0 loads virtual+84: PDB identifies `EvalBlendPose`, correcting the
decompiler's erroneous `IsInJumpAir`. It then schedules face and hand streams,
including the explicit fallback-hand calls at8a4c/8a8c, and sets their actual
initialization bits after scheduling.

`SetUpBlendPoseStream825f8398` gets actual named playback/ACS/fetch owners,
processes the native stream tokens using Clip Eval or SetSQTById and ordered
Blend/NBlend operations, and ExtractSQT at87a0. If addbindpose is true, it fetches
the actual `rig_TPose` and AddSQT when a stream produced values. Otherwise a
missing/empty stream fetches the actual `IdentityPose`. These are proven authored
fallback operations, not permission to skip EvalBlendPose or synthesize identity.

Relevant reached-tree source entries: Animatable EvalBlendPose82d6bb68,
Clip82d77e00, PhaseBlend82d793b8, ChannelBlend82d79bf0, AddSQT825884e8,
Pose82587b98, Mirror82c92878. Existing tree runtime, ClipFetch and buffered
fetch/blend owners should be reused; token production/extraction is the missing
consumer path, not another body animation evaluator.

## Presentation construction and restore

`InitAnimation827e1c08` loads original SkaterPresentationDB, creates SkaterACS
and full playback ownership, records actual hips and skateboard indices, and
allocates mBPSQTBuffer (+7c8), replay pairs (+a3c), interpolated matrices (+20)
and save storage. It FetchPose/ExtractSQT of actual `rig_Tpose` at20d4..20ec and
Syncs at29c4. SkaterFacePres playback is separately constructed. It also obtains
the original source hierarchies for InitOffsets and actual twist bone indices.

`PresentationObj<SkaterPresentation>::Restore827f03b0` reaches
`DoSimRestore827e6438` at827f0410. In the admitted non-NIS/CAS branch, it restores
actual previous/next data, frame mapping, model associations and blend values.
Raw695c..697c copies the SAME authored mBPSQTBuffer to each odd replay buffer.
Actual character recipe face selection looks up a FacePres pose (6b4c, with
source gender/name fallbacks) and FetchPose/ExtractSQT into Face_Offset0
(6da8/6dc4). After Sync6fc8, raw6fe0..6ffc copies odd to even, then calls
UpdateBodies7014. Authored initialization is therefore native behavior here;
using it to omit the subsequent live writes would still be incorrect.

`UpdateBody827de868` selects previous data when frameDataMap is valid, otherwise
the next data; it reads previous and/or next according to flag8000 and blends
only when required. `ReadSkater827ddb60` decompresses the selected Q4/Q3/T data,
writes named body quaternion ranges, truck quaternion/translation, hands under
live NoHandAnimation gates, and face deltas composed with the actual selected
face base under NoFaceAnimation gates. It then explicitly replaces hips and
skateboard SQT rotation/translation from the submitted full local matrices and
SetSQT at827de858. Generic `overlay_matching_parts` does not implement this graph.

`UpdateBodies827de988` extracts the blended SQT and Syncs atdeb1c, then
MoveRootToHips (deb24) and ComputeExpressions (deb2c). The latter unconditionally
uses the retained GlobalAttributeCollection animation_twistbones values for eight
TwistThatBone calls: arm, two forearm twists and upper-leg twist on both sides.
Source functions827d35d8/827d99a0/827d9540 still need numerical lifting.

The second command pass sets that corrected SQT, sets the ACTUAL
SkaterPresEntity InvBindPoseBuffer at+214 (rawdeb88), PoseWithMatMult (deb94),
ExtractMatrix (deba0), and Sync (debd0). It subsequently publishes the hips
matrix and performs the actual pose-saver service query. Model StartJobs at
827e7098 and later EndJobs/UpdateBoneTransforms/PackAndMultiply consume these
matrices; the existing GPU row interface expects the resulting per-mesh rows.

## Concrete next scope

The first missing nonnull producer is the hand/face stream path. Proposed
exclusive new files: `actor_animation_blend_pose_stream.rs`,
`actor_animation_hands_face_dispatch.rs`, focused tests and source evidence.
Reuse current retained tree runtime, source ClipFetch, named playback owners and
buffered commands. Narrow shared changes would add tree-child registration,
actual ExtractSQT destinations and only command variants reached by original
tokens (NBlend/SetSQTById if reached). Root owns the SkaterManager scheduling and
sync seam. Replace unknown hand storage only through successful actual extraction.

Full-rig InitAnimation/InitOffsets and the authored restore-base construction are
an independent useful owner implementation, but cannot alone publish a complete
live skinning pose. Following work must preserve compression, selected face base,
ReadSkater channels, actual previous/next blending, root/twist updates, inverse
bind and final model ownership. No body-only or named-trick fallback closes this
remaining chain.
