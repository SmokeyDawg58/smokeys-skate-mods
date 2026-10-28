# UpdatePoseBuffer retained publication — work in progress

Source: `Sk8::Physics::Skeleton::UpdatePoseBuffer`, `82b4d970..82b4ee98`, called from PostWipeoutCheck. Confidence high for retained ownership and the independently accepted first two loops; board visual tail draft awaits independent review. This packet does not claim a completed frame or rendered parity. No Cargo has been run by the author.

## Actual pose owner and W metadata integration

`ProcessData82b5ece8` retains the same `Arc<Mutex<AnimOutPhysInOwner>>` in constructor bindings, corresponding to Skeleton+2ca4. Native +0 global and +4 local arrays are this owner's existing `global_pose` and `local_pose`. Allocate no replacement pose. Parts map through SkeletonData bone_ids; unmapped animation bones must remain untouched.

New exclusive draft `physical_player_post_wipeout_pose_owner.rs` is a child of `actor_anim_out_phys_in.rs`, suggested module name `pose_lanes`. Root integration changes:

1. Register child, add `pub(crate) pose_w: pose_lanes::PoseWKnowledge` to the owner, initialize `Default::default()` in `new_sim_allocation`.
2. At the END of `reset_fields`' actual pose loop, call `mark_pose_w_known(Global)` then `mark_pose_w_known(Local)`, with constructed-allocation expectations. Native Reset DOES write all global/local matrices. This also establishes metadata at successful construction, because try_construct calls reset_fields after setting size. Metadata is not required before that actual reset/publication boundary.
3. In `actor_animation_get_phys_update_data.rs`, immediately after each `output.copy_from_slice`, call `mark_pose_w_known` for that particular buffer. Global metadata is committed BEFORE fetching/validating the local input, preserving native partial publication. Translate errors through the existing Missing/error boundary as appropriate. Source `GetPhysUpdateData` global memcpy97590 precedes local memcpy975a4.
4. All native 16-lane consumers must use typed reads or `pose_known_lanes` until migrated. The latter explicitly rejects unknown W. Relevant real-owner reads found: `skeleton_process_data_prefix.rs` trajectory full copy, `skeleton_process_data_output_runtime.rs` current-animation input. `physical_player_skeleton_anim_attributes.rs` trajectory speed consumes only translation XYZ and may keep those direct XYZ reads. HeadTracking's new actual head input must use `pose_matrix_typed(Global, named_bones[6])`. Animation-body/face/hand similarly named fields elsewhere belong to different owners.
5. Constructor/Reset writes and the two actual animation memcpy writes are the only production raw writers found by `rg` at draft time. Existing direct matrix assignments in owner unit tests are fully known values; their reset constructor establishes knowledge. Metadata invalidation is mandatory on every later unknown-W publication.

The child stores only four W-known bits per matrix; XYZ and known W remain in the SAME arrays. For unknown W a write leaves the raw storage lane intact and invalidates its bit. These stale bits are never a native W value. No renderer projection is introduced. A native full matrix copy propagates the unknown bit. A real Reset or real known animation memcpy restores knowledge.

Focused regression included: typed global write with unknown translation W, copy into local, unaffected bone retains known W, raw-full-read rejects unknown W, actual Reset restores both. Requires root registrations and reset hooks before compiling/running.

## Source publication order

Ascending Parts0..23: actual authored-parameter presence assertion, current typed PhysRecord transform, volume_to_bone, actual WorldToAnim. Fresh IDA PDB type inspection corrected the initial inventory: Skeleton+40 owns SkeletonData; its +8 mBoneHandles is `const Andale::PhysicsParamBoneData *[24]`, NOT rigid-body attachment pointers. The retained SkeletonData.parameters.bones entries are the actual authored objects; its constructor validates all24. Publish `WorldToAnim.compose(record.compose(volume_to_bone))` into global pose[bone_ids[part]] at db78..db84.

Then actual GetSkateboard/GetBody/Part6 Body.GetPosition at dbb8/dbc8/dbd8 captures the deck matrix BEFORE any local-pose publication. Descending Parts23..0: if parent exists, invert current GLOBAL parent pose and compose current GLOBAL child, then write local[bone_ids[part]]. Root part copies its GLOBAL matrix, preserving unknown W. Keep unmapped bones untouched. Native rigid inverse uses XYZ and establishes its own W lanes; do not require input W when the actual operation does not consume it.

