# Physical skeleton input producers

2026-09-07. Root integration; independent source and implementation
review PASS. Final strict and mounted validation PASS, detailed below.
Confidence **high**, not native per-tick numeric trace parity.

## Implemented boundary

`skeleton_data.rs` now retains original Andale hierarchy/PhysicsParam owners,
constructs physical bone/parent/joint/child mappings and both volume/bone
transforms, and copies the supplied physics pose. `skeleton_attributes.rs`
reads the original retained Vault instances and produces the material/drive/
bone/joint inputs for `SkeletonBody::Init`. These are actual constructor-input
producers, not an admitted humanoid, live pose-selection service, mutable Vault
editor, or complete `SkeletonBody` constructor.

The existing uncommitted PlayRemappedAnimation work is unchanged and unaccepted.
No renderer/dependency/pool changes are part of this slice.

## SkeletonData evidence and semantics

Source `Sk8::Physics::SkeletonData::Init` 0x82b56778..0x82b56d70,
`Andale::PhysicsParamData::GetParentID` 0x82b566e8,
`HierarchyData::GetBone` 0x82590fc0 and PhysicsParam `GetBone` 0x82b56620.
Source Init SHA-256
`422fe367a5e6567475955fe2bb9b43261af42790e39b50be306d385785579b50`;
GetParentID SHA-256
`615a0b34e346398f09db89f85569552587d6dc541b70845044b6a2c971d29354`.

Complete passes, in order:

1. Retain hierarchy/parameters; initialize the used animation-to-Part range.
2. Resolve all24 physical records in original order using all five packed name
   words. Name lookup takes FIRST match; duplicate animation IDs overwrite the
   inverse map with the LAST physical Part. Parser now retains original packed
   PhysicsBone names alongside its existing canonically decoded label.
3. Starting at each immediate animation parent, walk until the first ancestor
   whose name is represented in PhysicsParam. Resolve that animation index via
   the completed reverse map; append joints in ascending child-Part order.
4. For every parent Part, scan all24 candidate children in ascending order.
5. Construct each bone-to-volume matrix and its general inverse.
6. Copy animation parents; copy all16 supplied pose words per resolved bone.

PDB capacities:100 animation bones,24 Parts,22 joints,3 children perPart.
Native does not initialize root Part-to-joint slots, RootPartIdx, or unused
child/joint tails here. Host exposes only initialized semantic prefixes and
explicit absent root-joint mappings. Exact24 preflight is a checked-host
boundary: native outer Skeleton::Init asserts24 only AFTER Data::Init and
other work. Invalid indexing, nonterminating ancestor walks and capacity
overflow return errors; no uniqueness or global tree-shape constraint is added.
Unvisited invalid parent rows and represented self-parents remain accepted.
Pose has its OWN hierarchy count; pointer identity with mapping hierarchy is
not required. Host ownership is immutable and failure returns no partial owner.

### Shared numeric graphs

Independent raw instruction comparison approved existing `drive_basis_host`
(now crate-visible) and `world::inverse_collision_affine_reset` reuse. The
former uses source sqrt2 bits3fb504f3 from0x8227e8e0, no normalization, and
retains noncanonical axis W=[diagonal.x,difference.x,sum.x]. Position retains
all four authored words. Matrix stores are translation/Z/Y/X before inverse.
The latter is a GENERAL inverse, not board rigid transpose: cofactors YxZ,
XxY,ZxX; transposed W duplicates Y; determinant dot3; reciprocal seed and TWO
fused refinements; XOR-negated position accumulated Xmul/YFMA/ZFMA in all lanes.
Existing host nested-dot/reciprocal-estimate behavior is retained. Exact Xenon
estimate/reduction, denormal and exceptional-value payload parity is unproven;
source operand swaps with shared kernels are part of that exceptional boundary.
No new generic inverse, interpreter, normalization or singularity fallback.

## InitAttribData evidence and semantics

`Sk8::Physics::SkeletonBody::InitAttribData` 0x82b3c3b8..0x82b3d0a8,
called by ctor0x82b62200 at0x82b625b8 before Body Init/TeleportReset.
Source SHA-256
`7d7a245d4ccdf66960975f564c795193ade652793fcbf803be330d2311b7d6a6`.

Generated classes, each bound to default d7edbd362d7d2152 in this order:

| Class | Key | Generated ctor | Cached layout bytes |
|---|---|---|---|
| physics_skeleton | 9f862c2eaa016ce5 | 828b10e0 |112|
| physics_skeleton_bones | f60dfc082242b489 |82b3e5b8|576|
| physics_skeleton_drives | fdf618600d396c3b |828b0bb8|736|
| physics_skeleton_joints |9ceb4dce6a213c4e|82b3e630|440|

Generated ctors replace null cached-layout pointers with DefaultDataArea(size).
DefaultDataArea828b5340 returns83059d30; root and auditor independently verified
first736 bytes zero. This is distinct from scalar GetAttributePointer(index0)
fallback to DefaultDataArea4. New internal try_cached_layout returns retained
original collection layout without generic-node or parent-layout substitution;
it preserves pointer/range/relocated-payload failures. Class wrapper owns the
proven null default. Immutable captured-layout semantics only, no Change or
message-port/notification timing claim.

Material lookup chronology:

| Material | Restitution key | Friction key (looked up TWICE) | Load order |
|---|---|---|---|
| Normal |8a433894d23421e6|fd1e0898d6f2f5b8|second,first|
| Ragdoll |5e3ef6cc89228e59|b8d8eceaa7b3b603|second,first|
| Head |bac7c1be38090b31|b086b9c7da14b441|first,second|

