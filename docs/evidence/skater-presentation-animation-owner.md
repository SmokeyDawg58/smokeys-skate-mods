# Retained presentation animation construction and restore base

2026-09-12. New files are **unregistered drafts**, not a successful runtime or
render publication. Root owns Cargo, live compressed WriteSkater/ReadSkater and
the final UpdateBodies/model/GPU connection. No original assets are copied here.

## Constructor source and owners

InitAnimation827e1c08 loads `data\\anim/SkaterPresentationDB.abin` at1d10 into
the actual shared Andale database registry. Presentation owns a distinct command
manager and playback manager; using Simulation's similarly named playback map
would collide with simulation face ownership. The proposed constructor receives
the actual presentation managers with their shared database-manager Arc.

GetBone resolves `hips` at1d4c and `skateboard_root` at1d84 using all five packed
name words. The native constructor then creates `SkaterACS` and registers playback
`SkaterFull`. The latter literal is independently confirmed by initializer82f7f6b0.
Full-rig bind allocation follows at1ee0, 18 replay allocations at1ee8..1f28, nine
interpolated matrix buffers at1f3c..1f70, and save matrices at1f9c. All begin unknown.

GetPose(`rig_Tpose`)2068 must succeed. BeginSet20a4/Begin20ac, GetPoseGuid20d4,
FetchPose20e0, ExtractSQT20ec, End20f4/EndSet20fc populate that actual allocation.
The 14 named arm/forearm/leg and twist indices are resolved before Sync29c4.

Face database `data\\anim/SkaterFacePres.abin` loads at2a30, command owner
`FacePres` is constructed at2b04, and playback `SkaterFace` is registered at2b40
(literal independently confirmed by initializer82f7f698).

Fresh raw ACSParams layout proof uses the 64-byte PDB struct: capacities at0..16,
averages20..36, maxNBlend40, registered SQT44/matrix48, mode52, fetchmode56,
category60. Caller words are loaded into r9/r10 and the remaining48 bytes copied
to outgoing stack; decompiler NewACS arguments are unreliable here.

| Property | SkaterACS | FacePres |
| --- | --- | --- |
| capacities | 13,6,3,1,2 | 2,1,1,0,0 |
| averages | 6,3,1,1,2 | 1,.5,.5,0,0 |
| maxNBlend/fixedSQT/fixedMatrix | 0,0,0 | 0,0,0 |
| mode/fetchmode/category | 1,1,2 | 1,0,2 |
| characters | 9 | 9 |
| truncation offset | .5 | .5 |
| trajectory | true | false |
| codec | RAW0 | RAW0 |

Raw1c40..1ca8 establishes full values;2a34..2a74 updates FacePres values while
retaining category2. Raw1de4 and2af4 set the true/false trajectory argument.

## Offsets and temporary database lifetime

Load order is FaceSimPres2bcc, LeftHandPres2c3c, RightHandPres2cac, BodyPres2d14.
InitOffsets2d34 derives body matrix quaternion indices in exact order Torso, Head,
Arms, Legs, Skateboard, Skateboard_Trucks. Trucks also supply translation indices.
The 120-rotation assertion occurs after truck insertion at827dd948.

Hand quaternion counts are the original hand hierarchy bone counts, not inferred
full-rig ranges. Face0/1/2 offsets/counts derive from FaceSimPres; all three add to
Q3 and translation counts. Pose size at827ddb10 is8*Q4+6*(Q3+T), followed by
conditional allocation of two unknown ghost byte buffers.

Temporary destructors execute in FaceSim/Left/Right/Body order2d3c/4c/5c/6c.
DataBase destructor82d741cc calls Clear82d72328; Clear calls RemoveDataBase at
72344 before clearing the source data/maps. RemoveDataBase82d6bd28 writes the
actual slot null under the manager lock. It does not remove global part names.
The draft preserves this cleanup order and retains incomplete allocations if an
earlier source load/parse fails. mIsAnimInit is written only at2d7c after cleanup.