Remaining mandatory board tail starts de30: attached-body transform for front/back trucks, then wheels1..4. This is Part.GetBodyTransform (`try_attached_part_body_transform`), distinct from packed live Part.GetTransform. Live global physics_skeleton attributes, selected physics-truck-tilt geometry, retained wobble, actual average compressions, and local truck/wheel transforms must all be recovered before claiming UpdatePoseBuffer complete.

Concrete `physical_player_post_wipeout_pose.rs` child of SkeletonNormalAssembly now implements these first two loops directly from state.binding.body.data, actual typed history and actual root transforms. Caller must supply the SAME retained constructor.bindings.animation_input, plus actual board/backend from that same validated world pair. Returns `PoseBeforeBoardVisuals` carrying the native deck capture; it does not complete UpdatePoseBuffer. Suggested module name `post_wipeout_pose_live`.

Independent reviewer accepted the metadata child. Reviewer identified one integration-order requirement: trajectory unknown-W checking must occur AFTER the native current=next prefix, not eagerly in the outer runtime adapter. Preserve those preceding writes when a full-lane consumer encounters unknown W.

## Mandatory board visual tail draft

`physical_player_post_wipeout_pose_board.rs` is registered ONLY beneath the new, still-unregistered pose parent as `board_visuals`. Concrete `try_post_wipeout_pose_board` consumes the preceding token and SAME mutable pose. All other owners must be borrowed from the SAME validated retained world pair: actual board session/backend, board construction, current truck drives, Skeleton constructor, and actual global physics_skeleton.default (+ec).

Read order: front attached Body transform Part4 at de50, back Part5 at deb4; wheels Parts0..3 at df14,df58,df9c,dfe0. Only inverse truck orientation is consumed. Capture wheel Y and positions; inverse truck orientation transforms first two Y axes with front, last two with back. Four AngleBetweenVectors calls compare those axes to J with K winding. Wheels0/1 use their returned angle, wheels2/3 bitwise negate at e1c8/e24c BEFORE SinCos e1cc/e250. Independent review caught and corrected the initial missing negations.

SinCos produces rotations about X with native permute constants: bytes at82fd6e90 are `00010203141516170001020300010203`, selecting [outer.x,inner.y,outer.x,outer.x]. X column `(1,0,0,1)` follows Z insertion; Y `(0,cos,sin,0)`, Z `(0,-sin,cos,0)`, Pzero. The unusual X.w is retained in the draft; rotations are inner matrices and affine composition does not use their W as coefficients.

Live global physics_skeleton scalar fetches, absent field alone DefaultDataArea4/zero:

| Site | Key | Role |
| --- | --- | --- |
| e2dc | ecdbb05bb9a9c877 | truck angle scale |
| e334 | 770b3abe0e17c90d | symmetric angle cap |
| e38c | 023cbbcdaebdc56b | wobble tilt scale |
| e938 | 1ba7a9d7c680ffee | compression cap |

First three precede truck axis captures; fourth follows the two actual average-compression reads. The input references one actual global instance under retained owner borrowing; concurrent replacement/mixed singleton identities is outside this typed borrowed-owner boundary, not emulated by snapshots or default tuning. Missing global owner is never field-default zero.

Capture current_back+1ed0 X then current_front+1e90 X as winding normals. These are actual retained `SkateboardTruckDrives::current_back/current_front`, NOT level transforms (+1e10/+1e50) and not reconstructed from input tuning. Read actual Skeleton.use_physics_truck_tilt+3e41. When true, actual `Skeleton.GetPartTransform(0)` at e458 retains its live plain-animation selection; inverse orientation transforms normalized wheel0.P-wheel1.P into the back candidate, then wheel3.P-wheel2.P into the front candidate. NormalizeSafe fallback is explicit zero. Squared length below0.5 asserts; draft stops at the actual assert/debug-break boundary and does not implement debugger-resumed fallback. NaN squared comparison does not assert. When false, actual current_front.Z is bitwise negated for front candidate and actual current_back.Z supplies back candidate.

