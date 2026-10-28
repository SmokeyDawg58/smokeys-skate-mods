# ReadSkater retained allocation consumer, 2026-09-22

Status: unregistered packet, independently source/code approved; root owns Cargo and
integration. No original-data ReadSkater success is claimed. The production
consumer requires actual submitted Data matrices and readable backing bytes.

## Reuse and source boundary

`animation.rs::LocalPose::overlay_matching_parts` is a general whole-SQT overlay,
not native ReadSkater: source writes named quaternion channels and selected
translations while retaining scale and authored base. Existing format RAW
helpers are mathematical decoders; their sqrt/division is not the native
estimate/refinement graph. This packet leaves those general APIs unchanged.
The existing matrix-to-quaternion candidate graph is reused for hips/board.
Existing restore, offsets, command collector, SQT output allocations, SetData
job code, category2 scheduler, and job pools are reused, not reconstructed twice.
MoveRoot, twist expressions, inverse bind and full-pose extraction belong to
subsequent UpdateBodies827de988 and are not executed early by this packet.

## Native order and retained stores

ReadSkater827ddb60 consumes Data+bc unsigned compressed offset, calls Q4dba0,
Q3dbbc, Tdbe8(range.5), then resolves CURRENT SkaterFull playbackdc0c. It visits
Torso,Head,Arms,Legs,Skateboard,Trucks,LeftHand,RightHand,Face0,Face1,Face2.
Part lookup/assertions are sequential. Truck quaternion stores precede translation.
Hand cvar is sampled for each bone; suppressed channels still advance scratch.
Face cvar is likewise per bone. Face translation uses actual source quaternion
and existing destination translation; translation store precedes the source
quaternion reload and quaternion composition. The paired allocation edit keeps
alias read-after-write behavior rather than snapshotting the source pose.
Hips translatione570 precedes matrix quaternione700; skateboard translatione71c
precedes quaternione850. Scale is untouched. Only then SetSQTe858 is emitted.
Host typed failures retain earlier scratch/pose stores, but do not publish a
successful synchronized generation after an interrupted edit.

Face raw VMX128 operands were decoded independently because IDA's printed high
registers and standard VMADD/VNMSUB operand ordering are misleading. Translation
is cross(baseQ,delta), baseQ.w*delta+cross, cross(baseQ,inner), 2*outer+delta,
then existing destination translation plus that result. Each cross uses native
multiply then negative FMA and cyclic permute. Quaternion uses baseQ*deltaQ,
ordered vector FMAs and W=baseW*deltaW-dot3(baseQ,deltaQ); no normalization.

## RAW decoding

QuatDecompressQuantize3 at827d1220, Q4 at827d1300, TransDecompress827d13b8:
vspltish1 creates 00010001 words; vslw shifts each input word by1; vmrghh
interleaves the first four original and shifted halfwords. Expanded signed word
for halfword h is (h<<16)|((h<<1)&ffff), plus following halfword sign bit for
even lanes. Native signed-word float conversion scales by2^-31, then multiplies
32768 (table82111020), then scalar38000100. Q3 zeroes W before that scalar;
T scalar is range*38000100 and W becomes1 afterward. Q4 dot4 and two reciprocal
sqrt refinements normalize all4. Q3 computes1-dot4, evaluates reciprocal sqrt
and both refinements even for nonpositive radicand, then selects positive zero.
Dot reductions and estimate responses are supplied numeric services; this is
not unconditional bit parity with unmeasured hardware seed/rounding behavior.

The final translation Z can observe the sign bit of the following halfword.
Therefore the bytes API requires actual backing through that halfword, including
source-owned padding if known. It does not manufacture zero bytes or derive
local hips/board matrices from decoded data. Root owns WriteData/PostGather and
must connect the actual completed buffer/header producer.

## SetSQT producer/consumer

SetSQT82a2f3d8 checks command scopes; SetData82a2c828 consumes GetNextLocc84c,
then increments usage and stores source pointer in the command. The draft uses
an Arc to the same SQT allocation and a new dynamic slot. Capacity is the per-character native MaxSetReplace count (ACSParams cap[2],
full3/face1), distinct from the aggregate SetData pool54/18. Usage increments
after dynamic allocation and before pointer command admission.
EnQueue82a2c240 keeps the pointer and optional extraction destination; the actual
ACSsetData_job82a24000 copies at execution, then copies to attached extraction.
The existing selected SQT chain now admits SetData alone or SetData then Blend.
Mixed independent RAW Fetch+Set graphs remain a typed unsupported graph, rather
than inventing an ordering. The existing category2 affinity/submit/wait is reused.
A same-source extraction can already be pending when SetData executes; internal
job reads admit actual known storage in that case without completing extraction.

## Cvars and tests

NoHandAnimation initializer82f6c4f0/51c: Animation / cv_NoHandAnimation /
Hands in BindPose, false, nullcallback; ref830532d4, table82fd241c.
NoFaceAnimation initializer82f6c540/56c: Animation / cv_NoFaceAnimation /
Face in Bindpose, false, nullcallback; ref830532f8, table82fd2420.
Reserved indices280/281. Registration follows existing115; manager title order
is Face,Hands,Individual. Reads are linked-target reads, not retained snapshots.

Tests cover adjacent-halfword widening and bounds, negative-radicand estimate
execution, actual base composition with aliasing and retained scale, live cvar
updates, SetData execution-time source sampling with pending alias extraction,
and partial edits retaining stores without successful generation publication.
Patch apply-check and owned-file rustfmt checks passed. Cargo/original-data
execution remain root's gate.

Subsequent approved platform allocation policy is recorded in presentation-data-buffer-boundary-2026-09-22.md: a zero-initialized host backing allocation is explicit once at construction and retained across frames. This decoder observes that real backing, never appended fake padding.
