# Post-wipeout grounded foot IK

2026-09-13, unregistered draft; confidence **high** for recovered ordinary
semantics. Independent raw review and root integration/Cargo/corpus remain
required. This does not complete Update4, hand IK, post-IK target clearing,
HeadTracking, pose publication or a playable frame.

## Source and actual owner path

`Skeleton::UpdatePostPhysics_PostWipeoutCheck82b578b8` selects
DoPostPhysicsFootIK82b55bf0 at57bd4 when current8e8!=702, category8ec!=500,
and either physics8e4 is not200/201 or actual board Body360 contact count!=0.
Its arguments are the real IsWipeoutRequested boolean and the captured deck
matrix, replaced by the root-wobble adjustment if that branch executed. Call
this draft only after those actual caller decisions; no helper default skip
or unconditional successful result substitutes for the selection.

New files, registration proposal:

- physical_player_post_wipeout_foot_ik.rs: child `post_wipeout_foot` of
  skeleton_ik_state.rs, consuming the existing state arrays directly.
- physical_player_post_wipeout_foot_ik_runtime.rs: child
  `post_wipeout_foot_live` of dynamic_assembly_skeleton_normal.rs.
- physical_player_post_wipeout_foot_ik_owner.rs: child of skeleton_ik.rs.
- physical_player_post_wipeout_foot_ik_current.rs: child of
  skeleton_current_animation.rs.

The last two only split the existing mutable IK/attribute owner from its
retained DataIn, geometry and existing target mappings; they do not reset,
allocate or copy an alternate IK. The runtime source mutates the actual
attached Part via try_set_part_transform_typed, then the same actual
physics_history PhysRecord. It accepts the actual Processed owner, existing
global physics_animation Instance and separately the actual GLOBAL plain
animation Instance at GlobalAttributeCollection+0xd4.
Root must establish the normal world-pair identity before these borrows.

Root owns the typed PhysRecord W knowledge amendment required by this adapter:
`post_wipeout_record_transform(usize)->SkeletonRootMatrix` and
`try_set_post_wipeout_record_transform(usize,&SkeletonRootMatrix)` on
SkeletonPhysicsHistory. Their mutation is SetTransform only, not cached
positions/velocity/COM. Unknown native packed translation W propagates through
typed full-vector/matrix operations; dot3 consumes XYZ only. Host scratch W0
is used only for those XYZ numeric services, never to publish canonical W.

## Foot caller semantics

Read Processed8c4bit40000 at55c24. Only set reads state timer974; ordered timer
>exact3d23d70a returns at55c44. NaN timer continues. Transform actual deck Y by
DataIn.WorldToAnim direction-only graph, then always fetch global
physics_animation.default key`3b2641b64fcf3485` at55cd4. Absent field uses
DefaultDataArea4. Ordered minimum>localUpY skips the loop; NaN does not.

Process entries0 and1 in order. Existing parts_on_deck!=0 and no starting bail
selects deck.compose(part_to_deck), then actual parent PhysRecord position,
then stores actual part_to_world and parent_to_world from retained deck frames.
The original parent record read precedes caller target stores. Solve delta
from parent target P minus captured parent record P. Successful two-bone helper
is followed by caller Part/record primary writes, then caller Part/record parent
writes. Read kDrawPostPhysIK even if the helper returned false.

Otherwise skip an entry only if its actual external_target_set is true.
Read primary PhysRecord, evaluate dot(primaryY,deckY), read parent PhysRecord,
then native560cc40990198 tests !GT and skips intersection on unordered too.
Only positive ordered dot enters
DoesBoneIntersectDeck. If intersection succeeds, project new-minus-old primary
position along deckY and invoke the same helper. Successful result writes the
primary original matrix with new P, then freshly reads the parent record and
adds delta to its current P before writing actual Part and record. Both the
intersection-success and miss paths read kDrawPostPhysIK at their source sites.

## DoesBoneIntersectDeck82b53398

Transform bone and parent positions by rigid inverse of the actual deck.
Average them, then replace midpoint Y with bone-local Y. Read retained deck
halfwidth/totalhalflength and live OWN physics_skeletonik vector key
`693453cdb831119b` at534cc (DefaultDataArea16 if absent). Add that margin to
the width/0/total-length/0 bounds. Compare min(abs(boneX),abs(midpointX)) with
width, then separately min(abs(boneZ),abs(midpointZ)) with total length.
Native reuses one stack slot; decompiler incorrectly suggests the same scalar
for both tests. Raw53550/53560 BO4/BI24 tests !LT, rejecting unordered too.
VMX vminfp propagates either operand NaN; the following bounds test rejects it.
Rust min suppression is not used for unordered operands; payload cannot escape.