For each material restitution is looked up/loaded first. Both friction handles
are then acquired before either load. Static friction stores second lookup's
value, dynamic friction first's, even though normal/ragdoll versus head LOAD
ordering differs. Normal then ragdoll then head. Zero material is not changed
by InitAttribData and is not invented by the snapshot producer.

Then capture drive layout ONCE and copy23 full32-byte records to Parts1..23.
Source layout indices:
`[12,13,18,19,21,16,8,9,11,6,0,1,2,3,15,20,17,14,5,10,7,4,22]`.
Root drive0 stays exactly as supplied. Native ctor82b623cc..3ec initializes
all7 float fields and compliant byte to zero but not padding; constructor_value
represents that padding as unknown, not fabricated zero bytes.

After all drive copies, lookup/load ragdoll multiplier851dfc1d6128976f, then
collision057eddfa341f9492. For each Part1..23 evaluate field5*collision FIRST,
then ragdoll*fields0/1/2/3, then collision*field4; store fields0/1/2/3/4/5.
No clamps or fused arithmetic. Compliant/padding and collision priority unchanged.

Next capture Bones layout once; copy24 full24-byte records to Parts0..23 using
`[4,13,14,19,20,22,17,9,10,12,7,0,1,2,3,16,21,18,15,6,11,8,5,23]`.
Then capture Joints layout once; copy22 full20-byte records using
`[12,1,16,18,14,2,9,11,7,0,3,4,5,19,17,15,13,21,10,8,6,20]`.
Root checked every unrolled source copy and PDB generated member name; auditor
independently recovered the same permutations. Raw BIN order is NOT Part order.

Typed records preserve raw bool bytes, all padding bytes and float bit patterns
on copies. Bone booleans are bytes8/9, drive compliant byte24, joint ball byte0;
padding nonzero does not enable a false boolean. No guest ABI or host casts.

## Original-data validation inputs

Original external Xbox360 anim archive -> OnBoard.abin -> unique phys_tpose
type5 record2e00/ac0. Hierarchy30/550 has36 bones. The initial corpus check
exposed24 same-name/GUID0 rig_tpose records in six byte-content classes, so name
alone is insufficient. The audited6c0/codec524256 class has ten byte-identical
copies, but its VBR pose is unsupported by sample_raw_sqt. The corpus did NOT
reinterpret those bytes or add an unreviewed decoder. All72 OnBoard poses and
84 OffBoard poses are VBR. The final controlled fixture instead uses the
unique named RAW4452 rig_tpose from original SkaterBodyPres.abin (580/3d0).
Its hierarchy has the same36 packed names at the same indices, but six parent
rows differ (1,25,32..35). Decode and global-compose against ITS actual
hierarchy; supply that separate pose hierarchy to SkeletonData while retaining
OnBoard's actual mapping hierarchy/PhysicsParam. This exercises the source's
distinct pose-hierarchy contract; it is NOT a recovered live pose selection or
PlaybackData duplicate-resolution policy. No content identity is runtime tuning.

Frozen original-record FNV-1a64: hierarchy0bc88016eab190d9,
audited VBR rig_tpose8ad63282bf2f1783, phys_tposeadc20621dd1ce628.
SHA-256 respectively:
`17285fe195f48043953e1137683285a981447d9bb545bce885b453884f07f4f3`,
`b9f672224615e0228bde1e79fd5a9d3fcf32404a8639f1ccfa59c0f009460c16`,
`136d6dcaf907c96f51e6e5184f5c7ca46357ef1ef21ce0adf755162af7ed0ef5`.
Selected raw SkaterBodyPres rig_tpose FNVc94575cee13fe704, SHA-256
`546bc6ad83ba0a378e35f815542d50141b0400691984defa8b498a2a7e746b2c`.
Its hierarchy FNV12e7f9ba8e828c41, SHA-256
`5d3842f2ef4446725064a331a01671956b7e96dd1860099e0550cd30a3fceef7`.
Actual bone IDs
`[25,7,6,16,15,14,13,12,11,10,9,5,4,3,2,24,23,22,21,20,19,18,17,1]`;
roots Parts0/23,22 joints; hips children14/18/22, spine3 children2/6/10.
Original db.big -> retained skater.vlt/bin supplies attributes; frozen data
values remain assertions, not constructor tuning constants.

## Remaining integration

Need actual SkeletonBody shape/mass-definition creation using these inputs,
normal joint/culling/setup mutation and actual24-Part pool admission, separate
two-Part skeleton hook/drives, live physical-player pose, controls/input and
source World scheduler. Body Init installs group1/userdata1 before admission;
SetUpNormal later changes both to6 and animated dynamics/materials. Immutable
constructor attributes do not justify immutable live body inertia or fixed
publication group. Actual source skeleton/board pair is still missing.

## Validation result

Final shell87800 exit0: format check, workspace tests2182 core +187 formats
+38 app +3 symbols =2410 unit tests, six doctests, all-target warnings-denied
Clippy, all SIX ignored core original-data tests, and git diff whitespace check.
The app ignored corpus was not rerun. Eighteen new ordinary tests comprise12
mapping/pose/numeric discriminators,5 attribute chronology/permutation/default/
failure tests, and a four-class generated cached-layout ownership/override/
parent-null discriminator. Existing parser test now asserts retained packed words.
Both new mounted tests pass; independent review includes the final distinct raw
pose fixture and direct cached-layout test. No native numeric trace is claimed.

Previous board terrain results are unchanged: C/J/D6/6/3 three times then4/6/3,
same four deck centers, and separate eight-frame DMO contact hash0e7c704e6a10f08a.
No new humanoid simulation frames were run by this input-producer checkpoint.
