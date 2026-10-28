# Native retained BlendAnim producer

Recovered fresh IDA source and raw branch words 2026-09-19. Draft:
`crates/skate2-core/src/animation/runtime/actor_animation_blend_anim.rs`, intended child of
`actor_animation_construction`. No Cargo or shared-file edits by author.

## Exact entry and dependencies

`TransitionBlendAnim82d7ae70` rearranges arguments for `TransitionAnim82d7aa68`:
transType=1, requested length/speed/time/current-frame/placement, channel-from-weights=false.
First GetBaseAnim virtual call aaac; null invokes PlayAnim aad8 with original name,
speed and **positive zero time**. Length/time/placement are ignored on this branch.
Existing PlayAnim producer is reused, not re-created.

Shared integration contract agreed with root: `BodyAnimationReference::{Tree(BodyAnimationRoot),
TransitionBlend(usize)}`; `AnimationTransitionBlend.runtime: Option<TransitionBlendRuntime>`;
base body_root and bottom-channel Body reference use the enum. Runtime flags model only
known high three bits, preserving lower29 as unknown in diagnostic representation.
Expose clear_play_lists/clear_play_list to sibling modules. Common allocation helper
`try_allocate_body_animation_tree(name)` contains only NewTree, pool Clear, GetAnimTree,
and AddBindPose; producer retains caller-specific name/speed/time stores.
`try_set_body_animation_reference` must perform actual base then bottom-channel publication.

## Non-null producer order

1. For Blend, equality of blend-list size/max at ab10 prunes oldest via ab18/ab24.
   Recheck equality ab34 before next capacity check. Transition count==maxTrees-1
   ab60 independently prunes oldest ab68/ab74, then equality assertion ab88.
2. NewTree aba8, actual virtual GetAnimTree abc4. Null result clears transitions ae00,
   then tree free list ae08; base pointer/name remain unchanged. Allocation prefix survives
   until these native clear calls; an actual host ownership error is not a lookup miss.
3. Name stores abf0..ac20, SetSpeed ac30, only ordered-positive offset SetTime ac58.
4. Nonempty transition list AND placement!=0: youngest ad34, allocate blend ad4c,
   initialize from youngest.to to newtree ad54, flags ad78, youngest.to=newblend ad7c.
   Otherwise allocate ad90, GetBaseAnim ada0, Init adb8, flags ade4, SetBaseAnim adf4.
5. NewTransitionBlend82d7a2d8 checks both list capacities, pops LAST free slot,
   appends blend-order a3a0, then transition-order a3ac. No payload clearing.
6. Init82d6b810 writes to b83c and from b840 before length assertion.
   Raw b848=4199001c is BGT: zero, negative, unordered fail after child publication.
   Successful path writes length b868, resets time through virtual b878 (df88 stores0),
   then b884/b888 inserts current-frame20000000 and clears top2 bits. Blend wrapper
   later clears top2 again. Evaluated flag remains unchanged. Tests cover failed Init
   prefix and free-slot/payload reuse; they intentionally use inert typed transition
   references because Init does not dereference its children.

## Ownership pruning and release

ForceTransitionRelease82d7a990 first verifies membership. GetParentTransition82d72030
scans transition_order oldest first, tests FROM at20a4 before TO20b0. Replace parent
child (or virtual SetBaseAnim) with released.to BEFORE releasing released.from.
ReleaseAllTransitions82d7a908 recursively releases FROM a958 then TO a964 then
transition slot a970; tree leaf goes to TreeStore.Release a984.
TreeStore.Release82d79f00 identifies owning allocated pool, rejects already-free pool,
appends pool to free list; no pool clear or animation payload destruction occurs.
TransitionStore.Release82d7a490 appends free blend a5e8, removes blend-order a5f8,
then transition-order a6a8. Payload preserved. Typed draft validates actual node/owner
before pool release and exposes try_force_transition_release for AddTime consumer.
Sequence producer remains separately unsupported; do not populate the untagged integer
transition_order with sequence entries until store/reference representation supports it.

## Required live consumers (root/independent consumer audit)

AddTime82d6df98 advances FROM with local default properties unless current-frame flag,
then TO with caller properties, then its own clock. Complete82d6e070 is strict time>length.
GetBlend82d6e090 clamps ratio with native FSEL and uses smoothstep at length>=.05,
including unordered length branch behavior. Base property/attribute/time/length accessors
forward to TO. Eval82d6e198 evaluates FROM then TO before positive-length assertion and
emits the original blend token; evaluated flag commits after child evaluation.
Hand/face EvalBlendPose82d78f30 has different assertion/culling order; do not substitute
PhaseBlend's thresholds. Consumers and shared fields are owned by root, not this draft.

Validation still required after mounting: strict workspace checks and actual original7
neutral1170 PlayAnimation2172 continuation, including retained prior body tree, both
placement branches, missing-name clear prefix, and completed/capacity-forced release.