Inside bounds, fetch GLOBAL animation.default selected key, with singleton
requirement at53574/84:

- bail: `38e91dfbcb5e99f3`, generated FeetRelativeHeightWipeout82b3aff8;
- ordinary: `8e12e37ff8c71792`, FeetRelativeHeightOnDeck82b3afb0.

Correction before registration: FootIkLiveInput.global_animation is an
Option<&AttributeRuntimeInstance> for plain classbabb3ec110849e08,
defaultd7edbd362d7d2152, source GlobalAttributeCollection+d4. Absence errors only
at this reached global lookup. It must not route through
SkeletonBodyAttributes::GlobalAnimation: that misleadingly named existing route
is class74bc9c93affe6aac, the DIFFERENT physics_animation instance at+cc used
by the earlier minimum-deck-up test. Nor may the Skeleton-owned plain Instance
stand in for the global owner. Only an absent FIELD uses DefaultDataArea4.

Existing route inventory: SkeletonDriveRuntime.update_attributes retains
SkeletonDriveAttributes.update_global bound to that same plain class/default
(skeleton_drive_attributes.rs:111), used by its actual drive Update lookup_global.
It is private and has no borrowed-instance getter. PhysicalWorldServices has
manager/game_info/physical collection only, no GlobalAttributeCollection bundle.
Root must expose/reuse that retained global provider for the selected immutable
Vault projection or supply its coherent global bundle; this draft does not
create a replacement global on each callback.

Native5359c40990020 skips slope on !GT, including unordered. Only
ordered Z>boxHalfLength enters; use
source FSEL to choose Z-boxLength or totalLength-boxLength, multiply by retained
endSlope and FMA-add relative height. Raw535c440990078 returnsfalse on
!GT(height,boneLocalY), including unordered.
Success copies targetInDeckSpace, replaces Y, transforms back to world, then
returns true. Every source return path samples kDrawPostFeetIK, requiring the
actual debug overlay when enabled.

## DoTwoBoneIk_PostPhysics82b536c0

Resolve selected parent/grandparent from same SkeletonData, then GetBonePos
82b4a1c8 for selected,parent,grandparent in that order. It obtains bone position
by actual PhysRecord matrix applied to volumeToBone translation, not the
prephysics animation buffer. Target=end+delta. Reuse existing TwoBoneIKSolve
at53818 with fixed minimum0, maximum180, overridefalse, one attempt. No clamp,
no live minimum/maximum-angle attributes and no rotation normalization are
present in this postphysics wrapper.

Read kDrawPostFeetIK before checking the solver result; any non-Reachable
returns false. Otherwise form old and new plane normals as
cross(end-middle,root-middle), add, NormalizeSafe using existing numeric graph.
CalculateMatrixFromLineNew lower first, upper second only if lower succeeded.
If both succeed, capture BOTH current parent and grandparent PhysRecord
matrices, precompose the solved transforms, write lower Part then record at
53b30/40, then upper Part then record53b58/68. Finally another live debug read,
even if either line-frame result wasfalse. This differs from prephysics
ancestor ordering, so the old wrapper cannot be reused wholesale.

## Debug controls and limits

Reserved by root203/204, both category Physics_Skeleton_IK, defaultfalse,
callbacknull:

-203 kDrawPostFeetIK, title `Draw Post FeetIK Info`, initializer82f7d8c8,
 constructorcall82f7d8f4.
-204 kDrawPostPhysIK, title `Draw Post Physics`, initializer82f7d9b8,
 constructorcall82f7d9e4.

Runtime FootIkLiveDebug must sample those actual cvars and require the actual
overlay; no injected false accepted as a live implementation. Native debug
draw callees used here are the existing BLR stubs; this draft models the
observable cvar/singleton schedule, not a fabricated drawing renderer.

Focused tests cover lazy state-timer access, timer ordered/NaN gates, live
minimum sampling and NaN minimum reaching the real missing-mapping requirement.
Tests are synthetic source-order checks, not physical replay. Standalone
rustfmt passed; no Cargo by this author. Full kernel source review, real-owner
registration, typed record amendment, cvar binding, concrete actual-corpus
execution and numerical reference traces remain root integration requirements.