Front candidate angle uses -I with captured BACK winding, then back candidate uses -I with captured FRONT winding. Normalize to [-pi,pi) with exact source constant bits; FNMSUBS negates the rounded fused subtraction. A dedicated source helper retains negative zero on an exact positive turn instead of distributing the negation into FMA operands. No host remainder or libc angle substitution.

Actual outer deck_wobble_tilt times live scale is added/subtracted through two source FNMSUBS; two FSELs bound each result. First SinCos e800 uses front-derived tilt, second e85c back-derived tilt. Native front LOCAL bone uses the SECOND rotation, back LOCAL uses FIRST; this apparent crossing was independently verified from raw registers by reviewer.

Read actual average front/back compressions via board construction's retained wheel_compressions owner. Raw e964/e97040980008 tests !LT and skips replacement for unordered too;
only ordered values below.001 become0. NaN reaches the later FSEL cap graph. Subtract actual deck_wobble_squish; lower/upper FSEL caps retain unordered behavior. Publish front truck local as `Translate(frontOffset).compose(oldLocal.compose(backRotation))`, then back as `Translate(backOffset).compose(oldLocal.compose(frontRotation))`. Named indices are constructor.init.named_bones[1] front, [0] back. Then wheel locals named[2..6] each compose their captured rotation in ascending order. Late missing named bone preserves earlier writes. Corresponding globals remain the values from the preceding24-loop.

Unknown native W survives wheel full-lane composition; truck outer translation/rotation source graph establishes its own W when input XYZ are sufficient. No source W is filled from homogeneous rendering conventions. Native rigid inverse/dot3 ignore input W explicitly; using those operations never requires all lanes known.

Standalone rustfmt passed. Focused tests cover real Reset restoring metadata, unknown matrix copying, exact-turn negative zero, NaN compression versus cap selection, and rotated XYZ with unknown outer W propagation. Root must compile/register and run appropriate tests; no dynamic trace parity or complete PostWipeoutCheck is claimed. HandIK, active wobble root adjustment, and other caller continuations are outside this packet.

Root integration in progress after accepted workspace77: all three pose modules are registered. Same AnimOut owner now carries pose metadata; real Reset and separate animation memcpy hooks restore known lanes at actual publication points. The retained trajectory compatibility check executes after current=next; current-animation compatibility rejects unknown full lanes. Both guards are deliberately conservative until typed consumer migration. A regression checks the trajectory prefix on unknown-W failure.

New pose_pool/pose_world bind the actual checked board/Skeleton pair, retained constructor animation_input Arc and current actor input identity, then hold one pose guard across both stages. The global attribute getter borrows the already-retained SkeletonBodyAttributes.global_skeleton; no fresh binding or tuning default is created. Provider-presence checks assume fully constructed admitted owners and precede the tail's native value reads; deliberately stripped providers stop at that boundary. Runtime_path independently accepted ownership and store ordering. Direct solved-Actor corpus probe follows selected Fill only as an explicitly separate producer test, not as a recovered post-wipeout schedule; it checks mapped globals, untouched unmapped globals, retained Arc identity and successful mandatory tail execution. Independent local/truck/wheel value comparisons are not yet covered by that corpus. Clippy136 passes; workspace78/original-data validation pending.

September13 accepted retained pose checkpoint: workspace78 passes 4,192 unit tests and six doc tests; original66 passes all17 (8.80s); Clippy136, fmt50 and diff51 pass. UpdatePoseBuffer global/local mapping and mandatory truck/wheel/compression tail now execute on the SAME Actor AnimOutPhysIn allocation through validated board/Skeleton owners and retained global attributes. Actual Reset and animation memcpy hooks track W knowledge. Direct original-data pose probe verifies owner identity, mapped globals, untouched unmapped globals and successful full tail; it is deliberately separate from the still-incomplete post-wipeout caller schedule. Conservative full-lane consumer guards preserve the trajectory prefix. Independent review accepted source math and owner integration. No playable frame or independent native per-tick trace claim. Next: checked native PhysRecord.SetTransform validation (fresh review found draft setter skipped XYZ NaN assertions), active root wobble, foot/hand IK, IK postupdate, head adjustment, caller pose dispatch, remaining conditioners/Motion/camera/render schedule. Extra drafting remains parked.