## Selected restore base

This bounded API starts with the actual recipe resolved by UpdatePresEntity;
it does not claim to implement that entity/model association or frame mapping.
It supports the first selected skater using existing replay0/1 allocations.

Restore827e6838 reads linked `cv_IndividualFullbindpose`. Default false is proven
by initializer82f6c450, reference8305328c, table82fd2414. The false branch resolves
full hierarchy Face_Offset0 and chooses FacePres; true chooses full playback and
offset0. Odd replay receives full authored bind via OverrideWith6968. Face/full
command set begins69a4. Actual recipe schema is read6a60 and GetPose(schema)6b4c.

Only a missing schema pose invokes fallback. Raw6b70 compares recipe category
with3 (Category_CreateACharacter). That branch queries actual IsMale6b78 and
selects `CAC_male` or `CAC_female`; all other categories select `rig_Tpose`.
String addresses8210d4fc/8210d508/8210d514 were read directly. If both lookups
miss, source still begins/ends an empty command stream and retains the full bind
copy; it does not invent a pose. A successful pose fetch extracts into the SAME
odd replay allocation at its actual bone offset6dc4. EndSet6e54/Sync6fc8 precede
odd-to-even copy6fe0..6ffc. UpdateBodies7014 remains the next caller stage.

## Required shared amendments before registration

- Extend existing RAW ACS construction for the two exact descriptors, preserving
  actual pool sizing and matrix-engine constructor ownership. Do not use hand
  category0 parameters or a new synchronous decode shortcut.
- Add checked SQT subrange views sharing the allocation Arc; extract into the
  selected range and preserve untouched authored values. Add same-allocation
  OverrideWith with synchronized-source checks and generation tracking.
- Expose existing matrix-output allocation and StreamCommands narrowly for this
  owner; all fetching still uses existing mapped RAW FetchPose and ACS work.
- Store actual job affinity per queued instance. SetCategoryForJob824d4f44..58
  maps category0 to mask1, nonzero to mask4. WaitForJob category2 joins mask4 at
 824d5460; category0 joins mask1 at54f4. Host worker mask63 may consume both;
  caller joins must filter, not silently execute all categories.
- Reserve cvar115 after frame's114. Merge native table2414 in original table
  order; do not append it to physical registration order. Title is
  `Individual full(on)/facial(off) bindpose`, sectionAnimation, defaultfalse,
  nullcallback. Restore reads it live.

Two drafted offset tests independently vary serialized part order and verify
native packed ordering/byte size and missing/outside-range rejection. Pending
shared-allocation tests will verify subrange preservation and publication timing.
Original full/face constructor and actual recipe selection require the root's
original-data gate; no agent Cargo or fake pose was used.

## Restore ordering and hierarchy review follow-up

Fresh raw DoSimRestore confirms cvar target/value at827e6834/6838 precede
Face_Offset0 lookup6854 and offset68a0, then live GetPlaybackData68b8.
Bind OverrideWith6968 follows these operations and precedes BeginSet69a4.
The existing draft cvar-before-copy order was therefore correct and retained.
The selected playback is now resolved through the actual retained manager's
current entry at that source point, rather than substituting the constructor's
cached selected playback owner.

The old comment describing whole-full-rig copy was corrected: OverrideWith is
source-part matching in authored order, as independently reviewed at825920b0.
Presentation bind and all18 replay buffers now use allocate_for_hierarchy with
the same Arc of actual parsed SkaterPresentationDB hierarchy metadata. This
replaces anonymous-count allocation in these NEW unregistered draft files only;
registered hand/face allocation migration remains gate coordinated.

Root additional review: fresh OverrideWith825920b0 full raw+decomp confirms SOURCE hierarchy part loop, actual GetPartIndex destination, destination offset and SOURCE offset/count,48-byte copy. Proposed range patch remains unapplied; same-allocation forward-copy handling and synchronized replay allocation are required. This review does not admit the full presentation path.
