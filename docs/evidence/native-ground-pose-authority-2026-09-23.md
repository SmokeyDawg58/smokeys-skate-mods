# Native Ground pose authority after wheel support

Read-only audit of the matched `sk82_na_m.xex` IDA/PDB on 2026-09-23. No native
per-tick pose capture or Rust behavior change. Source call/store order and owner
selection below are **exact**; identifying the cause of the current reclined
Rust image remains a **hypothesis** until the intermediate matrices are logged.

## Two independent physical outputs

The upright board and the Skeleton root do not themselves specify the skater's
limb or hips pose. `Skeleton::UpdateRootTransforms` `0x82b42938` derives its
`AnimToWorld` from board Part 6, current animation Skate transform, and
Reckoning; `Skeleton::FillPhysOut` `0x82b58230` publishes that root (see
`skeleton-retained-root-transforms.md` and `presentation-root-w-2026-09-22.md`).
`SubmittableSkater::GetRootMatrix` `0x825f04a0` then reads the PhysOut root for
`SkaterPresentation::WriteData` `0x827dbf98` (root at `0x827dc240`, position
store `0x827dc328`). That is the **world placement** stream.

The skater **bone pose** has a separate live physical producer.
`PhysicalPlayerHiLOD::Update4_PostEngineAdjust` `0x82e01c78` calls
`Skeleton::UpdatePostPhysics` at `0x82e01d4c`, then state postphysics and
wipeout decision, then `Skeleton::UpdatePostPhysics_PostWipeoutCheck`
`0x82b578b8` at `0x82e01d94`. `UpdatePostPhysics` calls
`Skeleton::UpdatePhysicsRecords` `0x82b4cc00` at `0x82b576d8`.
`Skeleton::UpdatePhysicsRecords` calls
`SkeletonState::UpdatePhysicsRecord` `0x82b48288` at `0x82b4cc70`.
That producer sets PhysRecord Part 0 from **board Part 6** at `0x82b482dc`,
but Parts 1..23 from **the actual Skeleton assembly Parts** in ascending order
via `Part::GetTransform` and `PhysRecord::SetTransform` at
`0x82b48418/0x82b48428`. Part 23 is Hips, as independently recorded in
`skeleton-fill-suffix.md`. These Part transforms can differ from the board and
root even while all four wheels are in contact.

The post-wipeout method reads the live `kUpdatePoseBuffer` target at
`0x82b58200` and calls `Skeleton::UpdatePoseBuffer` `0x82b4d970` at
`0x82b58210` when true. The cvar initializer `0x82f7c5a8` registers
`Physics_Skeleton.kUpdatePoseBuffer` with default true at `0x82f7c5d4`.
`UpdatePoseBuffer` visits PhysRecord Parts 0..23, composes each current
transform with that SkeletonData Part's VolumeToBone and WorldToAnim matrices,
and writes mapped bones into the **existing Actor AnimOutPhysIn global pose**
at `0x82b4db78`. It next derives local pose from those globals, visiting Parts
23..0 at `0x82b4dc04..0x82b4de1c`; root Parts copy global, other Parts use
parent inverse times child. Its later truck/wheel visual tail is already
implemented in the retained owner (`post-wipeout-pose-live.md`). The current
Rust caller likewise runs the cvar-gated same-owner pose publication in
`physical_player_post_wipeout_live.rs`; this path should be measured, not remade.

`SubmittableSkater::GetLocalPose` `0x825f04e0` obtains that same local buffer
through `PhysicalPlayerHiLOD::GetLocalPoseBuffer` `0x82dffd88`. `WriteData`
reads full local Hips/board matrices at `0x827dc3c0..0x827dc43c`; `WriteSkater`
`0x827d8ab8` compresses selected local channels. ReadSkater restores replay,
`MoveRootToHips_Individual` `0x827d35d8` saves the replay Hips transform,
resets that Hips channel, and makes the replay board channel relative to Hips.
`UpdateBodies` `0x827de988` then executes root/expression correction and ACS
matrix extraction at `0x827deb24..0x827debd0`, overwrites output Hips slot 0
with the saved Hips matrix at `0x827dec00..0x827dec24`, and finally packs
model remap/inverse bind skin rows. This latter path is also already present in
the selected Rust presentation runtime; a mesh skin row is not a direct
reading of the physical hips orientation.

## Current observation and decisive boundary

The Rust `controlled-skating-release30-prebody-gravity-2026-09-23.log`
records Ground, four processed wheels, a board Y axis near `[0,1,0]`, and a
yaw-only PhysOut root at frame 30. Its first published mesh skin row set has a
large Y/Z rotation. Because those rows include model hierarchy and inverse
bind, this does **not** by itself prove which upstream pose stage rotated the
skater. No native matched-frame trace establishes the expected matrix values.

The narrow same-frame discriminator is:

1. After `UpdatePhysicsRecord` and before `UpdatePoseBuffer`, record board
   Part 6; Skeleton Parts 23 (Hips) and 1 (Neck); and PhysRecord entries 0,23,1.
   The existing PhysOut Skeleton lean computed from fresh Hips Part 23 at
   `0x82b594d8..0x82b59540` is a useful scalar corroboration.
2. Before and after cvar-gated `UpdatePoseBuffer`, record mapped AnimOut global
   and local Hips, Neck and skateboard matrices plus WorldToAnim. This isolates
   physical part/record drift from bone mapping or local conversion.
3. At presentation, record submitted local Hips/board; decoded replay before
   `MoveRootToHips`; saved Hips matrix; interpolated Hips and one torso bone
   before inverse bind; then the packed skin row. A divergence appearing only
   after replay implicates presentation arithmetic or hierarchy mapping.

If Part 23/PhysRecord Hips is already reclined while board Part 6 is upright,
the actionable owner is the Skeleton physical Part/drive/constraint route,
not board contact or world-root publication. If those Parts and the postpose
AnimOut local Hips remain upright, presentation replay/model conversion becomes
the target. The existing frame-30 log has not crossed this boundary.
